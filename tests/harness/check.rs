//! Divergence-check scenarios (plans 02-07, 02-13 and 03-02).
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

/// What the oracle prints refusing the undeclared `notes/todo.md` against
/// `origin/develop`.
const UNDECLARED_NOTE: &[u8] = b"diverges from origin/develop in ways .workspace-divergence does \
      not declare:\n  undeclared workspace-only path: notes/todo.md (declare: only notes/todo.md)\n";

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
        check_in_work(RepoConfig::devflow(), add_note)
            .expect(verdict(1))
            .oracle_stderr(UNDECLARED_NOTE),
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

/// What the oracle prints refusing the three quoted names: each C-quoted, so
/// none falls under the declared `.planning`.
const QUOTED_NAMES_REFUSAL: &[u8] = b"undeclared workspace-only path: \".planning/a\\nb.md\" \
      (declare: only \".planning/a\\nb.md\")\n  \
      undeclared workspace-only path: \".planning/\\303\\251.md\" \
      (declare: only \".planning/\\303\\251.md\")\n  \
      undeclared workspace-only path: \".planning/\\377.md\" \
      (declare: only \".planning/\\377.md\")\n";

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
            .oracle(OracleSide::Recorded(verdict(1)))
            .oracle_stderr(QUOTED_NAMES_REFUSAL),
    )
    .control(
        Role::KnownPositive,
        check_in_work(RepoConfig::devflow(), add_note)
            .expect(verdict(1))
            .oracle_stderr(UNDECLARED_NOTE),
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
        check_in_work(fork(), add_note)
            .expect(verdict(1))
            .oracle_stderr(
                b"diverges from upstream/develop in ways .workspace-divergence does not \
                  declare:\n  undeclared workspace-only path: notes/todo.md (declare: only \
                  notes/todo.md)\n",
            ),
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

/// Fixture step: the workspace branch gains an undeclared `scratch/y.md` in
/// a commit of its own, after the preset's workspace commit.
fn add_scratch(fixture: &mut Fixture) -> Result<(), String> {
    fixture.write(WORK, b"scratch/y.md", b"y\n")?;
    fixture.commit(WORK, b"chore(ws): add scratch\n")
}

/// Fixture step: the workspace branch gains `notes/x.md`, under the no-GSD
/// preset's declared `notes`, in a commit of its own.
fn add_custom_note(fixture: &mut Fixture) -> Result<(), String> {
    fixture.write(WORK, b"notes/x.md", b"x\n")?;
    fixture.commit(WORK, b"chore(ws): add a custom note\n")
}

/// What the oracle was observed doing in the no-GSD repository.
const NO_GSD_DEFECT: &str =
    "the oracle exits 1, refusing .wsx.conf and notes/todo.md as undeclared";

/// HARN-06 no-GSD repo for the divergence check, a guard with both controls
/// (HARN-05), and IF-02's negative control for the check: the repository
/// holds no GSD, devflow or agent file, and its workspace commit adds only
/// `notes/todo.md` and `.wsx.conf`, which declares `only notes`. `.wsx.conf`
/// is a built-in personal path and `notes` is declared (DIV-01, CONF-02), and
/// wsx reads no file of devflow's (IF-02), so the check passes, exit 0, and
/// changes nothing.
///
/// `fixed:`, by observation: devflow's check exits 1, listing `.wsx.conf`
/// and `notes/todo.md` as undeclared workspace-only paths, and changes
/// nothing. It reads declarations only from its own manifest file,
/// `.workspace-divergence`, which this repository does not have.
///
/// The known-positive control is the same repository whose workspace branch
/// also adds an undeclared `scratch/y.md` and must be refused (Parity). The
/// oracle refuses this repository whether or not `scratch/y.md` is there, so
/// on the oracle side its exit code alone would not show the planted file
/// being caught; its pinned stderr names `scratch/y.md`, which does. The
/// clean control is the devflow preset and must pass (Parity), so the oracle
/// is seen both refusing and passing in the same run.
fn no_gsd_repo() -> Scenario {
    Scenario::new(
        "no_gsd_repo",
        check_in_work(RepoConfig::no_gsd(), no_steps)
            .expect(verdict(0))
            .class(Class::Fixed {
                defect: NO_GSD_DEFECT,
            })
            .oracle(OracleSide::Recorded(verdict(1)))
            .oracle_stderr(
                b"  undeclared workspace-only path: .wsx.conf (declare: only .wsx.conf)\n  \
                  undeclared workspace-only path: notes/todo.md (declare: only notes/todo.md)\n",
            ),
    )
    .control(
        Role::KnownPositive,
        check_in_work(RepoConfig::no_gsd(), add_scratch)
            .expect(verdict(1))
            .oracle_stderr(
                b"undeclared workspace-only path: scratch/y.md (declare: only scratch/y.md)\n",
            ),
    )
    .control(
        Role::Clean,
        check_in_work(RepoConfig::devflow(), no_steps).expect(verdict(0)),
    )
    .harn06(CaseKind::NoGsd)
}

/// Why no case of `no_gsd_custom_personal_path` has an oracle side.
const NO_ORACLE_CUSTOM_PATH: &str = "devflow's check reads only its own manifest file, \
     .workspace-divergence, and its built-in list, not a custom personal path in .wsx.conf";

/// The check in `work` of the no-GSD repository, with no oracle side.
fn rust_only_no_gsd_check(fixture: FixtureFn) -> Case {
    check_in_work(RepoConfig::no_gsd(), fixture).oracle(OracleSide::NotImplemented {
        why: NO_ORACLE_CUSTOM_PATH,
    })
}

/// HARN-06 no-GSD repo with a custom personal path, a guard with both
/// controls (HARN-05). Rust-only: devflow's check has no custom personal
/// path. The workspace branch adds `notes/x.md` under the declared `notes`,
/// so every added path is declared (DIV-01, CONF-02): exit 0, nothing
/// changes.
///
/// The known-positive control adds an undeclared `scratch/y.md` and must be
/// refused; the clean control is the unmodified no-GSD preset and must pass.
fn no_gsd_custom_personal_path() -> Scenario {
    Scenario::new(
        "no_gsd_custom_personal_path",
        rust_only_no_gsd_check(add_custom_note).expect(verdict(0)),
    )
    .control(
        Role::KnownPositive,
        rust_only_no_gsd_check(add_scratch).expect(verdict(1)),
    )
    .control(
        Role::Clean,
        rust_only_no_gsd_check(no_steps).expect(verdict(0)),
    )
    .harn06(CaseKind::NoGsd)
}

/// Fixture step: `work` is migrated to the reftable ref backend. The step
/// then asks git which backend `work` uses and fails unless the answer is
/// `reftable`, so a migration that silently did nothing cannot leave the
/// scenario running on the files backend.
fn to_reftable(fixture: &mut Fixture) -> Result<(), String> {
    fixture.git_ok(WORK, ["refs", "migrate", "--ref-format=reftable"])?;
    let format = fixture.git_ok(WORK, ["rev-parse", "--show-ref-format"])?;
    if format == b"reftable\n" {
        Ok(())
    } else {
        Err(format!(
            "after git refs migrate, work reports ref format {:?}, not reftable",
            String::from_utf8_lossy(&format)
        ))
    }
}

/// Why wsx refuses a reftable repository on purpose.
const REFTABLE_REASON: &str = "gix cannot read reftable refs; wsx refuses a reftable repository with a message naming reftable";

/// GIT-03 `regression:`, a guard with both controls (HARN-05). The subject
/// is the devflow preset with `work` migrated to reftable. wsx refuses it
/// (exit 1 is a placeholder until IF-01 fixes the exit code in phase 5),
/// and the oracle's record is what the vendored check was observed to do on
/// the same repository.
///
/// The known-positive control is the files-backend preset with an undeclared
/// `notes/todo.md`, which must be refused with its pinned oracle stderr. The
/// clean control is the unmodified files-backend preset, which must pass:
/// the reftable subject's only difference from it is the ref backend.
fn reftable_refused() -> Scenario {
    Scenario::new(
        "reftable_refused",
        check_in_work(RepoConfig::devflow(), to_reftable)
            .expect(verdict(1))
            .class(Class::Regression {
                reason: REFTABLE_REASON,
            })
            .oracle(OracleSide::Recorded(verdict(0))),
    )
    .control(
        Role::KnownPositive,
        check_in_work(RepoConfig::devflow(), add_note)
            .expect(verdict(1))
            .oracle_stderr(UNDECLARED_NOTE),
    )
    .control(
        Role::Clean,
        check_in_work(RepoConfig::devflow(), no_steps).expect(verdict(0)),
    )
}

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 5";
    quoted_personal_names: oracle, rust = "Rust side not enabled until phase 5";
    fork_layout: oracle, rust = "Rust side not enabled until phase 5";
    no_gsd_repo: oracle, rust = "Rust side not enabled until phase 5";
    no_gsd_custom_personal_path: rust_only, rust = "Rust side not enabled until phase 5";
    reftable_refused: oracle, rust = "Rust side not enabled until phase 5";
}
