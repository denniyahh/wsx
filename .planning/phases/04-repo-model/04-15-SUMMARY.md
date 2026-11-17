---
phase: 04-repo-model
plan: 15
subsystem: git
tags: [rust, git, for-each-ref, refs, gix, model]

requires:
  - phase: 04-repo-model
    provides: "04-14's ConfigRead from one git config child, and Git::run (null stdin, env removal, version check)"
provides:
  - "RefRecord, Repo::list_refs, parse_ref_listing and check_full_ref_name in src/git/refs.rs"
  - "ObjectId::from_hex and NO_LAZY_FETCH in src/git/mod.rs"
  - "testing::reexec_within, a re-executed test child killed after a limit"
  - "resolve takes one for-each-ref listing; build, integration and candidates read only exact-name records"
  - "Repo::find_ref, Repo::ref_commit, Repo::refs_directly_under, SYMREF_MAX_DEPTH and RefTarget deleted"
affects: [04-16, 04-17, 04-18, phase-05]

actuals:
  tokens: 27573
  tasks: 2
  commits: 3
plan_head_before: 473cb527643b0c9fc82d29986ef37ae5045c5fe2
plan_head_after: 882ef7c478ab39b7038e002a40fb8d58cbd6bbd0

tech-stack:
  added: []
  patterns:
    - "Refs are only listed (git --no-lazy-fetch for-each-ref --format=%(refname)%00%(objectname)%00%(symref)), never read by name; every decision compares a record's exact name"
    - "A rung name passes check_full_ref_name before it is listed and again at its turn, so a name git cannot use as a ref raises Error::Gix where find_ref did"
    - "A test whose subject could hang runs it in testing::reexec_within with a limit"

key-files:
  created: []
  modified:
    - src/git/refs.rs
    - src/git/mod.rs
    - src/git/repo.rs
    - src/git/testing.rs
    - src/model/mod.rs

key-decisions:
  - "04-15: require_passes_a_git_failure_through keeps its assertion with a new fixture (a rung holding a blob), since D-35 makes a corrupt candidate a skipped ref, not a listing failure"
  - "04-15: errors reading wsx.baseRemote or wsx.baseRef list no rung; build raises them at their turn, so error order is unchanged"

patterns-established:
  - "Listing superset: callers filter list_refs records by exact name (G1)"

requirements-completed: [CONF-03, CONF-05]

coverage:
  - id: D1
    description: "list_refs lists through one for-each-ref child and parses raw bytes; a malformed record is refused; no pattern spawns nothing"
    requirement: CONF-03
    verification:
      - kind: unit
        ref: "src/git/refs.rs#git::refs::tests::parse_ref_listing_reads_raw_bytes"
        status: pass
      - kind: unit
        ref: "src/git/refs.rs#git::refs::tests::parse_ref_listing_refuses_a_record_without_three_fields"
        status: pass
      - kind: unit
        ref: "src/git/refs.rs#git::refs::tests::list_refs_without_a_pattern_spawns_nothing"
        status: pass
    human_judgment: false
  - id: D2
    description: "The workspace branch, the rungs and the candidates come from that listing and match git on each review shape (S2-S4, S6, S8, F8, D-30, D-34, D-35), git asserted first, each with a control that moves both"
    requirement: CONF-05
    verification:
      - kind: integration
        ref: "scripts/check.sh test loop over the 43 tests in Task 1's second <verify>"
        status: pass
    human_judgment: false
  - id: D3
    description: "A fifo where a loose ref would be cannot hang resolve"
    requirement: CONF-05
    verification:
      - kind: integration
        ref: "src/model/mod.rs#model::tests::fifo_ref_is_skipped_without_hanging"
        status: pass
      - kind: unit
        ref: "src/git/testing.rs#git::testing::reexec_within_kills_a_child_that_outlives_its_limit"
        status: pass
    human_judgment: false
  - id: D4
    description: "gix's model ref reads are deleted, the listing tests ported to list_refs, the namespace control re-pointed"
    verification:
      - kind: other
        ref: "Task 2 first <verify> (deletion scan with controls on 4234f31)"
        status: pass
      - kind: integration
        ref: "Task 2 second <verify> loop (12 tests)"
        status: pass
    human_judgment: false
  - id: D5
    description: "require_passes_a_git_failure_through, outside the plan's rewrite list, re-pointed to a new fixture"
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::require_passes_a_git_failure_through"
        status: pass
    human_judgment: true
    rationale: "The plan prohibits changing a D-01 to D-21 test other than four named ones (verification: judgment); whether this fixture change is inside that prohibition is a judgment for the verifier or operator"

duration: 25min
completed: 2026-11-17
status: complete
---

# Phase 4 Plan 15: Refs read through one for-each-ref listing Summary

