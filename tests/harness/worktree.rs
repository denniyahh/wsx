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
use crate::fixture::{ACTION_TIME, Fixture, Ident, RepoConfig, T0, TICK, WORK};
use crate::outcome::{CommitExpect, FileExpect, FilesExpect, Outcome, RefExpect, RefsExpect, Tip};
use crate::scenario::{Case, CaseKind, Class, FixtureFn, OracleSide, Role, Scenario};
use crate::sync::{
    B_FILE, File, NO_GSD_FILES, SEED_FILES, WORKSPACE_FILES, advance_develop, advance_upstream,
    date, fixture_commit, no_change, sync_merge, synced, synced_files, tree,
};

/// The new phase worktree, as wsx names it: after the branch (CONF-06).
const WSX_CHECKOUT: &str = "work/.worktrees/feature/phase-1";

/// The new phase worktree, as the oracle names it: after the phase number.
const ORACLE_CHECKOUT: &str = "work/.worktrees/phase-1";

/// The oracle's defect in every case where it creates a phase worktree.
const DIRECTORY_DEFECT: &str = "the oracle names the worktree directory after the phase number \
                                (.worktrees/phase-1), not after the branch as given \
                                (.worktrees/feature/phase-1)";

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
            defect: DIRECTORY_DEFECT,
        })
        .oracle(OracleSide::Recorded(outcome(ORACLE_CHECKOUT))),
    )
}

// ------------------------------------------------------- HARN-06 cases

/// The branch every new-worktree scenario creates or re-runs.
const PHASE_BRANCH: &str = "feature/phase-1";

/// The ref of `PHASE_BRANCH`.
const PHASE_REF: &str = "refs/heads/feature/phase-1";

/// The file `feat(1): work` adds on `feature/phase-1`.
const OWN_FILE: File = (b"src/w.txt", b"work\n");

/// `feature/topic`'s worktree in the removal scenario, named after the
/// branch as wsx names it (CONF-06).
const TOPIC_CHECKOUT: &str = "work/.worktrees/feature/topic";

/// A new worktree for `feature/phase-1` from `work`.
fn new_phase_worktree(config: RepoConfig, fixture: FixtureFn, no_sync: bool) -> Case {
    Case::new(
        config,
        fixture,
        Action::NewWorktree {
            checkout: WORK.to_owned(),
            branch: PHASE_BRANCH.to_owned(),
            no_sync,
        },
    )
}

/// `files` as plain files in `checkout`.
fn plain_files(checkout: &str, files: &[File]) -> Vec<FileExpect> {
    files
        .iter()
        .map(|(path, content)| FileExpect::plain(checkout, path, content))
        .collect()
}

/// `outcome` plus `PHASE_REF` at `tip` and `files` in `checkout`. `outcome`
/// must state its refs and files exactly.
fn with_phase_branch(mut outcome: Outcome, tip: Tip, checkout: &str, files: &[File]) -> Outcome {
    match (&mut outcome.refs, &mut outcome.files) {
        (RefsExpect::Exactly(refs), FilesExpect::Exactly(list)) => {
            refs.push(RefExpect {
                name: PHASE_REF.to_owned(),
                tip,
            });
            list.extend(plain_files(checkout, files));
        }
        _ => panic!("with_phase_branch extends an outcome with exact refs and files only"),
    }
    outcome
}

/// `feat: add b`, made on the integration branch at fixture commit `tick`.
fn feat_b_at(tick: i64) -> CommitExpect {
    fixture_commit(
        b"feat: add b\n",
        tick,
        b"chore: initial commit",
        &[SEED_FILES[0], SEED_FILES[1], B_FILE],
    )
}

/// The outcome of a new worktree for `feature/phase-1` after the sync merged
/// `feat: add b` (fixture commit 2) from `remote`'s `develop`: the branch at
/// the sync merge and its checkout `checkout` holding the merge's files.
fn synced_new_worktree(remote: &str, workspace_files: &[File], checkout: &str) -> Outcome {
    let ours = b"chore(ws): add workspace files";
    let feat_b = feat_b_at(2);
    let merge = sync_merge(ours, &feat_b, workspace_files);
    with_phase_branch(
        synced(remote, feat_b, ours, workspace_files, &[]),
        Tip::Commit(merge),
        checkout,
        &synced_files(workspace_files),
    )
}

