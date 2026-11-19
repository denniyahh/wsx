---
phase: 05-divergence-check-and-command-contract
reviewed: 2026-11-19T01:07:00Z
depth: deep
files_reviewed: 33
files_reviewed_list:
  - Cargo.toml
  - src/check/hooks.rs
  - src/check/mod.rs
  - src/check/report.rs
  - src/check/staleness.rs
  - src/cli/mod.rs
  - src/exit.rs
  - src/git/mod.rs
  - src/git/reads.rs
  - src/git/refs.rs
  - src/git/repo.rs
  - src/git/staleness.rs
  - src/git/trees.rs
  - src/json.rs
  - src/main.rs
  - src/model/mod.rs
  - tests/cli.rs
  - tests/harness/check.rs
  - tests/harness/contract.rs
  - tests/harness/main.rs
  - tests/harness/registry.rs
  - tests/harness/rust.rs
  - tests/if02_scan.rs
  - deny.toml
  - docs/exit-codes.md
  - docs/json/README.md
  - docs/json/check.v1.schema.json
  - src/cli/commands.rs
  - src/git/config.rs
  - src/git/objects.rs
  - src/git/spawn.rs
  - src/git/testing.rs
  - tests/harness/provenance.rs
findings:
  critical: 0
  warning: 5
  info: 5
  total: 10
  necessity: 1
status: issues_found
---

# Phase 05: Code Review Report

**Reviewed:** 2026-11-19T01:07:00Z
**Depth:** deep
**Files Reviewed:** 33
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Reviewed the phase 05 diff against `41062b2` at deep depth. I traced `cli::run` through `check::run`, `model::resolve` and `RepoModel::require`, the new git reads (`trees.rs`, `staleness.rs`, `Repo::hooks_path`), the exit classification and the JSON emitter and schema. I compared the check's judgement against `tests/oracle/check-workspace-divergence.sh` and the locked decisions D-01 to D-13 in `05-CONTEXT.md`. I found no blocker. The divergence evaluation matches the oracle: exact match for undeclared `modified` paths, and prefix coverage for stale `modified` entries, as `git diff --quiet -- <entry>` does. The JSON escaping and the exit-class mapping are sound, and the compile-time wildcard denial covers every variant.

The warnings fall into four groups:
- Two robustness gaps on the binary's contract: a panic exit (101) when stdout is closed, and a hard exit 6 caused by a symlink blob outside the hooks directory.
- A replace-ref guard comment that the new git children do not live up to.
- One impossible-state guard in `exit.rs`.
- An IF-02 scan that a split path literal evades.

Necessity was assessed against `necessity-review.md`.

## Warnings

### WR-01: Writing the verdict to a closed stdout panics with exit 101, outside the exit-code contract

**File:** `src/cli/mod.rs:37`, `src/cli/mod.rs:144-150`
**Issue:** The Rust runtime ignores SIGPIPE, so `println!` to a closed or broken stdout returns EPIPE and panics ("failed printing to stdout"), and the process exits 101. Both of these use `println!`:
- `Sinks::verdict` without `--json`, which runs on every `wsx check` run;
- `--version`.

Commands like `wsx check | head -c0`, `wsx check >&-` and a consumer that closes the pipe early all get exit 101 and a panic message. `docs/exit-codes.md` promises one of seven codes. The `--json` path handles exactly this failure (`Sinks::finish`, lines 159-166), so the two output paths are inconsistent. `eprintln!` in `Sinks::diagnostic` and `usage_error` has the same problem with a closed stderr.
**Fix:** Route every line through a fallible write, and map a failure to the documented class, as `finish` already does:
```rust
fn verdict(&self, line: &str) -> std::io::Result<()> {
    if self.json {
        writeln!(std::io::stderr().lock(), "{line}")
    } else {
        writeln!(std::io::stdout().lock(), "{line}")
    }
}
// caller: on Err, return ExitCode::from(ExitClass::GitFailed.code()) (or ignore EPIPE on stderr)
```

