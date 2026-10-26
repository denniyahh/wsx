//! Rules over the scenario registry (plans 02-05 and 02-15).
//!
//! Proves that every registered scenario follows the harness's scenario
//! rules, checked across the whole registry at once, and reports by name
//! each scenario whose Rust side is still disabled, with the reason.
//!
//! Isolation: works from the registered scenario definitions; any scenario it
//! runs gets its own `harness-<module>-<name>-` tree under
//! `CARGO_TARGET_TMPDIR`, like every scenario.

use crate::action::Action;
use crate::fixture::{Fixture, RepoConfig, WORK};
use crate::scenario::{Case, CaseKind, Entry, Kind, Scenario, validate};

/// Every HARN-06 case kind, with each action the oracle runs in that case and
/// must therefore have an oracle-recorded scenario for (success criterion 5).
///
/// From the HARN-06 case map (02-RESEARCH "HARN-06 case map", as resolved)
/// and orchestrator decision 3: the oracle records every case wherever it can
/// run it, the fork layout and the no-GSD repo included. What it lacks stays
/// Rust-only and is not listed: `rm` after a squash merge, a custom personal
/// path, a non-numeric scope.
const ORACLE_ACTIONS: [(CaseKind, &[&str]); 7] = [
    (CaseKind::Merges, &["cut", "pre-push"]),
    (CaseKind::SquashMerged, &["sync", "cut"]),
    (
        CaseKind::ForkLayout,
        &["sync", "new worktree", "cut", "check", "pre-push"],
    ),
    (CaseKind::ScopedAndUnscoped, &["cut"]),
    (
        CaseKind::NoGsd,
        &["sync", "new worktree", "cut", "check", "pre-push"],
    ),
    (CaseKind::PathNames, &["cut", "check", "pre-push"]),
    (CaseKind::ExistingPrBranch, &["cut"]),
];

/// The action a subject runs, in the words a gap names it with.
fn action_name(action: &Action) -> &'static str {
    match action {
        Action::Sync { .. } => "sync",
        Action::NewWorktree { .. } => "new worktree",
        Action::Cut { .. } => "cut",
        Action::Check { .. } => "check",
        Action::PrePush { .. } => "pre-push",
        Action::RemoveWorktree { .. } => "remove worktree",
    }
}

/// Each tagged entry's HARN-06 case kind, how it is registered and the action
/// its subject runs.
fn harn06_tags(entries: &[Entry]) -> Vec<(CaseKind, Kind, &'static str)> {
    entries
        .iter()
        .filter_map(|entry| {
            let scenario = (entry.build)();
            Some((
                scenario.harn06?,
                entry.kind,
                action_name(&scenario.subject.action),
            ))
        })
        .collect()
}

/// Each HARN-06 case the registry leaves uncovered, in plain words: a case
/// kind no scenario is tagged with (its actions are then not listed one by
/// one), and `<kind>: <action>` for each action in `ORACLE_ACTIONS` with no
/// oracle-recorded scenario of that kind whose subject runs it. The controls
/// every HARN-06 scenario needs are enforced by `validate`, not here.
pub fn harn06_gaps(entries: &[Entry]) -> Vec<String> {
    let tags = harn06_tags(entries);
    let mut gaps = Vec::new();
    for (case, actions) in ORACLE_ACTIONS {
        if !tags.iter().any(|&(tagged, _, _)| tagged == case) {
            gaps.push(format!("{case}: no scenario is tagged with this case"));
            continue;
        }
        for &action in actions {
            if !tags.contains(&(case, Kind::Oracle, action)) {
                gaps.push(format!("{case}: {action}"));
            }
        }
    }
    gaps
}

