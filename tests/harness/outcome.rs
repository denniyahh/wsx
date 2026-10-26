//! Expected outcomes, the observer that records what an action did, and the
//! comparator between the two.
//!
//! Expectations are typed and owned. The comparator destructures every
//! expectation struct without `..`, so a field added to one is a compile error
//! until the comparator handles it. The observer reads git's raw, NUL-separated
//! forms (`for-each-ref` with `%00`, `cat-file commit`, `ls-tree -z`,
//! `worktree list -z`) and keeps paths and messages as bytes, so it never
//! inherits a quoting bug from porcelain output.
//!
//! A symbolic ref is recorded by its target name, never by the object it
//! resolves to: a fetch that moves `origin/develop` leaves `origin/HEAD`
//! unchanged.
//!
//! Each checkout is also recorded beyond its files: what its HEAD holds (a
//! branch, or a detached commit), its index (`ls-files -s -z`, staged content
//! without stat data) and any operation left in progress (`MERGE_HEAD`,
//! `CHERRY_PICK_HEAD`, a rebase directory, ...). A cut that writes the PR tree
//! into the user's index, or leaves the checkout detached, changes none of the
//! files on disk, and only this record sees it.
//!
//! Refs are read in every namespace, not only branches, remote-tracking refs
//! and tags, and so are the refs of every bare remote under `remotes/`; the
//! work repository's local config is recorded too. A push to a ref no tracking
//! ref follows, a backup ref, or an upstream setting is otherwise invisible.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::env::{ScenarioDirs, hermetic_env, inherited_env};
use crate::fixture::{ACTION_TIME, Fixture, Ident, RepoConfig, WORK, dated};
use crate::oracle::{self, OracleCopy};
use crate::scenario::{CaseRun, Entry, Side, oracle_target, run_case};

/// Mode bits that make a file executable for someone.
const EXECUTABLE_BITS: u32 = 0o111;

/// Everything a scenario expects an action to do.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub exit: i32,
    pub refs: RefsExpect,
    /// Commits reachable afterwards from some ref or detached HEAD and from
    /// none before, in topological order, oldest first.
    pub new_commits: Vec<CommitExpect>,
    pub files: FilesExpect,
}

/// The repositories after the action: the work repository's refs, in every
/// namespace, and its local config. Under both forms every remote repository
/// under `remotes/` holds the same refs as before; no scenario expects a push.
#[derive(Clone, Debug, PartialEq)]
pub enum RefsExpect {
    /// Exactly these refs exist, each with the stated tip, and the local
    /// config is what it was plus exactly `config_added`.
    Exactly {
        refs: Vec<RefExpect>,
        /// The local config entries the action adds, each `key\nvalue` as
        /// `config --list -z` prints it (`config_entry` builds one). Their
        /// order is not compared.
        config_added: Vec<Vec<u8>>,
    },
    /// The repositories are untouched: the same refs exist as before, each
    /// holding what it held before (symbolic refs by target name), and the
    /// work repository's local config is the same.
    Unchanged,
}

impl RefsExpect {
    /// Exactly these refs, with the local config unchanged.
    pub fn exactly(refs: Vec<RefExpect>) -> RefsExpect {
        RefsExpect::Exactly {
            refs,
            config_added: Vec::new(),
        }
    }
}

/// A local config entry as `config --list -z` records it: `key\nvalue`.
pub fn config_entry(key: &str, value: &str) -> Vec<u8> {
    format!("{key}\n{value}").into_bytes()
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefExpect {
    pub name: String,
    pub tip: Tip,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tip {
    /// The ref holds what it held before: the same object, or for a symbolic
    /// ref the same target name.
    Unchanged,
    /// The ref points at a commit with exactly these properties.
    Commit(CommitExpect),
}

/// A commit, described without its id: signatures and fixture changes make ids
/// unstable, so identity is the message, people, dates, parents and tree.
#[derive(Clone, PartialEq)]
pub struct CommitExpect {
    pub message: Vec<u8>,
    pub author: Ident,
    /// `"<epoch> <offset>"`, e.g. `"1767312000 +0000"`.
    pub author_date: String,
    pub committer_name: String,
    pub committer_email: String,
    /// Each parent's subject line, in parent order.
    pub parents: Vec<Vec<u8>>,
    /// Every blob in the commit's tree, recursively.
    pub tree: Vec<TreeEntry>,
    /// Whether the commit carries a `gpgsig` or `gpgsig-sha256` header.
    pub signed: bool,
}

#[derive(Clone, PartialEq)]
pub struct TreeEntry {
    pub path: Vec<u8>,
    /// git's octal mode, e.g. `"100644"`.
    pub mode: String,
    pub content: Vec<u8>,
}

impl TreeEntry {
    /// A regular, non-executable file.
    pub fn file(path: &[u8], content: &[u8]) -> TreeEntry {
        TreeEntry {
            path: path.to_vec(),
            mode: "100644".to_owned(),
            content: content.to_vec(),
        }
    }
}

/// The checkouts after the action: their files on disk, HEADs, indexes and
/// operations in progress.
#[derive(Clone, Debug, PartialEq)]
pub enum FilesExpect {
    /// Exactly these files exist, across all checkouts, and exactly these
    /// checkouts exist, each on its stated branch. Every checkout's index
    /// also holds exactly its HEAD commit's tree (nothing staged, no
    /// conflict), and no checkout has an operation in progress.
    Exactly {
        files: Vec<FileExpect>,
        checkouts: Vec<CheckoutExpect>,
    },
    /// The checkouts are untouched: the same checkouts exist, each holds the
    /// same files, with the same content and executable bit, as before, and
    /// each has the same HEAD, the same index and the same pending operations.
    Unchanged,
}

#[derive(Clone, PartialEq)]
pub struct FileExpect {
    /// The checkout, relative to the scenario root.
    pub checkout: String,
    pub path: Vec<u8>,
    pub content: Vec<u8>,
    pub executable: bool,
}

/// A checkout that exists after the action, and the branch its HEAD is on.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckoutExpect {
    /// The checkout, relative to the scenario root.
    pub checkout: String,
    /// The branch, by its full ref name.
    pub branch: String,
}

impl CheckoutExpect {
    /// `checkout`, with its HEAD on `branch` (a full ref name).
    pub fn on(checkout: &str, branch: &str) -> CheckoutExpect {
        CheckoutExpect {
            checkout: checkout.to_owned(),
            branch: branch.to_owned(),
        }
    }
}

impl FileExpect {
    /// A non-executable file in `checkout`.
    pub fn plain(checkout: &str, path: &[u8], content: &[u8]) -> FileExpect {
        FileExpect {
            checkout: checkout.to_owned(),
            path: path.to_vec(),
            content: content.to_vec(),
            executable: false,
        }
    }
}

/// What a ref holds.
#[derive(Clone, Debug, PartialEq)]
pub enum RefValue {
    /// An object id.
    Object(String),
    /// A symbolic ref, by the name of its target.
    Symbolic(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FileState {
    pub content: Vec<u8>,
    pub executable: bool,
}

/// What a checkout's HEAD holds.
#[derive(Clone, Debug, PartialEq)]
pub enum HeadState {
    /// A branch, by its full ref name.
    Branch(String),
    /// A detached HEAD, by object id.
    Detached(String),
}

/// One staged index entry: git's octal mode and the blob's object id.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexEntry {
    pub mode: String,
    pub object: String,
}

/// A checkout's state beyond its files.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckoutState {
    pub head: HeadState,
    /// Every index entry, keyed by (path, stage).
    pub index: BTreeMap<(Vec<u8>, u8), IndexEntry>,
    /// The index HEAD's tree gives: each of its entries at stage 0. The
    /// index of a checkout with nothing staged equals it.
    pub head_tree: BTreeMap<(Vec<u8>, u8), IndexEntry>,
    /// The markers of operations in progress that exist, from
    /// `PENDING_MARKERS`.
    pub pending: BTreeSet<&'static str>,
}

/// The files and directories git leaves in a checkout's git dir while an
/// operation is in progress.
const PENDING_MARKERS: [&str; 7] = [
    "MERGE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
    "REBASE_HEAD",
    "rebase-merge",
    "rebase-apply",
    "sequencer",
];

/// The observable state of a fixture at one moment.
pub struct Snapshot {
    pub refs: BTreeMap<String, RefValue>,
    /// Every commit a ref points at, plus the new commits, by object id.
    pub commits: BTreeMap<String, CommitExpect>,
    /// Object ids of commits new since the previous snapshot, oldest first.
    pub new_commits: Vec<String>,
    /// Every file in every checkout, keyed by (checkout, path).
    pub files: BTreeMap<(String, Vec<u8>), FileState>,
    /// Every checkout's HEAD, index and pending operations, keyed by checkout.
    pub checkouts: BTreeMap<String, CheckoutState>,
    /// Every remote repository's refs, keyed by its path relative to the
    /// scenario root (`remotes/origin.git`).
    pub remote_refs: BTreeMap<String, BTreeMap<String, RefValue>>,
    /// The work repository's local config entries, `key=value`, in file order.
    pub config: Vec<Vec<u8>>,
}

/// Where the fixture keeps its bare remote repositories.
const REMOTES_DIR: &str = "remotes";

/// Records the fixture's refs, commits, files, checkout states, remote
/// repositories' refs and local config. With `before`, also the commits
/// reachable now from some ref or detached HEAD and from none in `before`.
pub fn observe(fixture: &Fixture, before: Option<&Snapshot>) -> Result<Snapshot, String> {
    let refs = read_refs(fixture, WORK)?;
    let checkouts = checkouts(fixture)?;
    let states = read_checkout_states(fixture, &checkouts)?;
    let new_commits = match before {
        Some(before) => read_new_commits(fixture, before, &refs, &states)?,
        None => Vec::new(),
    };
    let mut commits = BTreeMap::new();
    let tips = refs.values().filter_map(|value| match value {
        RefValue::Object(id) => Some(id),
        RefValue::Symbolic(_) => None,
    });
    for id in tips.chain(&new_commits) {
        if !commits.contains_key(id) {
            commits.insert(id.clone(), read_commit(fixture, id)?);
        }
    }
    Ok(Snapshot {
        refs,
        commits,
        new_commits,
        files: read_files(&checkouts)?,
        checkouts: states,
        remote_refs: read_remote_refs(fixture)?,
        config: read_config(fixture)?,
    })
}

fn utf8(bytes: &[u8], what: &str) -> Result<String, String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| format!("{what} is not UTF-8: {}", show(bytes)))
}

