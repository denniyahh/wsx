//! Phase-worktree scenarios (plans 02-07 and 02-12).
//!
//! Each scenario creates or removes a phase worktree in a fixture repository
//! and states the refs, commits and files that must result; the oracle side
//! runs devflow's phase-worktree script, the Rust side `wsx`.
//!
//! Isolation: every scenario builds its fixture in its own
//! `harness-worktree-<name>-` tree under `CARGO_TARGET_TMPDIR`, in the
//! hermetic environment.

scenarios! {}