/// Fixture step: a worktree at `work/.worktrees/phase-1`, where devflow puts
/// it, on a new branch `feature/phase-1` from `workspace/dev`.
fn phase_worktree(fixture: &mut Fixture) -> Result<(), String> {
    fixture.git_ok(
        WORK,
        [
            "worktree",
            "add",
            "--quiet",
            "-b",
            PHASE_BRANCH,
            ".worktrees/phase-1",
            "workspace/dev",
        ],
    )?;
    Ok(())
}

/// Fixture steps: `phase_worktree`, then `origin/develop` gains
/// `feat: add b` (fixture commit 2).
fn phase_worktree_then_advance(fixture: &mut Fixture) -> Result<(), String> {
    phase_worktree(fixture)?;
    advance_develop(fixture)
}

/// Fixture steps: `phase_worktree`, `feature/phase-1` commits
/// `feat(1): work` (fixture commit 2, adding `src/w.txt`), then
/// `origin/develop` gains `feat: add b` (fixture commit 3).
fn phase_work_then_advance(fixture: &mut Fixture) -> Result<(), String> {
    phase_worktree(fixture)?;
    fixture.write(ORACLE_CHECKOUT, OWN_FILE.0, OWN_FILE.1)?;
    fixture.commit(ORACLE_CHECKOUT, b"feat(1): work\n")?;
    advance_develop(fixture)
}

/// Re-running a new worktree for `feature/phase-1` when its worktree exists
/// and the branch has a commit of its own, after `origin/develop` advanced.
/// The expectation comes from WTREE-01 (sync the workspace first) and WTREE-03
/// (a re-run reports the existing worktree and exits 0; it fast-forwards only
/// a branch with no commits of its own, and never fails because work has
/// started): exit 0, `workspace/dev` at the sync merge, `feature/phase-1` and
/// its checkout untouched. The existing worktree sits at
/// `work/.worktrees/phase-1`, where devflow puts it, so both sides find it at
/// the same path.
///
/// `fixed:` by observation (RESEARCH W1): the oracle syncs, finds the
/// existing worktree and runs `merge --ff-only` there, which fails, so it
/// exits 1 with `not a fast-forward -- the branch has its own commits`. Its
/// refs, commits and files equal the expectation; only the exit code differs.
///
/// Counterpart: the same without `feat(1): work`. Both sides fast-forward
/// `feature/phase-1` to the sync merge and exit 0, Parity by observation.
fn rerun_with_own_commits() -> Scenario {
    let ours = b"chore(ws): add workspace files";
    let own_tree: Vec<File> = SEED_FILES
        .into_iter()
        .chain(WORKSPACE_FILES)
        .chain([OWN_FILE])
        .collect();
    let untouched = |exit| Outcome {
        exit,
        ..with_phase_branch(
            synced("origin", feat_b_at(3), ours, &WORKSPACE_FILES, &[]),
            Tip::Unchanged,
            ORACLE_CHECKOUT,
            &own_tree,
        )
    };
    let fast_forwarded = synced_new_worktree("origin", &WORKSPACE_FILES, ORACLE_CHECKOUT);
    Scenario::new(
        "rerun_with_own_commits",
        new_phase_worktree(RepoConfig::devflow(), phase_work_then_advance, false)
            .expect(untouched(0))
            .class(Class::Fixed {
                defect: "the oracle exits 1 when feature/phase-1 has a commit of its own",
            })
            .oracle(OracleSide::Recorded(untouched(1)))
            .oracle_stderr(b"not a fast-forward -- the branch has its own commits.\n"),
    )
    .control(
        Role::Counterpart,
        new_phase_worktree(RepoConfig::devflow(), phase_worktree_then_advance, false)
            .expect(fast_forwarded),
    )
}