/// Every ref of the repository at `repo`, in every namespace.
fn read_refs(fixture: &Fixture, repo: &str) -> Result<BTreeMap<String, RefValue>, String> {
    let listing = fixture.git_ok(
        repo,
        [
            "for-each-ref",
            "--format=%(refname)%00%(symref)%00%(objectname)",
        ],
    )?;
    let mut refs = BTreeMap::new();
    for line in listing
        .split(|byte| *byte == b'\n')
        .filter(|l| !l.is_empty())
    {
        let fields: Vec<&[u8]> = line.split(|byte| *byte == 0).collect();
        let [name, symref, object] = fields[..] else {
            return Err(format!("unexpected for-each-ref line {}", show(line)));
        };
        let value = if symref.is_empty() {
            RefValue::Object(utf8(object, "an object id")?)
        } else {
            RefValue::Symbolic(utf8(symref, "a symbolic ref target")?)
        };
        refs.insert(utf8(name, "a ref name")?, value);
    }
    Ok(refs)
}

fn object_tips(refs: &BTreeMap<String, RefValue>) -> BTreeSet<&String> {
    refs.values()
        .filter_map(|value| match value {
            RefValue::Object(id) => Some(id),
            RefValue::Symbolic(_) => None,
        })
        .collect()
}

/// The object ids of `refs` and of every detached HEAD in `checkouts`.
fn commit_tips<'a>(
    refs: &'a BTreeMap<String, RefValue>,
    checkouts: &'a BTreeMap<String, CheckoutState>,
) -> BTreeSet<&'a String> {
    let detached = checkouts.values().filter_map(|state| match &state.head {
        HeadState::Detached(id) => Some(id),
        HeadState::Branch(_) => None,
    });
    object_tips(refs).into_iter().chain(detached).collect()
}

fn read_new_commits(
    fixture: &Fixture,
    before: &Snapshot,
    refs: &BTreeMap<String, RefValue>,
    checkouts: &BTreeMap<String, CheckoutState>,
) -> Result<Vec<String>, String> {
    let after_tips = commit_tips(refs, checkouts);
    if after_tips.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec!["rev-list", "--topo-order", "--reverse"];
    args.extend(after_tips.iter().map(|id| id.as_str()));
    args.push("--not");
    args.extend(
        commit_tips(&before.refs, &before.checkouts)
            .iter()
            .map(|id| id.as_str()),
    );
    let listing = fixture.git_ok(WORK, args)?;
    listing
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| utf8(line, "a commit id"))
        .collect()
}

/// Every bare repository under `remotes/`, by its path relative to the
/// scenario root, with its refs.
fn read_remote_refs(
    fixture: &Fixture,
) -> Result<BTreeMap<String, BTreeMap<String, RefValue>>, String> {
    let dir = fixture.root.join(REMOTES_DIR);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(err) => return Err(format!("read {}: {err}", dir.display())),
    };
    let mut remotes = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("read {}: {err}", dir.display()))?;
        let name = utf8(entry.file_name().as_bytes(), "a remote repository name")?;
        let repo = format!("{REMOTES_DIR}/{name}");
        let refs = read_refs(fixture, &repo)?;
        remotes.insert(repo, refs);
    }
    Ok(remotes)
}

/// The work repository's local config, one `key=value` entry each.
fn read_config(fixture: &Fixture) -> Result<Vec<Vec<u8>>, String> {
    // `--list -z` ends each entry with NUL and puts a newline between key and
    // value; the newline is kept as is, since only equality matters.
    let listing = fixture.git_ok(WORK, ["config", "--local", "--list", "-z"])?;
    Ok(listing
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(<[u8]>::to_vec)
        .collect())
}

/// The first line of a commit's message.
fn subject(message: &[u8]) -> Vec<u8> {
    message
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default()
        .to_vec()
}

/// Splits a raw commit into its header lines and its message.
fn split_commit(raw: &[u8]) -> (&[u8], &[u8]) {
    match raw.windows(2).position(|pair| pair == b"\n\n") {
        Some(at) => (&raw[..at], &raw[at + 2..]),
        None => (raw, &[]),
    }
}

/// Parses `Name <email> <epoch> <offset>`.
fn parse_person(value: &[u8]) -> Result<(Ident, String), String> {
    let malformed = || format!("malformed identity {}", show(value));
    let open = value
        .iter()
        .position(|byte| *byte == b'<')
        .ok_or_else(malformed)?;
    let close = value
        .iter()
        .rposition(|byte| *byte == b'>')
        .ok_or_else(malformed)?;
    if close < open || value.get(close + 1) != Some(&b' ') {
        return Err(malformed());
    }
    let name = value[..open].strip_suffix(b" ").unwrap_or(&value[..open]);
    let ident = Ident {
        name: utf8(name, "an identity name")?,
        email: utf8(&value[open + 1..close], "an identity email")?,
    };
    Ok((ident, utf8(&value[close + 2..], "an identity date")?))
}

fn read_commit(fixture: &Fixture, id: &str) -> Result<CommitExpect, String> {
    let raw = fixture.git_ok(WORK, ["cat-file", "commit", id])?;
    let (headers, message) = split_commit(&raw);
    let mut author = None;
    let mut committer = None;
    let mut parents = Vec::new();
    let mut signed = false;
    for line in headers.split(|byte| *byte == b'\n') {
        if line.starts_with(b" ") {
            continue; // continuation of a multi-line header such as gpgsig
        }
        let (key, value) = match line.iter().position(|byte| *byte == b' ') {
            Some(at) => (&line[..at], &line[at + 1..]),
            None => (line, &line[line.len()..]),
        };
        match key {
            b"parent" => {
                let parent = fixture.git_ok(
                    WORK,
                    [
                        OsStr::new("cat-file"),
                        OsStr::new("commit"),
                        OsStr::from_bytes(value),
                    ],
                )?;
                parents.push(subject(split_commit(&parent).1));
            }
            b"author" => author = Some(parse_person(value)?),
            b"committer" => committer = Some(parse_person(value)?),
            b"gpgsig" | b"gpgsig-sha256" => signed = true,
            _ => {}
        }
    }
    let (author, author_date) = author.ok_or_else(|| format!("commit {id} has no author"))?;
    let (committer, _) = committer.ok_or_else(|| format!("commit {id} has no committer"))?;
    Ok(CommitExpect {
        message: message.to_vec(),
        author,
        author_date,
        committer_name: committer.name,
        committer_email: committer.email,
        parents,
        tree: read_tree(fixture, id)?,
        signed,
    })
}

fn read_tree(fixture: &Fixture, commit: &str) -> Result<Vec<TreeEntry>, String> {
    let listing = fixture.git_ok(WORK, ["ls-tree", "-r", "-z", "--full-tree", commit])?;
    let mut tree = Vec::new();
    for record in listing.split(|byte| *byte == 0).filter(|r| !r.is_empty()) {
        let malformed = || format!("unexpected ls-tree record {}", show(record));
        let tab = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(malformed)?;
        let fields: Vec<&[u8]> = record[..tab].split(|byte| *byte == b' ').collect();
        let [mode, kind, object] = fields[..] else {
            return Err(malformed());
        };
        if kind != b"blob" {
            return Err(format!(
                "the observer records blobs only, got a {} at {}",
                show(kind),
                show(&record[tab + 1..])
            ));
        }
        let content = fixture.git_ok(
            WORK,
            [
                OsStr::new("cat-file"),
                OsStr::new("blob"),
                OsStr::from_bytes(object),
            ],
        )?;
        tree.push(TreeEntry {
            path: record[tab + 1..].to_vec(),
            mode: utf8(mode, "a tree entry mode")?,
            content,
        });
    }
    Ok(tree)
}

/// One checkout of the work repository, as `worktree list` reports it.
struct Checkout {
    /// The absolute path.
    path: PathBuf,
    /// The path relative to the scenario root.
    name: String,
    head: HeadState,
}

