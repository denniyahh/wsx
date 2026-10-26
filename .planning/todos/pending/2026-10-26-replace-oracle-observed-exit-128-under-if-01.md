---
created: 2026-10-26T20:30:00.000Z
title: Replace oracle-observed exit 128 under IF-01
area: tests
severity: minor
files:
  - tests/harness/sync.rs
  - tests/harness/cut.rs
  - .planning/phases/02-oracle-harness/02-SECURITY.md
---

## Problem

`sync::fork_layout` and `cut::fork_layout` counterparts expect exit 128, the code devflow's
scripts were observed returning when the base remote is gone. No requirement fixes that refusal
code yet. The owner accepted this as AR-02-02 in 02-SECURITY.md on condition that it is replaced
before the Rust side is enabled.

## Solution

When IF-01 (phase 5) defines refusal exit codes, set both counterparts' expected exit to the
IF-01 code in the phase that enables each Rust side (sync: phase 7, cut: phase 9), and drop the
"oracle-observed" wording from their doc comments.
