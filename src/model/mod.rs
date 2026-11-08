//! The repository model: the one resolved view of a repository that every
//! wsx command reads. It names the workspace branch and its tip, the
//! integration branch, the base and push remotes, and the personal-path list
//! read from `.wsx.conf` at the workspace tip in the object database.
//!
//! No gix type appears here: every read goes through `crate::git`, and paths
//! and names are raw bytes.
//!
//! Every refusal names its cause and the fix: an `Error` renders a message a
//! user can act on without reading wsx's source.

// Nothing in `main` calls this module until phase 5's `wsx check`, which
// removes this attribute. `cfg_attr(not(test), ...)`: the tests use every
// item, so on the test target the expectation would be unfulfilled.
#![cfg_attr(not(test), expect(dead_code, reason = "no caller until phase 5"))]

pub mod manifest;

use std::collections::BTreeSet;
use std::fmt;

use crate::git::{self, ObjectId, RefTarget, Repo, RootEntry};

use manifest::{EntryKind, Manifest, ParseError, Problem, covers};

/// The local config key that makes a repository configured (D-15).
pub const SOURCE_BRANCH: &str = "wsx.sourceBranch";

/// The local config key naming the remote shared branches are read from
/// (D-02).
pub const BASE_REMOTE: &str = "wsx.baseRemote";

/// The local config key naming the remote PR branches are pushed to (D-02).
pub const PUSH_REMOTE: &str = "wsx.pushRemote";

/// The remote a base or push remote defaults to when its key is unset (D-02).
pub const DEFAULT_REMOTE: &[u8] = b"origin";

/// The worktree directory when `wsx.worktreeDir` is unset (D-20).
pub const DEFAULT_WORKTREE_DIR: &[u8] = b".worktrees";

/// The manifest's name in the workspace tip's root tree.
pub const MANIFEST_NAME: &[u8] = b".wsx.conf";

/// The branch shared work is integrated on, and the commit it was read at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Integration {
    /// The bare branch name, such as `develop`.
    pub branch: Vec<u8>,
    /// The full ref it was read from, such as `refs/remotes/origin/develop`.
    pub full_ref: Vec<u8>,
    /// The commit that ref held when the model was resolved.
    pub tip: ObjectId,
}

/// A resolved repository: every answer a command needs, read once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    /// The workspace branch's short name.
    pub workspace_branch: Vec<u8>,
    /// The commit `refs/heads/<workspace_branch>` held; the manifest was
    /// read from this commit's tree.
    pub workspace_tip: ObjectId,
    /// The integration branch.
    pub integration: Integration,
    /// The remote shared branches are read from, or `None` (D-04).
    pub base_remote: Option<Vec<u8>>,
    /// The remote PR branches are pushed to, or `None` (D-04).
    pub push_remote: Option<Vec<u8>>,
    /// The directory feature worktrees live in, relative to the worktree root.
    pub worktree_dir: Vec<u8>,
    /// The manifest at the workspace tip.
    pub manifest: Manifest,
}

/// What `resolve` found: a configured repository, or one that is not
/// configured, with the model wsx could infer for it or why it could not.
#[derive(Debug)]
pub enum RepoModel {
    /// Local config sets `wsx.sourceBranch`, and the model resolved.
    Configured(Box<Model>),
    /// Local config does not set `wsx.sourceBranch` (D-15, D-16).
    NotConfigured {
        /// The model from the one inferred workspace branch, or why there is
        /// none.
        inferred: Result<Box<Model>, Error>,
    },
}