/// Every checkout of the work repository.
fn checkouts(fixture: &Fixture) -> Result<Vec<Checkout>, String> {
    let listing = fixture.git_ok(WORK, ["worktree", "list", "--porcelain", "-z"])?;
    // Records are runs of NUL-terminated fields, each run ended by an empty
    // field: `worktree <path>`, `HEAD <id>`, then `branch <ref>` or `detached`.
    let mut found = Vec::new();
    for record in listing
        .split(|byte| *byte == 0)
        .collect::<Vec<_>>()
        .split(|f| f.is_empty())
    {
        if record.is_empty() {
            continue;
        }
        let malformed = || format!("unexpected worktree list record {record:?}");
        let field = |prefix: &[u8]| record.iter().find_map(|f| f.strip_prefix(prefix));
        let path = PathBuf::from(OsStr::from_bytes(
            field(b"worktree ").ok_or_else(malformed)?,
        ));
        let head = match (field(b"branch "), field(b"HEAD ")) {
            (Some(branch), _) => HeadState::Branch(utf8(branch, "a branch name")?),
            (None, Some(id)) if record.contains(&&b"detached"[..]) => {
                HeadState::Detached(utf8(id, "an object id")?)
            }
            _ => return Err(malformed()),
        };
        let name = path
            .strip_prefix(&fixture.root)
            .map_err(|_| format!("checkout {} is outside the scenario root", path.display()))?;
        let name = utf8(name.as_os_str().as_bytes(), "a checkout name")?;
        found.push(Checkout { path, name, head });
    }
    Ok(found)
}

fn read_files(checkouts: &[Checkout]) -> Result<BTreeMap<(String, Vec<u8>), FileState>, String> {
    let paths: Vec<PathBuf> = checkouts.iter().map(|c| c.path.clone()).collect();
    let mut files = BTreeMap::new();
    for checkout in checkouts {
        let mut found = Vec::new();
        walk(&checkout.path, &[], &paths, &mut found)?;
        for (path, state) in found {
            files.insert((checkout.name.clone(), path), state);
        }
    }
    Ok(files)
}

fn read_checkout_states(
    fixture: &Fixture,
    checkouts: &[Checkout],
) -> Result<BTreeMap<String, CheckoutState>, String> {
    let mut states = BTreeMap::new();
    for checkout in checkouts {
        let state = CheckoutState {
            head: checkout.head.clone(),
            index: read_index(fixture, &checkout.name)?,
            head_tree: read_head_tree(fixture, &checkout.name)?,
            pending: read_pending(fixture, checkout)?,
        };
        states.insert(checkout.name.clone(), state);
    }
    Ok(states)
}

/// Parses `ls-files -s -z`: `<mode> <object> <stage>\t<path>` records.
fn read_index(
    fixture: &Fixture,
    checkout: &str,
) -> Result<BTreeMap<(Vec<u8>, u8), IndexEntry>, String> {
    let listing = fixture.git_ok(checkout, ["ls-files", "-s", "-z"])?;
    let mut index = BTreeMap::new();
    for record in listing.split(|byte| *byte == 0).filter(|r| !r.is_empty()) {
        let malformed = || format!("unexpected ls-files record {}", show(record));
        let tab = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(malformed)?;
        let fields: Vec<&[u8]> = record[..tab].split(|byte| *byte == b' ').collect();
        let [mode, object, stage] = fields[..] else {
            return Err(malformed());
        };
        let [stage @ b'0'..=b'3'] = stage else {
            return Err(malformed());
        };
        let entry = IndexEntry {
            mode: utf8(mode, "an index entry mode")?,
            object: utf8(object, "an object id")?,
        };
        index.insert((record[tab + 1..].to_vec(), stage - b'0'), entry);
    }
    Ok(index)
}

/// Parses `ls-tree -r -z --full-tree HEAD`, `<mode> <type> <object>\t<path>`
/// records, into the stage-0 index entries HEAD's tree gives.
fn read_head_tree(
    fixture: &Fixture,
    checkout: &str,
) -> Result<BTreeMap<(Vec<u8>, u8), IndexEntry>, String> {
    let listing = fixture.git_ok(checkout, ["ls-tree", "-r", "-z", "--full-tree", "HEAD"])?;
    let mut tree = BTreeMap::new();
    for record in listing.split(|byte| *byte == 0).filter(|r| !r.is_empty()) {
        let malformed = || format!("unexpected ls-tree record {}", show(record));
        let tab = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(malformed)?;
        let fields: Vec<&[u8]> = record[..tab].split(|byte| *byte == b' ').collect();
        let [mode, _kind, object] = fields[..] else {
            return Err(malformed());
        };
        let entry = IndexEntry {
            mode: utf8(mode, "a tree entry mode")?,
            object: utf8(object, "an object id")?,
        };
        tree.insert((record[tab + 1..].to_vec(), 0), entry);
    }
    Ok(tree)
}

/// The `PENDING_MARKERS` present in `checkout`'s git dir.
fn read_pending(fixture: &Fixture, checkout: &Checkout) -> Result<BTreeSet<&'static str>, String> {
    let mut args = vec!["rev-parse"];
    for marker in PENDING_MARKERS {
        args.extend(["--git-path", marker]);
    }
    let listing = fixture.git_ok(&checkout.name, args)?;
    let paths: Vec<&[u8]> = listing
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    if paths.len() != PENDING_MARKERS.len() {
        return Err(format!(
            "rev-parse --git-path printed {} paths for {} markers",
            paths.len(),
            PENDING_MARKERS.len()
        ));
    }
    let mut pending = BTreeSet::new();
    for (marker, path) in PENDING_MARKERS.into_iter().zip(paths) {
        // A relative path is relative to the checkout; join keeps an absolute one.
        let path = checkout.path.join(OsStr::from_bytes(path));
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                pending.insert(marker);
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("stat {}: {err}", path.display())),
        }
    }
    Ok(pending)
}

