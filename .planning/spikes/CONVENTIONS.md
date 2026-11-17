# Spike Conventions

Patterns established across spike sessions. New spikes follow these unless the question requires otherwise.

## Stack
- Shell (bash) fixtures for git behaviour; a throwaway Rust crate under the spike dir for gix, pinned to wsx's gix
  version and features, built with `CARGO_TARGET_DIR` in the session scratch dir.

## Structure
- `.planning/spikes/NNN-name/`: README.md plus the scripts; each script takes its scratch dir as an argument.

## Patterns
- Isolate git: HOME, XDG_CONFIG_HOME and GIT_CONFIG_GLOBAL in scratch, GIT_CONFIG_NOSYSTEM=1, fixed author and dates,
  GIT_DIR and friends unset.
- Every case compares a reference git command with the reader under test, and has a control that changes the
  reference answer.
- Bound anything that can hang (fifo refs) with `timeout`.

## Tools & Libraries
- Pass a real `node` binary (NODE=...): the mise shim fails once HOME moves.