**The model's workspace branch, integration rungs and candidates now come from one `git --no-lazy-fetch for-each-ref` child per `resolve`, filtered by exact name, and gix's by-name ref reads (`find_ref`, `ref_commit`, `refs_directly_under`) are deleted.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-11-17T22:41:25Z
- **Completed:** 2026-11-17T23:06:30Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- `Repo::list_refs` runs `git --no-lazy-fetch for-each-ref --format=%(refname)%00%(objectname)%00%(symref) <patterns>` through `Git::run`; `parse_ref_listing` keeps raw bytes and refuses any record that is not three fields with a full hex id.
- `resolve` lists once: `refs/heads/<wsx.sourceBranch>` or the two candidate prefixes, plus every rung name that passes `check_full_ref_name`. `build`, `integration` and `candidates` read only records named exactly the ref they want.
- On this host's git 2.56.1 the model now matches git on each review shape: a linked worktree's private branch, a dangling symref and a corrupt ref are no candidate; a `refs/…` symlink and a `main-worktree/` alias read as missing; a dangling symbolic rung is absent; `<remote>/HEAD` is a default only when its `%(symref)` ends under the remote; a fifo is skipped within 10 s.
- `testing::reexec_within` kills a re-executed child past its limit, so a hang fails its test.

## Task Commits

1. **Task 1 RED: failing tests for refs read through for-each-ref** - `f83a63b` (test)
2. **Task 1 GREEN: read the model's refs through one for-each-ref listing** - `8bbeb42` (feat)
3. **Task 2: delete gix's model ref reads and test the git listing** - `882ef7c` (refactor)

## TDD Gate Compliance

- **RED** (`f83a63b`): `target/gsd/04-15-red-evidence.json`; `check tdd-red-evidence --raw --pick verdict` printed `RED_EVIDENCE_OK` (JUnit, exit 100). Negative control: the same record with the failure removed and exit 0 gave `INVALID_RED`. Semantic assessment: `private_branch_in_a_linked_worktree_is_not_a_candidate` ran, its git-first assertion (`for-each-ref refs/heads/workspace/*` from the linked worktree lists only `refs/heads/workspace/dev`) passed, and it failed at `inferred_model(&linked)` with `inferred: expected a model, got WorkspaceCandidates` listing `workspace/ghost`. Not a build, load or fixture failure.
- **GREEN** (`8bbeb42`): every test in the plan's loop passes; `scripts/check.sh all` ends `==> check.sh: all OK`.
- **REFACTOR**: Task 2 (`882ef7c`), gate green after it.
- Tracer gate (interactive, end-of-phase, automated-only verify): the second and third Task 1 `<verify>` commands were re-run after the GREEN commit and passed, so expansion went ahead.

RED outcome per new or rewritten test (each log under `target/gsd/04-15-red/`, last line the exit):

| Test | RED exit | What it met |
|------|----------|-------------|
| `private_branch_in_a_linked_worktree_is_not_a_candidate` | exit 100 | `inferred: expected a model, got WorkspaceCandidates` (dev, ghost) |
| `parse_ref_listing_reads_raw_bytes` | exit 100 | `assertion \`left == right\` failed`, left `Ok([])` |
| `parse_ref_listing_refuses_a_record_without_three_fields` | exit 100 | `expected Err for a two-field record, got Ok([])` |
| `list_refs_without_a_pattern_spawns_nothing` | exit 100 | its control: `list_refs with a pattern and no git: expected Err(Io), got Ok([])` against the stub |
| `reexec_within_kills_a_child_that_outlives_its_limit` | exit 0 | pin (`reexec_within` is real at RED) |
| `relative_symlinked_source_branch_resolves_as_git_does` | exit 0 | pin (gix already read through the relative symlink) |
| `git_style_symlinked_source_branch_reads_as_missing` | exit 0 | pin |
| `symlinked_remote_head_gives_no_integration_branch` | exit 0 | pin |
| `main_worktree_alias_source_branch_reads_as_missing` | exit 100 | `resolve: expected an error, got Configured(..)` |
| `dangling_symref_is_not_a_candidate` | exit 100 | `inferred: expected a model, got WorkspaceCandidates` |
| `dangling_symbolic_rung_is_absent` | exit 100 | `expected RepoModel::Configured, got Err(IntegrationRefSymbolic ..)` |
| `nested_ref_under_a_rung_name_does_not_satisfy_the_rung` | exit 0 | pin |
| `glob_bytes_in_a_remote_name_are_refused` | exit 0 | pin (`find_ref` already refused the name) |
| `fifo_ref_is_skipped_without_hanging` | exit 100 | `model::tests::state_child did not finish within 10s; killed it` |
| `corrupt_source_branch_ref_fails_closed` | exit 100 | `expected Err(WorkspaceBranchMissing), got Git(Gix { what: "read refs/heads/workspace/bad", .. })` |
| `remote_head_chain_into_a_local_branch_is_not_a_default` | exit 100 | `expected Err(NoIntegrationBranch), got IntegrationRefSymbolic` |
| `remote_head_chain_inside_the_remote_resolves` | exit 100 | `expected RepoModel::Configured, got Err(IntegrationRefSymbolic ..)` |
| `remote_head_cycle_is_not_a_default` | exit 100 | `expected Err(NoIntegrationBranch), got IntegrationRefSymbolic` |
| `corrupt_candidate_ref_is_skipped_as_git_skips_it` | exit 100 | `expected a model, got Git(Gix { what: "list the refs directly under refs/heads/workspace/" ..})` |