/// Records every file under `dir`, skipping `.git` and nested checkouts.
fn walk(
    dir: &Path,
    prefix: &[u8],
    checkouts: &[PathBuf],
    found: &mut Vec<(Vec<u8>, FileState)>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|err| format!("read {}: {err}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("read {}: {err}", dir.display()))?;
        let name = entry.file_name();
        let path = entry.path();
        if name == ".git" || checkouts.contains(&path) {
            continue;
        }
        let mut relative = prefix.to_vec();
        if !relative.is_empty() {
            relative.push(b'/');
        }
        relative.extend_from_slice(name.as_bytes());
        let meta =
            fs::symlink_metadata(&path).map_err(|err| format!("stat {}: {err}", path.display()))?;
        if meta.is_dir() {
            walk(&path, &relative, checkouts, found)?;
        } else if meta.is_file() {
            let content =
                fs::read(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
            let executable = meta.permissions().mode() & EXECUTABLE_BITS != 0;
            found.push((
                relative,
                FileState {
                    content,
                    executable,
                },
            ));
        } else {
            return Err(format!(
                "the observer records regular files only: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

/// Which part of the outcome differs.
#[derive(Clone, Debug, PartialEq)]
pub enum Field {
    Exit,
    RefSet,
    RefTip(String),
    NewCommitCount,
    Commit {
        which: String,
        part: CommitPart,
    },
    Files,
    /// The checkouts that exist; compared under both `FilesExpect` forms, as
    /// are the three below.
    CheckoutSet,
    /// What a checkout's HEAD holds.
    CheckoutHead(String),
    /// A checkout's staged entries.
    CheckoutIndex(String),
    /// The operations left in progress in a checkout.
    CheckoutPending(String),
    /// The action's stderr, against the case's `oracle_stderr` on the oracle
    /// side and its `rust_stderr` on the Rust side.
    Stderr,
    /// A remote repository's refs; compared under both `RefsExpect` forms, as
    /// is the config below.
    RemoteRefs(String),
    /// The work repository's local config.
    Config,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommitPart {
    Message,
    Author,
    AuthorDate,
    CommitterName,
    CommitterEmail,
    Parents,
    Tree,
    Signature,
}

/// One difference between the expected and the observed outcome.
#[derive(Clone, Debug)]
pub struct Mismatch {
    pub field: Field,
    pub expected: String,
    pub got: String,
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Field::Exit => write!(f, "exit code"),
            Field::RefSet => write!(f, "the set of refs"),
            Field::RefTip(name) => write!(f, "the tip of {name}"),
            Field::NewCommitCount => write!(f, "the number of new commits"),
            Field::Commit { which, part } => {
                let part = match part {
                    CommitPart::Message => "message",
                    CommitPart::Author => "author",
                    CommitPart::AuthorDate => "author date",
                    CommitPart::CommitterName => "committer name",
                    CommitPart::CommitterEmail => "committer email",
                    CommitPart::Parents => "parents",
                    CommitPart::Tree => "tree",
                    CommitPart::Signature => "signature",
                };
                write!(f, "{which}: {part}")
            }
            Field::Files => write!(f, "files on disk"),
            Field::CheckoutSet => write!(f, "the set of checkouts"),
            Field::CheckoutHead(name) => write!(f, "the HEAD of checkout {name}"),
            Field::CheckoutIndex(name) => write!(f, "the index of checkout {name}"),
            Field::CheckoutPending(name) => {
                write!(f, "the operations in progress in checkout {name}")
            }
            Field::Stderr => write!(f, "the action's stderr"),
            Field::RemoteRefs(repo) => write!(f, "the refs of the remote repository {repo}"),
            Field::Config => write!(f, "the work repository's local config"),
        }
    }
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n      expected: {}\n      got:      {}",
            self.field, self.expected, self.got
        )
    }
}

/// Bytes as a quoted, escaped string, for messages only.
fn show(bytes: &[u8]) -> String {
    format!("\"{}\"", bytes.escape_ascii())
}

fn show_list<'a>(items: impl IntoIterator<Item = &'a Vec<u8>>) -> String {
    let items: Vec<String> = items.into_iter().map(|item| show(item)).collect();
    format!("[{}]", items.join(", "))
}

fn describe_exit(exit: Option<i32>) -> String {
    match exit {
        Some(code) => code.to_string(),
        None => "no exit code (killed by a signal)".to_owned(),
    }
}

fn describe_ref(value: &RefValue, snapshot: &Snapshot) -> String {
    match value {
        RefValue::Symbolic(target) => format!("symbolic ref to {target}"),
        RefValue::Object(id) => match snapshot.commits.get(id) {
            Some(commit) => format!("commit {}", show(&subject(&commit.message))),
            None => format!("object {id}"),
        },
    }
}

/// Every difference between `expected` and what the action did, judged from
/// the snapshots before and after it and its exit code.
pub fn compare(
    expected: &Outcome,
    before: &Snapshot,
    after: &Snapshot,
    exit: Option<i32>,
) -> Vec<Mismatch> {
    let Outcome {
        exit: expected_exit,
        refs,
        new_commits,
        files,
    } = expected;
    let mut mismatches = Vec::new();
    if exit != Some(*expected_exit) {
        mismatches.push(Mismatch {
            field: Field::Exit,
            expected: expected_exit.to_string(),
            got: describe_exit(exit),
        });
    }
    let config_added: &[Vec<u8>] = match refs {
        RefsExpect::Exactly { refs, config_added } => {
            compare_refs(refs, before, after, &mut mismatches);
            config_added
        }
        RefsExpect::Unchanged => {
            let as_before: Vec<RefExpect> = before
                .refs
                .keys()
                .map(|name| RefExpect {
                    name: name.clone(),
                    tip: Tip::Unchanged,
                })
                .collect();
            compare_refs(&as_before, before, after, &mut mismatches);
            &[]
        }
    };
    compare_remote_refs(&before.remote_refs, &after.remote_refs, &mut mismatches);
    compare_config(&before.config, config_added, &after.config, &mut mismatches);
    compare_new_commits(new_commits, after, &mut mismatches);
    match files {
        FilesExpect::Exactly { files, checkouts } => {
            let expected_files = files
                .iter()
                .map(|file| {
                    let FileExpect {
                        checkout,
                        path,
                        content,
                        executable,
                    } = file;
                    let state = FileState {
                        content: content.clone(),
                        executable: *executable,
                    };
                    ((checkout.clone(), path.clone()), state)
                })
                .collect();
            compare_files(&expected_files, &after.files, &mut mismatches);
            compare_listed_checkouts(checkouts, &after.checkouts, &mut mismatches);
        }
        FilesExpect::Unchanged => {
            compare_files(&before.files, &after.files, &mut mismatches);
            compare_checkouts(&before.checkouts, &after.checkouts, &mut mismatches);
        }
    }
    mismatches
}

/// The local config against what it was plus `added`. With nothing added,
/// the entries must also keep their order.
fn compare_config<'a>(
    before: &'a [Vec<u8>],
    added: &'a [Vec<u8>],
    after: &'a [Vec<u8>],
    mismatches: &mut Vec<Mismatch>,
) {
    let mut want = entry_counts(before);
    for entry in added {
        *want.entry(entry).or_insert(0) += 1;
    }
    let got = entry_counts(after);
    if want == got && (!added.is_empty() || before == after) {
        return;
    }
    let (want_entries, got_entries) = differences(
        &want,
        &got,
        |entry| show(entry),
        |count| format!("{count} times"),
    );
    let expected = if added.is_empty() {
        format!("unchanged: {want_entries}")
    } else {
        format!("as before plus {}: {want_entries}", show_list(added))
    };
    mismatches.push(Mismatch {
        field: Field::Config,
        expected,
        got: got_entries,
    });
}

/// Under `FilesExpect::Exactly`: exactly the `listed` checkouts exist, each
/// on its branch, and every checkout's index holds its HEAD's tree with no
/// operation in progress.
fn compare_listed_checkouts(
    listed: &[CheckoutExpect],
    after: &BTreeMap<String, CheckoutState>,
    mismatches: &mut Vec<Mismatch>,
) {
    let expected_names: BTreeSet<&String> = listed.iter().map(|c| &c.checkout).collect();
    let got_names: BTreeSet<&String> = after.keys().collect();
    if expected_names != got_names {
        let join = |names: &BTreeSet<&String>| {
            names
                .iter()
                .map(|name| name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        mismatches.push(Mismatch {
            field: Field::CheckoutSet,
            expected: join(&expected_names),
            got: join(&got_names),
        });
    }
    for (name, state) in after {
        let CheckoutState {
            head,
            index,
            head_tree,
            pending,
        } = state;
        let listed_branch = listed
            .iter()
            .find(|c| c.checkout == *name)
            .map(|c| HeadState::Branch(c.branch.clone()));
        if let Some(want) = listed_branch
            && *head != want
        {
            mismatches.push(Mismatch {
                field: Field::CheckoutHead(name.clone()),
                expected: describe_head(&want),
                got: describe_head(head),
            });
        }
        if index != head_tree {
            let (want, got) = differences(
                head_tree,
                index,
                |(path, stage)| format!("{} stage {stage}", show(path)),
                |entry| format!("mode {}, object {}", entry.mode, entry.object),
            );
            mismatches.push(Mismatch {
                field: Field::CheckoutIndex(name.clone()),
                expected: format!("HEAD's tree: {want}"),
                got,
            });
        }
        if !pending.is_empty() {
            mismatches.push(Mismatch {
                field: Field::CheckoutPending(name.clone()),
                expected: "none".to_owned(),
                got: format!("{pending:?}"),
            });
        }
    }
}

/// How often each config entry occurs.
fn entry_counts(entries: &[Vec<u8>]) -> BTreeMap<&Vec<u8>, usize> {
    let mut counts = BTreeMap::new();
    for entry in entries {
        *counts.entry(entry).or_insert(0) += 1;
    }
    counts
}

/// Each remote repository's refs against what they were.
fn compare_remote_refs(
    before: &BTreeMap<String, BTreeMap<String, RefValue>>,
    after: &BTreeMap<String, BTreeMap<String, RefValue>>,
    mismatches: &mut Vec<Mismatch>,
) {
    let empty = BTreeMap::new();
    let repos: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    for repo in repos {
        let (was, now) = (
            before.get(repo).unwrap_or(&empty),
            after.get(repo).unwrap_or(&empty),
        );
        if was != now || before.contains_key(repo) != after.contains_key(repo) {
            let (want, got) = differences(
                was,
                now,
                |name| name.clone(),
                |value| match value {
                    RefValue::Object(id) => id.clone(),
                    RefValue::Symbolic(target) => format!("symbolic ref to {target}"),
                },
            );
            mismatches.push(Mismatch {
                field: Field::RemoteRefs(repo.clone()),
                expected: format!("unchanged: {want}"),
                got,
            });
        }
    }
}

fn describe_head(head: &HeadState) -> String {
    match head {
        HeadState::Branch(name) => format!("on branch {name}"),
        HeadState::Detached(id) => format!("detached at {id}"),
    }
}

/// Each checkout's HEAD, index and pending operations against what they were.
fn compare_checkouts(
    before: &BTreeMap<String, CheckoutState>,
    after: &BTreeMap<String, CheckoutState>,
    mismatches: &mut Vec<Mismatch>,
) {
    let names = |states: &BTreeMap<String, CheckoutState>| {
        states.keys().cloned().collect::<Vec<_>>().join(", ")
    };
    if before.keys().ne(after.keys()) {
        mismatches.push(Mismatch {
            field: Field::CheckoutSet,
            expected: names(before),
            got: names(after),
        });
    }
    for (name, was) in before {
        let Some(now) = after.get(name) else {
            continue; // already reported as a difference in the checkout set
        };
        let CheckoutState {
            head,
            index,
            // Not compared: HEAD is, and its branch's tip is stated by the
            // refs expectation.
            head_tree: _,
            pending,
        } = was;
        if *head != now.head {
            mismatches.push(Mismatch {
                field: Field::CheckoutHead(name.clone()),
                expected: format!("unchanged ({})", describe_head(head)),
                got: describe_head(&now.head),
            });
        }
        if *index != now.index {
            let (want, got) = differences(
                index,
                &now.index,
                |(path, stage)| format!("{} stage {stage}", show(path)),
                |entry| format!("mode {}, object {}", entry.mode, entry.object),
            );
            mismatches.push(Mismatch {
                field: Field::CheckoutIndex(name.clone()),
                expected: format!("unchanged: {want}"),
                got,
            });
        }
        if *pending != now.pending {
            let list = |set: &BTreeSet<&str>| format!("{set:?}");
            mismatches.push(Mismatch {
                field: Field::CheckoutPending(name.clone()),
                expected: format!("unchanged: {}", list(pending)),
                got: list(&now.pending),
            });
        }
    }
}

fn compare_refs(
    list: &[RefExpect],
    before: &Snapshot,
    after: &Snapshot,
    mismatches: &mut Vec<Mismatch>,
) {
    let expected_names: Vec<&String> = list
        .iter()
        .map(|r| &r.name)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let got_names: Vec<&String> = after.refs.keys().collect();
    if expected_names != got_names {
        let join = |names: &[&String]| {
            names
                .iter()
                .map(|name| name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        mismatches.push(Mismatch {
            field: Field::RefSet,
            expected: join(&expected_names),
            got: join(&got_names),
        });
    }
    for RefExpect { name, tip } in list {
        let Some(now) = after.refs.get(name) else {
            continue; // already reported as a difference in the ref set
        };
        match tip {
            Tip::Unchanged => {
                let was = before.refs.get(name);
                if was != Some(now) {
                    let expected = match was {
                        Some(was) => format!("unchanged ({})", describe_ref(was, before)),
                        None => "unchanged, but the ref did not exist before".to_owned(),
                    };
                    mismatches.push(Mismatch {
                        field: Field::RefTip(name.clone()),
                        expected,
                        got: describe_ref(now, after),
                    });
                }
            }
            Tip::Commit(commit) => match now {
                RefValue::Object(id) => match after.commits.get(id) {
                    Some(observed) => compare_commit(name, commit, observed, mismatches),
                    None => mismatches.push(Mismatch {
                        field: Field::RefTip(name.clone()),
                        expected: format!("commit {}", show(&subject(&commit.message))),
                        got: format!("object {id}, which the observer did not record"),
                    }),
                },
                RefValue::Symbolic(_) => mismatches.push(Mismatch {
                    field: Field::RefTip(name.clone()),
                    expected: format!("commit {}", show(&subject(&commit.message))),
                    got: describe_ref(now, after),
                }),
            },
        }
    }
}

fn compare_new_commits(
    expected: &[CommitExpect],
    after: &Snapshot,
    mismatches: &mut Vec<Mismatch>,
) {
    let observed: Vec<&CommitExpect> = after
        .new_commits
        .iter()
        .filter_map(|id| after.commits.get(id))
        .collect();
    if expected.len() != observed.len() {
        mismatches.push(Mismatch {
            field: Field::NewCommitCount,
            expected: format!(
                "{} {}",
                expected.len(),
                show_list(expected.iter().map(|c| &c.message))
            ),
            got: format!(
                "{} {}",
                observed.len(),
                show_list(observed.iter().map(|c| &c.message))
            ),
        });
    }
    for (index, (want, got)) in expected.iter().zip(&observed).enumerate() {
        compare_commit(&format!("new commit {}", index + 1), want, got, mismatches);
    }
}

fn describe_ident(ident: &Ident) -> String {
    format!("{} <{}>", ident.name, ident.email)
}

fn compare_commit(
    which: &str,
    expected: &CommitExpect,
    observed: &CommitExpect,
    mismatches: &mut Vec<Mismatch>,
) {
    let CommitExpect {
        message,
        author,
        author_date,
        committer_name,
        committer_email,
        parents,
        tree,
        signed,
    } = expected;
    let CommitExpect {
        message: got_message,
        author: got_author,
        author_date: got_author_date,
        committer_name: got_committer_name,
        committer_email: got_committer_email,
        parents: got_parents,
        tree: got_tree,
        signed: got_signed,
    } = observed;
    let mut push = |part: CommitPart, expected: String, got: String| {
        mismatches.push(Mismatch {
            field: Field::Commit {
                which: which.to_owned(),
                part,
            },
            expected,
            got,
        });
    };
    if message != got_message {
        push(CommitPart::Message, show(message), show(got_message));
    }
    if author != got_author {
        push(
            CommitPart::Author,
            describe_ident(author),
            describe_ident(got_author),
        );
    }
    if author_date != got_author_date {
        push(
            CommitPart::AuthorDate,
            author_date.clone(),
            got_author_date.clone(),
        );
    }
    if committer_name != got_committer_name {
        push(
            CommitPart::CommitterName,
            committer_name.clone(),
            got_committer_name.clone(),
        );
    }
    if committer_email != got_committer_email {
        push(
            CommitPart::CommitterEmail,
            committer_email.clone(),
            got_committer_email.clone(),
        );
    }
    if parents != got_parents {
        push(
            CommitPart::Parents,
            show_list(parents),
            show_list(got_parents),
        );
    }
    let tree_map = |tree: &[TreeEntry]| -> BTreeMap<Vec<u8>, (String, Vec<u8>)> {
        tree.iter()
            .map(
                |TreeEntry {
                     path,
                     mode,
                     content,
                 }| (path.clone(), (mode.clone(), content.clone())),
            )
            .collect()
    };
    let (want_tree, got_tree) = (tree_map(tree), tree_map(got_tree));
    if want_tree != got_tree {
        let (want, got) = differences(
            &want_tree,
            &got_tree,
            |path| show(path),
            |(mode, content)| format!("mode {mode}, {}", show(content)),
        );
        push(CommitPart::Tree, want, got);
    }
    if signed != got_signed {
        let words = |signed: bool| if signed { "signed" } else { "unsigned" }.to_owned();
        push(CommitPart::Signature, words(*signed), words(*got_signed));
    }
}

/// The entries of two maps that differ, rendered side by side: each key that
/// is missing, extra or different, with its value on each side.
fn differences<K: Ord, V: PartialEq>(
    expected: &BTreeMap<K, V>,
    observed: &BTreeMap<K, V>,
    name: impl Fn(&K) -> String,
    describe: impl Fn(&V) -> String,
) -> (String, String) {
    let keys: BTreeSet<&K> = expected.keys().chain(observed.keys()).collect();
    let (mut want, mut got) = (Vec::new(), Vec::new());
    for key in keys {
        let (left, right) = (expected.get(key), observed.get(key));
        if left != right {
            let side = |value: Option<&V>| match value {
                Some(value) => format!("{} = {}", name(key), describe(value)),
                None => format!("{} absent", name(key)),
            };
            want.push(side(left));
            got.push(side(right));
        }
    }
    (want.join("; "), got.join("; "))
}

fn compare_files(
    expected: &BTreeMap<(String, Vec<u8>), FileState>,
    observed: &BTreeMap<(String, Vec<u8>), FileState>,
    mismatches: &mut Vec<Mismatch>,
) {
    if expected != observed {
        let (want, got) = differences(
            expected,
            observed,
            |(checkout, path)| format!("{} in {checkout}", show(path)),
            |state| {
                let executable = if state.executable {
                    "executable"
                } else {
                    "not executable"
                };
                format!("{} ({executable})", show(&state.content))
            },
        );
        mismatches.push(Mismatch {
            field: Field::Files,
            expected: want,
            got,
        });
    }
}

/// The observed outcome in the expectation types, for transcribing a recorded
/// outcome from a real run. A symbolic ref that changed, a removed config
/// entry and a detached checkout have no expectation form yet and are left
/// out, and a run killed by a signal shows exit -1; the comparison still
/// reports each of them.
pub fn observed(before: &Snapshot, after: &Snapshot, exit: Option<i32>) -> Outcome {
    let refs = after
        .refs
        .iter()
        .filter_map(|(name, value)| {
            let tip = if before.refs.get(name) == Some(value) {
                Tip::Unchanged
            } else {
                match value {
                    RefValue::Object(id) => Tip::Commit(after.commits.get(id)?.clone()),
                    RefValue::Symbolic(_) => return None,
                }
            };
            Some(RefExpect {
                name: name.clone(),
                tip,
            })
        })
        .collect();
    let new_commits = after
        .new_commits
        .iter()
        .filter_map(|id| after.commits.get(id).cloned())
        .collect();
    let files = after
        .files
        .iter()
        .map(
            |(
                (checkout, path),
                FileState {
                    content,
                    executable,
                },
            )| FileExpect {
                checkout: checkout.clone(),
                path: path.clone(),
                content: content.clone(),
                executable: *executable,
            },
        )
        .collect();
    let mut config_added = Vec::new();
    let mut before_counts = entry_counts(&before.config);
    for entry in &after.config {
        match before_counts.get_mut(entry) {
            Some(count) if *count > 0 => *count -= 1,
            _ => config_added.push(entry.clone()),
        }
    }
    let checkouts = after
        .checkouts
        .iter()
        .filter_map(|(checkout, state)| match &state.head {
            HeadState::Branch(branch) => Some(CheckoutExpect::on(checkout, branch)),
            HeadState::Detached(_) => None,
        })
        .collect();
    Outcome {
        exit: exit.unwrap_or(-1),
        refs: RefsExpect::Exactly { refs, config_added },
        new_commits,
        files: FilesExpect::Exactly { files, checkouts },
    }
}

/// Debug for bytes as a byte-string literal, so a printed outcome can be
/// pasted back into a scenario.
struct Bytes<'a>(&'a [u8]);

impl fmt::Debug for Bytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"{}\"", self.0.escape_ascii())
    }
}

impl fmt::Debug for CommitExpect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parents: Vec<Bytes<'_>> = self.parents.iter().map(|p| Bytes(p)).collect();
        f.debug_struct("CommitExpect")
            .field("message", &Bytes(&self.message))
            .field("author", &self.author)
            .field("author_date", &self.author_date)
            .field("committer_name", &self.committer_name)
            .field("committer_email", &self.committer_email)
            .field("parents", &parents)
            .field("tree", &self.tree)
            .field("signed", &self.signed)
            .finish()
    }
}

