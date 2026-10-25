---
phase: 01-development-foundation
reviewed: 2026-10-25T10:51:19Z
depth: deep
iteration: 2
diff_base: bf68d0a5f77dac069836aee77bc61ad5fed96f49
files_reviewed: 13
files_reviewed_list:
  - .gitconfig
  - CLAUDE.md
  - CONTRIBUTING.md
  - scripts/assert-image-parity.sh
  - scripts/check-in-container.sh
  - scripts/check.sh
  - scripts/install-tools.sh
  - scripts/setup-repo.sh
  - scripts/workspace/cut-pr-branch.sh
  - tests/gate.rs
  - tests/gate_controls.rs
  - tests/image_parity.rs
  - tests/setup_repo.rs
prior_findings_resolved:
  - CR-01
  - CR-02
  - CR-03
  - WR-01
  - WR-02
  - WR-03
  - WR-04
  - IN-01
prior_findings_open:
  - IN-02
  - IN-03
  - IN-04
findings:
  critical: 0
  warning: 1
  info: 7
  total: 8
status: issues_found
---

# Phase 1: Code Review Report (iteration 2, incremental)

**Reviewed:** 2026-10-25T10:51:19Z
**Depth:** deep
**Files Reviewed:** 13
**Status:** issues_found

## Summary

This is an incremental re-review of `bf68d0a..HEAD` (85392a4) over the 13 files listed. It covers the fixes for CR-02, CR-03, WR-01 to WR-04 and IN-01, and the CR-01 resolution: `setup-repo.sh` now copies an allowlist of keys and has `all|policy|check` modes.

**No blocker was found.** Every prior Critical and Warning is fixed. The CR-01 redesign holds against everything tried: include directives, keys outside the allowlist, bare booleans and duplicate keys are each refused before anything is written.

One new Warning, reproduced. `setup-repo.sh policy` aborts part-way through with git's raw exit 5 when a policy key already has several values in local config. By then it has already removed the legacy include and written some keys. The `check` mode's advice ("run setup-repo.sh policy") then loops, because `policy` fails the same way every time.

