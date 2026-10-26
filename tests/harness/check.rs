//! Divergence-check scenarios (plans 02-07 and 02-13).
//!
//! Each scenario runs the check that the workspace branch differs from the
//! integration branch only in the paths its manifests allow, and states the
//! exit code and the repository state that must result.
//!
//! Isolation: every scenario builds its fixture in its own
//! `harness-check-<name>-` tree under `CARGO_TARGET_TMPDIR`, in the hermetic
//! environment.

use crate::action::Action;
use crate::fixture::{Fixture, ManifestKind, RepoConfig, WORK};
use crate::outcome::{FilesExpect, Outcome, RefsExpect};
use crate::scenario::{Case, FixtureFn, Role, Scenario};

/// Fixture step: the workspace branch gains `notes/todo.md` in a commit of
/// its own, after the preset's workspace commit.
fn add_note(fixture: &mut Fixture) -> Result<(), String> {
    fixture.write(WORK, b"notes/todo.md", b"todo\n")?;
    fixture.commit(WORK, b"chore(ws): add a note\n")
}

/// No fixture steps beyond the preset.
fn no_steps(_: &mut Fixture) -> Result<(), String> {
    Ok(())
}

/// The check in `work`: it reads and writes nothing in the repository.
fn check_in_work(config: RepoConfig, fixture: FixtureFn) -> Case {
    Case::new(
        config,
        fixture,
        Action::Check {
            checkout: WORK.to_owned(),
        },
    )
}

/// The check's verdict with the repository left as it was.
fn verdict(exit: i32) -> Outcome {
    Outcome {
        exit,
        refs: RefsExpect::Unchanged,
        new_commits: Vec::new(),
        files: FilesExpect::Unchanged,
    }
}

/// The oracle's self-test for the divergence check (HARN-01), a guard with
/// both controls (HARN-05). The subject declares `only notes` and carries
/// `notes/todo.md`, so every difference from the integration branch is
/// declared and the check passes. The known-positive control carries the
/// same file without the declaration and must be refused; the clean control
/// is the unmodified preset and must pass.
fn self_test() -> Scenario {
    Scenario::new(
        "self_test",
        check_in_work(
            RepoConfig::devflow().with_manifest_entry(ManifestKind::Only, b"notes"),
            add_note,
        )
        .expect(verdict(0)),
    )
    .control(
        Role::KnownPositive,
        check_in_work(RepoConfig::devflow(), add_note).expect(verdict(1)),
    )
    .control(
        Role::Clean,
        check_in_work(RepoConfig::devflow(), no_steps).expect(verdict(0)),
    )
}

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 5";
}
