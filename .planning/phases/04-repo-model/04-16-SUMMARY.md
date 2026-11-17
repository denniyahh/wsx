---
phase: 04-repo-model
plan: 16
subsystem: git
tags: [git, cat-file, batch, stdin, partial-clone, model, tdd]

requires:
  - phase: 04-repo-model
    provides: "04-15: refs listed through one for-each-ref child, ids taken from the listing"
provides:
  - "Git::run_with_input: the one spawn-helper method with a non-null stdin, written whole and closed before waiting"
  - "Repo::read_tips: workspace tip, .wsx.conf entry with its mode, and integration tip from one git --no-lazy-fetch cat-file --batch child"
  - "Error::ObjectMissing naming the request git answered missing"
  - "RootEntry in src/git/objects.rs; Repo::root_entry and Error::TreeNotSorted deleted"
  - "testing::fake_git, fake_git_log, real_git, sh_quote, counting_git shared test helpers"
affects: [04-17, 04-18, phase-05-check, guard, cut]

actuals:
  tokens: 23945
  tasks: 3
  commits: 5
plan_head_before: 4e39bbd3af6e534c031d5a1c9aca4e5c3a3d4a50
plan_head_after: 2c8f3d716c9675e258883ee720fe4e2271571a34

tech-stack:
  added: []
  patterns:
    - "Batched object reads parsed positionally (answer i belongs to request i), exact byte counts, nothing left over"
    - "A missing batch answer is classified by scanning the root tree's raw entries; the scan picks an error variant only"
    - "Child-count tests through a PATH wrapper that logs every git argv (counting_git)"

key-files:
  created:
    - src/git/objects.rs
  modified:
    - src/git/spawn.rs
    - src/git/mod.rs
    - src/git/refs.rs
    - src/git/testing.rs
    - src/model/mod.rs

key-decisions:
  - "read_tips keeps the root-tree scan (research P3 recommendation): a missing .wsx.conf answer with an entry in the tree is ManifestUnreadable (ObjectMissing), with none ManifestMissing, a gitlink entry submodule"
  - "build raises the batch's three answers in the pre-batch order: batch failure, rung peel (both Error::Git), worktreeDir, workspace tip, manifest (ManifestUnreadable for either read)"
  - "Two require tests that pinned gix's error shape now expect git's ObjectMissing naming the request (rung <blob>^{commit}; manifest <tip>:.wsx.conf)"

patterns-established:
  - "One stdin-fed git child: Git::run_with_input; every other child keeps /dev/null stdin"
  - "Every model git child count is pinned by name and order through counting_git"

requirements-completed: [CONF-02, CONF-07]

coverage:
  - id: D1
    description: "Git::run_with_input feeds exactly its buffer and then EOF; Git::run keeps a null stdin"
    verification:
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::run_with_input_feeds_exactly_the_buffer_then_eof"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::children_get_null_stdin"
        status: pass
    human_judgment: false
  - id: D2
    description: "Tips and the manifest (kind from %(objectmode)) come from one cat-file batch; CONF-07 and D-19 model tests pass unchanged"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/git/objects.rs#git::objects::tests::parse_batch_reads_missing_and_sized_records"
        status: pass
      - kind: unit
        ref: "src/git/objects.rs#git::objects::tests::tree_scan_finds_the_mode_of_a_root_entry"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::symlink_manifest_fails_closed (and directory, submodule, missing, missing_blob, executable)"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::unsorted_tree_manifest_fails_closed"
        status: pass
    human_judgment: false
  - id: D3
    description: "Resolving a blob:none partial clone fails closed as ManifestUnreadable and fetches nothing; replacing NO_LAZY_FETCH makes the test fail"
    requirement: CONF-07
    verification:
      - kind: integration
        ref: "src/model/mod.rs#model::tests::partial_clone_manifest_is_unreadable_without_a_fetch"
        status: pass
      - kind: other
        ref: "scratch control: NO_LAZY_FETCH -> --literal-pathspecs, log target/gsd/04-16-no-lazy-fetch.log"
        status: pass
    human_judgment: false
  - id: D4
    description: "Errors keep today's order: a blob rung is reported before an invalid wsx.worktreeDir"
    verification:
      - kind: integration
        ref: "src/model/mod.rs#model::tests::blob_rung_is_reported_before_an_invalid_worktree_dir"
        status: pass
    human_judgment: false
  - id: D5
    description: "gix's manifest read deleted (root_entry, TreeNotSorted); entry tests ported to read_tips"
    requirement: CONF-02
    verification:
      - kind: unit
        ref: "src/git/objects.rs#git::objects::tests::{regular_file_entry_is_read_as_git_reads_it, ..., tip_read_stays_on_the_id_it_was_given} (9 tests)"
        status: pass
      - kind: other
        ref: "Task 2 verify 3: rg finds no root_entry/TreeNotSorted in src; both present at 4234f31"
        status: pass
    human_judgment: false
  - id: D6
    description: "One resolve spawns four git children in its own worktree, five aimed at another, three with no single candidate"
    verification:
      - kind: integration
        ref: "src/model/mod.rs#model::tests::resolve_spawns_four_git_children_in_its_own_worktree"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::resolve_aimed_at_another_worktree_adds_the_env_var_query"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::resolve_without_one_candidate_skips_the_object_read"
        status: pass
    human_judgment: false

