# Requirements: wsx

**Defined:** 2026-10-18
**Core Value:** wsx must never lose or silently clobber a user's personal files. Every sync decision is either correct, reported as a conflict, or backed up.

Milestone 1 = Rust parity + hardened. "Parity" means exit codes, filesystem effects, base-state
effects and status classification match the Python oracle. Exact stdout wording does not count.
The oracle is `bin/wsx` at `origin/develop` (`d693c9e`, 0.1.0 plus the issue #1 fix, plain
`.git/hooks` guard shims and `--no-track`), not the 0.1.0 tag on `main`. Every intended difference
is a fixed defect or a decision listed here, and has a harness scenario.

## v1 Requirements

### Harness (black-box parity suite)

- [ ] **HARN-01**: The same scenario suite runs unchanged against the Python oracle and the Rust binary, selected by `WSX_BIN`
- [ ] **HARN-02**: Every scenario is hermetic. Its HOME, XDG dirs, `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` and hooks are isolated from the host, and the host's global hooks never fire. `LLVM_*` profiling variables are passed through so coverage still sees the spawned binary
- [ ] **HARN-03**: Hook scenarios exercise the binary under test, installed at the sandbox `$HOME/.local/bin/wsx` that the hook scripts call
- [ ] **HARN-04**: Every scenario asserts explicit expected outcomes (exit code, filesystem and base-state effects), never only "same as the oracle". The expected oracle outcome is recorded in the scenario, so the evidence survives cutover. Two classes of deliberate difference:
  - `fixed:` scenarios pass on Rust and fail on the oracle before cutover
  - `regression:` scenarios (deliberate refusals, e.g. reftable) succeed on the oracle and are refused by Rust
- [ ] **HARN-05**: The suite runs from `cargo test`/nextest, so coverage and mutation testing see it
- [ ] **HARN-06**: Permission-based scenarios refuse to run as root, so they can't pass trivially

### Core (types, manifest, base snapshot)

- [ ] **CORE-01**: Every `.wsx.conf` valid for the oracle parses to the same entries and policies. A differential proptest checks this against the Python parser before cutover, with its cases kept as fixtures after
- [ ] **CORE-02**: Managed paths are handled as raw bytes, so non-UTF-8 filenames sync correctly
- [ ] **CORE-03**: A v1 `base.json` is upgraded automatically on first run. Surrogate-escaped keys (`\udcXX`) are decoded back to their raw bytes without going through a strict JSON string decoder. The original is kept as `base.v1.json.bak` before the first v2 write. No spurious conflicts or transfers result
- [ ] **CORE-04**: `base.json` v2 stores one agreed hash per path and a separate stat record for each side. Path keys are encoded reversibly: valid UTF-8 as-is, any other byte escaped. The file is written temp + fsync + rename. The oracle and Rust never share one state file in tests
- [ ] **CORE-05**: A corrupt, unreadable or unknown-version `base.json` is never treated as empty and never overwritten. Commands stop with an error naming the file. Hook mode skips, exits 0, and records the reason where `status` and `doctor` show it
- [ ] **CORE-06**: Discovery and policy resolution match the oracle:
  - `.git`, `node_modules`, `.DS_Store`, `Thumbs.db` and `*.wsx-remote` are skipped on both sides
  - an uncovered path defaults to `skip`
  - the longest manifest match wins

### Sync (planner semantics)

- [ ] **SYNC-01**: All sync decisions come from a pure planner (no filesystem, git, clock or env access). `--dry-run` reports exactly the planned actions. The only difference a real run can show is a compare-and-swap abort (SAFE-02), which is reported
- [ ] **SYNC-02**: An unreadable local or source file or directory is never planned as a deletion. It is reported, and its subtree is left untouched (fixes the unreadable-file and unreadable-directory data loss)
- [ ] **SYNC-03**: Under `lww`, a pull never deletes a local file that was never synced. It is pushed instead (fixes lww local-only deletion)
- [ ] **SYNC-04**: When a file is deleted on both sides, its base entry is removed, so recreating the file later is not reported as a conflict
- [ ] **SYNC-05**: Under `lww`, identical mtimes with different content are reported as a conflict (`<file>.wsx-remote`), not resolved silently
- [ ] **SYNC-06**: The snapshot includes, for the target worktree, which paths git tracks (in the index or in HEAD). A managed path that is tracked, or has a tracked descendant, is blocked and reported in `sync`, `status` and `doctor`. wsx never writes a tracked file
- [ ] **SYNC-07**: A symlink at or above a managed path, on either the local or the source side, blocks that path. wsx performs no copy, unlink, parent cleanup, base update or marker write, and reports it. The only exception is the legacy layout migration defined in SYNC-08. Tests cover source-side, local-side and ancestor links
- [ ] **SYNC-08**: Legacy migration is defined as a local path reached through a symlink that resolves into the source worktree. It migrates at parity, and an interruption mid-migration never leads to source deletions on the next run
- [ ] **SYNC-09**: Leftover wsx temp files are never synced as user files
- [ ] **SYNC-10**: `wsx resolve --mine/--theirs` gives the same results as the oracle, and `--theirs` never writes through a symlinked parent
- [ ] **SYNC-11**: wsx records the conflict markers it writes. A pre-existing `<file>.wsx-remote` that wsx didn't create for that conflict is reported as a marker collision, and is never overwritten or deleted
- [ ] **SYNC-12**: Every copy preserves the source's mtime and permission bits, as the oracle's `shutil.copy2` does. `lww` and the stat cache depend on it

