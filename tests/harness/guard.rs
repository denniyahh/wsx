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
        [
            "worktree",
            "add",
            "--quiet",
            ".worktrees/develop",
            "develop",
        ],
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
    push(
        RepoConfig::devflow(),
        branches_from_develop,
        checkout,
        branch,
    )
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
    push_from(DEVELOP_CHECKOUT, "feature/leak")
        .expect(verdict(1))
        .oracle_stderr(LEAK_REFUSAL)
}

/// What the oracle prints refusing `feature/leak` for `.planning/x.md`.
const LEAK_REFUSAL: &[u8] =
    b"pre-push: REFUSING to push personal/agent artifacts to 'refs/heads/feature/leak':\n  \
      .planning/x.md\n";

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
        push(
            RepoConfig::devflow(),
            steps,
            DEVELOP_CHECKOUT,
            "feature/names",
        ),
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

// ------------------------------------- range, ref-pattern, merge and prefix

/// The oracle's defect behind `unlisted_branch_pattern`.
const UNLISTED_PATTERN_DEFECT: &str = "the hook checks only refs matching its listed branch \
     patterns (develop, main, feature/*, fix/*, chore/*, release/*), so a push to \
     refs/heads/wip/y is not checked";

/// The oracle's defect behind `new_branch_without_own_commits`.
const EMPTY_RANGE_DEFECT: &str = "the hook checks only the commits a new ref adds beyond \
     --remotes=origin and never the pushed tip's tree, so a branch with no commit of its own \
     gives an empty range";

/// The oracle's defect behind `merge_resolution_adds_personal_file`.
const MERGE_RESOLUTION_DEFECT: &str = "git log --name-only lists no file for a merge commit, \
     so a personal file added in a merge's conflict resolution is never seen, and the pushed \
     tip's tree is not checked";

/// The oracle's defect behind `name_prefix_is_not_personal`.
const NAME_PREFIX_DEFECT: &str = "the hook's personal-path regex ^(...|\\.planning|...) has \
     no path boundary, so it refuses .planningX/z.md as if it were under .planning";

/// Fixture steps: the develop checkout and `wip/y`, made from `develop`, whose
/// one commit adds `.planning/y.md`.
fn unlisted_branch(fixture: &mut Fixture) -> Result<(), String> {
    develop_checkout(fixture)?;
    branch_from_develop(fixture, "wip/y", &[(b".planning/y.md", b"y\n")])
}

/// GUARD-01: every updated ref other than the workspace branch is checked, so
/// wsx refuses a push of `.planning/y.md` to `refs/heads/wip/y` (exit 1).
fn unlisted_branch_pattern() -> Scenario {
    with_controls(Scenario::new(
        "unlisted_branch_pattern",
        fixed(
            push(
                RepoConfig::devflow(),
                unlisted_branch,
                DEVELOP_CHECKOUT,
                "wip/y",
            ),
            1,
            0,
            UNLISTED_PATTERN_DEFECT,
        ),
    ))
}

/// Fixture steps: the develop checkout and `feature/noown`, a new branch at
/// the `workspace/dev` tip: no commit of its own, and a tree carrying
/// `.planning`.
fn branch_at_workspace_tip(fixture: &mut Fixture) -> Result<(), String> {
    develop_checkout(fixture)?;
    fixture.git_ok(
        DEVELOP_CHECKOUT,
        ["branch", "feature/noown", "refs/heads/workspace/dev"],
    )?;
    Ok(())
}

/// GUARD-01 (a), the pushed tip's tree carries no personal path: wsx refuses
/// a new `feature/noown` at the workspace tip (exit 1), although the branch
/// has no commit of its own.
fn new_branch_without_own_commits() -> Scenario {
    with_controls(Scenario::new(
        "new_branch_without_own_commits",
        fixed(
            push(
                RepoConfig::devflow(),
                branch_at_workspace_tip,
                DEVELOP_CHECKOUT,
                "feature/noown",
            ),
            1,
            0,
            EMPTY_RANGE_DEFECT,
        ),
    ))
}

