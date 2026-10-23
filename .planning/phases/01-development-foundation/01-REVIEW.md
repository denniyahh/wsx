---
phase: 01-development-foundation
reviewed: 2026-10-23T20:23:53Z
depth: deep
files_reviewed: 31
files_reviewed_list:
  - CLAUDE.md
  - CONTRIBUTING.md
  - .config/nextest.toml
  - .devcontainer/Dockerfile
  - .devcontainer/devcontainer.json
  - .gitconfig
  - .github/dependabot.yml
  - .github/workflows/ci.yml
  - .github/workflows/deny.yml
  - .gitignore
  - .workspace-divergence
  - Cargo.toml
  - clippy.toml
  - deny.toml
  - rust-toolchain.toml
  - scripts/assert-image-parity.sh
  - scripts/check-in-container.sh
  - scripts/check.sh
  - scripts/install-tools.sh
  - scripts/setup-repo.sh
  - scripts/workspace/cut-pr-branch.sh
  - scripts/workspace/phase-worktree.sh
  - scripts/workspace/sync-workspace.sh
  - src/main.rs
  - tests/cli.rs
  - tests/fixtures/env-ban/Cargo.toml
  - tests/fixtures/env-ban/src/main.rs
  - tests/gate.rs
  - tests/gate_controls.rs
  - tests/image_parity.rs
  - tests/repo_policy.rs
findings:
  critical: 3
  warning: 4
  info: 4
  total: 11
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-10-23T20:23:53Z
**Depth:** deep
**Files Reviewed:** 31: 29 source and config files, plus CLAUDE.md and CONTRIBUTING.md (context, and in scope as docs). The lockfiles `Cargo.lock` and `tests/fixtures/env-ban/Cargo.lock` were left out by the scope filter.
**Status:** issues_found

## Summary

Scope: everything in `d693c9ec..HEAD` (ba901b9) outside `.planning/`. Backlog items 999.1–999.4 and the findings the earlier external review rejected are not reported again.

Every finding below except IN-03 was reproduced in a throwaway repo or a `git archive` copy under the session scratchpad, with a negative control, before it was written down. The reproduction steps are in each finding. The scratch trees have since been deleted.

Two findings contradict threats that `01-SECURITY.md` marks closed:
- **T-01-07** ("version verified before scan, CARGO_HOME/bin first"): CR-02 shows the version check can pass on one binary while a different, unverified binary runs.
- **T-01-15** ("per-job container image"): WR-02 shows a job running in an unpinned image that still passes the check.

CR-01 is a threat that is missing from the register altogether.

The supporting work is sound. Specifically:
- the env-ban clippy control and its empty-config counterpart
- the `NEXTEST_NO_TESTS=pass` control
- the `.gitconfig` exact-policy test
- the SHA-pin checker and its rejection fixture

Each has a real negative control and was not found defective.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: The tracked `.gitconfig` include lets any checked-out branch run code and disable the secret scan

**File:** `.gitconfig:1-14`, `scripts/setup-repo.sh:36,69-70,81-83,85-91`, `CONTRIBUTING.md:51-63`

**Issue:** `setup-repo.sh` adds `include.path ../.gitconfig` to `.git/config`. git resolves that path against the common git dir, so it points at the working-tree file of whatever branch is checked out in the main checkout. git re-reads it on every command. This is why git refuses to version config.

Failure scenario: the repository is public and accepts PRs. A PR changes `.gitconfig` to add `[core] fsmonitor = "<cmd>"`, or `core.pager`, `core.sshCommand`, `diff.external`, an alias, and so on. The maintainer runs `gh pr checkout N` or `git switch` in the main checkout. From the next `git` command on, `<cmd>` runs. That includes the git calls `gh` itself makes, and git calls in every linked worktree, because they all read the main checkout's copy.

The same file can also replace the secret scan. `setup-repo.sh` writes `hook.gitleaks-staged.command` (line 69) *before* the include (line 82). An included `[hook "gitleaks-staged"] command = true` therefore wins, because the last value read is the one git uses. `git hook list pre-commit`, the check at lines 87-91, still prints `gitleaks-staged`, so the setup check stays green while nothing is scanned.

`tests/repo_policy.rs::tracked_gitconfig_sets_exactly_the_shared_policy` would flag such a PR in CI. The code has already run on the reviewer's machine by then.

