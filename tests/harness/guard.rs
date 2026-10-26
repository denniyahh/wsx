//! Pre-push guard scenarios (plans 02-07 and 02-10).
//!
//! Each scenario feeds the guard the pushes git would hand a pre-push hook
//! and states whether the push must be refused, with the repository left
//! untouched either way. Every scenario carries a known-positive control the
//! guard must refuse and a clean control it must pass (HARN-05), run on the
//! same side as its subject, so a push recorded as allowed was allowed by a
//! guard shown alive in the same run.
//!
//! Fixture rules: feature branches are made from `develop` in the integration
//! checkout `work/.worktrees/develop`, never from `workspace/dev` (GUARD-01
//! (a) refuses a pushed tip whose tree carries a personal path, so a branch
//! made from the workspace branch is refused whatever its own commits do).
//! Every push except a push of the workspace branch runs from that checkout,
//! which carries no `.workspace-divergence`, so the hook's divergence-check
//! tail stays out of the policy result. New refs carry the all-zero remote sha
//! through `Action::push_input`.
//!
//! Isolation: every scenario builds its fixture in its own
//! `harness-guard-<name>-` tree under `CARGO_TARGET_TMPDIR`, in the hermetic
//! environment.

use crate::action::{Action, PushUpdate};
use crate::fixture::{Fixture, RepoConfig, WORK};
use crate::outcome::{FilesExpect, Outcome, RefsExpect};
use crate::scenario::{Case, CaseKind, Class, FixtureFn, OracleSide, Role, Scenario};

/// A checkout of the integration branch, beside the workspace checkout.
const DEVELOP_CHECKOUT: &str = "work/.worktrees/develop";

/// A file path and its content, both bytes.
type File = (&'static [u8], &'static [u8]);

/// Fixture step: a checkout of `develop` at `work/.worktrees/develop`.
fn develop_checkout(fixture: &mut Fixture) -> Result<(), String> {
    fixture.git_ok(
        WORK,
        ["worktree", "add", "--quiet", ".worktrees/develop", "develop"],
    )?;
    Ok(())
}

/// Fixture step: in the develop checkout, `branch` made from `develop` with
/// one commit writing `files`; the checkout is left on `develop`.
fn branch_from_develop(fixture: &mut Fixture, branch: &str, files: &[File]) -> Result<(), String> {
    fixture.git_ok(DEVELOP_CHECKOUT, ["checkout", "--quiet", "-b", branch])?;
    for (path, content) in files {
        fixture.write(DEVELOP_CHECKOUT, path, content)?;
    }
    fixture.commit(DEVELOP_CHECKOUT, format!("feat: {branch}\n").as_bytes())?;
    fixture.git_ok(DEVELOP_CHECKOUT, ["checkout", "--quiet", "develop"])?;
    Ok(())
}

/// Fixture steps: the develop checkout and two branches made there from
/// `develop`: `feature/clean`, whose one commit touches only
/// `src/clean.txt`, and `feature/leak`, whose one commit adds
/// `.planning/x.md`.
fn branches_from_develop(fixture: &mut Fixture) -> Result<(), String> {
    develop_checkout(fixture)?;
    branch_from_develop(fixture, "feature/clean", &[(b"src/clean.txt", b"clean\n")])?;
    branch_from_develop(fixture, "feature/leak", &[(b".planning/x.md", b"x\n")])
}

/// The guard run from `checkout` for a push of local `branch` to the same
/// branch on `origin`, in a fixture of `config` built by `steps`.
fn push(config: RepoConfig, steps: FixtureFn, checkout: &str, branch: &str) -> Case {
    let name = format!("refs/heads/{branch}");
    Case::new(
        config,
        steps,
        Action::PrePush {
            checkout: checkout.to_owned(),
            remote: "origin".to_owned(),
            updates: vec![PushUpdate {
                local_ref: name.clone(),
                remote_ref: name,
            }],
        },
    )
}

/// `push` in the devflow layout, after `branches_from_develop`.
fn push_from(checkout: &str, branch: &str) -> Case {
    push(RepoConfig::devflow(), branches_from_develop, checkout, branch)
}

/// The guard's verdict with the repository left as it was.
fn verdict(exit: i32) -> Outcome {
    Outcome {
        exit,
        refs: RefsExpect::Unchanged,
        new_commits: Vec::new(),
        files: FilesExpect::Unchanged,
    }
}

/// The known-positive control every guard scenario carries (HARN-05):
/// `feature/leak`, whose one commit adds `.planning/x.md`, pushed as a new
/// ref from the develop checkout. Both devflow and wsx refuse it (GUARD-01).
fn known_positive() -> Case {
    push_from(DEVELOP_CHECKOUT, "feature/leak").expect(verdict(1))
}

/// The clean control every guard scenario carries (HARN-05):
/// `feature/clean`, whose one commit touches only `src/clean.txt`, pushed as
/// a new ref from the develop checkout. Both devflow and wsx pass it
/// (GUARD-01).
fn clean_control() -> Case {
    push_from(DEVELOP_CHECKOUT, "feature/clean").expect(verdict(0))
}

