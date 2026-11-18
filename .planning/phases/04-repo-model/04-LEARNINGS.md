---
phase: 4
phase_name: "Repo model"
project: "wsx"
generated: "2026-11-18"
counts:
  decisions: 11
  lessons: 8
  patterns: 8
  surprises: 8
missing_artifacts: []
---

# Phase 4 Learnings: Repo model

## Decisions

### The model reads its inputs through the git binary, not gix (D-23)
Ref resolution, the workspace-candidate listing, the `.wsx.conf` blob at the tip, the `wsx.*` keys and the remote list go through the existing spawn helper. gix keeps object work by commit id for later phases. This amends Phase 3's "reads through gix" for these inputs only; `04-13-PLAN.md` as first written was superseded.

**Rationale:** Four review rounds each found another way gix's ref layer differs from git's, and CONF-07 requires the bytes git reads, so the inputs match git by construction instead of by refusing shapes one by one.
**Source:** 04-CONTEXT.md (D-23), STATE.md (04-13 to 04-17)

### Code that only makes gix agree with git is deleted, not kept as a backstop (D-25)
Refusals that state a product rule stay: D-22's replace-ref stop, the refs-namespace refusal (04-05) and D-18.

**Rationale:** Once git does the read, the agreement checks guard nothing; the product-rule refusals still do.
**Source:** 04-CONTEXT.md (D-25)

### One `resolve` spawns a fixed small number of git children (D-26, D-31)
At most 4 children aimed at wsx's own worktree and `resolve` p95 under 100 ms; aimed at another worktree the spawn helper adds one `rev-parse --local-env-vars`, once per process. Measured: p95 10.8 to 14.0 ms over 200 calls in each R3 repo, four git children each.

**Rationale:** Batched reads (one config read, one ref listing, one `cat-file --batch`) instead of a child per ref keep cost independent of repo size.
**Source:** 04-CONTEXT.md (D-26, D-31), STATE.md (04-18)

### Refs are read through `git for-each-ref`, never by name (D-29)
`wsx.sourceBranch`, the integration rungs and the workspace candidates are listed, not looked up with `rev-parse` or `cat-file --batch-check`. A fifo loose ref is skipped, so the guard never hangs a push; a ref git reaches only by name (a filesystem-symlink ref) reads as absent, and a missing `wsx.sourceBranch` stays a loud error.

**Rationale:** Chosen over by-name reads (match `rev-parse` but hang on a fifo) and by-name reads with a timeout (one arbitrary limit).
**Source:** 04-CONTEXT.md (D-29)

### `.wsx.conf` is read with one `git cat-file --batch` whose stdin is written and closed at once (D-33)
This amends Phase 3's null stdin for that one spawn-helper method; `Git::run` keeps a null stdin.

**Rationale:** Chosen over null stdin with `ls-tree` plus `cat-file blob`, which costs a fifth child.
**Source:** 04-CONTEXT.md (D-33)

### `GIT_CONFIG` is dropped for the config child only (D-38)
`git config` then reads the repository's own config as every other git command does.

**Rationale:** Chosen over refusing to run while it is set, and over accepting that a configured repo then reads as not configured.
**Source:** 04-CONTEXT.md (D-38), STATE.md (04-17), 04-17-SUMMARY.md

### Every git child is killed after 10 s (`GitTimedOut`); the in-process open is probed under the same limit (D-39)
gix's open in `Repo::discover` runs on a probed thread and fails with `OpenTimedOut`; the replace guard opens `packed-refs` only when it is a regular file. A fifo planted between probe and open is not caught.

**Rationale:** wsx never hangs where git would block. The in-process half came from the phase code review, which found the first wording claimed the config and `packed-refs` cases for the child limit alone.
**Source:** 04-CONTEXT.md (D-39), 04-REVIEW-DISPOSITION.md (CR-01), STATE.md (04-18 review)

