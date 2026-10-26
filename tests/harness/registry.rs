//! Rules over the scenario registry (plans 02-05 and 02-15).
//!
//! Proves that every registered scenario follows the harness's scenario
//! rules, checked across the whole registry at once, and reports by name
//! each scenario whose Rust side is still disabled, with the reason.
//!
//! Isolation: works from the registered scenario definitions; any scenario it
//! runs gets its own `harness-<module>-<name>-` tree under
//! `CARGO_TARGET_TMPDIR`, like every scenario.
