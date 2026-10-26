//! Sync: merging the integration branch into the workspace branch.
//!
//! `self_test` is the oracle's self-test (HARN-01): devflow's
//! `sync-workspace.sh` runs from the patched copy against a devflow-shaped
//! fixture and must produce exactly the stated outcome. The HARN-06 cases
//! (plan 02-12) sync after a squash-merged PR, in a fork layout and in a repo
//! with no GSD files, each with a counterpart.

use crate::action::Action;
use crate::fixture::{ACTION_TIME, Fixture, Ident, RepoConfig, T0, TICK, WORK};
use crate::outcome::{
    CommitExpect, FileExpect, FilesExpect, Outcome, RefExpect, RefsExpect, Tip, TreeEntry,
};
use crate::scenario::{Case, CaseKind, FixtureFn, Role, Scenario};

/// A file path and its content.
pub type File = (&'static [u8], &'static [u8]);

/// The files of fixture commit 0 on the integration branch.
pub const SEED_FILES: [File; 2] = [(b"README.md", b"# Fixture\n"), (b"src/a.txt", b"a\n")];

/// The files fixture commit 1 adds on the workspace branch.
pub const WORKSPACE_FILES: [File; 4] = [
    (
        b".planning/ROADMAP.md",
        b"# Roadmap\n\n### Phase 1: Example\n",
    ),
    (b".planning/STATE.md", b"# State\n"),
    (b".workspace-divergence", b"only .wsx.conf\n"),
    (
        b".wsx.conf",
        b"only .planning\nonly .workspace-divergence\n",
    ),
];

/// The file fixture commit 2 adds on the remote integration branch.
pub const B_FILE: File = (b"src/b.txt", b"b\n");

/// Fixture step: `origin/develop` gains commit 2, `feat: add b`.
pub fn advance_develop(fixture: &mut Fixture) -> Result<(), String> {
    fixture.advance_remote("origin", "develop", |fixture, clone| {
        fixture.write(clone, B_FILE.0, B_FILE.1)?;
        fixture.commit(clone, b"feat: add b\n")
    })
}

pub fn tree(files: &[File]) -> Vec<TreeEntry> {
    files
        .iter()
        .map(|(path, content)| TreeEntry::file(path, content))
        .collect()
}

pub fn date(epoch: i64) -> String {
    format!("{epoch} +0000")
}

/// Sync in `work` after `origin/develop` advanced: develop is fetched and
/// fast-forwarded, and the workspace branch gets a two-parent merge commit
/// made by the config identity at the action instant.
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
    Scenario::new(
        "self_test",
        Case::new(
            RepoConfig::devflow(),
            advance_develop,
            Action::Sync {
                checkout: WORK.to_owned(),
            },
        )
        .expect(Outcome {
            exit: 0,
            refs: RefsExpect::Exactly(vec![
                tip("refs/heads/develop", Tip::Commit(feat_b.clone())),
                tip("refs/heads/workspace/dev", Tip::Commit(merge.clone())),
                tip("refs/remotes/origin/HEAD", Tip::Unchanged),
                tip("refs/remotes/origin/develop", Tip::Commit(feat_b.clone())),
                tip("refs/remotes/origin/workspace/dev", Tip::Unchanged),
            ]),
            new_commits: vec![feat_b, merge],
            files: FilesExpect::Exactly(
                all_files
                    .iter()
                    .map(|(path, content)| FileExpect::plain(WORK, path, content))
                    .collect(),
            ),
        }),
    )
}

/// No fixture steps beyond the preset.
fn no_steps(_: &mut Fixture) -> Result<(), String> {
    Ok(())
}

/// Sync in `work` when the workspace branch already contains the integration
/// branch: nothing changes, and the script still exits 0.
fn already_up_to_date() -> Scenario {
    Scenario::new(
        "already_up_to_date",
        Case::new(
            RepoConfig::devflow(),
            no_steps,
            Action::Sync {
                checkout: WORK.to_owned(),
            },
        )
        .expect(Outcome {
            exit: 0,
            refs: RefsExpect::Unchanged,
            new_commits: Vec::new(),
            files: FilesExpect::Unchanged,
        }),
    )
}

// ------------------------------------------------------- HARN-06 cases

/// The files the no-GSD preset's commit 1 adds on the workspace branch.
pub const NO_GSD_FILES: [File; 2] = [
    (b".wsx.conf", b"only notes\n"),
    (b"notes/todo.md", b"todo\n"),
];

/// A commit authored by Ada Author at fixture commit `tick` and committed by
/// the config identity, with one parent (subject `parent`) and `files` as its
/// whole tree.
pub fn fixture_commit(message: &[u8], tick: i64, parent: &[u8], files: &[File]) -> CommitExpect {
    let harness = Ident::harness();
    CommitExpect {
        message: message.to_vec(),
        author: Ident::ada(),
        author_date: date(T0 + tick * TICK),
        committer_name: harness.name,
        committer_email: harness.email,
        parents: vec![parent.to_vec()],
        tree: tree(files),
        signed: false,
    }
}

