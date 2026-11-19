//! Tree reads by commit id: what a commit's tree holds at given paths, every
//! entry it holds, and blob bytes by id. Each read is one git child that runs
//! with `--no-lazy-fetch`, so a partial clone's missing object is `missing`,
//! never a fetch (D-13), and with `--no-replace-objects`, so a replace ref
//! made after the model's guard (D-22) changes nothing it reads.
//!
//! No path is ever an argument or a pathspec: a lookup sends each request as
//! `<commit id>:<path>` on stdin, NUL-terminated (`-Z`), so a path holding a
//! newline or pathspec magic is one request and nothing more; the listing
//! names only the commit id and reads NUL-terminated records (`-z`).
//!
//! Every caller reads these after `model::resolve` has named the commits
//! (05-01's read order), so a read here never resolves a ref.

use std::ffi::OsString;

use super::objects::{BATCH_FORMAT, BatchAnswer, parse_batch};
use super::repo::Repo;
use super::{Error, Git, NO_LAZY_FETCH, NO_REPLACE_OBJECTS, ObjectId};

/// git's tree entry mode of a directory.
pub const MODE_TREE: u32 = 0o040000;
/// git's tree entry mode of a regular file.
// `cfg_attr(not(test), ...)` on each item no command reaches yet: the
// tests use it, so on the test target the expectation would be
// unfulfilled. The reason is short so rustfmt keeps it on one line.
#[cfg_attr(not(test), expect(dead_code, reason = "not called by any command yet"))]
pub const MODE_BLOB: u32 = 0o100644;
/// git's tree entry mode of an executable file.
pub const MODE_EXECUTABLE: u32 = 0o100755;
/// git's tree entry mode of a symlink, whose blob holds the link text.
pub const MODE_SYMLINK: u32 = 0o120000;
/// git's tree entry mode of a gitlink (a submodule's commit).
pub const MODE_GITLINK: u32 = 0o160000;

/// One tree entry found at a requested path: its mode and object id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeEntry {
    /// The entry's mode, as git prints it in octal.
    pub mode: u32,
    /// The object the entry names.
    pub id: ObjectId,
}

/// One entry of a whole-tree listing: its mode, object id and raw path from
/// the tree's root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedEntry {
    /// The entry's mode, as git prints it in octal.
    pub mode: u32,
    /// The object the entry names.
    pub id: ObjectId,
    /// The path from the root tree, raw bytes, never quoted.
    pub path: Vec<u8>,
}

/// The lookup's batch format: id, type and entry mode of each answer.
const LOOKUP_FORMAT: &str = "--batch-check=%(objectname) %(objecttype) %(objectmode)";

/// The argv of the lookup child: NUL-terminated requests and answers.
const LOOKUP_ARGS: [&str; 5] = [
    NO_REPLACE_OBJECTS,
    NO_LAZY_FETCH,
    "cat-file",
    LOOKUP_FORMAT,
    "-Z",
];

/// The argv of the listing child, before the commit id: every entry,
/// directories included, NUL-terminated, from the root tree.
const LIST_ARGS: [&str; 7] = [
    NO_REPLACE_OBJECTS,
    NO_LAZY_FETCH,
    "ls-tree",
    "-r",
    "-t",
    "-z",
    "--full-tree",
];

/// The argv of the blob read child, by id.
const BLOB_ARGS: [&str; 4] = [NO_REPLACE_OBJECTS, NO_LAZY_FETCH, "cat-file", BATCH_FORMAT];