### WR-02: One missing symlink blob anywhere in the tree fails the whole check with exit 6

**File:** `src/check/mod.rs:193-200`, `src/git/trees.rs:145-152`
**Issue:** `check_hooks` collects every `120000` entry in the whole workspace-tip listing, not only those under the hooks directory or those a hook resolution reaches. It reads all of them in one `read_blobs` batch. `read_blobs` turns any `missing` answer into `Error::ObjectMissing`, and `--no-lazy-fetch` makes a partial clone's absent blob `missing`. A partial clone (`--filter=blob:none`) holds no blobs for workspace commits it never checked out. D-02 makes this case common: the check reads the workspace tip from a feature worktree. The result is that one unrelated symlink anywhere in the tree turns `wsx check` into exit 6 (`object_missing`), even when no hook is a symlink. The resolver already handles an absent link text: `splice` returns "the link text of … was not read" (`src/check/hooks.rs:217-222`). So the hard failure buys nothing.
**Fix:** Read the link texts so that a missing blob is absent from `targets` instead of an error. For example, add a `read_blobs_present` that returns `Vec<Option<Vec<u8>>>` and maps `BatchAnswer::Missing` to `None`. Alternatively, restrict `links` to the symlinks that resolution can reach. Then only a hook whose resolution needs the missing text fails, as a `hook_not_executable` problem.

### WR-03: The replace-ref guard does not cover the phase's new git children, against the comment's claim

**File:** `src/check/mod.rs:113-115`, `src/git/trees.rs:60-67`, `src/git/staleness.rs:27-36`
**Issue:** `check::run` says the model's replace-ref guard "stands for the reads by id after it (D-22)". The guard (`refuse_replace_refs`, called from `model/mod.rs:528`) checks the stores once. The reads after it come from two sources:
- `ls-tree`, `cat-file --batch-check` and `cat-file --batch` git children, spawned later, which apply any replace ref present when they run;
- gix's `tree_diff`, whose replacement map was loaded at open.

A replace ref created between the guard and those children makes the hook listing and the `only` lookups read replaced objects. The gix diff in the same run does not, so one report mixes two views of the same commit ids. `refs.rs:228-231` documents the race for gix's map. The new children make it wider, and `--no-replace-objects` closes it on the git side at no cost.
**Fix:** Prefix the new children's argv with git's global `--no-replace-objects`, which is valid because the guard has just shown that no replacement is in force:
```rust
const LIST_ARGS: [&str; 7] = ["--no-replace-objects", NO_LAZY_FETCH, "ls-tree", "-r", "-t", "-z", "--full-tree"];
// likewise LOOKUP_ARGS, BLOB_ARGS and REFLOG_ARGS
```
Alternatively, narrow the comment at `check/mod.rs:113-115` to what it guarantees.

### WR-04: Impossible-state guard: classifying a replace-ref refusal inside `ManifestUnreadable`

**File:** `src/exit.rs:99-101`, `src/exit.rs:114-125`, `docs/exit-codes.md:40`
**Category:** necessity
**Issue:** `classify` and `replacement_refusal` treat `ManifestUnreadable { source }` as a possible carrier of a replace-ref refusal, and the docs row for `manifest_unreadable` says "refused for a replace-ref refusal". That state cannot occur:
- The only constructor of `ManifestUnreadable` is the `unreadable` closure at `src/model/mod.rs:539-545`. It wraps the per-request results of `Repo::read_tips`, which yield parse errors (`GitBadOutput`) and `ObjectMissing` (`src/git/objects.rs:274-320`), never one of the three refusals.
- `refuse_replace_refs` runs before the batch (`src/model/mod.rs:528`) and returns its refusal as `Error::Git` through `?`.

