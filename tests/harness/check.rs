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
use crate::scenario::{Case, CaseKind, Class, FixtureFn, OracleSide, Role, Scenario};

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

/// Personal files named with a non-ASCII, a newline and a non-UTF-8 byte
/// under the declared `.planning`; git C-quotes each of these names in its
/// default path output.
const QUOTED_PERSONAL: [&[u8]; 3] = [
    b".planning/\xC3\xA9.md",
    b".planning/a\nb.md",
    b".planning/\xFF.md",
];

/// Fixture step: the workspace branch gains the three `QUOTED_PERSONAL`
/// files in a commit of its own, after the preset's workspace commit.
fn add_quoted_personal(fixture: &mut Fixture) -> Result<(), String> {
    for path in QUOTED_PERSONAL {
        fixture.write(WORK, path, b"personal\n")?;
    }
    fixture.commit(WORK, b"chore(ws): add personal notes\n")
}

/// What the oracle was observed doing with the quoted names.
const QUOTED_NAMES_DEFECT: &str = "the oracle exits 1, refusing the personal files \
     .planning/\\xC3\\xA9.md, .planning/a\\nb.md and .planning/\\xFF.md as undeclared";

/// HARN-06 path names for the divergence check, a guard with both controls
/// (HARN-05). The workspace branch adds three personal files under
/// `.planning`, which the devflow preset declares `only`, named with the
/// bytes 0xC3 0xA9, a newline and 0xFF. DIV-01 judges paths, whatever bytes
/// their names hold: every added path is under a declared `only` entry, so
/// the check passes, exit 0, and changes nothing.
///
/// `fixed:`, by observation: devflow's check exits 1, listing each of the
/// three files as an undeclared workspace-only path, and changes nothing. It
/// matches its built-in list (which holds `.planning`) against `git diff
/// --name-status` output, where git C-quotes these names
/// (`".planning/\303\251.md"`), so no quoted name falls under `.planning`.
///
/// The known-positive control adds an undeclared `notes/todo.md` and must be
/// refused; the clean control is the unmodified preset and must pass. Both
/// are Parity.
fn quoted_personal_names() -> Scenario {
    Scenario::new(
        "quoted_personal_names",
        check_in_work(RepoConfig::devflow(), add_quoted_personal)
            .expect(verdict(0))
            .class(Class::Fixed {
                defect: QUOTED_NAMES_DEFECT,
            })
            .oracle(OracleSide::Recorded(verdict(1))),
    )
    .control(
        Role::KnownPositive,
        check_in_work(RepoConfig::devflow(), add_note).expect(verdict(1)),
    )
    .control(
        Role::Clean,
        check_in_work(RepoConfig::devflow(), no_steps).expect(verdict(0)),
    )
    .harn06(CaseKind::PathNames)
}

/// The fork layout: the integration branch comes from `upstream`, the
/// workspace branch is pushed to `origin`.
fn fork() -> RepoConfig {
    RepoConfig::devflow().with_remotes("upstream", "origin")
}

/// HARN-06 fork layout for the divergence check, a guard with both controls
/// (HARN-05). The check compares the workspace branch with
/// `upstream/develop`, the base remote's integration branch (DIV-01,
/// CONF-04); the oracle takes that ref as its argument. The subject is the
/// declared preset: every difference is declared, so the check passes, exit
/// 0, and changes nothing.
///
/// The known-positive control adds an undeclared `notes/todo.md` and must be
/// refused; the clean control adds the same file declared `only notes` and
/// must pass.
fn fork_layout() -> Scenario {
    Scenario::new(
        "fork_layout",
        check_in_work(fork(), no_steps).expect(verdict(0)),
    )
    .control(
        Role::KnownPositive,
        check_in_work(fork(), add_note).expect(verdict(1)),
    )
    .control(
        Role::Clean,
        check_in_work(
            fork().with_manifest_entry(ManifestKind::Only, b"notes"),
            add_note,
        )
        .expect(verdict(0)),
    )
    .harn06(CaseKind::ForkLayout)
}

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 5";
    quoted_personal_names: oracle, rust = "Rust side not enabled until phase 5";
    fork_layout: oracle, rust = "Rust side not enabled until phase 5";
}