/// A new worktree for `feature/phase-1` in a repo with no GSD, devflow or
/// agent files, after `origin/develop` advanced. The expectation comes from
/// WTREE-01 (sync, then create the branch and its worktree from the workspace
/// branch, never reading ROADMAP.md), WTREE-02 (the workspace branch tracks
/// the personal path `notes`, so creation goes ahead) and IF-02: exit 0,
/// `feature/phase-1` at the sync merge, and its checkout at
/// `work/.worktrees/feature/phase-1` (CONF-06).
///
/// This scenario is IF-02's negative control: the oracle needs a
/// `### Phase 1:` heading in `.planning/ROADMAP.md` and cannot work in such a
/// repo, and wsx must.
///
/// `fixed:` by observation: the oracle exits 2 before it syncs or creates
/// anything. Its `grep` for the heading fails on the missing
/// `.planning/ROADMAP.md`, and so does the `grep` that lists the existing
/// headings for its error message, whose exit status 2 `pipefail` and
/// `set -e` turn into the script's.
///
/// Counterpart: the devflow layout, where the oracle does create the
/// worktree (the case `self_test` records), `fixed:` for the directory name.
fn no_gsd_repo() -> Scenario {
    Scenario::new(
        "no_gsd_repo",
        new_phase_worktree(RepoConfig::no_gsd(), advance_develop, false)
            .expect(synced_new_worktree("origin", &NO_GSD_FILES, WSX_CHECKOUT))
            .class(Class::Fixed {
                defect: "the oracle exits 2 with no '### Phase 1:' heading in \
                         .planning/ROADMAP.md and creates nothing: no sync merge, no \
                         feature/phase-1, no worktree",
            })
            .oracle(OracleSide::Recorded(no_change(2)))
            .oracle_stderr(b"error: no '### Phase 1:' heading in .planning/ROADMAP.md.\n"),
    )
    .control(
        Role::Counterpart,
        new_phase_worktree(RepoConfig::devflow(), advance_develop, false)
            .expect(synced_new_worktree(
                "origin",
                &WORKSPACE_FILES,
                WSX_CHECKOUT,
            ))
            .class(Class::Fixed {
                defect: DIRECTORY_DEFECT,
            })
            .oracle(OracleSide::Recorded(synced_new_worktree(
                "origin",
                &WORKSPACE_FILES,
                ORACLE_CHECKOUT,
            ))),
    )
    .harn06(CaseKind::NoGsd)
}

/// A new worktree for `feature/phase-1` in a fork layout: the integration
/// branch comes from the base remote `upstream`, and the push remote `origin`
/// has no `develop`. The expectation comes from WTREE-01 (sync first, from
/// `<base-remote>/<integration>` per WSYNC-01) and CONF-04 (the base remote is
/// configured separately): exit 0, `workspace/dev` at the sync merge of
/// `upstream/develop`, `feature/phase-1` at the same commit, and its checkout
/// at `work/.worktrees/feature/phase-1`.
///
/// `fixed:` by observation: the oracle runs `sync-workspace.sh` with no
/// arguments, which fetches `develop` from its hard-coded `origin`. The push
/// remote has no `develop`, so the fetch fails (`fatal: couldn't find remote
/// ref develop`) and the oracle exits 128 before it creates anything.
///
/// Counterpart: the same with `--no-sync`. No fetch and no merge:
/// `feature/phase-1` at the `workspace/dev` tip and its checkout, every other
/// ref unchanged. `fixed:` by observation, for the directory name only.
fn fork_layout() -> Scenario {
    let fork = || RepoConfig::devflow().with_remotes("upstream", "origin");
    let workspace_files: Vec<File> = SEED_FILES.into_iter().chain(WORKSPACE_FILES).collect();
    let workspace_tip = fixture_commit(
        b"chore(ws): add workspace files\n",
        1,
        b"chore: initial commit",
        &workspace_files,
    );
    let unchanged = [
        "refs/heads/develop",
        "refs/heads/workspace/dev",
        "refs/remotes/origin/workspace/dev",
        "refs/remotes/upstream/HEAD",
        "refs/remotes/upstream/develop",
    ];
    let not_synced = |checkout: &str| {
        with_phase_branch(
            Outcome {
                exit: 0,
                refs: RefsExpect::Exactly(
                    unchanged
                        .iter()
                        .map(|name| RefExpect {
                            name: (*name).to_owned(),
                            tip: Tip::Unchanged,
                        })
                        .collect(),
                ),
                new_commits: Vec::new(),
                files: FilesExpect::Exactly(plain_files(WORK, &workspace_files)),
            },
            Tip::Commit(workspace_tip.clone()),
            checkout,
            &workspace_files,
        )
    };
    Scenario::new(
        "fork_layout",
        new_phase_worktree(fork(), advance_upstream, false)
            .expect(synced_new_worktree(
                "upstream",
                &WORKSPACE_FILES,
                WSX_CHECKOUT,
            ))
            .class(Class::Fixed {
                defect: "the oracle fetches develop from origin, not from the base remote \
                         upstream, exits 128 and creates nothing: no sync merge, no \
                         feature/phase-1, no worktree",
            })
            .oracle(OracleSide::Recorded(no_change(128)))
            .oracle_stderr(b"fatal: couldn't find remote ref develop\n"),
    )
    .control(
        Role::Counterpart,
        new_phase_worktree(fork(), advance_upstream, true)
            .expect(not_synced(WSX_CHECKOUT))
            .class(Class::Fixed {
                defect: DIRECTORY_DEFECT,
            })
            .oracle(OracleSide::Recorded(not_synced(ORACLE_CHECKOUT))),
    )
    .harn06(CaseKind::ForkLayout)
}

