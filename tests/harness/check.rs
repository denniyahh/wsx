//! Divergence-check scenarios (plans 02-07, 02-13, 03-02 and 05-04).
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
    check_in(WORK, config, fixture)
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

/// What wsx prints refusing the undeclared `notes/todo.md`: wsx's pinned
/// reason for the Rust side.
const UNDECLARED_NOTE_WSX: &[u8] =
    b"undeclared workspace-only path: notes/todo.md (declare: only notes/todo.md)\n";

/// What wsx prints refusing the undeclared `scratch/y.md`: wsx's pinned
/// reason for the Rust side.
const UNDECLARED_SCRATCH_WSX: &[u8] =
    b"undeclared workspace-only path: scratch/y.md (declare: only scratch/y.md)\n";

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
            .oracle_stderr(UNDECLARED_NOTE)
            .rust_stderr(UNDECLARED_NOTE_WSX),
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
            .oracle_stderr(UNDECLARED_NOTE)
            .rust_stderr(UNDECLARED_NOTE_WSX),
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
            )
            .rust_stderr(UNDECLARED_NOTE_WSX),
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
            )
            .rust_stderr(UNDECLARED_SCRATCH_WSX),
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
        rust_only_no_gsd_check(add_scratch)
            .expect(verdict(1))
            .rust_stderr(UNDECLARED_SCRATCH_WSX),
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
/// is the devflow preset with `work` migrated to reftable. wsx refuses it:
/// wsx exits 5, the refused class (D-06), and the oracle's record is what the vendored check was observed to do on
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
            .expect(verdict(5))
            .rust_stderr(b"'reftable' storage format")
            .class(Class::Regression {
                reason: REFTABLE_REASON,
            })
            .oracle(OracleSide::Recorded(verdict(0))),
    )
    .control(
        Role::KnownPositive,
        check_in_work(RepoConfig::devflow(), add_note)
            .expect(verdict(1))
            .oracle_stderr(UNDECLARED_NOTE)
            .rust_stderr(UNDECLARED_NOTE_WSX),
    )
    .control(
        Role::Clean,
        check_in_work(RepoConfig::devflow(), no_steps).expect(verdict(0)),
    )
}

/// Fixture step: `work` gains a replace ref that replaces the workspace tip
/// with the integration tip. The step then lists `refs/replace/` and fails
/// unless exactly one ref is there, so a `git replace` that silently did
/// nothing cannot leave the scenario running without one.
fn add_replace_ref(fixture: &mut Fixture) -> Result<(), String> {
    fixture.git_ok(
        WORK,
        [
            "replace",
            "refs/heads/workspace/dev",
            "refs/remotes/origin/develop",
        ],
    )?;
    let listed = fixture.git_ok(
        WORK,
        ["for-each-ref", "--format=%(refname)", "refs/replace/"],
    )?;
    if listed
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .count()
        == 1
    {
        Ok(())
    } else {
        Err(format!(
            "after git replace, work lists {:?} under refs/replace/, not one ref",
            String::from_utf8_lossy(&listed)
        ))
    }
}

/// Why wsx refuses a repository holding a replace ref on purpose.
const REPLACE_REFS_REASON: &str = "wsx reads no object while a replace ref exists (phase 4 D-22)";

/// `regression:`, a guard with both controls (HARN-05). The subject is the
/// devflow preset whose `work` holds a replace ref swapping the workspace
/// tip for the integration tip. wsx reads no object while a replace ref
/// exists, so it refuses the repository: exit 5, the refused class (D-06).
/// The oracle's record is what the vendored check was observed to do on the
/// same repository.
///
/// The known-positive control is the preset with an undeclared
/// `notes/todo.md`, which must be refused; the clean control is the
/// unmodified preset, which must pass: the subject's only difference from it
/// is the replace ref.
fn replace_refs_refused() -> Scenario {
    Scenario::new(
        "replace_refs_refused",
        check_in_work(RepoConfig::devflow(), add_replace_ref)
            .expect(verdict(5))
            .rust_stderr(b"refusing the replace refs")
            .class(Class::Regression {
                reason: REPLACE_REFS_REASON,
            })
            .oracle(OracleSide::Recorded(verdict(0))),
    )
    .control(
        Role::KnownPositive,
        check_in_work(RepoConfig::devflow(), add_note)
            .expect(verdict(1))
            .oracle_stderr(UNDECLARED_NOTE)
            .rust_stderr(UNDECLARED_NOTE_WSX),
    )
    .control(
        Role::Clean,
        check_in_work(RepoConfig::devflow(), no_steps).expect(verdict(0)),
    )
}

