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

/// The local config key naming the integration branch outright, a bare
/// branch name on the base remote, with no fallback to the chain (D-08).
pub const BASE_REF: &str = "wsx.baseRef";

/// The remote a base or push remote defaults to when its key is unset (D-02).
pub const DEFAULT_REMOTE: &[u8] = b"origin";

/// The local config key naming the directory feature worktrees live in,
/// relative to the worktree root (D-20).
pub const WORKTREE_DIR: &str = "wsx.worktreeDir";

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

impl RepoModel {
    /// The model for a command that needs a workspace branch: the
    /// configured one, or the inferred one in a repository that is not
    /// configured (D-15, D-16).
    ///
    /// A manifest that is missing, not a blob, unreadable or invalid on the
    /// inferred branch is `Error::NotConfigured` with that failure as its
    /// reason, which renders the failure and says run `wsx init` (D-16). So
    /// is a refusal of a replacement git and gix follow differently
    /// (`git::Error::is_replacement_refusal`), which stops a configured
    /// repository (D-22) and leaves one with no key not configured.
    /// Every other error passes through unchanged, so zero or several
    /// candidates stay the list `Error::WorkspaceCandidates` renders
    /// (D-21). Whether a repository is configured is read from the variant,
    /// not from this: a caller that must tell the states apart, such as the
    /// push guard, matches on `RepoModel`.
    pub fn require(self) -> Result<Box<Model>, Error> {
        match self {
            RepoModel::Configured(model) => Ok(model),
            RepoModel::NotConfigured { inferred } => inferred.map_err(|err| match err {
                err @ (Error::ManifestMissing { .. }
                | Error::ManifestNotABlob { .. }
                | Error::ManifestUnreadable { .. }
                | Error::ManifestInvalid { .. }) => Error::NotConfigured {
                    reason: Box::new(err),
                },
                Error::Git(inner) if inner.is_replacement_refusal() => Error::NotConfigured {
                    reason: Box::new(Error::Git(inner)),
                },
                other => other,
            }),
        }
    }
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
/// recorded, as short names, which never count as workspace-branch
/// candidates (D-17); production passes an empty set until phase 8.
///
/// `Err` means exactly that the repository is configured and the model
/// fails closed; a repository that is not configured is a value. With
/// `wsx.sourceBranch` unset, the only candidate is inferred, and every
/// failure on the way, zero or several candidates included, stays in
/// `inferred` (D-15, D-16, D-21).
///
/// What a resolve reads, and when. It writes nothing to the repository.
/// Every tip comes from the same read of its ref that any check on that ref
/// used, never from a second lookup of the name: an integration rung's tip
/// is peeled from the id returned by the read that found the rung not
/// symbolic (04-07), and `ref_commit` reads each ref it follows once
/// (04-01). The workspace branch is read once to see that it exists and
/// once by `ref_commit` for its tip, and the first read checks nothing the
/// second could undo. `workspace_tip` and the manifest always come from one
/// commit, because `root_entry` takes that commit's id, not the branch name
/// (`manifest_read_stays_on_the_commit_it_was_given` moves the branch
/// between the two reads). Each tip is a value its ref held at some moment
/// during the call, and the two tips need not be from one instant: git has
/// no snapshot across refs. Every object read applies the replacement map
/// gix loaded when the `Repo` was opened, so a replace ref added or removed
/// after that, during one run, is not seen by these reads, and a later check
/// that lists replace refs can disagree with that map in that window
/// (accepted by the owner on 2026-11-03). Later reads in a command use the
/// ids recorded here. Only the one-commit half has a test; nothing pins the
/// rest.
pub fn resolve(repo: &Repo, recorded: &BTreeSet<Vec<u8>>) -> Result<RepoModel, Error> {
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
        None => {
            let inferred = candidates(repo, recorded).and_then(|mut names| {
                if names.len() != 1 {
                    return Err(Error::WorkspaceCandidates { candidates: names });
                }
                build(repo, names.remove(0))
            });
            Ok(RepoModel::NotConfigured {
                inferred: inferred.map(Box::new),
            })
        }
    }
}

/// The prefixes a workspace-branch candidate is listed under, each with its
/// trailing slash, since a prefix without one also matches `workspace-x`.
const CANDIDATE_PREFIXES: [&[u8]; 2] = [b"refs/heads/workspace/", b"refs/heads/personal/"];

