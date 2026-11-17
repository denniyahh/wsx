---
phase: 03
phase_name: "Git adapter"
project: "wsx"
generated: "2026-11-17"
counts:
  decisions: 8
  lessons: 5
  patterns: 4
  surprises: 4
missing_artifacts:
  - "03-UAT.md"
---

# Phase 03 Learnings: Git adapter

## Decisions

### Read-only git questions answered by gix; hooks and signing by the git binary
The git layer runs the real `git` for commits and merges with the snapshot environment unchanged, and reads merge-base, ahead/behind, tree diff and range walk through gix, each pinned to git as oracle.

**Rationale:** Hooks, signing, merge drivers and credentials are reached only by running git itself; reads are cheap and checkable against git's own answer.
**Source:** 03-CONTEXT.md, 03-VERIFICATION.md

### `ahead_behind` is built on `range_walk`
Two walks, one per direction, counted by set size, instead of a separate counting walk.

**Rationale:** The two reads cannot disagree.
**Source:** 03-03-SUMMARY.md

### Tree-diff record handling follows gix-diff's emission rules
A record whose entry mode is a tree is dropped; `TypeChanged` folds `BlobExecutable` into `Blob`, so an executable-bit change alone is `Modified`.

**Rationale:** gix-diff 0.70.0 emits a Modification only when both sides are trees or both are not, so a file replaced by a directory arrives as Deletion + Addition; git's diff-tree reports a mode-only change as modified.
**Source:** 03-03-SUMMARY.md

### Ban gix's `*_with_environment_overrides*` entry points in clippy.toml; one sanctioned open site
Each ban reason names the sanctioned route (`Repo::discover` / `open_gix`, siblings through `main_repo()`/`worktrees()`). The sanctioned `discover_with_environment_overrides_opts` is itself on the ban list, so its `expect` holds only while the ban does and a second call fails the gate.

**Rationale:** A signature alone does not stop a second open path; the gate does.
**Source:** 03-04-SUMMARY.md

### Fail closed on divergences from git instead of guessing
Added refusals: `RelativeRepoEnv`, `ObjectStoreEnv` (`GIT_OBJECT_DIRECTORY`, `GIT_ALTERNATE_OBJECT_DIRECTORIES`, `GIT_COMMON_DIR`, any value), `ImplicitWorkTree`, `StaleWorktree`, `GitEmptyOutput`; an unparseable git version is refused. `SUDO_UID` is honoured only as root, as git does.

**Rationale:** gix ignores the three object-store variables, so wsx would read a different store than git; the cost is refusing inside receive-pack hooks (owner told 2026-10-27). Phase 5 assigns exit codes.
**Source:** 03-CONTEXT.md, 03-REVIEW-FIX.md (CR-01, WR-01), 03-05-SUMMARY.md, 03-07-SUMMARY.md

### Recorded divergences in the safe direction
`GIT_CEILING_DIRECTORIES` gets a post-discovery check matching git; `safe.directory` given only via `-c`/`GIT_CONFIG_COUNT` is refused where git would open (gix reads it only from global and system config); the foreign-owner message prints git's own fix line.

**Rationale:** Research ran gix 0.90.0 next to git 2.56.0 and found the differences; wsx errs on the strict side.
**Source:** 03-CONTEXT.md

### `extensions.refStorage` is read from the repository's local config only
Ignored in global config, `config.worktree` and `GIT_CONFIG_COUNT`.

**Rationale:** All three were measured: git 2.56.0 ignores the key there.
**Source:** 03-10-SUMMARY.md

### Test hermeticity is fixed in the tests, not in scripts/check.sh
`rerun_scrubbed` re-runs a dirty-environment test as a child without the variables `git rev-parse --local-env-vars` lists; production discovery still honours `GIT_DIR`.

**Rationale:** A direct `cargo nextest run` is covered too.
**Source:** 03-10-SUMMARY.md, deferred-items.md

---

## Lessons

### Plan controls and checks must be run against the real fixture before being trusted
03-01's merge-base control (`main/feat~3` differs from `main/feat`) failed on a correct implementation because both give F1; the "no gix-protocol/gix-transport" check cannot pass since gix 0.90.0 depends on them unconditionally. Both were corrected to their intent (a pair with base C1; no network client crate and only an empty `gix-transport` default feature).

**Context:** Each correction kept a negative control (constant-F1 mutant; a scratch copy with `blocking-network-client` failing on `gix-credentials`).
**Source:** 03-01-SUMMARY.md

### A planned ordering proof can pass without the code under test
The sibling-order test stayed green with the sort deleted, because gix's own order equalled path order in that fixture; a new test with a worktree sorting before `main` went red without the sort.

