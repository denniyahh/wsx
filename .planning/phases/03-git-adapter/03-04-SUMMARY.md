---
phase: 03-git-adapter
plan: 04
subsystem: git
tags: [gix, clippy, disallowed-methods, lint-expectation, sha256, hooks]
status: complete

requires:
  - phase: 03-git-adapter
    provides: "03-01: open_gix (the only gix open call), Repo::discover, Repo::merge_base, Git::new/run, test support Scratch, git_ok, write, commit_all"
provides:
  - "clippy.toml bans all 14 gix 0.90.0 open/discover entry points; open_gix in src/git/repo.rs is the one production exception, under a reasoned #[expect(clippy::disallowed_methods)]"
  - "git::repo::tests::banned_gix_entry_points_refuse_a_missing_path: each banned entry point called once under its own expect, so a renamed entry point stops compiling and a misspelled ban path fails -D warnings"
  - "git::repo::tests::sha256_repo_opens: a --object-format=sha256 repository opens through Repo::discover with 64-character ids equal to git's"
affects: [03-05, 03-07, 03-08, 03-10]

actuals:
  tokens: 3669      # chars/4 over `git diff d0f2edf..HEAD` (clippy.toml + src/git/repo.rs, 212 insertions)
  tasks: 2
  commits: 2
plan_head_before: d0f2edf7300a04f31d31865b288a2ad618881565
plan_head_after: 59704069f1b59254841470c76c2b156a9babae9f

tech-stack:
  added: []
  patterns:
    - "A clippy ban list pinned by a test that calls each banned path under its own #[expect]: a stale path then fails -D warnings instead of only warning"

key-files:
  created: []
  modified:
    - clippy.toml
    - src/git/repo.rs

key-decisions:
  - "Each ban reason names the sanctioned route (Repo::discover / open_gix, siblings through main_repo()/worktrees()); the *_with_environment_overrides reasons say why they are still banned (default options, or the hook env applied to an arbitrary path such as a sibling worktree)"
  - "The sanctioned ThreadSafeRepository::discover_with_environment_overrides_opts is itself on the ban list, so open_gix's expect is fulfilled only while the ban holds and a second call anywhere fails the gate"

patterns-established:
  - "Pattern: one lint expectation per pinned call, never one per function, so a single unexpected call cannot hide behind a fulfilled expectation"

requirements-completed: [GIT-02]

coverage:
  - id: D1
    description: "Every gix open/discover entry point except open_gix fails cargo clippy -D warnings"
    requirement: "GIT-02"
    verification:
      - kind: command
        ref: "scratch control 1: planted gix::open(\".\") in a copy fails clippy rc 101 with `use of a disallowed method `gix::open``"
        status: pass
      - kind: command
        ref: "scripts/check.sh clippy on the real tree: rc 0, no `does not refer to a reachable function` and no other disallowed-method hit"
        status: pass
    human_judgment: false
  - id: D2
    description: "The ban list is pinned per entry: a misspelled or renamed ban path fails the gate"
    requirement: "GIT-02"
    verification:
      - kind: unit
        ref: "src/git/repo.rs#git::repo::tests::banned_gix_entry_points_refuse_a_missing_path"
        status: pass
      - kind: command
        ref: "scratch control 2: gix::open misspelled in a copy's clippy.toml fails clippy rc 101 with `this lint expectation is unfulfilled` at the gix::open call in the pin test"
        status: pass
    human_judgment: false
  - id: D3
    description: "A SHA-256 repository opens through Repo::discover and merge-base HEAD HEAD is 64 characters, equal to git rev-parse HEAD"
    requirement: "GIT-02"
    verification:
      - kind: unit
        ref: "src/git/repo.rs#git::repo::tests::sha256_repo_opens"
        status: pass
    human_judgment: false

duration: 4 min
completed: 2026-10-27
---

# Phase 3 Plan 04: gix open/discover ban and SHA-256 open Summary

**`clippy.toml` bans all 14 gix 0.90.0 open/discover entry points, leaving `open_gix` as the only open site; a per-call `#[expect]` pin test makes a stale ban path fail the gate, both directions are shown by scratch controls, and a SHA-256 repository opens with full-length ids.**

## Performance

