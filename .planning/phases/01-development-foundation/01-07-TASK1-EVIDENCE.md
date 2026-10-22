# Phase 1 Plan 07, Task 1 evidence (pre-checkpoint)

Recorded 2026-10-22, 20:27:20Z to 20:31Z, on `feature/phase-1` at `e067e90`. This file holds
Task 1's evidence and the owner's prepared commands until Task 3 writes `01-07-SUMMARY.md`.
It is not named `*-SUMMARY.md` on purpose: GSD counts a SUMMARY file as a finished plan, and
Task 3 is still open. Task 3 should fold this content into the SUMMARY.

No executor pushed, opened a PR or changed a ruleset. Every `gh` call was a GET.

## What these results do and do not establish

- **The gates passed once on this tree, on this host and under local rootless Docker.** That
  shows CI's commands work in the pinned image here. It does not show GitHub's container runtime
  behaves the same. Task 3 checks the real CI run.
- **The rehearsal's base is the local `develop`, `d693c9e`.** `git ls-remote origin` at 20:29Z
  gave the same commit for GitHub's `develop`, so today the rehearsal and the owner's cut start
  from the same base. If `develop` moves before the owner cuts, the result can differ.
- **The rehearsal ran with an empty global git config, so it did not sign anything.** The
  owner's real cut replays 14 commits with `git commit -C`, which follows the owner's own signing
  config. If signing is on, each replayed commit gets signed, which may need the SSH agent. The
  rehearsal does not exercise that.
- **The ruleset dry run proves the shape of the request bodies, not that GitHub accepts them.**
  Whether the PUT accepts `integration_id`, and the read-only `pull_request` parameters echoed
  back from the GET, is only known when the owner runs it. The plan's fallback covers
  `integration_id`.
- **The history scan matches fixed strings.** It finds the five named control artifacts and
  classic `ghp_` tokens with 36 characters. It would not find a fine-grained `github_pat_` token
  or a control artifact under another name.
- **The control literals do appear in planning files** (23 lines of `.planning` history: plans
  and summaries naming the controls). The scan excludes `.planning` by design, and these lines
  describe the controls; they are not the artifacts.
- **Precondition note.** Step 1 asks for a clean tree apart from untracked `.planning` files.
  `.planning/config.json` is modified and uncommitted. It is the orchestrator's change and was
  left alone. It cannot affect the gates (no code) or the rehearsal (which clones committed refs),
  but the owner's step 0 will list it and must commit it before the cut.

## Block 1: the gates on the final tree (exit 0)

Host `scripts/check.sh all` and `deps`, both through `scripts/check-in-container.sh`, in-image
parity, host parity, shellcheck, `bash -n` and modes. Quoted from the run:

```
==> rustc 1.99.1 (58cf8f9c3 2026-10-13)
     Summary [   0.009s] 2 tests run: 2 passed, 0 skipped
==> check.sh: all OK
==> cargo-deny 0.20.2 OK
advisories ok, bans ok, licenses ok, sources ok
==> check.sh: deps OK
image parity OK: mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie (4 workflow image(s))
```

In the container (`c1`, `c2`, `c3`):

```
==> image:  mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie
==> target: all
==> rustc 1.99.1 (58cf8f9c3 2026-10-13)
     Summary [   0.012s] 2 tests run: 2 passed, 0 skipped
==> check.sh: all OK
==> target: deps
==> cargo-deny 0.20.2 OK
advisories ok, bans ok, licenses ok, sources ok
==> check.sh: deps OK
image parity OK: mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie (4 workflow image(s))
```

No `command not found` in the container output. All five scripts are tracked as `100755`:
`assert-image-parity.sh`, `check-in-container.sh`, `check.sh`, `install-tools.sh`,
`setup-repo.sh`.

## Block 2: PR-cut rehearsal in a scratch clone (exit 0)

`git clone -b feature/phase-1` of the common git dir, with an empty global config and
`GIT_CONFIG_NOSYSTEM=1`, then a local `workspace/denniyahh` from
`origin/workspace/denniyahh`, then `scripts/workspace/cut-pr-branch.sh --phase 1` in the clone.
Rehearsal base: the clone's `origin/develop` = `d693c9ec18b2720f9a8a78fe8f8f76a180c1daa9`
(the local `develop`; `develop` and `origin/develop` are equal here).