/// `scenario` with the module's shared known-positive and clean controls.
fn with_controls(scenario: Scenario) -> Scenario {
    scenario
        .control(Role::KnownPositive, known_positive())
        .control(Role::Clean, clean_control())
}

/// A case whose oracle run was recorded with exit `oracle_exit`, the
/// repository untouched, where wsx is expected to exit `expected_exit`.
fn fixed(case: Case, expected_exit: i32, oracle_exit: i32, defect: &'static str) -> Case {
    case.expect(verdict(expected_exit))
        .class(Class::Fixed { defect })
        .oracle(OracleSide::Recorded(verdict(oracle_exit)))
}

/// The oracle's self-test for the pre-push hook (HARN-01), a guard with both
/// controls (HARN-05), each case driven the way git drives the hook (remote
/// name and URL as arguments, ref lines on stdin). The subject pushes
/// `feature/clean` as a new ref from the `develop` checkout: its commits touch
/// only `src/`, so the push passes (GUARD-01). The known-positive control
/// pushes `feature/leak`, whose commit adds `.planning/x.md`, and must be
/// refused. The clean control pushes `workspace/dev` from the workspace
/// checkout, where the remote already holds that commit: the personal-path
/// policy skips a workspace branch, and the divergence check the hook runs in
/// a checkout carrying `.workspace-divergence` passes.
fn self_test() -> Scenario {
    Scenario::new(
        "self_test",
        push_from(DEVELOP_CHECKOUT, "feature/clean").expect(verdict(0)),
    )
    .control(Role::KnownPositive, known_positive())
    .control(
        Role::Clean,
        push_from(WORK, "workspace/dev").expect(verdict(0)),
    )
}

// ------------------------------------------------- quoted personal names

/// The oracle's defect behind every quoted-name scenario.
const QUOTED_NAME_DEFECT: &str = "git log --name-only C-quotes a path holding a newline, a \
     non-ASCII or a non-UTF-8 byte (\".planning/...\"), so the hook's ^\\.planning match misses \
     the quoted name and the push is allowed";

/// Fixture steps: the develop checkout and `feature/names`, whose one commit
/// adds `src/n.txt` and the personal file `personal` (a path under
/// `.planning`, written from raw bytes).
fn names_branch(fixture: &mut Fixture, personal: &'static [u8]) -> Result<(), String> {
    develop_checkout(fixture)?;
    branch_from_develop(
        fixture,
        "feature/names",
        &[(b"src/n.txt", b"n\n"), (personal, b"personal\n")],
    )
}

/// A push of `feature/names` as a new ref from the develop checkout, after
/// `steps` built it: GUARD-02 reads path names NUL-separated and as raw
/// bytes, so wsx refuses it (exit 1) with the repository untouched.
fn quoted_name_case(steps: FixtureFn, oracle_exit: i32) -> Case {
    fixed(
        push(RepoConfig::devflow(), steps, DEVELOP_CHECKOUT, "feature/names"),
        1,
        oracle_exit,
        QUOTED_NAME_DEFECT,
    )
}

/// HARN-06 path names, GUARD-02: a branch adding `.planning/é.md` (the bytes
/// `0xC3 0xA9`) beside `src/n.txt`, pushed as a new ref. wsx refuses it.
/// The oracle was recorded allowing it (exit 0): `git log --name-only` prints
/// `".planning/\303\251.md"`, which its prefix match misses.
fn quoted_non_ascii_name() -> Scenario {
    let steps: FixtureFn = |fixture| names_branch(fixture, b".planning/\xC3\xA9.md");
    with_controls(Scenario::new(
        "quoted_non_ascii_name",
        quoted_name_case(steps, 0),
    ))
    .harn06(CaseKind::PathNames)
}

/// HARN-06 path names, GUARD-02: a branch adding `.planning/a<newline>b.md`
/// beside `src/n.txt`, pushed as a new ref. wsx refuses it. The oracle was
/// recorded allowing it (exit 0): `git log --name-only` prints
/// `".planning/a\nb.md"`, which its prefix match misses.
fn quoted_newline_name() -> Scenario {
    let steps: FixtureFn = |fixture| names_branch(fixture, b".planning/a\nb.md");
    with_controls(Scenario::new(
        "quoted_newline_name",
        quoted_name_case(steps, 0),
    ))
    .harn06(CaseKind::PathNames)
}

/// HARN-06 path names, GUARD-02: a branch adding `.planning/<0xFF>.md` (not
/// UTF-8) beside `src/n.txt`, pushed as a new ref. wsx refuses it. The oracle
/// was recorded allowing it (exit 0): `git log --name-only` prints
/// `".planning/\377.md"`, which its prefix match misses.
fn quoted_non_utf8_name() -> Scenario {
    let steps: FixtureFn = |fixture| names_branch(fixture, b".planning/\xFF.md");
    with_controls(Scenario::new(
        "quoted_non_utf8_name",
        quoted_name_case(steps, 0),
    ))
    .harn06(CaseKind::PathNames)
}

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 6";
    quoted_non_ascii_name: oracle, rust = "Rust side not enabled until phase 6";
    quoted_newline_name: oracle, rust = "Rust side not enabled until phase 6";
    quoted_non_utf8_name: oracle, rust = "Rust side not enabled until phase 6";
}