impl fmt::Debug for TreeEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeEntry")
            .field("path", &Bytes(&self.path))
            .field("mode", &self.mode)
            .field("content", &Bytes(&self.content))
            .finish()
    }
}

impl fmt::Debug for FileExpect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileExpect")
            .field("checkout", &self.checkout)
            .field("path", &Bytes(&self.path))
            .field("content", &Bytes(&self.content))
            .field("executable", &self.executable)
            .finish()
    }
}

/// The observer records a symbolic ref by its target name. After a fetch moves
/// `origin/develop`, `origin/HEAD` resolves to a new object but still targets
/// `refs/remotes/origin/develop`, so `Tip::Unchanged` holds for it. In the same
/// run `Tip::Unchanged` must fail for `origin/develop` itself: that half rules
/// out a check that passes only because the fetch changed nothing.
#[test]
fn symbolic_refs_are_recorded_by_target() {
    let dirs = ScenarioDirs::new("outcome", "symbolic_refs").expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let mut fixture = Fixture::new(&dirs, env, RepoConfig::devflow()).expect("devflow fixture");
    let before = observe(&fixture, None).expect("observe before the fetch");
    fixture
        .advance_remote("origin", "develop", |fixture, clone| {
            fixture.write(clone, b"src/b.txt", b"b\n")?;
            fixture.commit(clone, b"feat: add b\n")
        })
        .expect("advance origin/develop");
    fixture
        .git_ok(WORK, ["fetch", "--quiet", "origin", "develop"])
        .expect("fetch origin develop");
    let after = observe(&fixture, Some(&before)).expect("observe after the fetch");

    let head = "refs/remotes/origin/HEAD";
    let develop = "refs/remotes/origin/develop";
    let target = RefValue::Symbolic(develop.to_owned());
    assert_eq!(
        before.refs.get(head),
        Some(&target),
        "{head} before the fetch"
    );
    assert_eq!(
        after.refs.get(head),
        Some(&target),
        "{head} after the fetch"
    );

    let every_ref_unchanged = Outcome {
        exit: 0,
        refs: RefsExpect::exactly(
            after
                .refs
                .keys()
                .map(|name| RefExpect {
                    name: name.clone(),
                    tip: Tip::Unchanged,
                })
                .collect(),
        ),
        new_commits: Vec::new(),
        files: FilesExpect::Unchanged,
    };
    let tip_mismatches: Vec<Field> = compare(&every_ref_unchanged, &before, &after, Some(0))
        .into_iter()
        .map(|mismatch| mismatch.field)
        .filter(|field| matches!(field, Field::RefTip(_)))
        .collect();
    assert_eq!(
        tip_mismatches,
        vec![Field::RefTip(develop.to_owned())],
        "only the fetched ref moved; the symbolic ref to it did not"
    );
}

