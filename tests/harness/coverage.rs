//! Coverage of the spawned binary (plan 02-06).
//!
//! Proves that a coverage run counts code that only the spawned `wsx` binary
//! executes, so black-box scenarios contribute to coverage even though no
//! test calls that code in-process.
//!
//! Isolation: the binary runs inside the test's own `harness-coverage-` tree
//! under `CARGO_TARGET_TMPDIR`, with the hermetic environment, whose only
//! inherited keys are the `LLVM_` ones coverage needs.
