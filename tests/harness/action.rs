//! The abstract actions a scenario names, independent of any command line.
//!
//! A scenario says "sync in `work`", not `bash sync-workspace.sh origin
//! develop` or `wsx sync`. Each implementation maps an action through its own
//! adapter (`oracle.rs`, `rust.rs`), so a scenario stays valid after the oracle
//! is deleted and after wsx's CLI deliberately differs from the scripts.

use std::ffi::OsString;
use std::fmt;
use std::os::unix::ffi::OsStringExt;

use crate::env::{ScenarioDirs, hermetic_env, inherited_env};
use crate::fixture::{Fixture, RepoConfig, WORK};

/// A remote sha for a ref the remote does not have yet.
const ZERO_SHA: &str = "0000000000000000000000000000000000000000";

/// One ref a push updates, as git names it to a pre-push hook.
#[derive(Clone, Debug)]
pub struct PushUpdate {
    pub local_ref: String,
    pub remote_ref: String,
}

/// What the scenario does, in the checkout it does it from.
#[derive(Clone, Debug)]
pub enum Action {
    /// Merge the integration branch into the workspace branch.
    Sync { checkout: String },
    /// Create a worktree for `branch`, syncing first unless `no_sync`.
    NewWorktree {
        checkout: String,
        branch: String,
        no_sync: bool,
    },
    /// Cut a clean PR branch.
    Cut {
        checkout: String,
        scope: Option<String>,
        pr_branch: Option<String>,
    },
    /// Check the workspace divergence manifest.
    Check { checkout: String },
    /// Run the pre-push guard for a push of `updates` to `remote`.
    PrePush {
        checkout: String,
        remote: String,
        updates: Vec<PushUpdate>,
    },
    /// Remove the worktree for `branch`.
    RemoveWorktree {
        checkout: String,
        branch: String,
        force: bool,
    },
}

/// What git gives a pre-push hook: the remote's URL (its second argument) and
/// the ref lines on its stdin.
pub struct PushInput {
    pub url: OsString,
    pub stdin: Vec<u8>,
}

impl Action {
    /// The checkout the action runs in, relative to the scenario root.
    pub fn checkout(&self) -> &str {
        match self {
            Action::Sync { checkout }
            | Action::NewWorktree { checkout, .. }
            | Action::Cut { checkout, .. }
            | Action::Check { checkout }
            | Action::PrePush { checkout, .. }
            | Action::RemoveWorktree { checkout, .. } => checkout,
        }
    }

    /// For `PrePush` only: the remote URL from `git remote get-url` and one
    /// stdin line `<local ref> <local sha> <remote ref> <remote sha>` per
    /// update. The remote sha comes from `git ls-remote`, or is forty zeros for
    /// a ref the remote lacks. Every other action has no push input.
    pub fn push_input(&self, fixture: &Fixture) -> Result<Option<PushInput>, String> {
        let Action::PrePush {
            checkout,
            remote,
            updates,
        } = self
        else {
            return Ok(None);
        };
        let url = trimmed(fixture.git_ok(checkout, ["remote", "get-url", remote])?);
        let mut stdin = Vec::new();
        for PushUpdate {
            local_ref,
            remote_ref,
        } in updates
        {
            let local_sha = trimmed(fixture.git_ok(checkout, ["rev-parse", local_ref])?);
            let listed = fixture.git_ok(checkout, ["ls-remote", remote, remote_ref])?;
            let remote_sha = listed
                .split(|byte| *byte == b'\t')
                .next()
                .filter(|sha| !sha.is_empty())
                .map_or_else(|| ZERO_SHA.as_bytes().to_vec(), <[u8]>::to_vec);
            let line = [
                local_ref.as_bytes(),
                &local_sha,
                remote_ref.as_bytes(),
                &remote_sha,
            ]
            .join(&b' ');
            stdin.extend_from_slice(&line);
            stdin.push(b'\n');
        }
        Ok(Some(PushInput {
            url: OsString::from_vec(url),
            stdin,
        }))
    }
}

/// `bytes` without its trailing newline.
fn trimmed(mut bytes: Vec<u8>) -> Vec<u8> {
    while bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    bytes
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Sync { checkout } => write!(f, "sync in {checkout}"),
            Action::NewWorktree {
                checkout,
                branch,
                no_sync,
            } => {
                let sync = if *no_sync { " without syncing" } else { "" };
                write!(f, "new worktree for {branch} from {checkout}{sync}")
            }
            Action::Cut {
                checkout,
                scope,
                pr_branch,
            } => {
                write!(f, "cut a PR branch in {checkout}")?;
                if let Some(scope) = scope {
                    write!(f, " for scope {scope}")?;
                }
                if let Some(pr_branch) = pr_branch {
                    write!(f, " named {pr_branch}")?;
                }
                Ok(())
            }
            Action::Check { checkout } => write!(f, "divergence check in {checkout}"),
            Action::PrePush {
                checkout,
                remote,
                updates,
            } => {
                write!(f, "pre-push guard in {checkout} for a push to {remote} of")?;
                for PushUpdate {
                    local_ref,
                    remote_ref,
                } in updates
                {
                    write!(f, " {local_ref} -> {remote_ref}")?;
                }
                Ok(())
            }
            Action::RemoveWorktree {
                checkout,
                branch,
                force,
            } => {
                let force = if *force { " with --force" } else { "" };
                write!(f, "remove the worktree for {branch} from {checkout}{force}")
            }
        }
    }
}

/// The ref lines a pre-push guard reads: an existing remote ref carries the
/// remote's sha, a ref the remote lacks carries forty zeros.
#[test]
fn pre_push_input_lists_one_line_per_update() {
    let dirs = ScenarioDirs::new("action", "pre_push_input").expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let fixture = Fixture::new(&dirs, env, RepoConfig::devflow()).expect("devflow fixture");
    let tip = fixture
        .git_ok(WORK, ["rev-parse", "refs/heads/workspace/dev"])
        .expect("rev-parse");
    let tip = String::from_utf8(tip).expect("UTF-8 sha");
    let tip = tip.trim_end();
    let action = Action::PrePush {
        checkout: WORK.to_owned(),
        remote: "origin".to_owned(),
        updates: vec![
            PushUpdate {
                local_ref: "refs/heads/workspace/dev".to_owned(),
                remote_ref: "refs/heads/workspace/dev".to_owned(),
            },
            PushUpdate {
                local_ref: "refs/heads/workspace/dev".to_owned(),
                remote_ref: "refs/heads/feature/new".to_owned(),
            },
        ],
    };
    let input = action
        .push_input(&fixture)
        .expect("push input")
        .expect("a pre-push action has push input");
    let expected = format!(
        "refs/heads/workspace/dev {tip} refs/heads/workspace/dev {tip}\n\
         refs/heads/workspace/dev {tip} refs/heads/feature/new {zero}\n",
        zero = "0".repeat(40)
    );
    assert_eq!(
        String::from_utf8(input.stdin).expect("UTF-8 stdin"),
        expected,
        "pre-push stdin"
    );
    assert_eq!(
        input.url,
        dirs.root.join("remotes/origin.git").into_os_string()
    );
}