/// The check in `checkout`: it reads and writes nothing in the repository.
fn check_in(checkout: &str, config: RepoConfig, fixture: FixtureFn) -> Case {
    Case::new(
        config,
        fixture,
        Action::Check {
            checkout: checkout.to_owned(),
        },
    )
}

/// Fixture step: the workspace branch edits the shared `README.md` in a
/// commit of its own.
fn edit_readme(fixture: &mut Fixture) -> Result<(), String> {
    fixture.write(WORK, b"README.md", b"# Fixture, edited\n")?;
    fixture.commit(WORK, b"chore(ws): edit the readme\n")
}

/// Fixture step: the workspace branch deletes the shared `src/a.txt` in a
/// commit of its own.
fn delete_shared(fixture: &mut Fixture) -> Result<(), String> {
    fixture.remove(WORK, b"src/a.txt")?;
    fixture.commit(WORK, b"chore(ws): delete a shared file\n")
}

/// The devflow preset declaring `modified <path>`.
fn declaring_modified(path: &[u8]) -> RepoConfig {
    RepoConfig::devflow().with_manifest_entry(ManifestKind::Modified, path)
}

/// The known-positive control every scenario below shares: an undeclared
/// `notes/todo.md`, refused on both sides.
fn undeclared_note_in(checkout: &str, fixture: FixtureFn) -> Case {
    check_in(checkout, RepoConfig::devflow(), fixture)
        .expect(verdict(1))
        .oracle_stderr(UNDECLARED_NOTE)
        .rust_stderr(UNDECLARED_NOTE_WSX)
}

/// DIV-01 modified shared file, Parity. The workspace branch edits
/// `README.md`, which no `modified` entry names: exit 1 on both sides. The
/// clean control declares `modified README.md` and passes.
fn undeclared_modified() -> Scenario {
    Scenario::new(
        "undeclared_modified",
        check_in_work(RepoConfig::devflow(), edit_readme)
            .expect(verdict(1))
            .oracle_stderr(b"undeclared change to a develop file: README.md")
            .rust_stderr(
                b"undeclared change to a shared file: README.md (move it to the integration \
                  branch, or declare: modified README.md)\n",
            ),
    )
    .control(Role::KnownPositive, undeclared_note_in(WORK, add_note))
    .control(
        Role::Clean,
        check_in_work(declaring_modified(b"README.md"), edit_readme).expect(verdict(0)),
    )
}

/// DIV-01 deleted shared file, Parity. The workspace branch deletes
/// `src/a.txt`, which no `modified` entry names: exit 1 on both sides. The
/// clean control declares `modified src/a.txt` and passes.
fn undeclared_deleted() -> Scenario {
    Scenario::new(
        "undeclared_deleted",
        check_in_work(RepoConfig::devflow(), delete_shared)
            .expect(verdict(1))
            .oracle_stderr(b"undeclared change to a develop file: src/a.txt")
            .rust_stderr(
                b"undeclared deletion of a shared file: src/a.txt (restore it, or declare: \
                  modified src/a.txt)\n",
            ),
    )
    .control(Role::KnownPositive, undeclared_note_in(WORK, add_note))
    .control(
        Role::Clean,
        check_in_work(declaring_modified(b"src/a.txt"), delete_shared).expect(verdict(0)),
    )
}