/// Fixture steps: a worktree at `work/.worktrees/feature/topic` on a new
/// branch `feature/topic` from `workspace/dev` commits `feat: add b`
/// (fixture commit 2, adding `src/b.txt`), and `workspace/dev` lands it with
/// a `--no-ff` merge `chore: land feature/topic` (fixture commit 3).
fn topic_landed(fixture: &mut Fixture) -> Result<(), String> {
    fixture.git_ok(
        WORK,
        [
            "worktree",
            "add",
            "--quiet",
            "-b",
            "feature/topic",
            ".worktrees/feature/topic",
            "workspace/dev",
        ],
    )?;
    fixture.write(TOPIC_CHECKOUT, B_FILE.0, B_FILE.1)?;
    fixture.commit(TOPIC_CHECKOUT, b"feat: add b\n")?;
    fixture.git_tick(
        WORK,
        [
            "merge",
            "--quiet",
            "--no-ff",
            "-m",
            "chore: land feature/topic",
            "feature/topic",
        ],
    )?;
    Ok(())
}

/// Fixture steps: `topic_landed`, then `origin/develop` gains the PR's
/// squash commit `feat: add b (#1)` (fixture commit 4), adding the same
/// `src/b.txt`, and `work` fetches it.
fn topic_landed_then_squashed(fixture: &mut Fixture) -> Result<(), String> {
    topic_landed(fixture)?;
    fixture.advance_remote("origin", "develop", |fixture, clone| {
        fixture.write(clone, B_FILE.0, B_FILE.1)?;
        fixture.commit(clone, b"feat: add b (#1)\n")
    })?;
    fixture.git_ok(WORK, ["fetch", "--quiet", "origin"])?;
    Ok(())
}

/// Why no case of `rm_after_squash_merge` has an oracle side.
const NO_ORACLE_RM: &str = "devflow has no command that removes a worktree";

/// Removing `feature/topic` and its worktree after its change landed on the
/// integration branch by squash merge. Rust-only: devflow has no remove
/// command. The expectation comes from WTREE-04: the branch is merged into
/// the workspace branch, and its only change, `src/b.txt`, is on
/// `origin/develop` by content (the squash commit), so `rm` removes the
/// worktree and the branch: exit 0, `refs/heads/feature/topic` gone, the
/// `work/.worktrees/feature/topic` checkout gone, every other ref unchanged.
///
/// Counterpart: `origin/develop` without the squash commit, so the change has
/// not landed; `rm` refuses with refs, commits and files unchanged.
fn rm_after_squash_merge() -> Scenario {
    let remove = |fixture: FixtureFn| {
        Case::new(
            RepoConfig::devflow(),
            fixture,
            Action::RemoveWorktree {
                checkout: WORK.to_owned(),
                branch: "feature/topic".to_owned(),
                force: false,
            },
        )
        .oracle(OracleSide::NotImplemented { why: NO_ORACLE_RM })
    };
    let kept = [
        "refs/heads/develop",
        "refs/heads/workspace/dev",
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/develop",
        "refs/remotes/origin/workspace/dev",
    ];
    let removed = Outcome {
        exit: 0,
        refs: RefsExpect::Exactly(
            kept.iter()
                .map(|name| RefExpect {
                    name: (*name).to_owned(),
                    tip: Tip::Unchanged,
                })
                .collect(),
        ),
        new_commits: Vec::new(),
        files: FilesExpect::Exactly(plain_files(WORK, &synced_files(&WORKSPACE_FILES))),
    };
    Scenario::new(
        "rm_after_squash_merge",
        remove(topic_landed_then_squashed).expect(removed),
    )
    .control(Role::Counterpart, remove(topic_landed).expect(no_change(1)))
    .harn06(CaseKind::SquashMerged)
}

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 8";
    rerun_with_own_commits: oracle, rust = "Rust side not enabled until phase 8";
    no_gsd_repo: oracle, rust = "Rust side not enabled until phase 8";
    fork_layout: oracle, rust = "Rust side not enabled until phase 8";
    rm_after_squash_merge: rust_only, rust = "Rust side not enabled until phase 8";
}