impl Repo {
    /// For each of `paths`, in order, the entry `commit`'s tree holds there,
    /// or `None` when it holds none, from one `git --no-replace-objects
    /// --no-lazy-fetch cat-file --batch-check=... -Z` child fed
    /// `<commit id>:<path>` NUL requests through `Git::run_with_input`. A
    /// path is a request on stdin only, never an argument or a pathspec, so
    /// a newline or pathspec magic in it widens nothing. A gitlink, whose
    /// commit git usually does not hold, is answered `<id> submodule` and
    /// read as `MODE_GITLINK`. An empty `paths` spawns no child.
    ///
    /// The child's failure is `Error::GitFailed`; an answer count that is
    /// not one per request, or an answer that does not parse, is
    /// `Error::GitBadOutput`.
    pub fn lookup_paths(
        &self,
        git: &Git,
        commit: &ObjectId,
        paths: &[&[u8]],
    ) -> Result<Vec<Option<TreeEntry>>, Error> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let requests: Vec<Vec<u8>> = paths
            .iter()
            .map(|path| {
                debug_assert!(!path.contains(&0), "a path holds no NUL");
                [format!("{commit}:").as_bytes(), path].concat()
            })
            .collect();
        let mut input = Vec::new();
        for request in &requests {
            input.extend_from_slice(request);
            input.push(0);
        }
        let cwd = self.worktree().unwrap_or(self.git_dir());
        let stdout = git.run_with_input(cwd, LOOKUP_ARGS, &input)?;
        parse_lookup(&stdout, &requests).map_err(|problem| Error::GitBadOutput {
            argv: LOOKUP_ARGS.iter().map(OsString::from).collect(),
            problem,
        })
    }

    /// Every entry of `commit`'s tree, directories included, in git's tree
    /// order, from one `git --no-replace-objects --no-lazy-fetch ls-tree -r -t
    /// -z --full-tree <commit id>` child. Output that does not parse is
    /// `Error::GitBadOutput`, never a shorter list.
    pub fn list_tree(&self, git: &Git, commit: &ObjectId) -> Result<Vec<ListedEntry>, Error> {
        let mut argv: Vec<OsString> = LIST_ARGS.iter().map(OsString::from).collect();
        argv.push(OsString::from(commit.to_string()));
        let cwd = self.worktree().unwrap_or(self.git_dir());
        let stdout = git.run(cwd, &argv)?;
        parse_listing(&stdout).map_err(|problem| Error::GitBadOutput { argv, problem })
    }

    /// The bytes of each blob in `ids`, in order, from one
    /// `git --no-replace-objects --no-lazy-fetch cat-file --batch=...` child
    /// fed one hex id per line (an id holds no LF). An id git answers `missing` for, as a
    /// partial clone answers for a blob it never fetched, is `None`, so the
    /// caller decides whether it needed it; an object that is not a blob, or
    /// output that does not parse, is `Error::GitBadOutput`. An empty `ids`
    /// spawns no child.
    pub fn read_blobs(&self, git: &Git, ids: &[ObjectId]) -> Result<Vec<Option<Vec<u8>>>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut input = Vec::new();
        for id in ids {
            input.extend_from_slice(id.to_string().as_bytes());
            input.push(b'\n');
        }
        let cwd = self.worktree().unwrap_or(self.git_dir());
        let stdout = git.run_with_input(cwd, BLOB_ARGS, &input)?;
        let bad = |problem: String| Error::GitBadOutput {
            argv: BLOB_ARGS.iter().map(OsString::from).collect(),
            problem,
        };
        let answers = parse_batch(&stdout, ids.len()).map_err(bad)?;
        ids.iter()
            .zip(answers)
            .enumerate()
            .map(|(index, (id, answer))| match answer {
                BatchAnswer::Found { kind, data, .. } if kind == b"blob" => Ok(Some(data)),
                BatchAnswer::Missing => Ok(None),
                BatchAnswer::Found { kind, .. } => Err(bad(format!(
                    "answer {index}: {id} is a {}, not a blob",
                    String::from_utf8_lossy(&kind)
                ))),
                BatchAnswer::Submodule => {
                    Err(bad(format!("answer {index}: {id} answered as a submodule")))
                }
            })
            .collect()
    }
}

/// A mode git printed in octal, or `None` for text that is empty or holds
/// anything but octal digits.
fn octal_mode(text: &[u8]) -> Option<u32> {
    if text.is_empty() || !text.iter().all(|byte| matches!(byte, b'0'..=b'7')) {
        return None;
    }
    u32::from_str_radix(std::str::from_utf8(text).ok()?, 8).ok()
}