// ----------------------------------------- every expected field is compared

/// The entry of the registered scenario `module::name`.
fn registered(module: &str, name: &str) -> Entry {
    crate::all_scenarios()
        .into_iter()
        .find(|entry| entry.module == module && entry.name == name)
        .unwrap_or_else(|| panic!("{module}::{name} is registered"))
}

/// Runs the subject of the registered scenario `module::name` on the oracle
/// side, and returns its explicit expected outcome with the run.
fn oracle_run(module: &str, name: &str) -> (Outcome, CaseRun) {
    let entry = registered(module, name);
    let subject = (entry.build)().subject;
    let expected = subject
        .expected
        .clone()
        .unwrap_or_else(|| panic!("{module}::{name} states an expected outcome"));
    let run = run_case(Side::Oracle, &entry, &subject, &|dirs| {
        hermetic_env(&inherited_env(), dirs)
    })
    .unwrap_or_else(|err| panic!("the oracle runs {module}::{name}: {err}"));
    (expected, run)
}

/// The fields `expected` differs from `run` in, in report order.
fn mismatched_fields(expected: &Outcome, run: &CaseRun) -> Vec<Field> {
    compare(expected, &run.before, &run.after, run.run.exit)
        .into_iter()
        .map(|mismatch| mismatch.field)
        .collect()
}

/// An expectation's listed refs.
fn listed_refs(outcome: &mut Outcome) -> &mut Vec<RefExpect> {
    match &mut outcome.refs {
        RefsExpect::Exactly { refs, .. } => refs,
        RefsExpect::Unchanged => panic!("the expectation lists its refs"),
    }
}

/// An expectation's listed files.
fn listed_files(outcome: &mut Outcome) -> &mut Vec<FileExpect> {
    match &mut outcome.files {
        FilesExpect::Exactly { files, .. } => files,
        FilesExpect::Unchanged => panic!("the expectation lists its files"),
    }
}

/// The tip expectation of the listed ref `name`.
fn listed_tip<'a>(outcome: &'a mut Outcome, name: &str) -> &'a mut Tip {
    let entry = listed_refs(outcome)
        .iter_mut()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("the expectation lists {name}"));
    &mut entry.tip
}

/// `sync::self_test`'s second new commit: the two-parent sync merge.
fn sync_merge(outcome: &mut Outcome) -> &mut CommitExpect {
    let merge = &mut outcome.new_commits[1];
    assert_eq!(merge.parents.len(), 2, "new commit 2 is the sync merge");
    merge
}

/// Changes exactly one byte, the first, by flipping its lowest bit.
fn flip_first_byte(bytes: &mut [u8]) {
    bytes[0] ^= 1;
}

/// Changes exactly one byte of an ASCII string the same way.
fn flip_first_char(text: &mut String) {
    let mut bytes = std::mem::take(text).into_bytes();
    flip_first_byte(&mut bytes);
    *text = String::from_utf8(bytes).expect("flipping the low bit of an ASCII byte keeps it ASCII");
}

const ORIGIN_HEAD: &str = "refs/remotes/origin/HEAD";
const ORIGIN_WORKSPACE: &str = "refs/remotes/origin/workspace/dev";
const LOCAL_DEVELOP: &str = "refs/heads/develop";

/// Which reported field a mutation must produce: every mismatch reported
/// must satisfy it, and there must be at least one.
type FieldTest = Box<dyn Fn(&Field) -> bool>;

fn is(field: Field) -> FieldTest {
    Box::new(move |got| *got == field)
}

/// The sync merge's `part`, reported as new commit 2.
fn merge_part(part: CommitPart) -> FieldTest {
    is(Field::Commit {
        which: "new commit 2".to_owned(),
        part,
    })
}

