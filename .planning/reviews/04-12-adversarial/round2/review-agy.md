I have initiated `cargo check --tests` in the background to build the test dependencies for probing gix and git behavior, and will resume once it finishes.
## CONFIRMED

### Finding 1: 04-12 Task 2 RED verify command fails because `replace_base_without_a_trailing_slash_fails_closed` emits `ReplaceBaseDiffers`, not `"configured"`
- **Severity:** Critical
- **Plan Reference:** [.planning/phases/04-repo-model/04-12-PLAN.md:243-244](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/.planning/phases/04-repo-model/04-12-PLAN.md#L243-L244) (and line 227)
- **Reproduction & Mechanism:**
  In `model::tests::replace_base_without_a_trailing_slash_fails_closed` ([04-12-PLAN.md:227](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/.planning/phases/04-repo-model/04-12-PLAN.md#L227)), the test plants `git update-ref refs/replace/pre<a.blob> <b.blob>` and runs the child with `GIT_REPLACE_REF_BASE=refs/replace/pre`. Local config has `gitoxide.objects.replaceRefBase` unset.
  At the Task 2 RED commit (prior to implementing `is_replace_base`), `refuse_unfollowable_replacements` reaches line 340 of [src/git/refs.rs](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/src/git/refs.rs#L340):
  `git_base` is `refs/replace/pre`; `gix_base` is `DEFAULT_REPLACE_BASE` (`refs/replace/`). Because `gix_base != git_base` and `self.any_ref_starts_with(&git_base)?` is `true`, the guard returns:
  `Err(Error::ReplaceBaseDiffers { value: "refs/replace/", git_base: "refs/replace/pre" })`.
  The child process does not succeed and never reports `"configured"`. Instead, `child_state_with` returns:
  `"failed: refusing gitoxide.objects.replaceRefBase = refs/replace/: wsx's reader takes replace refs from under refs/replace/, while git takes them from under refs/replace/pre, so wsx could read other bytes than git..."`
  Consequently, the assertion failure message at RED is:
  `the child under GIT_REPLACE_REF_BASE=refs/replace/pre answered "failed: refusing gitoxide.objects.replaceRefBase = refs/replace/: ...", not "failed: refusing GIT_REPLACE_REF_BASE=refs/replace/pre: "`
  This output does not match `answered "configured", not`.
  The automated verification check at line 243:
  `rg -qF "$text" "$L"` (where `$text` is `'answered "configured", not'`)
  exits 1 and prints:
  `replace_base_without_a_trailing_slash_fails_closed failed without the intended text; see target/gsd/04-12-red/replace_base_without_a_trailing_slash_fails_closed.log`.
  Task 2 RED verification is blocked.

---

### Finding 2: Replace refs pointing to annotated tags cause git/gix divergence and blame `.wsx.conf` (`ManifestUnreadable`), violating CONF-07 and D-22
- **Severity:** Critical
- **Plan Reference:** [.planning/phases/04-repo-model/04-12-PLAN.md:121, 192](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/.planning/phases/04-repo-model/04-12-PLAN.md#L121), [src/git/refs.rs:362-375](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/src/git/refs.rs#L362-L375), [src/model/mod.rs:488-493](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/src/model/mod.rs#L488-L493)
- **Reproduction & Mechanism:**
  Probe run in scratch repository:
  ```bash
  git tag -a tag_b -m "tag pointing to b" <b_blob>
  git update-ref refs/replace/<a_blob> $(git rev-parse tag_b)
  git cat-file blob <branch>:.wsx.conf
  # Output: exit 0, prints "only b" (git peels the tag to the target blob)
  ```
  In gix 0.90.0 (`open/repository.rs:541-546`), the replacement map entry stores `(a_blob, tag_oid)` using `r.target.try_id()`, leaving the tag unpeeled.
  When `refuse_unfollowable_replacements` runs:
  - The name is direct (`refs/replace/<a_blob>`), 40 hex digits, so `git_replace_key` accepts it.
  - `find_ref` returns `RefTarget::Object(tag_oid)`.
  - `tag_oid` is not in `keyed` and is not symbolic (`RefTarget::Object`).
  - The guard returns `Ok(())`.
  When `build` reads the manifest via `repo.root_entry` -> `repo.find_blob(a_blob)`:
  Gix looks up `a_blob`, retrieves `tag_oid`, and fails:
  `find_blob failed: Object named <tag_oid> was supposed to be of kind blob, but was kind tag.`
  `build` ([src/model/mod.rs:488-493](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/src/model/mod.rs#L488-L493)) maps this non-refusal error through `unreadable(source)`, returning:
  `Error::ManifestUnreadable { branch, source }`.
  Git reads the replaced manifest `only b`; wsx refuses with `ManifestUnreadable`, blaming `.wsx.conf` instead of the replace ref.

---

### Finding 3: `packed-refs` peeled line for replace refs creates silent divergence between git and gix without guard detection
- **Severity:** Major
- **Plan Reference:** [.planning/phases/04-repo-model/04-12-PLAN.md:190-192](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/.planning/phases/04-repo-model/04-12-PLAN.md#L190-L192)
- **Reproduction & Mechanism:**
  When `refs/replace/<A>` points to a tag object and is packed via `git pack-refs --all --prune`, `.git/packed-refs` contains:
  ```
  <tag_oid> refs/replace/<A>
  ^<peeled_oid>
  ```
  Git uses `<peeled_oid>`.
  Gix's `open/repository.rs:541-546` maps `source -> r.target.try_id()` (`<tag_oid>`), discarding `r.object` (`<peeled_oid>`).
  The guard in 04-12 does not inspect whether a target in `packed-refs` has a peeled `^` record. Gix attempts to read `<tag_oid>` directly as a commit or blob and fails with an object kind mismatch, while git reads `<peeled_oid>`.

---

### Finding 4: Ref-name construction specification in 04-13 line 112 produces double slashes (`refs/replace//<path>`), making all loose replace refs fail name validation
- **Severity:** Major
- **Plan Reference:** [.planning/phases/04-repo-model/04-13-PLAN.md:112](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/.planning/phases/04-repo-model/04-13-PLAN.md#L112)
- **Reproduction & Mechanism:**
  [04-13-PLAN.md:112](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/.planning/phases/04-repo-model/04-13-PLAN.md#L112) specifies:
  `A regular file whose ref name, the base followed by its path below the root joined with '/', is not a valid full ref name through gix::refs::FullName::try_from is ReplacementRefUnlisted { path, kind: "a file whose name is not a valid ref name" }`
  In [04-12-PLAN.md:238](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/.planning/phases/04-repo-model/04-12-PLAN.md#L238), `is_replace_base` mandates that `base` ends with `'/'` (`refs/replace/`).
  Joining `base` (`refs/replace/`) and the relative path (`<blob>`) with `'/'` produces `refs/replace//<blob>`.
  `gix::refs::FullName::try_from(b"refs/replace//<blob>")` returns `Err` because consecutive slashes are forbidden in git ref names.
  Following this instruction literally causes every legitimate regular loose replace ref file to be falsely classified as `ReplacementRefUnlisted`.

---

### Finding 5: 04-13 Task 2 acceptance criterion lacks an automated `<verify>` command
- **Severity:** Minor
- **Plan Reference:** [.planning/phases/04-repo-model/04-13-PLAN.md:233-234](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/.planning/phases/04-repo-model/04-13-PLAN.md#L233-L234) (vs lines 219-231)
- **Reproduction & Mechanism:**
  Acceptance criterion line 233 requires:
  `- rg -qF 'ReplacementRefUnlisted' .planning/STATE.md exits 0 [HEAD=1 EXPECT==0], and the STATE commit touches no other file`
  And line 234 specifies:
  `- All six <verify> commands exit 0`
  Across all six `<verify>` blocks in Task 2 ([04-13-PLAN.md:219-231](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/.planning/phases/04-repo-model/04-13-PLAN.md#L219-L231)):
  1: cargo-mutants run; 2: allowlist check; 3: mutant identity; 4: coverage; 5: deps; 6: container gate.
  Unlike 04-12 Task 3 line 279, there is no automated verify check for `STATE.md` or git commit isolation in 04-13 Task 2.

---

## SUSPECTED

### Finding 6: Unverified handling of direct commit replacement by a tree or blob
- **Severity:** Minor
- **Plan Reference:** [src/model/mod.rs:485](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/src/model/mod.rs#L485), [.planning/phases/04-repo-model/04-12-PLAN.md:225, 239](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr2/snapshot/.planning/phases/04-repo-model/04-12-PLAN.md#L225)
- **Reasoning:**
  `04-12` tests a chained commit replacement (`<commit> -> <blob> -> <commit>`) in `replacement_chain_through_a_blob_on_the_workspace_tip_blames_the_ref`, where the duplicate/chain check detects `<blob>` in `keyed` and triggers `ReplacementRef`.
  If a direct replacement `<commit> -> <blob>` is created without a second chain hop (via `git update-ref refs/replace/<commit> <blob>`), the guard sees no chain, no duplicate, and a valid 40-hex key, returning `Ok(())`.
  `ref_commit` then fails to peel `<blob>` to a commit and wraps it in `ManifestUnreadable`, blaming `.wsx.conf`. In git, `git cat-file blob <branch>:.wsx.conf` fails with `fatal: path '.wsx.conf' exists on disk, but not in '<branch>'`.
  Both readers fail, but wsx's error attributes the failure to `.wsx.conf` rather than the replacement ref.

---

## R1–R13 status

| Item | Status | Plan Citation | Reason / Note |
|---|---|---|---|
| R1 | Closed | `04-13-PLAN.md:27-28, 110-114, 158-162, 173-176` | Loose symlinks and symlinked dirs detected by `refuse_unlisted_replace_files` via `read_dir` non-following file types. |
| R2 | Closed | `04-12-PLAN.md:28, 120, 227-230, 237-238` | `is_replace_base` enforces trailing slash and valid full ref name; returns `ReplaceBaseInvalid`. (Note: RED verify script has regex defect, Finding 1). |
| R3 | Closed | `04-12-PLAN.md:27, 225-226, 239` | Guard moved in `build` before `remote`, `integration`, and `ref_commit`. |
| R4 | Closed | `04-12-PLAN.md:25, 174, 189-194` | Full `git_replace_key` rule, `<A>.bak` test, and updated doc comment included in Task 1 GREEN commit. |
| R5 | Closed | `04-12-PLAN.md:25-26, 151, 260-288` | Dropped claim that pin tests test duplicate/chain logic across depths; added negative control `nested_replace_ref_shorter_than_an_id_reads_as_git_does` with cargo-mutants verification. |
| R6 | Closed | `04-12-PLAN.md:25, 180, 185` | Message text updated to `or stops with "duplicate replace ref" when another replace ref names {A_HEX} too`. |
| R7 | Closed | `04-12-PLAN.md:25, 117, 179, 185, 193` | Doc comment updated to state gix accepts 40 or 64 hex digits; added `nested_replacement_in_a_sha256_repository_fails_closed`. |
| R8 | Closed | `04-12-PLAN.md:31, 272`, `04-13-PLAN.md:31, 210` | `STATE.md` updated in 04-12 Task 3 (`ReplacementRefIgnored`, `ReplaceBaseInvalid`) and 04-13 Task 2 (`ReplacementRefUnlisted`). |
| R9 | Closed | `04-13-PLAN.md:32, 211-214, 219, 221, 223` | Mutant identity check via sorted `cmp -s`, exit 3/timeout handled explicitly, doctored controls verified. |
| R10 | Closed | `04-13-PLAN.md:33, 215, 225` | `scripts/check.sh coverage` added with output filtered to modified files. |
| R11 | Closed | `04-12-PLAN.md:6`, `04-13-PLAN.md:7-14, 211` | `04-12` depends on `04-11`; `04-13` includes all 6 files in `files_modified` and isolates output and allowlists. |
| R12 | Closed | `04-12-PLAN.md:105`, `04-13-PLAN.md:99` | Scope widening recorded in plan context and flagged for SUMMARY audit. |
| R13 | Closed | `04-12-PLAN.md:124, 190` | `shown` vs `lossy` quoting discrepancy documented and deferred to WR-02. |

---

### Clean Up Verification
All temporary scratch directories created in `/tmp` during probing have been removed; no background processes remain active. The repository working tree is unmodified.
