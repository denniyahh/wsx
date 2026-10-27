---
phase: 03-git-adapter
reviewed: 2026-10-27T17:53:55Z
depth: deep
iteration: 2
files_reviewed: 12
files_reviewed_list:
  - Cargo.toml
  - clippy.toml
  - deny.toml
  - src/git/mod.rs
  - src/git/reads.rs
  - src/git/repo.rs
  - src/git/spawn.rs
  - src/git/testing.rs
  - src/main.rs
  - tests/harness/check.rs
  - scripts/check.sh
  - tests/gate_controls.rs
findings:
  critical: 0
  warning: 5
  info: 2
  total: 7
status: issues_found
---

# Phase 3: Code Review Report (iteration 2)

**Reviewed:** 2026-10-27T17:53:55Z
**Depth:** deep
**Files Reviewed:** 12
**Status:** issues_found

## Summary

This is a re-review after iteration 1's fixes (CR-01, CR-02, WR-01..04 in 03-REVIEW-FIX.md)
and two later commits: e478d3f, which turns off auto-maintenance in the test fixtures, and
0bdcc33, which adds the checkout's `.tools/bin` to check.sh's PATH. I did not re-report any of
iteration 1's fixed items. None of the fixes was found wrong.

No blocker was found. There are five warnings:

- Three are discovery divergences from git in `Repo::discover`. Each one was **measured**: a
  throwaway probe binary built against the pinned gix 0.90.0, with the same open options as
  `open_gix` and a verbatim copy of `refuse_at_ceiling`'s logic, was run next to git 2.56.0 on
  scratch repos under `target/`. The probe and the repos were removed afterwards.
  - The ceiling check compares against the wrong directory.
  - An absolute `GIT_DIR` gives git and wsx different worktrees.
  - gix ignores `GIT_OBJECT_DIRECTORY`.
- Two come from 0bdcc33's `.tools/bin` change:
  - The container gate picks up the host's tools.
  - A stale `.tools/bin` gets a fix suggestion that does not fix it.

  These two were worked out by reading the scripts and checking the image's user. They were
  **not** run in the container, because the brief says not to run `check-in-container.sh`.

The e478d3f change itself is sound. The scratch global config disables `maintenance.auto` and
`gc.auto`. Every other test that writes `.gitconfig` appends to it, so the setting survives.
The trace2 test has a real counterpart: I confirmed by hand that the trace contains
`"maintenance","run","--auto"` when auto-maintenance is on. Both new gate tests
(`repo_tools_dir_wins_over_a_shadowing_nextest`) and the maintenance counterpart were run alone
through `scripts/check.sh test -E 'test(=...)'` and pass.

The rest of the scope was checked and is fine:

- `clippy.toml` bans every public open/discover entry point that wsx's feature set compiles.
  `Submodule::open` needs the `attributes` feature, which is off.
- `deny.toml` and `Cargo.toml` are consistent with CONTEXT.
- `main.rs` is unchanged.
- The `reftable_refused` harness scenario follows the regression rules. When phase 5 enables
  the Rust side, `rust_stderr` is enforced.
- With a bogus hook `GIT_DIR`, `git rev-parse --local-env-vars` still works (measured), so the
  spawn helper's removal-list query cannot be broken by the hook environment.

## Warnings

### WR-01: The ceiling check compares against the worktree, not the directory where discovery found the repository

**File:** `src/git/repo.rs:239-266` (the `found` value comes from `refusal_path`, lines 403-405)
**Issue:** `refuse_at_ceiling` decides "git would not have found this" by testing whether a
ceiling entry equals `refusal_path(repo)`, the worktree. The directory git stops in, though,
is the one holding `.git`, or the git dir itself when the start is inside it. Two settings move
the worktree away from that directory: `core.worktree` in the repo config, and
`GIT_WORK_TREE` without `GIT_DIR`. Measured with gix 0.90.0 and git 2.56.0, starting in
`r1/inner` with `GIT_CEILING_DIRECTORIES=r1`:

| setup | git | gix workdir | wsx (copied logic) |
|---|---|---|---|
| plain (control) | `fatal: not a git repository` | `r1` | refused at ceiling (correct) |
| `GIT_WORK_TREE=<elsewhere>` | `fatal: not a git repository` | `<elsewhere>` | **opens** |
| `r2/.git/config` `core.worktree=<wt2>`, start `r2/inner`, ceiling `r2` | `fatal: not a git repository` | `<wt2>` | **opens** |
| start `r1/.git/objects`, ceiling `r1` | finds `r1/.git` (exit 0) | `r1` | **refused** |

The first two failing rows are the unsafe direction. wsx then reads, through gix, a repository
the user fenced off with a ceiling, while its own git children aimed at that "worktree" find no
repository. That breaks the locked decision that `GIT_CEILING_DIRECTORIES` "matches git".
The last row is the stricter direction iteration 1 noted as "not reproduced"; it is now
reproduced and has the same root cause.
**Fix:** compare the ceilings with the directory discovery stopped in, not with the worktree:

```rust
// in refuse_at_ceiling, replacing `let found = refusal_path(repo)?;`
let start_dir = canonical(start)?;
let git_dir = canonical(repo.git_dir())?;
let found = if start_dir.starts_with(&git_dir) {
    git_dir // git recognises the git dir itself while climbing (bare, or inside .git)
} else {
    start_dir
        .ancestors()
        .find(|dir| dir.join(".git").exists())
        .map(Path::to_path_buf)
        .unwrap_or(git_dir)
};
```

Add git-oracle tests for the `GIT_WORK_TREE` and `core.worktree` rows: git exits 128, so wsx must
refuse. Add one for the inside-`.git` row: git opens, so wsx must open. Keep the existing ceiling
tests as controls.

### WR-02: An absolute `GIT_DIR` without `GIT_WORK_TREE` gives git and wsx different worktrees below the root

**File:** `src/git/repo.rs:341-384` (`REPO_PATH_VARS`, `refuse_relative_repo_env`)
**Issue:** iteration 1's WR-03 fix refuses a *relative* `GIT_DIR` below the worktree root.
One of its own reasons was that "git takes `main/sub` as the worktree, where gix takes
`main`". The same divergence happens with an *absolute* `GIT_DIR`, and nothing refuses it.
Measured from `r1/inner` with `GIT_DIR=<abs>/r1/.git` and no `GIT_WORK_TREE`:

- `git rev-parse --show-toplevel` prints `r1/inner`, because git takes the cwd as the
  worktree root.
- gix reports `workdir=r1`.
- `refuse_relative_repo_env` returns early for absolute values, and `refuse_at_ceiling` skips
  whenever `GIT_DIR` is set, so `Repo::discover` opens with worktree `r1`.

Scenario: a user with `GIT_DIR` exported runs wsx from a subdirectory. wsx and its own-worktree
children (which run at `r1`) treat `r1` as the worktree. The user's `git status` in the same
shell treats `r1/inner` as the root and every file outside it as deleted. So wsx's view
disagrees with git's for the same environment, which the module doc
(`src/git/mod.rs:4-7`, "honours `GIT_DIR` ... the way git does") rules out. Inside hooks this
does not happen, because git runs hooks at the worktree root.
**Fix:** apply the cwd-equals-worktree condition whenever `GIT_DIR` is set and neither
`GIT_WORK_TREE` nor `core.worktree` is, whether the value is relative or absolute:

```rust
let git_dir_sets_worktree = std::env::var_os("GIT_DIR").is_some()
    && std::env::var_os("GIT_WORK_TREE").is_none()
    && repo.config_snapshot().string("core.worktree").is_none();
if relative.is_none() && !git_dir_sets_worktree {
    return Ok(());
}
// ...then the existing cwd == worktree comparison, with an error naming GIT_DIR.
```

Add a git-oracle test: absolute `GIT_DIR` from `main/sub` must be refused. The existing
`a_relative_index_at_the_worktree_root_opens_like_git` and the hook tests act as controls.

### WR-03: gix ignores `GIT_OBJECT_DIRECTORY`, so wsx reads a different object store than git

**File:** `src/git/repo.rs:46-64` (`open_gix`), `src/git/repo.rs:341-348` (`REPO_PATH_VARS`)
**Issue:** measured at `r1` with `GIT_OBJECT_DIRECTORY=<abs>/emptyobjs`, an empty store:

- `git cat-file -t HEAD` fails ("could not get object info").
- gix reports `objects=r1/.git/objects` and resolves `HEAD` to its commit.

So gix ignores the variable entirely. Two consequences:

- **Inside a receive-pack quarantine** (`pre-receive`/`update` hooks), git exports
  `GIT_OBJECT_DIRECTORY` as the incoming directory. There, gix cannot see the pushed objects
  and wsx fails with a generic `Error::Gix`, not a named refusal.
- **With a user override**, gix reads objects that git, and so wsx's own git children, cannot
  see. Reads and writes then disagree.

Neither case is refused or recorded as a divergence in CONTEXT. `GIT_ALTERNATE_OBJECT_DIRECTORIES`
was not measured.
**Fix:** refuse in `Repo::discover` when `GIT_OBJECT_DIRECTORY` is set, with a typed error that
names the variable. Alternatively, record it as a divergence in CONTEXT "Decided after
research" and in `open_gix`'s docstring. Measure `GIT_ALTERNATE_OBJECT_DIRECTORIES` the same way
and treat it to match. Pin the result with a git-oracle test and a control without the
variable.

### WR-04: The container gate runs the host's `.tools/bin` binaries ahead of the tools it just installed

**File:** `scripts/check.sh:23-26`, together with `scripts/check-in-container.sh:141,146`
(outside the listed files, but it is the path 0bdcc33 changes the behaviour of)
**Issue:** `check-in-container.sh` bind-mounts the whole checkout at `/workspace`, so a host
`.tools/bin` is mounted too. Inside the container it:

1. installs the pinned, checksum-verified nextest and deny into `/usr/local/cargo/bin`;
2. runs `scripts/check.sh`, which finds `/workspace/.tools/bin` and puts it first on PATH.

The image runs as root (verified: `docker image inspect ... {{.Config.User}}` gives `root`), so
the host's mode-0700 `.tools` is readable. The container gate therefore runs host-installed
binaries that it checks only by version string, not by checksum. That defeats "the same gate in
the pinned image CI uses".

Concrete failure: a branch bumps `CARGO_NEXTEST_VERSION` in `install-tools.sh`, and the
developer's `.tools/bin` still holds the old build. `check-in-container.sh` installs the new
version, then `verify-cargo` reports the old one and the container gate fails, while CI (no
`.tools/`) passes. The commit message gives "the container mounts the checkout read-only" as the
reason for leaving the install default unchanged, but it did not consider that check.sh inside
the container reads `.tools/bin` too. This was reasoned from the scripts and the image config,
not run, because the brief says not to run `check-in-container.sh`.
**Fix:** mask the directory in the container. Docker cannot create a mount point inside the
read-only bind, so mask it only when it exists, the same way the script already handles
`target/`:

```bash
TOOLS_MASK=()
if [ -e "$REPO_ROOT/.tools" ]; then
    TOOLS_MASK=(--tmpfs /workspace/.tools:ro)
fi
# ... docker run ... "${TOOLS_MASK[@]}" ...
```

Add a gate_controls test in the style of `container_gate_mounts_its_own_checkout_not_the_callers`:
with a fixture `.tools/` present, the stub docker log must contain the mask, and without one it
must not.

### WR-05: A stale `.tools/bin` gets a fix suggestion that cannot fix it

**File:** `scripts/check.sh:23-26`; the message comes from `scripts/install-tools.sh:126,131,140`
(install destination at line 209)
**Issue:** when `.tools/bin` holds a tool at a version other than the pin, `verify-cargo` fails
with `...; run scripts/install-tools.sh install cargo-nextest`. Without `WSX_TOOLS_DIR`, that
command installs into `${CARGO_HOME:-~/.cargo}/bin`, which `.tools/bin` still shadows.
install-tools.sh then prints `installed cargo-nextest <pin>`, because it checks
`$DEST/cargo-nextest` directly. The next `check.sh test` fails with the same error and the same
advice. Following the printed fix makes no progress. This fails closed (no false green), but
the gate gives wrong advice.
**Fix:** when check.sh has prepended `.tools/bin`, give the right destination. Two options:

- Have check.sh set `WSX_TOOLS_DIR="$PWD/.tools/bin"` alongside the PATH change, and have
  install-tools.sh's three messages print `WSX_TOOLS_DIR=$WSX_TOOLS_DIR scripts/install-tools.sh
  install $tool` when `WSX_TOOLS_DIR` is set.
- Or have check.sh print a one-line note after a failed verify that `.tools/bin` is first on
  PATH, and how to refill it.

Add a scratch-checkout test whose stale `.tools/bin` stub must make stderr name `.tools/bin`.

## Info

### IN-01: The maintenance counterpart test starts a detached background writer in a scratch tree it deletes immediately

**File:** `src/git/testing.rs:1170-1180` (with `commit_starts_auto_maintenance`, 1137-1156)
**Issue:** with `maintenance.auto=true`, git 2.56.0 starts `git maintenance run --auto --quiet
--detach` (argv measured from trace2). That is the same detached writer e478d3f removes from
the fixtures, and it runs while `Scratch`'s `TempDir` is being deleted. `TempDir`'s drop swallows
removal errors, so a race would leave a `git-unit-auto-maintenance-on-*` tree under `target/tmp`
rather than fail. That was not reproduced: 7 runs, no leftover, no `maintenance.lock` seen.
**Fix:** pass `maintenance.autoDetach=false` with `maintenance.auto=true` in the counterpart.
git then runs `maintenance run --auto --quiet --no-detach` in the foreground (measured), and the
trace still contains `"maintenance","run","--auto"`, so the assertion is unchanged.

### IN-02: `.tools/bin` shadows every command check.sh runs, not only the pinned tools

**File:** `scripts/check.sh:18-26`
**Issue:** the comment says "the version checks below still apply to whatever is found". That
holds only for cargo-nextest, cargo-deny and cargo-llvm-cov. A `cargo`, `rustc`, `rustfmt`,
`git` or `xargs` placed in the git-ignored `.tools/bin` would run first for every gate target,
unchecked. Nothing in the repo puts them there today, so this is a documentation and
hardening gap, not a defect.
**Fix:** narrow the comment to the three pinned tools. Optionally, have check.sh refuse a
`.tools/bin` that contains anything other than the names install-tools.sh can install.

---

_Reviewed: 2026-10-27T17:53:55Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
