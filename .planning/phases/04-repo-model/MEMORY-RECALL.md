# Memory Recall (MemPalace)

_Wing: wsx · Mode: augment · Transport: cli · Hits scored cosine 0.67–0.72 against a nonsense-query control of 0.13_

## Prior decisions
- D-22: a silent mismatch with git is the worse failure; over-refusal (failing safe) is the accepted cost — 04-11-SUMMARY.md
- 04-11 put `Repo::refuse_unfollowable_replacements` first in `root_entry`, repo-wide, with `git::Error::ReplacementRef` / `ReplacementRefDuplicate` / `ReplaceBaseDiffers` read through `Error::is_replacement_refusal` — 04-11-SUMMARY.md

## Patterns
- 04-11's test matrix sorts shapes into "refused", "agreement, not refused", "agreement, refused anyway (fails safe)" and "not covered" — 04-11-PLAN.md
- Removal commands in refusal messages quote the ref name through `shell_word` — 04-11-SUMMARY.md

## Surprises / gotchas
- 04-11-PLAN.md listed a **nested name** under "agreement, not refused". The verifier disproved that: git applies `refs/replace/<dir>/<hex>` (keys on the last component), gix ignores it. That planning assumption is the gap — 04-11-PLAN.md vs 04-VERIFICATION.md
- Over-refusal shapes 04-11 named but never tested: non-hex name, case-differing non-hex names, `core.useReplaceRefs=false`, `GIT_NO_REPLACE_OBJECTS`, and a differing base whose only refs are nested or non-hex — 04-11-SUMMARY.md
- A corrupt ref under the replace base maps to `ManifestUnreadable`, which blames `.wsx.conf`; no test pins it — 04-11-SUMMARY.md
