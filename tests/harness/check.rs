//! Divergence-check scenarios (plans 02-07 and 02-13).
//!
//! Each scenario runs the check that the workspace branch differs from the
//! integration branch only in the paths its manifests allow, and states the
//! exit code and the repository state that must result.
//!
//! Isolation: every scenario builds its fixture in its own
//! `harness-check-<name>-` tree under `CARGO_TARGET_TMPDIR`, in the hermetic
//! environment.

scenarios! {}
