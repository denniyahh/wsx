---
created: 2026-11-05T19:00:00.000Z
title: Shell-quote the path in ForeignOwner's printed fix command
area: git
severity: minor
files:
  - src/git/mod.rs
---

## Problem

`Error::ForeignOwner` (src/git/mod.rs, the `Display` arm near line 141) prints
`git config --global --add safe.directory {path}` with `path.display()` unquoted. The path is chosen
by the directory's owner, who in exactly this refusal is another user, so a path containing shell
metacharacters (`$(...)`, spaces, `;`) makes the pasted command run or mis-split. Same defect class as
the Phase 4 third-review finding on 04-11's replacement-ref repair command
(.planning/reviews/04-r3-adversarial/SUMMARY.md, row 3). Found by reading the code; not run.

## Solution

Once Phase 4 plan 04-01 lands `shell_word` in `src/git/quote.rs`, render the path through it (with
the placeholder fallback for unquotable bytes) and add a paste test with a `$(touch marker)` path plus
the old-form control, as 04-11 does. Changes a Phase 3 message, so it is its own phase-scoped change,
not part of Phase 4.
