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

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 9";
    quoted_personal_names: oracle, rust = "Rust side not enabled until phase 9";
    topic_merge_dropped: oracle, rust = "Rust side not enabled until phase 9";
    integration_merged_into_feature: oracle, rust = "Rust side not enabled until phase 9";
    merge_resolution_adds_personal_file: oracle, rust = "Rust side not enabled until phase 9";
}