/// The lookup's answers in `stdout`, one NUL-terminated answer per request
/// in `requests`, in order: `<request> missing` is `None`,
/// `<id> submodule` a gitlink, and `<id> <type> <octal mode>` an entry.
/// Output not ending in NUL, an answer count that is not one per request,
/// or any other answer is `Err` naming the problem.
fn parse_lookup(stdout: &[u8], requests: &[Vec<u8>]) -> Result<Vec<Option<TreeEntry>>, String> {
    let Some(body) = stdout.strip_suffix(b"\0") else {
        return Err("the output does not end in NUL, so its last answer is incomplete".to_owned());
    };
    let answers: Vec<&[u8]> = body.split(|byte| *byte == 0).collect();
    if answers.len() != requests.len() {
        return Err(format!(
            "{} answers to {} requests",
            answers.len(),
            requests.len()
        ));
    }
    answers
        .iter()
        .zip(requests)
        .enumerate()
        .map(|(index, (answer, request))| {
            let bad = |problem: &str| format!("answer {index}: {problem}");
            if answer.strip_suffix(b" missing") == Some(request.as_slice()) {
                return Ok(None);
            }
            let fields: Vec<&[u8]> = answer.split(|byte| *byte == b' ').collect();
            let (id, mode) = match fields.as_slice() {
                [id, b"submodule"] => (id, Some(MODE_GITLINK)),
                [id, _kind, mode] => (id, octal_mode(mode)),
                _ => return Err(bad("an answer that is not an entry or missing")),
            };
            let id = ObjectId::from_hex(id).ok_or_else(|| bad("an id that is not full hex"))?;
            let mode = mode.ok_or_else(|| bad("a mode that is not octal"))?;
            Ok(Some(TreeEntry { mode, id }))
        })
        .collect()
}