/// Fixture steps: `feature/merge` from `develop` commits `feat: change a`
/// (`src/a.txt` = `mine`); `origin/develop` gains a commit setting
/// `src/a.txt` to `theirs` and is fetched; merging `origin/develop` into
/// `feature/merge` conflicts on `src/a.txt`, which is resolved to `merged`,
/// and the merge commit also adds `.planning/STATE.md`. The checkout is left
/// on `develop`.
fn merge_adding_personal_file(fixture: &mut Fixture) -> Result<(), String> {
    develop_checkout(fixture)?;
    fixture.git_ok(
        DEVELOP_CHECKOUT,
        ["checkout", "--quiet", "-b", "feature/merge"],
    )?;
    fixture.write(DEVELOP_CHECKOUT, b"src/a.txt", b"mine\n")?;
    fixture.commit(DEVELOP_CHECKOUT, b"feat: change a\n")?;
    fixture.advance_remote("origin", "develop", |fixture, clone| {
        fixture.write(clone, b"src/a.txt", b"theirs\n")?;
        fixture.commit(clone, b"feat: change a upstream\n")
    })?;
    fixture.git_ok(DEVELOP_CHECKOUT, ["fetch", "--quiet", "origin"])?;
    let merge = fixture.git(
        DEVELOP_CHECKOUT,
        ["merge", "--quiet", "--no-commit", "origin/develop"],
    )?;
    if merge.status.code() != Some(1) {
        return Err(format!(
            "merging origin/develop into feature/merge exited {:?}, not 1 with a conflict",
            merge.status.code()
        ));
    }
    fixture.write(DEVELOP_CHECKOUT, b"src/a.txt", b"merged\n")?;
    fixture.write(DEVELOP_CHECKOUT, b".planning/STATE.md", b"# State\n")?;
    fixture.git_ok(DEVELOP_CHECKOUT, ["add", "--all"])?;
    fixture.git_tick(DEVELOP_CHECKOUT, ["commit", "--quiet", "--no-edit"])?;
    fixture.git_ok(DEVELOP_CHECKOUT, ["checkout", "--quiet", "develop"])?;
    Ok(())
}

/// HARN-06 merges, GUARD-01 (a) and (b): merge commits are diffed against
/// their first parent and the pushed tip's tree is checked, so wsx refuses a
/// new `feature/merge` whose merge commit's conflict resolution adds
/// `.planning/STATE.md` (exit 1).
fn merge_resolution_adds_personal_file() -> Scenario {
    with_controls(Scenario::new(
        "merge_resolution_adds_personal_file",
        fixed(
            push(
                RepoConfig::devflow(),
                merge_adding_personal_file,
                DEVELOP_CHECKOUT,
                "feature/merge",
            ),
            1,
            0,
            MERGE_RESOLUTION_DEFECT,
        ),
    ))
    .harn06(CaseKind::Merges)
}

/// Fixture steps: the develop checkout and `feature/prefix`, made from
/// `develop`, whose one commit adds `.planningX/z.md`.
fn prefix_branch(fixture: &mut Fixture) -> Result<(), String> {
    develop_checkout(fixture)?;
    branch_from_develop(fixture, "feature/prefix", &[(b".planningX/z.md", b"z\n")])
}

/// CONF-01: a personal path matches itself and what is under it, never a mere
/// name prefix, so wsx passes a new `feature/prefix` adding `.planningX/z.md`
/// (exit 0). Its adjacency: the known-positive control adds `.planning/x.md`,
/// one character away, and both devflow and wsx refuse that.
fn name_prefix_is_not_personal() -> Scenario {
    with_controls(Scenario::new(
        "name_prefix_is_not_personal",
        fixed(
            push(
                RepoConfig::devflow(),
                prefix_branch,
                DEVELOP_CHECKOUT,
                "feature/prefix",
            ),
            0,
            1,
            NAME_PREFIX_DEFECT,
        )
        .oracle_stderr(
            b"pre-push: REFUSING to push personal/agent artifacts to 'refs/heads/feature/prefix':\n  \
              .planningX/z.md\n",
        ),
    ))
}