Taking the branch at face value also shows an inconsistency it would introduce. A bare `ManifestUnreadable` carrying a refusal would get class 5 but kind `manifest_unreadable`, while the same refusal wrapped in `NotConfigured` gets its own kind. That contradicts `docs/exit-codes.md:61` ("keeps its own kind … wherever it is reported").
**Fix:** Either drop the `ManifestUnreadable` arm from `replacement_refusal` and classify `ManifestUnreadable` as `GitFailed`, fixing the doc row to match. Or, if the possibility is to be kept on purpose for a future reader, return `of_git(source)` for a refusal there too, so the kind is consistent with line 61.

### WR-05: The IF-02 scan misses a forbidden path built from separate literals

**File:** `tests/if02_scan.rs:37-46`, `tests/if02_scan.rs:142-152`
**Issue:** `.planning/config.json` is matched only as one literal. The usual way to build that path in Rust, `root.join(".planning").join("config.json")` (or `[".planning", "config.json"]`), produces two literals, and neither matches. `.planning` is not in `COMPONENTS`, so the IF-02 gate passes on a real read of the GSD config. `concat!` and an escaped spelling (`"STATE\x2emd"`, which `literal.to_string()` keeps escaped) get through the same way. The positive controls only plant the single-literal spelling, so the test suite never exercises this gap.
**Fix:** Add `.planning` to `COMPONENTS`, so a whole-component literal is a finding. Unescape each literal's value before matching, for example by parsing it with `syn::LitStr`/`LitByteStr` or by decoding the escapes. Add a planted control that uses `.join(".planning").join("config.json")`.

## Info

### IN-01: The JSON README contradicts itself on where the verdict goes

**File:** `docs/json/README.md:34-36`
**Issue:** One sentence says the verdict line goes to stderr "whether or not `--json` is given". The next says that without `--json` it goes to stdout, which is what `Sinks::verdict` does.
**Fix:** "With `--json`, everything for a person, the verdict included, goes to stderr. Without it, the verdict line goes to stdout and the rest to stderr."

### IN-02: `wsx --version <extra>` is reported as an unknown command

**File:** `src/cli/mod.rs:85-91`
**Issue:** `--version` followed by anything falls through to the command lookup, which prints `unknown command "--version"`. The message misnames the problem.
**Fix:** When `first == "--version"` and `rest` is not empty, return `plain("--version takes no argument")`.

### IN-03: `core.hooksPath` spellings that git accepts are refused with a misleading reason

**File:** `src/check/hooks.rs:58-68`
**Issue:** `normalise` uses the manifest's strict `normalize_path`, so `scripts/./hooks`, `scripts//hooks` and `scripts/hooks//` are all NotChecked. Each one is a relative directory inside the repository that git uses. The reason given is "it is not a relative path inside the repository", which is false for them, and the hook check is silently weaker.
**Fix:** Collapse `.` and empty components (and, optionally, `..` that stays inside the root) before calling `normalize_path`, or give a reason that names the actual refusal.

### IN-04: A gitlink under the hooks directory gets impossible advice

**File:** `src/check/hooks.rs:136`
**Issue:** A `160000` entry below the hooks directory is reported as "committed 160000; commit it with mode 100755". A submodule cannot be committed with that mode.
**Fix:** Add a separate detail for `MODE_GITLINK`, for example "a submodule, not a hook file".

### IN-05: Relative `core.hooksPath` in a bare repository is resolved from the tree root, not `$GIT_DIR`

**File:** `src/check/mod.rs:181-189`, `src/check/hooks.rs:54-68`
**Issue:** githooks(5) says hooks run with `$GIT_DIR` as cwd in a bare repository, so git resolves a relative `core.hooksPath` there against `$GIT_DIR`. The check always treats the value as a path from the tree root. In a bare repository it can therefore judge a directory git never runs hooks from. This is a guess based on git's documentation, not tested here. A bare-repo fixture with a relative `core.hooksPath` would confirm it.
**Fix:** In a bare repository (`repo.worktree()` is `None`), report the hooks as NotChecked with a reason that names the bare layout.

---

_Reviewed: 2026-11-19T01:07:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