```
==> Source branch: feature/phase-1 (30 commits scoped to phase 1 not yet in origin/develop)
==> Commit classification summary:
    Included: 14 code commit(s)
    Excluded: 16 planning/environment-only commit(s)
==> Audit passed: 'feature/phase-1-pr' carries ZERO personal/agent artifacts.
==> Fidelity: 22 path(s) differ from origin/develop, all from replayed commits.
warning: these paths do not match feature/phase-1; review each before pushing:
  .gitignore
```

- The PR branch changes 22 paths against `develop`, the same set as the phase's own
  non-planning changes since `08b7293` (the merge base with `workspace/denniyahh`). None is
  under `.planning/` or `scripts/workspace/`, and `.workspace-divergence` is not among them.
- Every path except `.gitignore` is identical on `feature/phase-1` and the PR branch. The PR's
  `.gitignore` has `/target/`. Its only difference from the phase branch is that the
  "Personal workspace" block (`.worktrees/`, `graphify-out/`, `.planning/graphs/`,
  `.planning/research/.cache/`) is missing, as expected.
- The excluded 16 include `594da30` (01-06's restore of develop's CLAUDE.md), which is empty
  on top of develop.
- Negative controls in the same clone: the same set-equality check fails (exit 1) when
  `.planning` is not excluded; the stripped-path pattern matches all three planted sample paths;
  the content-equality check fails (exit 1) on `.gitignore`; the workspace block is found on the
  phase branch's `.gitignore`.
- No `feature/phase-1-pr` ref exists in the real repository (checked after the run).

## Block 3: history scan (exit 0)

Base `08b7293` (merge base of HEAD and `workspace/denniyahh`), 32 commits. The code history
(non-planning patches, 1895 lines) has none of: the vulnerable test crate, the planted env-var
name, the mismatched image tag, the planted missing-tool call, a 36-character `ghp_` token. The
whole phase history (7907 lines) has no 36-character `ghp_` token. The runtime-generated sample
token matches the pattern, and an extra positive sample for each of the other four literals
matches the same alternation.

## Block 4: Docker volumes removed (exit 0)

