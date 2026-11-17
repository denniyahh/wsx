//! The model's refs, listed through git, and peeling object ids to the
//! commits git names for them; callers never hold a gix type.
//!
//! Model refs are listed through `git for-each-ref` (`list_refs`), never
//! read by name, and each caller compares a record's exact name with the ref
//! it wants. An id is peeled by decoding its hex (`peel_commit`), for the
//! object work later phases do. So no read here resolves a name through
//! git's DWIM ladder or passes a name or an id to gix's revspec parser, which
//! reads a name shaped like `describe` output as the object its hex prefixes
//! (`clippy.toml` bans that parser;
//! `banned_gix_revspec_parsers_read_a_describe_shaped_ref_as_another_object`
//! in `repo.rs` pins the ban). `refuse_replace_refs` guards gix's object
//! reads.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;

use gix::bstr::ByteSlice;

use super::repo::Repo;
use super::{Error, Git, NO_LAZY_FETCH, ObjectId, shown};

/// One ref git listed: its full name, the object it ends at, and, for a
/// symbolic ref, the full ref its chain ends at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefRecord {
    /// The ref's full name, as git printed it.
    pub name: Vec<u8>,
    /// `%(objectname)`: the object the ref, followed to its end, names.
    pub id: ObjectId,
    /// `%(symref)`: the full ref a symbolic ref's chain ends at, or `None`
    /// for a ref that holds an object id.
    pub symref: Option<Vec<u8>>,
}

/// The config key gix reads its replace base from; git does not read it.
const REPLACE_BASE_KEY: &str = "gitoxide.objects.replaceRefBase";

/// The variable git reads its replace base from, and gix maps onto
/// `REPLACE_BASE_KEY`.
pub(super) const REPLACE_BASE_VAR: &str = "GIT_REPLACE_REF_BASE";

/// The replace base when neither `REPLACE_BASE_VAR` nor `REPLACE_BASE_KEY`
/// names one, in git and in gix.
const DEFAULT_REPLACE_BASE: &[u8] = b"refs/replace/";

/// Whether `name` is a valid branch name, as `git check-ref-format --branch`
/// decides: `refs/heads/<name>` is a valid full ref name, and `name` is
/// neither `HEAD` nor starts with `-`. A caller reads the branch only as that
/// one full ref.
///
/// gix's full-name check alone agrees with git on every name tried except
/// `HEAD` and a leading `-`, which gix accepts and git refuses, so those two
/// checks are added here. Measured on git 2.56.0 by
/// `branch_names_agree_with_git`: git accepts `develop`, `workspace/dev`,
/// `a/b`, `refs/heads/x` and a name holding the byte E9, and refuses
/// `HEAD`, `-x`, `a..b`, `a b`, `a.lock`, `.a`, `a/` and the empty name.
pub fn is_branch_name(name: &[u8]) -> bool {
    if name == b"HEAD" || name.starts_with(b"-") {
        return false;
    }
    let full = [b"refs/heads/".as_slice(), name].concat();
    gix::refs::FullName::try_from(full.as_bstr()).is_ok()
}

/// `bytes` for a message naming a ref.
fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Refuses `full` unless it is a valid full ref name, with gix's check, the
/// one gix applies to a ref name it reads. The error is `Error::Gix` with `what`
/// `read <full>`, so a name that cannot be a ref fails where it is read.
pub fn check_full_ref_name(full: &[u8]) -> Result<(), Error> {
    gix::refs::FullName::try_from(full.as_bstr())
        .map(|_| ())
        .map_err(|err| Error::Gix {
            what: format!("read {}", lossy(full)),
            message: err.to_string(),
        })
}

/// The format of every ref listing: name, object and symref target, each
/// field ended by NUL except the last. A ref name holds neither NUL nor LF
/// (git skips a ref whose name is not valid, F9), so a record is one line.
/// No `%(objecttype)`: it makes one ref naming a missing object fail the
/// whole listing (F6).
const LISTING_FORMAT: &str = "--format=%(refname)%00%(objectname)%00%(symref)";

/// The records of a `LISTING_FORMAT` listing. Empty stdout is no record;
/// otherwise stdout must end in LF and each line must be three NUL-separated
/// fields, a non-empty name, a full hex id and a target that is empty for a
/// ref holding an id. Anything else is `Err` naming the line, C-quoted.
fn parse_ref_listing(stdout: &[u8]) -> Result<Vec<RefRecord>, String> {
    if stdout.is_empty() {
        return Ok(Vec::new());
    }
    let Some(body) = stdout.strip_suffix(b"\n") else {
        return Err("the listing does not end in a newline".to_owned());
    };
    body.split(|byte| *byte == b'\n')
        .map(|line| {
            let refused = || format!("a record that is not name, id and symref: {}", shown(line));
            let fields: Vec<&[u8]> = line.split(|byte| *byte == 0).collect();
            let [name, id, symref] = fields.as_slice() else {
                return Err(refused());
            };
            if name.is_empty() {
                return Err(refused());
            }
            Ok(RefRecord {
                name: name.to_vec(),
                id: ObjectId::from_hex(id).ok_or_else(refused)?,
                symref: (!symref.is_empty()).then(|| symref.to_vec()),
            })
        })
        .collect()
}

