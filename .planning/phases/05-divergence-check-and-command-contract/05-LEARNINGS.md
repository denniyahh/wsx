---
phase: 5
phase_name: "Divergence check and command contract"
project: "wsx"
generated: "2026-11-19"
counts:
  decisions: 10
  lessons: 8
  patterns: 8
  surprises: 6
missing_artifacts:
  - "05-UAT.md"
---

# Phase 5 Learnings: Divergence check and command contract

## Decisions

### Exit codes are per class, not per error (D-06)
Seven classes (0 success, 1 problems found, 2 usage, 3 not configured, 4 invalid configuration, 5 repository refused, 6 git failed) apply to every command. The specific variant goes in the JSON `error.kind`.

**Rationale:** Codes per variant would freeze the internal error enum into the public contract, and the codes are costly to change once devflow branches on them.
**Source:** 05-CONTEXT.md (D-06)

---

### Every error variant is classified by an exhaustive match with no wildcard (D-07)
`exit::classify` and `of_git` list each `git::Error` and `model::Error` variant; replacement refusals take class 5 wherever they are wrapped. Wildcard lints are denied on the function.

**Rationale:** A new variant then fails to compile until it is classified. A planted `_` arm was shown to fail clippy with `match_wildcard_for_single_variants`.
**Source:** 05-CONTEXT.md (D-07), 05-01-SUMMARY.md, STATE.md (05-01)

---

### The check compares tip to tip and judges the workspace branch, not HEAD (D-01, D-02)
Two-dot comparison of the integration tip's tree with the workspace branch tip's tree; the working tree and index are never read. Run from a feature worktree, the check still judges the workspace branch.

**Rationale:** A merge-base comparison would pass a difference the oracle catches, which success criterion 1 forbids. Using the workspace branch rather than HEAD is a recorded difference from the oracle (`check::feature_worktree`).
**Source:** 05-CONTEXT.md (D-01, D-02), 05-04-SUMMARY.md

---

### Hook modes come from the workspace tip's tree, and a symlinked hook resolves in-tree (D-03)
`core.hooksPath` is git's effective value; every file under it must be `100755` in the tip's tree. A `120000` entry passes only if its target resolves inside the tree, following directory links under a 40-link budget, to a `100755` blob. `scripts/*.sh` outside the hooks directory is not judged.