/// The local branches directly under `workspace/` or `personal/`, as short
/// names in byte order across both prefixes, less those in `recorded`
/// (D-17, D-21). Remote-tracking branches and nested names are not listed,
/// and the checked-out branch gets no preference. A ref that fails to read
/// fails the whole listing rather than shortening it.
fn candidates(repo: &Repo, recorded: &BTreeSet<Vec<u8>>) -> Result<Vec<Vec<u8>>, Error> {
    let mut names = Vec::new();
    for prefix in CANDIDATE_PREFIXES {
        for full in repo.refs_directly_under(prefix)? {
            if let Some(short) = full.strip_prefix(BRANCH_PREFIX)
                && !recorded.contains(short)
            {
                names.push(short.to_vec());
            }
        }
    }
    names.sort();
    Ok(names)
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

/// The prefix of a local branch's full ref.
const BRANCH_PREFIX: &[u8] = b"refs/heads/";

/// `refs/heads/<branch>`.
fn branch_ref(branch: &[u8]) -> Vec<u8> {
    [BRANCH_PREFIX, branch].concat()
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
///
/// A failure reading the tip's commit or its root tree is
/// `Error::ManifestUnreadable`, since both are reads of the manifest; a
/// failure of the `find_ref` that checks the branch exists is the ref store
/// and stays `Error::Git`. So does a refusal of a replacement git and gix
/// follow differently, from `root_entry` before it reads anything
/// (`git::Error::is_replacement_refusal`): it does not know whether the ref
/// touches the manifest, so its message names the ref and does not say
/// `.wsx.conf` cannot be read (D-22).
fn build(repo: &Repo, branch: Vec<u8>) -> Result<Model, Error> {
    let full = branch_ref(&branch);
    if repo.find_ref(&full)?.is_none() {
        return Err(Error::WorkspaceBranchMissing { branch });
    }
    let base_remote = remote(repo, BASE_REMOTE)?;
    let push_remote = remote(repo, PUSH_REMOTE)?;
    let integration = integration(repo, base_remote.as_deref())?;
    let worktree_dir = worktree_dir(repo)?;
    let unreadable = |source: git::Error| Error::ManifestUnreadable {
        branch: branch.clone(),
        source,
    };
    let workspace_tip = repo.ref_commit(&full).map_err(unreadable)?;
    let manifest = match repo
        .root_entry(&workspace_tip, MANIFEST_NAME)
        .map_err(|source| {
            if source.is_replacement_refusal() {
                Error::Git(source)
            } else {
                unreadable(source)
            }
        })? {
        RootEntry::Absent => return Err(Error::ManifestMissing { branch }),
        RootEntry::NotABlob { kind } => return Err(Error::ManifestNotABlob { branch, kind }),
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

/// The worktree directory: `wsx.worktreeDir`, read once (D-18) and put
/// through the path rules of a manifest line (`manifest::normalize_path`,
/// D-14), so one trailing `/` is stripped and an absolute path, an empty
/// component or a `.` or `..` component is `Error::WorktreeDirInvalid`
/// (D-20); `DEFAULT_WORKTREE_DIR` when the key is unset.
fn worktree_dir(repo: &Repo) -> Result<Vec<u8>, Error> {
    match single(repo, WORKTREE_DIR)? {
        Some(value) => manifest::normalize_path(&value)
            .map_err(|problem| Error::WorktreeDirInvalid { value, problem }),
        None => Ok(DEFAULT_WORKTREE_DIR.to_vec()),
    }
}

/// The integration branches the chain tries, in order, before the base
/// remote's default (D-06, D-07).
const CHAIN: [&[u8]; 2] = [b"develop", b"next"];

/// `value` of `key` when it is a valid branch name, else
/// `Error::NotABranchName`. After this check the value is read only as one
/// exact full ref, never through git's DWIM ladder (03.1 D-04).
fn branch_name(key: &'static str, value: Vec<u8>) -> Result<Vec<u8>, Error> {
    if git::is_branch_name(&value) {
        Ok(value)
    } else {
        Err(Error::NotABranchName { key, value })
    }
}

/// The integration branch. `wsx.baseRef`, when set, is a bare branch name
/// read on the base remote (locally with no base remote), and a missing ref
/// is `Error::BaseRefMissing` with no fallback to the chain (D-08).
/// Otherwise `develop`, then `next`, then the branch the base remote's
/// `HEAD` names, each read from the base remote's tracking refs; with no
/// base remote, the local `develop`, then the local `next` (D-06, D-07).
/// Nothing else is tried: no `main` or `master`, no network call (D-09), and
/// a stale `<remote>/HEAD` is used as it is (D-10).
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
    if let Some(value) = single(repo, BASE_REF)? {
        let value = branch_name(BASE_REF, value)?;
        let tried = full(&value);
        return match found(&value)? {
            Some(integration) => Ok(integration),
            None => Err(Error::BaseRefMissing { value, tried }),
        };
    }
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

    use super::manifest::{EntryKind, ParseError, Problem};
    use super::{
        BASE_REF, BASE_REMOTE, Error, Integration, Model, PUSH_REMOTE, RepoModel, WORKTREE_DIR,
        resolve,
    };

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
        state_child_output_with(scratch, work, &[])
    }

    /// Runs `state_child` on `work` with the scratch environment plus each
    /// `(name, value)` in `extra`.
    fn state_child_output_with(scratch: &Scratch, work: &Path, extra: &[(&str, &str)]) -> Output {
        let mut env = scratch.env.clone();
        env.extend(
            extra
                .iter()
                .map(|(name, value)| ((*name).into(), (*value).into())),
        );
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
        child_state_with(scratch, work, &[])
    }

    /// `child_state` with each `(name, value)` in `extra` added to the
    /// child's environment.
    fn child_state_with(scratch: &Scratch, work: &Path, extra: &[(&str, &str)]) -> String {
        let child = state_child_output_with(scratch, work, extra);
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

    /// D-08: `wsx.baseRef` selects `main` though the chain would take
    /// `develop`, from the base remote's tracking ref.
    #[test]
    fn base_ref_overrides_the_chain() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("base-ref-overrides").expect("create scratch");
        let work = chain_repo(&scratch);
        set_config(&scratch, &work, BASE_REF, "main");
        assert_resolved(
            &scratch,
            &work,
            &integration_of(&work),
            "main",
            ORIGIN_MAIN,
            &[ORIGIN_DEVELOP, ORIGIN_NEXT],
        );
    }

    /// D-08: a `wsx.baseRef` with no ref is refused naming the ref tried,
    /// with no fallback to the `develop` that exists.
    #[test]
    fn missing_base_ref_names_the_ref() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("base-ref-missing").expect("create scratch");
        let work = chain_repo(&scratch);
        set_config(&scratch, &work, BASE_REF, "nope");
        tip(&scratch, &work, ORIGIN_DEVELOP);

        match refusal(&work) {
            Error::BaseRefMissing { value, tried } => {
                assert_eq!(String::from_utf8_lossy(&value), "nope", "value");
                assert_eq!(
                    String::from_utf8_lossy(&tried),
                    "refs/remotes/origin/nope",
                    "tried"
                );
            }
            other => panic!("resolve: expected Err(BaseRefMissing), got {other:?}"),
        }
    }

    /// D-08: `upstream/next` is a branch literally named that on the base
    /// remote `upstream`, not `upstream`'s `next`, which exists.
    #[test]
    fn base_ref_with_a_slash_is_a_literal_branch_name() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("base-ref-slash").expect("create scratch");
        let work = chain_repo(&scratch);
        let fork = with_fork(&scratch, &work);
        run(&scratch, &fork, &["branch", "next", ORIGIN_NEXT]);
        run(&scratch, &work, &["fetch", "-q", "upstream"]);
        set_config(&scratch, &work, BASE_REMOTE, "upstream");
        set_config(&scratch, &work, BASE_REF, "upstream/next");
        // The control: `upstream`'s `next` exists, so reading the value as
        // a remote and a branch would have resolved.
        tip(&scratch, &work, "refs/remotes/upstream/next");

        let err = refusal(&work);
        match &err {
            Error::BaseRefMissing { value, tried } => {
                assert_eq!(String::from_utf8_lossy(value), "upstream/next", "value");
                assert_eq!(
                    String::from_utf8_lossy(tried),
                    "refs/remotes/upstream/upstream/next",
                    "tried"
                );
            }
            other => panic!("resolve: expected Err(BaseRefMissing), got {other:?}"),
        }
        let rendered = err.to_string();
        assert!(
            rendered.contains("bare branch name"),
            "{rendered:?} does not say wsx.baseRef is a bare branch name"
        );
    }

    /// D-08: with no remote, `wsx.baseRef` reads `refs/heads/<value>`, over
    /// the local `develop` the chain would take.
    #[test]
    fn base_ref_resolves_locally_without_a_base_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("base-ref-local").expect("create scratch");
        let work = chain_repo(&scratch);
        run(&scratch, &work, &["branch", "trunk", ORIGIN_NEXT]);
        run(&scratch, &work, &["branch", "develop", ORIGIN_DEVELOP]);
        run(&scratch, &work, &["remote", "remove", "origin"]);
        set_config(&scratch, &work, BASE_REF, "trunk");
        assert_resolved(
            &scratch,
            &work,
            &integration_of(&work),
            "trunk",
            "refs/heads/trunk",
            &["refs/heads/develop", "refs/heads/main"],
        );
    }

    /// A5: a value `git check-ref-format --branch` refuses is not a branch
    /// name.
    #[test]
    fn invalid_base_ref_is_an_error() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("base-ref-invalid").expect("create scratch");
        let work = chain_repo(&scratch);
        set_config(&scratch, &work, BASE_REF, "-x");

        match refusal(&work) {
            Error::NotABranchName { key, value } => {
                assert_eq!(key, "wsx.baseRef", "key");
                assert_eq!(value, b"-x".to_vec(), "value");
            }
            other => panic!("resolve: expected Err(NotABranchName), got {other:?}"),
        }
    }

    /// D-18: `wsx.baseRef` set twice is refused with both values.
    #[test]
    fn repeated_base_ref_is_an_error() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("base-ref-repeated").expect("create scratch");
        let work = chain_repo(&scratch);
        for value in ["main", "next"] {
            run(
                &scratch,
                &work,
                &["config", "--local", "--add", BASE_REF, value],
            );
        }

        match refusal(&work) {
            Error::KeyRepeated { key, values } => {
                assert_eq!(key, "wsx.baseRef", "key");
                assert_eq!(values, [b"main".to_vec(), b"next".to_vec()], "values");
            }
            other => panic!("resolve: expected Err(KeyRepeated), got {other:?}"),
        }
    }

    /// D-18: `wsx.baseRef=` is refused as empty, not read as unset.
    #[test]
    fn empty_base_ref_is_an_error() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("base-ref-empty").expect("create scratch");
        let work = chain_repo(&scratch);
        set_config(&scratch, &work, BASE_REF, "");

        match refusal(&work) {
            Error::KeyEmpty { key } => assert_eq!(key, "wsx.baseRef", "key"),
            other => panic!("resolve: expected Err(KeyEmpty), got {other:?}"),
        }
    }

    /// D-06, D-08: a `wsx.baseRef` naming a symbolic ref is refused like
    /// every other rung, neither reported missing nor replaced by the
    /// `develop` that exists.
    #[test]
    fn symbolic_base_ref_is_refused() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("base-ref-symbolic").expect("create scratch");
        let work = chain_repo(&scratch);
        run(&scratch, &work, &["symbolic-ref", ORIGIN_ALIAS, WORKSPACE]);
        set_config(&scratch, &work, BASE_REF, "alias");
        tip(&scratch, &work, ORIGIN_DEVELOP);

        assert_symbolic(&refusal(&work), ORIGIN_ALIAS, WORKSPACE);
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

    /// `inferred` from `resolve(repo of work, recorded)`; fails the test
    /// unless the repository resolves as not configured.
    fn inferred_with(work: &Path, recorded: &BTreeSet<Vec<u8>>) -> Result<Box<Model>, Error> {
        let repo = Repo::discover(work).expect("discover the fixture");
        not_configured_in(&repo, recorded)
    }

    /// `inferred` from `resolve(repo, recorded)`; fails the test unless the
    /// repository resolves as not configured.
    fn not_configured_in(repo: &Repo, recorded: &BTreeSet<Vec<u8>>) -> Result<Box<Model>, Error> {
        match resolve(repo, recorded) {
            Ok(RepoModel::NotConfigured { inferred }) => inferred,
            other => panic!("resolve: expected RepoModel::NotConfigured, got {other:?}"),
        }
    }

    /// `inferred` for `work` with nothing recorded.
    fn inferred(work: &Path) -> Result<Box<Model>, Error> {
        inferred_with(work, &BTreeSet::new())
    }

    /// The model inferred for `work`; fails the test on an error.
    fn inferred_model(work: &Path) -> Model {
        match inferred(work) {
            Ok(model) => *model,
            Err(err) => panic!("inferred: expected a model, got {err:?}: {err}"),
        }
    }

    /// `names` as text, for comparing lists of branch names.
    fn texts(names: &[Vec<u8>]) -> Vec<String> {
        names
            .iter()
            .map(|name| String::from_utf8_lossy(name).into_owned())
            .collect()
    }

    /// The candidates `err` lists; fails the test unless it is
    /// `Error::WorkspaceCandidates`.
    fn listed_candidates(err: &Error) -> Vec<String> {
        match err {
            Error::WorkspaceCandidates { candidates } => texts(candidates),
            other => panic!("expected Err(WorkspaceCandidates), got {other:?}"),
        }
    }

    /// The candidates `inferred` lists for `work`.
    fn candidates_of(work: &Path) -> Vec<String> {
        match inferred(work) {
            Err(err) => listed_candidates(&err),
            Ok(model) => panic!("inferred: expected Err(WorkspaceCandidates), got {model:?}"),
        }
    }

    /// `require` on what `resolve` gives for `work`; fails the test when
    /// `resolve` itself errs.
    fn required(work: &Path) -> Result<Box<Model>, Error> {
        match resolved(work) {
            Ok(answer) => answer.require(),
            Err(err) => panic!("resolve: expected a RepoModel, got {err:?}"),
        }
    }

    /// The reason `require` wraps in `Error::NotConfigured` for `work`, and
    /// the wrapped error's rendering; fails the test on any other answer.
    fn wrapped_reason(work: &Path) -> (Error, String) {
        match required(work) {
            Err(err) => {
                let rendered = err.to_string();
                match err {
                    Error::NotConfigured { reason } => (*reason, rendered),
                    other => panic!("require: expected Err(NotConfigured), got {other:?}"),
                }
            }
            Ok(model) => panic!("require: expected Err(NotConfigured), got {model:?}"),
        }
    }

    /// `workspace/dev`'s tip in `work` commits `.wsx.conf` holding
    /// `content`, or no `.wsx.conf` for `None`; `main` stays checked out.
    fn commit_manifest(scratch: &Scratch, work: &Path, content: Option<&[u8]>) {
        run(scratch, work, &["switch", "-q", "workspace/dev"]);
        let path = work.join(".wsx.conf");
        match content {
            Some(bytes) => ok(write(&path, bytes)),
            None => std::fs::remove_file(&path)
                .unwrap_or_else(|err| panic!("remove {}: {err}", path.display())),
        }
        ok(commit_all(scratch, work, "manifest edit", T0 + 4 * TICK));
        run(scratch, work, &["switch", "-q", "main"]);
    }

    /// Deletes the loose object `.wsx.conf` at `workspace/dev`'s tip names
    /// in `work` and returns its id, which `git cat-file -e` then refuses.
    /// The tree entry stays, so only the object read fails.
    fn delete_manifest_object(scratch: &Scratch, work: &Path) -> String {
        let id = run(
            scratch,
            work,
            &["rev-parse", "refs/heads/workspace/dev:.wsx.conf"],
        );
        let id = String::from_utf8_lossy(&id).trim_end().to_owned();
        let (dir, file) = id.split_at(2);
        let path = work.join(".git/objects").join(dir).join(file);
        assert!(
            path.is_file(),
            "the manifest blob {id} is not the loose object {}",
            path.display()
        );
        std::fs::remove_file(&path)
            .unwrap_or_else(|err| panic!("remove {}: {err}", path.display()));
        assert_ne!(
            exit_code(scratch, work, &["cat-file", "-e", &id]),
            Some(0),
            "git cat-file -e {id} after its object was deleted"
        );
        id
    }

    /// `workspace/dev` renamed to `personal/workspace`, and
    /// `personal/workspace-backup` made from it: two candidates, neither
    /// under `workspace/`.
    fn two_personal_candidates(scratch: &Scratch, work: &Path) {
        run(
            scratch,
            work,
            &["branch", "-m", "workspace/dev", "personal/workspace"],
        );
        run(
            scratch,
            work,
            &["branch", "personal/workspace-backup", "personal/workspace"],
        );
    }

    /// The `git for-each-ref` names in `work` matching `glob`, one per line.
    fn for_each_ref(scratch: &Scratch, work: &Path, glob: &str) -> Vec<Vec<u8>> {
        testing::records(
            &run(
                scratch,
                work,
                &["for-each-ref", "--format=%(refname)", glob],
            ),
            b'\n',
        )
    }

    /// Success Criterion 3: `wsx.sourceBranch` names the workspace branch
    /// with another candidate present. Without the key the same repository
    /// has two candidates, so the key, not inference, picked it.
    #[test]
    fn source_branch_names_the_workspace() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-source-names").expect("create scratch");
        let work = repo_with_remote(&scratch);
        run(&scratch, &work, &["branch", "workspace/other"]);
        assert_eq!(
            candidates_of(&work),
            ["workspace/dev", "workspace/other"],
            "candidates without wsx.sourceBranch"
        );

        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        let model = configured(&work);
        assert_eq!(
            String::from_utf8_lossy(&model.workspace_branch),
            "workspace/dev",
            "workspace branch"
        );
        assert_eq!(
            model.workspace_tip.to_string(),
            tip(&scratch, &work, WORKSPACE),
            "workspace tip against git show-ref"
        );
    }

    /// D-21: the key is a short branch name, and need not be under a
    /// candidate prefix: `main`, carrying its own `.wsx.conf`, is used.
    #[test]
    fn source_branch_may_be_outside_the_candidate_prefixes() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-source-main").expect("create scratch");
        let work = repo_with_remote(&scratch);
        ok(write(&work.join(".wsx.conf"), b"only notes\n"));
        ok(commit_all(&scratch, &work, "main manifest", T0 + 4 * TICK));
        set_config(&scratch, &work, "wsx.sourceBranch", "main");

        let model = configured(&work);
        assert_eq!(
            String::from_utf8_lossy(&model.workspace_branch),
            "main",
            "workspace branch"
        );
        assert_eq!(
            model.workspace_tip.to_string(),
            tip(&scratch, &work, "refs/heads/main"),
            "workspace tip against git show-ref"
        );
        assert!(model.is_personal(b"notes/x"), "is_personal(notes/x)");
        assert!(
            !model.is_personal(b".planning/x"),
            "is_personal(.planning/x), which only workspace/dev's manifest lists"
        );
    }

    /// Success Criterion 3: with no key, the only local `workspace/*`
    /// branch is inferred, read from this repository's own tip.
    #[test]
    fn only_candidate_is_inferred() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-only-candidate").expect("create scratch");
        let work = repo_with_remote(&scratch);

        let model = inferred_model(&work);
        assert_eq!(
            String::from_utf8_lossy(&model.workspace_branch),
            "workspace/dev",
            "inferred workspace branch"
        );
        assert_eq!(
            model.workspace_tip.to_string(),
            tip(&scratch, &work, WORKSPACE),
            "inferred tip against git show-ref"
        );
        assert!(
            model.is_personal(b".planning/x"),
            "is_personal(.planning/x) from the inferred manifest"
        );
    }

    /// D-21: a local `personal/*` branch is a candidate too.
    #[test]
    fn personal_branch_is_a_candidate() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-personal-candidate").expect("create scratch");
        let work = repo_with_remote(&scratch);
        run(
            &scratch,
            &work,
            &["branch", "-m", "workspace/dev", "personal/dev"],
        );

        let model = inferred_model(&work);
        assert_eq!(
            String::from_utf8_lossy(&model.workspace_branch),
            "personal/dev",
            "inferred workspace branch"
        );
        assert_eq!(
            model.workspace_tip.to_string(),
            tip(&scratch, &work, "refs/heads/personal/dev"),
            "inferred tip against git show-ref"
        );
    }

    /// D-15: a fallback match names the branch and the repository stays not
    /// configured; only the key makes it configured, as the control shows.
    #[test]
    fn no_source_branch_is_not_configured() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-fallback-not-configured").expect("create scratch");
        let work = repo_with_remote(&scratch);
        match resolved(&work) {
            Ok(RepoModel::NotConfigured {
                inferred: Ok(model),
            }) => assert_eq!(
                String::from_utf8_lossy(&model.workspace_branch),
                "workspace/dev",
                "inferred workspace branch"
            ),
            other => {
                panic!("resolve: expected NotConfigured with an inferred model, got {other:?}")
            }
        }

        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");
        configured(&work);
    }

    /// Success Criterion 3: two candidates are refused, listing both in
    /// order; `personal/workspace` and `personal/workspace-backup` are two
    /// names, not one.
    #[test]
    fn several_candidates_are_listed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-several").expect("create scratch");
        let work = repo_with_remote(&scratch);
        two_personal_candidates(&scratch, &work);

        let err = match inferred(&work) {
            Err(err) => err,
            Ok(model) => panic!("inferred: expected Err(WorkspaceCandidates), got {model:?}"),
        };
        assert_eq!(
            listed_candidates(&err),
            ["personal/workspace", "personal/workspace-backup"],
            "candidates"
        );
        let rendered = err.to_string();
        assert!(
            rendered.contains("personal/workspace, personal/workspace-backup."),
            "{rendered:?} does not name both candidates"
        );
    }

    /// CONF-05: candidates are in byte order across both prefixes, so
    /// `personal/dev` comes before `workspace/dev`, the same on a repeat.
    #[test]
    fn candidates_sort_across_both_prefixes() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-candidate-order").expect("create scratch");
        let work = repo_with_remote(&scratch);
        run(
            &scratch,
            &work,
            &["branch", "personal/dev", "workspace/dev"],
        );

        let first = candidates_of(&work);
        assert_eq!(first, ["personal/dev", "workspace/dev"], "candidates");
        assert_eq!(candidates_of(&work), first, "candidates on a repeat");
    }

    /// Success Criterion 3: with no candidate the error says none exist and
    /// to run `wsx init`. The control: before `workspace/dev` was deleted,
    /// the same repository inferred it.
    #[test]
    fn zero_candidates_are_an_error() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-zero").expect("create scratch");
        let work = repo_with_remote(&scratch);
        assert_eq!(
            String::from_utf8_lossy(&inferred_model(&work).workspace_branch),
            "workspace/dev",
            "inferred before the deletion"
        );
        run(&scratch, &work, &["branch", "-q", "-D", "workspace/dev"]);

        let err = match inferred(&work) {
            Err(err) => err,
            Ok(model) => panic!("inferred: expected Err(WorkspaceCandidates), got {model:?}"),
        };
        assert_eq!(listed_candidates(&err), Vec::<String>::new(), "candidates");
        let rendered = err.to_string();
        assert!(
            rendered.ends_with("\nRun wsx init."),
            "{rendered:?} does not say run wsx init"
        );
    }

    /// D-21: several or zero candidates are not a refusal from `resolve`,
    /// which stays `Ok(NotConfigured)` so the guard can read it.
    #[test]
    fn several_candidates_do_not_fail_resolve() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-several-ok").expect("create scratch");
        let work = repo_with_remote(&scratch);
        two_personal_candidates(&scratch, &work);
        match resolved(&work) {
            Ok(RepoModel::NotConfigured { inferred: Err(err) }) => {
                assert_eq!(listed_candidates(&err).len(), 2, "candidates");
            }
            other => panic!("resolve with two candidates: got {other:?}"),
        }

        for branch in ["personal/workspace", "personal/workspace-backup"] {
            run(&scratch, &work, &["branch", "-q", "-D", branch]);
        }
        match resolved(&work) {
            Ok(RepoModel::NotConfigured { inferred: Err(err) }) => {
                assert!(listed_candidates(&err).is_empty(), "candidates");
            }
            other => panic!("resolve with no candidate: got {other:?}"),
        }
    }

    /// T-04-20: a corrupt loose ref under a candidate prefix fails the
    /// listing, and that failure stays in `inferred`: the repository is not
    /// configured, and the one readable candidate is not inferred.
    #[test]
    fn corrupt_candidate_ref_stays_in_inferred() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-corrupt-candidate").expect("create scratch");
        let work = repo_with_remote(&scratch);
        ok(write(
            &work.join(".git/refs/heads/workspace/bad"),
            b"garbage\n",
        ));
        assert_eq!(
            texts(&for_each_ref(&scratch, &work, "refs/heads/workspace/*")),
            ["refs/heads/workspace/dev"],
            "git for-each-ref skips the corrupt ref"
        );

        match inferred(&work) {
            Err(Error::Git(git::Error::Gix { what, .. })) => assert_eq!(
                what, "list the refs directly under refs/heads/workspace/",
                "the failed read"
            ),
            other => panic!("inferred: expected Err(Git(Gix)) from the listing, got {other:?}"),
        }
    }

    /// D-17: the recorded set is an input. One repository resolved twice:
    /// with nothing recorded `workspace/dev` and `workspace/feature` are two
    /// candidates; with `workspace/feature` recorded, `workspace/dev` is
    /// inferred. Production passes an empty set until phase 8.
    #[test]
    fn recorded_branches_are_not_candidates() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-recorded").expect("create scratch");
        let work = repo_with_remote(&scratch);
        run(&scratch, &work, &["branch", "workspace/feature"]);
        let repo = Repo::discover(&work).expect("discover the fixture");

        match not_configured_in(&repo, &BTreeSet::new()) {
            Err(err) => assert_eq!(
                listed_candidates(&err),
                ["workspace/dev", "workspace/feature"],
                "candidates with nothing recorded"
            ),
            Ok(model) => panic!("inferred with nothing recorded: got {model:?}"),
        }
        let recorded = BTreeSet::from([b"workspace/feature".to_vec()]);
        match not_configured_in(&repo, &recorded) {
            Ok(model) => assert_eq!(
                String::from_utf8_lossy(&model.workspace_branch),
                "workspace/dev",
                "inferred with workspace/feature recorded"
            ),
            Err(err) => panic!("inferred with workspace/feature recorded: got {err:?}"),
        }
    }

    /// A3: `workspace/a/b` is two levels under `workspace/`, so it is no
    /// candidate, as `git for-each-ref 'refs/heads/workspace/*'` agrees,
    /// though the branch exists.
    #[test]
    fn nested_branch_is_not_a_candidate() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-nested").expect("create scratch");
        let work = repo_with_remote(&scratch);
        run(&scratch, &work, &["branch", "-q", "-D", "workspace/dev"]);
        run(&scratch, &work, &["branch", "workspace/a/b"]);
        tip(&scratch, &work, "refs/heads/workspace/a/b");
        assert!(
            for_each_ref(&scratch, &work, "refs/heads/workspace/*").is_empty(),
            "git for-each-ref 'refs/heads/workspace/*' lists the nested branch"
        );

        assert_eq!(candidates_of(&work), Vec::<String>::new(), "candidates");
    }

    /// D-21: a remote-tracking `workspace/*` is no candidate.
    #[test]
    fn remote_only_branch_is_not_a_candidate() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-remote-only").expect("create scratch");
        let work = repo_with_remote(&scratch);
        run(&scratch, &work, &["branch", "-q", "-D", "workspace/dev"]);
        run(
            &scratch,
            &work,
            &["update-ref", "refs/remotes/origin/workspace/x", "HEAD"],
        );
        tip(&scratch, &work, "refs/remotes/origin/workspace/x");

        assert_eq!(candidates_of(&work), Vec::<String>::new(), "candidates");
    }

    /// CONF-05: `workspace-x` only extends the prefix's text; it is no
    /// candidate.
    #[test]
    fn name_extension_is_not_a_candidate() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-name-extension").expect("create scratch");
        let work = repo_with_remote(&scratch);
        run(&scratch, &work, &["branch", "-q", "-D", "workspace/dev"]);
        run(&scratch, &work, &["branch", "workspace-x"]);
        tip(&scratch, &work, "refs/heads/workspace-x");

        assert_eq!(candidates_of(&work), Vec::<String>::new(), "candidates");
    }

    /// D-21: checking out one of two candidates does not choose it.
    #[test]
    fn checked_out_branch_gets_no_preference() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-checked-out").expect("create scratch");
        let work = repo_with_remote(&scratch);
        run(&scratch, &work, &["branch", "workspace/other"]);
        run(&scratch, &work, &["switch", "-q", "workspace/dev"]);
        assert_eq!(
            String::from_utf8_lossy(&run(&scratch, &work, &["symbolic-ref", "HEAD"])).trim_end(),
            WORKSPACE,
            "git symbolic-ref HEAD"
        );

        assert_eq!(
            candidates_of(&work),
            ["workspace/dev", "workspace/other"],
            "candidates with workspace/dev checked out"
        );
    }

    /// D-21: a `wsx.sourceBranch` with no local branch fails closed, with no
    /// fallback to the candidate `workspace/dev` that exists.
    #[test]
    fn missing_source_branch_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-source-missing").expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", "ghost");
        assert_ne!(
            exit_code(
                &scratch,
                &work,
                &["show-ref", "--verify", "refs/heads/ghost"]
            ),
            Some(0),
            "git show-ref --verify refs/heads/ghost"
        );
        tip(&scratch, &work, WORKSPACE);

        match refusal(&work) {
            Error::WorkspaceBranchMissing { branch } => {
                assert_eq!(String::from_utf8_lossy(&branch), "ghost", "branch");
            }
            other => panic!("resolve: expected Err(WorkspaceBranchMissing), got {other:?}"),
        }
    }

    /// 03.1 D-04: the key is a short name, so `refs/heads/workspace/dev` is
    /// read as `refs/heads/refs/heads/workspace/dev`, which does not exist.
    #[test]
    fn full_ref_as_source_branch_is_not_found() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-source-full-ref").expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", WORKSPACE);
        tip(&scratch, &work, WORKSPACE);
        assert_ne!(
            exit_code(
                &scratch,
                &work,
                &[
                    "show-ref",
                    "--verify",
                    "refs/heads/refs/heads/workspace/dev"
                ]
            ),
            Some(0),
            "git show-ref --verify refs/heads/refs/heads/workspace/dev"
        );

        match refusal(&work) {
            Error::WorkspaceBranchMissing { branch } => {
                assert_eq!(String::from_utf8_lossy(&branch), WORKSPACE, "branch");
            }
            other => panic!("resolve: expected Err(WorkspaceBranchMissing), got {other:?}"),
        }
    }

    /// A value `git check-ref-format --branch` refuses is not a branch name.
    #[test]
    fn invalid_source_branch_is_an_error() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-source-invalid").expect("create scratch");
        let work = repo_with_remote(&scratch);
        set_config(&scratch, &work, "wsx.sourceBranch", "-x");

        match refusal(&work) {
            Error::NotABranchName { key, value } => {
                assert_eq!(key, "wsx.sourceBranch", "key");
                assert_eq!(value, b"-x".to_vec(), "value");
            }
            other => panic!("resolve: expected Err(NotABranchName), got {other:?}"),
        }
    }

    /// D-16: an invalid manifest on the inferred branch leaves the
    /// repository not configured; the failure stays in `inferred`.
    #[test]
    fn invalid_inferred_manifest_is_not_configured() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-inferred-invalid").expect("create scratch");
        let work = repo_with_remote(&scratch);
        commit_manifest(&scratch, &work, Some(b"sync a\n"));

        match inferred(&work) {
            Err(Error::ManifestInvalid { branch, .. }) => {
                assert_eq!(String::from_utf8_lossy(&branch), "workspace/dev", "branch");
            }
            other => panic!("inferred: expected Err(ManifestInvalid), got {other:?}"),
        }
    }

    /// D-16: no manifest on the inferred branch leaves the repository not
    /// configured.
    #[test]
    fn missing_inferred_manifest_is_not_configured() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-inferred-missing").expect("create scratch");
        let work = repo_with_remote(&scratch);
        commit_manifest(&scratch, &work, None);

        match inferred(&work) {
            Err(Error::ManifestMissing { branch }) => {
                assert_eq!(String::from_utf8_lossy(&branch), "workspace/dev", "branch");
            }
            other => panic!("inferred: expected Err(ManifestMissing), got {other:?}"),
        }
    }

    /// D-16: the manifest's tree entry present and its object gone leaves
    /// the repository not configured, as `ManifestUnreadable` naming the
    /// branch.
    #[test]
    fn unreadable_inferred_manifest_is_not_configured() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-inferred-unreadable").expect("create scratch");
        let work = repo_with_remote(&scratch);
        delete_manifest_object(&scratch, &work);

        match inferred(&work) {
            Err(Error::ManifestUnreadable { branch, .. }) => {
                assert_eq!(String::from_utf8_lossy(&branch), "workspace/dev", "branch");
            }
            other => panic!("inferred: expected Err(ManifestUnreadable), got {other:?}"),
        }
    }

    /// The control for the D-16 tests: the same invalid manifest with
    /// `wsx.sourceBranch` set fails closed, as `Err` from `resolve`.
    #[test]
    fn invalid_manifest_with_the_key_set_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-key-invalid-manifest").expect("create scratch");
        let work = repo_with_remote(&scratch);
        commit_manifest(&scratch, &work, Some(b"sync a\n"));
        set_config(&scratch, &work, "wsx.sourceBranch", "workspace/dev");

        match refusal(&work) {
            Error::ManifestInvalid { branch, .. } => {
                assert_eq!(String::from_utf8_lossy(&branch), "workspace/dev", "branch");
            }
            other => panic!("resolve: expected Err(ManifestInvalid), got {other:?}"),
        }
    }

    #[test]
    fn require_returns_the_configured_model() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-require-configured").expect("create scratch");
        let work = chain_repo(&scratch);

        match required(&work) {
            Ok(model) => assert_eq!(
                model.workspace_tip.to_string(),
                tip(&scratch, &work, WORKSPACE),
                "workspace tip against git show-ref"
            ),
            Err(err) => panic!("require: expected a model, got {err:?}"),
        }
    }

    #[test]
    fn require_returns_the_inferred_model() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-require-inferred").expect("create scratch");
        let work = repo_with_remote(&scratch);

        match required(&work) {
            Ok(model) => {
                assert_eq!(
                    String::from_utf8_lossy(&model.workspace_branch),
                    "workspace/dev",
                    "workspace branch"
                );
                assert_eq!(
                    model.workspace_tip.to_string(),
                    tip(&scratch, &work, WORKSPACE),
                    "workspace tip against git show-ref"
                );
            }
            Err(err) => panic!("require: expected the inferred model, got {err:?}"),
        }
    }

    /// D-16: `require` reads an invalid inferred manifest as "not
    /// configured, run `wsx init`", keeping the parse error as the reason.
    #[test]
    fn require_wraps_a_manifest_failure_as_not_configured() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-require-invalid").expect("create scratch");
        let work = repo_with_remote(&scratch);
        commit_manifest(&scratch, &work, Some(b"sync a\n"));

        let (reason, rendered) = wrapped_reason(&work);
        assert!(
            matches!(reason, Error::ManifestInvalid { .. }),
            "reason: expected ManifestInvalid, got {reason:?}"
        );
        assert!(
            rendered.contains(": sync a\n") && rendered.ends_with("\nRun wsx init."),
            "{rendered:?} does not carry the parse error and say run wsx init"
        );
    }

    /// D-16: `require` reads a missing inferred manifest as not configured.
    /// The control, still with no key: once `workspace/dev` commits a
    /// manifest again, `require` returns the model.
    #[test]
    fn require_wraps_a_missing_manifest_as_not_configured() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-require-missing").expect("create scratch");
        let work = repo_with_remote(&scratch);
        commit_manifest(&scratch, &work, None);

        let (reason, rendered) = wrapped_reason(&work);
        assert!(
            matches!(reason, Error::ManifestMissing { .. }),
            "reason: expected ManifestMissing, got {reason:?}"
        );
        assert!(
            rendered.ends_with("\nRun wsx init."),
            "{rendered:?} does not say run wsx init"
        );

        commit_manifest(&scratch, &work, Some(b"only .planning\n"));
        match required(&work) {
            Ok(model) => assert_eq!(
                String::from_utf8_lossy(&model.workspace_branch),
                "workspace/dev",
                "workspace branch once the manifest is back"
            ),
            Err(err) => panic!("require once the manifest is back: got {err:?}"),
        }
    }

    /// D-16: `require` reads an unreadable inferred manifest as not
    /// configured, and the rendering keeps the object store's error, which
    /// names the missing object. The control, still with no key: writing
    /// the same bytes back restores the object under the same id, and
    /// `require` returns the model.
    #[test]
    fn require_wraps_an_unreadable_manifest_as_not_configured() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-require-unreadable").expect("create scratch");
        let work = repo_with_remote(&scratch);
        let id = delete_manifest_object(&scratch, &work);

        let (reason, rendered) = wrapped_reason(&work);
        assert!(
            matches!(reason, Error::ManifestUnreadable { .. }),
            "reason: expected ManifestUnreadable, got {reason:?}"
        );
        assert!(
            rendered.contains(&id) && rendered.ends_with("\nRun wsx init."),
            "{rendered:?} does not name the object {id} and say run wsx init"
        );

        let copy = scratch.root.join("manifest-copy");
        ok(write(&copy, b"only .planning\n"));
        let copy_arg = copy.to_str().expect("UTF-8 scratch path");
        let restored = run(&scratch, &work, &["hash-object", "-w", copy_arg]);
        assert_eq!(
            String::from_utf8_lossy(&restored).trim_end(),
            id,
            "git hash-object -w of the same bytes"
        );
        match required(&work) {
            Ok(model) => assert_eq!(
                String::from_utf8_lossy(&model.workspace_branch),
                "workspace/dev",
                "workspace branch once the object is back"
            ),
            Err(err) => panic!("require once the object is back: got {err:?}"),
        }
    }

    /// D-21: `require` passes the candidate list through, not wrapped.
    #[test]
    fn require_lists_the_candidates() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-require-candidates").expect("create scratch");
        let work = repo_with_remote(&scratch);
        two_personal_candidates(&scratch, &work);

        match required(&work) {
            Err(err) => assert_eq!(
                listed_candidates(&err),
                ["personal/workspace", "personal/workspace-backup"],
                "candidates"
            ),
            Ok(model) => panic!("require: expected Err(WorkspaceCandidates), got {model:?}"),
        }
    }

    /// `require` passes a failure that is not a manifest read through: an
    /// inferred branch with no integration branch is `NoIntegrationBranch`.
    #[test]
    fn require_passes_other_failures_through() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-require-other").expect("create scratch");
        let work = repo_with_remote(&scratch);
        delete_refs(&scratch, &work, &[ORIGIN_DEVELOP, ORIGIN_NEXT]);
        run(
            &scratch,
            &work,
            &["update-ref", "--no-deref", "-d", ORIGIN_HEAD],
        );

        match required(&work) {
            Err(err) => {
                no_integration(&err);
            }
            Ok(model) => panic!("require: expected Err(NoIntegrationBranch), got {model:?}"),
        }
    }

    /// The line `git ls-tree refs/heads/workspace/dev -- .wsx.conf` prints
    /// in `work`, newline trimmed.
    fn workspace_manifest_line(scratch: &Scratch, work: &Path) -> String {
        let stdout = run(
            scratch,
            work,
            &["ls-tree", "refs/heads/workspace/dev", "--", ".wsx.conf"],
        );
        String::from_utf8_lossy(&stdout).trim_end().to_owned()
    }

    /// Fails the test unless `workspace_manifest_line` starts with
    /// `mode_and_type`, such as `120000 blob `.
    fn assert_workspace_mode(scratch: &Scratch, work: &Path, mode_and_type: &str) {
        let line = workspace_manifest_line(scratch, work);
        assert!(
            line.starts_with(mode_and_type),
            "git ls-tree refs/heads/workspace/dev -- .wsx.conf printed {line:?}, \
             not {mode_and_type:?}"
        );
    }

    /// On `workspace/dev` in `work`: `git rm -q .wsx.conf`, then `change` in
    /// the working tree, then `commit_all`; `main` is checked out again with
    /// `git switch -q -f`.
    fn replace_manifest(scratch: &Scratch, work: &Path, change: impl FnOnce(&Path)) {
        run(scratch, work, &["switch", "-q", "workspace/dev"]);
        run(scratch, work, &["rm", "-q", ".wsx.conf"]);
        change(work);
        ok(commit_all(scratch, work, "manifest shape", T0 + 4 * TICK));
        run(scratch, work, &["switch", "-q", "-f", "main"]);
    }

    /// `workspace/dev`'s `.wsx.conf` in `work` becomes a symlink to `a.txt`,
    /// which `git ls-tree` shows as `120000 blob`.
    fn symlink_manifest(scratch: &Scratch, work: &Path) {
        replace_manifest(scratch, work, |work| {
            std::os::unix::fs::symlink("a.txt", work.join(".wsx.conf"))
                .unwrap_or_else(|err| panic!("symlink .wsx.conf: {err}"));
        });
        assert_workspace_mode(scratch, work, "120000 blob ");
    }

    /// The branch and kind of `err`; fails the test unless it is
    /// `Error::ManifestNotABlob`.
    fn not_a_blob(err: &Error) -> (String, &'static str) {
        match err {
            Error::ManifestNotABlob { branch, kind } => {
                (String::from_utf8_lossy(branch).into_owned(), kind)
            }
            other => panic!("expected Err(ManifestNotABlob), got {other:?}"),
        }
    }

    /// The branch `err` names; fails the test unless it is
    /// `Error::ManifestUnreadable`.
    fn unreadable_branch(err: &Error) -> String {
        match err {
            Error::ManifestUnreadable { branch, .. } => {
                String::from_utf8_lossy(branch).into_owned()
            }
            other => panic!("expected Err(ManifestUnreadable), got {other:?}"),
        }
    }

    /// Success Criterion 5: no `.wsx.conf` at the tip of a configured
    /// workspace branch fails closed, naming the branch.
    #[test]
    fn missing_manifest_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-missing-manifest").expect("create scratch");
        let work = chain_repo(&scratch);
        commit_manifest(&scratch, &work, None);
        assert_eq!(
            workspace_manifest_line(&scratch, &work),
            "",
            "git ls-tree refs/heads/workspace/dev -- .wsx.conf"
        );

        let err = refusal(&work);
        let rendered = err.to_string();
        match err {
            Error::ManifestMissing { branch } => {
                assert_eq!(String::from_utf8_lossy(&branch), "workspace/dev", "branch");
            }
            other => panic!("resolve: expected Err(ManifestMissing), got {other:?}"),
        }
        assert!(
            rendered.starts_with("workspace/dev has no .wsx.conf at its tip"),
            "{rendered:?} does not name the branch"
        );
    }

    /// D-19, RESEARCH Pitfall 8: a symlink's link text is never parsed as a
    /// manifest.
    #[test]
    fn symlink_manifest_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-symlink-manifest").expect("create scratch");
        let work = chain_repo(&scratch);
        symlink_manifest(&scratch, &work);

        assert_eq!(
            not_a_blob(&refusal(&work)),
            ("workspace/dev".to_owned(), "symlink"),
            "resolve's refusal"
        );
    }

    #[test]
    fn directory_manifest_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-directory-manifest").expect("create scratch");
        let work = chain_repo(&scratch);
        replace_manifest(&scratch, &work, |work| {
            ok(write(&work.join(".wsx.conf/x"), b"x\n"));
        });
        assert_workspace_mode(&scratch, &work, "040000 tree ");

        assert_eq!(
            not_a_blob(&refusal(&work)),
            ("workspace/dev".to_owned(), "directory"),
            "resolve's refusal"
        );
    }

    /// A real gitlink, added to the index with mode 160000 and committed
    /// without `git add --all`, which would stage its removal.
    #[test]
    fn submodule_manifest_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-submodule-manifest").expect("create scratch");
        let work = chain_repo(&scratch);
        run(&scratch, &work, &["switch", "-q", "workspace/dev"]);
        run(&scratch, &work, &["rm", "-q", ".wsx.conf"]);
        let head = tip(&scratch, &work, "refs/heads/workspace/dev");
        let cacheinfo = format!("160000,{head},.wsx.conf");
        run(
            &scratch,
            &work,
            &["update-index", "--add", "--cacheinfo", &cacheinfo],
        );
        run(&scratch, &work, &["commit", "-q", "-m", "gitlink"]);
        run(&scratch, &work, &["switch", "-q", "-f", "main"]);
        assert_workspace_mode(&scratch, &work, "160000 commit ");
        let kind = run(
            &scratch,
            &work,
            &["cat-file", "-t", "refs/heads/workspace/dev:.wsx.conf"],
        );
        assert_eq!(
            String::from_utf8_lossy(&kind).trim_end(),
            "commit",
            "git cat-file -t refs/heads/workspace/dev:.wsx.conf"
        );

        assert_eq!(
            not_a_blob(&refusal(&work)),
            ("workspace/dev".to_owned(), "submodule"),
            "resolve's refusal"
        );
    }

    /// The tree entry present and its object gone fails closed as
    /// `ManifestUnreadable`, never as a missing manifest.
    #[test]
    fn missing_blob_object_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-missing-blob").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_manifest_object(&scratch, &work);

        assert_eq!(
            unreadable_branch(&refusal(&work)),
            "workspace/dev",
            "resolve's refusal"
        );
    }

    /// The tip commit's loose object gone: the ref still holds its id, and
    /// the read of the commit fails closed as `ManifestUnreadable`.
    #[test]
    fn missing_tip_commit_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-missing-tip").expect("create scratch");
        let work = chain_repo(&scratch);
        let id = tip(&scratch, &work, "refs/heads/workspace/dev");
        let (dir, file) = id.split_at(2);
        let path = work.join(".git/objects").join(dir).join(file);
        assert!(
            path.is_file(),
            "the tip commit {id} is not the loose object {}",
            path.display()
        );
        std::fs::remove_file(&path)
            .unwrap_or_else(|err| panic!("remove {}: {err}", path.display()));
        assert_ne!(
            exit_code(&scratch, &work, &["cat-file", "-e", &id]),
            Some(0),
            "git cat-file -e {id} after its object was deleted"
        );

        assert_eq!(
            unreadable_branch(&refusal(&work)),
            "workspace/dev",
            "resolve's refusal"
        );
    }

    #[test]
    fn invalid_manifest_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-invalid-manifest").expect("create scratch");
        let work = chain_repo(&scratch);
        commit_manifest(&scratch, &work, Some(b"only a\nsync a\n"));

        let err = refusal(&work);
        let rendered = err.to_string();
        match err {
            Error::ManifestInvalid { branch, source } => {
                assert_eq!(String::from_utf8_lossy(&branch), "workspace/dev", "branch");
                assert_eq!(source.line, 2, "the refused line");
            }
            other => panic!("resolve: expected Err(ManifestInvalid), got {other:?}"),
        }
        assert!(
            rendered.starts_with(".wsx.conf at the tip of workspace/dev: line 2: ")
                && rendered.ends_with(": sync a"),
            "{rendered:?} does not name line 2 and its text"
        );
    }

    /// The control for the non-blob refusals: an executable file is a
    /// regular file, and resolves.
    #[test]
    fn executable_manifest_resolves() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-executable-manifest").expect("create scratch");
        let work = chain_repo(&scratch);
        run(&scratch, &work, &["switch", "-q", "workspace/dev"]);
        run(
            &scratch,
            &work,
            &["update-index", "--chmod=+x", ".wsx.conf"],
        );
        run(&scratch, &work, &["commit", "-q", "-m", "executable"]);
        run(&scratch, &work, &["switch", "-q", "-f", "main"]);
        assert_workspace_mode(&scratch, &work, "100755 blob ");

        let model = configured(&work);
        assert_eq!(
            model.personal_paths(),
            vec![&b".wsx.conf"[..], &b".worktrees"[..], &b".planning"[..]],
            "personal paths"
        );
    }

    /// Third review, finding 2: a tip whose root tree is out of git's order
    /// fails closed, so a `.wsx.conf` git cannot resolve there reads neither
    /// as present nor as missing.
    #[test]
    fn unsorted_tree_manifest_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-unsorted-tree").expect("create scratch");
        let work = chain_repo(&scratch);
        let (commit, tree) = ok(testing::raw_tree_commit(
            &scratch,
            &work,
            &[("z.txt", b"z\n"), (".wsx.conf", b"only .planning\n")],
        ));
        run(
            &scratch,
            &work,
            &[
                "update-ref",
                "refs/heads/workspace/dev",
                &commit.to_string(),
            ],
        );
        assert_ne!(
            exit_code(
                &scratch,
                &work,
                &["cat-file", "blob", "refs/heads/workspace/dev:.wsx.conf"]
            ),
            Some(0),
            "git cat-file blob refs/heads/workspace/dev:.wsx.conf found it"
        );

        match refusal(&work) {
            Error::ManifestUnreadable {
                branch,
                source:
                    git::Error::TreeNotSorted {
                        commit: named_commit,
                        tree: named_tree,
                    },
            } => {
                assert_eq!(String::from_utf8_lossy(&branch), "workspace/dev", "branch");
                assert_eq!(named_commit, commit, "the commit TreeNotSorted names");
                assert_eq!(named_tree, tree, "the tree TreeNotSorted names");
            }
            other => {
                panic!("resolve: expected Err(ManifestUnreadable(TreeNotSorted)), got {other:?}")
            }
        }
    }

    /// D-16: with no `wsx.sourceBranch`, a symlink manifest on the inferred
    /// branch is `NotConfigured`, and `require` says run `wsx init`. The
    /// variant is asserted, not only the wrapping: link text parsed as a
    /// manifest would be `ManifestInvalid`, which `require` wraps too. The
    /// control, still with no key: a regular `.wsx.conf` committed in its
    /// place, and `require` returns the model.
    #[test]
    fn require_wraps_a_non_blob_manifest_as_not_configured() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-require-not-a-blob").expect("create scratch");
        let work = repo_with_remote(&scratch);
        symlink_manifest(&scratch, &work);

        match inferred(&work) {
            Err(err) => assert_eq!(
                not_a_blob(&err),
                ("workspace/dev".to_owned(), "symlink"),
                "inferred"
            ),
            Ok(model) => panic!("inferred: expected Err(ManifestNotABlob), got {model:?}"),
        }
        let (reason, rendered) = wrapped_reason(&work);
        assert_eq!(
            not_a_blob(&reason),
            ("workspace/dev".to_owned(), "symlink"),
            "require's reason"
        );
        assert!(
            rendered.ends_with("\nRun wsx init."),
            "{rendered:?} does not say run wsx init"
        );

        replace_manifest(&scratch, &work, |work| {
            ok(write(&work.join(".wsx.conf"), b"only .planning\n"));
        });
        assert_workspace_mode(&scratch, &work, "100644 blob ");
        match required(&work) {
            Ok(model) => assert_eq!(
                String::from_utf8_lossy(&model.workspace_branch),
                "workspace/dev",
                "workspace branch once the manifest is a regular file"
            ),
            Err(err) => panic!("require with a regular manifest: got {err:?}"),
        }
    }

    /// `workspace/dev`'s tip in `work` commits `.wsx.conf` holding `bytes`;
    /// `main` stays checked out.
    fn manifest_on_workspace(scratch: &Scratch, work: &Path, bytes: &[u8]) {
        commit_manifest(scratch, work, Some(bytes));
    }

    /// `chain_repo` with `bytes` as `workspace/dev`'s manifest, resolved.
    fn model_with_manifest(scratch: &Scratch, bytes: &[u8]) -> (PathBuf, Model) {
        let work = chain_repo(scratch);
        manifest_on_workspace(scratch, &work, bytes);
        let model = configured(&work);
        (work, model)
    }

    /// `paths` as text, for comparing personal-path lists.
    fn path_texts(paths: &[&[u8]]) -> Vec<String> {
        paths
            .iter()
            .map(|path| String::from_utf8_lossy(path).into_owned())
            .collect()
    }

    /// The model `work` resolves to with `wsx.worktreeDir` set to `value`.
    fn worktree_dir_resolved(scratch: &Scratch, work: &Path, value: &str) -> Result<Model, Error> {
        set_config(scratch, work, WORKTREE_DIR, value);
        match resolved(work) {
            Ok(RepoModel::Configured(model)) => Ok(*model),
            Ok(other) => panic!("resolve: expected Configured or an error, got {other:?}"),
            Err(err) => Err(err),
        }
    }

    /// The `value` and `problem` of `err`; fails the test unless it is
    /// `Error::WorktreeDirInvalid`.
    fn worktree_dir_problem(err: Error) -> (String, Problem) {
        match err {
            Error::WorktreeDirInvalid { value, problem } => {
                (String::from_utf8_lossy(&value).into_owned(), problem)
            }
            other => panic!("expected Err(WorktreeDirInvalid), got {other:?}"),
        }
    }

    /// CONF-02: no agent's directory is personal unless declared.
    #[test]
    fn undeclared_path_is_not_personal() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-undeclared").expect("create scratch");
        let (_, model) = model_with_manifest(&scratch, b"");
        assert!(model.manifest.entries.is_empty(), "the manifest's entries");

        assert!(
            !model.is_personal(b".planning/x"),
            "is_personal(.planning/x)"
        );
        assert!(!model.is_personal(b"CLAUDE.md"), "is_personal(CLAUDE.md)");
    }

    /// CONF-02: the same paths are personal once the manifest declares them
    /// `only`, in the same repository, on a fresh resolve.
    #[test]
    fn declared_path_is_personal() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-declared").expect("create scratch");
        let (work, before) = model_with_manifest(&scratch, b"");
        assert!(
            !before.is_personal(b".planning/x") && !before.is_personal(b"CLAUDE.md"),
            "the paths are personal before the manifest declares them"
        );

        manifest_on_workspace(&scratch, &work, b"only .planning\nonly CLAUDE.md\n");
        let after = configured(&work);
        assert!(
            after.is_personal(b".planning/x"),
            "is_personal(.planning/x)"
        );
        assert!(after.is_personal(b"CLAUDE.md"), "is_personal(CLAUDE.md)");
    }

    /// A `modified` line names a shared path, so it is read and not listed.
    #[test]
    fn modified_path_is_not_personal() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-modified").expect("create scratch");
        let (_, model) = model_with_manifest(&scratch, b"modified README.md\n");
        let entries: Vec<_> = model
            .manifest
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.kind.clone(),
                    String::from_utf8_lossy(&entry.path).into_owned(),
                )
            })
            .collect();
        assert_eq!(
            entries,
            [(EntryKind::Modified, "README.md".to_owned())],
            "the manifest's entries"
        );

        assert!(!model.is_personal(b"README.md"), "is_personal(README.md)");
        assert_eq!(
            path_texts(&model.personal_paths()),
            [".wsx.conf", ".worktrees"],
            "personal paths"
        );
    }

    #[test]
    fn built_ins_are_personal_with_an_empty_manifest() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-built-ins").expect("create scratch");
        let (_, model) = model_with_manifest(&scratch, b"");

        assert!(model.is_personal(b".wsx.conf"), "is_personal(.wsx.conf)");
        assert!(
            model.is_personal(b".worktrees/x"),
            "is_personal(.worktrees/x)"
        );
        assert!(!model.is_personal(b".wsx.confX"), "is_personal(.wsx.confX)");
        assert!(
            !model.is_personal(b".worktreesX"),
            "is_personal(.worktreesX)"
        );
    }

    /// D-20: `wsx.worktreeDir` replaces `.worktrees` in the built-ins.
    #[test]
    fn custom_worktree_dir_replaces_the_default() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-worktree-dir").expect("create scratch");
        let work = chain_repo(&scratch);
        manifest_on_workspace(&scratch, &work, b"only a\n");
        let model = worktree_dir_resolved(&scratch, &work, ".wt")
            .unwrap_or_else(|err| panic!("resolve with wsx.worktreeDir=.wt: {err:?}"));

        assert_eq!(
            path_texts(&model.personal_paths()),
            [".wsx.conf", ".wt", "a"],
            "personal paths"
        );
        assert!(model.is_personal(b".wt/x"), "is_personal(.wt/x)");
        assert!(
            !model.is_personal(b".worktrees/x"),
            "is_personal(.worktrees/x)"
        );
    }

    /// D-20, D-14: one trailing `/` is stripped, as from a manifest path.
    #[test]
    fn worktree_dir_trailing_slash_is_stripped() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-worktree-dir-slash").expect("create scratch");
        let work = chain_repo(&scratch);
        let model = worktree_dir_resolved(&scratch, &work, ".wt/")
            .unwrap_or_else(|err| panic!("resolve with wsx.worktreeDir=.wt/: {err:?}"));

        assert_eq!(model.worktree_dir, b".wt".to_vec(), "worktree dir");
        assert_eq!(
            path_texts(&model.personal_paths()),
            [".wsx.conf", ".wt", ".planning"],
            "personal paths"
        );
    }

    /// D-20, D-14: a `..` component is refused, as in a manifest path.
    #[test]
    fn worktree_dir_with_a_dot_component_is_refused() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-worktree-dir-dot").expect("create scratch");
        let work = chain_repo(&scratch);
        let err = match worktree_dir_resolved(&scratch, &work, "../x") {
            Err(err) => err,
            Ok(model) => panic!("resolve with wsx.worktreeDir=../x: got {model:?}"),
        };

        assert_eq!(
            worktree_dir_problem(err),
            ("../x".to_owned(), Problem::DotComponent),
            "resolve's refusal"
        );
    }

    #[test]
    fn absolute_worktree_dir_is_refused() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-worktree-dir-absolute").expect("create scratch");
        let work = chain_repo(&scratch);
        let err = match worktree_dir_resolved(&scratch, &work, "/x") {
            Err(err) => err,
            Ok(model) => panic!("resolve with wsx.worktreeDir=/x: got {model:?}"),
        };

        assert_eq!(
            worktree_dir_problem(err),
            ("/x".to_owned(), Problem::AbsolutePath),
            "resolve's refusal"
        );
    }

    /// D-18: a repeated `wsx.worktreeDir` is refused with both values.
    #[test]
    fn repeated_worktree_dir_is_an_error() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-worktree-dir-repeated").expect("create scratch");
        let work = chain_repo(&scratch);
        run(
            &scratch,
            &work,
            &["config", "--local", "--add", WORKTREE_DIR, "a"],
        );
        run(
            &scratch,
            &work,
            &["config", "--local", "--add", WORKTREE_DIR, "b"],
        );

        match refusal(&work) {
            Error::KeyRepeated { key, values } => {
                assert_eq!(key, WORKTREE_DIR, "key");
                assert_eq!(texts(&values), ["a", "b"], "values");
            }
            other => panic!("resolve: expected Err(KeyRepeated), got {other:?}"),
        }
    }

    /// D-18: an empty `wsx.worktreeDir` is refused, not read as unset.
    #[test]
    fn empty_worktree_dir_is_an_error() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-worktree-dir-empty").expect("create scratch");
        let work = chain_repo(&scratch);
        set_config(&scratch, &work, WORKTREE_DIR, "");

        match refusal(&work) {
            Error::KeyEmpty { key } => assert_eq!(key, WORKTREE_DIR, "key"),
            other => panic!("resolve: expected Err(KeyEmpty), got {other:?}"),
        }
    }

    /// D-19's empty edge: a manifest of comments declares nothing, and the
    /// list is exactly the built-ins.
    #[test]
    fn comments_only_manifest_leaves_the_built_ins() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-comments-only").expect("create scratch");
        let (_, model) = model_with_manifest(&scratch, b"# personal paths\n#only x\n");

        assert_eq!(
            path_texts(&model.personal_paths()),
            [".wsx.conf", ".worktrees"],
            "personal paths"
        );
    }

    #[test]
    fn one_line_manifest_adds_one_path() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-one-line").expect("create scratch");
        let (_, model) = model_with_manifest(&scratch, b"only a\n");

        assert_eq!(
            path_texts(&model.personal_paths()),
            [".wsx.conf", ".worktrees", "a"],
            "personal paths"
        );
    }

    /// The list is the built-ins, then each `only` line in file order, and
    /// the same on a repeated call and on a fresh resolve.
    #[test]
    fn personal_list_is_built_ins_then_declarations_in_file_order() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-list-order").expect("create scratch");
        let (work, model) = model_with_manifest(&scratch, b"only z\nmodified m\nonly a\n");
        let first = path_texts(&model.personal_paths());
        assert_eq!(
            first,
            [".wsx.conf", ".worktrees", "z", "a"],
            "personal paths"
        );

        assert_eq!(
            path_texts(&model.personal_paths()),
            first,
            "personal paths on a second call"
        );
        assert_eq!(
            path_texts(&configured(&work).personal_paths()),
            first,
            "personal paths on a fresh resolve"
        );
    }

    /// D-13: duplicate declarations are kept, with no collision.
    #[test]
    fn duplicate_declarations_stay_in_the_list() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-duplicates").expect("create scratch");
        let (_, model) = model_with_manifest(&scratch, b"only a\nonly a\n");

        assert_eq!(
            path_texts(&model.personal_paths()),
            [".wsx.conf", ".worktrees", "a", "a"],
            "personal paths"
        );
        assert!(model.is_personal(b"a/x"), "is_personal(a/x)");
    }

    /// D-13: declaring a built-in is accepted and kept in the manifest with
    /// its line number, and `.wsx.conf` stays personal.
    #[test]
    fn built_in_declaration_stays_in_the_manifest() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-built-in-declared").expect("create scratch");
        let (_, model) = model_with_manifest(&scratch, b"only .wsx.conf\n");
        let entry = model
            .manifest
            .entries
            .first()
            .unwrap_or_else(|| panic!("the manifest has no entries: {:?}", model.manifest));

        assert_eq!(entry.line, 1, "the entry's line");
        assert_eq!(entry.kind, EntryKind::Only, "the entry's kind");
        assert_eq!(entry.path, b".wsx.conf".to_vec(), "the entry's path");
        assert!(model.is_personal(b".wsx.conf"), "is_personal(.wsx.conf)");
    }

    /// `personal_paths()` as `state_child` prints it: each path C-quoted,
    /// joined with commas.
    fn personal_marker(model: &Model) -> String {
        model
            .personal_paths()
            .iter()
            .map(|path| String::from_utf8_lossy(&git::c_quote(path)).into_owned())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// `personal_paths()` of `model` as owned text.
    fn personal_list(model: &Model) -> Vec<String> {
        path_texts(&model.personal_paths())
    }

    /// `path` as a git argument; the scratch root is UTF-8.
    fn arg(path: &Path) -> &str {
        path.to_str().expect("UTF-8 scratch path")
    }

    /// `<work>`'s linked worktree `<root>/linked`, detached at `main`'s
    /// commit, canonical.
    fn linked_worktree(scratch: &Scratch, work: &Path) -> PathBuf {
        let linked = scratch.root.join("linked");
        run(
            scratch,
            work,
            &["worktree", "add", "-q", "--detach", arg(&linked), "main"],
        );
        linked
            .canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize {}: {err}", linked.display()))
    }

    /// Success Criterion 4, T-04-23: an uncommitted edit to `.wsx.conf`,
    /// which `git status` sees, changes nothing.
    #[test]
    fn working_tree_edit_changes_nothing() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-dirty-manifest").expect("create scratch");
        let work = chain_repo(&scratch);
        let before = personal_list(&configured(&work));
        run(&scratch, &work, &["switch", "-q", "workspace/dev"]);
        ok(write(&work.join(".wsx.conf"), b"only other\n"));
        let status = run(&scratch, &work, &["status", "--porcelain"]);
        assert_eq!(
            String::from_utf8_lossy(&status),
            " M .wsx.conf\n",
            "git status --porcelain after the edit"
        );

        let after = configured(&work);
        assert_eq!(
            personal_list(&after),
            before,
            "personal paths after the edit"
        );
        assert!(!after.is_personal(b"other/x"), "is_personal(other/x)");
    }

    /// Success Criterion 4: a checkout without the file resolves the list
    /// the workspace branch's checkout does.
    #[test]
    fn clean_checkout_resolves_the_same_list() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-clean-checkout").expect("create scratch");
        let work = chain_repo(&scratch);
        assert!(
            !work.join(".wsx.conf").exists(),
            "main's checkout holds .wsx.conf"
        );
        let from_main = personal_list(&configured(&work));
        run(&scratch, &work, &["switch", "-q", "workspace/dev"]);
        assert!(
            work.join(".wsx.conf").is_file(),
            "workspace/dev's checkout has no .wsx.conf"
        );

        let from_workspace = personal_list(&configured(&work));
        assert_eq!(
            from_main, from_workspace,
            "the list from main against workspace/dev's"
        );
        assert_eq!(
            from_main,
            [".wsx.conf", ".worktrees", ".planning"],
            "personal paths"
        );
    }

    /// CONF-07: a linked worktree resolves the main worktree's model.
    #[test]
    fn linked_worktree_resolves_the_same_list() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-linked-worktree").expect("create scratch");
        let work = chain_repo(&scratch);
        let main = configured(&work);
        let linked = linked_worktree(&scratch, &work);
        assert!(
            !linked.join(".wsx.conf").exists(),
            "the linked worktree holds .wsx.conf"
        );

        let from_linked = configured(&linked);
        assert_eq!(
            personal_list(&from_linked),
            personal_list(&main),
            "the list from the linked worktree against the main one"
        );
        assert_eq!(
            from_linked.workspace_tip, main.workspace_tip,
            "workspace tip"
        );
    }

    /// CONF-07: a bare clone with no remote, configured there, resolves the
    /// same list, its integration branch the local `develop`.
    #[test]
    fn bare_repo_resolves_the_same_model() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-bare").expect("create scratch");
        let work = chain_repo(&scratch);
        let main = configured(&work);
        run(
            &scratch,
            &work,
            &["branch", "develop", "refs/remotes/origin/develop"],
        );
        let bare = scratch.root.join("bare.git");
        run(
            &scratch,
            &scratch.root,
            &["clone", "-q", "--bare", arg(&work), arg(&bare)],
        );
        let bare = bare
            .canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize {}: {err}", bare.display()));
        run(&scratch, &bare, &["remote", "remove", "origin"]);
        set_config(&scratch, &bare, "wsx.sourceBranch", "workspace/dev");
        let develop = tip(&scratch, &bare, "refs/heads/develop");

        let model = configured(&bare);
        assert_eq!(
            model.integration.full_ref,
            b"refs/heads/develop".to_vec(),
            "integration ref"
        );
        assert_eq!(
            model.integration.tip.to_string(),
            develop,
            "integration tip"
        );
        assert_eq!(model.base_remote, None, "base remote");
        assert_eq!(
            model.workspace_tip.to_string(),
            tip(&scratch, &bare, "refs/heads/workspace/dev"),
            "workspace tip against git show-ref in the bare repository"
        );
        assert_eq!(
            personal_list(&model),
            personal_list(&main),
            "the list from the bare repository against the clone's"
        );
    }

    /// CONF-07: a SHA-256 repository resolves the same list, its tip a
    /// 64-digit id.
    #[test]
    fn sha256_repo_resolves_the_same_model() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-sha256").expect("create scratch");
        let main = configured(&chain_repo(&scratch));
        let repo_dir = scratch.root.join("sha256");
        run(
            &scratch,
            &scratch.root,
            &[
                "init",
                "-q",
                "-b",
                "main",
                "--object-format=sha256",
                arg(&repo_dir),
            ],
        );
        ok(write(&repo_dir.join("a.txt"), b"a\n"));
        ok(commit_all(&scratch, &repo_dir, "main", T0));
        run(&scratch, &repo_dir, &["branch", "develop"]);
        run(
            &scratch,
            &repo_dir,
            &["switch", "-q", "-c", "workspace/dev"],
        );
        ok(write(&repo_dir.join(".wsx.conf"), b"only .planning\n"));
        ok(commit_all(&scratch, &repo_dir, "manifest", T0 + TICK));
        run(&scratch, &repo_dir, &["switch", "-q", "main"]);
        set_config(&scratch, &repo_dir, "wsx.sourceBranch", "workspace/dev");
        let repo_dir = repo_dir
            .canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize {}: {err}", repo_dir.display()));
        let workspace = tip(&scratch, &repo_dir, "refs/heads/workspace/dev");

        let model = configured(&repo_dir);
        let tip_hex = model.workspace_tip.to_string();
        assert_eq!(
            tip_hex.len(),
            64,
            "workspace tip {tip_hex} is not SHA-256 length"
        );
        assert_eq!(tip_hex, workspace, "workspace tip against git show-ref");
        assert_eq!(
            personal_list(&model),
            personal_list(&main),
            "the list from the SHA-256 repository against the SHA-1 one"
        );
    }

    /// CONF-07 from a hook: a real pre-commit hook of a partial commit in a
    /// linked worktree runs `state_child`, with the `GIT_DIR` and
    /// `GIT_INDEX_FILE` git exports to it, and the child resolves the main
    /// worktree's list. The child prints both variables, so the test fails
    /// if git did not export them.
    #[test]
    fn hook_environment_resolves_the_same_list() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-hook").expect("create scratch");
        let work = chain_repo(&scratch);
        let expected = personal_marker(&configured(&work));
        let linked = linked_worktree(&scratch, &work);
        ok(testing::install_hook(
            &work.join(".git/hooks"),
            "pre-commit",
            &child_path("state_child"),
        ));
        ok(write(&linked.join("a.txt"), b"a in linked\n"));
        let mut env = scratch.env.clone();
        env.push((START_VAR.into(), linked.clone().into()));
        let output = ok(testing::git_in_env(
            &env,
            &linked,
            ["commit", "-q", "-m", "x", "a.txt"],
        ));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run("the partial commit in the linked worktree", &output)
        );
        let git_dir = work.join(".git/worktrees/linked");
        assert_eq!(
            ok(testing::marker(&output, "env_GIT_DIR")),
            git_dir.display().to_string(),
            "the hook's GIT_DIR"
        );
        let index = PathBuf::from(ok(testing::marker(&output, "env_GIT_INDEX_FILE")));
        let index = ok(testing::canonical_parent(&linked, &index));
        assert_eq!(index.parent(), Some(git_dir.as_path()), "the hook's index");

        assert_eq!(
            ok(testing::marker(&output, "state")),
            "configured",
            "child state; error: {:?}",
            testing::marker(&output, "error")
        );
        assert_eq!(
            ok(testing::marker(&output, "personal")),
            expected,
            "the list in the hook against the main worktree's"
        );
    }

    /// A model is a value: a commit after resolving changes neither its tip
    /// nor its list, and a fresh resolve sees the new ones.
    #[test]
    fn model_is_a_snapshot_of_the_tip_it_read() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-snapshot").expect("create scratch");
        let work = chain_repo(&scratch);
        let first = configured(&work);
        let first_tip = tip(&scratch, &work, "refs/heads/workspace/dev");
        assert_eq!(first.workspace_tip.to_string(), first_tip, "first tip");

        manifest_on_workspace(&scratch, &work, b"only notes\n");
        let new_tip = tip(&scratch, &work, "refs/heads/workspace/dev");
        assert_ne!(new_tip, first_tip, "the commit moved workspace/dev");
        assert_eq!(
            first.workspace_tip.to_string(),
            first_tip,
            "the first model's tip after the commit"
        );
        assert_eq!(
            personal_list(&first),
            [".wsx.conf", ".worktrees", ".planning"],
            "the first model's list after the commit"
        );

        let fresh = configured(&work);
        assert_eq!(fresh.workspace_tip.to_string(), new_tip, "a fresh tip");
        assert_eq!(
            personal_list(&fresh),
            [".wsx.conf", ".worktrees", "notes"],
            "a fresh list"
        );
    }

    /// Every file under `<work>/.git`, as its path relative to `.git` and its
    /// bytes (a symlink's target), sorted by path; no timestamps or modes.
    fn repository_snapshot(work: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        use std::os::unix::ffi::OsStrExt;

        let root = work.join(".git");
        let mut files = Vec::new();
        let mut pending = vec![root.clone()];
        while let Some(dir) = pending.pop() {
            let entries = std::fs::read_dir(&dir)
                .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
            for entry in entries {
                let entry = entry.unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
                let path = entry.path();
                let kind = entry
                    .file_type()
                    .unwrap_or_else(|err| panic!("stat {}: {err}", path.display()));
                let bytes = if kind.is_dir() {
                    pending.push(path);
                    continue;
                } else if kind.is_symlink() {
                    std::fs::read_link(&path)
                        .unwrap_or_else(|err| panic!("read link {}: {err}", path.display()))
                        .as_os_str()
                        .as_bytes()
                        .to_vec()
                } else {
                    std::fs::read(&path)
                        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
                };
                let relative = path
                    .strip_prefix(&root)
                    .unwrap_or_else(|err| {
                        panic!("{} under {}: {err}", path.display(), root.display())
                    })
                    .to_path_buf();
                files.push((relative, bytes));
            }
        }
        files.sort();
        files
    }

    /// The paths at which `before` and `after` differ.
    fn changed_paths(
        before: &[(PathBuf, Vec<u8>)],
        after: &[(PathBuf, Vec<u8>)],
    ) -> BTreeSet<PathBuf> {
        let before: BTreeSet<_> = before.iter().collect();
        let after: BTreeSet<_> = after.iter().collect();
        before
            .symmetric_difference(&after)
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// D-09, CONF-02's concurrency edge: resolving leaves every file under
    /// `.git` byte-identical, so an interrupted or parallel resolve has
    /// nothing to corrupt.
    #[test]
    fn resolve_changes_nothing_in_the_repository() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-read-only").expect("create scratch");
        let work = chain_repo(&scratch);
        let before = repository_snapshot(&work);
        assert!(
            before
                .iter()
                .any(|(path, _)| path == Path::new("refs/heads/workspace/dev")),
            "the snapshot does not hold refs/heads/workspace/dev"
        );

        configured(&work);
        let after = repository_snapshot(&work);
        assert_eq!(
            changed_paths(&before, &after),
            BTreeSet::new(),
            "files under .git that resolving changed"
        );
    }

    /// The control for the snapshot: a ref update between two snapshots
    /// shows up, so the snapshot is not blind.
    #[test]
    fn repository_snapshot_sees_a_ref_change() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-snapshot-control").expect("create scratch");
        let work = chain_repo(&scratch);
        let before = repository_snapshot(&work);
        run(&scratch, &work, &["update-ref", "refs/heads/x", "HEAD"]);
        let after = repository_snapshot(&work);

        assert!(
            changed_paths(&before, &after).contains(Path::new("refs/heads/x")),
            "the snapshot missed the new ref refs/heads/x: {:?}",
            changed_paths(&before, &after)
        );
    }

    /// The workspace manifest as git names it, read through every
    /// replacement git follows.
    const WORKSPACE_MANIFEST: &str = "refs/heads/workspace/dev:.wsx.conf";

    /// `git rev-parse --verify <rev>` in `work`, as hex. Read every id a
    /// test needs before it adds a replacement, which changes this answer.
    fn rev_id(scratch: &Scratch, work: &Path, rev: &str) -> String {
        let stdout = run(scratch, work, &["rev-parse", "--verify", rev]);
        String::from_utf8_lossy(&stdout).trim_end().to_owned()
    }

    /// On a new branch `name` from `workspace/dev` in `work`, `.wsx.conf`
    /// holding `bytes`, committed, with `main` checked out again by
    /// `git switch -q -f`. Returns the side commit's id: a real commit, with
    /// its tree at `<id>^{tree}` and its manifest blob at `<id>:.wsx.conf`.
    fn side_manifest(scratch: &Scratch, work: &Path, name: &str, bytes: &[u8]) -> String {
        run(
            scratch,
            work,
            &["switch", "-q", "-c", name, "workspace/dev"],
        );
        ok(write(&work.join(".wsx.conf"), bytes));
        ok(commit_all(scratch, work, name, T0 + 5 * TICK));
        run(scratch, work, &["switch", "-q", "-f", "main"]);
        rev_id(scratch, work, name)
    }

    /// One commit's ids: the commit, its root tree and its `.wsx.conf` blob.
    struct Objects {
        commit: String,
        tree: String,
        blob: String,
    }

    /// The `Objects` of the commit `rev` names in `work`.
    fn objects(scratch: &Scratch, work: &Path, rev: &str) -> Objects {
        Objects {
            commit: rev_id(scratch, work, &format!("{rev}^{{commit}}")),
            tree: rev_id(scratch, work, &format!("{rev}^{{tree}}")),
            blob: rev_id(scratch, work, &format!("{rev}:.wsx.conf")),
        }
    }

    /// The workspace tip `a` (`only .planning`) and the side manifests `b`
    /// (`only b`) and `c` (`only c`) in `work`, each read before any
    /// replacement exists.
    struct Sides {
        a: Objects,
        b: Objects,
        c: Objects,
    }

    /// Makes the side manifests `b` and `c` in `work` and reads `Sides`.
    fn sides(scratch: &Scratch, work: &Path) -> Sides {
        let b = side_manifest(scratch, work, "side-b", b"only b\n");
        let c = side_manifest(scratch, work, "side-c", b"only c\n");
        Sides {
            a: objects(scratch, work, "refs/heads/workspace/dev"),
            b: objects(scratch, work, &b),
            c: objects(scratch, work, &c),
        }
    }

    /// What `git cat-file blob refs/heads/workspace/dev:.wsx.conf` answers
    /// in `work` under `env`, whatever its exit status.
    fn git_manifest_in(env: &testing::Env, work: &Path) -> Output {
        ok(testing::git_in_env(
            env,
            work,
            ["cat-file", "blob", WORKSPACE_MANIFEST],
        ))
    }

    /// The bytes git prints for the workspace manifest in `work`; fails the
    /// test, printing git's stderr, when git fails.
    fn git_prints(scratch: &Scratch, work: &Path) -> String {
        let output = git_manifest_in(&scratch.env, work);
        assert_eq!(
            output.status.code(),
            Some(0),
            "git cat-file blob {WORKSPACE_MANIFEST} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// git's stderr for the workspace manifest in `work`; fails the test
    /// when git reads it.
    fn git_refuses(scratch: &Scratch, work: &Path) -> String {
        let output = git_manifest_in(&scratch.env, work);
        assert_ne!(
            output.status.code(),
            Some(0),
            "git cat-file blob {WORKSPACE_MANIFEST} printed {:?}",
            String::from_utf8_lossy(&output.stdout)
        );
        String::from_utf8_lossy(&output.stderr).into_owned()
    }

    /// Asserts a fresh `Repo::discover` of `work` resolves `Configured` with
    /// the personal paths the manifest `only <only>` gives.
    fn assert_lists(work: &Path, only: &str) -> Model {
        let model = configured(work);
        assert_eq!(
            personal_list(&model),
            vec![
                ".wsx.conf".to_owned(),
                ".worktrees".to_owned(),
                only.to_owned()
            ],
            "personal paths"
        );
        model
    }

    /// The `name` and `target` of `err`, which must be `Error::Git`
    /// wrapping `git::Error::ReplacementRef`.
    fn replacement_ref(err: &Error) -> (String, String) {
        match err {
            Error::Git(git::Error::ReplacementRef { name, target }) => (
                String::from_utf8_lossy(name).into_owned(),
                String::from_utf8_lossy(target).into_owned(),
            ),
            other => panic!("expected Err(Git(ReplacementRef)), got {other:?}"),
        }
    }

    /// `refs/replace/<hex>`.
    fn replace_ref(hex: &str) -> String {
        format!("refs/replace/{hex}")
    }

    /// Asserts `rendered` names the replace ref `name` and the command that
    /// removes it, and does not say `.wsx.conf` cannot be read.
    fn assert_blames_the_ref(rendered: &str, name: &str) {
        assert!(
            rendered.contains(&format!("git update-ref --no-deref -d {name}")),
            "{rendered:?} does not name {name} and the command that removes it"
        );
        assert!(
            !rendered.contains("cannot be read"),
            "{rendered:?} says the manifest cannot be read"
        );
    }

    /// One replacement of the manifest blob reads the bytes git prints, and
    /// removing it gives the original list again.
    #[test]
    fn single_replacement_reads_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-blob").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        run(&scratch, &work, &["replace", &ids.a.blob, &ids.b.blob]);
        assert_eq!(git_prints(&scratch, &work), "only b\n", "git's manifest");

        assert_lists(&work, "b");

        run(&scratch, &work, &["replace", "-d", &ids.a.blob]);
        assert_eq!(
            git_prints(&scratch, &work),
            "only .planning\n",
            "git's manifest with the replacement removed"
        );
        assert_lists(&work, ".planning");
    }

    #[test]
    fn single_tree_replacement_reads_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-tree").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        run(&scratch, &work, &["replace", &ids.a.tree, &ids.b.tree]);
        assert_eq!(git_prints(&scratch, &work), "only b\n", "git's manifest");

        let model = assert_lists(&work, "b");
        assert_eq!(
            model.workspace_tip.to_string(),
            tip(&scratch, &work, "refs/heads/workspace/dev"),
            "workspace tip against git show-ref"
        );
    }

    #[test]
    fn single_commit_replacement_reads_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-commit").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        run(&scratch, &work, &["replace", &ids.a.commit, &ids.b.commit]);
        assert_eq!(git_prints(&scratch, &work), "only b\n", "git's manifest");

        let model = assert_lists(&work, "b");
        let shown_tip = tip(&scratch, &work, "refs/heads/workspace/dev");
        assert_eq!(shown_tip, ids.a.commit, "git show-ref's tip");
        assert_eq!(
            model.workspace_tip.to_string(),
            shown_tip,
            "workspace tip against git show-ref"
        );
    }

    /// A chain on the manifest blob, which git follows to its end and gix
    /// for one hop, is refused, naming the first link.
    #[test]
    fn replacement_chain_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-chain").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        run(&scratch, &work, &["replace", &ids.a.blob, &ids.b.blob]);
        run(&scratch, &work, &["replace", &ids.b.blob, &ids.c.blob]);
        assert_eq!(git_prints(&scratch, &work), "only c\n", "git's manifest");

        let err = refusal(&work);
        let name = replace_ref(&ids.a.blob);
        assert_eq!(
            replacement_ref(&err),
            (name.clone(), ids.b.blob.clone()),
            "the refused ref and its target"
        );
        assert_blames_the_ref(&err.to_string(), &name);
    }

    #[test]
    fn replacement_chain_on_the_tip_tree_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-tree-chain").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        run(&scratch, &work, &["replace", &ids.a.tree, &ids.b.tree]);
        run(&scratch, &work, &["replace", &ids.b.tree, &ids.c.tree]);
        assert_eq!(git_prints(&scratch, &work), "only c\n", "git's manifest");

        assert_eq!(
            replacement_ref(&refusal(&work)),
            (replace_ref(&ids.a.tree), ids.b.tree.clone()),
            "the refused ref and its target"
        );
    }

    #[test]
    fn replacement_chain_on_the_tip_commit_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-commit-chain").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        run(&scratch, &work, &["replace", &ids.a.commit, &ids.b.commit]);
        run(&scratch, &work, &["replace", &ids.b.commit, &ids.c.commit]);
        assert_eq!(git_prints(&scratch, &work), "only c\n", "git's manifest");

        assert_eq!(
            replacement_ref(&refusal(&work)),
            (replace_ref(&ids.a.commit), ids.b.commit.clone()),
            "the refused ref and its target"
        );
    }

    #[test]
    fn replacement_cycle_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-cycle").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        run(&scratch, &work, &["replace", &ids.a.blob, &ids.b.blob]);
        run(
            &scratch,
            &work,
            &["update-ref", &replace_ref(&ids.b.blob), &ids.a.blob],
        );
        let stderr = git_refuses(&scratch, &work);
        assert!(
            stderr.contains("replace depth too high"),
            "git's refusal {stderr:?}"
        );

        let (name, _) = replacement_ref(&refusal(&work));
        assert!(
            [replace_ref(&ids.a.blob), replace_ref(&ids.b.blob)].contains(&name),
            "the refused ref {name} is not a link of the cycle"
        );
    }

    #[test]
    fn symbolic_replacement_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-symbolic").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        run(&scratch, &work, &["update-ref", "refs/tags/x", &ids.b.blob]);
        let name = replace_ref(&ids.a.blob);
        run(&scratch, &work, &["symbolic-ref", &name, "refs/tags/x"]);
        assert_eq!(git_prints(&scratch, &work), "only b\n", "git's manifest");

        let err = refusal(&work);
        assert_eq!(
            replacement_ref(&err),
            (name.clone(), "refs/tags/x".to_owned()),
            "the refused ref and its target"
        );
        let rendered = err.to_string();
        assert!(
            rendered.contains("it points at refs/tags/x"),
            "{rendered:?} does not name refs/tags/x"
        );
        assert_blames_the_ref(&rendered, &name);
    }

    #[test]
    fn replacement_chain_through_an_uppercase_name_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-upper").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        let upper = ids.b.blob.to_ascii_uppercase();
        assert_ne!(upper, ids.b.blob, "the id of b has no hex letter");
        run(&scratch, &work, &["replace", &ids.a.blob, &ids.b.blob]);
        run(
            &scratch,
            &work,
            &["update-ref", &replace_ref(&upper), &ids.c.blob],
        );
        assert_eq!(git_prints(&scratch, &work), "only c\n", "git's manifest");

        assert_eq!(
            replacement_ref(&refusal(&work)),
            (replace_ref(&ids.a.blob), ids.b.blob.clone()),
            "the refused ref and its target"
        );
    }

    #[test]
    fn packed_replacement_chain_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-packed").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        run(&scratch, &work, &["replace", &ids.a.blob, &ids.b.blob]);
        run(&scratch, &work, &["replace", &ids.b.blob, &ids.c.blob]);
        run(&scratch, &work, &["pack-refs", "--all"]);
        let loose = work.join(".git").join(replace_ref(&ids.a.blob));
        assert!(
            !loose.exists(),
            "{} is still a loose ref after git pack-refs --all",
            loose.display()
        );
        assert_eq!(git_prints(&scratch, &work), "only c\n", "git's manifest");

        assert_eq!(
            replacement_ref(&refusal(&work)),
            (replace_ref(&ids.a.blob), ids.b.blob.clone()),
            "the refused ref and its target"
        );
    }

    /// D-22: a chain among objects the manifest read never touches stops
    /// the model too, while git still reads `.wsx.conf`, and the error
    /// blames the ref, not the manifest.
    #[test]
    fn replacement_chain_on_an_unrelated_object_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-unrelated").expect("create scratch");
        let work = chain_repo(&scratch);
        let x = side_manifest(&scratch, &work, "side-x", b"only x\n");
        let y = side_manifest(&scratch, &work, "side-y", b"only y\n");
        let z = side_manifest(&scratch, &work, "side-z", b"only z\n");
        let [x, y, z] = [x, y, z].map(|id| rev_id(&scratch, &work, &format!("{id}:.wsx.conf")));
        run(&scratch, &work, &["replace", &x, &y]);
        run(&scratch, &work, &["replace", &y, &z]);
        assert_eq!(
            git_prints(&scratch, &work),
            "only .planning\n",
            "git's manifest"
        );

        let err = refusal(&work);
        let name = replace_ref(&x);
        assert_eq!(
            replacement_ref(&err),
            (name.clone(), y),
            "the refused ref and its target"
        );
        let rendered = err.to_string();
        assert_blames_the_ref(&rendered, &name);
        assert!(
            !rendered.contains(".wsx.conf"),
            "{rendered:?} names .wsx.conf"
        );
    }

    /// The control for the replacement arm in `build`: a manifest that
    /// really cannot be read, with no replace ref, still says so.
    #[test]
    fn missing_blob_still_renders_cannot_be_read() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-control").expect("create scratch");
        let work = chain_repo(&scratch);
        delete_manifest_object(&scratch, &work);
        assert_eq!(
            run(&scratch, &work, &["for-each-ref", "refs/replace/"]),
            b"",
            "git for-each-ref refs/replace/"
        );

        let err = refusal(&work);
        let rendered = err.to_string();
        assert_eq!(
            unreadable_branch(&err),
            "workspace/dev",
            "resolve's refusal"
        );
        assert!(
            rendered.contains(".wsx.conf at the tip of") && rendered.contains("cannot be read"),
            "{rendered:?} does not say the manifest cannot be read"
        );
    }

    /// The variable `GIT_REPLACE_REF_BASE` and the base a test sets it to.
    const REPLACE_BASE_VAR: &str = "GIT_REPLACE_REF_BASE";
    const OTHER_BASE: &str = "refs/other/";

    /// With `GIT_REPLACE_REF_BASE` set, git and gix both read replace refs
    /// under it, so a chain there is refused; without it neither applies
    /// those refs and the model resolves.
    #[test]
    fn replacement_chain_under_the_env_base_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-env-base").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        let other = |hex: &str| format!("{OTHER_BASE}{hex}");
        run(
            &scratch,
            &work,
            &["update-ref", &other(&ids.a.blob), &ids.b.blob],
        );
        run(
            &scratch,
            &work,
            &["update-ref", &other(&ids.b.blob), &ids.c.blob],
        );
        let mut env = scratch.env.clone();
        env.push((REPLACE_BASE_VAR.into(), OTHER_BASE.into()));
        let under_env = git_manifest_in(&env, &work);
        assert_eq!(
            String::from_utf8_lossy(&under_env.stdout),
            "only c\n",
            "git's manifest under {REPLACE_BASE_VAR}={OTHER_BASE}: {}",
            String::from_utf8_lossy(&under_env.stderr).trim_end()
        );
        assert_eq!(
            git_prints(&scratch, &work),
            "only .planning\n",
            "git's manifest without {REPLACE_BASE_VAR}"
        );

        let state = child_state_with(&scratch, &work, &[(REPLACE_BASE_VAR, OTHER_BASE)]);
        let refused = format!("failed: refusing {}: ", other(&ids.a.blob));
        assert!(
            state.starts_with(&refused),
            "the child under {REPLACE_BASE_VAR}={OTHER_BASE} answered {state:?}, not {refused:?}"
        );
        assert_eq!(
            child_state(&scratch, &work),
            "configured",
            "the child without {REPLACE_BASE_VAR}"
        );
    }

    /// D-16: a chain in a repository with no `wsx.sourceBranch` stays in
    /// `inferred`, and `require` reads it as run `wsx init`.
    #[test]
    fn replacement_chain_leaves_a_not_configured_repo_not_configured() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-unconfigured").expect("create scratch");
        let work = repo_with_remote(&scratch);
        let ids = sides(&scratch, &work);
        run(&scratch, &work, &["replace", &ids.a.blob, &ids.b.blob]);
        run(&scratch, &work, &["replace", &ids.b.blob, &ids.c.blob]);
        assert_eq!(git_prints(&scratch, &work), "only c\n", "git's manifest");

        let answer = resolved(&work).expect("resolve a repository with no wsx.sourceBranch");
        let name = replace_ref(&ids.a.blob);
        let RepoModel::NotConfigured { inferred } = answer else {
            panic!("resolve: expected RepoModel::NotConfigured, got {answer:?}");
        };
        let inferred_err = match &inferred {
            Err(err) => err,
            Ok(model) => panic!("inferred: expected an error, got {model:?}"),
        };
        assert_eq!(
            replacement_ref(inferred_err),
            (name.clone(), ids.b.blob.clone()),
            "the inferred refusal"
        );

        let required = RepoModel::NotConfigured { inferred }.require();
        let err = match required {
            Err(err @ Error::NotConfigured { .. }) => err,
            other => panic!("require: expected Err(NotConfigured), got {other:?}"),
        };
        let rendered = err.to_string();
        assert_blames_the_ref(&rendered, &name);
        assert!(
            rendered.contains(&name) && rendered.ends_with("Run wsx init."),
            "{rendered:?} does not name {name} and say run wsx init"
        );
    }

    #[test]
    fn replace_refs_differing_only_in_case_fail_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-case").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        let lower = replace_ref(&ids.a.blob);
        let upper = replace_ref(&ids.a.blob.to_ascii_uppercase());
        assert_ne!(upper, lower, "the id of a has no hex letter");
        run(&scratch, &work, &["update-ref", &lower, &ids.b.blob]);
        run(&scratch, &work, &["update-ref", &upper, &ids.c.blob]);
        let stderr = git_refuses(&scratch, &work);
        assert!(
            stderr.contains("duplicate replace ref"),
            "git's refusal {stderr:?}"
        );

        let err = refusal(&work);
        let rendered = err.to_string();
        match &err {
            Error::Git(git::Error::ReplacementRefDuplicate { first, second }) => {
                let named: BTreeSet<String> = [first, second]
                    .into_iter()
                    .map(|name| String::from_utf8_lossy(name).into_owned())
                    .collect();
                assert_eq!(
                    named,
                    BTreeSet::from([lower.clone(), upper.clone()]),
                    "the refs the refusal names"
                );
            }
            other => panic!("expected Err(Git(ReplacementRefDuplicate)), got {other:?}"),
        }
        assert_blames_the_ref(&rendered, &lower);
        assert_blames_the_ref(&rendered, &upper);

        run(&scratch, &work, &["update-ref", "--no-deref", "-d", &upper]);
        assert_eq!(
            git_prints(&scratch, &work),
            "only b\n",
            "git's manifest with the uppercase ref deleted"
        );
        assert_lists(&work, "b");
    }

    /// The replace-base key only gix reads.
    const REPLACE_BASE_KEY: &str = "gitoxide.objects.replaceRefBase";

    /// `gitoxide.objects.replaceRefBase` set to `refs/other/` in `work`'s own
    /// config, with no `GIT_REPLACE_REF_BASE`.
    fn set_replace_base_key(scratch: &Scratch, work: &Path) {
        set_config(scratch, work, REPLACE_BASE_KEY, OTHER_BASE);
    }

    /// Asserts `err` is `Error::Git` wrapping `git::Error::ReplaceBaseDiffers`
    /// with gix's base `refs/other/` and git's `refs/replace/`, rendered with
    /// the key and the command that unsets it.
    fn assert_base_differs(err: &Error) {
        match err {
            Error::Git(git::Error::ReplaceBaseDiffers { value, git_base }) => {
                assert_eq!(
                    (
                        String::from_utf8_lossy(value).as_ref(),
                        String::from_utf8_lossy(git_base).as_ref()
                    ),
                    (OTHER_BASE, "refs/replace/"),
                    "gix's base and git's"
                );
            }
            other => panic!("expected Err(Git(ReplaceBaseDiffers)), got {other:?}"),
        }
        let rendered = err.to_string();
        for part in [
            REPLACE_BASE_KEY,
            OTHER_BASE,
            "refs/replace/",
            "git config --local --unset gitoxide.objects.replaceRefBase",
        ] {
            assert!(rendered.contains(part), "{rendered:?} does not name {part}");
        }
        assert!(
            !rendered.contains("cannot be read"),
            "{rendered:?} says the manifest cannot be read"
        );
    }

    #[test]
    fn replace_base_key_git_ignores_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-base-key").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        set_replace_base_key(&scratch, &work);
        let other = format!("{OTHER_BASE}{}", ids.a.blob);
        run(&scratch, &work, &["update-ref", &other, &ids.c.blob]);
        assert_eq!(
            git_prints(&scratch, &work),
            "only .planning\n",
            "git's manifest, which ignores {REPLACE_BASE_KEY}"
        );

        assert_base_differs(&refusal(&work));

        run(
            &scratch,
            &work,
            &["config", "--local", "--unset", REPLACE_BASE_KEY],
        );
        assert_lists(&work, ".planning");
    }

    /// The key alone stops nothing: with no ref under either base, neither
    /// reader applies a replacement.
    #[test]
    fn replace_base_key_without_a_replace_ref_resolves() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-base-alone").expect("create scratch");
        let work = chain_repo(&scratch);
        set_replace_base_key(&scratch, &work);
        assert_eq!(
            run(
                &scratch,
                &work,
                &["for-each-ref", OTHER_BASE, "refs/replace/"]
            ),
            b"",
            "git for-each-ref {OTHER_BASE} refs/replace/"
        );
        assert_eq!(
            git_prints(&scratch, &work),
            "only .planning\n",
            "git's manifest"
        );

        assert_lists(&work, ".planning");
    }

    /// git's base is listed as well as gix's: a ref under `refs/replace/`
    /// alone, which git applies and gix does not, is refused.
    #[test]
    fn replace_ref_under_git_base_with_the_key_set_fails_closed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("model-replace-base-git").expect("create scratch");
        let work = chain_repo(&scratch);
        let ids = sides(&scratch, &work);
        set_replace_base_key(&scratch, &work);
        run(&scratch, &work, &["replace", &ids.a.blob, &ids.b.blob]);
        assert_eq!(git_prints(&scratch, &work), "only b\n", "git's manifest");

        assert_base_differs(&refusal(&work));

        run(&scratch, &work, &["replace", "-d", &ids.a.blob]);
        assert_lists(&work, ".planning");
    }

    #[test]
    #[ignore = "child half of the model config-source tests; runs only when re-executed"]
    fn state_child() {
        let start = std::env::var_os(START_VAR).expect("the parent set the start directory");
        let answer = Repo::discover(Path::new(&start))
            .map_err(Error::Git)
            .and_then(|repo| resolve(&repo, &BTreeSet::new()));
        testing::child_print("env_GIT_DIR", testing::env_or_unset("GIT_DIR"));
        testing::child_print(
            "env_GIT_INDEX_FILE",
            testing::env_or_unset("GIT_INDEX_FILE"),
        );
        match answer {
            Ok(RepoModel::Configured(model)) => {
                testing::child_print("state", "configured");
                testing::child_print("base_remote", remote_marker(model.base_remote.as_deref()));
                testing::child_print("push_remote", remote_marker(model.push_remote.as_deref()));
                testing::child_print("personal", personal_marker(&model));
            }
            Ok(RepoModel::NotConfigured { .. }) => testing::child_print("state", "not-configured"),
            Err(err) => {
                testing::child_print("state", "failed");
                testing::child_print("error", err.to_string().replace('\n', "\\n"));
            }
        }
    }
}
