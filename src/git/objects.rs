//! The model's object reads: the workspace tip, its manifest and the
//! integration tip, through one `git cat-file --batch` child.
//!
//! Every request names an id `git for-each-ref` printed, never a ref, so no
//! name is resolved here. The child runs with `--no-lazy-fetch`, so a
//! partial clone's missing blob is `missing`, never a fetch (D-09).

use std::ffi::OsString;

use super::refs::RootEntry;
use super::repo::Repo;
use super::{Error, Git, NO_LAZY_FETCH, ObjectId};

/// The batch format: id, type, size and the entry mode, the last empty for a
/// request that is not `<tree-ish>:<path>`. The default format has no mode
/// and reads a symlink as a blob (research G2).
const BATCH_FORMAT: &str = "--batch=%(objectname) %(objecttype) %(objectsize) %(objectmode)";

/// The tree entry mode of a gitlink, whose commit is rarely in this
/// repository, so git usually answers `missing` for it.
const GITLINK_MODE: &[u8] = b"160000";

/// What one `read_tips` batch answered, each part on its own, so the caller
/// raises them in its own order.
#[derive(Debug)]
pub struct TipObjects {
    /// The commit the workspace tip's id peels to.
    pub workspace_tip: Result<ObjectId, Error>,
    /// The entry the manifest name has in that commit's root tree.
    pub manifest: Result<RootEntry, Error>,
    /// The commit the integration rung's id peels to.
    pub integration_tip: Result<ObjectId, Error>,
}

/// One answer of `git cat-file --batch` in `BATCH_FORMAT`.
#[derive(Debug, PartialEq, Eq)]
enum BatchAnswer {
    /// git printed `<request> missing`.
    Missing,
    /// git printed the object: its id, type, mode (empty for a request that
    /// is not `<tree-ish>:<path>`) and bytes.
    Found {
        id: ObjectId,
        kind: Vec<u8>,
        mode: Vec<u8>,
        data: Vec<u8>,
    },
}

/// The `count` answers in `stdout`, in request order: answer i belongs to
/// request i, so the echoed request text is never parsed back. A header
/// ending in ` missing` is `Missing`; any other header is exactly four
/// space-separated fields (a full hex id, a type, a decimal size and a mode
/// that may be empty), followed by exactly that many bytes and one LF.
/// Anything else, a short read or a byte after the last answer included, is
/// `Err` naming the answer's index.
fn parse_batch(stdout: &[u8], count: usize) -> Result<Vec<BatchAnswer>, String> {
    let mut rest = stdout;
    let mut answers = Vec::with_capacity(count);
    for index in 0..count {
        let bad = |problem: &str| format!("answer {index}: {problem}");
        let line_end = rest
            .iter()
            .position(|&byte| byte == b'\n')
            .ok_or_else(|| bad("no header line"))?;
        let header = &rest[..line_end];
        rest = &rest[line_end + 1..];
        if header.ends_with(b" missing") {
            answers.push(BatchAnswer::Missing);
            continue;
        }
        let fields: Vec<&[u8]> = header.split(|&byte| byte == b' ').collect();
        let [id, kind, size, mode] = fields.as_slice() else {
            return Err(bad("a header that is not four fields"));
        };
        let id = ObjectId::from_hex(id).ok_or_else(|| bad("an id that is not full hex"))?;
        let size: usize = std::str::from_utf8(size)
            .ok()
            .filter(|text| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|text| text.parse().ok())
            .ok_or_else(|| bad("a size that is not a decimal number"))?;
        if rest.len() < size + 1 || rest[size] != b'\n' {
            return Err(bad("fewer bytes than its size, or no LF after them"));
        }
        answers.push(BatchAnswer::Found {
            id,
            kind: kind.to_vec(),
            mode: mode.to_vec(),
            data: rest[..size].to_vec(),
        });
        rest = &rest[size + 1..];
    }
    if rest.is_empty() {
        Ok(answers)
    } else {
        Err(format!(
            "{} bytes after answer {}",
            rest.len(),
            count.saturating_sub(1)
        ))
    }
}

/// The mode of the entry named exactly `name` in the raw tree `tree`, whose
/// entries are `<ascii mode> SP <name> NUL <id_len raw bytes>`, or `None`
/// when no entry has that name. A malformed entry is `Err`.
fn root_tree_mode(tree: &[u8], id_len: usize, name: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let mut rest = tree;
    while !rest.is_empty() {
        let space = rest
            .iter()
            .position(|&byte| byte == b' ')
            .ok_or("an entry without a space after its mode")?;
        let (mode, after_mode) = (&rest[..space], &rest[space + 1..]);
        let nul = after_mode
            .iter()
            .position(|&byte| byte == 0)
            .ok_or("an entry without a NUL after its name")?;
        let (entry_name, after_name) = (&after_mode[..nul], &after_mode[nul + 1..]);
        if mode.is_empty() || after_name.len() < id_len {
            return Err("an entry with an empty mode or a truncated id".to_owned());
        }
        if entry_name == name {
            return Ok(Some(mode.to_vec()));
        }
        rest = &after_name[id_len..];
    }
    Ok(None)
}