Hash `4fa63246f2d5` (first 12 hex of sha256 of this worktree's path). `wsx-ci-target-4fa63246f2d5`
and `wsx-ci-registry-4fa63246f2d5` both existed right after block 1, were removed with
`docker volume rm` (no `-f`), and no longer exist. `docker volume ls` lists no `wsx*` volume and
`docker ps -a` lists no container. No container was run after this block.

**Any later `scripts/check-in-container.sh` run from this worktree re-creates both volumes**,
including a phase verifier re-checking criterion 1. Whoever runs it should afterwards run:

```bash
docker volume rm wsx-ci-target-4fa63246f2d5 wsx-ci-registry-4fa63246f2d5
```

Kept on purpose: the image `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`. It was pulled
during research for this project, not created by an executor.

Scratch: the rehearsal clone `exec-01-07/cut.E2tWhA` was removed by its literal path. The rest of
`exec-01-07/` (block scripts and logs) is removed at the end of the plan.

## Block 5: ruleset dry run (GET only, exit 0)

Current state: both rulesets `active` with rules `pull_request`, `deletion`, `non_fast_forward`;
`25445011` updated_at `2026-10-19T21:01:17.523-04:00`, `25445012` updated_at
`2026-10-19T21:01:18.252-04:00`. `25445013 workspace-no-delete-no-force` was read, not changed
(updated_at `2026-10-19T21:01:18.973-04:00`).

Controls: each current ruleset fails the four-rule check (so the check can fail); the existing
three rules are carried into each body byte for byte; running the jq program on its own output
still gives exactly one `required_status_checks` rule.

Body for 25445011:

```json
{"name":"develop-merge-or-squash","target":"branch","enforcement":"active","conditions":{"ref_name":{"exclude":[],"include":["refs/heads/develop"]}},"bypass_actors":[],"rules":[{"type":"pull_request","parameters":{"required_approving_review_count":0,"dismiss_stale_reviews_on_push":false,"required_reviewers":[],"require_code_owner_review":false,"require_last_push_approval":false,"required_review_thread_resolution":false,"require_extra_approval_for_unattributed_changes":true,"allowed_merge_methods":["merge","squash"]}},{"type":"deletion"},{"type":"non_fast_forward"},{"type":"required_status_checks","parameters":{"strict_required_status_checks_policy":true,"do_not_enforce_on_create":false,"required_status_checks":[{"context":"Test","integration_id":15368},{"context":"Clippy","integration_id":15368},{"context":"Format","integration_id":15368}]}}]}
```

Body for 25445012:

```json
{"name":"main-squash-only","target":"branch","enforcement":"active","conditions":{"ref_name":{"exclude":[],"include":["refs/heads/main"]}},"bypass_actors":[],"rules":[{"type":"pull_request","parameters":{"required_approving_review_count":0,"dismiss_stale_reviews_on_push":false,"required_reviewers":[],"require_code_owner_review":false,"require_last_push_approval":false,"required_review_thread_resolution":false,"require_extra_approval_for_unattributed_changes":true,"allowed_merge_methods":["squash"]}},{"type":"deletion"},{"type":"non_fast_forward"},{"type":"required_status_checks","parameters":{"strict_required_status_checks_policy":true,"do_not_enforce_on_create":false,"required_status_checks":[{"context":"Test","integration_id":15368},{"context":"Clippy","integration_id":15368},{"context":"Format","integration_id":15368}]}}]}
```

The contexts equal the job `name:` values in `.github/workflows/ci.yml` (lines 51, 75, 90:
`Test`, `Clippy`, `Format`).

## Owner's command sequence (verbatim from the plan, Task 2)

Run these in a bash shell (your interactive shell is fish; the loop in step 5 is bash syntax), from the phase-1 worktree:

0. `git status --porcelain --untracked-files=no` must print nothing: `cut-pr-branch.sh` refuses on any uncommitted change to a tracked file, and the planning updates made at this checkpoint (for example STATE.md or ROADMAP.md) count. If it lists files, commit them first with `git add -u .planning && git commit -m "docs(01-07): record phase state before the PR cut"`; the cut classifies a commit that touches only `.planning` as planning-only and leaves it out of the PR. Untracked files (such as a SUMMARY draft) do not block the cut.
1. `scripts/workspace/cut-pr-branch.sh --phase 1`, then check its output shows `Audit passed` and that the only fidelity warning is `.gitignore` (expected: the PR copy has no workspace block).
2. `git push -u origin feature/phase-1-pr`
3. `gh pr create --base develop --head feature/phase-1-pr --title "Phase 1: development foundation" --body "Rust crate and gates, container and CI parity, supply-chain scanning. Evidence in the phase summaries."`
4. `gh pr checks <PR> --watch` until Test, Clippy and Format have finished; all three must pass. The `cargo deny (advisory)` job is advisory: report its result, but it does not block this step.
5. Only after step 4, update both rulesets (keeps every existing rule, replaces any earlier required-checks rule, pins each context to GitHub Actions):
   `for id in 25445011 25445012; do gh api "repos/denniyahh/wsx/rulesets/$id" | jq '{name, target, enforcement, conditions, bypass_actors, rules: ([.rules[] | select(.type != "required_status_checks")] + [{type: "required_status_checks", parameters: {strict_required_status_checks_policy: true, do_not_enforce_on_create: false, required_status_checks: [{context: "Test", integration_id: 15368}, {context: "Clippy", integration_id: 15368}, {context: "Format", integration_id: 15368}]}}])}' | gh api -X PUT "repos/denniyahh/wsx/rulesets/$id" --input - >/dev/null && echo "updated $id" || echo "FAILED $id"; done`
   If GitHub rejects `integration_id`, rerun without the three `integration_id` fields and say so in your reply. Do not touch ruleset 25445013.

Resume signal: reply `done PR=<number>` after step 5, noting any step that failed or any change to
the commands (for example dropping integration_id).

## Procedural notes for the SUMMARY's deviations section

- Block 2 ran with its `trap 'rm -rf "$S"' EXIT` replaced by printing `S`, and the clone was
  removed afterwards by its literal path (owner's instruction to delete scratch by explicit full
  path; same adjustment as 01-05). Its output was also copied to a log. The assertions were run
  unchanged.
- Blocks 1, 3 and 5 ran unchanged, with evidence printing appended after the block's exit code
  was captured. Block 4 ran unchanged apart from an added `echo` of the hash.
- This evidence file is an addition: the plan has Task 1 write into the SUMMARY, but a
  `01-07-SUMMARY.md` on disk would mark the plan finished before Task 3 runs.
