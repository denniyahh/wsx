//! Commit-signing scenarios (plan 02-08).
//!
//! Each scenario runs an action that creates commits in a repository
//! configured to sign them, and states which of the resulting commits must
//! carry a signature.
//!
//! Isolation: every scenario builds its fixture, and any signing key it
//! needs, in its own `harness-signing-<name>-` tree under
//! `CARGO_TARGET_TMPDIR`, in the hermetic environment.

scenarios! {}
