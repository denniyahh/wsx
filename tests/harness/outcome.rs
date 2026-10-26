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

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::env::{ScenarioDirs, hermetic_env, inherited_env};
use crate::fixture::{Fixture, Ident, RepoConfig, WORK};

/// Mode bits that make a file executable for someone.
const EXECUTABLE_BITS: u32 = 0o111;

/// Everything a scenario expects an action to do.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub exit: i32,
    pub refs: RefsExpect,
    /// Commits reachable afterwards from some ref and from no ref before, in
    /// topological order, oldest first.
    pub new_commits: Vec<CommitExpect>,
    pub files: FilesExpect,
}

/// The refs under refs/heads, refs/remotes and refs/tags after the action.
#[derive(Clone, Debug, PartialEq)]
pub enum RefsExpect {
    /// Exactly these refs exist, each with the stated tip.
    Exactly(Vec<RefExpect>),
    /// The same refs exist as before, each holding what it held before
    /// (symbolic refs by target name).
    Unchanged,
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

/// The files on disk in every checkout after the action.
#[derive(Clone, Debug, PartialEq)]
pub enum FilesExpect {
    /// Exactly these files exist, across all checkouts.
    Exactly(Vec<FileExpect>),
    /// Every checkout holds the same files, with the same content and
    /// executable bit, as before.
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

/// The observable state of a fixture at one moment.
pub struct Snapshot {
    pub refs: BTreeMap<String, RefValue>,
    /// Every commit a ref points at, plus the new commits, by object id.
    pub commits: BTreeMap<String, CommitExpect>,
    /// Object ids of commits new since the previous snapshot, oldest first.
    pub new_commits: Vec<String>,
    /// Every file in every checkout, keyed by (checkout, path).
    pub files: BTreeMap<(String, Vec<u8>), FileState>,
}

/// Records the fixture's refs, commits and files. With `before`, also the
/// commits reachable now from some ref and from no ref in `before`.
pub fn observe(fixture: &Fixture, before: Option<&Snapshot>) -> Result<Snapshot, String> {
    let refs = read_refs(fixture)?;
    let new_commits = match before {
        Some(before) => read_new_commits(fixture, before, &refs)?,
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
        files: read_files(fixture)?,
    })
}

fn utf8(bytes: &[u8], what: &str) -> Result<String, String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| format!("{what} is not UTF-8: {}", show(bytes)))
}

fn read_refs(fixture: &Fixture) -> Result<BTreeMap<String, RefValue>, String> {
    let listing = fixture.git_ok(
        WORK,
        [
            "for-each-ref",
            "--format=%(refname)%00%(symref)%00%(objectname)",
            "refs/heads",
            "refs/remotes",
            "refs/tags",
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

fn read_new_commits(
    fixture: &Fixture,
    before: &Snapshot,
    refs: &BTreeMap<String, RefValue>,
) -> Result<Vec<String>, String> {
    let after_tips = object_tips(refs);
    if after_tips.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec!["rev-list", "--topo-order", "--reverse"];
    args.extend(after_tips.iter().map(|id| id.as_str()));
    args.push("--not");
    args.extend(object_tips(&before.refs).iter().map(|id| id.as_str()));
    let listing = fixture.git_ok(WORK, args)?;
    listing
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| utf8(line, "a commit id"))
        .collect()
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

/// Every checkout of the work repository, as absolute paths.
fn checkouts(fixture: &Fixture) -> Result<Vec<PathBuf>, String> {
    let listing = fixture.git_ok(WORK, ["worktree", "list", "--porcelain", "-z"])?;
    Ok(listing
        .split(|byte| *byte == 0)
        .filter_map(|field| field.strip_prefix(b"worktree "))
        .map(|path| PathBuf::from(OsStr::from_bytes(path)))
        .collect())
}

fn read_files(fixture: &Fixture) -> Result<BTreeMap<(String, Vec<u8>), FileState>, String> {
    let checkouts = checkouts(fixture)?;
    let mut files = BTreeMap::new();
    for checkout in &checkouts {
        let name = checkout.strip_prefix(&fixture.root).map_err(|_| {
            format!(
                "checkout {} is outside the scenario root",
                checkout.display()
            )
        })?;
        let name = utf8(name.as_os_str().as_bytes(), "a checkout name")?;
        let mut found = Vec::new();
        walk(checkout, &[], &checkouts, &mut found)?;
        for (path, state) in found {
            files.insert((name.clone(), path), state);
        }
    }
    Ok(files)
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
    Commit { which: String, part: CommitPart },
    Files,
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
    match refs {
        RefsExpect::Exactly(list) => compare_refs(list, before, after, &mut mismatches),
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
        }
    }
    compare_new_commits(new_commits, after, &mut mismatches);
    match files {
        FilesExpect::Exactly(list) => {
            let expected_files = list
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
        }
        FilesExpect::Unchanged => compare_files(&before.files, &after.files, &mut mismatches),
    }
    mismatches
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
/// outcome from a real run. A symbolic ref that changed has no expectation
/// form yet and is left out, and a run killed by a signal shows exit -1; the
/// comparison still reports both.
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
    Outcome {
        exit: exit.unwrap_or(-1),
        refs: RefsExpect::Exactly(refs),
        new_commits,
        files: FilesExpect::Exactly(files),
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
        refs: RefsExpect::Exactly(
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
