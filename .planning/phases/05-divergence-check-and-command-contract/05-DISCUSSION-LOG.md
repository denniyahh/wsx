# Phase 5: Divergence check and command contract - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-18
**Phase:** 05-divergence-check-and-command-contract
**Areas discussed:** none in depth; the owner delegated all four

---

## Area selection

| Option | Description | Selected |
|--------|-------------|----------|
| Exit-code scheme | per-class codes or one per error | |
| JSON contract shape | global or per-command version; errors as JSON; where the schema lives | |
| Staleness report | reflog or FETCH_HEAD; report only or fail; no reflog or no remote | |
| Check target & unconfigured | model's workspace tip or HEAD; behaviour when not configured | |

**User's choice:** "you decide on all of these". Claude decided D-01..D-15 in CONTEXT.md.

## Todos

| Option | Selected |
|--------|----------|
| Exit 128 under IF-01 | |
| Re-verify phases 1-3 | ✓ (then reversed) |
| Dependabot re-check | |
| ForeignOwner quoting | |

**Follow-up:** the todo records a deferral to before the milestone audit, so Claude asked about the
conflict. Options: keep deferred (recommended) / fold at the end of Phase 5 / fold before Phase 5 work.
**User's choice:** Keep deferred.

## Claude's Discretion

All implementation decisions D-01..D-15; argument parser, serializer, validator, byte encoding,
module layout.

## Deferred Ideas

None raised.
