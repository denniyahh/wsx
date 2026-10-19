## CONFIRMED

### 1. Direct Scope Contradiction: Backup Retention Out of Scope in PROJECT.md but Mandatory in REQUIREMENTS.md
* **Citation:**
  * [.planning/PROJECT.md#L62](file://.planning/PROJECT.md#L62): `"- Maintenance commands (backup retention/cleanup, interrupted-sync recovery, richer doctor): milestone 2 baseline"`
  * [.planning/REQUIREMENTS.md#L48](file://.planning/REQUIREMENTS.md#L48) (`SAFE-05`): `"- [ ] **SAFE-05**: Backups are pruned by a retention rule (a bounded number per path), so the backup directory doesn't grow without limit"`
  * [.planning/research/SUMMARY.md#L67](file://.planning/research/SUMMARY.md#L67) vs [.planning/research/SUMMARY.md#L126](file://.planning/research/SUMMARY.md#L126): `"[accept; backup retention then moves into M1] (FEATURES)"`
* **Problem:** [PROJECT.md](file://.planning/PROJECT.md) explicitly puts "backup retention/cleanup" in **Out of Scope** for Milestone 1 and defers it to Milestone 2. Yet [REQUIREMENTS.md](file://.planning/REQUIREMENTS.md) promotes `SAFE-05` into Milestone 1 as an active, hard requirement without updating [PROJECT.md](file://.planning/PROJECT.md).
* **Why it matters:** Backing up *every* destructive write under any policy (`SAFE-03`) causes the `<git-dir>/wsx/backups` directory to grow aggressively with every hook run. If retention is implemented in M1, it violates the scope boundary in [PROJECT.md](file://.planning/PROJECT.md). If omitted, developer disk space leaks unbounded.
* **Suggested fix to requirement text:** Update [.planning/PROJECT.md#L62](file://.planning/PROJECT.md#L62) to remove "backup retention" from Out of Scope, and revise [.planning/REQUIREMENTS.md#L48](file://.planning/REQUIREMENTS.md#L48) (`SAFE-05`) to define the exact retention limit (e.g. `"Backups are pruned upon creation to retain at most N previous versions per managed path"`).

---

### 2. Negative Control Becomes Permanently Unexecutable Upon Python Deletion
* **Citation:**
  * [.planning/REQUIREMENTS.md#L17](file://.planning/REQUIREMENTS.md#L17) (`HARN-04`): `"- [ ] **HARN-04**: Every fixed defect and every deliberate difference has a fixed: scenario that passes on Rust and still fails on the oracle (the negative control)"`
  * [.planning/REQUIREMENTS.md#L81](file://.planning/REQUIREMENTS.md#L81) (`CUT-03`): `"- [ ] **CUT-03**: Python bin/wsx is deleted, and the harness keeps passing on Rust alone"`
  * [.planning/PROJECT.md#L54](file://.planning/PROJECT.md#L54): `"- [ ] Python bin/wsx deleted once the Rust binary passes the harness"`
* **Problem:** `HARN-04` requires that every fixed defect scenario asserts failure against the Python oracle. But `CUT-03` deletes [bin/wsx](file://bin/wsx). The moment [bin/wsx](file://bin/wsx) is deleted, `HARN-04` can never run in CI or local `cargo test`.
* **Why it matters:** The plan relies on `HARN-04` as the negative control against proxy measurement. Deleting the oracle either breaks the test suite post-cutover or turns `HARN-04` into an untestable phantom requirement. [.planning/research/STACK.md#L113](file://.planning/research/STACK.md#L113) recommended freezing oracle failures into golden snapshot files (`snapbox`), but that was completely omitted from [REQUIREMENTS.md](file://.planning/REQUIREMENTS.md).
* **Suggested fix to requirement text:** Update `HARN-04` and `CUT-03` to state:
  `"HARN-04: Prior to cutover, every fixed defect scenario verifies failure against the Python oracle via WSX_BIN. For post-cutover CI and regression testing, oracle failure outcomes are frozen into recorded snapshot fixtures that assert Rust's divergence from the oracle's baseline."`

---

### 3. Fail-Closed Guard Permanently Blocks Commits on Unborn HEAD and First Pushes Without `origin/main`
* **Citation:**
  * [.planning/REQUIREMENTS.md#L64](file://.planning/REQUIREMENTS.md#L64) (`GUARD-03`): `"- [ ] **GUARD-03**: The pre-push guard also runs on a branch's first push (no upstream)"`
  * [.planning/REQUIREMENTS.md#L65](file://.planning/REQUIREMENTS.md#L65) (`GUARD-04`): `"- [ ] **GUARD-04**: An internal error in the guard blocks the commit or push with a clear message (fails closed). WSX_SKIP=1 still bypasses"`
  * [bin/wsx#L803-L810](file://bin/wsx#L803-L810): [`base_ref()`](file://bin/wsx#L803-L810) defaults to `"origin/main"` if neither `wsx.baseRef` nor candidate branches exist.
  * [.planning/research/STACK.md#L37](file://.planning/research/STACK.md#L37): `"On an unborn HEAD (first commit), head_tree_id() fails; use the empty tree (MEDIUM, not executed)."`
* **Problem:** In any repo without an `origin/main` branch (e.g., local-only repos, repos with `upstream`, or new projects), the fallback base ref `"origin/main"` does not exist. Running git/gix commit range checks (`origin/main..HEAD`) fails with an internal resolution error. Furthermore, on an unborn HEAD (the repository's first commit), `gix` fails when resolving `head_tree_id()`. Under `GUARD-04`, any internal error causes the guard to **fail closed**.
* **Why it matters:** The guard will hard-block developers from making the initial commit of a repository and will permanently block the first push of a branch on repos without `origin/main`.
* **Suggested fix to requirement text:** Update `GUARD-03` and `GUARD-04` to state:
  `"GUARD-03: The pre-push guard verifies unpushed commits on a branch's first push against HEAD --not --remotes (or empty tree if no remotes exist). On an unborn HEAD or missing base ref, the guard evaluates staged paths against an empty tree rather than erroring, preventing fail-closed lockouts on initial commits."`

---

### 4. `serde_json` Rejects Lone-Surrogate Keys, Breaking the `base.json` v1→v2 Upgrade
* **Citation:**
  * [.planning/REQUIREMENTS.md#L24-L25](file://.planning/REQUIREMENTS.md#L24-L25) (`CORE-02`, `CORE-03`): `"Managed paths are handled as raw bytes, so non-UTF-8 filenames sync correctly"`, `"A 0.1.0 base.json (v1) is upgraded automatically on first run, including surrogate-escaped non-UTF-8 keys..."`
  * [bin/wsx#L220-L228](file://bin/wsx#L220-L228): [`save_base()`](file://bin/wsx#L220-L228) dumps keys using Python's `json.dumps()`, escaping invalid UTF-8 bytes as `\udc80`..`\udcff`.
  * [.planning/research/SUMMARY.md#L47](file://.planning/research/SUMMARY.md#L47): Dependencies: `"serde_json 1.0.154"`.
  * [.planning/research/PITFALLS.md#L220](file://.planning/research/PITFALLS.md#L220): `"N5: a lone-surrogate key makes serde_json reject the whole file [SOURCE]"`
* **Problem:** RFC 8259-compliant JSON parsers such as `serde_json` fail with `invalid unicode code point` when encountering lone surrogates (`\udcXX`). `serde_json` cannot deserialize Python's v1 `base.json` if it contains surrogate-escaped filenames. Furthermore, standard JSON object keys cannot be raw arbitrary bytes.
* **Why it matters:** The v1→v2 automatic migration in `CORE-03` will fail on any existing workspace that synced a non-UTF-8 path. Under `CORE-05` ("A corrupt or unreadable base.json stops wsx with an error"), wsx will refuse to run on that worktree.
* **Suggested fix to requirement text:** Update `CORE-03` and `CORE-04` to specify:
  `"CORE-03: The v1 base parser decodes JSON keys at the byte level, converting \udcXX escape sequences back to raw bytes (0xXX) without passing lone surrogates to serde_json."`
  `"CORE-04: base.json v2 encodes path keys as UTF-8 or hex/percent-escaped bytes, preserving valid JSON compatibility for non-UTF-8 paths."`

---

### 5. CAS Re-hash in `apply()` Downgrades Actions to Conflicts, Breaking `--dry-run` and Pure Planner Separation
* **Citation:**
  * [.planning/REQUIREMENTS.md#L31](file://.planning/REQUIREMENTS.md#L31) (`SYNC-01`): `"- [ ] **SYNC-01**: All sync decisions come from a pure planner (no filesystem, git, clock or env access). --dry-run reports exactly the actions a real run would apply"`
  * [.planning/REQUIREMENTS.md#L45](file://.planning/REQUIREMENTS.md#L45) (`SAFE-02`): `"- [ ] **SAFE-02**: Before overwriting or deleting a file, wsx re-hashes it. If it changed since planning, the action is downgraded to a conflict"`
  * [CLAUDE.md#L23-L25](file://CLAUDE.md#L23-L25): `"Sync decisions live entirely in plan(). apply() only executes what it is given... Put new decision logic in plan(), never in apply()."`
* **Problem:** `SAFE-02` mandates that `apply()` dynamically re-hashes files and changes overwrite/delete actions into conflicts. This is a sync decision made inside the executor, directly violating `SYNC-01` and the core architecture rule in [CLAUDE.md](file://CLAUDE.md).
* **Why it matters:** If `apply()` alters actions into conflicts during execution, `--dry-run` can never report those downgraded conflicts, falsifying `SYNC-01`'s claim that `--dry-run` reports "exactly the actions a real run would apply". Additionally, `apply()` must now handle conflict marker writes and state management on the fly.
* **Suggested fix to requirement text:** Reframe `SAFE-02` as an optimistic concurrency guard rather than a planner re-classification:
  `"SAFE-02: Before overwriting or deleting a file, the executor verifies its current hash matches the planner's expected observation hash. If mismatched (CAS failure), the executor aborts that action, writes no changes to that file or base, and reports a concurrent modification error."`

---

### 6. Missing Timestamp Preservation on File Copies Breaks `lww` Policy and Invalidates Stat Cache
* **Citation:**
  * [bin/wsx#L315-L321](file://bin/wsx#L315-L321): [`copy_file()`](file://bin/wsx#L315-L321) relies on `shutil.copy2()`, which preserves `mtime`.
  * [.planning/research/PITFALLS.md#L309](file://.planning/research/PITFALLS.md#L309): `"`std::fs::copy` copies content and permission bits only... It does not preserve mtime. Today the lww policy, the mtime-tie behaviour and the stat cache all rely on copy2's mtime preservation."`
  * [.planning/REQUIREMENTS.md#L33](file://.planning/REQUIREMENTS.md#L33) (`SYNC-03`), [.planning/REQUIREMENTS.md#L35](file://.planning/REQUIREMENTS.md#L35) (`SYNC-05`), [.planning/REQUIREMENTS.md#L50](file://.planning/REQUIREMENTS.md#L50) (`SAFE-07`).
* **Problem:** Rust's standard `std::fs::copy` assigns the current system time (`now`) to the copied file. Nowhere in `REQUIREMENTS.md` is `mtime` preservation required.
* **Why it matters:** If file writes set `mtime = now`, every file pulled or pushed under `lww` becomes newer than the original that was just copied. On the next sync run, the destination is falsely classified as modified and pushes back. The stat cache (`SAFE-07`) will also cache bogus timestamps.
* **Suggested fix to requirement text:** Add an explicit requirement under Safety:
  `"SAFE-08: Every copy operation preserves the source file's modification timestamp (mtime) on the destination file via set_times before atomic rename."`

---

### 7. Unspecified Mass-Deletion Threshold Suppresses Intentional Deletions in Non-Interactive Git Hooks
* **Citation:**
  * [.planning/REQUIREMENTS.md#L47](file://.planning/REQUIREMENTS.md#L47) (`SAFE-04`): `"- [ ] **SAFE-04**: If a run would delete more managed files than a set threshold, it applies nothing without --force and reports the count (the mass-deletion brake)"`
  * [.planning/REQUIREMENTS.md#L66](file://.planning/REQUIREMENTS.md#L66) (`GUARD-05`): `"- [ ] **GUARD-05**: Sync hooks never fail the git operation. Unknown hook names, panics, broken pipes and Ctrl-C all exit 0 in hook mode"`
  * [bin/wsx#L1117-L1121](file://bin/wsx#L1117-L1121): `setup`, `pull`, `push`, `sync` take only `-n/--dry-run`; `--force` exists only on `rm` ([bin/wsx#L1139](file://bin/wsx#L1139)).
* **Problem:** (1) "A set threshold" is completely undefined—no number, ratio, or config key exists in requirements, making `SAFE-04` unfalsifiable. (2) `push`, `pull`, and `sync` have no `--force` flag in 0.1.0. (3) In git hooks (`pre-commit`, `pre-push`), wsx runs non-interactively and fails open (exiting 0).
* **Why it matters:** If an agent or user intentionally deletes a managed directory exceeding the threshold, git hooks will silently suppress the deletions and exit 0. On the next `pull`, `session-start`, or sibling checkout, the remote worktree will restore the deleted files, resurrecting them.
* **Suggested fix to requirement text:** Update `SAFE-04` to state:
  `"SAFE-04: If a sync operation would delete more than wsx.maxDeletions (default: 5) files, it aborts all deletions and reports the threshold breach. Add --force to wsx sync/push/pull CLI options to override this brake. In hook mode, tripped brakes log a warning and exit 0 without applying deletions."`

---

### 8. Worktree ID Collision Between Main and Linked Worktrees Destroys Git Ignore Rules
* **Citation:**
  * [bin/wsx#L504-L505](file://bin/wsx#L504-L505): [`worktree_id()`](file://bin/wsx#L504-L505): `return ctx.git_dir.name if ctx.git_dir.name != ".git" else "main"`
  * [bin/wsx#L508-L510](file://bin/wsx#L508-L510): [`exclude_file()`](file://bin/wsx#L508-L510): `return ctx.common_dir / "info" / f"exclude.wsx-{worktree_id(ctx)}"`
  * [bin/wsx#L569-L570](file://bin/wsx#L569-L570): [`prune_orphan_excludes()`](file://bin/wsx#L569-L570) deletes `exclude.wsx-<id>` when `<git-dir>/worktrees/<id>` is missing.
  * [.planning/research/PITFALLS.md#L351](file://.planning/research/PITFALLS.md#L351): `"Worktree IDs collide. A linked worktree whose admin dir is named main already collides (CONCERNS)..."`
* **Problem:** If a user creates a linked worktree named `main` (`git worktree add ../main main`), its git administrative directory is `.git/worktrees/main`, producing `worktree_id = "main"`. The root worktree's git directory is `.git`, which also maps to `"main"`. Both worktrees overwrite `exclude.wsx-main`. When the linked worktree is removed, `prune_orphan_excludes()` unlinks `exclude.wsx-main`.
* **Why it matters:** The main worktree's `core.excludesFile` now points to a missing file. Personal planning files in the main checkout immediately appear as untracked files, and a subsequent `git add -A` sweeps them into git history. This bug was caught in [CONCERNS.md](file://.planning/codebase/CONCERNS.md) and [PITFALLS.md](file://.planning/research/PITFALLS.md), but omitted from [REQUIREMENTS.md](file://.planning/REQUIREMENTS.md).
* **Suggested fix to requirement text:** Add to Git requirements:
  `"GIT-06: Worktree exclude file names use unique worktree identifiers that prevent collisions between the main worktree and linked worktrees named 'main' (e.g. exclude.wsx-root vs exclude.wsx-wt-<name>), while prune_orphan_excludes recognizes both legacy and new naming."`

---

### 9. Symlink Policy Contradiction: Legacy Migration Unlinking vs Non-Migration Symlink Preservation
* **Citation:**
  * [.planning/REQUIREMENTS.md#L37](file://.planning/REQUIREMENTS.md#L37) (`SYNC-07`): `"- [ ] **SYNC-07**: Symlinks under managed paths (other than the old symlinked-layout migration) are never followed, replaced or deleted. They are reported"`
  * [.planning/REQUIREMENTS.md#L38](file://.planning/REQUIREMENTS.md#L38) (`SYNC-08`): `"- [ ] **SYNC-08**: Migration away from a symlinked workspace layout still works at parity, and an interruption mid-migration never leads to source deletions on the next run"`
  * [bin/wsx#L301-L312](file://bin/wsx#L301-L312): [`materialise_parents()`](file://bin/wsx#L301-L312) executes `current.unlink()` on *any* symlinked ancestor.
* **Problem:** `SYNC-07` states that symlinks under managed paths are "never followed, replaced or deleted", exempting only "the old symlinked-layout migration". However, neither [REQUIREMENTS.md](file://.planning/REQUIREMENTS.md) nor the codebase provides any heuristic or condition to distinguish an "old migration" symlink from an intentional user symlink.
* **Why it matters:** If the Rust engine implements 0.1.0's [`via_symlink()`](file://bin/wsx#L285-L298) and [`materialise_parents()`](file://bin/wsx#L301-L312), it unlinks and replaces *all* symlinks, destroying user symlinks (violating `SYNC-07`). If it refuses to unlink symlinks, legacy migration breaks (violating `SYNC-08`).
* **Suggested fix to requirement text:** Update `SYNC-07` and `SYNC-08` to specify the exact discriminator:
  `"SYNC-07: Symlinks whose targets resolve to the source worktree's corresponding path are treated as legacy layout migration symlinks (materialised to real files). All other symlinks under managed paths are skipped and reported without following, replacing, or unlinking."`

---

### 10. `wsx resolve --theirs` Errors on Modify-vs-Delete Conflicts, Causing Permanent Conflict Deadlocks
* **Citation:**
  * [bin/wsx#L707-L708](file://bin/wsx#L707-L708): [`cmd_resolve()`](file://bin/wsx#L686-L719):
    ```python
    if args.mine: ...
    else:
        if digest(src_path) is None:
            raise Problem(f"{rel} does not exist in the source worktree")
    ```
  * [.planning/REQUIREMENTS.md#L40](file://.planning/REQUIREMENTS.md#L40) (`SYNC-10`): `"- [ ] **SYNC-10**: wsx resolve --mine/--theirs gives the same results as 0.1.0, and --theirs never writes through a symlinked parent"`
  * [.planning/research/FEATURES.md#L65](file://.planning/research/FEATURES.md#L65): `"What the conflict sidecar holds when the source is absent is not covered."`
* **Problem:** If a file is deleted in the source worktree while modified in the local worktree, `plan()` flags a conflict. Under 0.1.0, running `wsx resolve <path> --theirs` crashes with `Problem: <rel> does not exist in the source worktree`.
* **Why it matters:** `SYNC-10` demands parity with 0.1.0 for `resolve`. Consequently, a developer can never accept the remote deletion using `--theirs`. The worktree remains permanently conflicted unless the user manually deletes the file and hacks the base snapshot.
* **Suggested fix to requirement text:** Update `SYNC-10` to state:
  `"SYNC-10: wsx resolve --theirs accepts source state even when the source file is deleted: it deletes the local file, removes the base entry, and clears any conflict markers, instead of raising an error."`

---

## ## SUSPECTED

### 11. Hermetic Environment Scrubbing May Invalidate `cargo-llvm-cov` Subprocess Instrumentation
* **Citation:**
  * [.planning/REQUIREMENTS.md#L15](file://.planning/REQUIREMENTS.md#L15) (`HARN-02`): `"- [ ] **HARN-02**: Every scenario is hermetic. Its HOME, XDG dirs, GIT_CONFIG_GLOBAL/GIT_CONFIG_SYSTEM and hooks are isolated from the host..."`
  * [.planning/REQUIREMENTS.md#L82](file://.planning/REQUIREMENTS.md#L82) (`CUT-04`): `"...cargo mutants on the planner (zero surviving mutants) all pass"`
  * [.planning/research/PITFALLS.md#L202](file://.planning/research/PITFALLS.md#L202): `"Coverage: cargo llvm-cov collects data from spawned binaries through environment variables (LLVM_PROFILE_FILE and others). A harness that calls env_clear() for hermeticity silently gets zero coverage [UNVERIFIED mechanism; verify in phase H]."`
* **Problem:** [SPECULATION] If the integration test runner executes `env_clear()` or scrubs all environment variables to enforce isolation, it will strip `LLVM_PROFILE_FILE`. Subprocess executions of the `wsx` binary under `assert_cmd` will exit without writing coverage profiles.
* **Why it matters:** Line and branch coverage assertions required for quality gates in [PROJECT.md](file://.planning/PROJECT.md) will report 0% coverage on CLI subcommands, misleading developers into believing commands are untested.
* **Suggested fix to requirement text:** In `HARN-02`, specify that hermetic environment scrubbing must explicitly allow-list `LLVM_*` profile variables when running under test harnesses.

---

### 12. Negative Control Model Incompatible with Intentional Regressions (Reftables)
* **Citation:**
  * [.planning/REQUIREMENTS.md#L17](file://.planning/REQUIREMENTS.md#L17) (`HARN-04`): `"- [ ] **HARN-04**: Every fixed defect and every deliberate difference has a fixed: scenario that passes on Rust and still fails on the oracle (the negative control)"`
  * [.planning/REQUIREMENTS.md#L56](file://.planning/REQUIREMENTS.md#L56) (`GIT-03`): `"- [ ] **GIT-03**: A reftable repo is refused with a clear message, never a crash or partial run"`
  * [.planning/research/STACK.md#L45](file://.planning/research/STACK.md#L45): `"The Python oracle works on these repos, so this is a parity regression."`
* **Problem:** [SPECULATION] On reftable repos, the Python oracle **succeeds** (exit 0) because it invokes the host `git` CLI, whereas Rust **fails/refuses** (exit 1/2) because `gix 0.89` lacks reftable support. This is an intentional regression, not a defect fix. `HARN-04` asserts that scenarios must "pass on Rust and still fail on the oracle", which is inverted here.
* **Why it matters:** The harness cannot express deliberate regressions under the `fixed:` test convention without test runner hacks or inverted assertions.
* **Suggested fix to requirement text:** Update `HARN-04` to distinguish defect fixes from deliberate regressions:
  `"HARN-04: Fixed defect scenarios assert success on Rust and failure on the oracle. Deliberate regression scenarios (e.g. GIT-03 reftable refusal) assert expected refusal on Rust and success on the oracle."`
