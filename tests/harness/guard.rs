//! Pre-push guard scenarios (plans 02-07 and 02-10).
//!
//! Each scenario feeds the guard the pushes git would hand a pre-push hook
//! and states whether the push must be refused, with the repository left
//! untouched either way.
//!
//! Isolation: every scenario builds its fixture in its own
//! `harness-guard-<name>-` tree under `CARGO_TARGET_TMPDIR`, in the hermetic
//! environment.

scenarios! {}