### Any replace ref stops the model in a configured repo before an object read (D-22, D-32)
`GIT_REPLACE_REF_BASE` is refused in `Repo::discover` before gix opens; detection also scans a linked worktree's own `refs/replace`. gix's divergence on `info/grafts` and `shallow` is out of this phase's inputs and went into the STATE obligations for the phases that use gix merge-base and walks.

**Rationale:** Later gix object reads by id can apply replacements differently from git.
**Source:** 04-CONTEXT.md (D-24, D-32), STATE.md (04-12, 04-13), 04-12-SUMMARY.md

### Behaviour tests are the contract; each amendment is named (D-27, D-34, D-35, D-37)
D-34 rewrote three D-06 chain tests to git's `%(symref)` answer; D-35 changed one test so a corrupt ref git skips is absent; D-37 keeps `unsorted_tree_manifest_fails_closed` fail-closed with git's inner cause and was widened on 2026-11-18 to two more tests 04-16 changed. A dangling symbolic rung is absent (D-30), a symbolic rung that resolves is `IntegrationRefSymbolic`.

**Rationale:** Changing a locked-behaviour test is recorded in the decision list so the verifier can rule on it from the record.
**Source:** 04-CONTEXT.md (D-27, D-30, D-34, D-35, D-37), 04-VERIFICATION.md, STATE.md (04-07)

### Lockfile bump for RUSTSEC-2026-0486 waited for the operator
`cargo update -p rustix` moved only rustix 1.1.5 to 1.1.6 (`e6881d1`); `scripts/check.sh deps` then exited 0 and the mutation pass and other gates were re-run on that tip.

**Rationale:** A lockfile update is a package-manager change the executor does not make on its own, and it invalidates an earlier pass-head check.
**Source:** deferred-items.md, 04-12-SUMMARY.md

### A `human_needed` policy flag was answered from the evidence
The 04-18 "STATE.md only through the state verbs" prohibition was decided by the orchestrator from `git show 962cd70 -- .planning/STATE.md` (6 insertions, 4 deletions, verb-shaped); the verifier reached the same reading independently.

**Rationale:** The operator's standing rule is that a policy flag is answered from evidence and only the residue is escalated.
**Source:** 04-UAT.md, 04-VERIFICATION.md

---

## Lessons

### A plan's RED verify piped `--raw` into an exact match that can never pass
`gsd_run check tdd-red-evidence ... --raw` prints the whole JSON, so `rg -qx 'RED_EVIDENCE_OK'` fails on a valid record. Executors ran `--raw --pick verdict` instead, with a negative control (`INVALID_RED` on a record that must not pass). The defect recurred in 04-14 through 04-17.

**Context:** First recorded in 04-13, which noted later plans with the same pipe would meet it; plan files were not edited.
**Source:** 04-13-SUMMARY.md, 04-14-SUMMARY.md, 04-15-SUMMARY.md, 04-16-SUMMARY.md, 04-17-SUMMARY.md

### A mutation pass is only evidence if it is fresh at the last src commit
04-12's first pass left 1 missed of 107 (`refuse_replace_base_env`'s match guard); 04-18's first pass left 9 missed of 265. Verification still found 04-18's pass stale after the review fixes (gap 2); the fresh pass at the final tip was 46 listed, 41 caught, 5 unviable, 0 missed, 0 timeout.

**Context:** Each survivor was closed with a named test, not an equivalence call; 04-18's allowlist stayed empty.
**Source:** 04-12-SUMMARY.md, 04-18-SUMMARY.md, 04-VERIFICATION.md

### Scope tests whose only control was git's answer can pass without showing anything
If gix never loaded an include, global or `config.worktree` value, "not returned" passes trivially. Tests now also assert the unfiltered read sees the value; the subsection test reads `wsx.sub.sourceBranch` so an empty result cannot be a failed write.

**Context:** Added as auto-fixes in 04-05.
**Source:** 04-05-SUMMARY.md

### Editing during a container run compiles a half-edited tree
The container mounts the live checkout; a run overlapping a source edit exited 101 and was discarded and repeated on the committed tip. The command was wrapped in `timeout 3000`.

