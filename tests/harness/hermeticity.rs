//! Hermeticity of the scenario environment (plan 02-09).
//!
//! Proves that git configuration, hooks and identity planted in a host
//! environment never reach a scenario: its fixture and its action see only
//! the harness's own configuration.
//!
//! Isolation: the planted host is itself a scratch tree inside the test's own
//! `harness-hermeticity-` tree under `CARGO_TARGET_TMPDIR`; the test process's
//! own environment is never changed.