The Info items are:
- two leftovers from the WR-04 and IN-01 fixes (one reproduced, one confirmed by evaluating the script's own slug logic);
- one environment-dependent root-resolution issue that predates this work (reproduced);
- test gaps in the new `setup-repo.sh` refusal branches;
- a pre-existing silent-drop path for merge commits in the PR cut (reproduced), in the same class as CR-03;
- the prior IN-04, which is still open.

What was run:
- `scripts/check.sh test -E 'binary(setup_repo)'`: 9/9 passed. This shows the new tests pass on this host. It does not show they cover the untested refusal branches listed in IN-04 below.
- Scratch reproductions under the session scratchpad, each with a control that has to come out the other way. The scratch directory has been deleted. The test run's own fixtures stay under the gitignored `target/tmp`.

Not done: no container run, no CI re-run, no mutation testing beyond grepping the tests for the refusal messages.

## Prior findings: status

| Prior | Status | Evidence |
|---|---|---|
| CR-01: tracked `.gitconfig` include runs code and can disable the scan | **Resolved** | `setup-repo.sh:52,101-134` reads with `--no-includes -z` and refuses any key outside `POLICY_KEYS` before writing anything. `:195-198` removes only the exact legacy include. `tests/setup_repo.rs:197-232` has the refusal test and an include counterpart that must run the fsmonitor. None of the allowlisted keys names a program. T-01-27 is registered. Residual: WR-01 and IN-05 below. |
| CR-02: pin check verifies one binary while cargo runs another | **Resolved** | `install-tools.sh verify-cargo` runs `cargo <sub> --version`, so the version comes from the binary cargo resolves. `check.sh:149,168` use it. `gate_controls.rs::nextest_pin_is_checked_on_the_binary_cargo_runs` puts a stub earlier on PATH than the real binary and asserts exit 1, the stub's version in stderr, and that the stub never ran. The cargo-deny side has only a shell control, as the fix report admits. |
| CR-03: PR cut silently drops a commit whose cherry-pick fails | **Resolved** for the reproduced class (`cut-pr-branch.sh:247-255` aborts on a failed pick that leaves no conflict). The untracked-file pre-check and the wider fidelity check were deliberately not applied. A related pre-existing path remains: see IN-06. |
| WR-01: several selectors bring back the false green | **Resolved** | `check.sh:79-112` counts selectors and refuses a second one. `gate.rs:65-90` covers the two-`-E`, `--filterset`/`--filterset=` and two-name forms with exact-match names, so the test cannot recurse into itself if the refusal regresses. The positional substring match is still open (folded into IN-04). |
| WR-02: nested `image:` passes a drifted job; untested refusal branches | **Resolved** | Only a direct child of `container:` counts (`assert-image-parity.sh:178-186`), and a file with no readable jobs is refused (`:191-194`). Six fixtures were added, including the reverse-order case. Traced bypasses that now fail closed: a quoted `"container":` or `container :` key, a flow mapping, a multi-line plain scalar under `container:`, `jobs: {}`, CRLF line endings. |
| WR-03: container gate uses the caller's directory | **Resolved** | `check-in-container.sh:32-34`, with a stub-docker test. See IN-03 for the CDPATH caveat. |
| WR-04: PR cut force-resets any branch | **Resolved**, with a narrow residual (IN-02) | `cut-pr-branch.sh:126-135,214`. |
| IN-01: stale usage and next steps | **Resolved**, with a residual (IN-01 below) | `cut-pr-branch.sh:6,355`. |
| IN-02 (`src/main.rs` panic), IN-03 (`phase-worktree.sh` regex) | **Open, not re-reviewed** | Outside this review's file scope. Neither file changed since `bf68d0a`. |
| IN-04: gate accepted forms untested | **Open** | See IN-04 below. |

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: `setup-repo.sh policy` aborts part-way with exit 5 when a policy key already has several local values, and `check` then points at a fix that cannot succeed

**File:** `scripts/setup-repo.sh:195-202` (also `:39`, `:77`, `:172`)

**Issue:** Line 201 runs `git config --local "$key" "$value"`. When the key already has two or more values in `.git/config`, git refuses with "cannot overwrite multiple values with a single value" and exits 5. That can come from an earlier `--add`, a hand edit, or a tool that appends. Under `set -e` the script stops there. The damage:
- **Wrong exit code.** The header (line 39) promises exit 1 on any failure; this exits 5.
- **Partial writes.** It has already removed the legacy include (line 196) and written every key before the failing one. The comment at line 77 ("Every refusal below happens before any config is written") holds for the script's own refusals, but not for this failure.
- **The advice loops.** `check` then reports drift and says "Run scripts/setup-repo.sh policy to apply it" (line 172). That fails identically on every run, so following the advice gets nowhere.

Reproduced in a scratch repo with `GIT_CONFIG_GLOBAL=/dev/null`: local config held `commit.gpgsign=true`, `commit.gpgsign=false` and `include.path=../.gitconfig`.
- `setup-repo.sh policy` printed `removed include.path ../.gitconfig`, then git's "cannot overwrite multiple values" error, and exited **5**.
- Local config afterwards: the include was gone, `gpg.format=ssh` had been written, both `commit.gpgsign` values remained, and the other 8 keys were unset.
- `check` exited 1 listing 9 drifted keys and told the user to run `policy`.
- **Control:** after `git config --local --unset-all commit.gpgsign`, `policy` exited 0 and copied all 10 keys.

**Fix:** Make the write converge regardless of what is already there. Add a test that seeds a duplicate local value and expects exit 0, then `check` exit 0.

```bash
for i in "${!policy_keys[@]}"; do
    git config --local --replace-all "${policy_keys[$i]}" "${policy_values[$i]}"
done
```

## Info

### IN-01: The IN-01 fix kept a usage example that names the wrong branch

**File:** `scripts/workspace/cut-pr-branch.sh:31-32` (logic at `:112-117`)

**Issue:** The example says that on `workspace/phase-45` the script "Creates feature/phase-45". The slug logic strips `workspace/`, then the shortest `*-` prefix, so `phase-45` becomes `45` and the branch is `feature/45`. This was confirmed by evaluating the script's own expansions in bash. The IN-01 fix rewrote this exact line (path only) without correcting it.

**Fix:** Change the example to `# Creates feature/45 off origin/develop`, or stop stripping the handle when the remainder starts with `phase-`.

### IN-02: `--force` re-cut keeps an upstream the branch already had

**File:** `scripts/workspace/cut-pr-branch.sh:212-214`

**Issue:** `git branch -f --no-track` does not remove existing tracking config. A PR branch cut by the pre-fix script tracked the base (`origin/develop`). After `--force`, it still tracks the base, so the `push.default=upstream` case that the comment at lines 212-213 describes remains for such branches.

Reproduced: a branch created with `--track` against `develop` still reported `develop` as its upstream after `git branch -f --no-track pr develop`. Control: a freshly created branch had no upstream.

The impact is narrow. This repo's `feature/phase-1-pr` tracks `origin/feature/phase-1-pr` (set by `push -u`), and `push.default` is unset here.

**Fix:** Inside the `--force` path, after line 214, drop the upstream only when it is the base:

```bash
if [ "$(git rev-parse --abbrev-ref "$PR_BRANCH@{upstream}" 2>/dev/null)" = "$BASE_REF" ]; then
    git branch --unset-upstream "$PR_BRANCH"
fi
```

### IN-03: An exported `CDPATH` redirects every script's repo-root resolution (pre-existing; WR-03's guarantee depends on it)

**File:** `scripts/check.sh:15`, `scripts/setup-repo.sh:42`, `scripts/assert-image-parity.sh:33`, `scripts/check-in-container.sh:32`

**Issue:** When a script is invoked by a relative path, the root `cd` targets `scripts/..`. Bash looks that path up in `CDPATH` before the current directory. If an exported `CDPATH` holds a directory that has a `scripts/` subdirectory, the gate moves to that other tree and gates it.

Reproduced with bash: with `CDPATH=<other>` exported, `cd "$(dirname scripts/check.sh)/.."` landed in `<other>`. Control: without `CDPATH` it landed in the invoking tree.

In `check-in-container.sh`, the `$(cd … && pwd)` capture also picks up the path that `cd` prints. `SCRIPT_DIR` becomes two lines and `git -C` fails, so that script fails loudly rather than gating the wrong tree.

The trigger depends on the environment: `CDPATH` is not exported in this environment, so it was not observed in real use. The `cd` lines in `check.sh`, `setup-repo.sh` and `assert-image-parity.sh` predate this work; the `check-in-container.sh` line is the WR-03 fix's new code.

**Fix:** Clear `CDPATH` for the `cd` and silence it, or start from an absolute path:

```bash
cd -P -- "$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." >/dev/null && pwd)"
```

### IN-04: Several `setup-repo.sh` refusal branches have no test; one is only proven where gitleaks is absent

**File:** `tests/setup_repo.rs` (branches at `scripts/setup-repo.sh:91-94,96-99,111-114,121-126,131-134`)

**Issue:** `rg` over `tests/` finds none of these messages: "more than once", "has no value", "sets no keys", "not found in", "gitleaks is required" and "older than". The duplicate-key, bare-boolean, empty-policy, missing-file, missing-gitleaks and old-git branches can each be deleted without failing a test.

The bare-boolean refusal matters. Without it, `[commit] gpgsign` (no `=`) would set `commit.gpgsign` to the literal string `commit.gpgsign`, because the value expansion at line 116 returns the whole entry.

Separately, "policy does not need gitleaks" is implied by the `policy` tests, which use no stub. But `hermetic()` keeps the inherited `PATH`, and this host has gitleaks at `/home/linuxbrew/.linuxbrew/bin/gitleaks`. A regression that dropped `[ "$mode" = all ] &&` at line 91 would therefore pass here and fail only in an environment without gitleaks, presumably CI. That was not verified.

**Fix:** Add one fixture per refusal, asserting exit 1, the message and an unchanged `local_config()`. Add a `policy` run whose `PATH` is the stub dir plus a minimal tool dir without gitleaks.

### IN-05: `check` cannot see a key that was dropped from `.gitconfig`, and it prints unset and empty values alike

**File:** `scripts/setup-repo.sh:154-178`

**Issue:** `check` only iterates over the keys the current `.gitconfig` sets. A key removed from the policy, or renamed (for example a `gitflow.prefix.*` change), stays in every clone's local config forever, and `check` still says "matches". Drift is also shown as `local ''` whether the key is unset or set to an empty string. This is a limit of the design, not a false green on any key the policy sets.

**Fix:** Document the limit beside the Undo block, or have `check` also list local keys that match `POLICY_KEYS` but are absent from `.gitconfig`. Use `--get` exit status 1 to print `unset`.

### IN-06: The PR cut counts merge commits as "planning-only" and drops any content they carry (pre-existing, same class as CR-03)

**File:** `scripts/workspace/cut-pr-branch.sh:205,233-240` (phase mode: `:182` `--no-merges`)

**Issue:** Without `-m`, `-c` or `--cc`, `git diff-tree --no-commit-id --name-only -r <merge>` prints nothing. So in non-phase mode a merge commit gets an empty `TOUCHED_FILES` and is counted under "Excluded: planning/environment-only". Any content introduced only by the merge never reaches the PR branch: a conflict resolution, or an "evil merge" fix.

Reproduced: on a merge commit with a hand-resolved conflict, `diff-tree` printed `[]`. Control: on a normal commit touching `f`, it printed `[f]`.

The fidelity block (`:333-344`) reports such paths only as a `warning:` line, and the run still ends with "Clean PR Branch Ready". No end-to-end cut over a merge was run. Phase mode skips merges by design.

**Fix:** In non-phase mode, refuse merge commits with content (`git rev-list --min-parents=2 "$FORK_POINT..$CURRENT_BRANCH"` that is non-empty, plus a `git diff-tree --cc` check). Or turn the fidelity `DIFFERING` warning into a failure for paths that no excluded commit explains.

### IN-07: Gate accepted forms are still untested, and a positional name is still a substring match (prior IN-04, carried)

**File:** `tests/gate.rs:38-90`, `scripts/check.sh:91-106`

**Issue:** Only `-E <expr>` is exercised as an accepted form (by `gate_controls.rs`'s nested-nextest control). A regression in the new selector counting that refused a single `--filterset=<expr>` or a single positional name would go unnoticed. The positional form is also still a substring match: `check.sh test version` passes when any test name contains `version`, even if the intended test was deleted or renamed. The fix report flagged this as outside WR-01's scope.

**Fix:** Add one accepted case per form with a no-match filter, expecting exit 4 and nextest's "no tests to run" rather than usage exit 2. Then either refuse positional names, steering users to `-E 'test(=name)'`, or document the substring semantics beside the `|` trap in CLAUDE.md and CONTRIBUTING.md.

---

_Reviewed: 2026-10-25T10:51:19Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