/// The seed files, `src/b.txt` and `workspace_files`: the tree of a
/// workspace branch after a sync merged `feat: add b` or its squash.
pub fn synced_files(workspace_files: &[File]) -> Vec<File> {
    SEED_FILES
        .into_iter()
        .chain([B_FILE])
        .chain(workspace_files.iter().copied())
        .collect()
}

/// The merge a sync makes on `workspace/dev` (tip subject `ours`) of
/// `theirs`: two parents, made by the config identity at the action instant,
/// with the tree `synced_files(workspace_files)`.
pub fn sync_merge(ours: &[u8], theirs: &CommitExpect, workspace_files: &[File]) -> CommitExpect {
    let harness = Ident::harness();
    CommitExpect {
        message: b"chore: sync develop into workspace/dev\n".to_vec(),
        author: harness.clone(),
        author_date: date(ACTION_TIME),
        committer_name: harness.name.clone(),
        committer_email: harness.email.clone(),
        parents: vec![ours.to_vec(), subject_of(theirs)],
        tree: tree(&synced_files(workspace_files)),
        signed: false,
    }
}

/// The outcome of a sync that fetches `theirs` from `remote`'s `develop`,
/// fast-forwards local `develop` to it and merges it into `workspace/dev`
/// (tip subject `ours`) with `sync_merge`. The work checkout then holds
/// `synced_files(workspace_files)`; the `unchanged` refs keep their tips.
pub fn synced(
    remote: &str,
    theirs: CommitExpect,
    ours: &[u8],
    workspace_files: &[File],
    unchanged: &[&str],
) -> Outcome {
    let all_files = synced_files(workspace_files);
    let merge = sync_merge(ours, &theirs, workspace_files);
    let tip = |name: &str, tip: Tip| RefExpect {
        name: name.to_owned(),
        tip,
    };
    let mut refs = vec![
        tip("refs/heads/develop", Tip::Commit(theirs.clone())),
        tip("refs/heads/workspace/dev", Tip::Commit(merge.clone())),
        tip(&format!("refs/remotes/{remote}/HEAD"), Tip::Unchanged),
        tip(
            &format!("refs/remotes/{remote}/develop"),
            Tip::Commit(theirs.clone()),
        ),
        tip("refs/remotes/origin/workspace/dev", Tip::Unchanged),
    ];
    refs.extend(unchanged.iter().map(|name| tip(name, Tip::Unchanged)));
    Outcome {
        exit: 0,
        refs: RefsExpect::Exactly(refs),
        new_commits: vec![theirs, merge],
        files: FilesExpect::Exactly(
            all_files
                .iter()
                .map(|(path, content)| FileExpect::plain(WORK, path, content))
                .collect(),
        ),
    }
}

/// The first line of a commit's message.
fn subject_of(commit: &CommitExpect) -> Vec<u8> {
    commit
        .message
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default()
        .to_vec()
}

/// An action that changes nothing and exits `exit`.
pub fn no_change(exit: i32) -> Outcome {
    Outcome {
        exit,
        refs: RefsExpect::Unchanged,
        new_commits: Vec::new(),
        files: FilesExpect::Unchanged,
    }
}

