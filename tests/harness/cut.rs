//! PR-cut scenarios (plans 02-07, 02-11 and 02-14).
//!
//! Each scenario cuts a pull-request branch from a workspace branch, leaving
//! out the personal paths, and states the branch, commits and files that
//! must result.
//!
//! Isolation: every scenario builds its fixture in its own
//! `harness-cut-<name>-` tree under `CARGO_TARGET_TMPDIR`, in the hermetic
//! environment.

scenarios! {}
