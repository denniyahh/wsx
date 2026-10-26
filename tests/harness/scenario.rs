//! Scenarios, the `scenarios!` registry macro, the scenario rules and the
//! runner.
//!
//! A scenario is a subject case (repo config, fixture steps, an abstract
//! action and an explicit expected outcome) under a name, plus control cases.
//! `scenarios!` turns a module's list of scenario functions into one test
//! module per scenario, holding `oracle()` and `rust()`, plus a `registry()` of
//! every scenario it declares. A disabled Rust side is generated with
//! `#[ignore = "<reason>"]`, so nextest reports it as skipped and a filter
//! selecting only it fails with "no tests to run" instead of passing over zero
//! tests.
//!
//! The runner builds each case's scratch tree, environment and fixture,
//! snapshots, runs the side's adapter at the action instant, snapshots again,
//! and judges the run. A failure fails the test with the differences in plain
//! words, and the observed outcome in scenario syntax.

use std::fmt;

use crate::action::Action;
use crate::env::{Env, ScenarioDirs, hermetic_env, inherited_env};
use crate::fixture::{ACTION_TIME, Fixture, RepoConfig, dated};
use crate::oracle::{self, ActionRun, OracleCopy};
use crate::outcome::{
    Field, FilesExpect, Mismatch, Outcome, RefExpect, RefsExpect, Snapshot, Tip, compare, observe,
    observed,
};
use crate::rust;

/// Fixture steps run after the preset is built.
pub type FixtureFn = fn(&mut Fixture) -> Result<(), String>;

/// How a case's expected outcome relates to what the oracle does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Class {
    /// The oracle does what wsx is expected to do.
    Parity,
    /// The oracle has a defect wsx fixes: its run differs from the expected
    /// outcome, and the case records what it does instead.
    Fixed { defect: &'static str },
    /// wsx refuses on purpose: the expected outcome is a refusal.
    Regression { reason: &'static str },
}

/// What the oracle side of a case is judged against.
#[derive(Clone, Debug, PartialEq)]
pub enum OracleSide {
    /// The expected outcome. Legal only beside an explicit `.expect(..)`.
    SameAsExpected,
    /// What the oracle was observed to do, for a `fixed:` or `regression:`
    /// case. It survives the oracle's deletion as the evidence of the defect.
    Recorded(Outcome),
    /// The oracle has no counterpart for this case; only a Rust-only scenario
    /// may say so.
    NotImplemented { why: &'static str },
}

/// One fixture, action and expected outcome.
pub struct Case {
    pub config: RepoConfig,
    pub fixture: FixtureFn,
    pub action: Action,
    /// Required: `validate` rejects a case without one.
    pub expected: Option<Outcome>,
    pub class: Class,
    pub oracle: OracleSide,
}

impl Case {
    pub fn new(config: RepoConfig, fixture: FixtureFn, action: Action) -> Case {
        Case {
            config,
            fixture,
            action,
            expected: None,
            class: Class::Parity,
            oracle: OracleSide::SameAsExpected,
        }
    }

    /// States the outcome the action must produce.
    pub fn expect(mut self, outcome: Outcome) -> Case {
        self.expected = Some(outcome);
        self
    }

    /// States how the expected outcome relates to the oracle's (default
    /// Parity).
    pub fn class(mut self, class: Class) -> Case {
        self.class = class;
        self
    }

    /// States what the oracle side is judged against (default
    /// SameAsExpected).
    pub fn oracle(mut self, oracle: OracleSide) -> Case {
        self.oracle = oracle;
        self
    }
}

/// What a control case is for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Role {
    /// A case a guard must refuse: shows the guard can fail.
    KnownPositive,
    /// A case a guard must let through: shows the guard can pass.
    Clean,
    /// A HARN-06 case's counterpart: the same situation without the property
    /// under test, which must come out differently.
    Counterpart,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::KnownPositive => write!(f, "known-positive"),
            Role::Clean => write!(f, "clean"),
            Role::Counterpart => write!(f, "counterpart"),
        }
    }
}

