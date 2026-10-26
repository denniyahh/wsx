---
phase: 02-oracle-harness
plan: 08
subsystem: testing
tags: [rust, nextest, oracle, harness, git, ssh-signing, comparator]

requires:
  - phase: 02-oracle-harness (02-02, 02-03)
    provides: observer, comparator (compare, Field, CommitPart), fixture presets, run_case
  - phase: 02-oracle-harness (02-05)
    provides: Case, Scenario, Role::Counterpart, validate and judge
  - phase: 02-oracle-harness (02-07)
    provides: cut::self_test (Parity) and the Action::Cut oracle mapping
provides:
  - Per-field mutation meta-test over one real sync::self_test oracle run (25 single-field mutations)
  - Edge tests for empty expectations, byte-exact names and messages, and ref-set versus commit-sequence ordering
  - RepoConfig::signing and RepoConfig::with_signing, honoured by the preset's last step (per-run ed25519 key and local signing config)
  - signing::sync_merge_signed and signing::cut_replay_signed (with an unsigned counterpart), all Parity
affects: [02-09, 02-10, 02-11, 02-12, 02-13, 02-14, phase-7, phase-9]

actuals:
  tokens: 6500
  tasks: 2
  commits: 3
plan_head_before: 20c32c8f8683d4b56d662e8c8c4ef269a7b44970
plan_head_after: 6f622d7f91ef1191c5282bc3958f839faa739cb6

tech-stack:
  added: []
  patterns:
    - "A scenario that reuses a registered case looks it up in all_scenarios() by module and name, changes only what it tests, and panics naming the case when the lookup fails"
    - "Signing is declared in the repo config (RepoConfig::with_signing), never switched on inside a fixture function"
    - "A mutation meta-test asserts that every reported mismatch is the mutated field, not only that one is present"

key-files:
  created: []
  modified:
    - tests/harness/outcome.rs
    - tests/harness/fixture.rs
    - tests/harness/signing.rs

key-decisions:
  - "names_are_observed_byte_exact declares i18n.commitEncoding=ISO-8859-1 on its fixture: under git's default UTF-8 commit encoding, git itself stores a non-UTF-8 message byte re-encoded as Latin-1 (0xFF becomes 0xC3 0xBF), so the raw byte would never reach the observer"
  - "The comparator needed no fix: every one of the 25 mutations was reported as its own field on the first run"
  - "All three signing cases are Parity by observation, as the plan predicted; no case differed on any field other than signed"

patterns-established:
  - "Signature expectations are stated per commit message (Signature { message, signed }) and applied to every expected ref tip and new commit with that message; a message that matches nothing panics"

requirements-completed: [HARN-03]

coverage:
  - id: D1
    description: "Changing any single expected field of sync::self_test's outcome is reported as that field and nothing else; the unmutated expectation reports no mismatch"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "tests/harness/outcome.rs#outcome::changing_any_expected_field_fails"
        status: pass
      - kind: integration
        ref: "tests/harness/outcome.rs#outcome::unchanged_expectation_has_no_mismatch"
        status: pass
    human_judgment: false
  - id: D2
    description: "An expected empty new-commit list and an unchanged file set are compared: one fabricated new commit, or a stray stray.profraw, is reported"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "tests/harness/outcome.rs#outcome::empty_expectations_are_compared"
        status: pass
    human_judgment: false
  - id: D3
    description: "Non-ASCII, newline and non-UTF-8 path names, and a message with a non-UTF-8 byte, a body and a trailer, are observed byte for byte in the tree, the message and the files on disk"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "tests/harness/outcome.rs#outcome::names_are_observed_byte_exact"
        status: pass
    human_judgment: false
  - id: D4
    description: "Refs compare as a set keyed by name; new commits compare as a sequence"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "tests/harness/outcome.rs#outcome::refs_compare_as_a_set_and_commits_as_a_sequence"
        status: pass
    human_judgment: false
  - id: D5
    description: "Signature presence asserted separately: a per-run SSH key signs the sync merge and the cut's replayed commit, while the fetched commit and the unsigned counterpart stay unsigned"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "tests/harness/signing.rs#signing::sync_merge_signed::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/signing.rs#signing::cut_replay_signed::oracle"
        status: pass
    human_judgment: false

