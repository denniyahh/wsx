---
phase: 02-oracle-harness
reviewed: 2026-10-26T00:00:00Z
depth: deep
files_reviewed: 27
files_reviewed_list:
  - .config/nextest.toml
  - CONTRIBUTING.md
  - Cargo.toml
  - deny.toml
  - rust-toolchain.toml
  - scripts/check.sh
  - scripts/install-tools.sh
  - tests/harness/action.rs
  - tests/harness/check.rs
  - tests/harness/coverage.rs
  - tests/harness/cut.rs
  - tests/harness/env.rs
  - tests/harness/fixture.rs
  - tests/harness/guard.rs
  - tests/harness/hermeticity.rs
  - tests/harness/main.rs
  - tests/harness/oracle.rs
  - tests/harness/outcome.rs
  - tests/harness/provenance.rs
  - tests/harness/registry.rs
  - tests/harness/rust.rs
  - tests/harness/scenario.rs
  - tests/harness/signing.rs
  - tests/harness/sync.rs
  - tests/harness/worktree.rs
  - tests/oracle/README.md
  - tests/oracle/oracle.patch
findings:
  critical: 1
  warning: 6
  info: 6
  total: 13
status: issues_found
---

# Phase 2: Code Review Report

**Reviewed:** 2026-10-26
**Depth:** deep
**Files Reviewed:** 27
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

The harness is carefully built. The scenario rules, the `fixed:` runtime check
(`FixedPassesOnOracle`), the patch-applied guard and the provenance blobs all come with negative
controls, and they hold up under tracing. The defects are in what the **observer** can see and
in what a recorded outcome can **prove**. Both are false-green classes that later phases (5 to 10)
will inherit when they turn on the Rust sides:

- The observer never reads HEAD, the index or git config. "The cut never touches the user's
  checkout" (PRCUT-06) is therefore asserted only through files on disk. A cut that overwrites
  the user's index passes.
- An `Outcome` pins only the exit code and the repository state, never the reason. Every
  refusal, and every recorded `fixed:` defect whose oracle exits 1, matches any other failure
  with the same code.
- `GIT_CEILING_DIRECTORIES` does not protect git commands whose cwd is the scenario root
  itself. The fixture runs `init` and `clone` there. A leak of the enclosing repository's early
  config was demonstrated (see WR-02).

Limits of this review. No `scripts/check.sh` gate was run. The ceiling leak was reproduced in a
scratch probe, with a negative control, not through the harness itself. The other findings were
traced statically from the cited lines. Every Rust side is disabled today, so none of these
gaps produces a false pass in the current suite. They become false passes when phases 5 to 10
turn the Rust sides on.

## Critical Issues

### CR-01: The observer is blind to HEAD and the index, so "never touches the user's checkout" is not tested

**File:** `tests/harness/outcome.rs:162-181` (`observe`), `tests/harness/outcome.rs:190-199`
(`read_refs`: only `refs/heads`, `refs/remotes`, `refs/tags`); relied on at
`tests/harness/cut.rs:58-64`, `tests/harness/cut.rs:159-161` and `tests/harness/cut.rs:853-857`

**Issue:** A `Snapshot` contains refs, commits and the files on disk in each checkout. It never
reads:
- any checkout's `HEAD` (which branch is checked out, or whether HEAD is detached);
- the index (staged content);
- in-progress operation state (`CHERRY_PICK_HEAD`, `MERGE_HEAD`).

The cut scenarios cite PRCUT-06 ("the cut never touches the user's checkout") as the source of
`FilesExpect::Unchanged` (cut.rs:58-59, 161). The recorded evidence also makes claims that the
harness never checks:
- cut.rs:61-64: "returns to `feature/phase-1` before it exits";
- cut.rs:855-856: "The source checkout is back on `feature/phase-1`".

Two concrete future false greens:
- A wsx cut that builds the PR tree through plumbing and forgets a private `GIT_INDEX_FILE`
  overwrites the user's index with the PR tree. The files on disk and the refs are unchanged, so
  every cut scenario passes, while `git status` in the user's checkout shows staged deletions of
  every `.planning` file.
- A wsx cut, sync or new-worktree that leaves the source checkout detached at the same commit
  is also invisible. The files, refs and commits are identical.

This is the harness's core contract. It is the acceptance gate for phase 9 (`cut`), and it
passes the exact regression PRCUT-06 exists to prevent.