impl Repo {
    /// The refs `git for-each-ref` lists for `patterns`, in one child run
    /// through `git` in the worktree (the git dir of a bare repository),
    /// `--no-lazy-fetch` first. Each pattern is one argument, its bytes as
    /// they are. With no pattern nothing is spawned and nothing is listed,
    /// since `for-each-ref` with no pattern lists every ref.
    ///
    /// The listing is a superset of the patterns: a pattern matches as a glob
    /// or as a prefix up to a slash, so `refs/remotes/origin/develop` also
    /// lists `refs/remotes/origin/develop/x` (F8). It skips what git skips:
    /// a missing, dangling, corrupt or badly named ref, a fifo, a symlink
    /// git resolves only by name, a chain too deep or a cycle (F7, F9). So a
    /// caller compares each record's exact name with the name it wants, and
    /// reads no record as present for having matched a pattern.
    ///
    /// A failed child is `Error::GitFailed`; output that does not parse is
    /// `Error::GitBadOutput`, never fewer records.
    pub fn list_refs(&self, git: &Git, patterns: &[Vec<u8>]) -> Result<Vec<RefRecord>, Error> {
        if patterns.is_empty() {
            return Ok(Vec::new());
        }
        let mut argv: Vec<OsString> = [NO_LAZY_FETCH, "for-each-ref", LISTING_FORMAT]
            .into_iter()
            .map(OsString::from)
            .collect();
        argv.extend(
            patterns
                .iter()
                .map(|pattern| OsStr::from_bytes(pattern).to_os_string()),
        );
        let cwd = self.worktree().unwrap_or(self.git_dir());
        let stdout = git.run(cwd, &argv)?;
        parse_ref_listing(&stdout).map_err(|problem| Error::GitBadOutput { argv, problem })
    }

    /// The commit `id` names, peeling annotated tags as git's
    /// `<id>^{commit}` does. No ref is read and no revspec is parsed: `id`
    /// is decoded as hex, so a name or a short prefix is refused.
    pub fn peel_commit(&self, id: &ObjectId) -> Result<ObjectId, Error> {
        self.peeled_commit(id)
            .map(|commit| ObjectId::from_gix(&commit.id))
    }

    /// The commit `id` peels to, as gix's commit. A decode failure is
    /// `ObjectId::to_gix`'s error; a missing object, or one that does not
    /// peel to a commit, is `Error::Gix` naming `id`.
    fn peeled_commit(&self, id: &ObjectId) -> Result<gix::Commit<'_>, Error> {
        let gix_id = id.to_gix(self.repo.object_hash())?;
        self.repo
            .find_object(gix_id)
            .map_err(|err| err.to_string())
            .and_then(|object| object.peel_to_commit().map_err(|err| err.to_string()))
            .map_err(|message| Error::Gix {
                what: format!("peel {id} to a commit"),
                message,
            })
    }

    /// Refuses every replace ref, whatever its shape, and every replace base
    /// other than `refs/replace/`, so no object is read through gix's
    /// replacement map while git could apply a replacement differently.
    ///
    /// Checked in this order, each reading only what it names:
    /// 1. `GIT_REPLACE_REF_BASE` in the process environment set to anything
    ///    but `refs/replace/` is `Error::ReplaceBaseSet`
    ///    (`refuse_replace_base_env`, which `Repo::discover` also runs before
    ///    gix opens the repository).
    /// 2. `gitoxide.objects.replaceRefBase` in gix's config snapshot, onto
    ///    which gix also maps that variable, set to anything but
    ///    `refs/replace/` is `Error::ReplaceBaseSet`, whether or not any ref
    ///    is under it. After these two steps both readers' base is
    ///    `refs/replace/`.
    /// 3. Any entry in the common dir's `refs/replace` directory is
    ///    `Error::ReplaceRefsPresent` naming the directory.
    /// 4. When the git dir is a linked worktree's, not the common dir, any
    ///    entry in the git dir's own `refs/replace` directory is
    ///    `Error::ReplaceRefsPresent` naming that directory.
    /// 5. The text `refs/replace/` anywhere in the common dir's
    ///    `packed-refs` is `Error::ReplaceRefsPresent` naming the file.
    ///
    /// No ref name is parsed and no `packed-refs` record is decoded: presence
    /// alone refuses, so no difference in how git and gix name, sort, follow
    /// or decode replace refs can change the bytes a read returns. A store
    /// that is missing holds nothing; any other failure to read one is
    /// `Error::ReplaceRefsUnreadable`. Every error returned here is one of the
    /// three refusals `Error::is_replacement_refusal` lists.
    ///
    /// A linked worktree's own git dir is read too: git reads only
    /// `refs/worktree/`, `refs/bisect/` and `refs/rewritten/` from it, while
    /// gix lists loose refs there for every prefix and builds its replacement
    /// map from them, so an entry in that dir's `refs/replace` is refused
    /// although git ignores it. Neither reader takes a replace ref from that
    /// dir's `packed-refs`, so it is not scanned.
    ///
    /// It over-refuses where neither reader would apply a replacement:
    /// replace refs turned off by `core.useReplaceRefs=false` or
    /// `GIT_NO_REPLACE_OBJECTS`; entries git skips, such as a `.lock` file, a
    /// name git calls bad, a dangling symlink, a fifo or an empty
    /// subdirectory; a ref outside the base whose name holds `refs/replace/`
    /// past its start; and `gitoxide.objects.replaceRefBase` set with nothing
    /// under it.
    ///
    /// gix loads its replacement map when the `Repo` is opened, and the
    /// stores are read later, so a replace ref added or removed in between,
    /// during one run, can make the map the reads use and the stores checked
    /// here disagree.
    pub fn refuse_replace_refs(&self) -> Result<(), Error> {
        refuse_replace_base_env()?;
        if let Some(value) = self.repo.config_snapshot().string(REPLACE_BASE_KEY)
            && value.as_bytes() != DEFAULT_REPLACE_BASE
        {
            return Err(Error::ReplaceBaseSet {
                name: REPLACE_BASE_KEY,
                value: value.to_vec(),
            });
        }
        let replace_dir = self.common_dir().join("refs/replace");
        if holds_an_entry(&replace_dir)? {
            return Err(Error::ReplaceRefsPresent {
                path: replace_dir,
                packed: false,
            });
        }
        if self.git_dir() != self.common_dir() {
            let private_dir = self.git_dir().join("refs/replace");
            if holds_an_entry(&private_dir)? {
                return Err(Error::ReplaceRefsPresent {
                    path: private_dir,
                    packed: false,
                });
            }
        }
        let packed_refs = self.common_dir().join("packed-refs");
        if packed_refs_name_the_base(&packed_refs)? {
            return Err(Error::ReplaceRefsPresent {
                path: packed_refs,
                packed: true,
            });
        }
        Ok(())
    }
}

