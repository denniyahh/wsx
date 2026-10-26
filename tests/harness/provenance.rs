//! Provenance of the vendored oracle (plan 02-04).
//!
//! Proves that the scripts vendored under `tests/oracle` are the files devflow
//! published at the pinned revision, and that the patch the harness applies to
//! its scratch copy still applies and changes only what it says it changes.
//!
//! Isolation: the checked-in files are only read; anything patched is a
//! scratch copy inside the test's own `harness-provenance-` tree under
//! `CARGO_TARGET_TMPDIR`.
