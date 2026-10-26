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
use crate::scenario::{Case, CaseKind, Class, FixtureFn, OracleSide, Role, Scenario};
use crate::sync::{B_FILE, File, SEED_FILES, date, tree};

/// The phase worktree the cut runs in.
const PHASE_CHECKOUT: &str = "work/.worktrees/phase-1";

/// The worktree of `feature/topic`, the source of every cut with no scope.
const TOPIC_CHECKOUT: &str = "work/.worktrees/topic";

/// Fixture step: a worktree at `work/.worktrees/phase-1` on a new branch
/// `feature/phase-1` from `workspace/dev`.
fn phase_worktree(fixture: &mut Fixture) -> Result<(), String> {
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
    Ok(())
}

/// Fixture steps: a worktree at `work/.worktrees/phase-1` on a new branch
/// `feature/phase-1` from `workspace/dev`, with two phase commits:
/// `feat(1): add b` adding `src/b.txt` (fixture commit 2), then
/// `docs(1): add plan` adding `.planning/PLAN.md`.
fn phase_branch(fixture: &mut Fixture) -> Result<(), String> {
    phase_worktree(fixture)?;
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
/// observable trace in refs, commits, files, HEAD, index or operations in
/// progress.
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

// ------------------------------------------------------- shared helpers

/// The refs every preset holds: the integration and workspace branches and
/// their remote-tracking refs.
const PRESET_REFS: [&str; 5] = [
    "refs/heads/develop",
    "refs/heads/workspace/dev",
    "refs/remotes/origin/HEAD",
    "refs/remotes/origin/develop",
    "refs/remotes/origin/workspace/dev",
];

/// `src/c.txt`, a file a second code commit adds.
const C_FILE: File = (b"src/c.txt", b"c\n");

/// `src/d.txt`, a file a third code commit adds.
const D_FILE: File = (b"src/d.txt", b"d\n");

/// The author date of fixture commit `tick`.
fn at(tick: i64) -> String {
    date(T0 + tick * TICK)
}

/// A commit authored by Ada Author at fixture commit `tick` and committed by
/// the config identity, with one parent (subject `parent`) and `files` as its
/// whole tree. A replayed commit keeps its source's author, author date and
/// message (PRCUT-02) and is committed by the config identity.
fn commit(message: &[u8], tick: i64, parent: &[u8], files: &[File]) -> CommitExpect {
    let harness = Ident::harness();
    CommitExpect {
        message: message.to_vec(),
        author: Ident::ada(),
        author_date: at(tick),
        committer_name: harness.name,
        committer_email: harness.email,
        parents: vec![parent.to_vec()],
        tree: tree(files),
        signed: false,
    }
}

/// The seed files plus `extra`.
fn seed_plus(extra: &[File]) -> Vec<File> {
    SEED_FILES.iter().chain(extra).copied().collect()
}

/// The outcome of a cut that writes `pr_branch` at `pr_tip`: every preset
/// ref and every one of `branches` unchanged, `new_commits` new, and the
/// checkouts' files unchanged (PRCUT-06).
fn cut_outcome(
    exit: i32,
    branches: &[&str],
    pr_branch: &str,
    pr_tip: CommitExpect,
    new_commits: Vec<CommitExpect>,
) -> Outcome {
    let mut refs: Vec<RefExpect> = PRESET_REFS
        .iter()
        .chain(branches)
        .map(|name| RefExpect {
            name: (*name).to_owned(),
            tip: Tip::Unchanged,
        })
        .collect();
    refs.push(RefExpect {
        name: pr_branch.to_owned(),
        tip: Tip::Commit(pr_tip),
    });
    Outcome {
        exit,
        refs: RefsExpect::Exactly(refs),
        new_commits,
        files: FilesExpect::Unchanged,
    }
}

/// A refused cut: exit 1 with refs, commits and files unchanged.
fn refusal() -> Outcome {
    Outcome {
        exit: 1,
        refs: RefsExpect::Unchanged,
        new_commits: Vec::new(),
        files: FilesExpect::Unchanged,
    }
}

/// `case` expecting `expected`, where the oracle was observed doing
/// `recorded` instead.
fn fixed(case: Case, expected: Outcome, recorded: Outcome, defect: &'static str) -> Case {
    case.expect(expected)
        .class(Class::Fixed { defect })
        .oracle(OracleSide::Recorded(recorded))
}

/// Fixture step: a worktree at `work/.worktrees/topic` on a new branch
/// `feature/topic` from `workspace/dev`.
fn topic_worktree(fixture: &mut Fixture) -> Result<(), String> {
    fixture.git_ok(
        WORK,
        [
            "worktree",
            "add",
            "--quiet",
            "-b",
            "feature/topic",
            ".worktrees/topic",
            "workspace/dev",
        ],
    )?;
    Ok(())
}

/// Fixture steps: `files` written and committed as `message` in `checkout`.
fn commit_files(
    fixture: &mut Fixture,
    checkout: &str,
    files: &[File],
    message: &[u8],
) -> Result<(), String> {
    for (path, content) in files {
        fixture.write(checkout, path, content)?;
    }
    fixture.commit(checkout, message)
}

/// Fixture steps: `origin/develop` gains one commit `message` writing
/// `files`, and `work` fetches it, so the cut's own fetch changes nothing.
fn advance_and_fetch(
    fixture: &mut Fixture,
    files: &'static [File],
    message: &'static [u8],
) -> Result<(), String> {
    fixture.advance_remote("origin", "develop", |fixture, clone| {
        commit_files(fixture, clone, files, message)
    })?;
    fixture.git_ok(WORK, ["fetch", "--quiet", "origin"])?;
    Ok(())
}

/// A cut with no scope from `feature/topic` to `feature/topic-pr`, in a
/// devflow fixture built by `steps`.
fn topic_cut(steps: FixtureFn) -> Case {
    Case::new(
        RepoConfig::devflow(),
        steps,
        Action::Cut {
            checkout: TOPIC_CHECKOUT.to_owned(),
            scope: None,
            pr_branch: Some("feature/topic-pr".to_owned()),
        },
    )
}

/// A cut scoped to phase 1 from `feature/phase-1`, to the default
/// `feature/phase-1-pr`, in a devflow fixture built by `steps`.
fn phase_cut(steps: FixtureFn) -> Case {
    Case::new(
        RepoConfig::devflow(),
        steps,
        Action::Cut {
            checkout: PHASE_CHECKOUT.to_owned(),
            scope: Some("1".to_owned()),
            pr_branch: None,
        },
    )
}

// ------------------------------------------------- quoted personal names

/// Personal files named with a non-ASCII, a newline and a non-UTF-8 byte;
/// git C-quotes each of these names in its default path output.
const QUOTED_PERSONAL: [File; 3] = [
    (b".planning/\xC3\xA9.md", b"personal\n"),
    (b".planning/a\nb.md", b"personal\n"),
    (b".planning/\xFF.md", b"personal\n"),
];

/// A personal file with a plain ASCII name, which git never quotes.
const PLAIN_PERSONAL: [File; 1] = [(b".planning/plain.md", b"personal\n")];

/// Fixture steps: `feature/topic` with one commit `feat: add b` (fixture
/// commit 2) adding `src/b.txt` and the three quoted-name personal files.
fn topic_with_quoted_names(fixture: &mut Fixture) -> Result<(), String> {
    topic_worktree(fixture)?;
    let files: Vec<File> = [B_FILE].into_iter().chain(QUOTED_PERSONAL).collect();
    commit_files(fixture, TOPIC_CHECKOUT, &files, b"feat: add b\n")
}

/// Fixture steps: `feature/topic` with one commit `feat: add b` (fixture
/// commit 2) adding `src/b.txt` and `.planning/plain.md`.
fn topic_with_plain_name(fixture: &mut Fixture) -> Result<(), String> {
    topic_worktree(fixture)?;
    let files: Vec<File> = [B_FILE].into_iter().chain(PLAIN_PERSONAL).collect();
    commit_files(fixture, TOPIC_CHECKOUT, &files, b"feat: add b\n")
}

/// What the oracle was observed doing with the quoted names.
const QUOTED_NAMES_DEFECT: &str = "feature/topic-pr's commit keeps the quoted-name personal \
     files .planning/\\xC3\\xA9.md, .planning/a\\nb.md and .planning/\\xFF.md in its tree";

/// HARN-06 path names, PRCUT-02: personal paths are removed from every
/// replayed commit, whatever bytes their names hold, so wsx cuts
/// `feature/topic-pr` at one commit carrying only `src/b.txt` beyond the
/// seed. The counterpart control is the same commit with a plain-named
/// personal file, isolating the name bytes as the variable.
///
/// `fixed:`, by observation: devflow's cut exits 0 with all three quoted
/// names in the PR commit's tree, because it matches personal paths against
/// git's C-quoted path output (`".planning/..."`). Its counterpart is Parity:
/// `.planning/plain.md` is stripped.
fn quoted_personal_names() -> Scenario {
    let replayed = commit(
        b"feat: add b\n",
        2,
        b"chore: initial commit",
        &seed_plus(&[B_FILE]),
    );
    let expected = cut_outcome(
        0,
        &["refs/heads/feature/topic"],
        "refs/heads/feature/topic-pr",
        replayed.clone(),
        vec![replayed],
    );
    let leaked_files: Vec<File> = seed_plus(&[B_FILE])
        .into_iter()
        .chain(QUOTED_PERSONAL)
        .collect();
    let leaked = commit(b"feat: add b\n", 2, b"chore: initial commit", &leaked_files);
    let recorded = cut_outcome(
        0,
        &["refs/heads/feature/topic"],
        "refs/heads/feature/topic-pr",
        leaked.clone(),
        vec![leaked],
    );
    Scenario::new(
        "quoted_personal_names",
        fixed(
            topic_cut(topic_with_quoted_names),
            expected.clone(),
            recorded,
            QUOTED_NAMES_DEFECT,
        ),
    )
    .control(
        Role::Counterpart,
        topic_cut(topic_with_plain_name).expect(expected),
    )
    .harn06(CaseKind::PathNames)
}

// ------------------------------------------------------- merge commits

/// Fixture steps: `feature/topic` with `feat: add b` (fixture commit 2),
/// then a `--no-ff` merge (commit 4) of `feature/other`, made from
/// `workspace/dev` with `feat: add c` (commit 3) adding `src/c.txt`.
fn topic_with_topic_merge(fixture: &mut Fixture) -> Result<(), String> {
    topic_worktree(fixture)?;
    commit_files(fixture, TOPIC_CHECKOUT, &[B_FILE], b"feat: add b\n")?;
    fixture.git_ok(
        TOPIC_CHECKOUT,
        [
            "checkout",
            "--quiet",
            "-b",
            "feature/other",
            "workspace/dev",
        ],
    )?;
    commit_files(fixture, TOPIC_CHECKOUT, &[C_FILE], b"feat: add c\n")?;
    fixture.git_ok(TOPIC_CHECKOUT, ["checkout", "--quiet", "feature/topic"])?;
    fixture.git_tick(
        TOPIC_CHECKOUT,
        [
            "merge",
            "--quiet",
            "--no-ff",
            "-m",
            "Merge branch 'feature/other' into feature/topic",
            "feature/other",
        ],
    )?;
    Ok(())
}

/// Fixture steps: `feature/topic` with `feat: add b` (fixture commit 2) and
/// `feat: add c` (commit 3), linear.
fn topic_linear(fixture: &mut Fixture) -> Result<(), String> {
    topic_worktree(fixture)?;
    commit_files(fixture, TOPIC_CHECKOUT, &[B_FILE], b"feat: add b\n")?;
    commit_files(fixture, TOPIC_CHECKOUT, &[C_FILE], b"feat: add c\n")
}

/// What the oracle was observed doing with the merge of another topic.
const TOPIC_MERGE_DEFECT: &str = "the cut exits 0 and writes feature/topic-pr with two new \
     commits, feat: add b and feat: add c, the merge of feature/other left out";

/// HARN-06 merges, PRCUT-01: a merge whose merged-in side is neither the
/// integration nor the workspace branch refuses the cut, so wsx exits 1 on
/// `feature/topic` carrying a merge of `feature/other`, with refs, commits
/// and files unchanged. The counterpart control carries the same commits
/// without the merge, and wsx replays both.
///
/// `fixed:`, by observation: devflow's cut replays `feat: add b` and
/// `feat: add c` (reachable through the merge) and counts the merge as
/// `Excluded: 1 planning/environment-only commit(s)`, since `diff-tree`
/// lists no files for a merge; it exits 0. Its counterpart is Parity.
fn topic_merge_dropped() -> Scenario {
    let feat_b = commit(
        b"feat: add b\n",
        2,
        b"chore: initial commit",
        &seed_plus(&[B_FILE]),
    );
    let feat_c = commit(
        b"feat: add c\n",
        3,
        b"feat: add b",
        &seed_plus(&[B_FILE, C_FILE]),
    );
    let recorded = cut_outcome(
        0,
        &["refs/heads/feature/other", "refs/heads/feature/topic"],
        "refs/heads/feature/topic-pr",
        feat_c.clone(),
        vec![feat_b.clone(), feat_c.clone()],
    );
    Scenario::new(
        "topic_merge_dropped",
        fixed(
            topic_cut(topic_with_topic_merge),
            refusal(),
            recorded,
            TOPIC_MERGE_DEFECT,
        ),
    )
    .control(
        Role::Counterpart,
        topic_cut(topic_linear).expect(cut_outcome(
            0,
            &["refs/heads/feature/topic"],
            "refs/heads/feature/topic-pr",
            feat_c.clone(),
            vec![feat_b, feat_c],
        )),
    )
    .harn06(CaseKind::Merges)
}

/// `src/d.txt` added on `origin/develop` as `feat: add d`.
const DEVELOP_D: [File; 1] = [D_FILE];

/// Fixture steps: `origin/develop` gains `feat: add d` (fixture commit 2),
/// fetched; `feature/topic` with `feat: add b` (commit 3).
fn topic_after_develop_advanced(fixture: &mut Fixture) -> Result<(), String> {
    advance_and_fetch(fixture, &DEVELOP_D, b"feat: add d\n")?;
    topic_worktree(fixture)?;
    commit_files(fixture, TOPIC_CHECKOUT, &[B_FILE], b"feat: add b\n")
}

/// Fixture steps: `topic_after_develop_advanced`, then `origin/develop`
/// merged into `feature/topic` with `--no-ff` (fixture commit 4).
fn topic_with_integration_merge(fixture: &mut Fixture) -> Result<(), String> {
    topic_after_develop_advanced(fixture)?;
    fixture.git_tick(
        TOPIC_CHECKOUT,
        [
            "merge",
            "--quiet",
            "--no-ff",
            "-m",
            "Merge remote-tracking branch 'origin/develop' into feature/topic",
            "origin/develop",
        ],
    )?;
    Ok(())
}

/// HARN-06 merges, PRCUT-01: a merge whose merged-in side is the integration
/// branch is skipped, its content being on the base already, so wsx cuts
/// `feature/topic-pr` with only `feat: add b` replayed onto the advanced
/// `origin/develop`. The counterpart control is the same branch without the
/// merge.
///
/// Parity, by observation, subject and counterpart: devflow's cut replays
/// `feat: add d` to nothing (it is on the base), counts the merge as
/// excluded (`diff-tree` lists no files for it) and replays `feat: add b`.
fn integration_merged_into_feature() -> Scenario {
    let replayed = commit(
        b"feat: add b\n",
        3,
        b"feat: add d",
        &seed_plus(&[D_FILE, B_FILE]),
    );
    let expected = cut_outcome(
        0,
        &["refs/heads/feature/topic"],
        "refs/heads/feature/topic-pr",
        replayed.clone(),
        vec![replayed],
    );
    Scenario::new(
        "integration_merged_into_feature",
        topic_cut(topic_with_integration_merge).expect(expected.clone()),
    )
    .control(
        Role::Counterpart,
        topic_cut(topic_after_develop_advanced).expect(expected),
    )
    .harn06(CaseKind::Merges)
}

/// `src/a.txt` as `feature/topic` changes it.
const A_MINE: File = (b"src/a.txt", b"mine\n");

/// `src/a.txt` as `origin/develop` changes it.
const A_THEIRS: [File; 1] = [(b"src/a.txt", b"theirs\n")];

/// `src/c.txt` added on `origin/develop`.
const DEVELOP_C: [File; 1] = [C_FILE];

/// Fixture steps: `feature/topic` with `feat: change a` (fixture commit 2,
/// `src/a.txt` = `mine`); `origin/develop` gains `feat: change a upstream`
/// (commit 3, `src/a.txt` = `theirs`), fetched; `origin/develop` merged
/// into `feature/topic` (commit 4), the conflict resolved to `merged` and
/// the resolution adding `.planning/MERGE-NOTES.md`. Fails unless the merge
/// conflicts.
fn topic_with_resolution_adding_personal_file(fixture: &mut Fixture) -> Result<(), String> {
    topic_worktree(fixture)?;
    commit_files(fixture, TOPIC_CHECKOUT, &[A_MINE], b"feat: change a\n")?;
    advance_and_fetch(fixture, &A_THEIRS, b"feat: change a upstream\n")?;
    let merge = fixture.git(
        TOPIC_CHECKOUT,
        ["merge", "--quiet", "--no-commit", "origin/develop"],
    )?;
    if merge.status.code() != Some(1) {
        return Err(format!(
            "merging origin/develop into feature/topic exited {:?}, not 1 with a conflict",
            merge.status.code()
        ));
    }
    fixture.write(TOPIC_CHECKOUT, b"src/a.txt", b"merged\n")?;
    fixture.write(TOPIC_CHECKOUT, b".planning/MERGE-NOTES.md", b"# Notes\n")?;
    fixture.git_ok(TOPIC_CHECKOUT, ["add", "--all"])?;
    fixture.git_tick(TOPIC_CHECKOUT, ["commit", "--quiet", "--no-edit"])?;
    Ok(())
}

/// Fixture steps: `feature/topic` with `feat: change a` (fixture commit 2,
/// `src/a.txt` = `mine`); `origin/develop` gains `feat: add c` (commit 3),
/// fetched and merged into `feature/topic` with `--no-ff` (commit 4), with
/// no conflict and no resolution changes.
fn topic_with_clean_integration_merge(fixture: &mut Fixture) -> Result<(), String> {
    topic_worktree(fixture)?;
    commit_files(fixture, TOPIC_CHECKOUT, &[A_MINE], b"feat: change a\n")?;
    advance_and_fetch(fixture, &DEVELOP_C, b"feat: add c\n")?;
    fixture.git_tick(
        TOPIC_CHECKOUT,
        [
            "merge",
            "--quiet",
            "--no-ff",
            "-m",
            "Merge remote-tracking branch 'origin/develop' into feature/topic",
            "origin/develop",
        ],
    )?;
    Ok(())
}

/// HARN-06 merges, PRCUT-01: a merge of the integration branch is skipped
/// only when it carries no resolution changes of its own in non-personal
/// paths; this one resolves `src/a.txt` to `merged` (and adds a personal
/// file), so wsx refuses the cut, naming the merge, with refs, commits and
/// files unchanged. The counterpart control's integration merge has no
/// resolution changes; wsx skips it and replays `feat: change a`.
///
/// `fixed:`, by observation: devflow's cut also exits 1, but for another
/// reason and with a leftover: it replays `feat: change a` onto the advanced
/// `origin/develop` first, that conflicts on `src/a.txt` (`unresolved code
/// conflict`), and it exits leaving `feature/topic-pr` created at
/// `origin/develop`'s tip, `feat: change a upstream`. It never reaches the
/// merge. Its counterpart is Parity.
fn merge_resolution_adds_personal_file() -> Scenario {
    let replayed = commit(
        b"feat: change a\n",
        2,
        b"feat: add c",
        &[SEED_FILES[0], A_MINE, C_FILE],
    );
    let upstream = commit(
        b"feat: change a upstream\n",
        3,
        b"chore: initial commit",
        &[SEED_FILES[0], A_THEIRS[0]],
    );
    let recorded = cut_outcome(
        1,
        &["refs/heads/feature/topic"],
        "refs/heads/feature/topic-pr",
        upstream,
        Vec::new(),
    );
    Scenario::new(
        "merge_resolution_adds_personal_file",
        fixed(
            topic_cut(topic_with_resolution_adding_personal_file),
            refusal(),
            recorded,
            "the refused cut leaves a new branch feature/topic-pr behind",
        ),
    )
    .control(
        Role::Counterpart,
        topic_cut(topic_with_clean_integration_merge).expect(cut_outcome(
            0,
            &["refs/heads/feature/topic"],
            "refs/heads/feature/topic-pr",
            replayed.clone(),
            vec![replayed],
        )),
    )
    .harn06(CaseKind::Merges)
}

/// A personal file a merge resolution adds.
const PERSONAL_NOTES: &[u8] = b".planning/MERGE-NOTES.md";

/// A non-personal file a merge resolution adds.
const CODE_NOTES: &[u8] = b"src/MERGE-NOTES.md";

/// Fixture steps: `workspace/dev` gains `chore(ws): change a locally`
/// (fixture commit 2, `src/a.txt` = `ws`); `feature/topic` from it with
/// `feat: add b` (commit 3); `origin/develop` gains `feat: change a upstream`
/// (commit 4, `src/a.txt` = `theirs`), fetched and merged into
/// `feature/topic` (commit 5). The merge conflicts on `src/a.txt`, which the
/// resolution sets to `origin/develop`'s `theirs`, and the resolution adds
/// `notes`. The conflict's feature side comes from before the fork, so no
/// replayed commit touches `src/a.txt`. Fails unless the merge conflicts.
fn topic_merge_resolution_adding(fixture: &mut Fixture, notes: &[u8]) -> Result<(), String> {
    commit_files(
        fixture,
        WORK,
        &[(b"src/a.txt", b"ws\n")],
        b"chore(ws): change a locally\n",
    )?;
    topic_worktree(fixture)?;
    commit_files(fixture, TOPIC_CHECKOUT, &[B_FILE], b"feat: add b\n")?;
    advance_and_fetch(fixture, &A_THEIRS, b"feat: change a upstream\n")?;
    let merge = fixture.git(
        TOPIC_CHECKOUT,
        ["merge", "--quiet", "--no-commit", "origin/develop"],
    )?;
    if merge.status.code() != Some(1) {
        return Err(format!(
            "merging origin/develop into feature/topic exited {:?}, not 1 with a conflict",
            merge.status.code()
        ));
    }
    fixture.write(TOPIC_CHECKOUT, A_THEIRS[0].0, A_THEIRS[0].1)?;
    fixture.write(TOPIC_CHECKOUT, notes, b"# Notes\n")?;
    fixture.git_ok(TOPIC_CHECKOUT, ["add", "--all"])?;
    fixture.git_tick(TOPIC_CHECKOUT, ["commit", "--quiet", "--no-edit"])?;
    Ok(())
}

/// Fixture steps: `topic_merge_resolution_adding` the personal
/// `.planning/MERGE-NOTES.md`.
fn topic_merge_resolution_adding_personal(fixture: &mut Fixture) -> Result<(), String> {
    topic_merge_resolution_adding(fixture, PERSONAL_NOTES)
}

/// Fixture steps: `topic_merge_resolution_adding` the non-personal
/// `src/MERGE-NOTES.md`.
fn topic_merge_resolution_adding_code(fixture: &mut Fixture) -> Result<(), String> {
    topic_merge_resolution_adding(fixture, CODE_NOTES)
}

/// What the oracle was observed doing with a merge resolution that adds a
/// non-personal file.
const CODE_RESOLUTION_DEFECT: &str = "the cut exits 0 and writes feature/topic-pr with one new \
     commit feat: add b, the merge's added src/MERGE-NOTES.md left out";

/// HARN-06 merges, PRCUT-01: a merge of the integration branch is skipped
/// when its own resolution changes touch only personal paths. Here the
/// conflict on `src/a.txt` is resolved to `origin/develop`'s side, so the
/// merge's only change against both parents is the personal
/// `.planning/MERGE-NOTES.md`, and wsx cuts `feature/topic-pr` with `feat:
/// add b` replayed onto `origin/develop`. Every replay before the merge
/// applies cleanly, so the merge itself is reached.
///
/// The counterpart control flips one variable, the added file's path: the
/// resolution adds the non-personal `src/MERGE-NOTES.md`, a resolution
/// change of its own in a non-personal path, so wsx refuses the cut, naming
/// the merge, with refs, commits and files unchanged.
///
/// Parity, by observation: devflow's cut replays `feat: add b`, replays
/// `feat: change a upstream` (reachable through the merge) to nothing, and
/// counts the merge as excluded, since `diff-tree` lists no files for a
/// merge. Its counterpart is `fixed:`: the oracle does the same there and
/// exits 0.
fn merge_resolution_only_personal_file() -> Scenario {
    let replayed = commit(
        b"feat: add b\n",
        3,
        b"feat: change a upstream",
        &[SEED_FILES[0], A_THEIRS[0], B_FILE],
    );
    let cut = cut_outcome(
        0,
        &["refs/heads/feature/topic"],
        "refs/heads/feature/topic-pr",
        replayed.clone(),
        vec![replayed],
    );
    Scenario::new(
        "merge_resolution_only_personal_file",
        topic_cut(topic_merge_resolution_adding_personal).expect(cut.clone()),
    )
    .control(
        Role::Counterpart,
        fixed(
            topic_cut(topic_merge_resolution_adding_code),
            refusal(),
            cut,
            CODE_RESOLUTION_DEFECT,
        ),
    )
    .harn06(CaseKind::Merges)
}

// ------------------------------------------------ existing PR branch

/// Fixture steps: `feature/phase-1` with `feat(1): add b` (fixture commit 2).
fn phase_with_b(fixture: &mut Fixture) -> Result<(), String> {
    phase_worktree(fixture)?;
    commit_files(fixture, PHASE_CHECKOUT, &[B_FILE], b"feat(1): add b\n")
}

/// Fixture steps: `phase_with_b`, then `feature/phase-1-pr` made from
/// `origin/develop` with `feat(1): add b` cherry-picked (fixture commit 3)
/// and a reviewer commit `fix(1): review fixup` (commit 4) changing
/// `src/b.txt`; the phase worktree is left on `feature/phase-1`.
fn phase_with_existing_pr(fixture: &mut Fixture) -> Result<(), String> {
    phase_with_b(fixture)?;
    fixture.git_ok(
        PHASE_CHECKOUT,
        [
            "checkout",
            "--quiet",
            "--no-track",
            "-b",
            "feature/phase-1-pr",
            "origin/develop",
        ],
    )?;
    fixture.git_tick(PHASE_CHECKOUT, ["cherry-pick", "feature/phase-1"])?;
    commit_files(
        fixture,
        PHASE_CHECKOUT,
        &[(b"src/b.txt", b"b reviewed\n")],
        b"fix(1): review fixup\n",
    )?;
    fixture.git_ok(PHASE_CHECKOUT, ["checkout", "--quiet", "feature/phase-1"])?;
    Ok(())
}

/// HARN-06 existing PR branch, PRCUT-08: a cut whose target PR branch
/// already exists, and is not the recorded result of a previous cut of this
/// source, refuses, so wsx exits 1 with `feature/phase-1-pr` and its
/// reviewer commit `fix(1): review fixup` unchanged. The counterpart control
/// has no PR branch yet, and wsx cuts it.
///
/// `fixed:`, by observation: devflow's cut runs `git branch -f` on the
/// existing branch, replays `feat(1): add b` onto `origin/develop` and exits
/// 0, so `feature/phase-1-pr` ends at the new replay and no ref reaches the
/// reviewer commit. Its counterpart is Parity.
fn existing_pr_branch_with_extra_commits() -> Scenario {
    let replayed = commit(
        b"feat(1): add b\n",
        2,
        b"chore: initial commit",
        &seed_plus(&[B_FILE]),
    );
    let recorded = cut_outcome(
        0,
        &["refs/heads/feature/phase-1"],
        "refs/heads/feature/phase-1-pr",
        replayed.clone(),
        vec![replayed.clone()],
    );
    Scenario::new(
        "existing_pr_branch_with_extra_commits",
        fixed(
            phase_cut(phase_with_existing_pr),
            refusal(),
            recorded,
            "the cut exits 0 and moves feature/phase-1-pr from fix(1): review fixup to a new \
             commit feat(1): add b",
        ),
    )
    .control(
        Role::Counterpart,
        phase_cut(phase_with_b).expect(cut_outcome(
            0,
            &["refs/heads/feature/phase-1"],
            "refs/heads/feature/phase-1-pr",
            replayed.clone(),
            vec![replayed],
        )),
    )
    .harn06(CaseKind::ExistingPrBranch)
}

// ----------------------------------------------- conflict mid-replay

/// Fixture steps: `phase_with_b`, then `feat(1): change a` (fixture commit
/// 3, `src/a.txt` = `mine`); `origin/develop` gains `feat: change a
/// upstream` (commit 4, `src/a.txt` = `theirs`), fetched.
fn phase_with_conflicting_commit(fixture: &mut Fixture) -> Result<(), String> {
    phase_with_b(fixture)?;
    commit_files(fixture, PHASE_CHECKOUT, &[A_MINE], b"feat(1): change a\n")?;
    advance_and_fetch(fixture, &A_THEIRS, b"feat: change a upstream\n")
}

/// Fixture steps: `phase_with_b`; `origin/develop` gains `feat: change a
/// upstream` (fixture commit 3, `src/a.txt` = `theirs`), fetched.
fn phase_b_after_upstream_change(fixture: &mut Fixture) -> Result<(), String> {
    phase_with_b(fixture)?;
    advance_and_fetch(fixture, &A_THEIRS, b"feat: change a upstream\n")
}

/// The interrupted-cut edge, PRCUT-03: the PR branch ref is written only
/// after every commit has replayed, and a conflict refuses the cut, so when
/// the second phase commit conflicts with `origin/develop` wsx exits 1 with
/// no `feature/phase-1-pr` created and refs, commits and files unchanged.
/// The counterpart control is the same branch without the conflicting
/// commit, and wsx cuts it onto the advanced `origin/develop`.
///
/// `fixed:`, by observation: devflow's cut replays `feat(1): add b`, then
/// stops on `unresolved code conflict` in `feat(1): change a` and exits 1,
/// leaving `feature/phase-1-pr` created at the first replay. The source
/// checkout is back on `feature/phase-1` with its files and index unchanged
/// and no operation left in progress. Its counterpart is Parity.
fn conflict_leaves_partial_pr_branch() -> Scenario {
    let replayed = commit(
        b"feat(1): add b\n",
        2,
        b"feat: change a upstream",
        &[SEED_FILES[0], A_THEIRS[0], B_FILE],
    );
    let recorded = cut_outcome(
        1,
        &["refs/heads/feature/phase-1"],
        "refs/heads/feature/phase-1-pr",
        replayed.clone(),
        vec![replayed.clone()],
    );
    Scenario::new(
        "conflict_leaves_partial_pr_branch",
        fixed(
            phase_cut(phase_with_conflicting_commit),
            refusal(),
            recorded,
            "the refused cut leaves a new branch feature/phase-1-pr behind, holding a new commit \
             feat(1): add b",
        ),
    )
    .control(
        Role::Counterpart,
        phase_cut(phase_b_after_upstream_change).expect(cut_outcome(
            0,
            &["refs/heads/feature/phase-1"],
            "refs/heads/feature/phase-1-pr",
            replayed.clone(),
            vec![replayed],
        )),
    )
}

// ------------------------------------------ scoped and unscoped commits

/// `src/e.txt`, the change already landed on `origin/develop`.
const LANDED: [File; 1] = [(b"src/e.txt", b"e\n")];

/// Fixture steps: `feature/phase-1` with `feat(1): scoped` (fixture commit
/// 2, `src/b.txt`), `fix: unscoped code` (commit 3, `src/c.txt`) and
/// `feat(1-02): second scoped` (commit 4, `src/d.txt`), in that order.
fn phase_with_unscoped_commit(fixture: &mut Fixture) -> Result<(), String> {
    phase_worktree(fixture)?;
    commit_files(fixture, PHASE_CHECKOUT, &[B_FILE], b"feat(1): scoped\n")?;
    commit_files(fixture, PHASE_CHECKOUT, &[C_FILE], b"fix: unscoped code\n")?;
    commit_files(
        fixture,
        PHASE_CHECKOUT,
        &[D_FILE],
        b"feat(1-02): second scoped\n",
    )
}

/// Fixture steps: `origin/develop` gains `fix: landed` (fixture commit 2,
/// `src/e.txt`), fetched; `feature/phase-1` with its own `fix: landed`
/// (commit 3) making the identical change, then `feat(1): scoped` (commit
/// 4, `src/b.txt`).
fn phase_with_landed_commit(fixture: &mut Fixture) -> Result<(), String> {
    advance_and_fetch(fixture, &LANDED, b"fix: landed\n")?;
    phase_worktree(fixture)?;
    commit_files(fixture, PHASE_CHECKOUT, &LANDED, b"fix: landed\n")?;
    commit_files(fixture, PHASE_CHECKOUT, &[B_FILE], b"feat(1): scoped\n")
}

/// HARN-06 scoped and unscoped commits, PRCUT-05: every commit the source
/// branch made since its fork that touches a non-personal path must be
/// replayed, so wsx refuses a phase-1 cut that would leave out `fix:
/// unscoped code`, naming it, with refs, commits and files unchanged. The
/// counterpart control is PRCUT-05's passing case: a scoped cut from a
/// branch that also carries work already landed on `origin/develop`, which
/// wsx cuts with `feat(1): scoped` replayed.
///
/// `fixed:`, by observation: devflow's cut replays `feat(1): scoped` and
/// `feat(1-02): second scoped` in their original order, leaves `fix:
/// unscoped code` out with no warning on stdout or stderr, and exits 0. Its
/// counterpart is Parity.
fn scoped_and_unscoped_commits() -> Scenario {
    let replayed = commit(
        b"feat(1): scoped\n",
        4,
        b"fix: landed",
        &[SEED_FILES[0], SEED_FILES[1], LANDED[0], B_FILE],
    );
    let scoped = commit(
        b"feat(1): scoped\n",
        2,
        b"chore: initial commit",
        &seed_plus(&[B_FILE]),
    );
    let second = commit(
        b"feat(1-02): second scoped\n",
        4,
        b"feat(1): scoped",
        &seed_plus(&[B_FILE, D_FILE]),
    );
    let recorded = cut_outcome(
        0,
        &["refs/heads/feature/phase-1"],
        "refs/heads/feature/phase-1-pr",
        second.clone(),
        vec![scoped, second],
    );
    Scenario::new(
        "scoped_and_unscoped_commits",
        fixed(
            phase_cut(phase_with_unscoped_commit),
            refusal(),
            recorded,
            "the cut exits 0 and writes feature/phase-1-pr with two new commits, feat(1): scoped \
             and feat(1-02): second scoped, fix: unscoped code left out",
        ),
    )
    .control(
        Role::Counterpart,
        phase_cut(phase_with_landed_commit).expect(cut_outcome(
            0,
            &["refs/heads/feature/phase-1"],
            "refs/heads/feature/phase-1-pr",
            replayed.clone(),
            vec![replayed],
        )),
    )
    .harn06(CaseKind::ScopedAndUnscoped)
}

// ----------------------------------------------------------- fork layout

/// The fork layout: `develop` comes from the base remote `upstream`, and the
/// push remote `origin` holds only the workspace branch.
fn fork() -> RepoConfig {
    RepoConfig::devflow().with_remotes("upstream", "origin")
}

/// The refs the fork preset holds: no `origin/develop` and no `origin/HEAD`.
const FORK_REFS: [&str; 5] = [
    "refs/heads/develop",
    "refs/heads/workspace/dev",
    "refs/remotes/origin/workspace/dev",
    "refs/remotes/upstream/HEAD",
    "refs/remotes/upstream/develop",
];

/// Fixture steps: `phase_with_b` (fixture commit 2); `upstream/develop`
/// gains `feat: add d` (commit 3, `src/d.txt`), fetched into `work`, so the
/// cut's own fetch changes nothing and local `develop` stays behind it.
fn fork_phase_with_b(fixture: &mut Fixture) -> Result<(), String> {
    phase_with_b(fixture)?;
    fixture.advance_remote("upstream", "develop", |fixture, clone| {
        commit_files(fixture, clone, &DEVELOP_D, b"feat: add d\n")
    })?;
    fixture.git_ok(WORK, ["fetch", "--quiet", "upstream"])?;
    Ok(())
}

/// Fixture steps: `fork_phase_with_b`, then the `upstream` remote removed
/// from `work`, taking its remote-tracking refs with it.
fn fork_phase_without_upstream(fixture: &mut Fixture) -> Result<(), String> {
    fork_phase_with_b(fixture)?;
    fixture.git_ok(WORK, ["remote", "remove", "upstream"])?;
    Ok(())
}

/// A phase-1 cut in the fork layout, built by `steps`.
fn fork_phase_cut(steps: FixtureFn) -> Case {
    Case::new(
        fork(),
        steps,
        Action::Cut {
            checkout: PHASE_CHECKOUT.to_owned(),
            scope: Some("1".to_owned()),
            pr_branch: None,
        },
    )
}

/// The exit code devflow's cut was observed returning when the base remote
/// is gone: `git fetch upstream develop` fails under `set -e`.
const NO_BASE_REMOTE_EXIT: i32 = 128;

/// HARN-06 fork layout, PRCUT-01 and CONF-04: the PR branch is made from the
/// freshly fetched `<base-remote>/<integration>`, and the base remote is
/// configured apart from the push remote, so wsx cuts `feature/phase-1-pr`
/// with `feat(1): add b` replayed onto `upstream/develop`'s tip `feat: add
/// d`, which local `develop` and the push remote `origin` lack.
///
/// The counterpart control removes the `upstream` remote, so the base remote
/// cannot be fetched: a refusal with refs, commits and files unchanged. Its
/// exit code is the oracle's observed one; the phase that enables the Rust
/// side revises it under IF-01.
///
/// Parity, by observation, subject and counterpart: devflow's cut takes its
/// base remote from `BASE_REMOTE`, so it fetches and cuts from `upstream`;
/// without the remote, `git fetch upstream develop` fails before anything is
/// created.
fn fork_layout() -> Scenario {
    let replayed = commit(
        b"feat(1): add b\n",
        2,
        b"feat: add d",
        &seed_plus(&[D_FILE, B_FILE]),
    );
    let mut refs: Vec<RefExpect> = FORK_REFS
        .iter()
        .chain(&["refs/heads/feature/phase-1"])
        .map(|name| RefExpect {
            name: (*name).to_owned(),
            tip: Tip::Unchanged,
        })
        .collect();
    refs.push(RefExpect {
        name: "refs/heads/feature/phase-1-pr".to_owned(),
        tip: Tip::Commit(replayed.clone()),
    });
    Scenario::new(
        "fork_layout",
        fork_phase_cut(fork_phase_with_b).expect(Outcome {
            exit: 0,
            refs: RefsExpect::Exactly(refs),
            new_commits: vec![replayed],
            files: FilesExpect::Unchanged,
        }),
    )
    .control(
        Role::Counterpart,
        fork_phase_cut(fork_phase_without_upstream).expect(Outcome {
            exit: NO_BASE_REMOTE_EXIT,
            ..refusal()
        }),
    )
    .harn06(CaseKind::ForkLayout)
}

// ------------------------------------------- re-cut after a squash merge

/// Fixture steps: `phase_with_b` (fixture commit 2), then `feat(1): add c`
/// (commit 3, `src/c.txt`).
fn phase_with_b_and_c(fixture: &mut Fixture) -> Result<(), String> {
    phase_with_b(fixture)?;
    commit_files(fixture, PHASE_CHECKOUT, &[C_FILE], b"feat(1): add c\n")
}

/// Fixture steps: `phase_with_b_and_c`; `origin/develop` gains `feat: add b
/// (#1)` (fixture commit 4) adding the same `src/b.txt`, as a squash merge of
/// the first PR would, fetched. No `feature/phase-1-pr` exists: the merged
/// PR's branch was deleted.
fn phase_after_squash_merge(fixture: &mut Fixture) -> Result<(), String> {
    phase_with_b_and_c(fixture)?;
    advance_and_fetch(fixture, &[B_FILE], b"feat: add b (#1)\n")
}

/// HARN-06 squash-merged PRs, PRCUT-01 and PRCUT-05: a re-cut after the
/// first commit `feat(1): add b` landed on `develop` as the squash commit
/// `feat: add b (#1)` replays the rest onto the fetched base, and a commit
/// whose change has already landed is not a left-out commit, so wsx cuts
/// `feature/phase-1-pr` at one new commit `feat(1): add c` whose parent is
/// `feat: add b (#1)`. The counterpart control is the same branch before the
/// squash lands, and wsx replays both commits.
///
/// The subject keeps a later commit on the branch on purpose: a re-cut where
/// every commit has landed (RESEARCH assumption A7; devflow ends it with "PR
/// branch is empty", exit 1) has no requirement fixing wsx's outcome, while
/// PRCUT-05 fixes this one.
///
/// Parity, by observation, subject and counterpart: devflow's cut
/// cherry-picks `feat(1): add b` to an empty change and counts it excluded,
/// then replays `feat(1): add c`.
fn recut_after_squash_merge() -> Scenario {
    let feat_c = commit(
        b"feat(1): add c\n",
        3,
        b"feat: add b (#1)",
        &seed_plus(&[B_FILE, C_FILE]),
    );
    let feat_b = commit(
        b"feat(1): add b\n",
        2,
        b"chore: initial commit",
        &seed_plus(&[B_FILE]),
    );
    let feat_c_on_b = commit(
        b"feat(1): add c\n",
        3,
        b"feat(1): add b",
        &seed_plus(&[B_FILE, C_FILE]),
    );
    Scenario::new(
        "recut_after_squash_merge",
        phase_cut(phase_after_squash_merge).expect(cut_outcome(
            0,
            &["refs/heads/feature/phase-1"],
            "refs/heads/feature/phase-1-pr",
            feat_c.clone(),
            vec![feat_c],
        )),
    )
    .control(
        Role::Counterpart,
        phase_cut(phase_with_b_and_c).expect(cut_outcome(
            0,
            &["refs/heads/feature/phase-1"],
            "refs/heads/feature/phase-1-pr",
            feat_c_on_b.clone(),
            vec![feat_b, feat_c_on_b],
        )),
    )
    .harn06(CaseKind::SquashMerged)
}

// --------------------------------------------------------- no-GSD repo

/// A cut with no scope from `feature/topic` to `feature/topic-pr`, in a
/// no-GSD fixture built by `steps`.
fn no_gsd_topic_cut(steps: FixtureFn) -> Case {
    Case::new(
        RepoConfig::no_gsd(),
        steps,
        Action::Cut {
            checkout: TOPIC_CHECKOUT.to_owned(),
            scope: None,
            pr_branch: Some("feature/topic-pr".to_owned()),
        },
    )
}

/// Fixture steps: a worktree at `work/.worktrees/topic` on a new branch
/// `feature/topic` from `develop`, not from the workspace branch, with one
/// commit `feat: add b` (fixture commit 2, `src/b.txt`).
fn topic_from_develop(fixture: &mut Fixture) -> Result<(), String> {
    fixture.git_ok(
        WORK,
        [
            "worktree",
            "add",
            "--quiet",
            "-b",
            "feature/topic",
            ".worktrees/topic",
            "develop",
        ],
    )?;
    commit_files(fixture, TOPIC_CHECKOUT, &[B_FILE], b"feat: add b\n")
}

/// Fixture steps: `topic_from_develop`, then the tracked `src/a.txt` changed
/// in the source checkout and left uncommitted.
fn topic_from_develop_dirty(fixture: &mut Fixture) -> Result<(), String> {
    topic_from_develop(fixture)?;
    fixture.write(TOPIC_CHECKOUT, b"src/a.txt", b"uncommitted\n")
}

/// HARN-06 no-GSD repo, PRCUT-01 and IF-02: the cut reads no GSD, devflow or
/// agent file, so in a repository with none of them wsx cuts
/// `feature/topic-pr` at one new commit `feat: add b` on `origin/develop`.
/// The source branch comes from `develop`, so the subject involves no
/// personal path at all (`no_gsd_custom_personal_path` covers stripping one).
///
/// The counterpart control flips one variable, the source checkout's tree:
/// an uncommitted change to the tracked `src/a.txt` refuses the cut (PRCUT-06),
/// with refs and commits unchanged and the modified file left as it was.
///
/// Parity, by observation, subject and counterpart: devflow's cut reads
/// `.workspace-divergence` with `git show ... 2>/dev/null` and skips it when
/// absent, then replays `feat: add b`; its `diff-index` check refuses the
/// dirty tree before it fetches.
fn no_gsd_repo() -> Scenario {
    let replayed = commit(
        b"feat: add b\n",
        2,
        b"chore: initial commit",
        &seed_plus(&[B_FILE]),
    );
    Scenario::new(
        "no_gsd_repo",
        no_gsd_topic_cut(topic_from_develop).expect(cut_outcome(
            0,
            &["refs/heads/feature/topic"],
            "refs/heads/feature/topic-pr",
            replayed.clone(),
            vec![replayed],
        )),
    )
    .control(
        Role::Counterpart,
        no_gsd_topic_cut(topic_from_develop_dirty).expect(refusal()),
    )
    .harn06(CaseKind::NoGsd)
}

/// Why no case of `no_gsd_custom_personal_path` has an oracle side.
const NO_ORACLE_CUSTOM_PATH: &str = "devflow's cut strips only its own built-in list and the \
     only entries of .workspace-divergence, not a personal path declared in .wsx.conf";

/// A no-GSD topic cut with no oracle side.
fn rust_only_no_gsd_cut(steps: FixtureFn) -> Case {
    no_gsd_topic_cut(steps).oracle(OracleSide::NotImplemented {
        why: NO_ORACLE_CUSTOM_PATH,
    })
}

/// `notes/x.md`, under the no-GSD preset's declared personal path `notes`.
const NOTE_FILE: File = (b"notes/x.md", b"note\n");

/// `scratch/y.md`, a path the no-GSD manifest does not declare.
const SCRATCH_FILE: File = (b"scratch/y.md", b"scratch\n");

/// Fixture steps: `feature/topic` from `workspace/dev` with `feat: add b`
/// (fixture commit 2), then `docs: notes` (commit 3) adding `notes/x.md`.
fn topic_with_declared_note(fixture: &mut Fixture) -> Result<(), String> {
    topic_worktree(fixture)?;
    commit_files(fixture, TOPIC_CHECKOUT, &[B_FILE], b"feat: add b\n")?;
    commit_files(fixture, TOPIC_CHECKOUT, &[NOTE_FILE], b"docs: notes\n")
}

/// Fixture steps: `feature/topic` from `workspace/dev` with `feat: add b`
/// (fixture commit 2), then `feat: add scratch` (commit 3) adding the
/// undeclared `scratch/y.md`.
fn topic_with_undeclared_scratch(fixture: &mut Fixture) -> Result<(), String> {
    topic_worktree(fixture)?;
    commit_files(fixture, TOPIC_CHECKOUT, &[B_FILE], b"feat: add b\n")?;
    commit_files(
        fixture,
        TOPIC_CHECKOUT,
        &[SCRATCH_FILE],
        b"feat: add scratch\n",
    )
}

/// HARN-06 no-GSD repo with a custom personal path, PRCUT-02, CONF-02 and
/// IF-02. Rust-only: devflow's cut has no custom personal path. The personal
/// paths are the manifest's list, so `docs: notes`, which touches only the
/// declared `notes/x.md`, is dropped and wsx cuts `feature/topic-pr` with
/// `feat: add b` only.
///
/// The counterpart control's second commit adds the undeclared
/// `scratch/y.md` instead, and wsx replays both commits.
fn no_gsd_custom_personal_path() -> Scenario {
    let feat_b = commit(
        b"feat: add b\n",
        2,
        b"chore: initial commit",
        &seed_plus(&[B_FILE]),
    );
    let feat_scratch = commit(
        b"feat: add scratch\n",
        3,
        b"feat: add b",
        &seed_plus(&[B_FILE, SCRATCH_FILE]),
    );
    Scenario::new(
        "no_gsd_custom_personal_path",
        rust_only_no_gsd_cut(topic_with_declared_note).expect(cut_outcome(
            0,
            &["refs/heads/feature/topic"],
            "refs/heads/feature/topic-pr",
            feat_b.clone(),
            vec![feat_b.clone()],
        )),
    )
    .control(
        Role::Counterpart,
        rust_only_no_gsd_cut(topic_with_undeclared_scratch).expect(cut_outcome(
            0,
            &["refs/heads/feature/topic"],
            "refs/heads/feature/topic-pr",
            feat_scratch.clone(),
            vec![feat_b, feat_scratch],
        )),
    )
    .harn06(CaseKind::NoGsd)
}

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 9";
    quoted_personal_names: oracle, rust = "Rust side not enabled until phase 9";
    topic_merge_dropped: oracle, rust = "Rust side not enabled until phase 9";
    integration_merged_into_feature: oracle, rust = "Rust side not enabled until phase 9";
    merge_resolution_adds_personal_file: oracle, rust = "Rust side not enabled until phase 9";
    existing_pr_branch_with_extra_commits: oracle, rust = "Rust side not enabled until phase 10";
    conflict_leaves_partial_pr_branch: oracle, rust = "Rust side not enabled until phase 9";
    scoped_and_unscoped_commits: oracle, rust = "Rust side not enabled until phase 10";
    fork_layout: oracle, rust = "Rust side not enabled until phase 9";
    recut_after_squash_merge: oracle, rust = "Rust side not enabled until phase 10";
    no_gsd_repo: oracle, rust = "Rust side not enabled until phase 9";
    no_gsd_custom_personal_path: rust_only, rust = "Rust side not enabled until phase 9";
    merge_resolution_only_personal_file: oracle, rust = "Rust side not enabled until phase 9";
}
