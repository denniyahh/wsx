---
created: 2026-11-19T01:01:00.000Z
title: GSD tdd.review-checkpoint counts only feat/fix commits as GREEN
area: tooling
severity: minor
files: []
---

## Problem

GSD's `check tdd-review-checkpoint 05` returned `block: true` for 05-05 and 05-08, both "missing GREEN".
Both plans prescribe their GREEN commits as `test(05-05): ...` and `docs(05-08)` / `test(05-08)`
(05-05-PLAN.md lines 108 and 145, 05-08-PLAN.md lines 108 and 144), and the executors committed exactly
those subjects after RED commits each verified `RED_EVIDENCE_OK`. The gate matches only the
`feat(NN-MM)` pattern, so a plan whose deliverable is test or doc code always fails it.

## Bypass taken (standing delegation, tooling defect)

Execute-phase 05 continued past the gate; the documented `--force-mvp-gate` escape is not
implemented. RED→GREEN order was checked by hand against `git log` for both plans.

## Fix

Upstream GSD: have the gate read the GREEN commit subject the plan declares, or accept
test/docs types for GREEN when the plan names them.