/// Why the model could not be resolved.
#[derive(Debug)]
pub enum Error {
    /// Returned when a read in the git layer fails.
    Git(git::Error),
    /// Returned when local config sets `key` more than once.
    KeyRepeated {
        key: &'static str,
        values: Vec<Vec<u8>>,
    },
    /// Returned when local config sets `key` to an empty value.
    KeyEmpty { key: &'static str },
    /// Returned when `key`'s value is not a valid branch name.
    NotABranchName { key: &'static str, value: Vec<u8> },
    /// Returned when `key` names a remote this repository does not define.
    NoSuchRemote { key: &'static str, remote: Vec<u8> },
    /// Returned when the ref `tried`, which `wsx.baseRef = value` names, is absent.
    BaseRefMissing { value: Vec<u8>, tried: Vec<u8> },
    /// Returned when no ref in `tried` names an integration branch.
    NoIntegrationBranch {
        remote: Option<Vec<u8>>,
        tried: Vec<Vec<u8>>,
    },
    /// Returned when the integration ref `full_ref` is symbolic, naming `target`.
    IntegrationRefSymbolic { full_ref: Vec<u8>, target: Vec<u8> },
    /// Returned when `refs/heads/<branch>` for `wsx.sourceBranch` does not exist.
    WorkspaceBranchMissing { branch: Vec<u8> },
    /// Returned when `wsx.sourceBranch` is unset and not exactly one branch qualifies.
    WorkspaceCandidates { candidates: Vec<Vec<u8>> },
    /// Returned when the tip of `branch` has no `.wsx.conf`.
    ManifestMissing { branch: Vec<u8> },
    /// Returned when `.wsx.conf` at the tip of `branch` is a `kind`, not a blob.
    ManifestNotABlob { branch: Vec<u8>, kind: &'static str },
    /// Returned when reading `.wsx.conf` at the tip of `branch` fails.
    ManifestUnreadable { branch: Vec<u8>, source: git::Error },
    /// Returned when `.wsx.conf` at the tip of `branch` does not parse.
    ManifestInvalid { branch: Vec<u8>, source: ParseError },
    /// Returned when `wsx.worktreeDir`'s value cannot name a directory in a tree.
    WorktreeDirInvalid { value: Vec<u8>, problem: Problem },
    /// Returned when a command needs a model and the repository is not configured.
    NotConfigured { reason: Box<Error> },
}

impl From<git::Error> for Error {
    fn from(err: git::Error) -> Error {
        Error::Git(err)
    }
}

/// `bytes` for prose in a message: git's C-style quoting, so a control byte
/// from a blob or a config value never reaches the terminal raw.
fn shown(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&git::c_quote(bytes)).into_owned()
}

/// `bytes` as an argument in a command line a message prints for the user to
/// paste: `shell_word`'s form, or `placeholder` when no shell quoting can
/// carry the bytes. Display quoting would leave a command substitution held
/// in a ref name live.
fn command_word(bytes: &[u8], placeholder: &str) -> String {
    git::shell_word(bytes).unwrap_or_else(|| placeholder.to_owned())
}

/// `items` for a message, each `shown`, `, `-separated.
fn listed(items: &[Vec<u8>]) -> String {
    items
        .iter()
        .map(|item| shown(item))
        .collect::<Vec<_>>()
        .join(", ")
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Git(err) => write!(f, "{err}"),
            Error::KeyRepeated { key, values } => write!(
                f,
                "{key} is set {} times in this repository's config ({}); it must be set once.\n\
                 To keep one value, run:\n\
                 \tgit config --local --replace-all {key} <value>",
                values.len(),
                listed(values)
            ),
            Error::KeyEmpty { key } => write!(
                f,
                "{key} is set but empty in this repository's config.\n\
                 Set it or remove it:\n\
                 \tgit config --local {key} <value>\n\
                 \tgit config --local --unset {key}"
            ),
            Error::NotABranchName { key, value } => {
                write!(f, "{key} = {} is not a valid branch name", shown(value))
            }
            Error::NoSuchRemote { key, remote } => write!(
                f,
                "{key} names the remote {}, which this repository does not define.\n\
                 Add it, or change the key:\n\
                 \tgit remote add {} <url>\n\
                 \tgit config --local {key} <remote>",
                shown(remote),
                command_word(remote, "<remote>")
            ),
            Error::BaseRefMissing { value, tried } => write!(
                f,
                "wsx.baseRef = {}, but {} does not exist, and wsx does not fall back to another branch.\n\
                 wsx.baseRef is a bare branch name resolved on the base remote \
                 (wsx.baseRemote, default origin), so origin/develop means a branch literally named that.",
                shown(value),
                shown(tried)
            ),
            Error::NoIntegrationBranch { remote, tried } => {
                writeln!(
                    f,
                    "no integration branch: none of {} names an integration branch.",
                    listed(tried)
                )?;
                match remote {
                    Some(remote) => write!(
                        f,
                        "Fix it with one of:\n\
                         \tgit remote set-head {} --auto\n\
                         \tgit config --local wsx.baseRef <branch>",
                        command_word(remote, "<remote>")
                    ),
                    None => f.write_str(
                        "Name it with:\n\
                         \tgit config --local wsx.baseRef <branch>",
                    ),
                }
            }
            Error::IntegrationRefSymbolic { full_ref, target } => write!(
                f,
                "{} is a symbolic ref to {}, so it could move the base away from where the \
                 integration branch lives; wsx reads the base only from a ref that holds a commit.\n\
                 Remove it, or name the branch to use:\n\
                 \tgit update-ref --no-deref -d {}\n\
                 \tgit config --local wsx.baseRef <branch>",
                shown(full_ref),
                shown(target),
                command_word(full_ref, "<ref>")
            ),
            Error::WorkspaceBranchMissing { branch } => write!(
                f,
                "wsx.sourceBranch = {branch}, but refs/heads/{branch} does not exist in this repository.\n\
                 Create or fetch the branch, or change wsx.sourceBranch.",
                branch = shown(branch)
            ),
            Error::WorkspaceCandidates { candidates } if candidates.is_empty() => f.write_str(
                "no workspace branch: wsx.sourceBranch is not set and there is no local workspace/* or personal/* branch.\n\
                 Run wsx init.",
            ),
            Error::WorkspaceCandidates { candidates } => write!(
                f,
                "cannot choose a workspace branch: wsx.sourceBranch is not set and several branches qualify: {}.\n\
                 Name one with:\n\
                 \tgit config --local wsx.sourceBranch <branch>",
                listed(candidates)
            ),
            Error::ManifestMissing { branch } => write!(
                f,
                "{branch} has no .wsx.conf at its tip, so wsx cannot tell which paths are personal.\n\
                 Commit one on {branch}, or run wsx init.",
                branch = shown(branch)
            ),
            Error::ManifestNotABlob { branch, kind } => write!(
                f,
                ".wsx.conf at the tip of {} is a {kind}, not a regular file, \
                 so wsx cannot read it as a manifest.",
                shown(branch)
            ),
            Error::ManifestUnreadable { branch, source } => write!(
                f,
                ".wsx.conf at the tip of {} cannot be read: {source}",
                shown(branch)
            ),
            Error::ManifestInvalid { branch, source } => {
                write!(f, ".wsx.conf at the tip of {}: {source}", shown(branch))
            }
            Error::WorktreeDirInvalid { value, problem } => write!(
                f,
                "wsx.worktreeDir = {} cannot name a directory in a git tree: {problem}",
                shown(value)
            ),
            Error::NotConfigured { reason } => write!(
                f,
                "this repository is not configured for wsx: {reason}\n\
                 Run wsx init."
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Resolves the model of `repo`. `recorded` is the set of branches `wsx new`
/// recorded, which never count as workspace-branch candidates (D-17).
///
/// `Err` means exactly that the repository is configured and the model
/// fails closed; a repository that is not configured is a value.
pub fn resolve(repo: &Repo, _recorded: &BTreeSet<Vec<u8>>) -> Result<RepoModel, Error> {
    match single(repo, SOURCE_BRANCH)? {
        Some(branch) => {
            // 03.1 D-04: after this check the branch is read only as the one
            // full ref `refs/heads/<branch>`, never through a DWIM ladder.
            if !git::is_branch_name(&branch) {
                return Err(Error::NotABranchName {
                    key: SOURCE_BRANCH,
                    value: branch,
                });
            }
            Ok(RepoModel::Configured(Box::new(build(repo, branch)?)))
        }
        None => Ok(RepoModel::NotConfigured {
            inferred: Err(Error::WorkspaceCandidates {
                candidates: Vec::new(),
            }),
        }),
    }
}

/// The one value of `key` in this repository's own config, or `None` when
/// it is unset. A key set more than once is `Error::KeyRepeated` with every
/// value, not git's last-one-wins, and a single empty value (`key =`, or a
/// key line with no `=`) is `Error::KeyEmpty` (D-18). Only `key` is read:
/// another `wsx.*` key, empty or repeated, is not this read's concern
/// (GUARD-05 turns a guard off with an empty `wsx.protectedBranches`).
fn single(repo: &Repo, key: &'static str) -> Result<Option<Vec<u8>>, Error> {
    let mut values = repo.local_config_values(key);
    match values.len() {
        0 => Ok(None),
        1 => {
            let value = values.remove(0);
            if value.is_empty() {
                return Err(Error::KeyEmpty { key });
            }
            Ok(Some(value))
        }
        _ => Err(Error::KeyRepeated { key, values }),
    }
}

/// `refs/heads/<branch>`.
fn branch_ref(branch: &[u8]) -> Vec<u8> {
    [b"refs/heads/".as_slice(), branch].concat()
}

/// The remote a base or push remote resolves to: `origin` when this
/// repository defines it, else none (D-02, D-04).
fn default_remote(repo: &Repo) -> Option<Vec<u8>> {
    repo.remote_configured(DEFAULT_REMOTE)
        .then(|| DEFAULT_REMOTE.to_vec())
}

/// The remote `key` (`wsx.baseRemote` or `wsx.pushRemote`) names, read on
/// its own (D-02). A named remote this repository does not define is
/// `Error::NoSuchRemote`, with no fallback to `origin` (D-03); an unset key
/// is `default_remote`. Nothing else is consulted: not git's
/// `remote.pushDefault`, which says where any `git push` goes, and not an
/// `upstream` remote (D-05), since pushing personal history to a remote the
/// owner did not name cannot be undone.
fn remote(repo: &Repo, key: &'static str) -> Result<Option<Vec<u8>>, Error> {
    match single(repo, key)? {
        Some(name) if repo.remote_configured(&name) => Ok(Some(name)),
        Some(remote) => Err(Error::NoSuchRemote { key, remote }),
        None => Ok(default_remote(repo)),
    }
}

/// The model with `branch` as the workspace branch. The manifest comes from
/// the root tree of the commit `refs/heads/<branch>` holds, read from the
/// object database, so neither an uncommitted edit nor the checked-out
/// branch changes which paths are personal.
fn build(repo: &Repo, branch: Vec<u8>) -> Result<Model, Error> {
    let full = branch_ref(&branch);
    if repo.find_ref(&full)?.is_none() {
        return Err(Error::WorkspaceBranchMissing { branch });
    }
    let base_remote = remote(repo, BASE_REMOTE)?;
    let push_remote = remote(repo, PUSH_REMOTE)?;
    let integration = integration(repo, base_remote.as_deref())?;
    let worktree_dir = DEFAULT_WORKTREE_DIR.to_vec();
    let workspace_tip = repo.ref_commit(&full)?;
    let manifest = match repo.root_entry(&workspace_tip, MANIFEST_NAME)? {
        RootEntry::Absent => return Err(Error::ManifestMissing { branch }),
        RootEntry::Blob(bytes) => match manifest::parse(&bytes) {
            Ok(manifest) => manifest,
            Err(source) => return Err(Error::ManifestInvalid { branch, source }),
        },
    };
    Ok(Model {
        workspace_branch: branch,
        workspace_tip,
        integration,
        base_remote,
        push_remote,
        worktree_dir,
        manifest,
    })
}

/// The integration branches the chain tries, in order, before the base
/// remote's default (D-06, D-07).
const CHAIN: [&[u8]; 2] = [b"develop", b"next"];

/// The integration branch: `develop`, then `next`, then the branch the base
/// remote's `HEAD` names, each read from the base remote's tracking refs;
/// with no base remote, the local `develop`, then the local `next` (D-06,
/// D-07). Nothing else is tried: no `main` or `master`, no network call
/// (D-09), and a stale `<remote>/HEAD` is used as it is (D-10).
///
/// Every rung is one exact read of a full ref through `find_ref`. A
/// symbolic ref on a rung is `Error::IntegrationRefSymbolic`, never followed
/// and never skipped, since following it can land on a local commit
/// (D-06). A rung's tip is the id that same read returned, peeled, so the
/// ref checked is the ref whose commit is used; a ref that exists and does
/// not peel to a commit is an error, with no fall-through to the next rung.
fn integration(repo: &Repo, base: Option<&[u8]>) -> Result<Integration, Error> {
    let prefix = match base {
        Some(remote) => [b"refs/remotes/".as_slice(), remote, b"/".as_slice()].concat(),
        None => b"refs/heads/".to_vec(),
    };
    let full = |branch: &[u8]| [prefix.as_slice(), branch].concat();
    let found = |branch: &[u8]| -> Result<Option<Integration>, Error> {
        let full_ref = full(branch);
        match repo.find_ref(&full_ref)? {
            None => Ok(None),
            Some(RefTarget::Symbolic(target)) => {
                Err(Error::IntegrationRefSymbolic { full_ref, target })
            }
            Some(RefTarget::Object(id)) => Ok(Some(Integration {
                branch: branch.to_vec(),
                tip: repo.peel_commit(&id)?,
                full_ref,
            })),
        }
    };
    let mut tried = Vec::new();
    for branch in CHAIN {
        if let Some(integration) = found(branch)? {
            return Ok(integration);
        }
        tried.push(full(branch));
    }
    if base.is_some() {
        // The remote default counts only as a symbolic ref naming a branch
        // under the base remote; an object id, a target elsewhere and a
        // dangling target each leave it in `tried`.
        let head = full(b"HEAD");
        if let Some(RefTarget::Symbolic(target)) = repo.find_ref(&head)?
            && let Some(branch) = target.strip_prefix(prefix.as_slice())
            && let Some(integration) = found(branch)?
        {
            return Ok(integration);
        }
        tried.push(head);
    }
    Err(Error::NoIntegrationBranch {
        remote: base.map(<[u8]>::to_vec),
        tried,
    })
}

impl Model {
    /// The paths that never reach a shared branch: `.wsx.conf`, then the
    /// worktree directory, then each `only` path in file order, duplicates
    /// kept. A `modified` path is shared, so it is not listed.
    pub fn personal_paths(&self) -> Vec<&[u8]> {
        let mut paths = vec![MANIFEST_NAME, self.worktree_dir.as_slice()];
        paths.extend(
            self.manifest
                .entries
                .iter()
                .filter(|entry| entry.kind == EntryKind::Only)
                .map(|entry| entry.path.as_slice()),
        );
        paths
    }

    /// Whether some entry of `personal_paths()` covers `path`.
    pub fn is_personal(&self, path: &[u8]) -> bool {
        self.personal_paths()
            .into_iter()
            .any(|entry| covers(entry, path))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};
    use std::process::Output;

    use crate::git::Repo;
    use crate::git::testing::{self, Scratch, T0, TICK, commit_all, git_ok, write};

    use crate::git;

    use super::manifest::{ParseError, Problem};
    use super::{BASE_REMOTE, Error, Integration, Model, PUSH_REMOTE, RepoModel, resolve};

    /// Fails the test with `err` when a fixture step fails.
    fn ok<T>(result: Result<T, String>) -> T {
        result.unwrap_or_else(|err| panic!("{err}"))
    }

    /// `<root>/work`, a clone of `<root>/source`. In `source`, `main`,
    /// `develop` and `next` each hold their own commit, and none holds
    /// `.wsx.conf`. In `work`, `workspace/dev` (from `main`) commits
    /// `.wsx.conf` = `only .planning`, and `main` stays checked out, so the
    /// working tree has no `.wsx.conf`. The clone has
    /// `refs/remotes/origin/{HEAD,main,develop,next}`, `HEAD` symbolic to
    /// `refs/remotes/origin/main`. Canonical, as `Repo` compares paths.
    fn repo_with_remote(scratch: &Scratch) -> PathBuf {
        let source = scratch.root.join("source");
        std::fs::create_dir_all(&source)
            .unwrap_or_else(|err| panic!("create {}: {err}", source.display()));
        ok(git_ok(scratch, &source, ["init", "-q", "-b", "main"]));
        ok(write(&source.join("a.txt"), b"a\n"));
        ok(commit_all(scratch, &source, "main", T0));
        ok(git_ok(scratch, &source, ["switch", "-q", "-c", "develop"]));
        ok(write(&source.join("d.txt"), b"d\n"));
        ok(commit_all(scratch, &source, "develop", T0 + TICK));
        ok(git_ok(
            scratch,
            &source,
            ["switch", "-q", "-c", "next", "main"],
        ));
        ok(write(&source.join("n.txt"), b"n\n"));
        ok(commit_all(scratch, &source, "next", T0 + 2 * TICK));
        ok(git_ok(scratch, &source, ["switch", "-q", "main"]));

        ok(git_ok(
            scratch,
            &scratch.root,
            ["clone", "-q", "source", "work"],
        ));
        let work = scratch.root.join("work");
        ok(git_ok(
            scratch,
            &work,
            ["switch", "-q", "-c", "workspace/dev"],
        ));
        ok(write(&work.join(".wsx.conf"), b"only .planning\n"));
        ok(commit_all(scratch, &work, "manifest", T0 + 3 * TICK));
        ok(git_ok(scratch, &work, ["switch", "-q", "main"]));
        assert!(
            !work.join(".wsx.conf").exists(),
            "the fixture's working tree, on main, holds .wsx.conf"
        );
        work.canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize {}: {err}", work.display()))
    }

    /// `git config --local <key> <value>` in `work`. Run it before
    /// `Repo::discover`: a `Repo` keeps the config it was opened with.
    fn set_config(scratch: &Scratch, work: &Path, key: &str, value: &str) {
        ok(git_ok(scratch, work, ["config", "--local", key, value]));
    }

    /// The id `git show-ref --verify <full>` prints in `work`.
    fn tip(scratch: &Scratch, work: &Path, full: &str) -> String {
        let stdout = ok(git_ok(scratch, work, ["show-ref", "--verify", full]));
        String::from_utf8_lossy(&stdout)
            .split_whitespace()
            .next()
            .unwrap_or_else(|| panic!("git show-ref --verify {full} printed no id"))
            .to_owned()
    }

    /// The configured model `resolve` gives for `work`; fails the test on
    /// any other answer.
    fn configured(work: &Path) -> Model {
        let repo = Repo::discover(work).expect("discover the fixture");
        match resolve(&repo, &BTreeSet::new()) {
            Ok(RepoModel::Configured(model)) => *model,
            other => panic!("resolve: expected RepoModel::Configured, got {other:?}"),
        }
    }

    #[test]
    fn configured_repo_resolves_from_the_object_database() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-configured").expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        let develop = tip(&scratch, &work, "refs/remotes/origin/develop");
        let next = tip(&scratch, &work, "refs/remotes/origin/next");
        let main = tip(&scratch, &work, "refs/remotes/origin/main");
        assert!(
            develop != next && next != main && develop != main,
            "the fixture's rungs share a commit: develop {develop}, next {next}, main {main}"
        );

        let model = configured(&work);
        assert_eq!(
            model.workspace_branch,
            b"workspace/dev".to_vec(),
            "workspace branch"
        );
        assert_eq!(
            model.workspace_tip.to_string(),
            tip(&scratch, &work, "refs/heads/workspace/dev"),
            "workspace tip against git show-ref"
        );
        assert_eq!(
            model.integration.branch,
            b"develop".to_vec(),
            "integration branch"
        );
        assert_eq!(
            model.integration.full_ref,
            b"refs/remotes/origin/develop".to_vec(),
            "integration ref"
        );
        let integration_tip = model.integration.tip.to_string();
        assert_eq!(
            integration_tip, develop,
            "integration tip against develop's"
        );
        assert_ne!(integration_tip, next, "integration tip is next's");
        assert_ne!(integration_tip, main, "integration tip is main's");
        assert_eq!(model.base_remote, Some(b"origin".to_vec()), "base remote");
        assert_eq!(model.push_remote, Some(b"origin".to_vec()), "push remote");
        assert_eq!(
            model.personal_paths(),
            vec![&b".wsx.conf"[..], &b".worktrees"[..], &b".planning"[..]],
            "personal paths"
        );
        assert!(
            model.is_personal(b".planning/x"),
            "is_personal(.planning/x)"
        );
        assert!(model.is_personal(b".wsx.conf"), "is_personal(.wsx.conf)");
        assert!(!model.is_personal(b".planningX"), "is_personal(.planningX)");
    }

    #[test]
    fn model_moves_with_the_workspace_tip() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-moves").expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        let first = configured(&work);

        ok(git_ok(&scratch, &work, ["switch", "-q", "workspace/dev"]));
        ok(write(&work.join(".wsx.conf"), b"only notes\n"));
        ok(commit_all(&scratch, &work, "notes", T0 + 4 * TICK));
        ok(git_ok(&scratch, &work, ["switch", "-q", "main"]));

        let moved = configured(&work);
        assert_eq!(
            moved.workspace_tip.to_string(),
            tip(&scratch, &work, "refs/heads/workspace/dev"),
            "workspace tip after the commit, against git show-ref"
        );
        assert_ne!(
            moved.workspace_tip, first.workspace_tip,
            "workspace tip before and after the commit"
        );
        assert!(
            !moved.is_personal(b".planning/x"),
            "is_personal(.planning/x) after the manifest dropped it"
        );
        assert!(
            moved.is_personal(b"notes/x"),
            "is_personal(notes/x) after the manifest added it"
        );
    }

    #[test]
    fn no_source_branch_and_no_candidate_is_not_configured() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-not-configured").expect("create scratch");
        let work = repo_with_remote(&scratch);
        ok(git_ok(
            &scratch,
            &work,
            ["branch", "-q", "-D", "workspace/dev"],
        ));
        let repo = Repo::discover(&work).expect("discover the fixture");
        match resolve(&repo, &BTreeSet::new()) {
            Ok(RepoModel::NotConfigured { inferred }) => assert!(
                inferred.is_err(),
                "inferred a model with no candidate branch: {inferred:?}"
            ),
            other => panic!("resolve: expected RepoModel::NotConfigured, got {other:?}"),
        }
    }

    /// A `ParseError` for the line `sync b` at line 2.
    fn unknown_kind() -> ParseError {
        ParseError {
            line: 2,
            text: b"sync b".to_vec(),
            problem: Problem::UnknownKind,
        }
    }

    #[test]
    fn git_failure_renders_the_inner_message() {
        let err = Error::Git(git::Error::Gix {
            what: "read refs/heads/x".to_owned(),
            message: "m".to_owned(),
        });
        assert_eq!(err.to_string(), "read refs/heads/x: m");
    }

    #[test]
    fn repeated_key_message_lists_the_values_and_the_fix() {
        let err = Error::KeyRepeated {
            key: "wsx.sourceBranch",
            values: vec![b"workspace/a".to_vec(), b"workspace/b".to_vec()],
        };
        assert_eq!(
            err.to_string(),
            "wsx.sourceBranch is set 2 times in this repository's config (workspace/a, workspace/b); \
             it must be set once.\n\
             To keep one value, run:\n\
             \tgit config --local --replace-all wsx.sourceBranch <value>"
        );
    }

    #[test]
    fn empty_key_message_names_the_key_and_the_fix() {
        let err = Error::KeyEmpty {
            key: "wsx.baseRemote",
        };
        assert_eq!(
            err.to_string(),
            "wsx.baseRemote is set but empty in this repository's config.\n\
             Set it or remove it:\n\
             \tgit config --local wsx.baseRemote <value>\n\
             \tgit config --local --unset wsx.baseRemote"
        );
    }

    #[test]
    fn not_a_branch_name_message_names_the_key_and_value() {
        let err = Error::NotABranchName {
            key: "wsx.sourceBranch",
            value: b"a..b".to_vec(),
        };
        assert_eq!(
            err.to_string(),
            "wsx.sourceBranch = a..b is not a valid branch name"
        );
    }

    #[test]
    fn no_such_remote_message_names_the_key_and_the_remote() {
        let err = Error::NoSuchRemote {
            key: "wsx.baseRemote",
            remote: b"upstream".to_vec(),
        };
        assert_eq!(
            err.to_string(),
            "wsx.baseRemote names the remote upstream, which this repository does not define.\n\
             Add it, or change the key:\n\
             \tgit remote add upstream <url>\n\
             \tgit config --local wsx.baseRemote <remote>"
        );
    }

    #[test]
    fn base_ref_missing_message_says_bare_branch_name() {
        let err = Error::BaseRefMissing {
            value: b"upstream/next".to_vec(),
            tried: b"refs/remotes/origin/upstream/next".to_vec(),
        };
        assert_eq!(
            err.to_string(),
            "wsx.baseRef = upstream/next, but refs/remotes/origin/upstream/next does not exist, \
             and wsx does not fall back to another branch.\n\
             wsx.baseRef is a bare branch name resolved on the base remote \
             (wsx.baseRemote, default origin), so origin/develop means a branch literally named that."
        );
    }

    #[test]
    fn no_integration_branch_message_lists_refs_and_both_fixes() {
        let err = Error::NoIntegrationBranch {
            remote: Some(b"origin".to_vec()),
            tried: vec![
                b"refs/remotes/origin/develop".to_vec(),
                b"refs/remotes/origin/next".to_vec(),
                b"refs/remotes/origin/HEAD".to_vec(),
            ],
        };
        assert_eq!(
            err.to_string(),
            "no integration branch: none of refs/remotes/origin/develop, refs/remotes/origin/next, \
             refs/remotes/origin/HEAD names an integration branch.\n\
             Fix it with one of:\n\
             \tgit remote set-head origin --auto\n\
             \tgit config --local wsx.baseRef <branch>"
        );
    }

    #[test]
    fn no_integration_branch_without_a_remote_names_only_base_ref() {
        let err = Error::NoIntegrationBranch {
            remote: None,
            tried: vec![b"refs/heads/develop".to_vec(), b"refs/heads/next".to_vec()],
        };
        assert_eq!(
            err.to_string(),
            "no integration branch: none of refs/heads/develop, refs/heads/next \
             names an integration branch.\n\
             Name it with:\n\
             \tgit config --local wsx.baseRef <branch>"
        );
    }

    #[test]
    fn integration_ref_symbolic_message_names_both_refs_and_the_fixes() {
        let err = Error::IntegrationRefSymbolic {
            full_ref: b"refs/remotes/origin/develop".to_vec(),
            target: b"refs/remotes/origin/main".to_vec(),
        };
        assert_eq!(
            err.to_string(),
            "refs/remotes/origin/develop is a symbolic ref to refs/remotes/origin/main, \
             so it could move the base away from where the integration branch lives; \
             wsx reads the base only from a ref that holds a commit.\n\
             Remove it, or name the branch to use:\n\
             \tgit update-ref --no-deref -d refs/remotes/origin/develop\n\
             \tgit config --local wsx.baseRef <branch>"
        );
    }

    #[test]
    fn workspace_branch_missing_message_names_the_ref() {
        let err = Error::WorkspaceBranchMissing {
            branch: b"workspace/dev".to_vec(),
        };
        assert_eq!(
            err.to_string(),
            "wsx.sourceBranch = workspace/dev, but refs/heads/workspace/dev does not exist \
             in this repository.\n\
             Create or fetch the branch, or change wsx.sourceBranch."
        );
    }

    #[test]
    fn zero_candidates_message_says_run_init() {
        let err = Error::WorkspaceCandidates {
            candidates: Vec::new(),
        };
        assert_eq!(
            err.to_string(),
            "no workspace branch: wsx.sourceBranch is not set and there is no local \
             workspace/* or personal/* branch.\n\
             Run wsx init."
        );
    }

    #[test]
    fn several_candidates_message_lists_them() {
        let err = Error::WorkspaceCandidates {
            candidates: vec![b"personal/b".to_vec(), b"workspace/a".to_vec()],
        };
        assert_eq!(
            err.to_string(),
            "cannot choose a workspace branch: wsx.sourceBranch is not set and several branches \
             qualify: personal/b, workspace/a.\n\
             Name one with:\n\
             \tgit config --local wsx.sourceBranch <branch>"
        );
    }

    #[test]
    fn manifest_missing_message_names_the_branch() {
        let err = Error::ManifestMissing {
            branch: b"workspace/dev".to_vec(),
        };
        assert_eq!(
            err.to_string(),
            "workspace/dev has no .wsx.conf at its tip, so wsx cannot tell which paths are personal.\n\
             Commit one on workspace/dev, or run wsx init."
        );
    }

    #[test]
    fn manifest_not_a_blob_message_names_the_kind() {
        let err = Error::ManifestNotABlob {
            branch: b"workspace/dev".to_vec(),
            kind: "symlink",
        };
        assert_eq!(
            err.to_string(),
            ".wsx.conf at the tip of workspace/dev is a symlink, not a regular file, \
             so wsx cannot read it as a manifest."
        );
    }

    #[test]
    fn manifest_unreadable_message_names_the_branch_and_the_cause() {
        let err = Error::ManifestUnreadable {
            branch: b"workspace/dev".to_vec(),
            source: git::Error::Gix {
                what: "read .wsx.conf".to_owned(),
                message: "object missing".to_owned(),
            },
        };
        assert_eq!(
            err.to_string(),
            ".wsx.conf at the tip of workspace/dev cannot be read: read .wsx.conf: object missing"
        );
    }

    #[test]
    fn manifest_invalid_message_names_the_line() {
        let err = Error::ManifestInvalid {
            branch: b"workspace/dev".to_vec(),
            source: unknown_kind(),
        };
        assert_eq!(
            err.to_string(),
            ".wsx.conf at the tip of workspace/dev: line 2: \
             expected a comment or an \"only <path>\" or \"modified <path>\" line: sync b"
        );
    }

    #[test]
    fn worktree_dir_invalid_message_names_the_value() {
        let err = Error::WorktreeDirInvalid {
            value: b"trees".to_vec(),
            problem: Problem::UnknownKind,
        };
        assert_eq!(
            err.to_string(),
            "wsx.worktreeDir = trees cannot name a directory in a git tree: \
             expected a comment or an \"only <path>\" or \"modified <path>\" line"
        );
    }

    #[test]
    fn not_configured_message_wraps_the_reason_and_says_run_init() {
        let err = Error::NotConfigured {
            reason: Box::new(Error::ManifestInvalid {
                branch: b"personal/x".to_vec(),
                source: unknown_kind(),
            }),
        };
        assert_eq!(
            err.to_string(),
            "this repository is not configured for wsx: .wsx.conf at the tip of personal/x: \
             line 2: expected a comment or an \"only <path>\" or \"modified <path>\" line: sync b\n\
             Run wsx init."
        );
    }

    #[test]
    fn escape_bytes_in_a_value_render_quoted() {
        let err = Error::NotABranchName {
            key: "wsx.sourceBranch",
            value: b"a\x1b[31mb".to_vec(),
        };
        let rendered = err.to_string();
        assert!(
            !rendered.contains('\x1b'),
            "a raw ESC byte reached the message: {rendered:?}"
        );
        assert_eq!(
            rendered,
            r#"wsx.sourceBranch = "a\033[31mb" is not a valid branch name"#
        );
    }

    #[test]
    fn command_lines_quote_a_value_for_the_shell() {
        let remote = Error::NoSuchRemote {
            key: "wsx.baseRemote",
            remote: b"$(x)".to_vec(),
        };
        assert_eq!(
            remote.to_string(),
            "wsx.baseRemote names the remote $(x), which this repository does not define.\n\
             Add it, or change the key:\n\
             \tgit remote add '$(x)' <url>\n\
             \tgit config --local wsx.baseRemote <remote>"
        );

        let symbolic = Error::IntegrationRefSymbolic {
            full_ref: b"refs/remotes/origin/a\x1bb".to_vec(),
            target: b"refs/remotes/origin/main".to_vec(),
        };
        let rendered = symbolic.to_string();
        assert!(
            !rendered.contains('\x1b'),
            "a raw ESC byte reached the message: {rendered:?}"
        );
        assert_eq!(
            rendered,
            "\"refs/remotes/origin/a\\033b\" is a symbolic ref to refs/remotes/origin/main, \
             so it could move the base away from where the integration branch lives; \
             wsx reads the base only from a ref that holds a commit.\n\
             Remove it, or name the branch to use:\n\
             \tgit update-ref --no-deref -d <ref>\n\
             \tgit config --local wsx.baseRef <branch>"
        );
    }

    /// The variable a parent hands `state_child`: the directory it discovers
    /// from.
    const START_VAR: &str = "WSX_TEST_START";

    /// The full libtest path of a child half in this module.
    fn child_path(name: &str) -> String {
        format!("model::tests::{name}")
    }

    /// Appends `text` to `<work>/.git/config`. Run it before
    /// `Repo::discover`.
    fn append_config(work: &Path, text: &[u8]) {
        let path = work.join(".git/config");
        let mut config =
            std::fs::read(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        config.extend_from_slice(text);
        ok(write(&path, &config));
    }

    /// What `resolve` answers for `work`, discovered in this process.
    fn resolved(work: &Path) -> Result<RepoModel, Error> {
        let repo = Repo::discover(work).expect("discover the fixture");
        resolve(&repo, &BTreeSet::new())
    }

    /// `git config <args>` in `work`: its exit code and stdout.
    fn git_config(scratch: &Scratch, work: &Path, args: &[&str]) -> (Option<i32>, Vec<u8>) {
        let mut argv = vec!["config"];
        argv.extend_from_slice(args);
        let output = ok(testing::git(scratch, work, &argv));
        (output.status.code(), output.stdout)
    }

    /// Runs `state_child` on `work` with the scratch environment.
    fn state_child_output(scratch: &Scratch, work: &Path) -> Output {
        let mut env = scratch.env.clone();
        env.push((START_VAR.into(), work.into()));
        ok(testing::reexec(
            &child_path("state_child"),
            &env,
            &scratch.root,
            b"",
        ))
    }

    /// Runs `state_child` on `work` with the scratch environment and returns
    /// its `state` marker, with its `error` marker when it printed one.
    fn child_state(scratch: &Scratch, work: &Path) -> String {
        let child = state_child_output(scratch, work);
        let state = ok(testing::marker(&child, "state"));
        match testing::marker(&child, "error") {
            Ok(error) => format!("{state}: {error}"),
            Err(_) => state,
        }
    }

    /// A repeated `wsx.sourceBranch` is refused, naming every value in order,
    /// rather than resolved by git's last-one-wins (D-18): the key is set, so
    /// the repository is configured (D-15) and the refusal is an `Err`.
    #[test]
    fn repeated_source_branch_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-key-repeated").expect("create scratch");
        let work = repo_with_remote(&scratch);
        for value in ["workspace/other", "workspace/dev"] {
            ok(git_ok(
                &scratch,
                &work,
                ["config", "--local", "--add", "wsx.sourceBranch", value],
            ));
        }
        match resolved(&work) {
            Err(Error::KeyRepeated { key, values }) => {
                assert_eq!(key, "wsx.sourceBranch");
                assert_eq!(
                    values,
                    [b"workspace/other".to_vec(), b"workspace/dev".to_vec()]
                );
            }
            other => panic!("resolve: expected Err(KeyRepeated), got {other:?}"),
        }
    }

    /// `wsx.sourceBranch=` is refused as empty (D-18), not read as unset.
    #[test]
    fn empty_source_branch_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-key-empty").expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", "");
        match resolved(&work) {
            Err(Error::KeyEmpty { key }) => assert_eq!(key, "wsx.sourceBranch"),
            other => panic!("resolve: expected Err(KeyEmpty), got {other:?}"),
        }
    }