duration: 19min
completed: 2026-11-17
status: complete
---

# Phase 4 Plan 16: Tips and manifest through one cat-file batch Summary

**The workspace tip, the `.wsx.conf` entry with its mode and the integration tip now come from one `git --no-lazy-fetch cat-file --batch` child fed through `Git::run_with_input`; gix's manifest read and `TreeNotSorted` are gone, and the child count is pinned at four.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-11-17T23:09:20Z
- **Completed:** 2026-11-17T23:28:29Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- `Git::run_with_input` writes its buffer from a scoped thread, drops the pipe and only then waits; it is the only child with a non-null stdin (D-33).
- `Repo::read_tips` sends `<ws>^{commit}`, `<ws>^{tree}`, `<ws>:.wsx.conf` and `<rung>^{commit}` in one batch; `parse_batch` reads answers positionally with exact byte counts; the entry kind comes from `%(objectmode)`; a `missing` entry is classified by scanning the root tree.
- `build` calls `read_tips` after `refuse_replace_refs` and the rung choice, and raises its answers in the pre-batch order; `integration` returns the rung unpeeled. The model calls no `peel_commit` and no `root_entry`.
- Partial clone: resolving fails closed as `ManifestUnreadable` and the blob is still not local afterwards (one `file://` blob:none fixture, git 2.56.1).
- Child counts measured on the host's git 2.56.1: four in wsx's own worktree, five aimed at another, three with two candidates.

## Task Commits

1. **Task 1 (tracer): the tips and the manifest from one batch** - `00fec42` (test, RED), `c305f2a` (feat, GREEN)
2. **Task 2: no lazy fetch, error order, gix read deleted** - `af43087` (test), `624319f` (refactor)
3. **Task 3: pin the child count** - `2c8f3d7` (test)

## TDD Gate Compliance

- **RED** (`00fec42`): `target/gsd/04-16-red-evidence.json` (JUnit, exit 100); `gsd_run check tdd-red-evidence ... --raw --pick verdict` printed `RED_EVIDENCE_OK`. Negative control: the same record with the failure removed and exit 0 gave `INVALID_RED`. Semantic assessment: `run_with_input_feeds_exactly_the_buffer_then_eof` ran, both parent-side `assert_ne` controls passed, the child exited 0 inside its 30 s bound, and the test failed on `the blob git hashed from run_with_input's stdin` (left `e69de29b...`, the empty blob; right `08015dfb...`, the id of `INPUT`), because the stub runs `Git::run` with a null stdin. Not a build, load or fixture failure.
- Other RED logs (`target/gsd/04-16-red/`), each ending `exit 100`:
  - `parse_batch_reads_missing_and_sized_records`: `assertion \`left == right\` failed`, left `Ok([])`
  - `parse_batch_refuses_a_short_read`: `expected Err for a short read, got Ok([])`
  - `tree_scan_finds_the_mode_of_a_root_entry`: `assertion \`left == right\` failed: .wsx.conf`, left `Ok(None)`
  - `unsorted_tree_manifest_fails_closed`: `resolve: expected Err(ManifestUnreadable(ObjectMissing)), got ManifestUnreadable { ..., source: TreeNotSorted { ... } }`
  - `object_missing_message_names_the_request` passed at RED, as planned.
