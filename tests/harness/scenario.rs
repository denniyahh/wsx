//! Scenarios, the `scenarios!` registry macro, and the runner.
//!
//! A scenario is a case (repo config, fixture steps, an abstract action and an
//! explicit expected outcome) under a name. `scenarios!` turns a module's list
//! of scenario functions into one test module per scenario, holding
//! `oracle()` and `rust()`, plus a `registry()` of every scenario it declares.
//! A disabled Rust side is generated with `#[ignore = "<reason>"]`, so nextest
//! reports it as skipped and a filter selecting only it fails with "no tests to
//! run" instead of passing over zero tests.
//!
//! The runner builds the scenario's scratch tree, environment and fixture,
//! snapshots, runs the side's adapter at the action instant, snapshots again,
//! and compares. A mismatch fails the test with the expected and observed
//! outcome in plain words, and the observed outcome in scenario syntax.

use std::fmt;

use crate::action::Action;
use crate::env::{Env, ScenarioDirs, hermetic_env, inherited_env};
use crate::fixture::{ACTION_TIME, Fixture, RepoConfig, dated};
use crate::oracle::{self, ActionRun, OracleCopy};
use crate::outcome::{Outcome, Snapshot, compare, observe, observed};
use crate::rust;

/// Fixture steps run after the preset is built.
pub type FixtureFn = fn(&mut Fixture) -> Result<(), String>;

/// One fixture, action and expected outcome.
pub struct Case {
    pub config: RepoConfig,
    pub fixture: FixtureFn,
    pub action: Action,
    /// Required: a case without one fails its test.
    pub expected: Option<Outcome>,
}

impl Case {
    pub fn new(config: RepoConfig, fixture: FixtureFn, action: Action) -> Case {
        Case {
            config,
            fixture,
            action,
            expected: None,
        }
    }

    /// States the outcome the action must produce.
    pub fn expect(mut self, outcome: Outcome) -> Case {
        self.expected = Some(outcome);
        self
    }
}

/// A named case.
pub struct Scenario {
    pub name: &'static str,
    pub subject: Case,
}

impl Scenario {
    pub fn new(name: &'static str, subject: Case) -> Scenario {
        Scenario { name, subject }
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

/// One registered scenario, as `scenarios!` records it.
pub struct Entry {
    /// The module that declares it, e.g. `sync`.
    pub module: &'static str,
    pub name: &'static str,
    /// Why the Rust side is disabled, or `None` once it is enabled.
    pub rust_disabled: Option<&'static str>,
    pub build: fn() -> Scenario,
}

/// The snapshots around one action and the action's own run.
pub struct CaseRun {
    pub before: Snapshot,
    pub after: Snapshot,
    pub run: ActionRun,
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
/// and panics with a plain-words report unless the outcome is the expected one.
pub fn run_with_env(side: Side, entry: &Entry, make_env: &dyn Fn(&ScenarioDirs) -> Env) {
    let label = format!("scenario {}::{} ({side} side)", entry.module, entry.name);
    let scenario = (entry.build)();
    assert_eq!(
        scenario.name, entry.name,
        "{label}: its function builds a scenario named {:?}",
        scenario.name
    );
    let case = &scenario.subject;
    let Some(expected) = &case.expected else {
        panic!("{label}: the case states no expected outcome; add .expect(Outcome {{ .. }})");
    };
    let CaseRun { before, after, run } =
        run_case(side, entry, case, make_env).unwrap_or_else(|err| panic!("{label}: {err}"));
    let mismatches = compare(expected, &before, &after, run.exit);
    if !mismatches.is_empty() {
        let listed: Vec<String> = mismatches
            .iter()
            .enumerate()
            .map(|(index, mismatch)| format!("  {}. {mismatch}", index + 1))
            .collect();
        panic!(
            "{label}: {action} did not produce the expected outcome ({count} differences):\n\
             {listed}\n\nstdout of the action:\n{stdout}\nstderr of the action:\n{stderr}\n\
             observed outcome, in scenario syntax:\n{observed:#?}",
            action = case.action,
            count = mismatches.len(),
            listed = listed.join("\n"),
            stdout = String::from_utf8_lossy(&run.stdout),
            stderr = String::from_utf8_lossy(&run.stderr),
            observed = observed(&before, &after, run.exit),
        );
    }
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
