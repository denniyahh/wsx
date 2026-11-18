# Memory Recall (MemPalace)

_Wing: wsx · Mode: augment · Transport: mcp · Recalled 2026-11-18, before discuss/plan (no CONTEXT.md yet; topic from ROADMAP Phase 5 goal)_

Signal check: topic queries scored 0.40-0.56; a nonsense control query scored 0.17-0.22, so the hits below are real
matches, not noise. Most hits are Phase 4 material the check will consume; nothing in the palace is about
`wsx check` itself yet. Native memory (STATE.md obligations for Phase 5, .planning/graphs/) is primary under augment.

## Prior decisions
- The personal-path list comes only from a regular or executable `.wsx.conf` blob at the workspace tip; a symlink, directory, gitlink or out-of-order root tree is refused before parsing — 04-09-SUMMARY.md (milestones), 04-CONTEXT.md D-14/D-16
- Manifest grammar: `#` comments and blank lines only; any other unknown line is an error naming the line; a line wsx cannot read exactly is never shortened into a path — 04-CONTEXT.md D-11, 04-04-PLAN.md prohibition
- No personal path is hard-coded except `.wsx.conf` and the worktree directory; `.planning` or `CLAUDE.md` is personal only when declared (bears on DIV-01 and IF-02's source scan) — 04-09-PLAN.md prohibition
- A workspace-branch fallback match names the branch but the repo stays NotConfigured (guard exits 0); a configured repo with a missing or invalid manifest fails closed — kg: wsx decided (Phase 4 D-15, D-16), valid_from 2026-11-01
- Model inputs (refs, listings, config, manifest blob) are read through the git binary; gix only reads objects by id — kg: wsx phase 04 decided D-24/D-25/D-26, valid_from 2026-11-17
- 04-18 left Phase 5 obligations in STATE.md: the read order, the exit codes to assign, and gix's info/grafts and shallow gap — 04-18-PLAN.md (planning); read STATE.md for the text

## Patterns
- Every refusal test keeps a positive control and asserts the intended failure text, not only the exit code; RED evidence via `check tdd-red-evidence … --raw --pick verdict` (the plain `--raw | rg -qx` pipe never matches) — 04-LEARNINGS.md lesson 1, 04-02-SUMMARY.md
- Mutation passes count only when fresh at the last src commit; plan the full pass as a long, load-sensitive last task — 04-LEARNINGS.md

## Surprises / gotchas
- `git for-each-ref` has no `-z` on git 2.56.0; use `--format=%(refname)%00` and require NUL-terminated lines — 04-08-SUMMARY.md
- A RESEARCH measurement of pathspec behaviour was wrong for a repo that tracked the path; assert git's answer in the test rather than copying a measurement — 04-04-SUMMARY.md
- Deferred: real recorded `wsx-new` branch set for model::resolve goes to Phase 8 — kg: wsx phase 04 deferred_to_phase_8, valid_from 2026-11-08
