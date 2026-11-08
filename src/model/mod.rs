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

use crate::git::{self, ObjectId, Repo};

use manifest::{Manifest, ParseError, covers};

/// The local config key that makes a repository configured (D-15).
pub const SOURCE_BRANCH: &str = "wsx.sourceBranch";

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
    /// The git layer could not answer.
    Git(git::Error),
    /// The value of `key` is not a valid branch name.
    NotABranchName { key: &'static str, value: Vec<u8> },
    /// No ref in `tried` names an integration branch; `remote` is the base
    /// remote, or `None` when there is none.
    NoIntegrationBranch {
        remote: Option<Vec<u8>>,
        tried: Vec<Vec<u8>>,
    },
    /// `wsx.sourceBranch` names `branch`, and `refs/heads/<branch>` does not
    /// exist.
    WorkspaceBranchMissing { branch: Vec<u8> },
    /// `wsx.sourceBranch` is unset and not exactly one branch qualifies.
    WorkspaceCandidates { candidates: Vec<Vec<u8>> },
    /// The tip of `branch` has no `.wsx.conf`.
    ManifestMissing { branch: Vec<u8> },
    /// The `.wsx.conf` at the tip of `branch` does not parse.
    ManifestInvalid { branch: Vec<u8>, source: ParseError },
}

impl From<git::Error> for Error {
    fn from(err: git::Error) -> Error {
        Error::Git(err)
    }
}

/// `bytes` for a message.
fn shown(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `items` for a message, `, `-separated.
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
            Error::NotABranchName { key, value } => {
                write!(f, "{key} = {} is not a valid branch name", shown(value))
            }
            Error::NoIntegrationBranch { remote, tried } => {
                write!(
                    f,
                    "no integration branch: none of {} names an integration branch.\n",
                    listed(tried)
                )?;
                match remote {
                    Some(remote) => write!(
                        f,
                        "Fix it with one of:\n\
                         \tgit remote set-head {} --auto\n\
                         \tgit config --local wsx.baseRef <branch>",
                        shown(remote)
                    ),
                    None => f.write_str(
                        "Name it with:\n\
                         \tgit config --local wsx.baseRef <branch>",
                    ),
                }
            }
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
            Error::ManifestInvalid { branch, source } => {
                write!(f, ".wsx.conf at the tip of {}: {source}", shown(branch))
            }
        }
    }
}

/// Resolves the model of `repo`. `recorded` is the set of branches `wsx new`
/// recorded, which never count as workspace-branch candidates (D-17).
///
/// `Err` means exactly that the repository is configured and the model
/// fails closed; a repository that is not configured is a value.
pub fn resolve(_repo: &Repo, _recorded: &BTreeSet<Vec<u8>>) -> Result<RepoModel, Error> {
    Ok(RepoModel::NotConfigured {
        inferred: Err(Error::WorkspaceCandidates {
            candidates: Vec::new(),
        }),
    })
}

impl Model {
    /// The paths that never reach a shared branch.
    pub fn personal_paths(&self) -> Vec<&[u8]> {
        Vec::new()
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
    use std::path::{Path, PathBuf};

    use crate::git::Repo;
    use crate::git::testing::{self, Scratch, T0, TICK, commit_all, git_ok, write};

    use super::{Model, RepoModel, resolve};

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
}