**Context:** 04-10, Deviation 3.
**Source:** 04-10-SUMMARY.md

### A repository with no `.git/config` failed `resolve` once any global `remote.*` or `wsx.*` key printed (WR-03)
`read_config` now maps a NotFound canonicalize of the own config to no own keys; any other canonicalize error still fails naming the file. The verifier reproduced the failing message on the previous tree as the negative control.

**Context:** Verification gap 1 against roadmap SC 5 (CONF-07, D-16); fixed in 9b996b1, test first.
**Source:** 04-VERIFICATION.md, 04-REVIEW-DISPOSITION.md

### A fifo at `.git/config` or `packed-refs` hung wsx in-process though D-39 said it was stopped (CR-01)
The first D-39 wording relied on the child limit alone; gix's own open and the replace guard's `packed-refs` open ran outside it. Fixed in d8cae67 (test 3d44c24).

**Context:** Critical finding of the phase code review.
**Source:** 04-REVIEW-DISPOSITION.md, 04-CONTEXT.md (D-39)

### A new advisory turned the deps gate red with no lockfile change
`scripts/check.sh deps` failed on 04-12's tip with RUSTSEC-2026-0486 (rustix 1.1.5, via `gix-index 0.58.0` and `tempfile 3.27.0`); `Cargo.lock` was unchanged since before the phase.

**Context:** Found in 04-12 Task 3; the bump invalidated the earlier pass-head check, so the gates were re-run.
**Source:** deferred-items.md, 04-12-SUMMARY.md