/// Fixture step: `origin/develop` gains `shared/x.md`, and `work` fetches
/// it and merges it into the workspace branch, so the file is on both tips
/// and is no difference between them (05-RESEARCH Pitfall 4). The step then
/// asks git for the file at `refs/remotes/origin/develop` and for the
/// workspace tip's parents, and fails unless the file is there and the tip
/// is a merge.
fn integration_carries_shared(fixture: &mut Fixture) -> Result<(), String> {
    fixture.advance_remote("origin", "develop", |fixture, scratch| {
        fixture.write(scratch, b"shared/x.md", b"x\n")?;
        fixture.commit(scratch, b"feat: add a shared file\n")
    })?;
    fixture.git_ok(WORK, ["fetch", "--quiet", "origin"])?;
    fixture.git_tick(
        WORK,
        ["merge", "--quiet", "--no-ff", "--no-edit", "origin/develop"],
    )?;
    fixture
        .git_ok(
            WORK,
            ["cat-file", "-e", "refs/remotes/origin/develop:shared/x.md"],
        )
        .map_err(|err| format!("refs/remotes/origin/develop lacks shared/x.md: {err}"))?;
    let parents = fixture.git_ok(WORK, ["rev-list", "--parents", "-n", "1", "workspace/dev"])?;
    let count = parents
        .split(|byte| byte.is_ascii_whitespace())
        .filter(|word| !word.is_empty())
        .count();
    if count == 3 {
        Ok(())
    } else {
        Err(format!(
            "the workspace tip is not a two-parent merge: rev-list --parents printed {:?}",
            String::from_utf8_lossy(&parents)
        ))
    }
}

/// DIV-02 stale `only` entry, Parity. The integration branch carries
/// `shared/x.md` and the manifests declare `only shared`: exit 1 on both
/// sides. The clean control is the same history without the declaration.
fn stale_only() -> Scenario {
    Scenario::new(
        "stale_only",
        check_in_work(
            RepoConfig::devflow().with_manifest_entry(ManifestKind::Only, b"shared"),
            integration_carries_shared,
        )
        .expect(verdict(1))
        .oracle_stderr(b"stale declaration: 'only shared' but origin/develop carries it")
        .rust_stderr(
            b"stale declaration: only shared, but refs/remotes/origin/develop carries it\n",
        ),
    )
    .control(Role::KnownPositive, undeclared_note_in(WORK, add_note))
    .control(
        Role::Clean,
        check_in_work(RepoConfig::devflow(), integration_carries_shared).expect(verdict(0)),
    )
}

/// DIV-02 stale `modified` entry, Parity. The manifests declare `modified
/// README.md` and the workspace branch leaves it as the integration branch
/// has it: exit 1 on both sides. The clean control edits it.
fn stale_modified() -> Scenario {
    Scenario::new(
        "stale_modified",
        check_in_work(declaring_modified(b"README.md"), no_steps)
            .expect(verdict(1))
            .oracle_stderr(b"stale declaration: 'modified README.md' but it matches origin/develop")
            .rust_stderr(
                b"stale declaration: modified README.md, but it matches \
                  refs/remotes/origin/develop\n",
            ),
    )
    .control(Role::KnownPositive, undeclared_note_in(WORK, add_note))
    .control(
        Role::Clean,
        check_in_work(declaring_modified(b"README.md"), edit_readme).expect(verdict(0)),
    )
}

/// Why wsx refuses a declaration a built-in already covers.
const REDUNDANT_REASON: &str = "wsx reports an entry under a built-in personal path as redundant \
     (DIV-02); the oracle has no redundancy rule";

/// DIV-02 redundant declaration, `regression:`. The manifests declare
/// `only .worktrees/x`, under the built-in worktree directory: wsx exits 1
/// naming the built-in. The oracle's record is what the vendored check was
/// observed to do on the same repository.
fn redundant_builtin() -> Scenario {
    Scenario::new(
        "redundant_builtin",
        check_in_work(
            RepoConfig::devflow().with_manifest_entry(ManifestKind::Only, b".worktrees/x"),
            no_steps,
        )
        .expect(verdict(1))
        .rust_stderr(
            b"redundant declaration: only .worktrees/x is already personal as the built-in \
              .worktrees\n",
        )
        .class(Class::Regression {
            reason: REDUNDANT_REASON,
        })
        .oracle(OracleSide::Recorded(verdict(0))),
    )
    .control(Role::KnownPositive, undeclared_note_in(WORK, add_note))
    .control(
        Role::Clean,
        check_in_work(RepoConfig::devflow(), no_steps).expect(verdict(0)),
    )
}