/// Refuses `GIT_REPLACE_REF_BASE` in the process environment set to anything
/// but `refs/replace/`, git's default, as `Error::ReplaceBaseSet` naming the
/// variable and its value: git takes replace refs from under it, and gix's
/// open fails on some values before any ref is read.
pub fn refuse_replace_base_env() -> Result<(), Error> {
    match std::env::var_os(REPLACE_BASE_VAR).map(OsStringExt::into_vec) {
        Some(value) if value != DEFAULT_REPLACE_BASE => Err(Error::ReplaceBaseSet {
            name: REPLACE_BASE_VAR,
            value,
        }),
        _ => Ok(()),
    }
}

/// Whether the directory `dir` holds any entry, of any kind. A missing `dir`
/// holds none; any other failure to list it, a `dir` that is not a directory
/// included, is `Error::ReplaceRefsUnreadable` naming it.
fn holds_an_entry(dir: &Path) -> Result<bool, Error> {
    let unreadable = |source| Error::ReplaceRefsUnreadable {
        path: dir.to_path_buf(),
        source,
    };
    match std::fs::read_dir(dir) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(unreadable(err)),
        Ok(mut entries) => match entries.next() {
            None => Ok(false),
            Some(entry) => entry.map(|_| true).map_err(unreadable),
        },
    }
}