/// The HARN-06 case kinds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaseKind {
    Merges,
    SquashMerged,
    ForkLayout,
    ScopedAndUnscoped,
    NoGsd,
    PathNames,
    ExistingPrBranch,
}

impl fmt::Display for CaseKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let words = match self {
            CaseKind::Merges => "merge commits",
            CaseKind::SquashMerged => "squash-merged PRs",
            CaseKind::ForkLayout => "fork layout",
            CaseKind::ScopedAndUnscoped => "scoped and unscoped commits in one cut",
            CaseKind::NoGsd => "no-GSD repo",
            CaseKind::PathNames => "non-ASCII, newline and non-UTF-8 path names",
            CaseKind::ExistingPrBranch => "existing PR branch with extra commits",
        };
        write!(f, "{words}")
    }
}

/// A control case and its role.
pub struct Control {
    pub role: Role,
    pub case: Case,
}

/// A named subject case with its controls.
pub struct Scenario {
    pub name: &'static str,
    pub subject: Case,
    pub controls: Vec<Control>,
    /// The HARN-06 case kind the scenario covers, if any.
    pub harn06: Option<CaseKind>,
}

impl Scenario {
    pub fn new(name: &'static str, subject: Case) -> Scenario {
        Scenario {
            name,
            subject,
            controls: Vec::new(),
            harn06: None,
        }
    }

    /// Adds a control case, run after the subject on the same side.
    pub fn control(mut self, role: Role, case: Case) -> Scenario {
        self.controls.push(Control { role, case });
        self
    }

    /// Tags the scenario with the HARN-06 case kind it covers.
    pub fn harn06(mut self, kind: CaseKind) -> Scenario {
        self.harn06 = Some(kind);
        self
    }
}

/// Which implementation runs the action.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    Oracle,
    Rust,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Oracle => write!(f, "oracle"),
            Side::Rust => write!(f, "Rust"),
        }
    }
}

/// How a scenario is registered: with an oracle test, or Rust-only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Oracle,
    RustOnly,
}

/// One registered scenario, as `scenarios!` records it.
pub struct Entry {
    /// The module that declares it, e.g. `sync`.
    pub module: &'static str,
    pub name: &'static str,
    /// Why the Rust side is disabled, or `None` once it is enabled.
    pub rust_disabled: Option<&'static str>,
    pub kind: Kind,
    pub build: fn() -> Scenario,
}

/// Which case of a scenario a rejection concerns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaseRef {
    Subject,
    /// The n-th control, counting from 1.
    Control(usize),
}

impl fmt::Display for CaseRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaseRef::Subject => write!(f, "the subject"),
            CaseRef::Control(n) => write!(f, "control {n}"),
        }
    }
}

/// A rule of success criterion 4 or HARN-05 that a scenario breaks.
#[derive(Clone, Debug, PartialEq)]
pub enum Rejection {
    NoExplicitExpectation(CaseRef),
    FixedWithoutRecordedOracle(CaseRef),
    FixedRecordMatchesExpected(CaseRef),
    RegressionNotARefusal(CaseRef),
    RecordedWithoutDifferenceClass(CaseRef),
    GuardWithoutKnownPositive,
    GuardWithoutCleanControl,
    KnownPositiveNotRefused(CaseRef),
    CleanControlRefused(CaseRef),
    OracleKindWithoutOracleSide { case: CaseRef, why: &'static str },
    RustOnlyWithOracleSide(CaseRef),
    Harn06WithoutControl(CaseKind),
}

/// Whether `action` is a guard, which must be shown both refusing and passing.
pub fn is_guard(action: &Action) -> bool {
    matches!(action, Action::Check { .. } | Action::PrePush { .. })
}

/// Checks `scenario`, registered as `kind`, against the scenario rules.
pub fn validate(kind: Kind, scenario: &Scenario) -> Result<(), Vec<Rejection>> {
    let _ = (kind, scenario);
    Ok(())
}

/// The outcome a case's oracle run is compared with.
pub fn oracle_target(case: &Case) -> &Outcome {
    expected_of(case)
}

/// A validated case's expected outcome.
fn expected_of(case: &Case) -> &Outcome {
    case.expected
        .as_ref()
        .expect("validate rejects a case without an expected outcome before it runs")
}

/// The snapshots around one action and the action's own run.
pub struct CaseRun {
    pub before: Snapshot,
    pub after: Snapshot,
    pub run: ActionRun,
}

/// Why a case's run fails.
#[derive(Debug)]
pub enum CaseFailure {
    /// The run differs from the outcome the side is judged against.
    Mismatches(Vec<Mismatch>),
    /// A `fixed:` case's oracle run meets its expected outcome.
    FixedPassesOnOracle,
}

impl fmt::Display for CaseFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaseFailure::Mismatches(mismatches) => {
                write!(
                    f,
                    "the run differs from the outcome it is judged against ({} differences):",
                    mismatches.len()
                )?;
                for (index, mismatch) in mismatches.iter().enumerate() {
                    write!(f, "\n  {}. {mismatch}", index + 1)?;
                }
                Ok(())
            }
            CaseFailure::FixedPassesOnOracle => write!(
                f,
                "the oracle's run meets the expected outcome, so it does not show this fixed: \
                 case's defect; a fixed: case must fail on the oracle"
            ),
        }
    }
}