Reproduced in a scratch repo with `GIT_CONFIG_GLOBAL=/dev/null`:
- `main`'s `.gitconfig` only sets `gpg.format`. `git status` on `main` created no marker file (negative control).
- After `git switch pr`, where the PR branch's `.gitconfig` sets `core.fsmonitor = "touch …/PWNED; false"`, the next `git status` created `PWNED`.
- With the include added after the hook (setup-repo's order), `git config --get hook.gitleaks-staged.command` returned `gitleaks git --pre-commit --staged --redact` on `main` and `true` on a branch whose `.gitconfig` overrides it. `git hook list pre-commit` printed `gitleaks-staged` in both cases.

**Fix:** Drop the live include. Have `setup-repo.sh` copy the policy keys into local config once, using `.gitconfig` only as the source. Read the file from a trusted ref rather than the working tree, and allow only the keys the policy needs:

```bash
# setup-repo.sh: replace the include block (lines 72-83)
readonly POLICY_KEYS='^(gpg\.format|commit\.gpgsign|tag\.gpgsign|gitflow\.(branch|prefix)\.[a-z]+)$'
git config --no-includes -f .gitconfig --list | while IFS='=' read -r key value; do
    [[ $key =~ $POLICY_KEYS ]] || { echo "error: .gitconfig sets unexpected key $key" >&2; exit 1; }
    git config --local --replace-all "$key" "$value"
done
# and remove any existing include.path ../.gitconfig
git config --local --unset-all include.path '^\.\./\.gitconfig$' || true
```

Then update CONTRIBUTING.md:51-63, `.gitconfig`:3-14 and the CLAUDE.md trap at line 45-46. Add the threat to `01-SECURITY.md`. If the include is kept on purpose, record it as an accepted risk with this scenario written out. Do not leave it unregistered.

### CR-02: The pin check verifies one binary while cargo runs another (nextest and cargo-deny)

**File:** `scripts/check.sh:129-132`, `scripts/check.sh:145-150` (the fix in dfa6c20); claimed closed as T-01-07 in `01-SECURITY.md`

**Issue:** Both comments say "cargo finds `cargo-nextest` in $CARGO_HOME/bin before PATH". Each then runs `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" scripts/install-tools.sh verify …`. That premise is false whenever `$CARGO_HOME/bin` is already on `PATH`, which is the normal rustup setup.

cargo puts `$CARGO_HOME/bin` first only when it is *absent* from `PATH`; otherwise it follows `PATH` order (cargo#11020). So when an earlier `PATH` directory also holds the tool, `verify` checks the pinned copy in `$CARGO_HOME/bin`, prints `==> cargo-nextest 0.9.148 OK`, and cargo then runs the earlier, unverified copy.

This host is that configuration:
- `/home/linuxbrew/.linuxbrew/bin` is PATH entry 1 and `~/.cargo/bin` is entry 5.
- `cargo-nextest` and `cargo-deny` are brew-installed there.
- CONTRIBUTING.md:19-24 tells contributors to install the pinned tools into cargo's `bin` directory.

Following the documented setup therefore leaves brew's copy as the one that runs. Whatever version `brew upgrade` installs next runs under a printed "pinned version OK". That is the exact case run_deps' comment (lines 145-148) says it prevents.

Reproduced in a `git archive` copy:
- **Case A:** `CARGO_HOME=$S/ch`, with `$S/ch/bin/cargo-nextest` linked to the real pinned binary, and `PATH=$S/fake:$S/ch/bin:~/.cargo/bin:/usr/bin:/bin`, where `$S/fake/cargo-nextest` is a stub that prints and exits 0. Running `bash scripts/check.sh test` printed `==> cargo-nextest 0.9.148 OK`, then `FAKE cargo-nextest ran with: nextest run --locked …`, then `==> check.sh: test OK`, exit 0. Zero tests ran.
- **Control B:** the same setup with `$S/ch/bin` removed from `PATH`. The real nextest ran 28 tests. This confirms cargo prepends `$CARGO_HOME/bin` only when it is missing from `PATH`.
- `check.sh deps` with a stub `cargo-deny`: `==> cargo-deny 0.20.2 OK`, then `FAKE cargo-deny ran with: deny --locked check`, then `==> check.sh: deps OK`, exit 0.

No test covers this. `gate_controls.rs` only checks that the tool is missing, and deleting line 132 or line 150 fails nothing.

**Fix:** Check the version through cargo, so the lookup is the one the gate uses:

```bash
# install-tools.sh: add a mode, or do it in check.sh
nextest_line="$(cargo nextest --version)"   # runs the exact binary `cargo nextest run` will run
[[ "${nextest_line%%$'\n'*}" =~ ^cargo-nextest\ 0\.9\.148(\ |$) ]] || { echo "error: cargo nextest reports '$nextest_line'" >&2; exit 1; }
deny_line="$(cargo deny --version)"
```

Keep the pattern in `install-tools.sh`, for example with a `verify-cargo TOOL` mode that runs `cargo "${tool#cargo-}" --version`. Add a gate_controls test in which a stub `cargo-nextest` that prints a wrong version comes earlier on `PATH`. Running `check.sh test` must then exit 1 and must not print `check.sh: test OK`. Re-open T-01-07.

### CR-03: The PR cut silently drops code commits whose cherry-pick fails, and still reports success

**File:** `scripts/workspace/cut-pr-branch.sh:220`, `:252-254`, `:128`, `:289-305`

**Issue:** Line 220 runs `git cherry-pick "$HASH" --no-commit >/dev/null 2>&1 || true`, which swallows every cherry-pick failure. A *conflict* leaves unmerged entries, which lines 232-249 catch. But git sometimes refuses a pick outright and leaves nothing staged, for example when "untracked working tree files would be overwritten". In that case line 252 (`git diff --cached --quiet`) succeeds, and the commit is counted under "Excluded: planning/environment-only". Its code never reaches the PR branch.

The checks after that cannot see the loss:
- The forbidden-path audit only inspects what was replayed.
- The fidelity check (289-305) builds `TOUCHED_SET` from `INCLUDED_HASHES` only and compares it with paths that changed. A dropped commit appears in neither set.

The guard at line 128 (`git diff-index --quiet HEAD --`) ignores untracked files, so nothing stops the run beforehand. This checkout currently has untracked, non-ignored `.planning/state.json` and `.planning/milestone.lock`.

Reproduced in a scratch repo:
- Branch `feature/phase-9` has three commits:
  - `feat(09-01)` adds `src/a.rs` and `.planning/state.json`
  - `chore(09-01)` runs `git rm --cached .planning/state.json`, so the file stays untracked
  - `feat(09-02)` adds `src/b.rs`
- `cut-pr-branch.sh` exited 0 with `Included: 1`, `Excluded: 2`, `Audit passed`, `Fidelity: 1 path(s) differ … all from replayed commits` and `Clean PR Branch Ready`. `git diff --name-status origin/develop feature/phase-9-pr` showed only `A src/b.rs`, so `src/a.rs` was lost.
- **Control:** with the untracked file moved aside, the same run produced `Included: 2` and both `src/a.rs` and `src/b.rs`.

**Fix:** Never treat a failed pick as an empty one. Refuse untracked files up front:

```bash
# after line 131
if [ -n "$(git ls-files --others --exclude-standard)" ]; then
    echo "error: untracked files present; they can block a replayed commit. Move them aside first." >&2; exit 1
fi
# line 220
if ! git cherry-pick "$HASH" --no-commit >/dev/null 2>"$WORK/err"; then
    if [ -z "$(git diff --name-only --diff-filter=U)" ]; then
        echo "error: cherry-pick of $HASH failed without a conflict:" >&2; cat "$WORK/err" >&2
        git reset --hard HEAD --quiet; git checkout "$ORIG_BRANCH" --quiet; exit 1
    fi
fi
```

Also make the fidelity check cover every commit that touched a non-forbidden path, not just the included ones. Any such commit that was "excluded" as empty should have its paths compared against `$ORIG_BRANCH`.

## Warnings

### WR-01: `check.sh test` accepts several selectors, which reintroduces the documented false green

**File:** `scripts/check.sh:19-25`, `:73-98`, `:134`; `CONTRIBUTING.md:81-86`; `CLAUDE.md:28-30`

**Issue:** The docs warn that `-E 'test(=a) | test(=b)'` passes when one name is wrong, and say to run one test per command. The allowlist, however, accepts any number of `-E` flags and positional name filters (usage line 25 even advertises `...`). nextest ORs them together, so the same false green comes back through a form the docs do not mention. A positional name filter is also a *substring* match, so `--no-tests=fail` is satisfied by any partial match.

Reproduced in a `git archive` copy:
- `check.sh test -E 'test(=version_flag_prints_package_version)' -E 'test(=no_such_xyz)'` ran 1 test, printed `check.sh: test OK` and exited 0.
- `check.sh test version_flag_prints_package_version no_such_xyz` exited 0 the same way.
- **Control:** `check.sh test -E 'test(=no_such_xyz)'` alone exited 4.

**Fix:** In the argument loop, count selectors and refuse a second one:

```bash
selectors=0
# in each accepted branch: selectors=$((selectors + 1))
if [ "$selectors" -gt 1 ]; then
    echo "error: check.sh test takes at most one selector; a second one hides a selector that matches nothing" >&2; exit 2
fi
```

Alternatively, document the `-E a -E b` and multi-name forms beside the `|` trap. Add `tests/gate.rs` cases for both forms.

### WR-02: The parity check passes a job that runs in an unpinned image, and two of its refusal branches have no tests

**File:** `scripts/assert-image-parity.sh:169-171`, `:135`, `:59-62`, `:67-70`; `tests/image_parity.rs`

**Issue:** Lines 169-171 accept an `image:` line at *any* depth below `container:`, and the last match wins. The comment at lines 91-97 says only `container.image` decides where steps run. A nested key under a sub-map of `container:` therefore overrides the real image. Reproduced with a fixture tree using the real script:

```yaml
    container:
      image: evil/image:9.9.9-1-trixie
      env:
        image: mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie
```

That printed `image parity OK: … (1 job container image(s))` and exited 0. **Control:** renaming the nested key to `other:` reported the mismatch with `job 'test': evil/image:9.9.9-1-trixie` and exited 1. The trigger is contrived, since an env var would have to be named `image`, but a guard that passes on a job running elsewhere is the failure T-01-15 claims to rule out.

Related: line 135 only recognises a bare `jobs:`. A workflow file whose jobs block it fails to read, for example quoted `"jobs":`, adds zero jobs and is never refused per file. Only the global `total_images -eq 0` catches it, and the other workflow files' images satisfy that.

Test gaps in `tests/image_parity.rs`, where deleting the relevant line fails no test:
- the non-exact-tag refusal (lines 59-62, e.g. `FROM …:latest`)
- the "no workflow files" refusal (67-70)
- the reusable-workflow `uses:` job refusal

**Fix:** Read `image:` only at the first indent below `container:`:

```bash
elif [ "$container_indent" -ge 0 ] && [ "$indent" -gt "$container_indent" ]; then
    [ "$container_child_indent" -lt 0 ] && container_child_indent="$indent"
    if [ "$indent" -eq "$container_child_indent" ] && [[ $line =~ ^[[:space:]]*image:(.*)$ ]]; then
        job_image="$(image_value "${BASH_REMATCH[1]}")"
    fi
fi
```

Reset `container_child_indent=-1` wherever `container_indent` is reset. Refuse any workflow file from which zero jobs were parsed. Add fixture tests for the nested-image case, `FROM …:latest`, an empty workflows directory and a `uses:` job.

### WR-03: `check-in-container.sh` gates the repo under the current directory, not its own repo

**File:** `scripts/check-in-container.sh:29-30`

**Issue:** `REPO_ROOT="$(git rev-parse --show-toplevel)"` uses the caller's working directory. Every other gate script finds its repo from its own location (`check.sh:15`, `assert-image-parity.sh:32`, `setup-repo.sh:28`).

In this repository's layout the linked worktrees live *inside* the main checkout (`.worktrees/phase-1`). Running `.worktrees/phase-1/scripts/check-in-container.sh` from the main checkout therefore sets `REPO_ROOT` to the main checkout. The script then mounts the main checkout and runs *its* `scripts/check.sh`. The phase-1 changes are never compiled, yet the result is green (or red) under the worktree's script name.

Run from inside an unrelated repo, it gates that repo with whatever that repo's `scripts/check.sh` is. Not executed here, because Docker is out of scope. It follows directly from lines 29-30 and 134-143.

**Fix:**

```bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
```

### WR-04: The PR cut force-resets any branch it is given, including `develop`

**File:** `scripts/workspace/cut-pr-branch.sh:97-113`, `:190`

**Issue:** `git branch -f "$PR_BRANCH" "$BASE_REF"` runs without checking whether `PR_BRANCH` already exists, holds commits that are not replayed, or is a shared branch. Two cases:
- **The usage invites a mix-up:** `[TARGET_PR_BRANCH] [BASE_BRANCH]`. Running `cut-pr-branch.sh develop` (or `main`) resets local `develop` to `origin/develop`, which drops unpushed local commits from the branch; only the reflog keeps them. The script then commits the phase onto `develop` and prints `git push -u origin develop` as the next step. That breaks design constraint 2 ("never write shared branches").
- **Re-cutting after review:** fixups made directly on `feature/phase-N-pr` are discarded without a word.

Also, `git branch -f … origin/develop` sets the PR branch's upstream to `origin/develop` (the run printed "set up to track 'origin/develop'"). Under `push.default=upstream`, a bare `git push` would go to `develop`.

Reproduced in a scratch repo: `develop` had an unpushed commit (`LOCAL`). `cut-pr-branch.sh develop` exited 0 with `Clean PR Branch Ready: develop`. Afterwards `develop` held only `base`, `feat(09-01)` and `feat(09-02)`, and `LOCAL` was gone from the branch.

**Fix:** Before line 190:

```bash
case "$PR_BRANCH" in main|develop|master|workspace/*|personal/*|"$BASE_BRANCH")
    echo "error: refusing to use '$PR_BRANCH' as the PR branch" >&2; exit 1;; esac
if git show-ref --verify --quiet "refs/heads/$PR_BRANCH" &&
   [ -n "$(git rev-list "$BASE_REF..$PR_BRANCH" --not "$CURRENT_BRANCH")" ]; then
    echo "error: '$PR_BRANCH' has commits not on $CURRENT_BRANCH; delete it or pass a new name" >&2; exit 1
fi
git branch -f --no-track "$PR_BRANCH" "$BASE_REF"
```

## Info

### IN-01: The PR cut's "next steps" and usage text are stale

**File:** `scripts/workspace/cut-pr-branch.sh:6,25-28,320`

**Issue:** Line 320 tells the user to test the cut branch with `./tests/sandbox.sh`. That is the frozen Python suite, which does not exercise any of the Rust gate (CONTRIBUTING.md:5-7). The usage examples also name `./scripts/cut-pr-branch.sh`, but the script lives at `scripts/workspace/cut-pr-branch.sh`.

**Fix:** Make line 320 read `git checkout $PR_BRANCH && scripts/check.sh all`, and correct the paths in lines 6 and 25-28.

### IN-02: `wsx --version` panics with exit 101 when it cannot write to stdout

**File:** `src/main.rs:17`

**Issue:** `println!` panics on a write error. `wsx --version >/dev/full` printed `failed printing to stdout: No space left on device` as a panic and exited 101, a code outside the documented 0/2 contract (lines 5-6). With stdout closed (`>&-`) it exited 0 and printed nothing.

**Fix:** Use `writeln!(std::io::stdout().lock(), …)` and map an error to an `ExitCode` defined in the contract (or to 1). Add a `cli.rs` case that sends stdout to `/dev/full`.

### IN-03: `phase-worktree.sh` puts the phase number into a regex unescaped

**File:** `scripts/workspace/phase-worktree.sh:33,45,51`

**Issue:** Line 33 allows `.` in `PHASE`, and lines 45 and 51 use `PHASE` inside an ERE, where `.` matches any character. `1.1` would therefore satisfy the existence check against `### Phase 101:`. That cannot happen with today's 15 phases; it is speculative and was not reproduced.

**Fix:** Use `grep -qF "### Phase ${PHASE}:"` and anchor with `awk`, or escape the dots with `${PHASE//./\\.}`.

### IN-04: `tests/gate.rs` tests only refusals, never an accepted form

**File:** `tests/gate.rs:38-63`

**Issue:** The allowlist added in dfa6c20 accepts `--filterset=<expr>` and positional name filters, but no test runs either form through check.sh. Only `-E <expr>` is exercised, indirectly, by `gate_controls.rs::no_match_filter_fails_even_when_env_allows_no_tests`. A regression that refused `--filterset=` or name filters would go unnoticed.

**Fix:** Add one accepted case per form, with a no-match filter, asserting exit 4 and nextest's "no tests to run" (not usage exit 2). That fits next to the existing nested-nextest control.

---

_Reviewed: 2026-10-23T20:23:53Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