duration: 8min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 08: Exhaustive comparison and per-run signing Summary

**The comparator catches every single-field change in an expected outcome, one byte at a time (25 mutations over one real oracle sync). Signature presence is asserted as its own field, using a throwaway ed25519 key generated in each scenario's TempDir.**

## Performance

- **Duration:** about 8 min (2026-10-26T18:28:11Z to about 18:36Z)
- **Tasks:** 2 of 2
- **Files modified:** 3 (`tests/harness/outcome.rs`, `tests/harness/fixture.rs`, `tests/harness/signing.rs`)

## Accomplishments

- `outcome::changing_any_expected_field_fails` takes `sync::self_test` from `all_scenarios()`, runs its subject once on the oracle side, and applies 25 single-field mutations to its expectation. Each mutation must produce at least one mismatch, and every mismatch must be the mutated field. The rows are:
  1. exit code
  2. ref set: one extra ref
  3. ref set: one missing ref
  4. ref tip: an unchanged symbolic ref (`origin/HEAD`) expected at a commit
  5. ref tip: a moved ref (`refs/heads/develop`) expected unchanged
  6. ref tip: an unchanged ref (`origin/workspace/dev`) expected at another commit (reported as that ref's commit fields)
  7. new-commit count: one fewer
  8. new-commit count: one more
  9. new-commit order: the two swapped
  10. commit message: one byte
  11. author name: one byte
  12. author email: one byte
  13. author date: one byte
  14. committer name: one byte
  15. committer email: one byte
  16. parents: the two swapped
  17. tree path: one byte
  18. tree mode (`100644` to `100755`)
  19. tree content: one byte
  20. signature
  21. file path: one byte
  22. file content: one byte
  23. file executable bit
  24. files: one extra
  25. files: one missing
- `outcome::unchanged_expectation_has_no_mismatch` is the clean direction: the same real run against the unmutated expectation reports nothing.
- `outcome::empty_expectations_are_compared`: `sync::already_up_to_date` (from the registry) reports exactly `[NewCommitCount]` for one fabricated new commit, and exactly `[Files]` for a stray `stray.profraw`. The unmodified run reports nothing.
- `outcome::names_are_observed_byte_exact`: `.planning/\xC3\xA9.md`, `.planning/a\nb.md` and `.planning/\xFF.md` are checked three ways: on disk through `OsStr::from_bytes`, in the commit tree and in the observed files. The message (`0xFF`, a blank line, a body line, a `Signed-off-by:` trailer) matches both the raw `cat-file commit` and the observer. The U+FFFD lossy form of the `0xFF` name is absent.
- `outcome::refs_compare_as_a_set_and_commits_as_a_sequence`: refs listed in reverse give no mismatch; swapped new commits give at least one.
- `RepoConfig::signing` (false in every preset) and `RepoConfig::with_signing()`. When signing is declared, the preset's last step, after its commits, local config and push, does the following:
  - runs `ssh-keygen -q -t ed25519 -N '' -C harness -f <root>/signkey` through `env::command`
  - writes `<root>/allowed_signers` as `harness@example.invalid namespaces="git" <type> <base64>`
  - sets `gpg.format ssh`, `user.signingkey <root>/signkey`, `commit.gpgsign true` and `gpg.ssh.allowedSignersFile` in `work`'s local config
- `signing::sync_merge_signed::oracle` declares signing on `sync::self_test`'s subject. It expects the sync merge signed, as both the `workspace/dev` tip and new commit 2. It expects `feat: add b` unsigned, as the `develop` and `origin/develop` tips and new commit 1.
- `signing::cut_replay_signed::oracle` declares signing on `cut::self_test`'s subject and expects the replayed `feat(1): add b` signed. Its counterpart control, with no signing declared, expects the same replay unsigned.

## Task Commits

1. **Task 1: Every expected field is compared, byte-exactly**: `8061388` (test)
2. **Task 2: Per-run SSH signing and signature presence asserted separately**
   - RED: `94605bc` (test)
   - GREEN: `6f622d7` (feat)

## TDD Gate Compliance

- **RED, Task 2** (`94605bc`):
  - Command and exit: `scripts/check.sh test -E 'test(=signing::sync_merge_signed::oracle)'` exited 100.
  - Report: JUnit `target/nextest/default/junit.xml`. The run started at 2026-10-26T18:33:47.599Z and the report was written at 18:33:50.040Z.
  - Record: `target/gsd/02-08-red-evidence.json` (gitignored).
  - Classifier verdict: **`RED_EVIDENCE_OK`** (`target_test_failed`, matched `wsx::harness#signing::sync_merge_signed::oracle`, 1 test, 1 fail).
  - Semantic assessment: the test compiled clean under `clippy -D warnings`, validation passed, and the fixture and oracle sync ran (exit 0, merge made). The report had exactly 2 differences, both the merge commit's signature (the `refs/heads/workspace/dev` tip and new commit 2: expected signed, got unsigned). `feat: add b` was expected unsigned and was not reported, so the failure is the planned one and not a blanket flip.
- The `test(02-08)` signing commit `94605bc` precedes the `feat(02-08)` commit `6f622d7`.
- Task 1 is not TDD, as the plan states: the comparator existed. Its evidence is the two directions that must disagree, plus the negative controls below.

## Classification of each case (recording rule)

| Case | Predicted | Observed |
|------|-----------|----------|
| `signing::sync_merge_signed`, subject | Parity | Parity: passed first run after GREEN, no field other than `signed` differed |
| `signing::cut_replay_signed`, subject | Parity | Parity |
| `signing::cut_replay_signed`, counterpart control 1 | Parity | Parity |

## Negative controls (all run, all reverted, none committed)

Each control broke one thing on purpose. The relevant test had to fail naming the broken field, and the file was then restored and checked against its saved sha256.

- **Signature comparison disabled** (`if false && signed != got_signed`): `changing_any_expected_field_fails` failed with `commit signature: reported []`.
- **Empty new-commit list skipped** (early return in `compare_new_commits`): `empty_expectations_are_compared` failed with "one new commit against an expected empty list".
- **Unchanged file set skipped** (`FilesExpect::Unchanged => {}`): the same test failed with "one stray file against an unchanged file set".
- **Lossy tree paths** (`String::from_utf8_lossy` in `read_tree`): `names_are_observed_byte_exact` failed on the `.planning` path set.
- **Refs compared in listed order** (the `BTreeSet` sort removed): `refs_compare_as_a_set_and_commits_as_a_sequence` failed with "refs listed in reverse order".
- **Fetched `feat: add b` expected signed**: `sync_merge_signed` failed with 3 signature differences (`develop`, `origin/develop`, new commit 1).
- **Counterpart expected signed**: `cut_replay_signed` failed on counterpart control 1 with 2 signature differences.
- **Signing not declared on the cut subject**: `cut_replay_signed` failed on the subject with 2 signature differences.
- **Misnamed nextest filter**: `scripts/check.sh test -E 'test(=outcome::no_such_test)'` exited 4.
- **Touched-files check**: adding `tests/harness/cut.rs` to the observed list made the comparison exit 1.
- **Throwaway probe test** (not committed): a fixture with signing declared made one commit. `git verify-commit HEAD` exited 0 ("Good "git" signature for harness@example.invalid with ED25519 key"). `HEAD~1` (the preset commit, made before the key existed) exited 1, and `%G?` read `G` and `N`. The private key mode is 600. This confirms the `allowed_signers` format, although the harness asserts only presence.

## Verification

- Task 1's verify chain (five `&&`-chained single-test commands) exited 0. Task 2's chain (both signing tests) exited 0.
- `scripts/check.sh all` exited 0 after each non-RED commit, ending `==> check.sh: all OK` (108 passed, 8 skipped; the skipped tests are the ignored Rust sides).
- Acceptance checks, all as specified:
  - `fn changing_any_expected_field_fails`, `from_bytes`, `fn with_signing`, `ssh-keygen`, `with_signing`/`all_scenarios`, `signed: true`, and `signed: false` are each found by `rg -q` (exit 0).
  - The plan's commits touch exactly `tests/harness/fixture.rs`, `tests/harness/outcome.rs` and `tests/harness/signing.rs` under `tests/` (exit 0).
  - No tracked file name contains `signkey` (exit 0).
- No `harness-*` scratch tree is left under `target/tmp` after the runs.

## What these checks do not establish

- **Presence, not validity.** Signature presence is asserted, not signature validity. `git verify-commit` needs the allowed-signers file and is not part of HARN-03. The one-off probe above checked validity once; no committed test does.
- **One outcome shape.** The mutation test proves the comparator on one real outcome shape, a sync merge with listed refs and listed files. Other shapes (`RefsExpect::Unchanged`, a symbolic ref changing target) rely on the same exhaustive destructuring. Each field is mutated on one commit only, the sync merge as new commit 2. The comparison of a ref tip's commit uses the same `compare_commit`, but no ref-tip row mutates a single commit field.
- **Tree granularity.** Tree path, mode and content are all reported as the single `Tree` field. The test proves each one is caught, not that the report says which of the three changed (the mismatch text does name the entry).
- **A baseline check.** The `rg -q 'from_bytes'` acceptance check also passed at baseline, because `read_commit` already used `OsStr::from_bytes`. It does not show the test builds names from bytes. Reading the test does: names are byte literals, checked on disk through `OsStr::from_bytes`.
- **Latin-1 commits only.** The non-UTF-8 message is observed byte for byte only in a repository that declares a Latin-1 commit encoding. In a default UTF-8 repository, git rewrites the byte before the observer ever sees it, which is a git behavior and not an observer one.

## Decisions Made

- `names_are_observed_byte_exact` declares `i18n.commitEncoding=ISO-8859-1` (see the deviation below).
- `signing.rs` has its own small `registered` lookup, and `outcome.rs` has one returning the `Entry` that `run_case` needs. The plan asks for the signing helper in `signing.rs`, and sharing it would have meant a cross-module `pub` helper for five lines.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in the test as planned] A non-UTF-8 commit message byte is rewritten by git, not by the observer**
- **Found during:** Task 1, `names_are_observed_byte_exact`, first run.
- **Issue:** the observed message held `0xC3 0xBF` where `0xFF` was given. Git's commit path stores a non-UTF-8 byte re-encoded as Latin-1 to UTF-8 when the commit encoding is UTF-8, so the raw byte never reaches the object store.
- **Fix:** the test's fixture declares `RepoConfig::devflow().with_local_config("i18n.commitEncoding", "ISO-8859-1")`, so git stores the bytes as given (with an `encoding` header the observer ignores). The test also checks that the raw `git cat-file commit` output ends with the given message, which separates "git rewrote it" from "the observer rewrote it". The comparator and observer were not changed.
- **Files modified:** `tests/harness/outcome.rs`
- **Commit:** `8061388`

**2. [Rule 3 - Blocking] Two clippy findings in the first Task 1 draft** (`type_complexity` on the mutation table and `redundant_closure`): fixed with a `Mutation` type alias and `.all(&reported)` before commit. The signature negative control was re-run after the change and still failed as expected.

**Total deviations:** 2 auto-fixed (1 test-design bug, 1 lint). **Impact:** none on scope. The comparator needed no fix.

## Issues Encountered

None.

## Known Stubs

None.

## User Setup Required

None.

## Next Phase Readiness

- 02-09 and the HARN-06 case plans (02-10 to 02-14) can transcribe recorded outcomes through the observer. The comparator was not changed, so no earlier recording is invalidated.
- A scenario that needs signed commits declares `.with_signing()` on its config. Any commit made after the preset in the work repository or its worktrees is then signed, fixture commits included, so assert `signed` only on commits the action creates. Commits made in an `advance_remote` scratch clone are never signed.

## Self-Check: PASSED

- All three modified files exist. Commits `8061388`, `94605bc` and `6f622d7` are ancestors of HEAD (`git merge-base --is-ancestor`).
- `git rev-list --count 20c32c8..HEAD` is 3 at SUMMARY time.