/// Whether the file `path` holds the text `refs/replace/` anywhere, found by
/// a byte search rather than a record parser. A missing `path` holds nothing;
/// any other failure to read it is `Error::ReplaceRefsUnreadable` naming it.
fn packed_refs_name_the_base(path: &Path) -> Result<bool, Error> {
    match std::fs::read(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(Error::ReplaceRefsUnreadable {
            path: path.to_path_buf(),
            source,
        }),
        Ok(bytes) => Ok(bytes.find(DEFAULT_REPLACE_BASE).is_some()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::process::Output;

    use gix::bstr::{BStr, BString};

    use super::super::testing::{self, Scratch, T0, TICK, commit_all, git_ok, write};
    use super::super::{Error, Git, ObjectId};
    use super::{RefRecord, Repo, is_branch_name, packed_refs_name_the_base, parse_ref_listing};

    /// `<root>/repo` with `a.txt` and `sub/b.txt` committed on `main`, the
    /// annotated tag `v1` of that commit, `v1nest` of `v1` and `vtree` of
    /// the commit's tree, each checked by git to be a tag object.
    fn tagged_repo(scratch: &Scratch) -> PathBuf {
        let repo_dir = scratch.root.join("repo");
        std::fs::create_dir_all(&repo_dir)
            .unwrap_or_else(|err| panic!("create {}: {err}", repo_dir.display()));
        let ok =
            |args: &[&str]| git_ok(scratch, &repo_dir, args).unwrap_or_else(|err| panic!("{err}"));
        ok(&["init", "-q", "-b", "main"]);
        write(&repo_dir.join("a.txt"), b"a\n").expect("write a.txt");
        write(&repo_dir.join("sub/b.txt"), b"b\n").expect("write sub/b.txt");
        commit_all(scratch, &repo_dir, "first", T0).expect("commit");
        ok(&["tag", "-a", "v1", "-m", "v1", "HEAD"]);
        ok(&["tag", "-a", "v1nest", "-m", "v1nest", "v1"]);
        ok(&["tag", "-a", "vtree", "-m", "vtree", "HEAD^{tree}"]);
        for tag in ["v1", "v1nest", "vtree"] {
            let kind = ok(&["cat-file", "-t", tag]);
            assert_eq!(
                String::from_utf8_lossy(&kind).trim_end(),
                "tag",
                "{tag} must be an annotated tag object"
            );
        }
        repo_dir
    }

    /// The object id git names for `rev` in `dir`, from
    /// `testing::object_id` (`git rev-parse --verify`), never from gix.
    fn id(scratch: &Scratch, dir: &Path, rev: &str) -> ObjectId {
        testing::object_id(scratch, dir, rev).unwrap_or_else(|err| panic!("{err}"))
    }

    /// git's peel of `hex` to a commit, `git rev-parse --verify
    /// <hex>^{commit}` in `dir`, whatever its exit status.
    fn git_peel(scratch: &Scratch, dir: &Path, hex: &str) -> Output {
        let rev = format!("{hex}^{{commit}}");
        testing::git(scratch, dir, ["rev-parse", "--verify", rev.as_str()])
            .unwrap_or_else(|err| panic!("{err}"))
    }

    /// The commit git peels `hex` to in `dir`, as hex; fails the test,
    /// printing git's stderr, when git refuses.
    fn git_peeled(scratch: &Scratch, dir: &Path, hex: &str) -> String {
        let output = git_peel(scratch, dir, hex);
        assert_eq!(
            output.status.code(),
            Some(0),
            "git rev-parse --verify {hex}^{{commit}} refused: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned()
    }

    /// git refuses to peel `id` to a commit in `dir`, and so does
    /// `peel_commit`, as `Error::Gix` rendering `peel <hex> to a commit: `
    /// followed by gix's message.
    fn assert_refused_as_git_does(scratch: &Scratch, dir: &Path, repo: &Repo, id: &ObjectId) {
        let hex = id.to_string();
        let git = git_peel(scratch, dir, &hex);
        assert_ne!(
            git.status.code(),
            Some(0),
            "git rev-parse --verify {hex}^{{commit}} accepted it, printing {:?}",
            String::from_utf8_lossy(&git.stdout).trim_end()
        );
        let prefix = format!("peel {hex} to a commit: ");
        match repo.peel_commit(id) {
            Err(err @ Error::Gix { .. }) => {
                let rendered = err.to_string();
                let message = rendered.strip_prefix(prefix.as_str());
                assert!(
                    message.is_some_and(|message| !message.is_empty()),
                    "peel_commit({hex}) rendered {rendered:?}, not {prefix:?} and gix's message"
                );
            }
            other => panic!(
                "peel_commit({hex}): expected Error::Gix, got {other:?}; git refused with {}",
                String::from_utf8_lossy(&git.stderr).trim_end()
            ),
        }
    }

    /// `<root>/<name>`, made with `git init --object-format=<format>`, with
    /// `a.txt` committed on `main` at `T0` and the annotated tag `v1` of
    /// that commit; canonical.
    fn repo_with_format(scratch: &Scratch, name: &str, format: &str) -> PathBuf {
        let repo_dir = scratch.root.join(name);
        let repo_arg = repo_dir.to_str().expect("UTF-8 scratch path");
        let format_arg = format!("--object-format={format}");
        git_ok(
            scratch,
            &scratch.root,
            ["init", "-q", "-b", "main", format_arg.as_str(), repo_arg],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        write(&repo_dir.join("a.txt"), b"a\n").expect("write a.txt");
        commit_all(scratch, &repo_dir, "first", T0).expect("commit");
        git_ok(scratch, &repo_dir, ["tag", "-a", "v1", "-m", "v1", "HEAD"])
            .unwrap_or_else(|err| panic!("{err}"));
        repo_dir
            .canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize {}: {err}", repo_dir.display()))
    }

    /// The rendering of `peel_commit(id)`'s error; fails the test when it
    /// succeeds.
    fn peel_refusal(repo: &Repo, id: &ObjectId) -> String {
        match repo.peel_commit(id) {
            Err(err) => err.to_string(),
            Ok(peeled) => panic!("peel_commit({id:?}) accepted it, peeling to {peeled}"),
        }
    }

    #[test]
    fn peel_commit_of_a_commit_is_that_commit() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-commit").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let head = id(&scratch, &repo_dir, "HEAD");
        let from_git = git_peeled(&scratch, &repo_dir, &head.to_string());
        assert_eq!(
            from_git,
            head.to_string(),
            "git peels commit {head} to itself"
        );
        let peeled = repo
            .peel_commit(&head)
            .unwrap_or_else(|err| panic!("peel_commit({head}): {err}"));
        assert_eq!(
            peeled.to_string(),
            from_git,
            "peel_commit({head}) against git's {from_git}"
        );
    }

    #[test]
    fn peel_commit_peels_an_annotated_tag_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-tag").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let tag = id(&scratch, &repo_dir, "v1");
        let from_git = git_peeled(&scratch, &repo_dir, &tag.to_string());
        let peeled = repo
            .peel_commit(&tag)
            .unwrap_or_else(|err| panic!("peel_commit({tag}): {err}"));
        assert_eq!(
            peeled.to_string(),
            from_git,
            "peel_commit of tag v1 {tag} against git's {from_git}"
        );
        assert_ne!(peeled, tag, "peel_commit returned the tag v1's own id");
    }

    #[test]
    fn peel_commit_peels_a_nested_tag_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-nested-tag").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let outer = id(&scratch, &repo_dir, "v1nest");
        let inner = id(&scratch, &repo_dir, "v1");
        let from_git = git_peeled(&scratch, &repo_dir, &outer.to_string());
        let peeled = repo
            .peel_commit(&outer)
            .unwrap_or_else(|err| panic!("peel_commit({outer}): {err}"));
        assert_eq!(
            peeled.to_string(),
            from_git,
            "peel_commit of tag v1nest {outer} against git's {from_git}"
        );
        assert_ne!(
            peeled, outer,
            "peel_commit returned the tag v1nest's own id"
        );
        assert_ne!(
            peeled, inner,
            "peel_commit stopped at the inner tag v1 {inner}"
        );
    }

    #[test]
    fn peel_commit_refuses_a_tree_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-tree").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let tree = id(&scratch, &repo_dir, "HEAD^{tree}");
        assert_refused_as_git_does(&scratch, &repo_dir, &repo, &tree);
    }

    #[test]
    fn peel_commit_refuses_a_blob_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-blob").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let blob = id(&scratch, &repo_dir, "HEAD:a.txt");
        assert_refused_as_git_does(&scratch, &repo_dir, &repo, &blob);
    }

    #[test]
    fn peel_commit_refuses_a_tag_of_a_tree_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-tree-tag").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let tag = id(&scratch, &repo_dir, "vtree");
        assert_refused_as_git_does(&scratch, &repo_dir, &repo, &tag);
    }

    #[test]
    fn peel_commit_refuses_a_missing_object_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-missing").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let other_dir = scratch.root.join("other");
        std::fs::create_dir_all(&other_dir)
            .unwrap_or_else(|err| panic!("create {}: {err}", other_dir.display()));
        git_ok(&scratch, &other_dir, ["init", "-q", "-b", "main"]).expect("git init other");
        write(&other_dir.join("x.txt"), b"x\n").expect("write x.txt");
        commit_all(&scratch, &other_dir, "other", T0 + TICK).expect("commit in other");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let foreign = id(&scratch, &other_dir, "HEAD");
        let hex = foreign.to_string();
        let present = testing::git(&scratch, &repo_dir, ["cat-file", "-e", hex.as_str()])
            .unwrap_or_else(|err| panic!("{err}"));
        assert_ne!(
            present.status.code(),
            Some(0),
            "{hex} from <root>/other exists in <root>/repo"
        );
        assert_refused_as_git_does(&scratch, &repo_dir, &repo, &foreign);
    }

    #[test]
    fn peel_commit_refuses_a_sha1_id_in_a_sha256_repository() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-sha1-in-sha256").expect("create scratch");
        let sha1_dir = repo_with_format(&scratch, "sha1", "sha1");
        let sha256_dir = repo_with_format(&scratch, "sha256", "sha256");
        let repo = Repo::discover(&sha256_dir).expect("discover the SHA-256 fixture");

        let sha1 = id(&scratch, &sha1_dir, "HEAD");
        let git = git_peel(&scratch, &sha256_dir, &sha1.to_string());
        assert_ne!(
            git.status.code(),
            Some(0),
            "git peeled the SHA-1 id {sha1} in a SHA-256 repository to {:?}",
            String::from_utf8_lossy(&git.stdout).trim_end()
        );
        assert_eq!(
            peel_refusal(&repo, &sha1),
            format!("read object id {sha1}: a sha1 id in a sha256 repository")
        );
    }

    #[test]
    fn peel_commit_refuses_a_sha256_id_in_a_sha1_repository() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-sha256-in-sha1").expect("create scratch");
        let sha1_dir = repo_with_format(&scratch, "sha1", "sha1");
        let sha256_dir = repo_with_format(&scratch, "sha256", "sha256");
        let repo = Repo::discover(&sha1_dir).expect("discover the SHA-1 fixture");

        let sha256 = id(&scratch, &sha256_dir, "HEAD");
        let git = git_peel(&scratch, &sha1_dir, &sha256.to_string());
        assert_ne!(
            git.status.code(),
            Some(0),
            "git peeled the SHA-256 id {sha256} in a SHA-1 repository to {:?}",
            String::from_utf8_lossy(&git.stdout).trim_end()
        );
        assert_eq!(
            peel_refusal(&repo, &sha256),
            format!("read object id {sha256}: a sha256 id in a sha1 repository")
        );
    }

    #[test]
    fn peel_commit_peels_a_tag_in_a_sha256_repository() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-sha256-tag").expect("create scratch");
        let sha256_dir = repo_with_format(&scratch, "sha256", "sha256");
        let repo = Repo::discover(&sha256_dir).expect("discover the SHA-256 fixture");

        let tag = id(&scratch, &sha256_dir, "v1");
        let from_git = git_peeled(&scratch, &sha256_dir, &tag.to_string());
        let peeled = repo
            .peel_commit(&tag)
            .unwrap_or_else(|err| panic!("peel_commit({tag}): {err}"));
        assert_eq!(
            peeled.to_string(),
            from_git,
            "peel_commit of tag v1 {tag} against git's {from_git}"
        );
        assert_eq!(
            from_git.len(),
            64,
            "git's peel {from_git} is not SHA-256 length"
        );
        assert_ne!(peeled, tag, "peel_commit returned the tag v1's own id");
    }

    // The inputs below are not ids git printed, so they are built directly.
    // git resolves each of them; `peel_commit` refuses each at the hex
    // decode, so a name has no path to a commit through the id boundary.

    #[test]
    fn peel_commit_refuses_a_branch_name() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-branch-name").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let head = id(&scratch, &repo_dir, "HEAD");
        assert_eq!(
            git_peeled(&scratch, &repo_dir, "main"),
            head.to_string(),
            "git peels the branch name main to HEAD"
        );
        assert_eq!(
            peel_refusal(&repo, &ObjectId("main".to_owned())),
            "read object id main: A hash sized 4 hexadecimal characters is invalid"
        );
    }

    #[test]
    fn peel_commit_refuses_an_abbreviated_id() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-abbreviated").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let head = id(&scratch, &repo_dir, "HEAD").to_string();
        let short = head.get(..7).expect("HEAD's id is longer than 7");
        assert_eq!(
            git_peeled(&scratch, &repo_dir, short),
            head,
            "git peels the prefix {short} to HEAD"
        );
        assert_eq!(
            peel_refusal(&repo, &ObjectId(short.to_owned())),
            format!("read object id {short}: A hash sized 7 hexadecimal characters is invalid")
        );
    }

    #[test]
    fn peel_commit_refuses_an_empty_id() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-empty").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        assert_eq!(
            peel_refusal(&repo, &ObjectId(String::new())),
            "read object id : A hash sized 0 hexadecimal characters is invalid"
        );
    }

    #[test]
    fn peel_commit_refuses_a_describe_shaped_ref_name() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-describe-shaped").expect("create scratch");
        let fixture = testing::describe_shaped_ref(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let repo = Repo::discover(&fixture.repo).expect("discover fixture");

        assert_eq!(
            git_peeled(&scratch, &fixture.repo, &fixture.name),
            fixture.one,
            "git peels {} to ONE",
            fixture.name
        );
        assert_eq!(
            peel_refusal(&repo, &ObjectId(fixture.name.clone())),
            format!(
                "read object id {}: A hash sized {} hexadecimal characters is invalid",
                fixture.name,
                fixture.name.len()
            )
        );
    }

    /// The id `git show-ref --verify <full>` prints in `dir`, or `None` when
    /// git finds no ref of exactly that name. Not `git rev-parse`, whose DWIM
    /// rules would find a ref under another name.
    fn show_ref(scratch: &Scratch, dir: &Path, full: &str) -> Option<String> {
        let output = testing::git(scratch, dir, ["show-ref", "--verify", full])
            .unwrap_or_else(|err| panic!("{err}"));
        if output.status.code() != Some(0) {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let id = stdout
            .split_whitespace()
            .next()
            .unwrap_or_else(|| panic!("git show-ref --verify {full} exited 0 and printed no id"));
        Some(id.to_owned())
    }

    /// `is_branch_name` agrees with `git check-ref-format --branch` on each
    /// name in the table, which git accepts some of and refuses others of,
    /// so a check that always answers one way fails.
    #[test]
    fn branch_names_agree_with_git() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("branch-names").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let names: [&[u8]; 13] = [
            b"develop",
            b"workspace/dev",
            b"a/b",
            b"HEAD",
            b"-x",
            b"a..b",
            b"a b",
            b"a.lock",
            b".a",
            b"a/",
            b"",
            b"\xE9",
            b"refs/heads/x",
        ];
        let (mut accepted, mut refused) = (0, 0);
        for name in names {
            let output = testing::git(
                &scratch,
                &repo_dir,
                [
                    OsStr::new("check-ref-format"),
                    OsStr::new("--branch"),
                    OsStr::from_bytes(name),
                ],
            )
            .unwrap_or_else(|err| panic!("{err}"));
            let shown = BStr::new(name);
            let git_accepts = match output.status.code() {
                Some(0) => true,
                Some(128) => false,
                other => panic!(
                    "git check-ref-format --branch {shown:?} exited {other:?}: {}",
                    String::from_utf8_lossy(&output.stderr).trim_end()
                ),
            };
            assert_eq!(
                is_branch_name(name),
                git_accepts,
                "is_branch_name({shown:?}) against git check-ref-format --branch"
            );
            if git_accepts {
                accepted += 1;
            } else {
                refused += 1;
            }
        }
        assert!(
            accepted > 0 && refused > 0,
            "git accepted {accepted} and refused {refused} of the table"
        );
    }

    #[test]
    fn listing_reads_a_describe_shaped_ref_as_that_ref() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("ref-commit-describe-shaped").expect("create scratch");
        let fixture = testing::describe_shaped_ref(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let prefix = fixture
            .other
            .get(..7)
            .expect("a hex id of at least 7 characters");
        let alias = format!("refs/heads/alias-1-g{prefix}");
        git_ok(
            &scratch,
            &fixture.repo,
            ["symbolic-ref", alias.as_str(), fixture.name.as_str()],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            show_ref(&scratch, &fixture.repo, &fixture.name).as_deref(),
            Some(fixture.one.as_str()),
            "git show-ref --verify {} prints ONE",
            fixture.name
        );
        assert_eq!(
            id(&scratch, &fixture.repo, &alias).to_string(),
            fixture.one,
            "git rev-parse --verify {alias} prints ONE"
        );
        let repo = Repo::discover(&fixture.repo).expect("discover the fixture");

        #[expect(
            clippy::disallowed_methods,
            reason = "the control that this fixture reaches gix's describe-output parsing, which reads the name as the object its hex prefixes"
        )]
        let parsed = repo
            .repo
            .rev_parse_single(BStr::new(fixture.name.as_bytes()))
            .map(|found| found.to_string());
        match parsed {
            Ok(parsed) => assert_eq!(
                parsed, fixture.other,
                "gix's rev_parse_single({}) against Y",
                fixture.name
            ),
            Err(err) => panic!("gix's rev_parse_single({}): {err}", fixture.name),
        }

        let git = Git::for_repo(scratch.env.clone(), &repo);
        let listing = repo
            .list_refs(
                &git,
                &[fixture.name.as_bytes().to_vec(), alias.as_bytes().to_vec()],
            )
            .unwrap_or_else(|err| panic!("list_refs({}, {alias}): {err}", fixture.name));
        let listed_id = |full: &str| {
            listing
                .iter()
                .find(|record| record.name == full.as_bytes())
                .unwrap_or_else(|| panic!("list_refs did not list {full}: {listing:?}"))
                .id
                .to_string()
        };
        assert_eq!(
            listed_id(&fixture.name),
            fixture.one,
            "list_refs({}) against ONE, which the ref holds",
            fixture.name
        );
        assert_eq!(
            listed_id(&alias),
            fixture.one,
            "list_refs({alias}), symbolic to {}, against ONE",
            fixture.name
        );
    }

    /// The prefixes workspace-branch candidates are listed under (D-21),
    /// each with its trailing slash.
    const CANDIDATE_PREFIXES: [&str; 2] = ["refs/heads/workspace/", "refs/heads/personal/"];

    /// The branches `listing_repo` adds: one level under each prefix, one
    /// nested two levels down, and names that only extend a prefix's text.
    const LISTING_BRANCHES: [&str; 8] = [
        "workspace/a",
        "workspace/nested/deep",
        "workspace-x",
        "workspaceY",
        "personal/workspace",
        "personal/workspace-backup",
        "personal-z",
        "notworkspace/q",
    ];

    /// `tagged_repo` with each of `LISTING_BRANCHES` made at `HEAD`.
    fn listing_repo(scratch: &Scratch) -> PathBuf {
        let repo_dir = tagged_repo(scratch);
        for branch in LISTING_BRANCHES {
            git_ok(scratch, &repo_dir, ["branch", branch]).unwrap_or_else(|err| panic!("{err}"));
        }
        repo_dir
    }

    /// `names` as a set of `BString`, so a failed comparison prints names.
    fn name_set<I: IntoIterator<Item = Vec<u8>>>(names: I) -> BTreeSet<BString> {
        names.into_iter().map(BString::from).collect()
    }

    /// The names `list_refs` lists in `repo` for `patterns`, in its
    /// order, through the scratch's git; fails the test on an error.
    fn list_names(scratch: &Scratch, repo: &Repo, patterns: &[String]) -> Vec<Vec<u8>> {
        let git = Git::for_repo(scratch.env.clone(), repo);
        let patterns: Vec<Vec<u8>> = patterns.iter().map(|p| p.as_bytes().to_vec()).collect();
        repo.list_refs(&git, &patterns)
            .unwrap_or_else(|err| panic!("list_refs({patterns:?}): {err}"))
            .into_iter()
            .map(|record| record.name)
            .collect()
    }

    /// The one-level glob `<prefix>*` of each prefix, the patterns
    /// `git_listed` passes to git.
    fn globs(prefixes: &[&str]) -> Vec<String> {
        prefixes.iter().map(|prefix| format!("{prefix}*")).collect()
    }

    /// What `list_refs` lists in `repo` for the globs of
    /// `CANDIDATE_PREFIXES`, as a set.
    fn listed(scratch: &Scratch, repo: &Repo) -> BTreeSet<BString> {
        name_set(list_names(scratch, repo, &globs(&CANDIDATE_PREFIXES)))
    }

    /// The full names `git for-each-ref 'refs/heads/workspace/*'
    /// 'refs/heads/personal/*'` lists in `dir`, as raw bytes. for-each-ref
    /// has no `-z`, so each name is formatted with a NUL after it; a ref name
    /// holds neither NUL nor LF, so every line must be one name and its NUL.
    fn git_listed(scratch: &Scratch, dir: &Path) -> BTreeSet<BString> {
        let mut args = vec![
            "for-each-ref".to_owned(),
            "--format=%(refname)%00".to_owned(),
        ];
        args.extend(globs(&CANDIDATE_PREFIXES));
        let stdout = git_ok(scratch, dir, &args).unwrap_or_else(|err| panic!("{err}"));
        name_set(testing::records(&stdout, b'\n').into_iter().map(
            |record| match record.split_last() {
                Some((0, name)) => name.to_vec(),
                _ => panic!(
                    "git for-each-ref printed a line not ending in NUL: {:?}",
                    BStr::new(&record)
                ),
            },
        ))
    }

    /// A3: the listing is git's one-level glob. Of the fixture's branches
    /// only `workspace/a`, `personal/workspace` and
    /// `personal/workspace-backup` are directly under a prefix; the nested
    /// `workspace/nested/deep` and the names that only extend a prefix's
    /// text are not, for git as for `list_refs` over the same globs.
    #[test]
    fn listing_matches_for_each_ref() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-for-each-ref").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        let from_git = git_listed(&scratch, &repo_dir);
        assert_eq!(
            from_git,
            name_set([
                b"refs/heads/personal/workspace".to_vec(),
                b"refs/heads/personal/workspace-backup".to_vec(),
                b"refs/heads/workspace/a".to_vec(),
            ]),
            "git for-each-ref's one-level globs over the fixture"
        );
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");

        assert_eq!(
            listed(&scratch, &repo),
            from_git,
            "list_refs against git for-each-ref"
        );
    }

    /// The control for the differential: a new branch under a prefix changes
    /// git's answer and `list_refs`'s alike, so the two do not
    /// agree only because the answer is constant.
    #[test]
    fn listing_changes_with_a_new_branch() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-new-branch").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        let git_before = git_listed(&scratch, &repo_dir);
        let before = listed(
            &scratch,
            &Repo::discover(&repo_dir).expect("discover the fixture"),
        );

        git_ok(&scratch, &repo_dir, ["branch", "workspace/b"])
            .unwrap_or_else(|err| panic!("{err}"));
        let git_after = git_listed(&scratch, &repo_dir);
        let after = listed(
            &scratch,
            &Repo::discover(&repo_dir).expect("discover the fixture"),
        );

        assert_ne!(git_after, git_before, "git for-each-ref before and after");
        assert_ne!(after, before, "list_refs before and after");
        assert_eq!(after, git_after, "list_refs against git, after");
        assert!(
            after.contains(&BString::from("refs/heads/workspace/b")),
            "list_refs after the branch was made: {after:?}"
        );
    }

    /// Packed and loose refs are listed together: after `git pack-refs
    /// --all`, which leaves `workspace/a` only in `packed-refs`, and one new
    /// loose branch, the listing still equals git's and holds both.
    #[test]
    fn listing_survives_packing() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-packed").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        let ok =
            |args: &[&str]| git_ok(&scratch, &repo_dir, args).unwrap_or_else(|err| panic!("{err}"));
        ok(&["pack-refs", "--all"]);
        let git_dir = repo_dir.join(".git");
        assert!(
            !git_dir.join("refs/heads/workspace/a").exists(),
            "git pack-refs --all left refs/heads/workspace/a loose"
        );
        ok(&["branch", "workspace/loose"]);
        assert!(
            git_dir.join("refs/heads/workspace/loose").is_file(),
            "git branch did not write refs/heads/workspace/loose loose"
        );
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");

        let names = listed(&scratch, &repo);
        assert_eq!(
            names,
            git_listed(&scratch, &repo_dir),
            "list_refs against git for-each-ref, packed and loose"
        );
        for full in ["refs/heads/workspace/a", "refs/heads/workspace/loose"] {
            assert!(
                names.contains(&BString::from(full)),
                "{full} is missing from {names:?}"
            );
        }
    }

    /// The names come back in byte order, whatever order they were made in:
    /// `C` sorts before `a`, as uppercase ASCII does.
    #[test]
    fn listing_is_sorted() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-sorted").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        for branch in ["workspace/b", "workspace/C"] {
            git_ok(&scratch, &repo_dir, ["branch", branch]).unwrap_or_else(|err| panic!("{err}"));
        }
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");

        let names = list_names(&scratch, &repo, &globs(&["refs/heads/workspace/"]));
        assert_eq!(
            names.iter().cloned().map(BString::from).collect::<Vec<_>>(),
            vec![
                BString::from("refs/heads/workspace/C"),
                BString::from("refs/heads/workspace/a"),
                BString::from("refs/heads/workspace/b"),
            ],
            "list_refs(refs/heads/workspace/*)"
        );
        assert!(
            names.windows(2).all(|pair| pair[0] < pair[1]),
            "not in byte order: {names:?}"
        );
    }

    /// A branch whose name holds the byte E9, not UTF-8, is listed with
    /// that byte, as git lists it.
    #[test]
    fn non_utf8_branch_is_listed_by_its_bytes() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-non-utf8").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        let branch: &[u8] = b"workspace/raw\xE9";
        git_ok(
            &scratch,
            &repo_dir,
            [OsStr::new("branch"), OsStr::from_bytes(branch)],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let full = BString::from([b"refs/heads/".as_slice(), branch].concat());
        let from_git = git_listed(&scratch, &repo_dir);
        assert!(
            from_git.contains(&full),
            "git for-each-ref does not list {full:?}: {from_git:?}"
        );
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");

        assert_eq!(
            listed(&scratch, &repo),
            from_git,
            "list_refs against git for-each-ref"
        );
    }

    /// D-35: a loose ref file holding garbage under a prefix is skipped by
    /// git, which warns and lists the rest, and by `list_refs`, which lists
    /// what git lists. The other prefix still lists.
    #[test]
    fn corrupt_ref_under_the_prefix_is_skipped_as_git_skips_it() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-listing-corrupt").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        let before = git_listed(&scratch, &repo_dir);
        write(
            &repo_dir.join(".git/refs/heads/workspace/bad"),
            b"garbage\n",
        )
        .expect("write .git/refs/heads/workspace/bad");
        assert_eq!(
            git_listed(&scratch, &repo_dir),
            before,
            "git for-each-ref skips the corrupt ref and lists the rest"
        );
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");

        assert_eq!(
            listed(&scratch, &repo),
            before,
            "list_refs beside the corrupt workspace ref"
        );
        assert_eq!(
            name_set(list_names(
                &scratch,
                &repo,
                &globs(&["refs/heads/personal/"])
            )),
            name_set([
                b"refs/heads/personal/workspace".to_vec(),
                b"refs/heads/personal/workspace-backup".to_vec(),
            ]),
            "list_refs(refs/heads/personal/*) beside the corrupt workspace ref"
        );
    }

    /// From a linked worktree the listing is the main worktree's, and git's
    /// from that worktree: branches are shared, not per worktree.
    #[test]
    fn linked_worktree_lists_the_same_branches() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-listing-worktree").expect("create scratch");
        let trees = testing::linked_worktrees(&scratch).unwrap_or_else(|err| panic!("{err}"));
        for branch in ["workspace/a", "personal/b", "workspace/n/deep"] {
            git_ok(&scratch, &trees.main, ["branch", branch]).unwrap_or_else(|err| panic!("{err}"));
        }
        let main = listed(
            &scratch,
            &Repo::discover(&trees.main).expect("discover the main worktree"),
        );
        assert_eq!(
            main,
            name_set([
                b"refs/heads/personal/b".to_vec(),
                b"refs/heads/workspace/a".to_vec(),
            ]),
            "list_refs from the main worktree"
        );

        let linked = listed(
            &scratch,
            &Repo::discover(&trees.wt1).expect("discover the linked worktree"),
        );
        assert_eq!(
            linked, main,
            "from the linked worktree against the main one"
        );
        assert_eq!(
            linked,
            git_listed(&scratch, &trees.wt1),
            "from the linked worktree against git for-each-ref there"
        );
    }

    /// A `packed-refs` that is a directory fails to read, and is refused as
    /// unreadable replace refs naming it, never read as holding none.
    #[test]
    fn packed_refs_that_fail_to_read_are_unreadable() {
        let scratch = Scratch::new("packed-refs-unreadable").expect("create scratch");
        match packed_refs_name_the_base(&scratch.root) {
            Err(Error::ReplaceRefsUnreadable { path, .. }) => {
                assert_eq!(path, scratch.root, "the path the refusal names");
            }
            other => panic!("expected Err(ReplaceRefsUnreadable), got {other:?}"),
        }
    }

    /// A SHA-1 id as git prints it.
    const SHA1_HEX: &str = "0123456789abcdef0123456789abcdef01234567";

    /// The `ObjectId` of `hex`; fails the test unless it is a full id.
    fn hex_id(hex: &str) -> ObjectId {
        ObjectId::from_hex(hex.as_bytes()).unwrap_or_else(|| panic!("{hex} is not a full id"))
    }

    /// A listing's fields are raw bytes: a name holding the byte E9 stays that
    /// byte, a direct ref has no symref, and a symbolic one names its target.
    #[test]
    fn parse_ref_listing_reads_raw_bytes() {
        let mut stdout = b"refs/heads/caf\xe9\0".to_vec();
        stdout.extend_from_slice(SHA1_HEX.as_bytes());
        stdout.extend_from_slice(b"\0\nrefs/remotes/origin/HEAD\0");
        stdout.extend_from_slice(SHA1_HEX.as_bytes());
        stdout.extend_from_slice(b"\0refs/remotes/origin/main\n");

        assert_eq!(
            parse_ref_listing(&stdout),
            Ok(vec![
                RefRecord {
                    name: b"refs/heads/caf\xe9".to_vec(),
                    id: hex_id(SHA1_HEX),
                    symref: None,
                },
                RefRecord {
                    name: b"refs/remotes/origin/HEAD".to_vec(),
                    id: hex_id(SHA1_HEX),
                    symref: Some(b"refs/remotes/origin/main".to_vec()),
                },
            ])
        );
    }

    /// A record with two fields, not three, is refused, naming the record,
    /// never read as a ref with no symref.
    #[test]
    fn parse_ref_listing_refuses_a_record_without_three_fields() {
        let stdout = format!("refs/heads/x\0{SHA1_HEX}\n");
        match parse_ref_listing(stdout.as_bytes()) {
            Err(problem) => assert!(
                problem.contains(&format!("refs/heads/x\\000{SHA1_HEX}")),
                "{problem:?} does not name the record"
            ),
            other => panic!("expected Err for a two-field record, got {other:?}"),
        }
    }

    /// With no pattern `list_refs` lists nothing and runs no git: a `Git`
    /// whose PATH holds no git still answers. Control: the same `Git` with a
    /// pattern fails to start git.
    #[test]
    fn list_refs_without_a_pattern_spawns_nothing() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-list-no-pattern").expect("create scratch");
        let repo_dir = repo_with_format(&scratch, "repo", "sha1");
        let empty = scratch.root.join("no-git");
        std::fs::create_dir_all(&empty)
            .unwrap_or_else(|err| panic!("create {}: {err}", empty.display()));
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");
        let git = Git::for_repo(vec![("PATH".into(), empty.into())], &repo);

        assert_eq!(
            repo.list_refs(&git, &[])
                .unwrap_or_else(|err| panic!("list_refs with no pattern: {err}")),
            Vec::<RefRecord>::new()
        );
        match repo.list_refs(&git, &[b"refs/heads/main".to_vec()]) {
            Err(Error::Io { .. }) => {}
            other => panic!("list_refs with a pattern and no git: expected Err(Io), got {other:?}"),
        }
    }
}