/// The commit a `<id>^{commit}` request named: `Missing` is
/// `Error::ObjectMissing` for `request`; an answer of another type cannot
/// come from a `^{commit}` peel, so it is `Error::GitBadOutput`.
fn peeled_commit(
    answer: BatchAnswer,
    request: &[u8],
    argv: &[OsString],
) -> Result<ObjectId, Error> {
    match answer {
        BatchAnswer::Missing => Err(Error::ObjectMissing {
            request: request.to_vec(),
        }),
        BatchAnswer::Found { id, kind, .. } if kind == b"commit" => Ok(id),
        BatchAnswer::Found { kind, .. } => Err(Error::GitBadOutput {
            argv: argv.to_vec(),
            problem: format!(
                "{} answered with a {}, not a commit",
                String::from_utf8_lossy(request),
                String::from_utf8_lossy(&kind)
            ),
        }),
    }
}

/// The manifest entry from the `<ws>:<name>` answer `entry`, the
/// `<ws>^{tree}` answer `tree` consulted only when `entry` is `Missing`.
fn manifest_entry(
    entry: BatchAnswer,
    tree: BatchAnswer,
    requests: &Requests,
    name: &[u8],
    argv: &[OsString],
) -> Result<RootEntry, Error> {
    let bad = |problem: String| Error::GitBadOutput {
        argv: argv.to_vec(),
        problem,
    };
    match entry {
        BatchAnswer::Found {
            kind, mode, data, ..
        } => match mode.as_slice() {
            b"100644" | b"100755" if kind == b"blob" => Ok(RootEntry::Blob(data)),
            b"120000" => Ok(RootEntry::NotABlob { kind: "symlink" }),
            b"040000" => Ok(RootEntry::NotABlob { kind: "directory" }),
            GITLINK_MODE => Ok(RootEntry::NotABlob { kind: "submodule" }),
            _ => Err(bad(format!(
                "{} is a {} with mode {:?}",
                String::from_utf8_lossy(&requests.entry),
                String::from_utf8_lossy(&kind),
                String::from_utf8_lossy(&mode)
            ))),
        },
        BatchAnswer::Missing => match tree {
            BatchAnswer::Missing => Err(Error::ObjectMissing {
                request: requests.tree.clone(),
            }),
            BatchAnswer::Found { id, kind, data, .. } if kind == b"tree" => {
                match root_tree_mode(&data, id.0.len() / 2, name).map_err(bad)? {
                    None => Ok(RootEntry::Absent),
                    Some(mode) if mode == GITLINK_MODE => {
                        Ok(RootEntry::NotABlob { kind: "submodule" })
                    }
                    Some(_) => Err(Error::ObjectMissing {
                        request: requests.entry.clone(),
                    }),
                }
            }
            BatchAnswer::Found { kind, .. } => Err(bad(format!(
                "{} answered with a {}, not a tree",
                String::from_utf8_lossy(&requests.tree),
                String::from_utf8_lossy(&kind)
            ))),
        },
    }
}

/// The four request lines of one `read_tips` batch.
struct Requests {
    /// `<ws>^{commit}`.
    tip: Vec<u8>,
    /// `<ws>^{tree}`.
    tree: Vec<u8>,
    /// `<ws>:<name>`.
    entry: Vec<u8>,
    /// `<rung>^{commit}`.
    rung: Vec<u8>,
}