**Fix:** Record per-checkout HEAD state, the index and pending-operation markers in `Snapshot`,
and compare them under `FilesExpect::Unchanged`, or under a new `CheckoutExpect::Unchanged`:
```rust
// in read_files / per checkout:
let head = fixture.git_ok(&name, ["symbolic-ref", "-q", "HEAD"])          // branch, or
    .or_else(|_| fixture.git_ok(&name, ["rev-parse", "HEAD"]))?;          // detached id
let index = fixture.git_ok(&name, ["ls-files", "-s", "-z"])?;            // staged entries
let pending = ["CHERRY_PICK_HEAD", "MERGE_HEAD", "REVERT_HEAD", "REBASE_HEAD"]
    .iter().filter(|r| git rev-parse --git-path <r> exists).collect();
```
Add a negative control: a scenario whose action is `git read-tree <other>` in the checkout
must fail `FilesExpect::Unchanged`.

## Warnings

### WR-01: Refusals and recorded `fixed:` defects pin only the exit code, never the reason

**File:** `tests/harness/outcome.rs:31-38` (`Outcome` has only `exit`, `refs`, `new_commits`,
`files`). Examples: `tests/harness/guard.rs:405-407`, `tests/harness/check.rs:184-185`,
`tests/harness/check.rs:212-216`

**Issue:** Every oracle guard failure exits 1. That includes "base ref not found", an
unreadable `FORBIDDEN_PATHS`, a failed `git log` and the policy refusal itself. A recorded
`verdict(1)` therefore carries no evidence for the defect text beside it. Examples:
- `FORK_TAIL_DEFECT` ("base ref 'origin/develop' not found", guard.rs:405) is matched by any
  exit-1 refusal.
- `QUOTED_NAMES_DEFECT` (check.rs:91-93) says the oracle refuses "as undeclared". The record
  would also match a refusal caused by the newline in a name breaking the `while IFS=$'\t' read`
  loop.

These records are meant to survive the oracle's deletion as "the evidence of the defect"
(scenario.rs:50-52), but they cannot discriminate the defect.

`check::no_gsd_repo` shows the same weakness in a control. Its known-positive (check.rs:216,
undeclared `scratch/y.md`) runs in a repository whose subject is recorded as refused by the
oracle anyway (check.rs:212). On the oracle side that control therefore passes whether or not
the planted file exists.

On the Rust side, any wsx error path that exits 1 (a missing `.wsx.conf`, an unreadable
config) satisfies every known-positive and every refusal expectation in the suite.

**Fix:** Add an optional `stderr: Option<&'static [u8]>` field to `Outcome`, a required
substring that `compare` checks byte-exact. Set it on every recorded refusal and every
known-positive control, at least for the oracle side. For the no-GSD check, add a known-positive
whose oracle verdict differs from the subject's, or document that the control does not
discriminate on the oracle side.

### WR-02: `GIT_CEILING_DIRECTORIES` does not cover git run with cwd equal to the scenario root