/// Every HARN-06 case kind is tagged on a registered scenario, and each
/// action the oracle runs in that case has an oracle-recorded one. Prints,
/// per kind, the tagged scenarios with their action and registration.
///
/// Controls: `harn06_gaps_names_a_missing_case_kind` and
/// `harn06_gaps_names_a_missing_oracle_action` show the same check names a
/// missing kind and a missing action, so an empty list here is not a check
/// that never reports.
#[test]
fn every_harn06_case_has_a_scenario() {
    let entries = crate::all_scenarios();
    for (case, _) in ORACLE_ACTIONS {
        println!("{case}:");
        for entry in &entries {
            let scenario = (entry.build)();
            if scenario.harn06 == Some(case) {
                let registered = match entry.kind {
                    Kind::Oracle => "oracle-recorded",
                    Kind::RustOnly => "Rust-only",
                };
                println!(
                    "  {}::{} ({}, {registered})",
                    entry.module,
                    entry.name,
                    action_name(&scenario.subject.action)
                );
            }
        }
    }
    let gaps = harn06_gaps(&entries);
    assert!(gaps.is_empty(), "HARN-06 gaps:\n{}", gaps.join("\n"));
}

/// The scenarios whose Rust side is disabled: a header
/// `Rust side not enabled (<N>):` and one line `  <module>::<name> (<reason>)`
/// per such entry, sorted by module, then name. A later phase reads it to see
/// which scenarios it still has to enable.
pub fn disabled_report(entries: &[Entry]) -> String {
    let mut disabled: Vec<(&str, &str, &str)> = entries
        .iter()
        .filter_map(|entry| Some((entry.module, entry.name, entry.rust_disabled?)))
        .collect();
    disabled.sort_unstable();
    let mut report = format!("Rust side not enabled ({}):", disabled.len());
    for (module, name, reason) in disabled {
        report.push_str(&format!("\n  {module}::{name} ({reason})"));
    }
    report
}

/// Prints the disabled-side report of every registered scenario. Its output
/// shows in every full run through the `success-output = "final"` override in
/// `.config/nextest.toml`.
#[test]
fn rust_sides_not_enabled() {
    let entries = crate::all_scenarios();
    let report = disabled_report(&entries);
    println!("{report}");
    let disabled: Vec<&Entry> = entries
        .iter()
        .filter(|entry| entry.rust_disabled.is_some())
        .collect();
    let mut lines = report.lines();
    assert_eq!(
        lines.next(),
        Some(format!("Rust side not enabled ({}):", disabled.len()).as_str())
    );
    let listed: Vec<&str> = lines.collect();
    assert_eq!(listed.len(), disabled.len(), "{report}");
    for entry in disabled {
        let line = format!(
            "  {}::{} ({})",
            entry.module,
            entry.name,
            entry.rust_disabled.unwrap_or_default()
        );
        assert!(listed.contains(&line.as_str()), "{line:?} in {report}");
    }
}

/// Every registered scenario builds and passes `validate` under the kind it
/// is registered with.
#[test]
fn every_scenario_is_well_formed() {
    let entries = crate::all_scenarios();
    assert!(!entries.is_empty(), "the registry lists no scenario");
    let mut malformed = Vec::new();
    for entry in &entries {
        if let Err(rejections) = validate(entry.kind, &(entry.build)()) {
            for rejection in rejections {
                malformed.push(format!("{}::{}: {rejection}", entry.module, entry.name));
            }
        }
    }
    println!("validated {} registered scenarios", entries.len());
    assert!(malformed.is_empty(), "{}", malformed.join("\n"));
}

/// A synthetic entry's builder: the report never builds a scenario.
fn never_built() -> Scenario {
    unreachable!("disabled_report reads entries without building them")
}

fn synthetic(
    module: &'static str,
    name: &'static str,
    rust_disabled: Option<&'static str>,
) -> Entry {
    Entry {
        module,
        name,
        rust_disabled,
        kind: Kind::Oracle,
        build: never_built,
    }
}

#[test]
fn report_omits_enabled_rust_sides() {
    let report = disabled_report(&[
        synthetic(
            "sync",
            "still_disabled",
            Some("Rust side not enabled until phase 7"),
        ),
        synthetic("sync", "now_enabled", None),
    ]);
    assert_eq!(
        report,
        "Rust side not enabled (1):\n  sync::still_disabled (Rust side not enabled until phase 7)"
    );
    assert!(!report.contains("now_enabled"), "{report}");
}

