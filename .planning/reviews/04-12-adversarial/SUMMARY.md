# 04-12 adversarial review — round 1 (tier A)

Lanes: agy (gemini-3.8-flash-high), codex (gpt-6.1-sol, high), pi (deepseek-flash). All three returned; none dropped. Snapshot at HEAD 31dac49; reviewers changed no tracked file (diff -rq against a fresh archive was empty).

## Confirmed — fold into 04-12

| # | Sev | Lane | Finding | Checked by orchestrator |
|---|-----|------|---------|-------------------------|
| R1 | critical | codex | A loose replace ref that is a filesystem symlink (or sits in a symlinked dir) is followed by git and skipped by gix (`gix-ref-0.69.1/src/store/file/loose/iter.rs:47` drops non-regular files), so neither the new listing nor the old loops see it: wsx reads the original bytes, git the replacement. | source line read |
| R2 | major | codex, pi#5, agy#6 | A `GIT_REPLACE_REF_BASE` without a trailing `/` (e.g. `refs/replace/pre`): git aborts with `BUG: refs.c:1908`, gix applies `refs/replace/pre<A>`; the plan's rule admits it. The slashless base also flips direct refs to `ReplacementRefIgnored` untested. Refuse a base that does not end in `/` outright, with a test, and stop claiming full agreement at plan lines 100/290 until then. | lane repro (two lanes) |
| R3 | major | codex | `build` calls `ref_commit` (model/mod.rs:485) before `root_entry` runs the guard (refs.rs:217), so a replacement on the workspace commit (e.g. `<commit> -> blob` plus nested link) fails peeling and is blamed on `.wsx.conf` (`ManifestUnreadable`), against D-22. Guard must run before the first object read. | source read |
| R4 | major | pi#1 | Task 1 GREEN commits a known CONF-07 violation (suffixed `<A>.bak` passes until Task 2) and orders a doc comment that contradicts its own key function. Ship the full git key rule in the first GREEN commit. | plan text |
| R5 | major | pi#2, agy#4 | The five "pin" tests in Task 2 have no failing direction: the length check fires before the duplicate/chain loops, so deleting those loops leaves them green, and cargo-mutants cannot produce "remove nested refusal". Each needs a demonstrated red (scratch-copy negative control asserting the failure text), or drop the claim that they pin the chain/duplicate logic. | plan text + refs.rs:354-375 |
| R6 | major | agy#3, agy#5 | When git itself dies (`fatal: duplicate replace ref`, exit 128) on a nested/suffixed ref that duplicates a direct one, wsx's `ReplacementRefIgnored` message claims git reads it as a replacement — false. Message wording must hold for every shape it fires on (fail-closed is right, the claim is not). | lane repro |
| R7 | major | pi#3 | Plan's gix rule "an id of the repository's length" is wrong: `gix-hash` `from_hex` accepts 40 or 64 digits regardless of repo kind. Over-refusal covers it, but the doc comment must not state it, and a SHA-256 repo test is missing. | lane source cite |
| R8 | major | pi#4 | STATE.md's phase-5 obligation (line ~233) names three replacement variants; the new fourth must be added (bookkeeping step in the plan). | lane cite |
| R9 | major | codex#4 | Task 3 mutation checks compare counts, not mutant identity, and a timeout with zero caught passes. Compare the listed and tested mutant sets and settle exit-3/timeout explicitly. | lane repro with stubbed lists |
| R10 | minor | codex#5 | Coverage measurement dropped; global rule requires `cargo llvm-cov` on modified code (project has `scripts/check.sh coverage`). | rule text |
| R11 | minor | pi#6 | `depends_on` omits 04-11; Task 3 may commit into files outside `files_modified` and append to 04-10's allowlist file. | plan text |
| R12 | minor | pi#7 | D-22 text names "chained or symbolic" only; record the widening (in plan; CONTEXT stays per its gate). | — |
| R13 | minor | pi#8 | New listing error quotes via `shown`; its twin `any_ref_starts_with` uses `lossy`. Note it, do not fix (WR-02). | — |

## Rejected

- agy#1 "`super::shown` is private so refs.rs cannot call it": false. `refs` is a child module of `git`; Rust lets a child use its ancestor's private items.
- agy#2 "`.tools/bin/cargo-nextest` does not exist": true only in the snapshot (gitignored); it exists in the checkout (`.tools/bin/cargo-nextest`, `cargo-deny`).

## Next

Fold R1–R13 into 04-12 (planner revision), then round 2 on the revised plan.
