# 04-12/04-13 adversarial review — round 2 (tier A)

Lanes: agy, codex, pi (deepseek-flash) plus gsd-plan-checker. All returned. Snapshot at 345c17d; no tracked file changed (diff -rq); reviewers left only a build `target/`.

R1–R13: all three lanes mark R1–R4, R7–R13 closed. R5 partly (pi: chain-order claim has no failing direction). R6 not closed (pi, codex): see S2.

## To fold in (round 3)

| # | Sev | Lanes | Finding | Status |
|---|-----|-------|---------|--------|
| S1 | major | pi#2, codex#2, agy#2, agy#6 | Guard errors that are not refusals (invalid packed name `Invalid packed reference`, undecodable loose ref, `Error::Io` from the 04-13 walk, a replacement whose target is the wrong kind) still map to `ManifestUnreadable`, blaming `.wsx.conf` — against D-22. Needs a blame-the-replace-store variant or mapping, with a test. | 3 lanes, repro'd by pi and codex |
| S2 | major | pi#1, codex#4, checker I2 | Refusal wording false for some shapes it fires on: `ReplacementRefUnlisted` for the invalid-name kind (git exits 128 `bad file`, does not read it); `ReplacementRefIgnored` when `core.useReplaceRefs=false` / `GIT_NO_REPLACE_OBJECTS` (git ignores it). Word only what wsx knows, per kind, and test every rendered kind. | repro'd |
| S3 | critical | codex#1 | packed-refs whose header says `sorted` but whose records are not: git aborts (`BUG: refs/packed-backend.c:1020`), gix reads the replacement, model resolves Configured. Control: sorted order → both read `only b`. | single lane, with control — planner to re-probe |
| S4 | critical | agy#1 | Claim: 04-12 Task 2 RED test `replace_base_without_a_trailing_slash_fails_closed` fails at RED with `ReplaceBaseDiffers` text, not `answered "configured", not`, so the RED log check exits 1. Checker asserted the opposite (gix maps the variable onto its key per 04-11's env-base test). | DISPUTED — planner must settle by reading/running at HEAD |
| S5 | critical | agy#2, agy#3 | Claim: a replace ref whose target is an annotated tag (loose, or packed with a `^` peeled line) — git prints the peeled blob, gix stores the unpeeled tag id and fails "kind tag", blamed on `.wsx.conf`. | single lane — planner to re-probe; at minimum S1 covers the blame |
| S6 | major | agy#4 | 04-13:112 "the base followed by its path ... joined with '/'" yields `refs/replace//<x>` when the base ends in `/`, so every regular loose ref would be refused. Fix the wording to concatenate. | plan text |
| S7 | warning | checker W1 | 04-12 doc sentence "every ref git applies is then directly under the base and applied by gix too" is false at 04-12's tip (symlinks deferred to 04-13); scope it, and have 04-13 restate. | checker |
| S8 | warning | checker W2 | STATE.md:235 "runs only inside root_entry" becomes false after 04-12 Task 2; correct it in the same STATE edit. | checker |
| S9 | minor | pi#5 | STATE.md edits use `Edit`; 04-10 used `gsd_run query state.add-blocker`. Use the GSD command where one fits (hand edit of GSD output is an incident). | plan text |
| S10 | minor | pi#3 | `scripts/check.sh deps` `<fails_when>` cites `advisories ok` but nothing reads the output. | plan text |
| S11 | minor | pi#4 | "before 04-11's chain checks" ordering claim has no failing direction; drop or add a control. | plan text |
| S12 | minor | pi#6 | "fifo, socket" generalised to "any other entry"; device files unmeasured. Refuse every non-dir non-regular entry, or name only measured kinds. | speculation |
| S13 | minor | pi#7 | `is_replace_base` admits per-worktree bases (`refs/worktree/`…) whose walk arm is untested. | plan text |
| S14 | minor | agy#5 | 04-13 Task 2's STATE acceptance criterion has no `<verify>` command. | plan text |
| S15 | info | checker I1 | "ten model tests" vs eleven listed. | plan text |

## Not folded in

- codex#3 (TOCTOU: map cached at `Repo::discover`, ref removed before `resolve`): inherited, already accepted as T-04-37 (`src/model/mod.rs:350-355`, 04-11-PLAN:184). No change.
- codex SUSPECTED Unicode NFD base under `core.precomposeUnicode`: covered by S1's blame fix; no separate change.