**Context:** Found by deleting the sort during 03-08 GREEN.
**Source:** 03-08-SUMMARY.md

### The suite went red whenever the runner exported `GIT_DIR`
`Repo::discover` honours the process `GIT_DIR` by design, so tests that discover from a scratch path opened the caller's repository (17 of 207, later 24 of 244 failing). A false red, never a false green; resolved in 03-10.

**Context:** Found in 03-04, recounted in 03-07 and 03-08, fixed in 03-10 with a pin test that fails (38 passed, 1 failed) when the guard is dropped.
**Source:** deferred-items.md, 03-04-SUMMARY.md, 03-08-SUMMARY.md, 03-10-SUMMARY.md

### Code review found worktree and object-store divergences that tests built from happy paths missed
A `GIT_DIR` that gix opens as bare but git gives the current directory as worktree was accepted (CR-01); gix ignores `GIT_COMMON_DIR`, so wsx and git read different stores (WR-01); the install hint was wrong when `$CARGO_HOME/bin` resolved first (WR-02). Fixed over three review iterations, each with a before-fix failing test and a control.

**Context:** git's rules were measured with git 2.56.0 before relying on them; `GIT_TEST_ASSUME_DIFFERENT_OWNER=1` showed git checks no ownership with an explicit `GIT_DIR`.
**Source:** 03-REVIEW-FIX.md, 03-CONTEXT.md

### Test scope is the case built, not the class
Hooks are tested in `.git/hooks` and signing in local config only; merge drivers and credentials have no test; read parity holds on one criss-cross fixture; the root `chown` path ran only in the rootless container; `SUDO_UID` proves one direction per environment.

**Context:** Stated by the verifier as the limits of the passing gates.
**Source:** 03-VERIFICATION.md

---

## Patterns

### Oracle-pinned parity test with a negative control
Assert git's answer first, then wsx's, and add a control that must differ (a pair with a different base; a twin repository differing only in the backend; a repository without the variable).

**When to use:** Any read or refusal whose correctness is "what git does".
**Source:** 03-01-SUMMARY.md, 03-06-SUMMARY.md, 03-REVIEW-FIX.md

### Mutation controls reverted and compared byte for byte
Plant a targeted mutant, confirm the intended test fails, revert, and compare the file with the gated version.

**When to use:** After GREEN, to show which tests catch which regression.
**Source:** 03-03-SUMMARY.md

### Re-exec a test as a child with a scratch environment
`reexec` starts from `env_clear()`; `rerun_scrubbed` strips repository-local variables; shared hook fixtures live in `testing.rs`. `marker()` matches a child's fact line anywhere in a line, and a failed stdin write is an error, not an unfed child.

**When to use:** Tests whose behaviour depends on the process environment, hooks or ownership.
**Source:** 03-07-SUMMARY.md, 03-08-SUMMARY.md, 03-10-SUMMARY.md

### Clippy disallowed-methods ban pinned by a test, proven with planted and misspelled controls
Pin each banned call under `#[expect(clippy::disallowed_methods)]`, count with a multiline regex plus an independent count, and run the clippy controls as `bash -c` in the background when the tool ceiling is below the plan's timeout.

**When to use:** Any new banned-call group.
**Source:** 03-04-SUMMARY.md

---

## Surprises

### The reftable oracle record was exit 0, as research predicted
The harness observed `verdict(0)` ("every difference from origin/develop is declared."); a first control with no stderr pin was rejected by `validate` before the oracle ran, so it tested nothing.

**Impact:** The transcription branch of the plan did not trigger; the control was redone with `.oracle_stderr(b"reftable")` and failed on the run itself.
**Source:** 03-02-SUMMARY.md

### An empty `--local-env-vars` list had no accurate error
`GitFailed` renders "exited unsuccessfully", so a zero-exit empty list needed a new `Error::GitEmptyOutput`.

**Impact:** One more refusal variant for Phase 5 to give an exit code.
**Source:** 03-07-SUMMARY.md

### rustfmt broke one-line acceptance greps
Mandated reason strings pushed attributes past 100 columns, so single-line regexes for `#[expect(` and the dead-code expectation matched nothing; the checks moved to multiline and the dead-code reason was shortened to "no caller until phase 5".

**Impact:** Plan criteria adjusted; no code change beyond wording.
**Source:** 03-04-SUMMARY.md, 03-01-SUMMARY.md

### Sourcing the GSD resolver reference file as a script started a stray shell
It started an interactive `bash` (PID 1077263), stopped by PID; it wrote nothing to the repository.

**Impact:** None on the repo; `gsd-tools.cjs` was then called directly with `node`.
**Source:** 03-03-SUMMARY.md