/// The feature worktree's checkout, under `work`'s built-in worktree
/// directory.
const FEATURE: &str = "work/.worktrees/feat";

/// Fixture step: `work` gains the linked worktree `.worktrees/feat` on a new
/// branch `feat` at the workspace tip. The step then asks the worktree for
/// its branch and fails unless it is `feat`.
fn add_feature_worktree(fixture: &mut Fixture) -> Result<(), String> {
    fixture.git_ok(
        WORK,
        [
            "worktree",
            "add",
            "--quiet",
            ".worktrees/feat",
            "-b",
            "feat",
        ],
    )?;
    let branch = fixture.git_ok(FEATURE, ["rev-parse", "--abbrev-ref", "HEAD"])?;
    if branch == b"feat\n" {
        Ok(())
    } else {
        Err(format!(
            "the feature worktree reports branch {:?}, not feat",
            String::from_utf8_lossy(&branch)
        ))
    }
}

/// Fixture step: the feature worktree, then `feature.txt` committed on
/// `feat`.
fn commit_on_feature(fixture: &mut Fixture) -> Result<(), String> {
    add_feature_worktree(fixture)?;
    fixture.write(FEATURE, b"feature.txt", b"feature\n")?;
    fixture.commit(FEATURE, b"feat: add a feature file\n")
}

/// Fixture step: `notes/todo.md` committed on the workspace branch, then the
/// feature worktree.
fn note_then_feature_worktree(fixture: &mut Fixture) -> Result<(), String> {
    add_note(fixture)?;
    add_feature_worktree(fixture)
}

/// What the oracle was observed doing in the feature worktree.
const FEATURE_WORKTREE_DEFECT: &str = "the oracle judges HEAD, the feature branch, and exits 1 \
     refusing feature.txt as undeclared";

/// D-02 feature worktree, `fixed:`. Run from a linked worktree whose branch
/// `feat` adds `feature.txt`, wsx checks the workspace branch the repo
/// model names, not HEAD, and passes. The oracle judges HEAD and refuses
/// the feature file. The known-positive control commits an undeclared
/// `notes/todo.md` on the workspace branch before the worktree is added and
/// is refused on both sides; the clean control is the worktree with no
/// commit.
fn feature_worktree() -> Scenario {
    Scenario::new(
        "feature_worktree",
        check_in(FEATURE, RepoConfig::devflow(), commit_on_feature)
            .expect(verdict(0))
            .class(Class::Fixed {
                defect: FEATURE_WORKTREE_DEFECT,
            })
            .oracle(OracleSide::Recorded(verdict(1)))
            .oracle_stderr(b"undeclared workspace-only path: feature.txt"),
    )
    .control(
        Role::KnownPositive,
        undeclared_note_in(FEATURE, note_then_feature_worktree),
    )
    .control(
        Role::Clean,
        check_in(FEATURE, RepoConfig::devflow(), add_feature_worktree).expect(verdict(0)),
    )
}

scenarios! {
    self_test: oracle, rust_enabled;
    quoted_personal_names: oracle, rust_enabled;
    fork_layout: oracle, rust_enabled;
    no_gsd_repo: oracle, rust_enabled;
    no_gsd_custom_personal_path: rust_only, rust_enabled;
    reftable_refused: oracle, rust_enabled;
    replace_refs_refused: oracle, rust_enabled;
    undeclared_modified: oracle, rust_enabled;
    undeclared_deleted: oracle, rust_enabled;
    stale_only: oracle, rust_enabled;
    stale_modified: oracle, rust_enabled;
    redundant_builtin: oracle, rust_enabled;
    feature_worktree: oracle, rust_enabled;
}
