//! Pre-push guard scenarios (plans 02-07 and 02-10).
//!
//! Each scenario feeds the guard the pushes git would hand a pre-push hook
//! and states whether the push must be refused, with the repository left
//! untouched either way.
//!
//! Isolation: every scenario builds its fixture in its own
//! `harness-guard-<name>-` tree under `CARGO_TARGET_TMPDIR`, in the hermetic
//! environment.

use crate::action::{Action, PushUpdate};
use crate::fixture::{Fixture, RepoConfig, WORK};
use crate::outcome::{FilesExpect, Outcome, RefsExpect};
use crate::scenario::{Case, Role, Scenario};

/// A checkout of the integration branch, beside the workspace checkout.
const DEVELOP_CHECKOUT: &str = "work/.worktrees/develop";

/// Fixture steps: a checkout of `develop` at `work/.worktrees/develop`, and
/// two branches made there from `develop`: `feature/clean`, whose one commit
/// touches only `src/clean.txt`, and `feature/leak`, whose one commit adds
/// `.planning/x.md`. The checkout is left on `develop`.
///
/// The branches start from `develop`, not from the workspace branch: a branch
/// made from `workspace/dev` carries `.planning` in its tree, which GUARD-01
/// (a) refuses however clean the branch's own commit is.
fn branches_from_develop(fixture: &mut Fixture) -> Result<(), String> {
    fixture.git_ok(
        WORK,
        ["worktree", "add", "--quiet", ".worktrees/develop", "develop"],
    )?;
    fixture.git_ok(DEVELOP_CHECKOUT, ["checkout", "--quiet", "-b", "feature/clean"])?;
    fixture.write(DEVELOP_CHECKOUT, b"src/clean.txt", b"clean\n")?;
    fixture.commit(DEVELOP_CHECKOUT, b"feat: add clean\n")?;
    fixture.git_ok(DEVELOP_CHECKOUT, ["checkout", "--quiet", "develop"])?;
    fixture.git_ok(DEVELOP_CHECKOUT, ["checkout", "--quiet", "-b", "feature/leak"])?;
    fixture.write(DEVELOP_CHECKOUT, b".planning/x.md", b"x\n")?;
    fixture.commit(DEVELOP_CHECKOUT, b"feat: add x\n")?;
    fixture.git_ok(DEVELOP_CHECKOUT, ["checkout", "--quiet", "develop"])?;
    Ok(())
}

/// The guard run from `checkout` for a push of local `branch` to the same
/// branch on `origin`.
fn push_from(checkout: &str, branch: &str) -> Case {
    let name = format!("refs/heads/{branch}");
    Case::new(
        RepoConfig::devflow(),
        branches_from_develop,
        Action::PrePush {
            checkout: checkout.to_owned(),
            remote: "origin".to_owned(),
            updates: vec![PushUpdate {
                local_ref: name.clone(),
                remote_ref: name,
            }],
        },
    )
}

/// The guard's verdict with the repository left as it was.
fn verdict(exit: i32) -> Outcome {
    Outcome {
        exit,
        refs: RefsExpect::Unchanged,
        new_commits: Vec::new(),
        files: FilesExpect::Unchanged,
    }
}

/// The oracle's self-test for the pre-push hook (HARN-01), a guard with both
/// controls (HARN-05), each case driven the way git drives the hook (remote
/// name and URL as arguments, ref lines on stdin). The subject pushes
/// `feature/clean` as a new ref from the `develop` checkout: its commits touch
/// only `src/`, so the push passes (GUARD-01). The known-positive control
/// pushes `feature/leak`, whose commit adds `.planning/x.md`, and must be
/// refused. The clean control pushes `workspace/dev` from the workspace
/// checkout, where the remote already holds that commit: the personal-path
/// policy skips a workspace branch, and the divergence check the hook runs in
/// a checkout carrying `.workspace-divergence` passes.
fn self_test() -> Scenario {
    Scenario::new(
        "self_test",
        push_from(DEVELOP_CHECKOUT, "feature/clean").expect(verdict(0)),
    )
    .control(
        Role::KnownPositive,
        push_from(DEVELOP_CHECKOUT, "feature/leak").expect(verdict(1)),
    )
    .control(
        Role::Clean,
        push_from(WORK, "workspace/dev").expect(verdict(0)),
    )
}

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 6";
}
