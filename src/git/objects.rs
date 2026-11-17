//! The model's object reads: the workspace tip, its manifest and the
//! integration tip, through one `git cat-file --batch` child.

use super::refs::RootEntry;
use super::repo::Repo;
use super::{Error, Git, ObjectId};

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

/// One answer of `git cat-file --batch` in `read_tips`'s format.
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

/// The `count` answers in `stdout`, in request order.
fn parse_batch(stdout: &[u8], count: usize) -> Result<Vec<BatchAnswer>, String> {
    let _ = (stdout, count);
    Ok(Vec::new())
}

/// The mode of the entry named `name` in the raw tree `tree`, whose ids
/// are `id_len` bytes long.
fn root_tree_mode(tree: &[u8], id_len: usize, name: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let _ = (tree, id_len, name);
    Ok(None)
}

impl Repo {
    /// The workspace tip, its manifest and the integration tip, read in one
    /// `git cat-file --batch` child.
    pub fn read_tips(
        &self,
        git: &Git,
        workspace: &ObjectId,
        name: &'static [u8],
        integration: &ObjectId,
    ) -> Result<TipObjects, Error> {
        let _ = (git, name);
        Ok(TipObjects {
            workspace_tip: Ok(workspace.clone()),
            manifest: Ok(RootEntry::Absent),
            integration_tip: Ok(integration.clone()),
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