    /// A `sourceBranch` line with no `=` under `[wsx]` is the same empty
    /// value, as `git config --local --get` prints it, and is refused alike.
    #[test]
    fn source_branch_without_a_value_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-key-no-value").expect("create scratch");
        let work = repo_with_remote(&scratch);
        append_config(&work, b"[wsx]\n\tsourceBranch\n");
        assert_eq!(
            git_config(&scratch, &work, &["--local", "--get", "wsx.sourceBranch"]),
            (Some(0), b"\n".to_vec())
        );
        match resolved(&work) {
            Err(Error::KeyEmpty { key }) => assert_eq!(key, "wsx.sourceBranch"),
            other => panic!("resolve: expected Err(KeyEmpty), got {other:?}"),
        }
    }

    /// D-18 applies to each key this phase reads, not to the `wsx` section:
    /// an empty `wsx.protectedBranches` (GUARD-05's way to turn that guard
    /// off) and a repeated `wsx.other`, both present as git reads them,
    /// leave a configured repository configured.
    #[test]
    fn keys_this_phase_does_not_read_are_not_refused() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-other-keys").expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        set_config(&scratch, &work, "wsx.protectedBranches", "");
        for value in ["a", "b"] {
            ok(git_ok(
                &scratch,
                &work,
                ["config", "--local", "--add", "wsx.other", value],
            ));
        }
        assert_eq!(
            git_config(
                &scratch,
                &work,
                &["--local", "--get-all", "wsx.protectedBranches"]
            ),
            (Some(0), b"\n".to_vec())
        );
        assert_eq!(
            git_config(&scratch, &work, &["--local", "--get-all", "wsx.other"]),
            (Some(0), b"a\nb\n".to_vec())
        );
        configured(&work);
    }

    /// `wsx.sourceBranch` in global config leaves the repository not
    /// configured (D-01): plain `git config --get` returns it and
    /// `--local --get` does not, and `resolve` in a child with the scratch
    /// environment reports `NotConfigured`.
    #[test]
    fn global_source_branch_does_not_configure() {
        let scratch = Scratch::new("model-global-key").expect("create scratch");
        let work = repo_with_remote(&scratch);
        let global = scratch.home.join(".gitconfig");
        let mut config = std::fs::read(&global).expect("read the scratch global config");
        config.extend_from_slice(b"[wsx]\n\tsourceBranch = workspace/dev\n");
        ok(write(&global, &config));

        assert_eq!(
            git_config(&scratch, &work, &["--get", "wsx.sourceBranch"]),
            (Some(0), b"workspace/dev\n".to_vec())
        );
        assert_eq!(
            git_config(&scratch, &work, &["--local", "--get", "wsx.sourceBranch"]),
            (Some(1), Vec::new())
        );
        assert_eq!(child_state(&scratch, &work), "not-configured");
    }

    /// The control for the child: the same child reports `configured` for
    /// the value set in the repository's own config.
    #[test]
    fn local_source_branch_configures_the_same_child() {
        let scratch = Scratch::new("model-local-key-child").expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        assert_eq!(child_state(&scratch, &work), "configured");
    }

    /// `wsx.sourceBranch` in an `include.path` file leaves the repository
    /// not configured (D-01, A1), while `git config --local --includes`
    /// returns it.
    #[test]
    fn included_source_branch_does_not_configure() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-included-key").expect("create scratch");
        let work = repo_with_remote(&scratch);
        ok(write(
            &work.join(".git/extra.conf"),
            b"[wsx]\n\tsourceBranch = workspace/dev\n",
        ));
        append_config(&work, b"[include]\n\tpath = extra.conf\n");
        assert_eq!(
            git_config(
                &scratch,
                &work,
                &["--local", "--includes", "--get", "wsx.sourceBranch"]
            ),
            (Some(0), b"workspace/dev\n".to_vec())
        );
        match resolved(&work) {
            Ok(RepoModel::NotConfigured { .. }) => {}
            other => panic!("resolve: expected RepoModel::NotConfigured, got {other:?}"),
        }
    }

    /// `remote` as `state_child` prints it: the name, or `<none>`.
    fn remote_marker(remote: Option<&[u8]>) -> String {
        remote.map_or_else(
            || "<none>".to_owned(),
            |name| String::from_utf8_lossy(name).into_owned(),
        )
    }

    /// Adds `upstream`, `<root>/fork-source`, to `work` and fetches it, and
    /// returns `fork-source`'s path. `fork-source` is a clone of `source`
    /// whose `develop` gains one commit of its own, so
    /// `refs/remotes/upstream/develop` and `refs/remotes/origin/develop` hold
    /// different commits, and both differ from `origin/main` and
    /// `origin/next`: a test can tell which remote's and which rung's tip a
    /// resolved base took. Asserts those four tips distinct before it
    /// returns, the control that the fixture can tell them apart.
    fn with_fork(scratch: &Scratch, work: &Path) -> PathBuf {
        ok(git_ok(
            scratch,
            &scratch.root,
            ["clone", "-q", "source", "fork-source"],
        ));
        let fork = scratch.root.join("fork-source");
        ok(git_ok(
            scratch,
            &fork,
            ["switch", "-q", "-c", "develop", "origin/develop"],
        ));
        ok(write(&fork.join("u.txt"), b"u\n"));
        ok(commit_all(
            scratch,
            &fork,
            "upstream develop",
            T0 + 4 * TICK,
        ));
        ok(git_ok(scratch, &fork, ["switch", "-q", "main"]));
        ok(git_ok(
            scratch,
            work,
            [
                OsStr::new("remote"),
                OsStr::new("add"),
                OsStr::new("upstream"),
                fork.as_os_str(),
            ],
        ));
        ok(git_ok(scratch, work, ["fetch", "-q", "upstream"]));

        let refs = [
            "refs/remotes/upstream/develop",
            "refs/remotes/origin/develop",
            "refs/remotes/origin/next",
            "refs/remotes/origin/main",
        ];
        let tips = refs.map(|full| tip(scratch, work, full));
        let distinct: BTreeSet<&String> = tips.iter().collect();
        assert_eq!(
            distinct.len(),
            tips.len(),
            "the fork fixture's candidates share a commit: {refs:?} hold {tips:?}"
        );
        fork
    }

    /// Asserts `integration` is `develop` read from `full_ref`, its tip the
    /// id `git show-ref --verify <full_ref>` prints and not the tip of any
    /// ref in `rejected`, each of which must exist.
    fn assert_integration(
        scratch: &Scratch,
        work: &Path,
        integration: &Integration,
        full_ref: &str,
        rejected: &[&str],
    ) {
        assert_resolved(scratch, work, integration, "develop", full_ref, rejected);
    }

    /// Asserts `integration` is `branch` read from `full_ref`, its tip the
    /// id `git show-ref --verify <full_ref>` prints and not the tip of any
    /// ref in `rejected`, each of which must exist: the candidates the chain
    /// passed over or must ignore, so a right label on a wrong tip fails.
    fn assert_resolved(
        scratch: &Scratch,
        work: &Path,
        integration: &Integration,
        branch: &str,
        full_ref: &str,
        rejected: &[&str],
    ) {
        assert_eq!(
            integration.branch,
            branch.as_bytes().to_vec(),
            "integration branch"
        );
        assert_eq!(
            integration.full_ref,
            full_ref.as_bytes().to_vec(),
            "integration ref"
        );
        let got = integration.tip.to_string();
        assert_eq!(
            got,
            tip(scratch, work, full_ref),
            "integration tip against git show-ref --verify {full_ref}"
        );
        for other in rejected {
            assert_ne!(
                got,
                tip(scratch, work, other),
                "integration tip is {other}'s"
            );
        }
    }

    /// A fork layout: the base from `upstream` and the push remote `origin`,
    /// each from its own key (D-02, D-05), with the integration branch read
    /// from `upstream`'s tracking ref, not `origin`'s.
    #[test]
    fn fork_layout_resolves_each_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-fork-layout").expect("create scratch");
        let work = repo_with_remote(&scratch);
        with_fork(&scratch, &work);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        set_config(&scratch, &work, BASE_REMOTE, "upstream");
        set_config(&scratch, &work, PUSH_REMOTE, "origin");

        let model = configured(&work);
        assert_eq!(model.base_remote, Some(b"upstream".to_vec()), "base remote");
        assert_eq!(model.push_remote, Some(b"origin".to_vec()), "push remote");
        assert_integration(
            &scratch,
            &work,
            &model.integration,
            "refs/remotes/upstream/develop",
            &["refs/remotes/origin/develop", "refs/remotes/upstream/main"],
        );
    }

    /// `wsx.pushRemote` alone moves only the push remote: the base still
    /// defaults to `origin`.
    #[test]
    fn push_remote_can_differ_from_a_default_base() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-push-only").expect("create scratch");
        let work = repo_with_remote(&scratch);
        with_fork(&scratch, &work);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        set_config(&scratch, &work, PUSH_REMOTE, "upstream");

        let model = configured(&work);
        assert_eq!(model.base_remote, Some(b"origin".to_vec()), "base remote");
        assert_eq!(model.push_remote, Some(b"upstream".to_vec()), "push remote");
        assert_integration(
            &scratch,
            &work,
            &model.integration,
            "refs/remotes/origin/develop",
            &[
                "refs/remotes/upstream/develop",
                "refs/remotes/origin/next",
                "refs/remotes/origin/main",
            ],
        );
    }

    /// With neither key set, both remotes are `origin` (D-02).
    #[test]
    fn remotes_default_to_origin() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-remote-defaults").expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");

        let model = configured(&work);
        assert_eq!(model.base_remote, Some(b"origin".to_vec()), "base remote");
        assert_eq!(model.push_remote, Some(b"origin".to_vec()), "push remote");
        assert_integration(
            &scratch,
            &work,
            &model.integration,
            "refs/remotes/origin/develop",
            &["refs/remotes/origin/next", "refs/remotes/origin/main"],
        );
    }

    /// `remote.pushDefault` names where any `git push` goes, not where wsx
    /// pushes a PR branch: set to `upstream` in local config, it leaves the
    /// push remote `origin` (D-02).
    #[test]
    fn push_default_is_not_read() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-push-default").expect("create scratch");
        let work = repo_with_remote(&scratch);
        with_fork(&scratch, &work);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        set_config(&scratch, &work, "remote.pushDefault", "upstream");
        assert_eq!(
            git_config(&scratch, &work, &["--local", "--get", "remote.pushDefault"]),
            (Some(0), b"upstream\n".to_vec())
        );

        let model = configured(&work);
        assert_eq!(model.push_remote, Some(b"origin".to_vec()), "push remote");
        assert_eq!(model.base_remote, Some(b"origin".to_vec()), "base remote");
        assert_integration(
            &scratch,
            &work,
            &model.integration,
            "refs/remotes/origin/develop",
            &["refs/remotes/upstream/develop"],
        );
    }

    /// The `NoSuchRemote` key and remote `resolve` refuses the fixture with
    /// once `key` names the remote `nope`, which it does not define; fails
    /// the test on any other answer. The fixture defines `origin`, so a
    /// fallback to it would resolve.
    fn refused_remote(name: &str, key: &str) -> (&'static str, Vec<u8>) {
        let scratch = Scratch::new(name).expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        set_config(&scratch, &work, key, "nope");
        match resolved(&work) {
            Err(Error::NoSuchRemote { key, remote }) => (key, remote),
            other => panic!("resolve: expected Err(NoSuchRemote), got {other:?}"),
        }
    }

    /// `wsx.baseRemote` naming a remote this repository does not define is
    /// refused by key and name, with no fallback to `origin` (D-03).
    #[test]
    fn missing_base_remote_names_the_key_and_the_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        assert_eq!(
            refused_remote("model-missing-base", BASE_REMOTE),
            ("wsx.baseRemote", b"nope".to_vec())
        );
    }

    /// The same for `wsx.pushRemote`.
    #[test]
    fn missing_push_remote_names_the_key_and_the_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        assert_eq!(
            refused_remote("model-missing-push", PUSH_REMOTE),
            ("wsx.pushRemote", b"nope".to_vec())
        );
    }

    /// With no key and no `origin`, there is no base or push remote, though
    /// the repository has a remote under another name (D-04), and the
    /// integration branch comes from local refs (D-07).
    #[test]
    fn no_origin_means_no_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-no-origin").expect("create scratch");
        let work = repo_with_remote(&scratch);
        ok(git_ok(
            &scratch,
            &work,
            ["branch", "-q", "develop", "refs/remotes/origin/develop"],
        ));
        ok(git_ok(
            &scratch,
            &work,
            ["remote", "rename", "origin", "upstream"],
        ));
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        assert_eq!(
            ok(git_ok(&scratch, &work, ["remote"])),
            b"upstream\n".to_vec(),
            "git remote, the control that another remote exists"
        );

        let model = configured(&work);
        assert_eq!(model.base_remote, None, "base remote");
        assert_eq!(model.push_remote, None, "push remote");
        assert_integration(
            &scratch,
            &work,
            &model.integration,
            "refs/heads/develop",
            &["refs/heads/main"],
        );
    }

    /// With `origin` and `upstream` and no key, the base is `origin` (D-05).
    #[test]
    fn upstream_is_preferred_nowhere() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-upstream-nowhere").expect("create scratch");
        let work = repo_with_remote(&scratch);
        with_fork(&scratch, &work);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");

        let model = configured(&work);
        assert_eq!(model.base_remote, Some(b"origin".to_vec()), "base remote");
        assert_eq!(model.push_remote, Some(b"origin".to_vec()), "push remote");
        assert_integration(
            &scratch,
            &work,
            &model.integration,
            "refs/remotes/origin/develop",
            &[
                "refs/remotes/upstream/develop",
                "refs/remotes/origin/next",
                "refs/remotes/origin/main",
            ],
        );
    }

    /// When only `upstream` holds an integration branch and no key names it,
    /// the refusal names `origin` and every ref tried is `origin`'s: wsx
    /// never turns to another remote (D-05).
    #[test]
    fn upstream_is_not_auto_detected() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-upstream-not-detected").expect("create scratch");
        let work = repo_with_remote(&scratch);
        with_fork(&scratch, &work);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        for full in ["refs/remotes/origin/develop", "refs/remotes/origin/next"] {
            ok(git_ok(&scratch, &work, ["update-ref", "-d", full]));
        }
        ok(git_ok(
            &scratch,
            &work,
            ["update-ref", "--no-deref", "-d", "refs/remotes/origin/HEAD"],
        ));
        // Controls: upstream's develop is there to be found, and deleting
        // the symbolic HEAD left the branch it named.
        tip(&scratch, &work, "refs/remotes/upstream/develop");
        tip(&scratch, &work, "refs/remotes/origin/main");

        match resolved(&work) {
            Err(Error::NoIntegrationBranch { remote, tried }) => {
                assert_eq!(remote, Some(b"origin".to_vec()), "remote");
                assert!(!tried.is_empty(), "no ref tried");
                for full in &tried {
                    assert!(
                        full.starts_with(b"refs/remotes/origin/"),
                        "tried a ref that is not origin's: {}",
                        String::from_utf8_lossy(full)
                    );
                }
            }
            other => panic!("resolve: expected Err(NoIntegrationBranch), got {other:?}"),
        }
    }

    /// `wsx.baseRemote` set twice is refused with both values (D-18).
    #[test]
    fn repeated_base_remote_is_an_error() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-base-repeated").expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        for value in ["origin", "nope"] {
            ok(git_ok(
                &scratch,
                &work,
                ["config", "--local", "--add", BASE_REMOTE, value],
            ));
        }
        match resolved(&work) {
            Err(Error::KeyRepeated { key, values }) => {
                assert_eq!(key, "wsx.baseRemote");
                assert_eq!(values, [b"origin".to_vec(), b"nope".to_vec()]);
            }
            other => panic!("resolve: expected Err(KeyRepeated), got {other:?}"),
        }
    }

    /// `wsx.pushRemote=` is refused as empty (D-18), not read as unset.
    #[test]
    fn empty_push_remote_is_an_error() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-push-empty").expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        set_config(&scratch, &work, PUSH_REMOTE, "");
        match resolved(&work) {
            Err(Error::KeyEmpty { key }) => assert_eq!(key, "wsx.pushRemote"),
            other => panic!("resolve: expected Err(KeyEmpty), got {other:?}"),
        }
    }

    /// A remote named `a/b` resolves its base from `refs/remotes/a/b/`.
    #[test]
    fn remote_name_with_a_slash_resolves() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-slash-remote").expect("create scratch");
        let work = repo_with_remote(&scratch);
        let fork = with_fork(&scratch, &work);
        ok(git_ok(
            &scratch,
            &work,
            [
                OsStr::new("remote"),
                OsStr::new("add"),
                OsStr::new("a/b"),
                fork.as_os_str(),
            ],
        ));
        ok(git_ok(&scratch, &work, ["fetch", "-q", "a/b"]));
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        set_config(&scratch, &work, BASE_REMOTE, "a/b");

        let model = configured(&work);
        assert_eq!(model.base_remote, Some(b"a/b".to_vec()), "base remote");
        assert_integration(
            &scratch,
            &work,
            &model.integration,
            "refs/remotes/a/b/develop",
            &["refs/remotes/origin/develop"],
        );
    }

    /// A global `[remote "origin"]` section in a repository with no origin
    /// does not make one: the base comes from local `develop` (D-04, A2)
    /// rather than failing against a remote this repository lacks. Run in a
    /// child, since the section is global config; `git remote` listing
    /// `origin` is the control that the section is seen.
    #[test]
    fn global_origin_section_does_not_hide_local_branches() {
        let scratch = Scratch::new("model-global-origin").expect("create scratch");
        let work = repo_with_remote(&scratch);
        ok(git_ok(
            &scratch,
            &work,
            ["branch", "-q", "develop", "refs/remotes/origin/develop"],
        ));
        ok(git_ok(&scratch, &work, ["remote", "remove", "origin"]));
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        let global = scratch.home.join(".gitconfig");
        let mut config = std::fs::read(&global).expect("read the scratch global config");
        config.extend_from_slice(b"[remote \"origin\"]\n\tprune = true\n");
        ok(write(&global, &config));
        assert_eq!(
            ok(git_ok(&scratch, &work, ["remote"])),
            b"origin\n".to_vec(),
            "git remote does not list the global section, so this test shows nothing"
        );

        let child = state_child_output(&scratch, &work);
        let state = ok(testing::marker(&child, "state"));
        assert_eq!(
            state,
            "configured",
            "child error: {:?}",
            testing::marker(&child, "error")
        );
        assert_eq!(ok(testing::marker(&child, "base_remote")), "<none>");
        assert_eq!(ok(testing::marker(&child, "push_remote")), "<none>");
    }

    /// `repo_with_remote` with `wsx.sourceBranch=workspace/dev`, so the
    /// repository is configured and resolving it reaches the chain.
    fn chain_repo(scratch: &Scratch) -> PathBuf {
        let work = repo_with_remote(scratch);
        set_config(scratch, &work, "wsx.sourceBranch", "workspace/dev");
        work
    }

    /// `git <args>` in `work`; fails the test when git fails.
    fn run(scratch: &Scratch, work: &Path, args: &[&str]) -> Vec<u8> {
        ok(git_ok(scratch, work, args))
    }

    /// `git <args>` in `work`: its exit code, whatever it is.
    fn exit_code(scratch: &Scratch, work: &Path, args: &[&str]) -> Option<i32> {
        ok(testing::git(scratch, work, args)).status.code()
    }

    /// Deletes each direct ref in `fulls` from `work`.
    fn delete_refs(scratch: &Scratch, work: &Path, fulls: &[&str]) {
        for full in fulls {
            run(scratch, work, &["update-ref", "-d", full]);
        }
    }

    /// The integration branch `resolve` gives for `work`.
    fn integration_of(work: &Path) -> Integration {
        configured(work).integration
    }

    /// The error `resolve` refuses `work` with; fails the test when it
    /// resolves.
    fn refusal(work: &Path) -> Error {
        match resolved(work) {
            Err(err) => err,
            Ok(answer) => panic!("resolve: expected an error, got {answer:?}"),
        }
    }

    /// The `remote` and `tried` of `err`, which must be
    /// `Error::NoIntegrationBranch`.
    fn no_integration(err: &Error) -> (Option<&[u8]>, Vec<String>) {
        match err {
            Error::NoIntegrationBranch { remote, tried } => (
                remote.as_deref(),
                tried
                    .iter()
                    .map(|full| String::from_utf8_lossy(full).into_owned())
                    .collect(),
            ),
            other => panic!("resolve: expected Err(NoIntegrationBranch), got {other:?}"),
        }
    }

    /// Asserts `err` is `Error::IntegrationRefSymbolic` naming `full_ref`
    /// and its `target`.
    fn assert_symbolic(err: &Error, full_ref: &str, target: &str) {
        match err {
            Error::IntegrationRefSymbolic {
                full_ref: got_ref,
                target: got_target,
            } => {
                assert_eq!(
                    String::from_utf8_lossy(got_ref),
                    full_ref,
                    "the symbolic ref named"
                );
                assert_eq!(
                    String::from_utf8_lossy(got_target),
                    target,
                    "the target named"
                );
            }
            other => panic!("resolve: expected Err(IntegrationRefSymbolic), got {other:?}"),
        }
    }

    const ORIGIN_DEVELOP: &str = "refs/remotes/origin/develop";
    const ORIGIN_NEXT: &str = "refs/remotes/origin/next";
    const ORIGIN_MAIN: &str = "refs/remotes/origin/main";
    const ORIGIN_HEAD: &str = "refs/remotes/origin/HEAD";
    const ORIGIN_ALIAS: &str = "refs/remotes/origin/alias";
    const WORKSPACE: &str = "refs/heads/workspace/dev";

    /// D-06: `develop` on the base remote wins over the remote default,
    /// which `git symbolic-ref` shows names `origin/main`.
    #[test]
    fn develop_beats_the_remote_default() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-develop").expect("create scratch");
        let work = chain_repo(&scratch);
        assert_eq!(
            run(&scratch, &work, &["symbolic-ref", ORIGIN_HEAD]),
            b"refs/remotes/origin/main\n".to_vec(),
            "git symbolic-ref {ORIGIN_HEAD}"
        );
        assert_resolved(
            &scratch,
            &work,
            &integration_of(&work),
            "develop",
            ORIGIN_DEVELOP,
            &[ORIGIN_NEXT, ORIGIN_MAIN],
        );
    }

    /// D-06: with no `develop` tracking ref, `next` is the next rung.
    #[test]
    fn next_follows_develop() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-next").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP]);
        assert_resolved(
            &scratch,
            &work,
            &integration_of(&work),
            "next",
            ORIGIN_NEXT,
            &[ORIGIN_MAIN],
        );
    }

    /// D-06: with neither, the branch `<remote>/HEAD` names, read from its
    /// own tracking ref; the control that an ordinary remote default
    /// resolves.
    #[test]
    fn remote_default_follows_next() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-default").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        assert_resolved(
            &scratch,
            &work,
            &integration_of(&work),
            "main",
            ORIGIN_MAIN,
            &[WORKSPACE],
        );
    }

    /// D-06: a local `develop` does not stand in for a missing tracking ref
    /// while a base remote exists; neither its label nor its commit is used.
    #[test]
    fn local_develop_does_not_redirect_a_remote_base() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-local-develop").expect("create scratch");
        let work = chain_repo(&scratch);
        run(&scratch, &work, &["branch", "develop", ORIGIN_DEVELOP]);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP]);
        assert_resolved(
            &scratch,
            &work,
            &integration_of(&work),
            "next",
            ORIGIN_NEXT,
            &["refs/heads/develop", ORIGIN_MAIN],
        );
    }

    /// D-07, the control for the test above: with no remote, the same local
    /// `develop` is the integration branch.
    #[test]
    fn local_branches_count_only_without_a_base_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-local-only").expect("create scratch");
        let work = chain_repo(&scratch);
        run(&scratch, &work, &["branch", "develop", ORIGIN_DEVELOP]);
        run(&scratch, &work, &["remote", "remove", "origin"]);
        let model = configured(&work);
        assert_eq!(model.base_remote, None, "base remote");
        assert_resolved(
            &scratch,
            &work,
            &model.integration,
            "develop",
            "refs/heads/develop",
            &["refs/heads/main"],
        );
    }

    /// D-07: with no remote and no local `develop`, the local `next`.
    #[test]
    fn local_next_follows_local_develop() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-local-next").expect("create scratch");
        let work = chain_repo(&scratch);
        run(&scratch, &work, &["branch", "next", ORIGIN_NEXT]);
        run(&scratch, &work, &["remote", "remove", "origin"]);
        assert_resolved(
            &scratch,
            &work,
            &integration_of(&work),
            "next",
            "refs/heads/next",
            &["refs/heads/main"],
        );
    }

    /// D-09: with no rung present, the error lists the three refs tried, in
    /// order, and names both fixes.
    #[test]
    fn no_integration_branch_lists_the_refs_tried() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-none").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        run(
            &scratch,
            &work,
            &["update-ref", "--no-deref", "-d", ORIGIN_HEAD],
        );

        let err = refusal(&work);
        let (remote, tried) = no_integration(&err);
        assert_eq!(remote, Some(&b"origin"[..]), "remote");
        assert_eq!(tried, [ORIGIN_DEVELOP, ORIGIN_NEXT, ORIGIN_HEAD], "tried");
        let rendered = err.to_string();
        for fix in ["git remote set-head origin --auto", "wsx.baseRef"] {
            assert!(rendered.contains(fix), "{rendered:?} does not name {fix:?}");
        }
    }

    /// D-09: an existing `origin/main` is never guessed as the default.
    #[test]
    fn main_is_never_guessed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-no-main").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        run(
            &scratch,
            &work,
            &["update-ref", "--no-deref", "-d", ORIGIN_HEAD],
        );
        tip(&scratch, &work, ORIGIN_MAIN);
        tip(&scratch, &work, "refs/heads/main");

        let (_, tried) = no_integration(&refusal(&work));
        assert!(
            !tried.iter().any(|full| full.ends_with("/main")),
            "tried main: {tried:?}"
        );
    }

    /// D-09: with no remote, an existing local `main` is not guessed either.
    #[test]
    fn local_main_is_never_guessed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-no-local-main").expect("create scratch");
        let work = chain_repo(&scratch);
        run(&scratch, &work, &["remote", "remove", "origin"]);
        tip(&scratch, &work, "refs/heads/main");

        let err = refusal(&work);
        let (remote, tried) = no_integration(&err);
        assert_eq!(remote, None, "remote");
        assert_eq!(tried, ["refs/heads/develop", "refs/heads/next"], "tried");
    }

    /// D-09: a `<remote>/HEAD` naming a deleted branch, which git cannot
    /// resolve, is no default.
    #[test]
    fn dangling_remote_head_is_not_a_default() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-dangling-head").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        run(
            &scratch,
            &work,
            &["symbolic-ref", ORIGIN_HEAD, "refs/remotes/origin/gone"],
        );
        assert_ne!(
            exit_code(&scratch, &work, &["rev-parse", "--verify", ORIGIN_HEAD]),
            Some(0),
            "git rev-parse --verify {ORIGIN_HEAD} resolved the dangling HEAD"
        );

        let (_, tried) = no_integration(&refusal(&work));
        assert!(
            tried.iter().any(|full| full == ORIGIN_HEAD),
            "tried {tried:?}"
        );
    }

    /// D-09: a `<remote>/HEAD` holding an object id names no branch.
    #[test]
    fn object_remote_head_is_not_a_default() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-object-head").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        let main = tip(&scratch, &work, ORIGIN_MAIN);
        run(
            &scratch,
            &work,
            &["update-ref", "--no-deref", ORIGIN_HEAD, main.as_str()],
        );
        assert_ne!(
            exit_code(&scratch, &work, &["symbolic-ref", "-q", ORIGIN_HEAD]),
            Some(0),
            "git symbolic-ref {ORIGIN_HEAD}: HEAD is still symbolic"
        );
        assert_eq!(tip(&scratch, &work, ORIGIN_HEAD), main, "HEAD's id");

        let (_, tried) = no_integration(&refusal(&work));
        assert!(
            tried.iter().any(|full| full == ORIGIN_HEAD),
            "tried {tried:?}"
        );
    }

    /// D-09: a `<remote>/HEAD` naming a local branch is no default, and the
    /// error says none of the refs names an integration branch, never that
    /// this existing `HEAD` does not exist.
    #[test]
    fn remote_head_outside_the_remote_is_not_a_default() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-outside-head").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        run(
            &scratch,
            &work,
            &["symbolic-ref", ORIGIN_HEAD, "refs/heads/main"],
        );
        assert_eq!(
            run(&scratch, &work, &["symbolic-ref", ORIGIN_HEAD]),
            b"refs/heads/main\n".to_vec(),
            "git symbolic-ref {ORIGIN_HEAD}, the control that HEAD exists"
        );

        let err = refusal(&work);
        let (_, tried) = no_integration(&err);
        assert!(
            tried.iter().any(|full| full == ORIGIN_HEAD),
            "tried {tried:?}"
        );
        let rendered = err.to_string();
        assert!(rendered.contains(ORIGIN_HEAD), "{rendered:?}");
        assert!(
            rendered.contains("names an integration branch"),
            "{rendered:?}"
        );
        assert!(
            !rendered.contains("exist"),
            "{rendered:?} says a ref that exists does not"
        );
    }

    /// D-10: a stale `<remote>/HEAD` is used as is. The source renamed its
    /// default to `trunk` and nothing was fetched, so `HEAD` still names
    /// `origin/main`, whose tracking ref exists.
    #[test]
    fn stale_remote_head_is_accepted() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-stale-head").expect("create scratch");
        let work = chain_repo(&scratch);
        let source = scratch.root.join("source");
        run(&scratch, &source, &["branch", "-m", "main", "trunk"]);
        assert_eq!(
            run(&scratch, &source, &["symbolic-ref", "HEAD"]),
            b"refs/heads/trunk\n".to_vec(),
            "the source's default"
        );
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        assert_eq!(
            run(&scratch, &work, &["symbolic-ref", ORIGIN_HEAD]),
            b"refs/remotes/origin/main\n".to_vec(),
            "git symbolic-ref {ORIGIN_HEAD} after the rename"
        );

        assert_resolved(
            &scratch,
            &work,
            &integration_of(&work),
            "main",
            ORIGIN_MAIN,
            &[WORKSPACE],
        );
    }

    /// D-09: resolution reads only local refs: with the remote's directory
    /// deleted, the remote default resolves to the same answer, tip
    /// included.
    #[test]
    fn resolution_needs_no_remote_to_be_reachable() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-unreachable").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        let before = integration_of(&work);
        let source = scratch.root.join("source");
        std::fs::remove_dir_all(&source)
            .unwrap_or_else(|err| panic!("remove {}: {err}", source.display()));
        assert!(!source.exists(), "the remote's directory is still there");

        let after = integration_of(&work);
        assert_eq!(after, before, "the answer before and after the deletion");
        assert_resolved(&scratch, &work, &after, "main", ORIGIN_MAIN, &[WORKSPACE]);
    }

    /// A rung holding a blob, which git cannot peel to a commit, fails
    /// closed: no fall-through to `next`.
    #[test]
    fn integration_ref_holding_a_blob_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-blob").expect("create scratch");
        let work = chain_repo(&scratch);
        let blob = ok(testing::object_id(&scratch, &work, "refs/heads/main:a.txt"));
        run(
            &scratch,
            &work,
            &["update-ref", ORIGIN_DEVELOP, blob.to_string().as_str()],
        );
        let peel = format!("{ORIGIN_DEVELOP}^{{commit}}");
        assert_ne!(
            exit_code(&scratch, &work, &["rev-parse", "--verify", peel.as_str()]),
            Some(0),
            "git peeled {ORIGIN_DEVELOP}, holding a blob, to a commit"
        );
        tip(&scratch, &work, ORIGIN_NEXT);

        match refusal(&work) {
            Error::Git(_) => {}
            other => panic!("resolve: expected Err(Git), got {other:?}"),
        }
    }

    /// D-06: `HEAD -> alias -> refs/heads/workspace/dev` is refused at
    /// `alias`. `git rev-parse` following the chain to the workspace tip is
    /// the control that it does reach a local commit.
    #[test]
    fn remote_head_chain_into_a_local_branch_is_refused() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-head-alias-local").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        run(&scratch, &work, &["symbolic-ref", ORIGIN_ALIAS, WORKSPACE]);
        run(
            &scratch,
            &work,
            &["symbolic-ref", ORIGIN_HEAD, ORIGIN_ALIAS],
        );
        let workspace = tip(&scratch, &work, WORKSPACE);
        assert_eq!(
            ok(testing::object_id(&scratch, &work, ORIGIN_ALIAS)).to_string(),
            workspace,
            "git rev-parse --verify {ORIGIN_ALIAS}"
        );

        let err = refusal(&work);
        assert_symbolic(&err, ORIGIN_ALIAS, WORKSPACE);
        let rendered = err.to_string();
        for fix in [
            "git update-ref --no-deref -d refs/remotes/origin/alias",
            "wsx.baseRef",
        ] {
            assert!(rendered.contains(fix), "{rendered:?} does not name {fix:?}");
        }
    }

    /// D-06: a chain that never leaves the remote is refused too.
    #[test]
    fn remote_head_chain_inside_the_remote_is_refused() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-head-alias-remote").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        run(
            &scratch,
            &work,
            &["symbolic-ref", ORIGIN_ALIAS, ORIGIN_MAIN],
        );
        run(
            &scratch,
            &work,
            &["symbolic-ref", ORIGIN_HEAD, ORIGIN_ALIAS],
        );

        assert_symbolic(&refusal(&work), ORIGIN_ALIAS, ORIGIN_MAIN);
    }

    /// D-06: a cycle, which git cannot resolve, is refused at `alias`.
    #[test]
    fn remote_head_cycle_is_refused() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-head-cycle").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        run(
            &scratch,
            &work,
            &["symbolic-ref", ORIGIN_ALIAS, ORIGIN_HEAD],
        );
        run(
            &scratch,
            &work,
            &["symbolic-ref", ORIGIN_HEAD, ORIGIN_ALIAS],
        );
        assert_ne!(
            exit_code(&scratch, &work, &["rev-parse", "--verify", ORIGIN_HEAD]),
            Some(0),
            "git rev-parse --verify {ORIGIN_HEAD} resolved the cycle"
        );

        assert_symbolic(&refusal(&work), ORIGIN_ALIAS, ORIGIN_HEAD);
    }

    /// D-06: a symbolic `develop` is refused, with no fall-through to the
    /// `next` that is present.
    #[test]
    fn symbolic_develop_is_refused() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-symbolic-develop").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP]);
        run(
            &scratch,
            &work,
            &["symbolic-ref", ORIGIN_DEVELOP, WORKSPACE],
        );
        tip(&scratch, &work, ORIGIN_NEXT);

        assert_symbolic(&refusal(&work), ORIGIN_DEVELOP, WORKSPACE);
    }

    /// D-06: a symbolic `next` is refused, with no fall-through to the
    /// remote default, which is still usable.
    #[test]
    fn symbolic_next_is_refused() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-symbolic-next").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        run(&scratch, &work, &["symbolic-ref", ORIGIN_NEXT, WORKSPACE]);
        assert_eq!(
            run(&scratch, &work, &["symbolic-ref", ORIGIN_HEAD]),
            b"refs/remotes/origin/main\n".to_vec(),
            "git symbolic-ref {ORIGIN_HEAD}"
        );
        tip(&scratch, &work, ORIGIN_MAIN);

        assert_symbolic(&refusal(&work), ORIGIN_NEXT, WORKSPACE);
    }

    /// D-07: the rule holds on local rungs: a local `develop` aliased to
    /// `main` is refused.
    #[test]
    fn symbolic_local_develop_is_refused() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-symbolic-local").expect("create scratch");
        let work = chain_repo(&scratch);
        run(&scratch, &work, &["remote", "remove", "origin"]);
        run(
            &scratch,
            &work,
            &["symbolic-ref", "refs/heads/develop", "refs/heads/main"],
        );

        assert_symbolic(&refusal(&work), "refs/heads/develop", "refs/heads/main");
    }

    /// The loose file `.git/heads/refs/remotes/origin/develop`, a path gix
    /// tries for that full name and git does not, cannot stand in for the
    /// deleted tracking ref.
    #[test]
    fn ref_under_another_path_does_not_satisfy_a_rung() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("chain-other-path").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP]);
        let workspace = tip(&scratch, &work, WORKSPACE);
        ok(write(
            &work.join(".git/heads").join(ORIGIN_DEVELOP),
            format!("{workspace}\n").as_bytes(),
        ));
        assert_ne!(
            exit_code(&scratch, &work, &["show-ref", "--verify", ORIGIN_DEVELOP]),
            Some(0),
            "git show-ref --verify {ORIGIN_DEVELOP} found the file under .git/heads/"
        );

        assert_resolved(
            &scratch,
            &work,
            &integration_of(&work),
            "next",
            ORIGIN_NEXT,
            &[WORKSPACE, ORIGIN_MAIN],
        );
    }

    #[test]
    #[ignore = "child half of the model config-source tests; runs only when re-executed"]
    fn state_child() {
        let start = std::env::var_os(START_VAR).expect("the parent set the start directory");
        let answer = Repo::discover(Path::new(&start))
            .map_err(Error::Git)
            .and_then(|repo| resolve(&repo, &BTreeSet::new()));
        match answer {
            Ok(RepoModel::Configured(model)) => {
                testing::child_print("state", "configured");
                testing::child_print("base_remote", remote_marker(model.base_remote.as_deref()));
                testing::child_print("push_remote", remote_marker(model.push_remote.as_deref()));
            }
            Ok(RepoModel::NotConfigured { .. }) => testing::child_print("state", "not-configured"),
            Err(err) => {
                testing::child_print("state", "failed");
                testing::child_print("error", err.to_string().replace('\n', "\\n"));
            }
        }
    }
}