/// The entries in `git ls-tree -z` output: records
/// `<octal mode> SP <type> SP <id> TAB <path> NUL`. Output not ending in
/// NUL, or a record of any other shape, is `Err` naming the problem.
fn parse_listing(stdout: &[u8]) -> Result<Vec<ListedEntry>, String> {
    if stdout.is_empty() {
        return Ok(Vec::new());
    }
    let Some(body) = stdout.strip_suffix(b"\0") else {
        return Err("the output does not end in NUL, so its last record is incomplete".to_owned());
    };
    body.split(|byte| *byte == 0)
        .enumerate()
        .map(|(index, record)| {
            let bad = |problem: &str| format!("record {index}: {problem}");
            let tab = record
                .iter()
                .position(|byte| *byte == b'\t')
                .ok_or_else(|| bad("no TAB before its path"))?;
            let fields: Vec<&[u8]> = record[..tab].split(|byte| *byte == b' ').collect();
            let [mode, _kind, id] = fields.as_slice() else {
                return Err(bad("a header that is not three fields"));
            };
            Ok(ListedEntry {
                mode: octal_mode(mode).ok_or_else(|| bad("a mode that is not octal"))?,
                id: ObjectId::from_hex(id).ok_or_else(|| bad("an id that is not full hex"))?,
                path: record[tab + 1..].to_vec(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Stdio;

    use super::super::testing::{
        self, Scratch, T0, TICK, commit_all, git_in_env, git_ok, object_id, write,
    };
    use super::super::{Error, Git, ObjectId, Repo};
    use super::{
        ListedEntry, MODE_BLOB, MODE_EXECUTABLE, MODE_GITLINK, MODE_SYMLINK, MODE_TREE, TreeEntry,
        parse_listing, parse_lookup,
    };

    /// The commit a gitlink names; no object in the fixture has this id, as
    /// a submodule's commit is usually absent from its superproject.
    const GITLINK: &str = "5555555555555555555555555555555555555555";

    /// The path holding a newline.
    const NEWLINE_PATH: &[u8] = b"a\nb";

    /// A repository at `<root>/repo` whose `HEAD` holds `a.txt` (100644),
    /// `x` (100755), `l` (a symlink to `a.txt`), `d/f`, the file
    /// `NEWLINE_PATH` and the gitlink `sub` naming `GITLINK`; returns the
    /// canonical repository and git's id of `HEAD`. The gitlink is staged with
    /// `update-index --cacheinfo` and committed on its own, as `commit_all`'s
    /// `add --all` would drop an entry with no directory.
    fn every_kind(scratch: &Scratch) -> (PathBuf, ObjectId) {
        let repo = scratch.root.join("repo");
        fs::create_dir_all(&repo).unwrap_or_else(|err| panic!("create repo: {err}"));
        git_ok(scratch, &repo, ["init", "-q", "-b", "main"]).unwrap_or_else(|err| panic!("{err}"));
        write(&repo.join("a.txt"), b"a\n").unwrap_or_else(|err| panic!("{err}"));
        write(&repo.join("x"), b"#!/bin/sh\n").unwrap_or_else(|err| panic!("{err}"));
        fs::set_permissions(repo.join("x"), fs::Permissions::from_mode(0o755))
            .unwrap_or_else(|err| panic!("chmod x: {err}"));
        std::os::unix::fs::symlink("a.txt", repo.join("l"))
            .unwrap_or_else(|err| panic!("symlink l: {err}"));
        write(&repo.join("d/f"), b"f\n").unwrap_or_else(|err| panic!("{err}"));
        let newline = std::ffi::OsStr::new(std::str::from_utf8(NEWLINE_PATH).expect("UTF-8"));
        write(&repo.join(newline), b"newline\n").unwrap_or_else(|err| panic!("{err}"));
        commit_all(scratch, &repo, "files", T0).unwrap_or_else(|err| panic!("{err}"));
        let cacheinfo = format!("160000,{GITLINK},sub");
        git_ok(
            scratch,
            &repo,
            ["update-index", "--add", "--cacheinfo", cacheinfo.as_str()],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let env = testing::dated(&scratch.env, T0 + TICK);
        let output = git_in_env(&env, &repo, ["commit", "-q", "-m", "sub"])
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run("git commit the gitlink", &output)
        );
        let repo = repo
            .canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize repo: {err}"));
        let head = object_id(scratch, &repo, "HEAD").unwrap_or_else(|err| panic!("{err}"));
        (repo, head)
    }

    /// git's id of `<commit>:<path>` in `repo`.
    fn entry_id(scratch: &Scratch, repo: &Path, commit: &ObjectId, path: &[u8]) -> ObjectId {
        let rev = format!("{commit}:{}", String::from_utf8_lossy(path));
        object_id(scratch, repo, &rev).unwrap_or_else(|err| panic!("{err}"))
    }

    /// What git's own `cat-file --batch-check=... -Z` prints for `input`
    /// in `repo`: the oracle the lookup is held to.
    fn git_batch_check(scratch: &Scratch, repo: &Path, input: &[u8]) -> Vec<u8> {
        let mut child = testing::command("git", repo, &scratch.env)
            .args([
                "cat-file",
                "--batch-check=%(objectname) %(objecttype) %(objectmode)",
                "-Z",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|err| panic!("spawn git cat-file: {err}"));
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(input)
            .unwrap_or_else(|err| panic!("write git cat-file's stdin: {err}"));
        let output = child
            .wait_with_output()
            .unwrap_or_else(|err| panic!("wait for git cat-file: {err}"));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run("git cat-file --batch-check -Z", &output)
        );
        output.stdout
    }

    /// `Repo::discover(repo)` and a helper whose children see the scratch
    /// environment.
    fn open(scratch: &Scratch, repo: &Path) -> (Repo, Git) {
        let found =
            Repo::discover(repo).unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()));
        let git = Git::for_repo(scratch.env.clone(), &found);
        (found, git)
    }

    #[test]
    fn lookup_paths_answers_each_request_in_order() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("trees-lookup").expect("scratch");
        let (repo, head) = every_kind(&scratch);
        let id = |path: &[u8]| entry_id(&scratch, &repo, &head, path);
        let (x, d, a, l, sub) = (id(b"x"), id(b"d"), id(b"a.txt"), id(b"l"), id(b"sub"));
        assert_eq!(sub, ObjectId(GITLINK.to_owned()), "git's id of the gitlink");

        let paths: [&[u8]; 6] = [b"x", b"missing", b"d", b"a.txt", b"l", b"sub"];
        let mut input = Vec::new();
        for path in paths {
            input.extend_from_slice(format!("{head}:").as_bytes());
            input.extend_from_slice(path);
            input.push(0);
        }
        let expected_git = format!(
            "{x} blob 100755\0{head}:missing missing\0{d} tree 040000\0\
             {a} blob 100644\0{l} blob 120000\0{sub} submodule\0"
        );
        assert_eq!(
            String::from_utf8_lossy(&git_batch_check(&scratch, &repo, &input)),
            expected_git,
            "git's own batch-check answers"
        );

        let (found, git) = open(&scratch, &repo);
        let answers = found
            .lookup_paths(&git, &head, &paths)
            .unwrap_or_else(|err| panic!("lookup_paths: {err}"));
        let entry = |mode, id: &ObjectId| {
            Some(TreeEntry {
                mode,
                id: id.clone(),
            })
        };
        assert_eq!(
            answers,
            vec![
                entry(MODE_EXECUTABLE, &x),
                None,
                entry(MODE_TREE, &d),
                entry(MODE_BLOB, &a),
                entry(MODE_SYMLINK, &l),
                entry(MODE_GITLINK, &sub),
            ]
        );
    }

    #[test]
    fn lookup_paths_reads_a_path_holding_a_newline() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("trees-newline").expect("scratch");
        let (repo, head) = every_kind(&scratch);
        let blob = entry_id(&scratch, &repo, &head, NEWLINE_PATH);
        let (found, git) = open(&scratch, &repo);
        let answers = found
            .lookup_paths(&git, &head, &[NEWLINE_PATH])
            .unwrap_or_else(|err| panic!("lookup_paths: {err}"));
        assert_eq!(
            answers,
            vec![Some(TreeEntry {
                mode: MODE_BLOB,
                id: blob
            })]
        );
    }

    #[test]
    fn list_tree_lists_every_entry_with_its_mode() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("trees-list").expect("scratch");
        let (repo, head) = every_kind(&scratch);
        let listed = |mode, path: &[u8]| ListedEntry {
            mode,
            id: entry_id(&scratch, &repo, &head, path),
            path: path.to_vec(),
        };
        let expected = vec![
            listed(MODE_BLOB, NEWLINE_PATH),
            listed(MODE_BLOB, b"a.txt"),
            listed(MODE_TREE, b"d"),
            listed(MODE_BLOB, b"d/f"),
            listed(MODE_SYMLINK, b"l"),
            listed(MODE_GITLINK, b"sub"),
            listed(MODE_EXECUTABLE, b"x"),
        ];
        let (found, git) = open(&scratch, &repo);
        let listing = found
            .list_tree(&git, &head)
            .unwrap_or_else(|err| panic!("list_tree: {err}"));
        assert_eq!(listing, expected);
    }

    #[test]
    fn read_blobs_returns_each_blob_in_order() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("trees-blobs").expect("scratch");
        let (repo, head) = every_kind(&scratch);
        let link = entry_id(&scratch, &repo, &head, b"l");
        let file = entry_id(&scratch, &repo, &head, b"a.txt");
        let (found, git) = open(&scratch, &repo);
        let blobs = found
            .read_blobs(&git, &[link.clone(), file])
            .unwrap_or_else(|err| panic!("read_blobs: {err}"));
        assert_eq!(blobs, vec![Some(b"a.txt".to_vec()), Some(b"a\n".to_vec())]);

        let absent = ObjectId("0123456789012345678901234567890123456789".to_owned());
        let answers = found
            .read_blobs(&git, &[absent, link])
            .unwrap_or_else(|err| panic!("read_blobs with an absent id: {err}"));
        assert_eq!(answers, vec![None, Some(b"a.txt".to_vec())]);
    }

    /// `every_kind` with `HEAD`'s commit replaced, through `git replace`, by a
    /// commit of the empty tree; returns the repository, the original id and
    /// git's id of its `a.txt`, read before the replacement.
    fn replaced_head(scratch: &Scratch) -> (PathBuf, ObjectId, ObjectId) {
        let (repo, head) = every_kind(scratch);
        let a = entry_id(scratch, &repo, &head, b"a.txt");
        let other = git_ok(
            scratch,
            &repo,
            ["commit-tree", EMPTY_TREE, "-m", "replacement"],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let other = String::from_utf8(other).expect("hex");
        git_ok(
            scratch,
            &repo,
            ["replace", head.0.as_str(), other.trim_end()],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        (repo, head, a)
    }

    /// git's id of the empty tree, which every repository has.
    const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

    /// A replace ref present when the lookup runs does not change what the
    /// commit's tree holds: the lookup reads the commit's own tree.
    #[test]
    fn lookup_paths_ignores_a_replace_ref() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("trees-lookup-replaced").expect("scratch");
        let (repo, head, a) = replaced_head(&scratch);
        let (found, git) = open(&scratch, &repo);
        let answers = found
            .lookup_paths(&git, &head, &[b"a.txt"])
            .unwrap_or_else(|err| panic!("lookup_paths: {err}"));
        assert_eq!(
            answers,
            vec![Some(TreeEntry {
                mode: MODE_BLOB,
                id: a
            })]
        );
    }

    /// A replace ref present when the listing runs does not change it: the
    /// listing is of the commit's own tree.
    #[test]
    fn list_tree_ignores_a_replace_ref() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("trees-list-replaced").expect("scratch");
        let (repo, head, _) = replaced_head(&scratch);
        let (found, git) = open(&scratch, &repo);
        let listing = found
            .list_tree(&git, &head)
            .unwrap_or_else(|err| panic!("list_tree: {err}"));
        let paths: Vec<&[u8]> = listing.iter().map(|entry| entry.path.as_slice()).collect();
        assert_eq!(
            paths,
            vec![NEWLINE_PATH, b"a.txt", b"d", b"d/f", b"l", b"sub", b"x"]
        );
    }

    /// A replace ref present when the blob read runs does not change the
    /// bytes it returns.
    #[test]
    fn read_blobs_ignores_a_replace_ref() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("trees-blobs-replaced").expect("scratch");
        let (repo, head) = every_kind(&scratch);
        let a = entry_id(&scratch, &repo, &head, b"a.txt");
        let x = entry_id(&scratch, &repo, &head, b"x");
        git_ok(&scratch, &repo, ["replace", a.0.as_str(), x.0.as_str()])
            .unwrap_or_else(|err| panic!("{err}"));
        let (found, git) = open(&scratch, &repo);
        let blobs = found
            .read_blobs(&git, &[a])
            .unwrap_or_else(|err| panic!("read_blobs: {err}"));
        assert_eq!(blobs, vec![Some(b"a\n".to_vec())]);
    }

    /// A tree asked for as a blob is `Error::GitBadOutput` naming it, not
    /// its tree bytes.
    #[test]
    fn read_blobs_refuses_an_object_that_is_not_a_blob() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("trees-not-a-blob").expect("scratch");
        let (repo, head) = every_kind(&scratch);
        let tree = entry_id(&scratch, &repo, &head, b"d");
        let (found, git) = open(&scratch, &repo);
        match found.read_blobs(&git, std::slice::from_ref(&tree)) {
            Err(Error::GitBadOutput { problem, .. }) => {
                assert_eq!(problem, format!("answer 0: {tree} is a tree, not a blob"));
            }
            other => panic!("expected GitBadOutput for the tree {tree}, got {other:?}"),
        }
    }

    /// A full hex id for the pure parser tests.
    const ID: &str = "1111111111111111111111111111111111111111";

    #[test]
    fn parse_listing_refuses_a_record_without_a_tab() {
        let stdout = format!("100644 blob {ID} a.txt\0");
        assert_eq!(
            parse_listing(stdout.as_bytes()),
            Err("record 0: no TAB before its path".to_owned())
        );
    }

    #[test]
    fn parse_listing_refuses_output_not_ending_in_nul() {
        let stdout = format!("100644 blob {ID}\ta.txt");
        assert_eq!(
            parse_listing(stdout.as_bytes()),
            Err("the output does not end in NUL, so its last record is incomplete".to_owned())
        );
    }

    #[test]
    fn parse_lookup_refuses_fewer_answers_than_requests() {
        let requests = vec![
            format!("{ID}:a").into_bytes(),
            format!("{ID}:b").into_bytes(),
        ];
        let stdout = format!("{ID} blob 100644\0");
        assert_eq!(
            parse_lookup(stdout.as_bytes(), &requests),
            Err("1 answers to 2 requests".to_owned())
        );
    }

    #[test]
    fn parse_lookup_refuses_a_mode_that_is_not_octal() {
        let requests = vec![format!("{ID}:a").into_bytes()];
        let stdout = format!("{ID} blob 100648\0");
        assert_eq!(
            parse_lookup(stdout.as_bytes(), &requests),
            Err("answer 0: a mode that is not octal".to_owned())
        );
    }

    /// A signed mode, which `u32`'s parser would take, is not octal.
    #[test]
    fn parse_lookup_refuses_a_signed_mode() {
        let requests = vec![format!("{ID}:a").into_bytes()];
        let stdout = format!("{ID} blob +100644\0");
        assert_eq!(
            parse_lookup(stdout.as_bytes(), &requests),
            Err("answer 0: a mode that is not octal".to_owned())
        );
    }
}
