//! The Rust adapter: runs the `wsx` binary cargo built for this test run.
//!
//! Every Rust side is disabled in phase 2, so these command lines are best
//! guesses. The phase that enables a Rust side (5-10) revises the line for its
//! action in the same commit.

use std::ffi::OsString;
use std::process::Stdio;

use crate::action::Action;
use crate::env::{Env, ScenarioDirs, command, hermetic_env, inherited_env};
use crate::fixture::{Fixture, RepoConfig, WORK};
use crate::oracle::{ActionRun, AdapterError, spawn_and_wait};

/// The `wsx` binary under test.
const WSX: &str = env!("CARGO_BIN_EXE_wsx");

/// The wsx arguments for `action`. A pre-push guard gets the remote's URL as
/// git gives it to a hook, so `fixture` is consulted for it.
pub fn wsx_argv(action: &Action, fixture: &Fixture) -> Result<Vec<OsString>, String> {
    let mut argv: Vec<OsString> = Vec::new();
    let mut push = |arg: &str| argv.push(arg.into());
    match action {
        Action::Sync { .. } => push("sync"),
        Action::NewWorktree {
            branch, no_sync, ..
        } => {
            push("new");
            push(branch);
            if *no_sync {
                push("--no-sync");
            }
        }
        Action::Cut {
            scope, pr_branch, ..
        } => {
            push("cut");
            if let Some(scope) = scope {
                push("--scope");
                push(scope);
            }
            if let Some(pr_branch) = pr_branch {
                push(pr_branch);
            }
        }
        Action::Check { .. } => push("check"),
        Action::PrePush { remote, .. } => {
            push("hook");
            push("pre-push");
            push(remote);
            let input = action
                .push_input(fixture)?
                .ok_or("a pre-push action has push input")?;
            argv.push(input.url);
        }
        Action::RemoveWorktree { branch, force, .. } => {
            push("rm");
            push(branch);
            if *force {
                push("--force");
            }
        }
    }
    Ok(argv)
}

/// Runs `wsx` for `action` in its checkout, feeding a pre-push guard its ref
/// lines on stdin.
pub fn run(action: &Action, fixture: &Fixture, env: &Env) -> Result<ActionRun, AdapterError> {
    let argv = wsx_argv(action, fixture).map_err(AdapterError::Spawn)?;
    let input = action.push_input(fixture).map_err(AdapterError::Spawn)?;
    let mut child = command(WSX, &fixture.checkout(action.checkout()), env);
    child
        .args(&argv)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if input.is_some() {
        child.stdin(Stdio::piped());
    }
    spawn_and_wait(
        child,
        input.as_ref().map(|input| input.stdin.as_slice()),
        "wsx",
    )
}

fn strings(argv: Vec<OsString>) -> Vec<String> {
    argv.into_iter()
        .map(|arg| arg.into_string().expect("argv is UTF-8"))
        .collect()
}

/// Documents the best-guess wsx command line for every action in one
/// place. The phase that enables an action's Rust side revises its line.
#[test]
fn every_action_maps_to_a_wsx_command() {
    let dirs = ScenarioDirs::new("rust", "every_action").expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let fixture = Fixture::new(&dirs, env, RepoConfig::devflow()).expect("devflow fixture");
    let work = || WORK.to_owned();
    let remote_url = dirs.root.join("remotes/origin.git");
    let remote_url = remote_url.to_str().expect("UTF-8 scratch path");
    let cases: Vec<(Action, Vec<&str>)> = vec![
        (Action::Sync { checkout: work() }, vec!["sync"]),
        (
            Action::NewWorktree {
                checkout: work(),
                branch: "feature/x".to_owned(),
                no_sync: false,
            },
            vec!["new", "feature/x"],
        ),
        (
            Action::NewWorktree {
                checkout: work(),
                branch: "feature/x".to_owned(),
                no_sync: true,
            },
            vec!["new", "feature/x", "--no-sync"],
        ),
        (
            Action::Cut {
                checkout: work(),
                scope: None,
                pr_branch: None,
            },
            vec!["cut"],
        ),
        (
            Action::Cut {
                checkout: work(),
                scope: Some("02-02".to_owned()),
                pr_branch: Some("pr/x".to_owned()),
            },
            vec!["cut", "--scope", "02-02", "pr/x"],
        ),
        (Action::Check { checkout: work() }, vec!["check"]),
        (
            Action::PrePush {
                checkout: work(),
                remote: "origin".to_owned(),
                updates: Vec::new(),
            },
            vec!["hook", "pre-push", "origin", remote_url],
        ),
        (
            Action::RemoveWorktree {
                checkout: work(),
                branch: "feature/x".to_owned(),
                force: false,
            },
            vec!["rm", "feature/x"],
        ),
        (
            Action::RemoveWorktree {
                checkout: work(),
                branch: "feature/x".to_owned(),
                force: true,
            },
            vec!["rm", "feature/x", "--force"],
        ),
    ];
    for (action, expected) in cases {
        let argv = wsx_argv(&action, &fixture).expect("argv");
        assert_eq!(strings(argv), expected, "wsx command line for {action}");
    }
}