- **Duration:** about 4 min (2026-10-27T02:07:22Z to about 02:11Z), including two scratch gix rebuilds
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- 14 `disallowed-methods` entries for gix, with a group header saying why (inside a hook, any other open ignores the hook's environment or applies it to a sibling worktree) and one sentence added to the exceptions comment.
- `#[expect(clippy::disallowed_methods, reason = "the single sanctioned way wsx opens a repository; see clippy.toml")]` on `open_gix`.
- `banned_gix_entry_points_refuse_a_missing_path`: 14 `let` statements, each under its own reasoned `expect`, each calling one entry point on `<scratch root>/does-not-exist` and asserting an error, naming the entry point in the failure.
- `sha256_repo_opens`: `git init --object-format=sha256`, one commit, `Repo::discover`, `merge_base(b"HEAD", b"HEAD")` is 64 characters and equals `git rev-parse HEAD` run through `Git::run`.

## Task Commits

1. **Task 1: Ban every other gix open path and pin the ban list** - `a689819` (chore)
2. **Task 2: A SHA-256 repository opens and answers a read** - `5970406` (test)

## Controls (each must produce the opposite result)

| Control | Expected | Observed |
|---------|----------|----------|
| Copy with a planted `gix::open(".")` test | clippy fails with ``use of a disallowed method `gix::open` `` | rc 101, exactly that error and no other disallowed-method hit |
| Copy with `path = "gix::open_misspelled"` in clippy.toml | clippy fails with `this lint expectation is unfulfilled` | rc 101, one unfulfilled expectation at `src/git/repo.rs:137`, the `gix::open` call's expect; clippy also printed the `does not refer to a reachable function` warning, which alone leaves clippy at rc 0 (measured in RESEARCH Pitfall 2, not re-measured here) |
| `sha256_repo_opens` with `--object-format=sha1` (in place, then restored from a backup copy) | the length assertion fails | rc 100, `merge-base id baec2491... is not SHA-256 length` (40 characters) |

Both clippy controls built into `target/gsd/ban-control` from a copy under the session scratchpad; each `EXIT` trap removed its copy and that build tree.

## Decisions Made

See `key-decisions` in the frontmatter. Neither changes a plan decision; both are wording and placement within the plan's instruction.

## Deviations from Plan

### Acceptance criterion adjusted to rustfmt's layout

**1. [Rule 3 - Blocking] Criterion 3's single-line regex cannot match formatted code**
- **Found during:** Task 1 acceptance check
- **Issue:** `rg -c '^\s*#\[expect\(clippy::disallowed_methods' src/git/repo.rs` printed nothing. The mandated reason strings make each attribute longer than 100 columns, so `cargo fmt` (part of the gate) puts `#[expect(` and `clippy::disallowed_methods,` on separate lines. The code was not changed; the count was taken in a form that matches the formatted layout.
- **Check used instead:** `rg -U --count-matches '#\[expect\(\s*clippy::disallowed_methods' src/git/repo.rs` prints `15`; an independent count, `rg -c '^\s*clippy::disallowed_methods,$'`, also prints `15`; `rg -c 'is_err\(\);$'` prints `14` (the pinned calls); the same multiline count on the 03-03 HEAD finds nothing (rc 1).
- **Files modified:** none for this deviation
- **Commit:** n/a

### Controls run through `bash -c` in the background

The two clippy controls ran as the plan's command lines wrapped in `bash -c`, with the tool in background mode, because the Bash tool's 600 s ceiling is below the plan's `timeout 1200`. The `rm -rf` in each trap was not refused by the harness, so no path substitution was needed.

**Total deviations:** 1 (criterion wording, no code impact). **Impact:** none on the delivered behaviour.

## Issues Encountered

- **The suite goes red when the test process has `GIT_DIR` set** (pre-existing, out of scope, logged in `deferred-items.md`). With `GIT_DIR` pointing at this checkout's git dir, the new pin test fails (`gix::discover_with_environment_overrides opened the missing path ...`), and the whole suite reports 17 of 207 failed. Without it, all 207 pass. This is a false red only (never a false green), and it is a property of how the suite is invoked, not of this plan's code.

## Verification Results (limits stated)

- `scripts/check.sh all` exited 0 after each commit: 206 then 207 tests passed, 42 skipped both times (the same skip count before and after this plan).
- Both new tests were also run alone with `--no-tests=fail -E 'test(=...)'`, each `1 passed`.
- **What the ban does not cover:** it covers gix's public open/discover entry points in 0.90.0 as read from its source (`lib.rs`, `open/repository.rs`, `discover.rs`, `open/options.rs`). A new entry point in a later gix release is not banned until that upgrade commit adds it, and the pin test cannot detect an entry point it does not call. It also does not cover opening a repository through a crate below gix (`gix-discover`, for example) or through a path clippy does not see as a call to these items.
- **What the pin test does not establish:** that the entry points behave identically on an existing repository; it only asserts they refuse a missing path, which is what lets it run without finding the wsx checkout.
- **SHA-256:** one repository with one commit, read through `merge_base`. It does not establish that every read (tree diff, range walk) handles SHA-256; only the open and one id round-trip are pinned, which is the scope CONTEXT decided.

## Known Stubs

None.

## Threat Flags

None. The plan's threats T-03-10 (second open site) and T-03-11 (stale ban path) are mitigated as planned and shown by controls 1 and 2; T-03-SC: no install, every cargo call `--locked`.

## Cleanup

- Removed: the session scratch directory `wsx-scratch` (both `ban.*` copies already removed by their traps), `target/gsd/ban-control` (confirmed absent after each control), and the `repo.rs` backup used for the SHA-1 control.
- Kept until the end of this run: the two gate logs and a guard script in the session scratchpad, removed before handback.
- `.planning/state.json` was untracked before this plan started and was left untouched.

## Next Phase Readiness

Ready for 03-05. Any later plan that needs a sibling worktree must go through `Repository::main_repo()` / `worktrees()`; a direct open now fails `scripts/check.sh clippy`.

## Self-Check: PASSED

- FOUND: clippy.toml, src/git/repo.rs
- FOUND: a689819, 5970406 (both ancestors of HEAD; `check evaluation-scope --plan 03-04 --commits-only` resolves both)
- Acceptance: 14 `path = "gix::` entries; `gix::open::Options::open` present; 15 disallowed-methods expectations (multiline count, see deviation); 2 non-comment `discover_with_environment_overrides_opts(` calls; `--object-format=sha256` present in `src/git/repo.rs`.
