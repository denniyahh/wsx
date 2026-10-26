//! Rules over the scenario registry (plans 02-05 and 02-15).
//!
//! Proves that every registered scenario follows the harness's scenario
//! rules, checked across the whole registry at once, and reports by name
//! each scenario whose Rust side is still disabled, with the reason.
//!
//! Isolation: works from the registered scenario definitions; any scenario it
//! runs gets its own `harness-<module>-<name>-` tree under
//! `CARGO_TARGET_TMPDIR`, like every scenario.

use crate::scenario::{Entry, Kind, Scenario};

/// The scenarios whose Rust side is disabled: a header
/// `Rust side not enabled (<N>):` and one line `  <module>::<name> (<reason>)`
/// per such entry, sorted by module, then name.
pub fn disabled_report(entries: &[Entry]) -> String {
    let _ = entries;
    String::new()
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