**File:** `tests/harness/env.rs:126`. Affected callers: `tests/harness/fixture.rs:224`
(`init` at `"."`), `fixture.rs:241-251` (`clone` at `"."`), `fixture.rs:398-409`
(`bare_remote`, run at `"."`), `fixture.rs:558-567` (`advance_remote`'s `clone`). Also
`tests/harness/hermeticity.rs:595` and `tests/harness/coverage.rs:28`. False claim at
`tests/harness/main.rs:14-15`.

**Issue:** git applies a ceiling only to directories that are proper ancestors of the cwd. With
cwd equal to `dirs.root`, discovery continues upward through `target/tmp` into the wsx checkout.

Verified in a scratch probe:
- In `$P/outer/root`, with `GIT_CEILING_DIRECTORIES=$P/outer/root`, running
  `git rev-parse --show-toplevel` printed the enclosing repository.
- An alias defined in the enclosing repository's local config ran from the ceiling directory.
- One level below the ceiling, the same alias was "not a git command" (the negative control).
- Run from `target/tmp/ceilprobe` inside this worktree, `git rev-parse --show-toplevel` returned
  `/var/home/denniyahh/Github/wsx/.worktrees/phase-2`.

`init.defaultBranch` and `init.templateDir` did not leak. The leak is limited to early config:
`alias.*`, `pager.*`, `core.pager` and the like. None of these is set in the current wsx local
config, so no impact today. The module doc's claim "nothing discovers the wsx checkout around
`target/`" is still false.

**Fix:** Put the ceiling at the root's parent, so the root itself is below it:
```rust
pair("GIT_CEILING_DIRECTORIES", dirs.root.parent().expect("root has a parent").as_os_str()),
```
Alternatively, run the root-level `init` and `clone` with cwd set to a dedicated child
directory. Add a test that runs `git rev-parse --show-toplevel` with cwd `dirs.root` and expects
"not a git repository".

### WR-03: The observer ignores remote repositories, other ref namespaces and git config

**File:** `tests/harness/outcome.rs:190-199`, `tests/harness/outcome.rs:229-247`

**Issue:** Only the `work` repository's `refs/heads`, `refs/remotes` and `refs/tags` are read.
The new-commit walk starts from those tips only. Changes that go unobserved:
- a write to a bare remote (`remotes/*.git`) that does not also update a tracking ref, such as a
  push by URL or a push with an explicit refspec;
- any ref under `refs/stash`, `refs/notes` or a custom backup namespace (`refs/wsx/...`);
- commits reachable only from those refs or from a detached worktree HEAD;
- local config changes, such as `branch.<x>.merge` or upstream settings written by a
  new-worktree command.

A wsx refusal that "leaves refs unchanged" but writes `refs/wsx/backup`, or pushes to `origin`
by URL, passes `RefsExpect::Unchanged` with `new_commits: []`.

**Fix:** Read `for-each-ref` with no namespace filter, excluding only per-worktree `HEAD`s, which
CR-01's HEAD record covers. Snapshot each `remotes/*.git` repository's refs the same way.
Snapshot `git config --local --list -z`. Under the `Unchanged` variants, a diff in any of these
then becomes a mismatch.

### WR-04: The `CARGO_LLVM_COV_SETUP=no` fail-closed guarantee is claimed but never enforced

**File:** `rust-toolchain.toml:12-16`, `scripts/install-tools.sh:4-6`, `CONTRIBUTING.md:29-31`

**Issue:** Both comments say that coverage "is always invoked with CARGO_LLVM_COV_SETUP=no".
Nothing in the repository sets it: no script, test or workflow does
(`rg CARGO_LLVM_COV_SETUP` matches only these two comments). The CONTRIBUTING instructions for
rerunning the coverage proof do not mention it. A rerun on a machine that lacks `llvm-tools`
therefore lets cargo-llvm-cov install the component silently, which the locked decision
forbids.

The locked decision also says the absence of `cargo-llvm-cov` is "a hard failure, as for the
other tools". CONTRIBUTING.md:29 calls it "optional for contributors", and no gate verifies it.
That deviation is reasonable while no gate runs coverage, but it is not recorded as a decision
change.

**Fix:** Add a coverage command that sets the variable itself, for example
`scripts/check.sh coverage` running
`CARGO_LLVM_COV_SETUP=no cargo llvm-cov nextest ...` after
`install-tools.sh verify-cargo cargo-llvm-cov`. Then reword the two comments to point at that
command. Record the "optional" deviation in CONTEXT, or make the tool a hard requirement of
that target.

### WR-05: `spawn_and_wait` turns a child that exits early into an adapter error, and can deadlock

**File:** `tests/harness/oracle.rs:155-165`

**Issue:** stdin is written in full with `write_all` before `wait_with_output` starts draining
stdout and stderr. This has two consequences:
1. **The run is lost.** If the child exits without reading stdin, for example a future wsx
   `hook pre-push` that rejects its arguments, `write_all` returns `BrokenPipe` (std ignores
   SIGPIPE). The case is then reported as "the Rust adapter produced no run". The exit code and
   stderr, which the report would otherwise show, are lost. Whether this happens depends on
   timing, so it is flaky.
2. **The run can hang.** A child that fills the stdout pipe (64 KiB) before reading a stdin
   larger than the pipe buffer deadlocks. No timeout bounds it outside CI.

**Fix:** Write stdin from a separate thread, or accept `ErrorKind::BrokenPipe` from `write_all`
and continue to `wait_with_output`:
```rust
if let Err(err) = pipe.write_all(input) {
    if err.kind() != std::io::ErrorKind::BrokenPipe { return Err(fail("write stdin to", err)); }
}
drop(pipe);
```

### WR-06: `check.sh fmt` now depends on git, which the script header says it never does

**File:** `scripts/check.sh:14`, `scripts/check.sh:125`

**Issue:** Line 14 says "so the gate never depends on git". Line 125 now runs
`git ls-files -z -- '*.rs' | xargs -0 rustfmt ...`.
- **Outside a git work tree** (a `git archive` snapshot, which the external-review lanes use), it
  fails with git's "not a git repository". There is no `require_tool git` and no explanation.
- **A tracked `.rs` file deleted but not yet staged** is still listed, so rustfmt fails with
  "No such file".
- **An untracked new harness module** declared inside the `scenarios!` / `harness_modules!`
  macros is checked by neither `cargo fmt` nor this step until it is added to git.

**Fix:** Correct the header comment, and call `require_tool git` in `run_fmt`. Pass
`--cached --others --exclude-standard` and filter to existing files:
`git ls-files -z --cached --others --exclude-standard -- '*.rs' | xargs -0 -r ...`. That drops
deleted entries, or the files can be re-checked with `[ -e ]`.

## Info

### IN-01: The mode check reads the index, not the files the oracle copy executes

**File:** `tests/harness/provenance.rs:551-567`, `tests/harness/oracle.rs:104-121`

**Issue:** `vendored_files_have_their_recorded_modes` reads `git ls-files -s`, which is the
index. `copy_tree` copies whatever permissions the files have on disk. The patched scripts run
`"${WSX_ORACLE_DIR}/sync-workspace.sh"` and `.../check-workspace-divergence.sh` directly, and
`phase-worktree.sh` tests them with `-x`. A checkout with lost executable bits, such as one
with `core.fileMode=false`, therefore passes the mode test. The oracle then silently skips the
sync ("not found; skipping sync") or fails with exit 126. The doc comment at
provenance.rs:571-573 implies coverage that the test does not give.

**Fix:** Have `OracleCopy::prepare` assert `mode & 0o111 != 0` on each copied script.

### IN-02: `is_refusal` ignores files, and a `regression:` case may leave its oracle side at the expected outcome

**File:** `tests/harness/scenario.rs:349-351`, `tests/harness/scenario.rs:372-376`

**Issue:** A "refusal" may expect files on disk to change. A `Regression` case with
`OracleSide::SameAsExpected` validates, but it is then indistinguishable from parity. No
registered scenario uses `Regression` yet.

**Fix:** Require `outcome.files == FilesExpect::Unchanged` in `is_refusal`, or document why it
is not required.

### IN-03: Pre-push input takes the first `ls-remote` line, which can be a different ref

**File:** `tests/harness/action.rs:99-104`

**Issue:** `git ls-remote <remote> <pattern>` matches by trailing path components and can return
several lines. For example, `dev` matches `refs/heads/workspace/dev`. The code takes the first
sha. All current callers pass full ref names, so the input is correct today.

**Fix:** Pass `--refs` and select the line whose second field equals `remote_ref` exactly.

### IN-04: `CommitExpect` omits the committer date

**File:** `tests/harness/outcome.rs:66-82`

**Issue:** A replay or merge that ignores `GIT_COMMITTER_DATE` (for example, a Rust
implementation that stamps the wall clock) is not caught. Nondeterministic ids are invisible by
design, but the field itself is never compared.

**Fix:** Add `committer_date` to `CommitExpect` and compare it.

### IN-05: The nextest override can silently match nothing, and host runs have no timeout

**File:** `.config/nextest.toml:18-19`

**Issue:**
- **Override.** If `registry::rust_sides_not_enabled` is renamed, the `success-output` override
  matches nothing without any error, and the disabled-side report disappears from every run.
- **Timeout.** There is no `slow-timeout = { period, terminate-after }`. CI has job timeouts;
  a local run with a hung child does not.

**Fix:** Add a test that asserts the override's test name exists in the registry module, or rely
on `--no-tests=fail` in a dedicated invocation. Add
`slow-timeout = { period = "60s", terminate-after = 5 }`.

### IN-06: The coverage tests are vacuous outside a coverage run, and control B is not the decided control

**File:** `tests/harness/coverage.rs:72-117`

**Issue:** In an ordinary run the binary is not instrumented, so both assertions about the
absence of `.profraw` files hold trivially. The locked decision's negative control is "the same
line is uncovered when the binary is not spawned". Control B instead drops the `LLVM_*`
passthrough. This is documented in the module header, but the weaker control is not recorded as
a deviation.

**Fix:** Record the substitution in CONTEXT or SUMMARY, or add the decided control to the
one-time proof procedure.

---

_Reviewed: 2026-10-26_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