- **GREEN** (`c305f2a`) descends from the RED commit; the Task 1 verify loop (29 tests) and `scripts/check.sh all` passed. Tracer gate (interactive, end-of-phase, automated verify only): verify re-run green before expanding.

## No-lazy-fetch control

With `NO_LAZY_FETCH` replaced by `--literal-pathspecs` in the checkout (restored afterwards; tree clean), `target/gsd/04-16-no-lazy-fetch.log` line 26:

`resolve: expected an error, got Configured(Model { workspace_branch: [119, ...], workspace_tip: ObjectId("ae1f62f7be11fae0816e8028c42dada8333ff6ce"), ...`

Run after `af43087` and again after `624319f`; both times the test failed with that text.

## Files Created/Modified

- `src/git/objects.rs` - `RootEntry`, `TipObjects`, `Repo::read_tips`, `parse_batch`, `root_tree_mode`, unit tests and the nine ported entry tests
- `src/git/spawn.rs` - `Git::run_with_input`, shared `exited_zero`, module doc on the one stdin exception, its test and ignored `input_child`
- `src/git/mod.rs` - `mod objects`, re-exports, `Error::ObjectMissing` and its message test; `TreeNotSorted` removed
- `src/git/refs.rs` - `RootEntry`, `root_entry` and their tests removed
- `src/git/testing.rs` - `real_git`, `sh_quote`, `fake_git`, `fake_git_log`, `counting_git`; `git::objects::tests::` in `DISCOVERING_SUITES`
- `src/model/mod.rs` - `build` on `read_tips`, `integration` unpeeled, `resolve` doc, partial-clone, order and child-count tests

## Decisions Made

- Kept the root-tree scan (research P3): without it a partial-clone user would be told there is no `.wsx.conf`.
- `read_tips` maps any unexpected answer shape to `GitBadOutput` instead of panicking (a wrong answer count included).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] RED verify pipes `--raw` into `rg -qx 'RED_EVIDENCE_OK'`**
- **Found during:** Task 1 (RED)
- **Issue:** `--raw` prints the whole JSON, so the plan's literal check can never match (confirmed: it exits 1 on a valid record).
- **Fix:** ran it with `--raw --pick verdict`; negative control (failure removed, exit 0) prints `INVALID_RED`.
- **Committed in:** n/a (verification only)

**2. [Rule 1 - Bug] Two `require` tests pinned gix's error shape**
- **Found during:** Task 1 (GREEN), `scripts/check.sh all`
- **Issue:** `require_passes_a_git_failure_through` expected `Error::Git(Gix { what: "peel <blob> to a commit" })`; `require_wraps_an_unreadable_manifest_as_not_configured` expected the rendering to name the deleted blob's id. The batch reports git's `missing` for the request.
- **Fix:** the first now expects `Error::Git(ObjectMissing { request: <blob>^{commit} })`; the second expects the rendering to name `<tip>:.wsx.conf`. Both keep their outer behaviour (passed through unwrapped; wrapped as not configured, with the restore control).
- **Files modified:** src/model/mod.rs
- **Committed in:** c305f2a

**3. [Rule 3 - Blocking] New discovering test module must be listed**
- **Found during:** Task 2
- **Issue:** the ported entry tests call `rerun_scrubbed()`, and `discovering_suites_are_the_modules_that_rerun_scrubbed` requires every such module in `DISCOVERING_SUITES`.
- **Fix:** added `git::objects::tests::` (src/git/testing.rs, outside Task 2's file list).
- **Committed in:** 624319f

---

**Total deviations:** 3 auto-fixed (1 bug, 2 blocking). **Impact:** no scope change; the two rewritten assertions follow D-37's precedent for `unsorted_tree_manifest_fails_closed`.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Every model input now comes from git children; ready for 04-17. Latency of the four children is 04-18's to measure.

---
*Phase: 04-repo-model*
*Completed: 2026-11-17*

## Self-Check: PASSED

- FOUND: src/git/objects.rs, src/git/spawn.rs, src/git/mod.rs, src/git/refs.rs, src/git/testing.rs, src/model/mod.rs
- FOUND: 00fec42, c305f2a, af43087, 624319f, 2c8f3d7 (ancestors of HEAD); GREEN c305f2a descends from RED 00fec42
