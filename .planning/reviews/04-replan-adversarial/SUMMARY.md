# Adversarial review of the replanned Phase 4 (HEAD 9e8b2fc)

Lanes: codex gpt-6.1-sol high, rc 0, 5 findings. agy gemini-3.8-flash-high high, rc 0, 7 confirmed + 1 suspected. No lane dropped.
Verification: gix 0.90.0, git 2.56.0, with controls (probe-drive.sh, probe-main.rs.txt). Phase 4 code does not exist yet: the probes reimplement the plans' wording, so they test the design, not an implementation.

## Confirmed, no race needed
1. 04-11:132 duplicate replacement differing only in hex case. The lower-cased set hides it: guard passes, gix reads `only intermediate`, git exits 128 `duplicate replace ref`. Control: removing the uppercase ref restores agreement. (codex)
2. 04-11:132 `gitoxide.objects.replaceRefBase` in local config. Guard passes, gix reads `only far`, git reads `only original`. Control: unsetting restores agreement. (codex)
3. 04-10:103 `--iterate` gate certifies a survivor after a production fix. cargo-mutants 27.1.0 toy crate: iterate rc 0 with 0 mutants tested; fresh run rc 2, 1 missed. 04-10:190 admits the limit but the gate stands. (codex)
4. 04-RESEARCH:771-774 `encode_path` checks the last byte, so `a /` stays bare, and 04-04:110 refuses `only a /`. Fails closed. Proptest generates no trailing slash. By reading only. (agy)
5. RESEARCH integration(): `tried.push(head)` is unconditional, so `none of {tried} exists` is false when origin/HEAD exists but points outside the remote or holds an object id. By reading only. (agy)

## Confirmed, needs a ref change inside the run
6. 04-01:174 `ref_commit` validates then looks the name up again. Swapping the ref in between returned the workspace commit as integration tip; control returned the remote commit. Contradicts 04-09:235 "reads each ref once". Unfixed part of 04-REVIEWS:64. (codex, agy)
7. 04-11:132 guard reads live refs, gix uses the map from open. Chain deleted after open: guard passes, stale gix reads `only intermediate`, git `only original`. Rated low here against codex's Major: a chain must be removed within the run. Frequency not measured. (codex)

## Rejected
- agy 1, workspace_tip differs under commit replacement: false. Tip stayed the original id, equal to show-ref, and bytes matched git.
- agy 3, slashless base makes gix follow a chain: false as stated. gix read `only original`, git died with BUG. Guard does see nothing, and 04-11:82 discloses this.
- agy 5, commit cycle fails before the guard: false, tip read succeeds and guard refuses. Remainder is true but cosmetic: another error such as NoIntegrationBranch can surface first; the model still stops.
- agy 6, 6946ae9 superseded: false, it is an ancestor of HEAD. It would be missing only from a shallow clone.
- agy suspected, require hides not-configured: by design (D-15, D-16); RepoModel keeps the state.

## Residual risks
- Probes are reimplementations; the guard's ref listing uses a filtered `all()`, not the plan's `refs_directly_under`.
- Race likelihood not measured. Whether Phase 12 passes a trailing slash to encode_path is unknown until planned.