### An orchestrator logged a working GSD gate as unrunnable
`query ui.safety-gate` answers `Unknown command: ui`, but the workflow runs `gsd-tools check <id>`, and `check ui.safety-gate 04` answers `block: false`. No defect; corrected 2026-11-17.
**Context:** Upstream triage (open-gsd/gsd-core#5494) agrees.
**Source:** deferred-items.md

---

## Patterns

### Assert git's answer first, then wsx's, with a control that changes git's answer
Tests read git's view of the fixture before asserting wsx's, so a fixture that failed to write the key cannot pass (empty `--get-all` line, two values for `wsx.other`).

**When to use:** Any model input whose correctness is "what git does".
**Source:** 04-05-SUMMARY.md, 04-06-SUMMARY.md

### Positive control beside every absent or empty answer
`upstream_is_not_auto_detected` asserts `tried` is non-empty; `keyless_local_remote_section_is_not_a_remote` adds a `git remote add` that must make the section defined; `included_remote_is_a_remote` asserts `git config --local --get` does not see the url.

**When to use:** A test whose pass condition is "nothing found".
**Source:** 04-06-SUMMARY.md, 04-14-SUMMARY.md

### RED evidence classifier with negative controls
`check tdd-red-evidence` with `--raw --pick verdict`; controls: a nonexistent target test gives `INVALID_RED`, wrong intended text gives `failed without the intended text`, a test that passed at RED gives `did not fail at the RED commit`.

**When to use:** Any RED commit in a TDD plan.
**Source:** 04-13-SUMMARY.md

### Mutation-pass gate: fresh, whole, tested equals listed, empty allowlist
No `--iterate`; sum of caught, unviable, missed and timeout equals the `--list` total; pass head recorded and source unchanged since. Doctored copies of `mutants.out` (last `caught.txt` line deleted or replaced, a line appended to `timeout.txt`) must exit 1 with the intended message.

**When to use:** Closing a plan that changes critical code.
**Source:** 04-12-SUMMARY.md, 04-18-SUMMARY.md

### Test the hook with a real hook
`hook_environment_resolves_the_same_list` installs a pre-commit hook, makes a partial commit in a linked worktree and asserts the `GIT_DIR` and `GIT_INDEX_FILE` git exported, instead of hand-setting the environment.

**When to use:** Behaviour that must hold under git's hook environment.
**Source:** 04-09-SUMMARY.md

### Shared fixtures for raw objects live in `src/git/testing.rs`
`testing::raw_tree_commit` writes blob and tree bytes to files and calls `git hash-object --literally -t tree -w <file>`; `testing::records` is the one NUL/record split for the quote and manifest differentials.

**When to use:** A recipe needed by two private test modules.
**Source:** 04-09-SUMMARY.md, 04-03-SUMMARY.md

### Keep gix out of the model layer with a source boundary check
`rg -qF 'gix::' src/model` must exit 1; test modules render bytes through `crate::git::c_quote`.

**When to use:** Whenever a layer is meant to be free of a dependency.
**Source:** 04-10-SUMMARY.md

### A test module that reruns scrubbed must be listed in `DISCOVERING_SUITES`
`git::objects::tests::` was added when the ported entry tests began calling `rerun_scrubbed()`; the meta-test requires every such module.

**When to use:** Adding a test module that runs discovery under a runner `GIT_DIR`.
**Source:** 04-16-SUMMARY.md

---

## Surprises

### gix 0.90.0 finds refs under other names that git never reads
`find_ref` fell back to `.git/heads/<full>`, `.git/tags/` and `.git/remotes/`; 04-01 treats a ref returned under another name as absent.

**Impact:** Part of the divergence that led to D-23.
**Source:** STATE.md (04-01), 04-01-SUMMARY.md

### `git cat-file --batch` answers an absent-commit gitlink with `<oid> submodule`, not `missing`
A `.wsx.conf` that is a real submodule failed the model with `ManifestUnreadable(GitBadOutput)`. Fixed with `BatchAnswer::Submodule`, which `manifest_entry` maps to `NotABlob { kind: "submodule" }`.

**Impact:** Production behaviour changed for a submodule manifest only; found by closing mutation survivor `objects.rs:201:35` with a real-git test (RED `c28bdd6`, GREEN `128f281`).
**Source:** 04-18-SUMMARY.md, STATE.md (04-18)

### `git for-each-ref` has no `-z` on git 2.56.0
The differential uses `--format=%(refname)%00` and requires each line to end in NUL.

**Impact:** A ref name holds neither NUL nor LF, so the split is raw-byte and safe.
**Source:** 04-08-SUMMARY.md

### RESEARCH's "`.plan` selects nothing" was measured on a repo that did not track `.plan`
This plan tracks `.plan`, and git 2.56.0's `:(literal).plan` lists it, so the test expects `[.plan]` and asserts each selection literally.

**Impact:** A pinned RESEARCH measurement was wrong for this repo; the test asserts git's answer rather than copying it.
**Source:** 04-04-SUMMARY.md

### D-35 removed the fixture of an unlisted test
`require_passes_a_git_failure_through` used a corrupt candidate ref to get a git failure; D-35 skips that ref, so `require` resolved and the test failed. The plan's rewrite list did not name it; the fixture became a blob under `refs/remotes/origin/develop`.

**Impact:** Touched the prohibition on changing D-01 to D-21 tests; recorded as `human_judgment` and ruled within the prohibition's intent by the verifier.
**Source:** 04-15-SUMMARY.md, 04-VERIFICATION.md

### `Problem::BadQuoting` had no test among the sixteen planned
Threat T-04-06 (`only "a"b` read as `a`) was tested only inside `c_unquote`; 17 tests were added instead of 16.

**Impact:** A threat mitigation had no test at the model level until the executor noticed.
**Source:** 04-03-SUMMARY.md

### Sourcing a Markdown file started an interactive shell the agent could not stop
`source` on `gsd-run-resolver.md` ran its fence line as `bash`, leaving PIDs 1580080, 1580129 and 1580130 waiting on stdin; `kill` by those PIDs was denied by the permission classifier.

**Impact:** Leftover processes outlived the plan, no repo effect; the same shape as the 03.1 surprise.
**Source:** 04-11-SUMMARY.md

### The fresh 04-18 mutation pass took 34 min against 19 min for the first
The final pass ran under a load average of about 17 from its own two jobs.

**Impact:** A full pass is a long, load-sensitive step to schedule as a plan's last task.
**Source:** 04-18-SUMMARY.md