No git-side assertion failed at RED. The plan's first `<verify>` (with `--pick verdict`) exits 0; its control: a pin's log (`exit 0`) fails the `exit [1-9]` check.

## Files Created/Modified

- `src/git/refs.rs` - `RefRecord`, `check_full_ref_name`, `list_refs`, `parse_ref_listing`; gix ref reads deleted; listing tests ported
- `src/git/mod.rs` - `ObjectId::from_hex`, `NO_LAZY_FETCH`, exports; `RefsNamespace` doc
- `src/git/repo.rs` - namespace control on gix's own ref read; `refuse_refs_namespace` doc
- `src/git/testing.rs` - `reexec_within`, `sleeping_child` and its test; `spawn_child` shared with `reexec`
- `src/model/mod.rs` - `resolve` on one listing, `workspace_patterns`, `rung_names`, `listed_rungs`, `candidates`/`build`/`integration` on records; 11 new and 4 rewritten tests; `state_child`'s `inferred` marker

## Decisions Made

- An error reading `wsx.baseRemote` or `wsx.baseRef` lists no rung; `build` raises it at its turn, so error order is today's.
- `<remote>/HEAD` as a default: branch is the rest of its `%(symref)`, `full_ref` that target, tip HEAD's own `%(objectname)` peeled (D-34).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] RED verify pipes `--raw` into `rg -qx 'RED_EVIDENCE_OK'`**
- **Found during:** Task 1 RED
- **Issue:** `--raw` prints the whole JSON, so the line never matches (known from 04-13).
- **Fix:** ran it with `--raw --pick verdict`; control: a passing record gives `INVALID_RED`.
- **Commit:** none (verification only)

**2. [Rule 1 - Bug] `corrupt_source_branch_ref_fails_closed`'s control wrote `main`'s tip**
- **Found during:** Task 1 RED
- **Issue:** `main` has no `.wsx.conf` in `repo_with_remote`, so `configured(&work)` would fail with `ManifestMissing`, not succeed.
- **Fix:** the control writes `workspace/dev`'s tip.
- **Commit:** `f83a63b`

**3. [Rule 3 - Blocking] `RefTarget`'s export removed in the GREEN commit, not Task 2**
- **Found during:** Task 1 GREEN
- **Issue:** once the model stopped using it, the unused `pub use` failed `clippy -D warnings`, so the GREEN gate could not pass.
- **Fix:** dropped `RefTarget` from `src/git/mod.rs`'s `pub use` in `8bbeb42`; Task 2 deleted the type.
- **Commit:** `8bbeb42`

**4. [Rule 3 - Blocking] `require_passes_a_git_failure_through` lost its fixture to D-35**
- **Found during:** Task 1 GREEN (`scripts/check.sh all`; it also failed `discovering_tests_pass_with_a_runner_git_dir`)
- **Issue:** the 04-10 test made a corrupt candidate ref fail the listing to get a non-replacement `Error::Git` through `require`; D-35 makes that ref skipped, so `require` resolved. The plan's rewrite list did not name this test.
- **Fix:** same assertion (`require` returns the `Error::Git` unwrapped), new fixture: `refs/remotes/origin/develop` holds a blob, git first refuses `^{commit}`, and the error is `Gix { what: "peel <blob> to a commit" }`. Control: with `require`'s guard `inner.is_replacement_refusal()` mutated to `true` the test fails; the file was restored byte-identical (sha256 checked).
- **Files modified:** `src/model/mod.rs`
- **Commit:** `8bbeb42`
- This touches the plan's prohibition on changing D-01 to D-21 tests beyond four named ones; recorded as `human_judgment: true` (D5).

**5. [Rule 1 - Bug] `glob_bytes_in_a_remote_name_are_refused`'s git-first assertion**
- **Issue:** the glob `refs/remotes/o*/develop` also matches `origin/develop`, so git prints two names, not only `ox/develop`.
- **Fix:** the test asserts the listing contains `refs/remotes/ox/develop`.
- **Commit:** `f83a63b`

---

**Total deviations:** 5 auto-fixed (3 blocking, 2 bugs). **Impact:** no scope added; deviation 4 is a test-fixture change outside the plan's list.

## Issues Encountered

None beyond the deviations.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for 04-16 (`read_tips`, and `manifest_read_stays_on_the_commit_it_was_given` ported from `list_refs`). Each shape is pinned on one fixture on this host's git 2.56.1; the mutation pass covering this plan runs in 04-18.

---
*Phase: 04-repo-model*
*Completed: 2026-11-17*

## Self-Check: PASSED

- FOUND: src/git/refs.rs
- FOUND: src/git/mod.rs
- FOUND: src/git/repo.rs
- FOUND: src/git/testing.rs
- FOUND: src/model/mod.rs
- FOUND: f83a63b
- FOUND: 8bbeb42
- FOUND: 882ef7c