// ------------------------------------------------------ HARN-06 completeness
//
// Synthetic entries: `harn06_gaps` builds each entry's scenario only to read
// its HARN-06 tag and its subject's action, so the fixture does nothing.

/// Fixture step that does nothing.
fn no_steps(_: &mut Fixture) -> Result<(), String> {
    Ok(())
}

/// A scenario tagged `case` whose subject runs `action`.
fn tagged_scenario(case: CaseKind, action: Action) -> Scenario {
    Scenario::new(
        "synthetic",
        Case::new(RepoConfig::devflow(), no_steps, action),
    )
    .harn06(case)
}

fn sync_action() -> Action {
    Action::Sync {
        checkout: WORK.to_owned(),
    }
}

fn new_worktree_action() -> Action {
    Action::NewWorktree {
        checkout: WORK.to_owned(),
        branch: "feature/phase-1".to_owned(),
        no_sync: false,
    }
}

fn cut_action() -> Action {
    Action::Cut {
        checkout: WORK.to_owned(),
        scope: None,
        pr_branch: None,
    }
}

fn check_action() -> Action {
    Action::Check {
        checkout: WORK.to_owned(),
    }
}

fn pre_push_action() -> Action {
    Action::PrePush {
        checkout: WORK.to_owned(),
        remote: "origin".to_owned(),
        updates: Vec::new(),
    }
}

/// An oracle entry whose scenario is tagged `CaseKind::$case` and whose
/// subject runs `$action`. Each use defines its own builder, because an
/// entry's builder is a plain `fn` pointer.
macro_rules! tagged {
    ($case:ident, $action:expr) => {{
        fn build() -> Scenario {
            tagged_scenario(CaseKind::$case, $action)
        }
        Entry {
            module: "synthetic",
            name: stringify!($case),
            rust_disabled: None,
            kind: Kind::Oracle,
            build,
        }
    }};
}

/// Oracle entries covering every HARN-06 case kind and, for each, every
/// action the HARN-06 case map has the oracle record.
fn full_coverage() -> Vec<Entry> {
    vec![
        tagged!(Merges, cut_action()),
        tagged!(Merges, pre_push_action()),
        tagged!(SquashMerged, sync_action()),
        tagged!(SquashMerged, cut_action()),
        tagged!(ForkLayout, sync_action()),
        tagged!(ForkLayout, new_worktree_action()),
        tagged!(ForkLayout, cut_action()),
        tagged!(ForkLayout, check_action()),
        tagged!(ForkLayout, pre_push_action()),
        tagged!(ScopedAndUnscoped, cut_action()),
        tagged!(NoGsd, sync_action()),
        tagged!(NoGsd, new_worktree_action()),
        tagged!(NoGsd, cut_action()),
        tagged!(NoGsd, check_action()),
        tagged!(NoGsd, pre_push_action()),
        tagged!(PathNames, cut_action()),
        tagged!(PathNames, check_action()),
        tagged!(PathNames, pre_push_action()),
        tagged!(ExistingPrBranch, cut_action()),
    ]
}

/// Without any fork-layout scenario, the one gap is the fork layout itself,
/// not each of its actions.
#[test]
fn harn06_gaps_names_a_missing_case_kind() {
    let mut entries = full_coverage();
    entries.retain(|entry| (entry.build)().harn06 != Some(CaseKind::ForkLayout));
    assert_eq!(
        harn06_gaps(&entries),
        ["fork layout: no scenario is tagged with this case"]
    );
}

/// With every case kind and every oracle action covered, except that the only
/// no-GSD scenario whose subject is a cut is Rust-only, the one gap is that
/// action: a Rust-only scenario does not record the oracle.
#[test]
fn harn06_gaps_names_a_missing_oracle_action() {
    let mut entries = full_coverage();
    for entry in &mut entries {
        let scenario = (entry.build)();
        if scenario.harn06 == Some(CaseKind::NoGsd)
            && action_name(&scenario.subject.action) == "cut"
        {
            entry.kind = Kind::RustOnly;
        }
    }
    assert_eq!(harn06_gaps(&entries), ["no-GSD repo: cut"]);
}