### Safety (filesystem execution)

- [ ] **SAFE-01**: Only one wsx run per repo applies changes at a time, across all worktrees. A hook run waits at most 2 s for the lock (one named constant, tested just under and just over it), then skips, exits 0, and records the skip where `status` and `doctor` show it
- [ ] **SAFE-02**: Before overwriting or deleting a file, the executor checks that it still matches the hash the plan expected. On a mismatch it aborts that one action: no write, no base change, no marker. It reports the abort and continues with the rest
- [ ] **SAFE-03**: The planner attaches a backup instruction to every destructive action:
  - overwrite on pull or push, under any policy
  - `delete-local` and `delete-remote`
  - `resolve`
  - marker replacement

  The executor backs up before acting. Each action kind is tested on its own. Backup names never collide, even within one second
- [ ] **SAFE-04**: The mass-deletion brake counts the planned `delete-local` and `delete-remote` actions before anything is applied. When the count is above the threshold (default 10, git config `wsx.maxDeletions`):
  - `sync`/`push`/`pull` apply nothing and exit non-zero unless `--force` (new on those commands)
  - hook mode applies the non-delete actions, skips every delete, exits 0, and records the trip where `status` and `doctor` show it

  Tests cover threshold − 1, threshold, threshold + 1, and `--force`
- [ ] **SAFE-05**: After each backup, older backups of the same path are pruned automatically, keeping the newest 10. No separate maintenance command is needed
- [ ] **SAFE-06**: A crash or kill at any point during apply leaves no data lost, and the next run converges correctly. File data is written and fsynced before the base is updated. Fault-injection tests check this
- [ ] **SAFE-07**: Unchanged files are not re-hashed on every hook. A stat cache skips hashing, but never authorises a destructive action on its own
- [ ] **SAFE-08**: Backups are stored under the common git dir (`<common-dir>/wsx/backups/<worktree-id>/`), so they survive `wsx rm` removing the worktree's own git dir. A test removes a worktree and recovers its final-push backup

### Git (adapter)

- [ ] **GIT-01**: wsx resolves the right worktree, git-dir and common-dir even when the hook environment exports `GIT_DIR`/`GIT_INDEX_FILE`. Parity with the oracle's issue #1 fix, extended to every git call made from a hook
- [ ] **GIT-02**: Child processes wsx starts get a scrubbed git environment (the variables `git rev-parse --local-env-vars` lists) and a null stdin
- [ ] **GIT-03**: A reftable repo is refused with a clear message, never a crash or partial run (a `regression:` scenario)
- [ ] **GIT-04**: A repo owned by another user (reduced trust) is refused, as git does, never processed with repo config silently ignored
- [ ] **GIT-05**: A user's custom global `core.excludesFile` is kept in the generated per-worktree ignore file
- [ ] **GIT-06**: Worktree ids are collision-free. A linked worktree named `main` doesn't share an exclude file with the main checkout. Pruning recognises the legacy `exclude.wsx-<id>` names and never removes the live main checkout's file

### Guard and hooks

- [ ] **GUARD-01**: The commit guard blocks staged managed paths that contain non-ASCII or special characters (fixes quoted paths slipping through). Staged names are read NUL-separated and repo-relative, so `diff.relative` cannot hide a path either
- [ ] **GUARD-02**: The pre-push guard checks every commit in the range it computes from git state, using every commit's touched paths, so a managed file added and later removed, or renamed into a managed path, is still caught (a deliberate difference from the oracle). `git push origin <other-branch>` remains a documented gap until milestone 2's HOOK-02
- [ ] **GUARD-03**: The pre-push guard also runs on a branch's first push (no upstream), and the commit guard works on a repo's first commit (unborn HEAD)
- [ ] **GUARD-04**: The guard fails closed only once wsx has confirmed the repo is wsx-configured. An internal error after that point blocks the commit or push with a clear message. Unconfigured repos and repos wsx refuses (reftable, another owner) exit 0, so the machine-wide hook never blocks unrelated repos. `WSX_SKIP=1` still bypasses
- [ ] **GUARD-05**: Sync hooks never fail the git operation. Unknown hook names, panics, broken pipes and Ctrl-C all exit 0 in hook mode
- [ ] **GUARD-06**: Agent session hooks (Claude Code, Copilot) sync the worktree named in the stdin JSON `cwd`, not the process cwd. Only those hooks read stdin, and only when it isn't a terminal, with a size bound. Git hook paths never read stdin
- [ ] **GUARD-07**: Plain `.git/hooks` guard shims for repos without husky work at parity: same marker, never overwrite a foreign hook, leave `core.hooksPath` repos alone, re-arm on session start. `doctor` checks the shim is executable, not just present, and when `core.hooksPath` is set says so and shows how to chain `wsx hook <name>` from that manager

