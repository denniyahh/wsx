//! Phase-worktree scenarios (plans 02-07 and 02-12).
//!
//! Each scenario creates or removes a phase worktree in a fixture repository
//! and states the refs, commits and files that must result; the oracle side
//! runs devflow's phase-worktree script, the Rust side `wsx`.
//!
//! Isolation: every scenario builds its fixture in its own
//! `harness-worktree-<name>-` tree under `CARGO_TARGET_TMPDIR`, in the
//! hermetic environment.

use crate::action::Action;
use crate::fixture::{ACTION_TIME, Ident, RepoConfig, T0, TICK, WORK};
use crate::outcome::{
    CommitExpect, FileExpect, FilesExpect, Outcome, RefExpect, RefsExpect, Tip,
};
use crate::scenario::{Case, Class, OracleSide, Scenario};
use crate::sync::{B_FILE, File, SEED_FILES, WORKSPACE_FILES, advance_develop, date, tree};

/// The new phase worktree, as wsx names it: after the branch (CONF-06).
const WSX_CHECKOUT: &str = "work/.worktrees/feature/phase-1";

/// The new phase worktree, as the oracle names it: after the phase number.
const ORACLE_CHECKOUT: &str = "work/.worktrees/phase-1";

/// A new worktree for `feature/phase-1` from `work` after `origin/develop`
/// advanced. The expectation comes from WTREE-01 (sync the workspace first,
/// then create the branch and its worktree from the workspace branch) and
/// CONF-06 (the worktree lives under `.worktrees/`, named after the branch).
/// The sync is the one `sync::self_test` expects, so a run that skips it fails
/// on the missing merge commit, not only on the worktree.
///
/// `fixed:` by observation: the oracle syncs and creates the same branch at
/// the same commit, but puts the worktree at `.worktrees/phase-1`, after the
/// phase number it derives, where wsx puts it at `.worktrees/feature/phase-1`,
/// after the branch it is given. The record is the oracle run's observed
/// outcome, which differs from the expectation only in that directory.
fn self_test() -> Scenario {
    let harness = Ident::harness();
    let feat_b = CommitExpect {
        message: b"feat: add b\n".to_vec(),
        author: Ident::ada(),
        author_date: date(T0 + 2 * TICK),
        committer_name: harness.name.clone(),
        committer_email: harness.email.clone(),
        parents: vec![b"chore: initial commit".to_vec()],
        tree: tree(&[SEED_FILES[0], SEED_FILES[1], B_FILE]),
        signed: false,
    };
    let all_files: Vec<File> = SEED_FILES
        .into_iter()
        .chain([B_FILE])
        .chain(WORKSPACE_FILES)
        .collect();
    let merge = CommitExpect {
        message: b"chore: sync develop into workspace/dev\n".to_vec(),
        author: harness.clone(),
        author_date: date(ACTION_TIME),
        committer_name: harness.name.clone(),
        committer_email: harness.email.clone(),
        parents: vec![
            b"chore(ws): add workspace files".to_vec(),
            b"feat: add b".to_vec(),
        ],
        tree: tree(&all_files),
        signed: false,
    };
    let tip = |name: &str, tip: Tip| RefExpect {
        name: name.to_owned(),
        tip,
    };
    let files_in = |checkout: &str| -> Vec<FileExpect> {
        all_files
            .iter()
            .map(|(path, content)| FileExpect::plain(checkout, path, content))
            .collect()
    };
    let outcome = |new_checkout: &str| Outcome {
        exit: 0,
        refs: RefsExpect::Exactly(vec![
            tip("refs/heads/develop", Tip::Commit(feat_b.clone())),
            tip("refs/heads/feature/phase-1", Tip::Commit(merge.clone())),
            tip("refs/heads/workspace/dev", Tip::Commit(merge.clone())),
            tip("refs/remotes/origin/HEAD", Tip::Unchanged),
            tip("refs/remotes/origin/develop", Tip::Commit(feat_b.clone())),
            tip("refs/remotes/origin/workspace/dev", Tip::Unchanged),
        ]),
        new_commits: vec![feat_b.clone(), merge.clone()],
        files: FilesExpect::Exactly(
            files_in(WORK)
                .into_iter()
                .chain(files_in(new_checkout))
                .collect(),
        ),
    };
    Scenario::new(
        "self_test",
        Case::new(
            RepoConfig::devflow(),
            advance_develop,
            Action::NewWorktree {
                checkout: WORK.to_owned(),
                branch: "feature/phase-1".to_owned(),
                no_sync: false,
            },
        )
        .expect(outcome(WSX_CHECKOUT))
        .class(Class::Fixed {
            defect: "the oracle names the worktree directory after the phase number \
                     (.worktrees/phase-1), not after the branch as given \
                     (.worktrees/feature/phase-1)",
        })
        .oracle(OracleSide::Recorded(outcome(ORACLE_CHECKOUT))),
    )
}

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 8";
}
