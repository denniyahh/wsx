# 04-12/04-13 adversarial review — round 3 (tier A, final)

Lanes: agy, codex, pi (deepseek-flash) + gsd-plan-checker, at 40eedd6. All returned. No tracked file changed.

S1–S15: all lanes and the checker mark them closed (codex: S1/S3/S5 closed except the two shapes below).

## Open after round 3

| # | Sev | Lane | Finding | Orchestrator check |
|---|-----|------|---------|--------------------|
| T1 | critical | codex | A packed-refs record ending in CRLF: git 2.56.1 exits 128 `bad file`; gix reads the replacement (`only b`). LF control: both `only b`. | gix-ref-0.69.1 `packed/decode/mod.rs:34` treats `\r` as a line end — confirmed |
| T2 | major | codex | Replacement of an object missing from the store by an annotated tag: git prints the peeled blob; the target-kind loop skips a missing original, gix fails "kind tag", blamed on `.wsx.conf`. | lane repro with control |
| T3 | warning | checker | `ReplacementRefDuplicate` wording false for two non-hex names differing in case (git ignores both). | plan text |
| T4 | warning | checker | Estimates understate plan size (~69–70k at chars/4, possibly ~90k by tokenizer). | measured sizes |
| T5 | info | checker | `ReplaceRefsUnreadable` text vs object-header reads; guard-before-remote precedence undocumented. | plan text |

agy and pi: nothing critical or major. pi and agy scope: proportionate; `PackedRefsDuplicate` could narrow to replace records.

Pattern: each round found new git/gix parser differences in the packed-refs/object path (R1 symlinks, S3 unsorted, S5 tag targets, T1 CRLF, T2 missing original). Tier A round cap reached; operator decides.
