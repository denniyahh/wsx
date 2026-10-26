//! PR-cut scenarios (plans 02-07, 02-11 and 02-14).
//!
//! Each scenario cuts a pull-request branch from a workspace branch, leaving
//! out the personal paths, and states the branch, commits and files that
//! must result.
//!
//! Isolation: every scenario builds its fixture in its own
//! `harness-cut-<name>-` tree under `CARGO_TARGET_TMPDIR`, in the hermetic
//! environment.

use crate::action::Action;
use crate::fixture::{Fixture, Ident, RepoConfig, T0, TICK, WORK};
use crate::outcome::{CommitExpect, FilesExpect, Outcome, RefExpect, RefsExpect, Tip};
use crate::scenario::{Case, Scenario};
use crate::sync::{B_FILE, SEED_FILES, date, tree};

/// The phase worktree the cut runs in.
const PHASE_CHECKOUT: &str = "work/.worktrees/phase-1";

/// Fixture steps: a worktree at `work/.worktrees/phase-1` on a new branch
/// `feature/phase-1` from `workspace/dev`, with two phase commits:
/// `feat(1): add b` adding `src/b.txt` (fixture commit 2), then
/// `docs(1): add plan` adding `.planning/PLAN.md`.
fn phase_branch(fixture: &mut Fixture) -> Result<(), String> {
    fixture.git_ok(
        WORK,
        [
            "worktree",
            "add",
            "--quiet",
            "-b",
            "feature/phase-1",
            ".worktrees/phase-1",
            "workspace/dev",
        ],
    )?;
    fixture.write(PHASE_CHECKOUT, B_FILE.0, B_FILE.1)?;
    fixture.commit(PHASE_CHECKOUT, b"feat(1): add b\n")?;
    fixture.write(PHASE_CHECKOUT, b".planning/PLAN.md", b"# Plan\n")?;
    fixture.commit(PHASE_CHECKOUT, b"docs(1): add plan\n")
}

/// The oracle's self-test for the PR cut (HARN-01): a phase-1 cut from the
/// phase worktree. The expectation comes from PRCUT-01 (a new PR branch from
/// the freshly fetched `origin/develop`, with commits replayed onto it),
/// PRCUT-02 (the planning-only commit is dropped; the replayed commit keeps
/// its author, author date and message, and is committed by the config
/// identity) and PRCUT-06, the source of the files-unchanged expectation (the
/// cut never touches the user's checkout).
///
/// Parity, by observation: devflow's script checks `feature/phase-1-pr` out
/// in the source worktree only while it replays, and returns to
/// `feature/phase-1` before it exits, so the transient checkout leaves no
/// observable trace in refs, commits or files.
fn self_test() -> Scenario {
    let harness = Ident::harness();
    let replayed = CommitExpect {
        message: b"feat(1): add b\n".to_vec(),
        author: Ident::ada(),
        author_date: date(T0 + 2 * TICK),
        committer_name: harness.name.clone(),
        committer_email: harness.email.clone(),
        parents: vec![b"chore: initial commit".to_vec()],
        tree: tree(&[SEED_FILES[0], SEED_FILES[1], B_FILE]),
        signed: false,
    };
    let tip = |name: &str, tip: Tip| RefExpect {
        name: name.to_owned(),
        tip,
    };
    let unchanged = |name: &str| tip(name, Tip::Unchanged);
    Scenario::new(
        "self_test",
        Case::new(
            RepoConfig::devflow(),
            phase_branch,
            Action::Cut {
                checkout: PHASE_CHECKOUT.to_owned(),
                scope: Some("1".to_owned()),
                pr_branch: None,
            },
        )
        .expect(Outcome {
            exit: 0,
            refs: RefsExpect::Exactly(vec![
                unchanged("refs/heads/develop"),
                unchanged("refs/heads/feature/phase-1"),
                tip(
                    "refs/heads/feature/phase-1-pr",
                    Tip::Commit(replayed.clone()),
                ),
                unchanged("refs/heads/workspace/dev"),
                unchanged("refs/remotes/origin/HEAD"),
                unchanged("refs/remotes/origin/develop"),
                unchanged("refs/remotes/origin/workspace/dev"),
            ]),
            new_commits: vec![replayed],
            files: FilesExpect::Unchanged,
        }),
    )
}

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 9";
}