### Lifecycle

- [ ] **LIFE-01**: `wsx new` creates the worktree and branch from the base ref with `--no-track` and the same `post-checkout` behaviour as the oracle, then arms the guard (husky or git hook shims) and runs setup
- [ ] **LIFE-02**: `wsx rm` pushes personal state back before removing, refuses on conflicts unless `--force`, and identifies the main worktree correctly, including with `--separate-git-dir` and symlinked paths
- [ ] **LIFE-03**: `wsx list` and `wsx doctor` report the same states as the oracle, plus:
  - skipped, symlink, tracked-path and marker-collision reports
  - deletion-brake trips and skipped hook runs
  - an unreadable base
  - a husky repo whose `.husky/pre-push` is missing, so the push guard can't run
- [ ] **LIFE-04**: wsx never holds its lock while running a subprocess that can fire git hooks (e.g. `git worktree add`, `npm install`). It re-acquires the lock and re-plans before applying anything afterwards
- [ ] **LIFE-05**: `wsx new` derives the fetch remote correctly for local base refs such as `feature/x`

### Cutover

- [ ] **CUT-01**: `install.sh` installs the Rust binary in place of the Python script, and the existing husky, plain git hook, Claude, Copilot and fish integrations keep working
- [ ] **CUT-02**: The author's existing worktrees keep working after swapping the binary, verified on real state
- [ ] **CUT-03**: Python `bin/wsx` is deleted, and the harness keeps passing on Rust alone with every recorded expectation intact
- [ ] **CUT-04**: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` and `cargo deny check` pass. `cargo mutants` on the planner module leaves zero surviving mutants, apart from any listed as equivalent, each with a justification

## v2 Requirements

Milestone 2: public baseline release. Tracked, not in the current roadmap.

### Hooks without husky

- **HOOK-01**: Config-based hooks on git ≥ 2.54 (per-worktree, alongside `core.hooksPath`), extending the plain `.git/hooks` shims
- **HOOK-02**: Pre-push uses the real pushed refs
- **HOOK-03**: A worktree created by any tool (not only `wsx new`) is set up through a repo-level `post-checkout` hook

### Agent integrations

- **AGNT-01**: Codex CLI session integration
- **AGNT-02**: Antigravity session integration (Antigravity CLI, which replaced Gemini CLI for unpaid users on 2026-06-18)
- **AGNT-03**: Every agent integration (Claude Code, Copilot, Codex, Antigravity) pushes at the end of each turn; session-end push stays as a best-effort extra. Consistent across runtimes (owner decision 2026-10-19). Depends on the M1 stat cache and lock (SAFE-01, SAFE-07). Acceptance must cover: parallel agents finishing turns at once (lock losers skip and retry next turn, nothing lost), and a mid-task edit reaching the source before the task ends (other worktrees see it only on their next pull)
- **AGNT-04**: Confirm whether Claude Code's worktree auto-removal keeps unpushed gitignored files

### Maintenance

- **MAINT-01**: Backup inspection and restore command
- **MAINT-02**: Richer `doctor` (nested symlinks, stale locks, interrupted runs)

### Generality and distribution

- **GEN-01**: Base-ref default chain no longer team-specific (`origin/devtest` first)
- **GEN-02**: Reftable support resolved before the public release, either through gix or a git-binary fallback
- **DIST-01**: Versioned release binaries and a documented install for strangers

## Out of Scope

| Feature | Reason |
|---------|--------|
| macOS / Windows | Linux only for now; their locking and symlink semantics add scope |
| Interactive prompts | wsx runs inside git hooks and agent hooks |
| Writing tracked files (`.gitignore`, `.husky/`) | A load-bearing design rule |
| Resolving conflicts on the user's behalf | A design rule. Conflicts keep both sides |
| One-shot copy-at-create (like `.worktreeinclude`) as the main mode | Continuous two-way sync is the product |
| Gemini CLI adapter | Antigravity CLI replaces it; owner decision 2026-10-19 |
| Live-SQLite-consistent `lww` copies | Unverified risk (review 2026-10-19); M1 `lww` is a byte copy and the docs must not promise more |
| Broader "other developer repo needs" features | To be explored after the milestone 2 baseline ships |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|

**Coverage:**
- v1 requirements: 54 total
- Mapped to phases: 0 (roadmap pending)
- Unmapped: 54

---
*Requirements defined: 2026-10-18*
*Last updated: 2026-10-19 after adversarial review (`.planning/reviews/m1-adversarial/SUMMARY.md`) and rebasing the oracle on `origin/develop`*