/// A mutation's name, the single change it makes, and the field the
/// comparison must report.
type Mutation = (&'static str, fn(&mut Outcome), FieldTest);

/// One row per field of the HARN-03 expected outcome.
fn mutations() -> Vec<Mutation> {
    vec![
        ("exit code", |o| o.exit += 1, is(Field::Exit)),
        (
            "ref set: one extra ref",
            |o| {
                listed_refs(o).push(RefExpect {
                    name: "refs/heads/extra".to_owned(),
                    tip: Tip::Unchanged,
                });
            },
            is(Field::RefSet),
        ),
        (
            "ref set: one missing ref",
            |o| listed_refs(o).retain(|entry| entry.name != ORIGIN_WORKSPACE),
            is(Field::RefSet),
        ),
        (
            "ref tip: an unchanged symbolic ref expected at a commit",
            |o| {
                let merge = sync_merge(o).clone();
                *listed_tip(o, ORIGIN_HEAD) = Tip::Commit(merge);
            },
            is(Field::RefTip(ORIGIN_HEAD.to_owned())),
        ),
        (
            "ref tip: a moved ref expected unchanged",
            |o| *listed_tip(o, LOCAL_DEVELOP) = Tip::Unchanged,
            is(Field::RefTip(LOCAL_DEVELOP.to_owned())),
        ),
        (
            "ref tip: an unchanged ref expected at another commit",
            |o| {
                let merge = sync_merge(o).clone();
                *listed_tip(o, ORIGIN_WORKSPACE) = Tip::Commit(merge);
            },
            Box::new(|got| matches!(got, Field::Commit { which, .. } if which == ORIGIN_WORKSPACE)),
        ),
        (
            "new-commit count: one fewer",
            |o| {
                o.new_commits.pop();
            },
            is(Field::NewCommitCount),
        ),
        (
            "new-commit count: one more",
            |o| {
                let extra = sync_merge(o).clone();
                o.new_commits.push(extra);
            },
            is(Field::NewCommitCount),
        ),
        (
            "new-commit order: the two swapped",
            |o| o.new_commits.swap(0, 1),
            Box::new(|got| {
                matches!(got, Field::Commit { which, .. }
                    if which == "new commit 1" || which == "new commit 2")
            }),
        ),
        (
            "commit message: one byte",
            |o| flip_first_byte(&mut sync_merge(o).message),
            merge_part(CommitPart::Message),
        ),
        (
            "commit author name: one byte",
            |o| flip_first_char(&mut sync_merge(o).author.name),
            merge_part(CommitPart::Author),
        ),
        (
            "commit author email: one byte",
            |o| flip_first_char(&mut sync_merge(o).author.email),
            merge_part(CommitPart::Author),
        ),
        (
            "commit author date: one byte",
            |o| flip_first_char(&mut sync_merge(o).author_date),
            merge_part(CommitPart::AuthorDate),
        ),
        (
            "commit committer name: one byte",
            |o| flip_first_char(&mut sync_merge(o).committer_name),
            merge_part(CommitPart::CommitterName),
        ),
        (
            "commit committer email: one byte",
            |o| flip_first_char(&mut sync_merge(o).committer_email),
            merge_part(CommitPart::CommitterEmail),
        ),
        (
            "commit parents: the two swapped",
            |o| sync_merge(o).parents.swap(0, 1),
            merge_part(CommitPart::Parents),
        ),
        (
            "commit tree path: one byte",
            |o| flip_first_byte(&mut sync_merge(o).tree[0].path),
            merge_part(CommitPart::Tree),
        ),
        (
            "commit tree mode",
            |o| sync_merge(o).tree[0].mode = "100755".to_owned(),
            merge_part(CommitPart::Tree),
        ),
        (
            "commit tree content: one byte",
            |o| flip_first_byte(&mut sync_merge(o).tree[0].content),
            merge_part(CommitPart::Tree),
        ),
        (
            "commit signature",
            |o| {
                let merge = sync_merge(o);
                merge.signed = !merge.signed;
            },
            merge_part(CommitPart::Signature),
        ),
        (
            "file path: one byte",
            |o| flip_first_byte(&mut listed_files(o)[0].path),
            is(Field::Files),
        ),
        (
            "file content: one byte",
            |o| flip_first_byte(&mut listed_files(o)[0].content),
            is(Field::Files),
        ),
        (
            "file executable bit",
            |o| {
                let file = &mut listed_files(o)[0];
                file.executable = !file.executable;
            },
            is(Field::Files),
        ),
        (
            "files: one extra",
            |o| listed_files(o).push(FileExpect::plain(WORK, b"extra.txt", b"extra\n")),
            is(Field::Files),
        ),
        (
            "files: one missing",
            |o| {
                listed_files(o).remove(0);
            },
            is(Field::Files),
        ),
    ]
}

/// The negative direction of `changing_any_expected_field_fails`: the real
/// run of `sync::self_test` meets its unmutated expectation.
#[test]
fn unchanged_expectation_has_no_mismatch() {
    let (expected, run) = oracle_run("sync", "self_test");
    assert_eq!(mismatched_fields(&expected, &run), Vec::<Field>::new());
}

/// Each single-field change to `sync::self_test`'s expectation, compared with
/// one real oracle run, is reported naming that field and nothing else.
#[test]
fn changing_any_expected_field_fails() {
    let (expected, run) = oracle_run("sync", "self_test");
    assert_eq!(
        mismatched_fields(&expected, &run),
        Vec::<Field>::new(),
        "the unmutated expectation is clean, so each mismatch below comes from its mutation"
    );
    let mut missed = Vec::new();
    for (what, mutate, reported) in mutations() {
        let mut mutated = expected.clone();
        mutate(&mut mutated);
        assert_ne!(
            mutated, expected,
            "the mutation \"{what}\" changes the expectation"
        );
        let fields = mismatched_fields(&mutated, &run);
        if fields.is_empty() || !fields.iter().all(&reported) {
            missed.push(format!("{what}: reported {fields:?}"));
        }
    }
    assert!(
        missed.is_empty(),
        "mutations not reported as their own field:\n{}",
        missed.join("\n")
    );
}

/// An empty new-commit list and an unchanged file set are compared, not
/// skipped: one extra new commit, or one stray file, is reported.
#[test]
fn empty_expectations_are_compared() {
    let (expected, mut run) = oracle_run("sync", "already_up_to_date");
    assert!(expected.new_commits.is_empty(), "no new commit is expected");
    assert_eq!(expected.files, FilesExpect::Unchanged);
    assert_eq!(mismatched_fields(&expected, &run), Vec::<Field>::new());

    let fabricated = "f".repeat(40);
    let tip = run
        .after
        .commits
        .values()
        .next()
        .expect("a ref points at a commit")
        .clone();
    run.after.commits.insert(
        fabricated.clone(),
        CommitExpect {
            message: b"chore: fabricated\n".to_vec(),
            ..tip
        },
    );
    run.after.new_commits.push(fabricated.clone());
    assert_eq!(
        mismatched_fields(&expected, &run),
        vec![Field::NewCommitCount],
        "one new commit against an expected empty list"
    );
    run.after.new_commits.pop();
    run.after.commits.remove(&fabricated);

    run.after.files.insert(
        (WORK.to_owned(), b"stray.profraw".to_vec()),
        FileState {
            content: b"profile\n".to_vec(),
            executable: false,
        },
    );
    assert_eq!(
        mismatched_fields(&expected, &run),
        vec![Field::Files],
        "one stray file against an unchanged file set"
    );
}

/// Paths and messages are observed as raw bytes: non-ASCII, newline and
/// non-UTF-8 names, and a message carrying a non-UTF-8 byte, a body and a
/// trailer, come back byte for byte in the tree, the message and the files.
#[test]
fn names_are_observed_byte_exact() {
    let names: [&[u8]; 3] = [
        b".planning/\xC3\xA9.md",
        b".planning/a\nb.md",
        b".planning/\xFF.md",
    ];
    let message: &[u8] =
        b"docs: names \xFF\n\nA body line.\n\nSigned-off-by: Ada Author <ada@example.invalid>\n";
    let dirs = ScenarioDirs::new("outcome", "byte_exact_names").expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    // Under the default UTF-8 commit encoding, git itself reads a non-UTF-8
    // byte in a message as Latin-1 and stores it re-encoded (0xFF becomes
    // 0xC3 0xBF). A declared Latin-1 encoding makes git store the bytes as
    // given, so what the observer returns can be held to them.
    let config = RepoConfig::devflow().with_local_config("i18n.commitEncoding", "ISO-8859-1");
    let mut fixture = Fixture::new(&dirs, env, config).expect("devflow fixture");
    let before = observe(&fixture, None).expect("observe before the commit");
    for name in names {
        fixture.write(WORK, name, name).expect("write a named file");
        let on_disk = fixture.checkout(WORK).join(OsStr::from_bytes(name));
        assert_eq!(
            fs::read(&on_disk).expect("read the file back by its byte name"),
            name,
            "{} is on disk under its exact bytes",
            show(name)
        );
    }
    fixture
        .commit(WORK, message)
        .expect("commit the named files");
    let after = observe(&fixture, Some(&before)).expect("observe after the commit");

    let [id] = &after.new_commits[..] else {
        panic!("one new commit, got {:?}", after.new_commits);
    };
    let raw = fixture
        .git_ok(WORK, ["cat-file", "commit", id.as_str()])
        .expect("read the raw commit");
    assert!(
        raw.ends_with(message),
        "git stored the message as given: {}",
        show(&raw)
    );
    let commit = &after.commits[id];
    assert_eq!(commit.message, message, "the message, byte for byte");
    let planning: BTreeSet<&[u8]> = commit
        .tree
        .iter()
        .map(|entry| entry.path.as_slice())
        .filter(|path| path.starts_with(b".planning/"))
        .collect();
    let mut want: BTreeSet<&[u8]> = names.into_iter().collect();
    want.extend([&b".planning/ROADMAP.md"[..], b".planning/STATE.md"]);
    assert_eq!(planning, want, "the .planning paths in the commit tree");
    for name in names {
        let entry = commit
            .tree
            .iter()
            .find(|entry| entry.path == name)
            .unwrap_or_else(|| panic!("{} is in the tree", show(name)));
        assert_eq!(entry.content, name, "{} content in the tree", show(name));
        let file = after
            .files
            .get(&(WORK.to_owned(), name.to_vec()))
            .unwrap_or_else(|| panic!("{} is observed on disk", show(name)));
        assert_eq!(file.content, name, "{} content on disk", show(name));
    }
    // A lossy decoder would have turned 0xFF into U+FFFD; its bytes must not
    // appear anywhere the observer recorded a name.
    let lossy = String::from_utf8_lossy(names[2]).into_owned().into_bytes();
    assert_ne!(lossy, names[2], "the lossy form differs from the raw bytes");
    assert!(
        !planning.contains(lossy.as_slice())
            && !after.files.contains_key(&(WORK.to_owned(), lossy.clone())),
        "the lossy name {} was observed",
        show(&lossy)
    );
}

/// Refs compare as a set keyed by name, so their order does not matter; new
/// commits compare as a sequence, so theirs does.
#[test]
fn refs_compare_as_a_set_and_commits_as_a_sequence() {
    let (expected, run) = oracle_run("sync", "self_test");
    let mut reordered = expected.clone();
    listed_refs(&mut reordered).reverse();
    assert_ne!(reordered, expected, "reversing changes the listed order");
    assert_eq!(
        mismatched_fields(&reordered, &run),
        Vec::<Field>::new(),
        "refs listed in reverse order"
    );
    let mut swapped = expected.clone();
    swapped.new_commits.swap(0, 1);
    assert_ne!(
        mismatched_fields(&swapped, &run),
        Vec::<Field>::new(),
        "new commits listed in reverse order"
    );
}

// --------------------- checkout and repository state under the Unchanged forms

/// The fields an all-`Unchanged` outcome (exit 0, refs and files unchanged,
/// no new commit) reports after `act` changes the devflow preset.
fn untouched_mismatches(name: &str, act: fn(&mut Fixture)) -> Vec<Field> {
    let dirs = ScenarioDirs::new("outcome", name).expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let mut fixture = Fixture::new(&dirs, env, RepoConfig::devflow()).expect("devflow fixture");
    let before = observe(&fixture, None).expect("observe before");
    act(&mut fixture);
    let after = observe(&fixture, Some(&before)).expect("observe after");
    let untouched = Outcome {
        exit: 0,
        refs: RefsExpect::Unchanged,
        new_commits: Vec::new(),
        files: FilesExpect::Unchanged,
    };
    compare(&untouched, &before, &after, Some(0))
        .into_iter()
        .map(|mismatch| mismatch.field)
        .collect()
}

/// The clean direction: observing twice with nothing in between reports no
/// difference, so each mismatch below comes from its change.
#[test]
fn untouched_checkout_has_no_mismatch() {
    assert_eq!(untouched_mismatches("untouched_checkout", |_| {}), []);
}

/// `git read-tree develop` writes another tree into `work`'s index and leaves
/// the files on disk, the refs and HEAD alone: only the index differs.
#[test]
fn files_unchanged_reports_an_overwritten_index() {
    let fields = untouched_mismatches("overwritten_index", |fixture| {
        fixture
            .git_ok(WORK, ["read-tree", "develop"])
            .expect("read develop's tree into the index");
    });
    assert_eq!(fields, [Field::CheckoutIndex(WORK.to_owned())]);
}

/// `git checkout --detach` leaves `work` at the same commit with the same
/// files and index: only HEAD differs.
#[test]
fn files_unchanged_reports_a_detached_head() {
    let fields = untouched_mismatches("detached_head", |fixture| {
        fixture
            .git_ok(WORK, ["checkout", "--quiet", "--detach"])
            .expect("detach HEAD");
    });
    assert_eq!(fields, [Field::CheckoutHead(WORK.to_owned())]);
}

/// A `MERGE_HEAD` left in `work`'s git dir, as an interrupted merge leaves
/// it: only the pending operations differ.
#[test]
fn files_unchanged_reports_a_pending_operation() {
    let fields = untouched_mismatches("pending_merge", |fixture| {
        let head = fixture
            .git_ok(WORK, ["rev-parse", "HEAD"])
            .expect("read HEAD");
        let marker = fixture.checkout(WORK).join(".git/MERGE_HEAD");
        fs::write(&marker, head).expect("write MERGE_HEAD");
    });
    assert_eq!(fields, [Field::CheckoutPending(WORK.to_owned())]);
}

/// A ref outside `refs/heads`, `refs/remotes` and `refs/tags`, such as a
/// backup ref, is part of the ref set.
#[test]
fn refs_unchanged_reports_a_ref_in_another_namespace() {
    let fields = untouched_mismatches("backup_ref", |fixture| {
        fixture
            .git_ok(WORK, ["update-ref", "refs/wsx/backup", "HEAD"])
            .expect("write a backup ref");
    });
    assert_eq!(fields, [Field::RefSet]);
}

/// A push to a ref no fetch refspec tracks, such as a backup ref on
/// `origin`, moves no tracking ref in `work`: only the remote repository's
/// refs differ. (A push by a URL equal to a configured remote's would also
/// update that remote's tracking ref, so it would not show this.)
#[test]
fn refs_unchanged_reports_a_push_to_an_untracked_remote_ref() {
    let fields = untouched_mismatches("untracked_push", |fixture| {
        fixture
            .git_ok(WORK, ["push", "--quiet", "origin", "HEAD:refs/wsx/backup"])
            .expect("push to an untracked ref");
    });
    assert_eq!(fields, [Field::RemoteRefs("remotes/origin.git".to_owned())]);
}

/// An upstream setting written to `work`'s local config is reported.
#[test]
fn refs_unchanged_reports_a_local_config_change() {
    let fields = untouched_mismatches("config_change", |fixture| {
        fixture
            .git_ok(
                WORK,
                ["config", "branch.workspace/dev.merge", "refs/heads/other"],
            )
            .expect("set an upstream");
    });
    assert_eq!(fields, [Field::Config]);
}

/// A commit reachable only from a detached HEAD is a new commit.
#[test]
fn a_commit_on_a_detached_head_is_new() {
    let fields = untouched_mismatches("detached_commit", |fixture| {
        fixture
            .git_ok(WORK, ["checkout", "--quiet", "--detach"])
            .expect("detach HEAD");
        fixture
            .git_ok(
                WORK,
                ["commit", "--quiet", "--allow-empty", "-m", "detached"],
            )
            .expect("commit on the detached HEAD");
    });
    assert_eq!(
        fields,
        [Field::NewCommitCount, Field::CheckoutHead(WORK.to_owned())]
    );
}

// ----------------------- checkout and repository state under the Exactly forms

/// The fields the oracle target of `module::name`'s subject reports when the
/// oracle runs it and `act` then changes the fixture, as a later step of the
/// same action would.
fn oracle_run_then(module: &str, name: &str, act: fn(&mut Fixture)) -> Vec<Field> {
    let entry = registered(module, name);
    let case = (entry.build)().subject;
    let dirs = ScenarioDirs::new("outcome", name).expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let mut fixture = Fixture::new(&dirs, env.clone(), case.config.clone()).expect("fixture");
    (case.fixture)(&mut fixture).expect("fixture steps");
    let copy = OracleCopy::prepare(&dirs, &env).expect("oracle copy");
    let before = observe(&fixture, None).expect("observe before");
    let run = oracle::run(&case.action, &fixture, &copy, &dated(&env, ACTION_TIME))
        .expect("the oracle runs the action");
    act(&mut fixture);
    let after = observe(&fixture, Some(&before)).expect("observe after");
    compare(oracle_target(&case), &before, &after, run.exit)
        .into_iter()
        .map(|mismatch| mismatch.field)
        .collect()
}

/// The clean direction: the sync as the oracle runs it meets
/// `sync::self_test`'s exact expectation, so each mismatch below comes from
/// its change.
#[test]
fn exactly_reports_nothing_for_the_sync_as_run() {
    assert_eq!(oracle_run_then("sync", "self_test", |_| {}), []);
}

/// A sync that leaves `work` detached at the merge it made changes no ref,
/// file or commit the expectation lists: only HEAD differs.
#[test]
fn files_exactly_reports_a_checkout_left_detached() {
    let fields = oracle_run_then("sync", "self_test", |fixture| {
        fixture
            .git_ok(WORK, ["checkout", "--quiet", "--detach"])
            .expect("detach HEAD");
    });
    assert_eq!(fields, [Field::CheckoutHead(WORK.to_owned())]);
}

/// A new worktree created detached, with its files in place, is reported:
/// the scenario's checkout must be on `feature/phase-1`.
#[test]
fn files_exactly_reports_a_new_checkout_left_detached() {
    let fields = oracle_run_then("worktree", "self_test", |fixture| {
        fixture
            .git_ok(
                "work/.worktrees/phase-1",
                ["checkout", "--quiet", "--detach"],
            )
            .expect("detach the new checkout's HEAD");
    });
    assert_eq!(
        fields,
        [Field::CheckoutHead("work/.worktrees/phase-1".to_owned())]
    );
}

/// `git read-tree develop` after the sync stages another tree in `work` and
/// leaves its files and HEAD alone: only the index differs.
#[test]
fn files_exactly_reports_an_index_that_differs_from_head() {
    let fields = oracle_run_then("sync", "self_test", |fixture| {
        fixture
            .git_ok(WORK, ["read-tree", "develop"])
            .expect("read develop's tree into the index");
    });
    assert_eq!(fields, [Field::CheckoutIndex(WORK.to_owned())]);
}

/// A `MERGE_HEAD` left behind by a merge that otherwise succeeded is
/// reported as a pending operation only.
#[test]
fn files_exactly_reports_a_merge_left_in_progress() {
    let fields = oracle_run_then("sync", "self_test", |fixture| {
        let head = fixture
            .git_ok(WORK, ["rev-parse", "HEAD"])
            .expect("read HEAD");
        let marker = fixture.checkout(WORK).join(".git/MERGE_HEAD");
        fs::write(&marker, head).expect("write MERGE_HEAD");
    });
    assert_eq!(fields, [Field::CheckoutPending(WORK.to_owned())]);
}

/// A checkout the expectation does not list is reported, beside its files.
#[test]
fn files_exactly_reports_an_unlisted_checkout() {
    let fields = oracle_run_then("sync", "self_test", |fixture| {
        fixture
            .git_ok(
                WORK,
                ["worktree", "add", "--quiet", ".worktrees/extra", "develop"],
            )
            .expect("add a worktree");
    });
    assert_eq!(fields, [Field::Files, Field::CheckoutSet]);
}

/// An upstream setting the sync writes is reported: the exact refs form
/// expects the local config unchanged unless it lists the entries added.
#[test]
fn refs_exactly_reports_a_local_config_change() {
    let fields = oracle_run_then("sync", "self_test", |fixture| {
        fixture
            .git_ok(
                WORK,
                ["config", "branch.workspace/dev.merge", "refs/heads/other"],
            )
            .expect("set an upstream");
    });
    assert_eq!(fields, [Field::Config]);
}

/// A push to a ref no fetch refspec tracks is reported under the exact refs
/// form too, though the work repository's refs all match.
#[test]
fn refs_exactly_reports_a_push_to_an_untracked_remote_ref() {
    let fields = oracle_run_then("sync", "self_test", |fixture| {
        fixture
            .git_ok(WORK, ["push", "--quiet", "origin", "HEAD:refs/wsx/backup"])
            .expect("push to an untracked ref");
    });
    assert_eq!(fields, [Field::RemoteRefs("remotes/origin.git".to_owned())]);
}

/// `cut::self_test` states the upstream the oracle's cut writes for the PR
/// branch. The same expectation without those entries fails on the config
/// alone, so the stated entries are compared, not ignored.
#[test]
fn refs_exactly_compares_the_config_entries_it_lists() {
    let (mut expected, run) = oracle_run("cut", "self_test");
    assert_eq!(mismatched_fields(&expected, &run), Vec::<Field>::new());
    let RefsExpect::Exactly { config_added, .. } = &mut expected.refs else {
        panic!("cut::self_test lists its refs");
    };
    assert!(!config_added.is_empty(), "the cut adds config entries");
    config_added.clear();
    assert_eq!(mismatched_fields(&expected, &run), [Field::Config]);
}
