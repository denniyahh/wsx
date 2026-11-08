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

use crate::git::{self, ObjectId, Repo, RootEntry};

use manifest::{EntryKind, Manifest, ParseError, Problem, covers};

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
            Error::KeyRepeated { .. }
            | Error::KeyEmpty { .. }
            | Error::NoSuchRemote { .. }
            | Error::BaseRefMissing { .. }
            | Error::IntegrationRefSymbolic { .. }
            | Error::ManifestNotABlob { .. }
            | Error::ManifestUnreadable { .. }
            | Error::WorktreeDirInvalid { .. }
            | Error::NotConfigured { .. } => f.write_str(""),
        }
    }
}

/// Resolves the model of `repo`. `recorded` is the set of branches `wsx new`
/// recorded, which never count as workspace-branch candidates (D-17).
///
/// `Err` means exactly that the repository is configured and the model
/// fails closed; a repository that is not configured is a value.
pub fn resolve(repo: &Repo, _recorded: &BTreeSet<Vec<u8>>) -> Result<RepoModel, Error> {
    match single(repo, SOURCE_BRANCH) {
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

/// The value of `key` in local config: the last one, or `None` when unset.
fn single(repo: &Repo, key: &'static str) -> Option<Vec<u8>> {
    repo.local_config_values(key).pop()
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

/// The model with `branch` as the workspace branch. The manifest comes from
/// the root tree of the commit `refs/heads/<branch>` holds, read from the
/// object database, so neither an uncommitted edit nor the checked-out
/// branch changes which paths are personal.
fn build(repo: &Repo, branch: Vec<u8>) -> Result<Model, Error> {
    let full = branch_ref(&branch);
    if repo.find_ref(&full)?.is_none() {
        return Err(Error::WorkspaceBranchMissing { branch });
    }
    let base_remote = default_remote(repo);
    let push_remote = default_remote(repo);
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

/// The integration branch: `develop` on the base remote's tracking refs, or
/// the local `develop` when there is no base remote (D-06, D-07).
fn integration(repo: &Repo, base: Option<&[u8]>) -> Result<Integration, Error> {
    let prefix = match base {
        Some(remote) => [b"refs/remotes/".as_slice(), remote, b"/".as_slice()].concat(),
        None => b"refs/heads/".to_vec(),
    };
    let branch = b"develop".to_vec();
    let full_ref = [prefix.as_slice(), branch.as_slice()].concat();
    match repo.find_ref(&full_ref)? {
        Some(_) => {
            let tip = repo.ref_commit(&full_ref)?;
            Ok(Integration {
                branch,
                full_ref,
                tip,
            })
        }
        None => Err(Error::NoIntegrationBranch {
            remote: base.map(<[u8]>::to_vec),
            tried: vec![full_ref],
        }),
    }
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
    use std::path::{Path, PathBuf};

    use crate::git::Repo;
    use crate::git::testing::{self, Scratch, T0, TICK, commit_all, git_ok, write};

    use crate::git;

    use super::manifest::{ParseError, Problem};
    use super::{Error, Model, RepoModel, resolve};

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
}