impl Repo {
    /// The workspace tip, its manifest entry and the integration tip, read
    /// in one `git --no-lazy-fetch cat-file --batch` child fed through
    /// `Git::run_with_input`. Four requests, one per line: `<ws>^{commit}`
    /// and `<rung>^{commit}`, which peel annotated and nested tags as git
    /// does and are `missing` for a blob or a tree (F4); `<ws>^{tree}`; and
    /// `<ws>:<name>`, the entry with its mode. `name` must hold no LF.
    ///
    /// The manifest is a blob only for mode `100644` or `100755`; a
    /// symlink (`120000`, whose blob holds its link text), a directory and a
    /// gitlink are `RootEntry::NotABlob`. git answers `missing` for an
    /// absent path, for an entry whose object is gone (a partial clone's
    /// blob included) and for an entry its sorted lookup misses in an
    /// unsorted tree (F3). Only then is the root tree's answer read, and
    /// only to pick the error: no entry is `Absent`, a gitlink entry is
    /// `NotABlob`, any other entry `Error::ObjectMissing`; no byte git did
    /// not return is read. The whole blob is read before the caller's size
    /// check, as gix's `find_blob` read it (research G10).
    ///
    /// The child's failure is the outer `Err`, and output that does not
    /// parse is `Error::GitBadOutput`; each part's own answer is its field.
    pub fn read_tips(
        &self,
        git: &Git,
        workspace: &ObjectId,
        name: &'static [u8],
        integration: &ObjectId,
    ) -> Result<TipObjects, Error> {
        debug_assert!(!name.contains(&b'\n'), "a batch request holds no LF");
        let requests = Requests {
            tip: format!("{workspace}^{{commit}}").into_bytes(),
            tree: format!("{workspace}^{{tree}}").into_bytes(),
            entry: [format!("{workspace}:").as_bytes(), name].concat(),
            rung: format!("{integration}^{{commit}}").into_bytes(),
        };
        let mut input = Vec::new();
        for line in [
            &requests.tip,
            &requests.tree,
            &requests.entry,
            &requests.rung,
        ] {
            input.extend_from_slice(line);
            input.push(b'\n');
        }
        let argv: Vec<OsString> = [NO_LAZY_FETCH, "cat-file", BATCH_FORMAT]
            .into_iter()
            .map(OsString::from)
            .collect();
        let cwd = self.worktree().unwrap_or(self.git_dir());
        let stdout = git.run_with_input(cwd, &argv, &input)?;
        let answers = parse_batch(&stdout, 4).map_err(|problem| Error::GitBadOutput {
            argv: argv.clone(),
            problem,
        })?;
        let [tip, tree, entry, rung] =
            <[BatchAnswer; 4]>::try_from(answers).map_err(|answers| Error::GitBadOutput {
                argv: argv.clone(),
                problem: format!("{} answers to 4 requests", answers.len()),
            })?;
        Ok(TipObjects {
            workspace_tip: peeled_commit(tip, &requests.tip, &argv),
            manifest: manifest_entry(entry, tree, &requests, name, &argv),
            integration_tip: peeled_commit(rung, &requests.rung, &argv),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::ObjectId;
    use super::{BatchAnswer, parse_batch, root_tree_mode};

    /// A commit id, a blob id and a request git cannot answer.
    const COMMIT: &str = "1111111111111111111111111111111111111111";
    const BLOB: &str = "2222222222222222222222222222222222222222";
    const UNKNOWN: &str = "3333333333333333333333333333333333333333^{commit}";

    fn id(hex: &str) -> ObjectId {
        ObjectId(hex.to_owned())
    }

    #[test]
    fn parse_batch_reads_missing_and_sized_records() {
        let stdout =
            format!("{UNKNOWN} missing\n{COMMIT} commit 3 \nabc\n{BLOB} blob 5 120000\nl\nnk\n\n");
        assert_eq!(
            parse_batch(stdout.as_bytes(), 3),
            Ok(vec![
                BatchAnswer::Missing,
                BatchAnswer::Found {
                    id: id(COMMIT),
                    kind: b"commit".to_vec(),
                    mode: Vec::new(),
                    data: b"abc".to_vec(),
                },
                BatchAnswer::Found {
                    id: id(BLOB),
                    kind: b"blob".to_vec(),
                    mode: b"120000".to_vec(),
                    data: b"l\nnk\n".to_vec(),
                },
            ])
        );
    }

    #[test]
    fn parse_batch_refuses_a_short_read() {
        let short = format!("{BLOB} blob 9 100644\nabc");
        match parse_batch(short.as_bytes(), 1) {
            Err(_) => {}
            other => panic!("expected Err for a short read, got {other:?}"),
        }
        let trailing = format!("{COMMIT} commit 3 \nabc\nX");
        match parse_batch(trailing.as_bytes(), 1) {
            Err(_) => {}
            other => panic!("expected Err for a trailing byte, got {other:?}"),
        }
    }

    /// One raw tree entry: `<mode> SP <name> NUL` and a 20-byte id.
    fn entry(mode: &str, name: &str) -> Vec<u8> {
        let mut bytes = format!("{mode} {name}\0").into_bytes();
        bytes.extend_from_slice(&[0xab; 20]);
        bytes
    }

    #[test]
    fn tree_scan_finds_the_mode_of_a_root_entry() {
        let tree = [
            entry("100644", ".wsx.conf"),
            entry("40000", "d"),
            entry("160000", "s"),
        ]
        .concat();
        assert_eq!(
            root_tree_mode(&tree, 20, b".wsx.conf"),
            Ok(Some(b"100644".to_vec())),
            ".wsx.conf"
        );
        assert_eq!(
            root_tree_mode(&tree, 20, b"s"),
            Ok(Some(b"160000".to_vec())),
            "s"
        );
        assert_eq!(root_tree_mode(&tree, 20, b"x"), Ok(None), "x");
        let truncated = &tree[..tree.len() - 1];
        assert!(
            root_tree_mode(truncated, 20, b"x").is_err(),
            "a truncated id must be Err"
        );
    }
}
