//! Sync: merging the integration branch into the workspace branch.
//!
//! `self_test` is the oracle's self-test (HARN-01): devflow's
//! `sync-workspace.sh` runs from the patched copy against a devflow-shaped
//! fixture and must produce exactly the stated outcome.

use crate::action::Action;
use crate::fixture::{ACTION_TIME, Fixture, Ident, RepoConfig, T0, TICK, WORK};
use crate::outcome::{
    CommitExpect, FileExpect, FilesExpect, Outcome, RefExpect, RefsExpect, Tip, TreeEntry,
};
use crate::scenario::{Case, Scenario};

/// A file path and its content.
pub type File = (&'static [u8], &'static [u8]);

/// The files of fixture commit 0 on the integration branch.
pub const SEED_FILES: [File; 2] = [(b"README.md", b"# Fixture\n"), (b"src/a.txt", b"a\n")];

/// The files fixture commit 1 adds on the workspace branch.
pub const WORKSPACE_FILES: [File; 4] = [
    (b".planning/ROADMAP.md", b"# Roadmap\n\n### Phase 1: Example\n"),
    (b".planning/STATE.md", b"# State\n"),
    (b".workspace-divergence", b"only .wsx.conf\n"),
    (b".wsx.conf", b"only .planning\nonly .workspace-divergence\n"),
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

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 7";
    already_up_to_date: oracle, rust = "Rust side not enabled until phase 7";
}