// ------------------------------------------------------ fork and no-GSD repos

/// The oracle's defect behind `fork_layout`'s clean control.
const FORK_TAIL_DEFECT: &str = "the hook's divergence-check tail hard-codes origin/develop, \
     which a fork (develop on upstream) lacks, so a workspace push from a checkout carrying \
     .workspace-divergence fails: base ref 'origin/develop' not found";

/// The oracle's defect behind `no_gsd_repo`'s counterpart.
const FIXED_LIST_DEFECT: &str = "the hook's personal-path list is fixed and agent-specific \
     (it includes .planning), so it refuses .planning/x.md in a repo that never declared it \
     personal";

/// The fork layout: `develop` comes from `upstream`, the workspace branch is
/// pushed to `origin`, and `origin` holds no `develop`.
fn fork() -> RepoConfig {
    RepoConfig::devflow().with_remotes("upstream", "origin")
}

/// Fixture steps: the develop checkout and `feature/pr`, made from `develop`
/// (which tracks `upstream/develop`), whose one commit adds `src/pr.txt`.
fn pr_branch(fixture: &mut Fixture) -> Result<(), String> {
    develop_checkout(fixture)?;
    branch_from_develop(fixture, "feature/pr", &[(b"src/pr.txt", b"pr\n")])
}

/// HARN-06 fork layout, GUARD-01: the range is bounded by the remote ref's old
/// value and `<base-remote>/<integration>`, so a new `feature/pr` adding only
/// `src/pr.txt`, pushed to `origin` from the develop checkout, passes (exit
/// 0). Known-positive: a branch adding `.planning/x.md`, pushed the same way,
/// is refused (exit 1). Clean control: `workspace/dev` pushed from `work`,
/// where `origin` already holds it, passes (exit 0): GUARD-01 skips the
/// workspace branch. The oracle was recorded passing the subject and refusing
/// the known-positive, as wsx does, and refusing the clean control (exit 1):
/// its divergence-check tail runs in `work` against a hard-coded
/// `origin/develop`. Its range bound `--remotes=origin` does not over-scan
/// here, because `origin/workspace/dev` already contains `develop`'s commits.
fn fork_layout() -> Scenario {
    Scenario::new(
        "fork_layout",
        push(fork(), pr_branch, DEVELOP_CHECKOUT, "feature/pr").expect(verdict(0)),
    )
    .control(
        Role::KnownPositive,
        push(
            fork(),
            branches_from_develop,
            DEVELOP_CHECKOUT,
            "feature/leak",
        )
        .expect(verdict(1))
        .oracle_stderr(LEAK_REFUSAL),
    )
    .control(
        Role::Clean,
        fixed(
            push(fork(), develop_checkout, WORK, "workspace/dev"),
            0,
            1,
            FORK_TAIL_DEFECT,
        )
        .oracle_stderr(b"check-workspace-divergence: base ref 'origin/develop' not found.\n"),
    )
    .harn06(CaseKind::ForkLayout)
}

/// HARN-06 no-GSD repo, IF-02: in a repo with no GSD, devflow or agent files
/// (its `.wsx.conf` declares only `notes`), the guard run from `work` for
/// `workspace/dev`, which `origin` already holds, passes (exit 0) with the
/// repository untouched: GUARD-01 skips the workspace branch. Counterpart: in
/// the same repo, `feature/leak` adding `.planning/x.md`, pushed as a new ref
/// from the develop checkout, also passes (exit 0): `.planning` is personal
/// only where it is declared (CONF-02), and this repo does not declare it.
/// The known-positive and clean controls are the module's shared ones, in
/// devflow's layout, showing the hook alive in the same run. The oracle was
/// recorded passing the subject, as wsx does (`work` carries no
/// `.workspace-divergence`, so its tail does not run), and refusing the
/// counterpart (exit 1).
fn no_gsd_repo() -> Scenario {
    with_controls(
        Scenario::new(
            "no_gsd_repo",
            push(
                RepoConfig::no_gsd(),
                develop_checkout,
                WORK,
                "workspace/dev",
            )
            .expect(verdict(0)),
        )
        .control(
            Role::Counterpart,
            fixed(
                push(
                    RepoConfig::no_gsd(),
                    branches_from_develop,
                    DEVELOP_CHECKOUT,
                    "feature/leak",
                ),
                0,
                1,
                FIXED_LIST_DEFECT,
            )
            .oracle_stderr(LEAK_REFUSAL),
        ),
    )
    .harn06(CaseKind::NoGsd)
}

