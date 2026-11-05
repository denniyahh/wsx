# Phase 4 plans, third adversarial review (after the 03.1 revision)

Snapshot: `b9bff9f` (git archive; diffed against a fresh archive afterwards, no tracked file changed).
Lens: adversarial. Lanes asked for: codex (gpt-6.1-sol, high) and agy (gemini-3.8-flash-high).

## Lane status

- **codex: returned.** Read all 11 plans in full. 4 findings, 0 suspected. 4 of 4 had their cited
  quotes found in the plans (one finding, number 2, cites the wrong file for one quote, see below).
- **agy: DROPPED.** Exit 3, 0 bytes of output, `RESOURCE_EXHAUSTED (429): Individual quota reached,
  resets in 41h51m` (`agy.err`). It reviewed nothing. This round is a single-lane review.

## Confirmed findings (checked by the orchestrator, not just repeated)

| # | Sev | Plan | Defect | How it was checked |
|---|-----|------|--------|--------------------|
| 1 | High | 04-01 (find_ref, line 187) | A `refs/namespaces` setting (config `gitoxide.core.refsNamespace` or env `GIT_NAMESPACE`) makes gix read the namespaced ref but report the plain name, so the planned "returned name equals requested name" check passes while the model reads another tip and manifest. No plan or source mentions namespaces; the `GIT_OBJECT_DIRECTORY` refusal list in `src/git/repo.rs` has no entry for it. | Absence in plans and `src/git` confirmed by search. gix-ref source applies `self.namespace` in `find.rs`. The end-to-end behaviour was measured by codex only; not reproduced here. |
| 2 | High | 04-09 / 04-RESEARCH | gix `Tree::find_entry` is a linear scan, so an unsorted tree holding `.wsx.conf` yields the manifest, while git cannot resolve the path. The plans say a corrupt tree "could read as `Absent`", which still fails closed. This case reads as present instead. | Git side reproduced: `cat-file blob <ref>:.wsx.conf` exits 128 on the unsorted tree, prints the blob on the sorted control. gix source read: `.find(...)` over the iterator. gix side not run. Citation error: the quoted sentence is at `04-RESEARCH.md:485`, not `04-09-PLAN.md:97`. |
| 3 | Medium | 04-11 (ReplacementRef text, line 144) | The refusal prints `git update-ref --no-deref -d {name}` with `{name}` passed through `c_quote`, which does not quote shell metacharacters. A replace ref named `refs/replace/$(touch${IFS}marker)` is valid, and pasting the printed command runs the substitution. | Fully reproduced in a scratch repo: `check-ref-format` accepts the name, the printed command created the marker, passing the name as one argument created none. Scratch removed. |
| 4 | Medium | 04-01 (fixture, line 137), 04-06 (lines 78, 135), 04-07 | `repo_with_remote` puts `develop`, `next` and `main` on the same commit, and the fork fixture's `develop` is a plain clone, so tests that check the chosen branch label cannot tell a wrong-rung or wrong-remote tip from the right one. 04-07's tip comparison against `git show-ref` is stated only for the `main` rung. | Fixture text read in the plans: `commit_all` at T0 then `git branch develop` and `git branch next`. Codex's control (a commit made only on upstream's `develop`) not re-run. |

## What this does not establish

- Findings 1 and 2 describe gix behaviour that only codex ran. The model code does not exist yet, so
  nothing here shows the executed Phase 4 would fail; it shows the plans do not prevent it.
- One lane found these four. Absence of other findings is not a clean bill: codex's COVERAGE lists
  all plans read, but its depth per plan is unknown.