**Rationale:** Reading the index (the oracle's `ls-files -s`) can pass a hook that is 100644 in the commit. The symlink and hop-limit rules were added by the owner at plan review.
**Source:** 05-CONTEXT.md (D-03), 05-06-SUMMARY.md

---

### Staleness is reported with two times and never fails the check (D-12, D-13)
`ref_moved_at` is the newest reflog entry of the remote-tracking ref; `fetched_at` is the newest `FETCH_HEAD` mtime across the main and every linked worktree. A local integration branch reports that staleness does not apply. The reads never reach the exit class.

**Rationale:** On git 2.56.1 a fresh clone writes no reflog entry and a no-op fetch writes `FETCH_HEAD` but no reflog entry, so reflog alone misreports. An offline check must not fail because the network is down.
**Source:** 05-CONTEXT.md (D-12, D-13), 05-03-SUMMARY.md, 05-07-SUMMARY.md

---

### With `--json`, stdout holds exactly one versioned object and non-UTF-8 bytes are `{"hex": ...}` (D-08, D-09)
Every object carries `command` and `schema_version`; a failure adds `error {kind, message, exit_code}`. Human text stays on stderr. A byte field is a string if valid UTF-8, else hex, always through `json::bytes`. The emitter is hand-written, with serde_json only as a dev-dependency to read it back.

**Rationale:** A path must never be converted lossily, and the emitter needs no runtime dependency.
**Source:** 05-CONTEXT.md (D-08, D-09), 05-02-SUMMARY.md

---

### A pinned schema file never changes; any change is a new version file with its own pin (D-10)
`docs/json/check.v1.schema.json` is pinned by blob id. A change is `<name>.v<N+1>.schema.json` with a new pin, old file and pin kept. A command without a schema file fails the contract test.

**Rationale:** This makes an edit without a version bump fail, as success criterion 4 requires.
**Source:** 05-08-SUMMARY.md, 05-CONTEXT.md (D-10)

---

### jsonschema 0.61.2 validates the output, with MIT-0 allowed in deny.toml
`scripts/check.sh deps` failed only on MIT-0 (`borrow-or-share 0.2.4`); the operator chose to allow it rather than fall back to boon. Removing the allowance made deps fail naming that crate.

**Rationale:** Operator decision of 2026-11-18, taken at a checkpoint.
**Source:** 05-08-SUMMARY.md, STATE.md (05-08)

---

### IF-02 is enforced by a token scan for spellings plus a runtime comparison for reads that reach output (05-05, 05-10)
The scan walks `src/` with proc-macro2, skips doc attributes, treats unlexable files as findings and has no allowlist. A runtime test requires `check` output to be identical across agent-file states. `.planning` was not added to the scan's components.

**Rationale:** A `.planning` component contradicts D-14 (`.planning` is path data) and would fail the real tree on 28 lines, fixable only by the allowlist D-14 forbids. The operator chose this split (option 1).
**Source:** 05-05-SUMMARY.md, 05-10-SUMMARY.md, 05-VERIFICATION.md, 05-REVIEW-FIX.md (WR-05)

---

### Each item no command reaches yet carries its own dead-code expectation naming its first caller's phase (05-09)
Twenty items carry `cfg_attr(not(test), expect(dead_code, reason = ...))` with the earliest ROADMAP phase needing them; module-level expectations came off `src/git/mod.rs` and `src/model/mod.rs`. Three mutants were allowlisted as equivalent.

**Rationale:** rustc reports an expectation left on a used item, so each phase's first caller must remove it. Every other mutation survivor got a behaviour test.
**Source:** 05-09-SUMMARY.md, STATE.md (05-09)

---

## Lessons

### A test control must discriminate, not just run
The plan's control for the global `core.hooksPath` test was vacuous because `read_config` only asks for `^(wsx|remote)\.` keys. It became `git config --local --get core.hooksPath` exiting 1. In 05-06 the plan's `../../x` example stayed inside the tree, so `../../../x` became the leave-the-tree case and `../../x` the passing boundary control.

**Context:** Review of plan-given controls during RED in 05-03 and 05-06; several resolver tests also gained a positive that must still trip.
**Source:** 05-03-SUMMARY.md, 05-06-SUMMARY.md

---

### A RED commit cannot be gated on the full check
`scripts/check.sh all` cannot pass when the target test fails by design, so RED commits were gated on fmt and clippy and GREEN commits on `all`. A title-only `docs/exit-codes.md` kept the RED a test failure instead of a build failure.

**Context:** Both were needed so RED evidence was a real assertion failure, as the classifier requires.
**Source:** 05-08-SUMMARY.md, 05-01-SUMMARY.md

---

### A sentinel that holds no key a reader acts on cannot detect a lenient reader
State A held only a `note` key, so a reader that defaults on parse failure gave the same output in A and B. The fix gave A real non-default keys, added a well-formed state C with different values, and two lenient-reader controls. The RED run against the old sentinels failed on the planned assertion.

**Context:** Found by 05-VERIFICATION reading the gap and by 05-REVIEW-GAP WR-02, not by a planted reader in `src/`.
**Source:** 05-REVIEW-GAP.md (WR-02), 05-11-SUMMARY.md, 05-VERIFICATION.md

---

### The scan's "the lexer refuses every malformed escape" invariant was false for CRLF continuations
proc-macro2 accepts a backslash followed by `\r\n`, so `unescape` panicked on valid CRLF source. The RED run panicked with `cannot decode the escape`, confirming the premise before the fix.

**Context:** Found in the gap review; `.gitattributes` forcing LF was only an optional suggestion.
**Source:** 05-REVIEW-GAP.md (WR-01), 05-11-SUMMARY.md

---

### Hard failures on one missing object turned a local fault into a whole-check failure
One missing symlink blob anywhere in the tree failed the check with exit 6. `read_blobs` now answers `None` for a missing id and only a hook whose resolution needs it becomes a problem.

**Context:** Review finding WR-02, fixed in ca829f2 with an exit 0 and an exit 1 contract test.
**Source:** 05-REVIEW-FIX.md (WR-02), 05-REVIEW-DISPOSITION.md

---

### Printing to stdout can panic outside the exit-code contract
`println!` on a closed stdout exited 101. `--version`, the verdict and the JSON object now go through one fallible `stdout_line` that exits 6; stderr writes are dropped on failure. 05-02 had handled only the JSON write and noted its path was untested.

**Context:** Review finding WR-01; both new tests failed with the panic before the fix.
**Source:** 05-REVIEW-FIX.md (WR-01), 05-02-SUMMARY.md

---

### The replace-ref guard did not cover the phase's new git children
The lookup, listing and blob-read children now lead with `--no-replace-objects`. `REFLOG_ARGS` was left alone because the reflog read prints only a selector and time, which a replace ref cannot change.

**Context:** Review finding WR-03; each new test read the replaced object before the fix.
**Source:** 05-REVIEW-FIX.md (WR-03)

---

### Review findings can be stale by the time they are triaged
WR-05 was skipped as already fixed: gap plan 05-10 landed after the review was written. Its other suggestion, `.planning` as a component, was declined as a locked-decision change for the operator. Six of ten findings remain open (WR-04, IN-01 to IN-05).

**Context:** The disposition table records the residue.
**Source:** 05-REVIEW-FIX.md (WR-05), 05-REVIEW-DISPOSITION.md

---

## Patterns

### Exhaustive classification with a planted-wildcard control
One arm per variant with `{ .. }`, wildcard lints denied, plus a copy of the source with `_ =>` that must fail clippy with the intended lint.

**When to use:** Any mapping from an enum that must stay total as variants are added.
**Source:** 05-01-SUMMARY.md

---

### Tree paths travel only on stdin as NUL-framed requests
`cat-file --batch-check -Z` takes `<id>:<path>` records; no path appears on argv or in a pathspec, so a newline path works. A lookup is `None` only when the answer is exactly `<its request> missing`.

**When to use:** Any git read keyed by a path that may hold odd bytes.
**Source:** 05-03-SUMMARY.md, 05-04-SUMMARY.md

---

### Assert git's own answer before wsx's
Each git-read test checks the oracle (batch-check text, rev-parse ids, `--git-path`, `reflog exists`) first; fixture steps assert git's view before the action runs.

**When to use:** Tests of reads that must match git by construction.
**Source:** 05-03-SUMMARY.md, 05-06-SUMMARY.md

---

### One sink value decides each line's stream
Diagnostics go to stderr, the verdict to stdout or stderr under `--json`, and the object goes to stdout once, last, in one write. Contract tests include the binary's own COMMANDS table via `#[path]`.

**When to use:** Every later command's output, so no command keeps a private copy of the table.
**Source:** 05-02-SUMMARY.md

---

### Source rules over `src/` are token scans, never text greps
Comments drop out, doc comments arrive as skipped `#[doc]` attributes, literals are decoded, and `concat!` and `.join` chains are matched by joined value with each argument still checked alone. Planted controls live under `CARGO_TARGET_TMPDIR`.

**When to use:** Any rule about what source may name.
**Source:** 05-05-SUMMARY.md, 05-10-SUMMARY.md

---

### A new command cannot pass the suite without its contract coverage
A COMMANDS entry needs an enabled no-agent-files run with a parity control, a `CommandId` arm in the schema test, a schema file and a pin.

**When to use:** Each later command plan.
**Source:** 05-05-SUMMARY.md, 05-08-SUMMARY.md

---

### Compare the same fixture across agent-file states
The runtime test runs `check` and `check --json` over states A, B and C, with raw-read and lenient-reader controls that each make the states differ.

**When to use:** Showing a command does not read a class of files, beyond what a scan can see.
**Source:** 05-10-SUMMARY.md, 05-11-SUMMARY.md

---

### Run a possibly endless walk under a deadline
`judge_within_deadline` runs the walk on a thread with `recv_timeout`, so a mutant that stops a hop count growing fails as an assertion instead of a 300 s cargo-mutants timeout.

**When to use:** Tests for loops that a mutant could make unbounded.
**Source:** 05-09-SUMMARY.md

---

## Surprises

### The oracle exits 0 on a replace-ref repository and on `only .worktrees/x`
For `replace_refs_refused` the oracle exits 0 because git applies the replace ref so the tips compare equal; for `redundant_builtin` it exits 0 on the built-in-covered entry. Both are recorded as such.

**Impact:** wsx deliberately differs from the oracle here, and the oracle side is recorded rather than asserted equal.
**Source:** 05-01-SUMMARY.md, 05-04-SUMMARY.md

---

### The harness observer refuses a committed symlink in a checkout
It records regular files only, so the hook-symlink fixture removes the link from the working tree after committing it; the commit and index keep it, which is all either side reads.

**Impact:** A fixture-only workaround that later symlink scenarios must repeat.
**Source:** 05-06-SUMMARY.md, STATE.md (05-06)

---

### `ExitClass::name` could not lose its dead-code expectation in 05-02
The failure object carries `kind` and `exit_code`, not the class name, so `--json` never reads it.

**Impact:** The 05-01 expectation was kept with a corrected reason.
**Source:** 05-02-SUMMARY.md

---

### The `splice` mutant read as a timeout though a test failed
A standalone deadline test failed at 10 s, but nextest waited for the two loop tests, which spun until cargo-mutants' 300 s limit.

**Impact:** Both loop tests were moved under the deadline helper.
**Source:** 05-09-SUMMARY.md

---

### The roadmap's phase 5 criteria were verified only after two gap rounds
05-VERIFICATION went to gaps_found twice on SC5 (scan missed `Path::new(".planning").join("config.json")` and escaped spellings, then a lenient read through a non-literal base), closed by 05-10 and 05-11. The phase ended with 11 plans and status passed, 5/5.

**Impact:** Plans 05-10 and 05-11 were gap-closure plans added after 05-09; one advisory residual remains.
**Source:** 05-VERIFICATION.md, 05-10-SUMMARY.md, 05-11-SUMMARY.md

---

### A lenient read of a key no state holds still escapes
A read of, for example, a `workflow.*` key of `.planning/config.json` gets its default in A, B and C and its `planning_dir.join("config.json")` spelling is clean in the scan. It follows from operator option 1, was established by reading the code and has no planted reader as evidence.

**Impact:** Would be narrowed by adding keys to the states or closed by a scan rule the operator ruled out.
**Source:** 05-VERIFICATION.md (advisory)