/// Judges `case`'s `run` on `side`.
pub fn judge(side: Side, case: &Case, run: &CaseRun) -> Result<(), CaseFailure> {
    let _ = side;
    let mismatches = compare(expected_of(case), &run.before, &run.after, run.run.exit);
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(CaseFailure::Mismatches(mismatches))
    }
}

/// The module that declares a generated scenario module, from the generated
/// module's `module_path!()` (`harness::sync::self_test` gives `sync`).
pub fn declaring_module(path: &'static str) -> &'static str {
    path.rsplit("::").nth(1).unwrap_or(path)
}

/// Declares scenarios. Each `name: oracle, rust = "<reason>";` generates
/// `pub mod name` with `#[test] fn oracle()` and an ignored `#[test] fn rust()`;
/// `name: oracle, rust_enabled;` generates both tests unignored. `name` must
/// also be a `fn name() -> Scenario` in the invoking module. The invocation
/// also defines `registry()`, listing every scenario it declares; `scenarios! {}`
/// declares none.
macro_rules! scenarios {
    (@disabled rust $why:literal) => {
        Some($why)
    };
    (@disabled rust_enabled) => {
        None
    };
    (@rust rust $why:literal) => {
        #[test]
        #[ignore = $why]
        fn rust() {
            crate::scenario::run(crate::scenario::Side::Rust, &entry());
        }
    };
    (@rust rust_enabled) => {
        #[test]
        fn rust() {
            crate::scenario::run(crate::scenario::Side::Rust, &entry());
        }
    };
    ($($name:ident : oracle, $rust:ident $(= $why:literal)? ;)*) => {
        $(
            pub mod $name {
                pub fn entry() -> crate::scenario::Entry {
                    crate::scenario::Entry {
                        module: crate::scenario::declaring_module(module_path!()),
                        name: stringify!($name),
                        rust_disabled: scenarios!(@disabled $rust $($why)?),
                        kind: crate::scenario::Kind::Oracle,
                        build: super::$name,
                    }
                }

                #[test]
                fn oracle() {
                    crate::scenario::run(crate::scenario::Side::Oracle, &entry());
                }

                scenarios!(@rust $rust $($why)?);
            }
        )*

        /// Every scenario this module declares.
        pub fn registry() -> Vec<crate::scenario::Entry> {
            vec![$($name::entry()),*]
        }
    };
}

/// Runs `entry`'s scenario on `side` in the hermetic environment built from
/// the test process's own environment.
pub fn run(side: Side, entry: &Entry) {
    run_with_env(side, entry, &|dirs| hermetic_env(&inherited_env(), dirs));
}

/// Runs `entry`'s scenario on `side` with the environment `make_env` builds,
/// and panics with `try_run_with_env`'s report unless every case passes.
pub fn run_with_env(side: Side, entry: &Entry, make_env: &dyn Fn(&ScenarioDirs) -> Env) {
    if let Err(report) = try_run_with_env(side, entry, make_env) {
        panic!("{report}");
    }
}

/// The report's name for a case: `subject` or `<role> control <n>`.
fn case_name(which: CaseRef, role: Option<Role>) -> String {
    match (which, role) {
        (CaseRef::Control(n), Some(role)) => format!("{role} control {n}"),
        _ => "subject".to_owned(),
    }
}

/// The report's class label: empty for Parity.
fn class_label(class: Class) -> String {
    match class {
        Class::Parity => String::new(),
        Class::Fixed { defect } => format!(" (fixed: {defect})"),
        Class::Regression { reason } => format!(" (regression: {reason})"),
    }
}

/// One failing case's report entry.
fn failure_entry(label: &str, case: &Case, failure: &CaseFailure, run: &CaseRun) -> String {
    match failure {
        CaseFailure::FixedPassesOnOracle => format!("{label}: {failure}"),
        CaseFailure::Mismatches(_) => format!(
            "{label}: {failure}\n\naction: {action}\nstdout of the action:\n{stdout}\n\
             stderr of the action:\n{stderr}\nobserved outcome, in scenario syntax:\n{observed:#?}",
            action = case.action,
            stdout = String::from_utf8_lossy(&run.run.stdout),
            stderr = String::from_utf8_lossy(&run.run.stderr),
            observed = observed(&run.before, &run.after, run.run.exit),
        ),
    }
}

/// Runs `entry`'s scenario on `side` with the environment `make_env` builds,
/// and returns a report with one entry per failing case instead of panicking:
/// `<module>::<name>, <case><class>: <failure>`.
pub fn try_run_with_env(
    side: Side,
    entry: &Entry,
    make_env: &dyn Fn(&ScenarioDirs) -> Env,
) -> Result<(), String> {
    let id = format!("{}::{}", entry.module, entry.name);
    let scenario = (entry.build)();
    if scenario.name != entry.name {
        return Err(format!(
            "{id}: its function builds a scenario named {:?}",
            scenario.name
        ));
    }
    let case = &scenario.subject;
    let label = format!(
        "{id}, {}{}",
        case_name(CaseRef::Subject, None),
        class_label(case.class)
    );
    if case.expected.is_none() {
        return Err(format!(
            "{label}: the case states no expected outcome; add .expect(Outcome {{ .. }})"
        ));
    }
    let run = run_case(side, entry, case, make_env).map_err(|err| format!("{label}: {err}"))?;
    judge(side, case, &run).map_err(|failure| failure_entry(&label, case, &failure, &run))
}

/// Builds `case`'s scratch tree (named after `entry`), environment and fixture,
/// runs `side`'s adapter at the action instant and observes before and after.
/// Does not compare; returns the error a fixture step or the adapter hit.
pub fn run_case(
    side: Side,
    entry: &Entry,
    case: &Case,
    make_env: &dyn Fn(&ScenarioDirs) -> Env,
) -> Result<CaseRun, String> {
    let dirs = ScenarioDirs::new(entry.module, entry.name)?;
    let env = make_env(&dirs);
    let mut fixture = Fixture::new(&dirs, env.clone(), case.config.clone())
        .map_err(|err| format!("building the fixture preset failed: {err}"))?;
    (case.fixture)(&mut fixture).map_err(|err| format!("a fixture step failed: {err}"))?;
    let copy = match side {
        Side::Oracle => Some(OracleCopy::prepare(&dirs, &env)?),
        Side::Rust => None,
    };
    let before = observe(&fixture, None)?;
    let action_env = dated(&env, ACTION_TIME);
    let run = match &copy {
        Some(copy) => oracle::run(&case.action, &fixture, copy, &action_env),
        None => rust::run(&case.action, &fixture, &action_env),
    }
    .map_err(|err| format!("the {side} adapter produced no run: {err}"))?;
    let after = observe(&fixture, Some(&before))?;
    Ok(CaseRun { before, after, run })
}

/// Every registered entry names the module that declares it and the scenario
/// its builder returns, so a later lookup by name finds the right scenario.
/// A module may register no scenario yet (`scenarios! {}`): a scenario
/// function it defines but leaves out of `scenarios!` is unused, which
/// `clippy -D warnings` already rejects.
#[test]
fn registry_entries_match_their_scenarios() {
    let registries = crate::registries();
    let mut registered = 0;
    for (module, entries) in &registries {
        for entry in entries {
            assert_eq!(
                entry.module, *module,
                "scenario {} is registered by module {module}",
                entry.name
            );
            assert_eq!(
                (entry.build)().name,
                entry.name,
                "the builder of {module}::{} returns a scenario of another name",
                entry.name
            );
            registered += 1;
        }
    }
    let all = crate::all_scenarios();
    assert_eq!(
        all.len(),
        registered,
        "all_scenarios() concatenates every registry"
    );
    let self_test = all
        .iter()
        .find(|entry| entry.module == "sync" && entry.name == "self_test")
        .expect("sync::self_test is registered");
    assert_eq!(
        self_test.rust_disabled,
        Some("Rust side not enabled until phase 7")
    );
}

/// A module with no scenarios yet.
mod empty {
    scenarios! {}
}

/// `scenarios! {}` expands, without warnings, to an empty registry.
#[test]
fn an_empty_scenario_list_registers_nothing() {
    assert!(empty::registry().is_empty());
}

// ------------------------------------------------------------ scenario rules
//
// Synthetic scenarios: validation never builds a fixture, so the fixture
// function does nothing and the outcomes are small literals.

/// The defect text of every synthetic and real `fixed:` case below.
const DEFECT: &str = "the oracle leaves the workspace branch behind";

/// Fixture step that does nothing.
fn no_steps(_: &mut Fixture) -> Result<(), String> {
    Ok(())
}

/// Exit 0, nothing changed.
fn passes() -> Outcome {
    Outcome {
        exit: 0,
        refs: RefsExpect::Unchanged,
        new_commits: Vec::new(),
        files: FilesExpect::Unchanged,
    }
}

/// Exit 1, nothing changed: a refusal.
fn refuses() -> Outcome {
    Outcome {
        exit: 1,
        ..passes()
    }
}

fn sync_case() -> Case {
    Case::new(
        RepoConfig::devflow(),
        no_steps,
        Action::Sync {
            checkout: crate::fixture::WORK.to_owned(),
        },
    )
}

fn pre_push_case() -> Case {
    Case::new(
        RepoConfig::devflow(),
        no_steps,
        Action::PrePush {
            checkout: crate::fixture::WORK.to_owned(),
            remote: "origin".to_owned(),
            updates: Vec::new(),
        },
    )
}

/// A guard scenario's subject: a push the guard lets through.
fn guard_subject() -> Case {
    pre_push_case().expect(passes())
}

#[test]
fn rejects_fixed_matching_expected_on_oracle() {
    let case = sync_case()
        .expect(passes())
        .class(Class::Fixed { defect: DEFECT })
        .oracle(OracleSide::Recorded(passes()));
    assert_eq!(
        validate(Kind::Oracle, &Scenario::new("copy", case)),
        Err(vec![Rejection::FixedRecordMatchesExpected(
            CaseRef::Subject
        )])
    );
}

#[test]
fn rejects_scenario_without_explicit_expectation() {
    assert_eq!(
        validate(Kind::Oracle, &Scenario::new("bare", sync_case())),
        Err(vec![Rejection::NoExplicitExpectation(CaseRef::Subject)])
    );
}

#[test]
fn rejects_fixed_without_recorded_oracle_outcome() {
    let case = sync_case()
        .expect(passes())
        .class(Class::Fixed { defect: DEFECT });
    assert_eq!(
        validate(Kind::Oracle, &Scenario::new("unrecorded", case)),
        Err(vec![Rejection::FixedWithoutRecordedOracle(
            CaseRef::Subject
        )])
    );
}

#[test]
fn rejects_regression_that_is_not_a_refusal() {
    let reason = "wsx refuses a sync onto a dirty checkout";
    let proceeds = sync_case()
        .expect(passes())
        .class(Class::Regression { reason });
    assert_eq!(
        validate(Kind::Oracle, &Scenario::new("proceeds", proceeds)),
        Err(vec![Rejection::RegressionNotARefusal(CaseRef::Subject)])
    );
    let refusal = sync_case()
        .expect(refuses())
        .class(Class::Regression { reason });
    assert_eq!(
        validate(Kind::Oracle, &Scenario::new("refusal", refusal)),
        Ok(())
    );
}

#[test]
fn rejects_recorded_oracle_outcome_under_parity() {
    let case = sync_case()
        .expect(passes())
        .oracle(OracleSide::Recorded(refuses()));
    assert_eq!(
        validate(Kind::Oracle, &Scenario::new("recorded", case)),
        Err(vec![Rejection::RecordedWithoutDifferenceClass(
            CaseRef::Subject
        )])
    );
}

#[test]
fn rejects_guard_without_known_positive() {
    let scenario = Scenario::new("guard", guard_subject())
        .control(Role::Clean, pre_push_case().expect(passes()));
    assert_eq!(
        validate(Kind::Oracle, &scenario),
        Err(vec![Rejection::GuardWithoutKnownPositive])
    );
}

#[test]
fn rejects_guard_without_clean_control() {
    let scenario = Scenario::new("guard", guard_subject())
        .control(Role::KnownPositive, pre_push_case().expect(refuses()));
    assert_eq!(
        validate(Kind::Oracle, &scenario),
        Err(vec![Rejection::GuardWithoutCleanControl])
    );
}

#[test]
fn guard_with_no_controls_reports_both_missing_roles() {
    assert_eq!(
        validate(Kind::Oracle, &Scenario::new("guard", guard_subject())),
        Err(vec![
            Rejection::GuardWithoutKnownPositive,
            Rejection::GuardWithoutCleanControl,
        ])
    );
}

#[test]
fn rejects_known_positive_control_that_passes() {
    let scenario = Scenario::new("guard", guard_subject())
        .control(Role::KnownPositive, pre_push_case().expect(passes()))
        .control(Role::Clean, pre_push_case().expect(passes()));
    assert_eq!(
        validate(Kind::Oracle, &scenario),
        Err(vec![Rejection::KnownPositiveNotRefused(CaseRef::Control(
            1
        ))])
    );
}

#[test]
fn rejects_clean_control_that_refuses() {
    let scenario = Scenario::new("guard", guard_subject())
        .control(Role::KnownPositive, pre_push_case().expect(refuses()))
        .control(Role::Clean, pre_push_case().expect(refuses()));
    assert_eq!(
        validate(Kind::Oracle, &scenario),
        Err(vec![Rejection::CleanControlRefused(CaseRef::Control(2))])
    );
}

#[test]
fn rejects_oracle_kind_with_unimplemented_oracle_side() {
    let why = "devflow has no remove command";
    let case = sync_case()
        .expect(passes())
        .oracle(OracleSide::NotImplemented { why });
    assert_eq!(
        validate(Kind::Oracle, &Scenario::new("unimplemented", case)),
        Err(vec![Rejection::OracleKindWithoutOracleSide {
            case: CaseRef::Subject,
            why
        }])
    );
}

#[test]
fn rejects_rust_only_kind_with_an_oracle_side() {
    assert_eq!(
        validate(
            Kind::RustOnly,
            &Scenario::new("claims", sync_case().expect(passes()))
        ),
        Err(vec![Rejection::RustOnlyWithOracleSide(CaseRef::Subject)])
    );
    let unimplemented = sync_case()
        .expect(passes())
        .oracle(OracleSide::NotImplemented {
            why: "devflow has no remove command",
        });
    assert_eq!(
        validate(Kind::RustOnly, &Scenario::new("rust_only", unimplemented)),
        Ok(())
    );
}

#[test]
fn rejects_harn06_case_without_control() {
    let scenario = Scenario::new("merges", sync_case().expect(passes())).harn06(CaseKind::Merges);
    assert_eq!(
        validate(Kind::Oracle, &scenario),
        Err(vec![Rejection::Harn06WithoutControl(CaseKind::Merges)])
    );
}

#[test]
fn accepts_every_harn06_kind_with_a_counterpart() {
    let kinds = [
        CaseKind::Merges,
        CaseKind::SquashMerged,
        CaseKind::ForkLayout,
        CaseKind::ScopedAndUnscoped,
        CaseKind::NoGsd,
        CaseKind::PathNames,
        CaseKind::ExistingPrBranch,
    ];
    for kind in kinds {
        let with_control = Scenario::new("case", sync_case().expect(passes()))
            .harn06(kind)
            .control(Role::Counterpart, sync_case().expect(refuses()));
        assert_eq!(
            validate(Kind::Oracle, &with_control),
            Ok(()),
            "{kind} with a counterpart"
        );
        let without = Scenario::new("case", sync_case().expect(passes())).harn06(kind);
        assert_eq!(
            validate(Kind::Oracle, &without),
            Err(vec![Rejection::Harn06WithoutControl(kind)]),
            "{kind} without a control"
        );
    }
}

#[test]
fn accepts_well_formed_parity_scenario() {
    assert_eq!(
        validate(
            Kind::Oracle,
            &Scenario::new("parity", sync_case().expect(passes()))
        ),
        Ok(())
    );
}

#[test]
fn accepts_guard_with_both_controls_in_either_order() {
    let clean_first = Scenario::new("guard", guard_subject())
        .control(Role::Clean, pre_push_case().expect(passes()))
        .control(Role::KnownPositive, pre_push_case().expect(refuses()));
    assert_eq!(validate(Kind::Oracle, &clean_first), Ok(()));
    let known_positive_first = Scenario::new("guard", guard_subject())
        .control(Role::KnownPositive, pre_push_case().expect(refuses()))
        .control(Role::Clean, pre_push_case().expect(passes()));
    assert_eq!(validate(Kind::Oracle, &known_positive_first), Ok(()));
}

/// An empty fixture snapshot: no refs, commits or files.
fn empty_snapshot() -> Snapshot {
    Snapshot {
        refs: Default::default(),
        commits: Default::default(),
        new_commits: Vec::new(),
        files: Default::default(),
    }
}

#[test]
fn fixed_case_is_judged_against_its_recorded_oracle_outcome() {
    let case = sync_case()
        .expect(passes())
        .class(Class::Fixed { defect: DEFECT })
        .oracle(OracleSide::Recorded(refuses()));
    let (before, after) = (empty_snapshot(), empty_snapshot());
    // The observed run (exit 0, nothing changed) is exactly the expected one.
    assert!(compare(case.expected.as_ref().unwrap(), &before, &after, Some(0)).is_empty());
    assert_eq!(oracle_target(&case), &refuses());
    assert!(!compare(oracle_target(&case), &before, &after, Some(0)).is_empty());
}

// ------------------------------------------------- fixed: cases on the oracle

/// The subject of the registered scenario `module::name`.
fn registered_subject(module: &str, name: &str) -> Case {
    let entry = crate::all_scenarios()
        .into_iter()
        .find(|entry| entry.module == module && entry.name == name)
        .unwrap_or_else(|| panic!("{module}::{name} is registered"));
    (entry.build)().subject
}

/// `sync::self_test` as a `fixed:` case whose record is its own expected
/// outcome with the ref list reversed.
fn fixed_record_reordered() -> Scenario {
    let subject = registered_subject("sync", "self_test");
    let mut record = subject
        .expected
        .clone()
        .expect("self_test states an outcome");
    let RefsExpect::Exactly(refs) = &mut record.refs else {
        panic!("self_test lists its refs");
    };
    refs.reverse();
    Scenario::new(
        "fixed_record_reordered",
        subject
            .class(Class::Fixed { defect: DEFECT })
            .oracle(OracleSide::Recorded(record)),
    )
}

/// `sync::already_up_to_date` as a `fixed:` case expecting exit
/// `expected_exit` with the devflow preset's five refs listed one by one,
/// recording the registered expectation (refs `Unchanged`) with exit
/// `record_exit`.
fn listed_refs_variant(name: &'static str, expected_exit: i32, record_exit: i32) -> Scenario {
    let subject = registered_subject("sync", "already_up_to_date");
    let registered = subject
        .expected
        .clone()
        .expect("already_up_to_date states an outcome");
    let listed = [
        "refs/heads/develop",
        "refs/heads/workspace/dev",
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/develop",
        "refs/remotes/origin/workspace/dev",
    ]
    .map(|name| RefExpect {
        name: name.to_owned(),
        tip: Tip::Unchanged,
    });
    let expected = Outcome {
        exit: expected_exit,
        refs: RefsExpect::Exactly(listed.to_vec()),
        ..registered.clone()
    };
    let record = Outcome {
        exit: record_exit,
        ..registered
    };
    Scenario::new(
        name,
        subject
            .expect(expected)
            .class(Class::Fixed { defect: DEFECT })
            .oracle(OracleSide::Recorded(record)),
    )
}

fn fixed_record_unchanged() -> Scenario {
    listed_refs_variant("fixed_record_unchanged", 0, 0)
}

fn fixed_case_differing() -> Scenario {
    listed_refs_variant("fixed_case_differing", 1, 0)
}

fn fixed_record_drift() -> Scenario {
    listed_refs_variant("fixed_record_drift", 1, 2)
}

/// An oracle-capable entry in module `scenario`.
fn local_entry(name: &'static str, build: fn() -> Scenario) -> Entry {
    Entry {
        module: "scenario",
        name,
        rust_disabled: None,
        kind: Kind::Oracle,
        build,
    }
}

/// The hermetic environment every scenario runs in.
fn hermetic(dirs: &ScenarioDirs) -> Env {
    hermetic_env(&inherited_env(), dirs)
}

/// The one-line report of a `fixed:` case of this module whose oracle run
/// meets its expected outcome.
fn passes_on_oracle_report(name: &str) -> String {
    format!(
        "scenario::{name}, subject (fixed: {DEFECT}): {}",
        CaseFailure::FixedPassesOnOracle
    )
}

#[test]
fn fixed_record_reordering_expected_refs_fails_on_oracle() {
    let entry = local_entry("fixed_record_reordered", fixed_record_reordered);
    assert_eq!(validate(Kind::Oracle, &(entry.build)()), Ok(()));
    assert_eq!(
        try_run_with_env(Side::Oracle, &entry, &hermetic),
        Err(passes_on_oracle_report("fixed_record_reordered"))
    );
}

#[test]
fn fixed_record_using_unchanged_for_listed_refs_fails_on_oracle() {
    let entry = local_entry("fixed_record_unchanged", fixed_record_unchanged);
    assert_eq!(validate(Kind::Oracle, &(entry.build)()), Ok(()));
    assert_eq!(
        try_run_with_env(Side::Oracle, &entry, &hermetic),
        Err(passes_on_oracle_report("fixed_record_unchanged"))
    );
}

#[test]
fn fixed_case_differing_on_oracle_passes() {
    let entry = local_entry("fixed_case_differing", fixed_case_differing);
    assert_eq!(validate(Kind::Oracle, &(entry.build)()), Ok(()));
    assert_eq!(try_run_with_env(Side::Oracle, &entry, &hermetic), Ok(()));
}

#[test]
fn fixed_record_drift_fails_on_oracle() {
    let entry = Entry {
        module: "scenario",
        name: "fixed_record_drift",
        rust_disabled: None,
        kind: Kind::Oracle,
        build: fixed_record_drift,
    };
    let scenario = (entry.build)();
    assert_eq!(validate(Kind::Oracle, &scenario), Ok(()));
    let case = &scenario.subject;
    let run = run_case(Side::Oracle, &entry, case, &hermetic).expect("the oracle runs the sync");
    let Err(CaseFailure::Mismatches(mismatches)) = judge(Side::Oracle, case, &run) else {
        panic!("a fixed: case whose oracle run differs from its record fails with mismatches");
    };
    assert!(
        mismatches
            .iter()
            .any(|m| m.field == Field::Exit && m.expected == "2" && m.got == "0"),
        "the exit code is reported as expected 2, observed 0: {mismatches:#?}"
    );
}