/// Fixture steps: `feature/topic` from `workspace/dev` commits `feat: add b`
/// (fixture commit 2, adding `src/b.txt`) and is landed into `workspace/dev`
/// with a `--no-ff` merge `chore: land feature/topic` (fixture commit 3).
pub fn land_topic(fixture: &mut Fixture) -> Result<(), String> {
    fixture.git_ok(WORK, ["checkout", "--quiet", "-b", "feature/topic"])?;
    fixture.write(WORK, B_FILE.0, B_FILE.1)?;
    fixture.commit(WORK, b"feat: add b\n")?;
    fixture.git_ok(WORK, ["checkout", "--quiet", "workspace/dev"])?;
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

/// Fixture steps: `land_topic`, then `origin/develop` gains the PR's squash
/// commit `feat: add b (#1)` (fixture commit 4), adding the same `src/b.txt`.
pub fn land_topic_then_squash(fixture: &mut Fixture) -> Result<(), String> {
    land_topic(fixture)?;
    fixture.advance_remote("origin", "develop", |fixture, clone| {
        fixture.write(clone, B_FILE.0, B_FILE.1)?;
        fixture.commit(clone, b"feat: add b (#1)\n")
    })
}

/// Sync after a PR was squash-merged into `origin/develop`, when the
/// workspace branch already landed the same change from `feature/topic`.
/// The expectation comes from WSYNC-01: `origin/develop` is merged into the
/// workspace branch with a real merge, and both sides add the same
/// `src/b.txt`, so the merge is clean and its tree holds the file once.
///
/// Counterpart: the same fixture before the squash lands. `origin/develop`
/// is then already contained in the workspace branch, and WSYNC-02 makes the
/// sync a no-op with exit 0.
fn after_squash_merged_pr() -> Scenario {
    let squash = fixture_commit(
        b"feat: add b (#1)\n",
        4,
        b"chore: initial commit",
        &[SEED_FILES[0], SEED_FILES[1], B_FILE],
    );
    let sync = |fixture: FixtureFn| {
        Case::new(
            RepoConfig::devflow(),
            fixture,
            Action::Sync {
                checkout: WORK.to_owned(),
            },
        )
    };
    Scenario::new(
        "after_squash_merged_pr",
        sync(land_topic_then_squash).expect(synced(
            "origin",
            squash,
            b"chore: land feature/topic",
            &WORKSPACE_FILES,
            &["refs/heads/feature/topic"],
        )),
    )
    .control(Role::Counterpart, sync(land_topic).expect(no_change(0)))
    .harn06(CaseKind::SquashMerged)
}

/// Fixture step: `upstream/develop` gains `feat: add b` at the next tick.
pub fn advance_upstream(fixture: &mut Fixture) -> Result<(), String> {
    fixture.advance_remote("upstream", "develop", |fixture, clone| {
        fixture.write(clone, B_FILE.0, B_FILE.1)?;
        fixture.commit(clone, b"feat: add b\n")
    })
}

/// Fixture steps: `advance_upstream`, then the `upstream` remote is removed
/// from `work`, taking its remote-tracking refs with it.
fn advance_upstream_then_drop_it(fixture: &mut Fixture) -> Result<(), String> {
    advance_upstream(fixture)?;
    fixture.git_ok(WORK, ["remote", "remove", "upstream"])?;
    Ok(())
}

/// Sync in a fork layout: the integration branch comes from the base remote
/// `upstream`, and the push remote `origin` has no `develop`. The expectation
/// comes from WSYNC-01 (merge `<base-remote>/<integration>`) and CONF-04 (the
/// base remote is configured separately from the push remote): `upstream/develop`
/// is fetched, local `develop` fast-forwarded to it and merged into the
/// workspace branch. The oracle runs `sync-workspace.sh upstream develop`.
///
/// Counterpart: the `upstream` remote removed, so the base remote cannot be
/// fetched. The sync refuses with refs, commits and files unchanged. No
/// requirement fixes the exit code of that refusal, so the expected code is
/// the one the oracle was observed to exit with: git's own 128 from the
/// failed fetch, passed through by `set -e`. The phase that enables the Rust
/// side revises it under IF-01.
fn fork_layout() -> Scenario {
    let feat_b = fixture_commit(
        b"feat: add b\n",
        2,
        b"chore: initial commit",
        &[SEED_FILES[0], SEED_FILES[1], B_FILE],
    );
    let sync = |fixture: FixtureFn| {
        Case::new(
            RepoConfig::devflow().with_remotes("upstream", "origin"),
            fixture,
            Action::Sync {
                checkout: WORK.to_owned(),
            },
        )
    };
    Scenario::new(
        "fork_layout",
        sync(advance_upstream).expect(synced(
            "upstream",
            feat_b,
            b"chore(ws): add workspace files",
            &WORKSPACE_FILES,
            &[],
        )),
    )
    .control(
        Role::Counterpart,
        sync(advance_upstream_then_drop_it)
            .expect(no_change(128))
            .oracle_stderr(b"fatal: 'upstream' does not appear to be a git repository\n"),
    )
    .harn06(CaseKind::ForkLayout)
}

/// Sync in a repo with no GSD, devflow or agent files: the workspace branch
/// adds only `notes/todo.md` and a `.wsx.conf` naming `notes`. The
/// expectation comes from WSYNC-01 and IF-02 (wsx needs none of those files):
/// `origin/develop` is merged as in a devflow repo, and the merge's tree holds
/// the seed files, `src/b.txt`, `notes/todo.md` and `.wsx.conf`, nothing else.
///
/// Counterpart: the same repo already up to date, a no-op with exit 0
/// (WSYNC-02).
fn no_gsd_repo() -> Scenario {
    let feat_b = fixture_commit(
        b"feat: add b\n",
        2,
        b"chore: initial commit",
        &[SEED_FILES[0], SEED_FILES[1], B_FILE],
    );
    let sync = |fixture: FixtureFn| {
        Case::new(
            RepoConfig::no_gsd(),
            fixture,
            Action::Sync {
                checkout: WORK.to_owned(),
            },
        )
    };
    Scenario::new(
        "no_gsd_repo",
        sync(advance_develop).expect(synced(
            "origin",
            feat_b,
            b"chore(ws): add workspace files",
            &NO_GSD_FILES,
            &[],
        )),
    )
    .control(Role::Counterpart, sync(no_steps).expect(no_change(0)))
    .harn06(CaseKind::NoGsd)
}

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 7";
    already_up_to_date: oracle, rust = "Rust side not enabled until phase 7";
    after_squash_merged_pr: oracle, rust = "Rust side not enabled until phase 7";
    fork_layout: oracle, rust = "Rust side not enabled until phase 7";
    no_gsd_repo: oracle, rust = "Rust side not enabled until phase 7";
}