/// Why the custom-personal-path cases have no oracle side.
const HARD_CODED_PATHS: &str = "devflow's pre-push hook hard-codes its own personal paths, so a \
     path declared in .wsx.conf has no oracle counterpart";

/// `case`, which the oracle cannot run, expecting exit `exit`.
fn rust_only(case: Case, exit: i32) -> Case {
    case.expect(verdict(exit))
        .oracle(OracleSide::NotImplemented {
            why: HARD_CODED_PATHS,
        })
}

/// Fixture steps: the develop checkout and `feature/notes`, made from
/// `develop`, whose one commit adds `notes/x.md`.
fn notes_branch(fixture: &mut Fixture) -> Result<(), String> {
    develop_checkout(fixture)?;
    branch_from_develop(fixture, "feature/notes", &[(b"notes/x.md", b"x\n")])
}

/// Fixture steps: the develop checkout and `feature/conf`, made from
/// `develop`, whose one commit adds `.wsx.conf`.
fn wsx_conf_branch(fixture: &mut Fixture) -> Result<(), String> {
    develop_checkout(fixture)?;
    branch_from_develop(fixture, "feature/conf", &[(b".wsx.conf", b"only notes\n")])
}

/// HARN-06 no-GSD repo with a custom personal path, Rust-only: in a repo with
/// no GSD, devflow or agent files, whose `.wsx.conf` declares `notes`, wsx
/// refuses a new `feature/notes` adding `notes/x.md` (exit 1): GUARD-01 reads
/// the manifest's personal paths (CONF-02, IF-02). Known-positive: a branch
/// adding `.wsx.conf`, a built-in personal path (CONF-02), is refused (exit
/// 1). Clean control: `feature/clean`, touching only `src/clean.txt`, passes
/// (exit 0).
fn no_gsd_custom_personal_path() -> Scenario {
    Scenario::new(
        "no_gsd_custom_personal_path",
        rust_only(
            push(
                RepoConfig::no_gsd(),
                notes_branch,
                DEVELOP_CHECKOUT,
                "feature/notes",
            ),
            1,
        ),
    )
    .control(
        Role::KnownPositive,
        rust_only(
            push(
                RepoConfig::no_gsd(),
                wsx_conf_branch,
                DEVELOP_CHECKOUT,
                "feature/conf",
            ),
            1,
        ),
    )
    .control(
        Role::Clean,
        rust_only(
            push(
                RepoConfig::no_gsd(),
                branches_from_develop,
                DEVELOP_CHECKOUT,
                "feature/clean",
            ),
            0,
        ),
    )
    .harn06(CaseKind::NoGsd)
}

scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 6";
    quoted_non_ascii_name: oracle, rust = "Rust side not enabled until phase 6";
    quoted_newline_name: oracle, rust = "Rust side not enabled until phase 6";
    quoted_non_utf8_name: oracle, rust = "Rust side not enabled until phase 6";
    unlisted_branch_pattern: oracle, rust = "Rust side not enabled until phase 6";
    new_branch_without_own_commits: oracle, rust = "Rust side not enabled until phase 6";
    merge_resolution_adds_personal_file: oracle, rust = "Rust side not enabled until phase 6";
    name_prefix_is_not_personal: oracle, rust = "Rust side not enabled until phase 6";
    fork_layout: oracle, rust = "Rust side not enabled until phase 6";
    no_gsd_repo: oracle, rust = "Rust side not enabled until phase 6";
    no_gsd_custom_personal_path: rust_only, rust = "Rust side not enabled until phase 6";
}
