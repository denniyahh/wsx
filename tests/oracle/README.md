# The vendored oracle

This directory holds devflow's workspace scripts, copied here unchanged. They are the behavioural
oracle that wsx is measured against: each scenario in the test harness (`tests/harness`) runs once
against these scripts and once against the `wsx` binary, and states the outcome it expects from
each.

The directory is temporary. It is deleted at cutover, once every scenario passes on the Rust side
with its recorded expectations intact (requirement REL-02).

## Source

- Repository: devflow
- Branch: `develop`
- Commit: `ee6ec792d03f1495fc8c4d90d5894ff08b968d80` ("Merge pull request #228 from
  denniyahh/feature/dependabot", 2026-10-23)

## Vendored files

Every file below is byte-for-byte the file devflow has at that commit. The "blob" column is the
identifier git computes from a file's exact bytes (`git hash-object --no-filters <file>`), so any
change to a file, even one byte, changes it. The test `provenance::vendored_files_match_devflow_blobs`
hashes each file here and fails if one no longer matches.

| File here | devflow path | Mode | Blob |
|---|---|---|---|
| `sync-workspace.sh` | `scripts/sync-workspace.sh` | 100755 | `82ed2d7572e03c7527f3deced2a51562ba891e85` |
| `phase-worktree.sh` | `scripts/phase-worktree.sh` | 100755 | `ae6e47022137768a6ff947151c7169b898781719` |
| `cut-pr-branch.sh` | `scripts/cut-pr-branch.sh` | 100755 | `d77eda55b84aa67215ecf51ddde2b38c982a6989` |
| `check-workspace-divergence.sh` | `scripts/check-workspace-divergence.sh` | 100755 | `0041f8a50fae25a2b88b1fe4ac9393314697909f` |
| `hooks/pre-push` | `scripts/hooks/pre-push` | 100755 | `67ef847ecf98dfaa1b124be5b10179b7a0134dc9` |
| `LICENSE` | `LICENSE` | 100644 | `9fde56fe9dc9a61107fb774925149bd11942de67` |

Mode 100755 is an executable file and 100644 a plain one. The blob test checks content only; the
modes are what `git ls-files -s tests/oracle` reports for this directory.

After `oracle.patch` is applied (see below), the three scripts it changes have these blobs. The
test `provenance::oracle_patch_applies_cleanly` checks them, and checks that the other three files
keep the blobs above.

| File | Blob after the patch |
|---|---|
| `check-workspace-divergence.sh` | `21465e914f6caed713da72a75824f72ba56756ec` |
| `phase-worktree.sh` | `55536494523bff4c6faa1af4582231b755a9fc3e` |
| `hooks/pre-push` | `093b279b591b27401613c7c4dbe5edcdcd3d907f` |

## Attribution

devflow is licensed under the MIT License, Copyright (c) 2026 Dennis Kim, with the Apache License,
Version 2.0 as an alternative. The full text is in `LICENSE` beside this file, copied unchanged from
devflow.

## The patch

The scripts expect to run from the root of a devflow checkout, where they call each other as
`scripts/<name>`. In the harness they run from a scratch copy of this directory against a test
repository that has no `scripts/` directory. `oracle.patch` is the one change made to them: the
harness copies this directory into the scenario's scratch directory and applies the patch to that
copy. The files here are never patched.

Each redirected path is written `"${WSX_ORACLE_DIR:?}/<name>"`. The harness sets `WSX_ORACLE_DIR` to
the scratch copy for each script it runs. If the variable is missing, bash stops the script with
`WSX_ORACLE_DIR: parameter null or not set` instead of quietly looking somewhere else.

| File | Lines (devflow) | What changes | Why |
|---|---|---|---|
| `check-workspace-divergence.sh` | 30 | `FORBIDDEN_PATHS` is read from `"${WSX_ORACLE_DIR:?}/cut-pr-branch.sh"` instead of `scripts/cut-pr-branch.sh` | The test repository has no `scripts/cut-pr-branch.sh`; the copy does |
| `hooks/pre-push` | 209 | The divergence check runs as `"${WSX_ORACLE_DIR:?}/check-workspace-divergence.sh" origin/develop` instead of `./scripts/check-workspace-divergence.sh origin/develop` | Same reason: the check lives in the copy, not in the test repository |
| `hooks/pre-push` | 233-250 | The CI-mirror tail is replaced by a two-line comment and `exit 0` | That tail runs `scripts/check-in-container.sh all` (or `scripts/check.sh all`), which would run this repository's container gate inside a test fixture |
| `phase-worktree.sh` | 54 and 61 | The existence test and the call of `sync-workspace.sh` use `"${WSX_ORACLE_DIR:?}/sync-workspace.sh"` instead of `scripts/sync-workspace.sh` | Without it the script would print `scripts/sync-workspace.sh not found; skipping sync` and never sync |

Messages the scripts print are not changed, so some still name `scripts/...` paths.

## Rules

- Never edit a vendored file, not even to make a scenario pass. The oracle has to keep recording
  devflow's real behaviour, defects included: those defects are the negative controls for the
  scenarios that record a defect wsx has fixed.
- The patch adapts paths only. It leaves `origin`, `origin/develop` and `cut-pr-branch.sh`'s
  `FORBIDDEN_PATHS` line alone, because those are oracle behaviour, not paths.
- The harness applies the patch with `git apply` to the scratch copy, with
  `GIT_CEILING_DIRECTORIES` set to the scenario's scratch directory. Without it, `git apply` run
  inside a git work tree skips every file and still exits 0
  (`provenance::patch_apply_without_ceiling_is_a_silent_no_op` shows this). The harness then
  refuses any copy whose patched scripts do not mention `WSX_ORACLE_DIR`.

## Re-vendoring at a new devflow commit

1. Copy the five scripts and `LICENSE` from devflow at the new commit, unchanged.
2. Rebuild `oracle.patch` against the new files, adapting paths only.
3. Update this README, and in `tests/harness/provenance.rs` update `DEVFLOW_COMMIT`,
   `VENDORED_BLOBS` and `PATCHED_BLOBS`. The test
   `provenance::readme_records_the_pinned_commit_and_blobs` fails while this README and those
   constants disagree.
4. Expect the recorded oracle outcomes in the scenarios to need review: a new devflow revision can
   behave differently.
