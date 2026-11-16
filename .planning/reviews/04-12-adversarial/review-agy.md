## CONFIRMED

### Finding 1: Task 1 GREEN step cannot compile — `super::shown` is private to `src/git/mod.rs`
- **Severity:** Critical
- **Citation:** `.planning/phases/04-repo-model/04-12-PLAN.md:155-156` vs [src/git/mod.rs:325](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr/snapshot/src/git/mod.rs#L325)
- **Proof:**
  Line 156 specifies:
  > `A listing or an item that fails to read is Error::Gix with what list the refs under {base}, the base through super::shown, not lossy...`
  Line 155 restricts the commit:
  > `GREEN step (commit feat(04-12): refuse nested replace refs git applies and gix ignores), all in src/git/refs.rs:`
  In [src/git/mod.rs:325](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr/snapshot/src/git/mod.rs#L325), `shown` is defined as `fn shown(bytes: &[u8]) -> String`. It is private to `src/git/mod.rs`, not `pub(crate)`.
  Any call from `src/git/refs.rs` to `super::shown` fails compilation with `error[E0603]: function shown is private`. Task 1's RED step action (line 153) explicitly enumerates the edits to `src/git/mod.rs` (adding `ReplacementRefIgnored`, its `Display` arm, `is_replacement_refusal`, and test extensions), omitting any change to make `shown` crate-visible.

---

### Finding 2: Task 3 precondition fails and halts execution — `.tools/bin/cargo-nextest` does not exist
- **Severity:** Critical
- **Citation:** `.planning/phases/04-repo-model/04-12-PLAN.md:221, 236` vs [scripts/check.sh:18-34](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr/snapshot/scripts/check.sh#L18-L34), [.gitignore:2](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr/snapshot/.gitignore#L2)
- **Proof:**
  Task 3 defines a hard precondition:
  > `<precondition>cargo mutants --version exits 0, .tools/bin/cargo-nextest exists, and docker info exits 0; stop and report which does not hold.</precondition>`
  In a fresh worktree/snapshot, `.tools/bin/cargo-nextest` does not exist (`.tools/` is gitignored).
  Running `test -f .tools/bin/cargo-nextest` exits 1.
  Furthermore, the host environment has `cargo nextest 0.9.152` installed globally. Running `scripts/check.sh all` without `.tools/bin/cargo-nextest` (0.9.148) on PATH immediately aborts:
  `error: cargo-nextest run as 'cargo nextest' reports 'cargo-nextest 0.9.152', expected 0.9.148; run scripts/install-tools.sh install cargo-nextest`
  Nowhere in Tasks 1, 2, or 3 does the plan instruct running `WSX_TOOLS_DIR=.tools/bin scripts/install-tools.sh install cargo-nextest`. An automated runner stops immediately at Task 3's precondition, and cannot pass Task 1 or 2 verification on the host.

---

### Finding 3: Contradiction with Verification Gap — Plan shortcuts nested refs before duplicate/chain loops, yielding false error diagnostics on duplicates
- **Severity:** Major
- **Citation:** `.planning/phases/04-repo-model/04-12-PLAN.md:25, 100, 127, 158, 194` vs [.planning/phases/04-repo-model/04-VERIFICATION.md:55](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr/snapshot/.planning/phases/04-repo-model/04-VERIFICATION.md#L55)
- **Proof:**
  `04-VERIFICATION.md` missing item 2 explicitly requires:
  > `"Key the duplicate and chain checks on the last component across all depths, so a nested ref cannot hide a chain or a duplicate"`
  `04-12-PLAN.md:127` claims in the audit table that this is covered by Task 1:
  > `| VERIFICATION gap | key duplicate and chain checks on the last component across depths | Task 1 rule (refused before the keyed loops); Task 2 tests M5, M6 |`
  In `04-12-PLAN.md:158`, the duplicate and chain loops are left strictly one-level. Any nested ref is intercepted earlier and returned as `Error::ReplacementRefIgnored`.
  Tested in scratch repo with git 2.56.1 planting `refs/replace/<A>` (direct) and `refs/replace/team/<A>` (nested):
  - Command: `git cat-file blob <A>`
  - Output: `fatal: duplicate replace ref: team/<A>` (exit 128). Git aborts execution.
  Under the plan's code, wsx refuses with `ReplacementRefIgnored`, rendering:
  > `refusing refs/replace/team/{A_HEX}: git reads it as a replacement of {A_HEX}, taking the object id from the start of the last part of its name, while wsx's reader takes a replace ref only when its whole name after the replace base is an object id, so wsx could read other bytes than git.`
  This error message is factually untrue: git does not read it as a replacement of `{A_HEX}`, nor could wsx read other bytes than git; git halts with a duplicate ref conflict. wsx reports a nonexistent silent byte divergence instead of the duplicate failure git actually produced.

---

### Finding 4: Unfalsifiable Pin Tests lacking negative controls in Task 2
- **Severity:** Major
- **Citation:** `.planning/phases/04-repo-model/04-12-PLAN.md:193-194, 201`
- **Proof:**
  In Task 2, `nested_replacement_duplicating_a_direct_one_fails_closed` and `nested_replacement_in_a_chain_fails_closed` are committed in the "Pin step" without a RED run.
  Because Task 1's preliminary check returns `Error::ReplacementRefIgnored` whenever `stripped.len() != hex_len`, the nested ref in both tests triggers `ReplacementRefIgnored` before any duplicate check (`keyed.insert`) or chain check (`keyed.contains_key`) is executed:
  1. If lines 354-360 of [src/git/refs.rs](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr/snapshot/src/git/refs.rs#L354-L360) (duplicate detection) are deleted, `nested_replacement_duplicating_a_direct_one_fails_closed` still passes.
  2. If lines 364-375 of [src/git/refs.rs](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr/snapshot/src/git/refs.rs#L364-L375) (chain detection) are deleted, `nested_replacement_in_a_chain_fails_closed` still passes.
  Neither test exercises duplicate or chain detection logic; both assert only that a nested ref is rejected by the length check.

---

### Finding 5: `git_replace_key` assumes git ignores suffixed bytes, but git triggers duplicate ref aborts
- **Severity:** Minor
- **Citation:** `.planning/phases/04-repo-model/04-12-PLAN.md:95, 199`
- **Proof:**
  The plan asserts: "the id is the first `hex_len` bytes of the last component... and the bytes after them are ignored, as git ignores them".
  Tested in scratch repo with git 2.56.1 planting `refs/replace/<A>` and `refs/replace/<A>.bak`:
  - Command: `git cat-file blob <A>`
  - Output: `fatal: duplicate replace ref: <A>.bak` (exit 128).
  Git does not benignly ignore suffix bytes when another ref with the same prefix id exists. wsx diagnoses `refs/replace/<A>.bak` as `ReplacementRefIgnored` ("wsx could read other bytes than git") rather than a duplicate collision.

---

## SUSPECTED

### Finding 6: Misleading error diagnosis when `GIT_REPLACE_REF_BASE` lacks a trailing slash
- **Severity:** Minor
- **Citation:** `.planning/phases/04-repo-model/04-12-PLAN.md:102, 156, 158` vs [04-REVIEW.md:91-95](file:///tmp/claude-1000/-var-home-denniyahh-Github-wsx/21a6fcdb-a844-40bf-85a9-2914cac8c18f/scratchpad/xr/snapshot/.planning/phases/04-repo-model/04-REVIEW.md#L91-L95)
- **Proof:**
  Tested git 2.56.1 with `GIT_REPLACE_REF_BASE=refs/replace` (no trailing slash) and `refs/replace/<A>` present:
  - Command: `GIT_REPLACE_REF_BASE=refs/replace git cat-file blob <A>`
  - Output: `BUG: refs.c:1908: ref pattern must end in a trailing slash when trimming` (exit 128).
  Under Task 1's `refs_starting_with(&git_base)`:
  - `full` is `refs/replace/<hex>`
  - `full.strip_prefix(b"refs/replace")` is `b"/<hex>"` (length 41)
  - `git_replace_key(b"/<hex>", 40)` returns `Some(<hex>)`
  - `stripped.len() != hex_len` (41 != 40) is true
  wsx returns `Error::ReplacementRefIgnored { name: b"refs/replace/<hex>", original: <hex> }`, claiming git reads `{hex}` while wsx could read other bytes. In reality, git crashes with an internal assertion failure. While WR-01 was deferred, widening the listing to any depth causes ordinary direct replace refs to report false `ReplacementRefIgnored` errors under slashless bases.
