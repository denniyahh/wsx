## CONFIRMED

1. **Citation:** [REQUIREMENTS.md:27](.planning/REQUIREMENTS.md:27) says corrupt `base.json` “stops wsx” and is “never treated as empty.” [ARCHITECTURE.md:304](.planning/research/ARCHITECTURE.md:304) says corrupt JSON should be treated as empty and merely warned about.

   **Problem:** Direct contradictory safety behavior.

   **Why it matters:** Empty history resurrects deletions and destroys conflict provenance; the Python oracle does exactly this via [bin/wsx:211](bin/wsx:211).

   **Suggested requirement text:** “A malformed, unreadable, or future-version state file is a hard error in interactive mode and a reported no-op in hook mode; it is never interpreted as an empty base. Tests corrupt JSON and assert neither managed tree nor state is rewritten.”

2. **Citation:** [REQUIREMENTS.md:48](.planning/REQUIREMENTS.md:48) makes retention an M1 requirement. [PROJECT.md:62](.planning/PROJECT.md:62) defers backup retention to M2; [FEATURES.md:200](.planning/research/FEATURES.md:200) does the same.

   **Problem:** M1 scope is contradictory.

   **Why it matters:** Backup-before-every-destructive-write multiplies storage immediately. Deferring pruning means the M1 safety mechanism has unbounded disk use.

   **Suggested requirement text:** “M1 performs automatic retention after a successful backup, retaining exactly the documented bound per logical path; no M2 maintenance command is required.” Remove the M2 deferral.

3. **Citation:** [REQUIREMENTS.md:46](.planning/REQUIREMENTS.md:46) requires a backup before *every* overwrite or delete. The proposed action model has `DeleteRemote { expect_src: Sha256 }` with no backup instruction at [ARCHITECTURE.md:172](.planning/research/ARCHITECTURE.md:172), despite requiring the executor to make no decisions at [ARCHITECTURE.md:161](.planning/research/ARCHITECTURE.md:161).

   **Problem:** The architecture cannot meet SAFE-03 without violating its own “executor only executes” rule.

   **Why it matters:** The current oracle deletes without a backup at [bin/wsx:474](bin/wsx:474). This is exactly the direction most likely to lose the authoritative source copy.

   **Suggested requirement text:** “Every destructive action carries an explicit backup plan for its target, including `delete-local`, `delete-remote`, overwrite, conflict-marker replacement, and marker cleanup; tests exercise each action independently.”

4. **Citation:** [REQUIREMENTS.md:37](.planning/REQUIREMENTS.md:37) says non-migration symlinks are “never followed, replaced or deleted.” The architecture says `Obs::Symlink` is explicitly mapped to absent at [ARCHITECTURE.md:159](.planning/research/ARCHITECTURE.md:159).

   **Problem:** Treating a symlink as absent recreates the Python deletion/replacement behavior the requirement prohibits.

   **Why it matters:** The oracle’s `digest()` converts symlinks to `None` at [bin/wsx:237](bin/wsx:237), and a `pull` can then unlink the destination symlink at [bin/wsx:317](bin/wsx:317).

   **Suggested requirement text:** “A leaf or ancestor symlink outside the explicitly detected legacy migration layout produces `Blocked(Symlink)`; it performs no copy, unlink, parent cleanup, base update, or marker write. Include source-side, local-side, and ancestor-symlink negative cases.”

5. **Citation:** [REQUIREMENTS.md:36](.planning/REQUIREMENTS.md:36) requires tracked manifest paths to be skipped. The declared planner input contains only source/local observations and marker state at [ARCHITECTURE.md:150](.planning/research/ARCHITECTURE.md:150); the Git trait has no tracked-path operation at [ARCHITECTURE.md:336](.planning/research/ARCHITECTURE.md:336).

   **Problem:** The proposed plan has no data source from which `plan()`, `status`, or `doctor` can implement SYNC-06.

   **Why it matters:** The current oracle never checks the index before copying, as the audit confirms at [PITFALLS.md:321](.planning/research/PITFALLS.md:321). This permits writes to team-owned content.

   **Suggested requirement text:** “Snapshot construction includes byte-preserving tracked-path membership for the target worktree, defined against the index and HEAD. Any exact path or descendant tracked by either definition is `Blocked(Tracked)`, with tests for tracked file, tracked descendant, staged deletion, and `doctor` reporting.”

6. **Citation:** [PROJECT.md:80](.planning/PROJECT.md:80) permits Git CLI fallback only where gix lacks required functionality. The research summary says gix 0.89 supports config writes and worktree removal and explicitly resolves that Stack wins at [SUMMARY.md:36](.planning/research/SUMMARY.md:36) and [SUMMARY.md:115](.planning/research/SUMMARY.md:115). The architecture nevertheless routes both to CLI at [ARCHITECTURE.md:359](.planning/research/ARCHITECTURE.md:359) and [ARCHITECTURE.md:364](.planning/research/ARCHITECTURE.md:364).

   **Problem:** The routing plan contradicts the project decision and its own adjudicated research.

   **Why it matters:** It silently broadens subprocess surface, environment-scrubbing burden, and divergent semantics under hooks.

   **Suggested requirement text:** “M1 uses gix 0.89 for config writes and worktree removal after differential fixtures prove parity; CLI fallback is limited to `worktree add -b` and fetch, unless a documented failing gix fixture is added.”

7. **Citation:** [REQUIREMENTS.md:23](.planning/REQUIREMENTS.md:23) promises raw-byte path handling, but does not define v2 encoding or coexistence. The architecture’s JSON example uses path strings as object keys at [ARCHITECTURE.md:284](.planning/research/ARCHITECTURE.md:284), while the pitfalls document requires byte encoding and a separate v2 filename at [PITFALLS.md:224](.planning/research/PITFALLS.md:224). It also says Python reading an in-place v2 causes conflicts at [ARCHITECTURE.md:305](.planning/research/ARCHITECTURE.md:305).

   **Problem:** The persistence protocol is simultaneously required, undecided, and incompatible with its stated raw-byte guarantee.

   **Why it matters:** A filename containing `0xff` produces a lone surrogate in v1; naive `serde_json` rejects it, as recorded at [PITFALLS.md:17](.planning/research/PITFALLS.md:17).

   **Suggested requirement text:** “Specify the v2 filename, exclusive ownership/cutover rule, rollback artifact, and reversible ASCII encoding for every path byte. Fixtures must upgrade a v1 lone-surrogate key, then prove byte-identical path recovery and no mixed Python/Rust-state corruption.”

8. **Citation:** The oracle excludes every `*.wsx-remote` file from discovery at [bin/wsx:264](bin/wsx:264), overwrites that name when reporting a conflict at [bin/wsx:435](bin/wsx:435), and later deletes it at [bin/wsx:478](bin/wsx:478). SAFE-03 is broad but neither it nor HARN-04 requires a marker-collision case.

   **Problem:** A user file already named `<managed-file>.wsx-remote` is invisible to sync and is later overwritten or deleted as tool state.

   **Why it matters:** This is silent data loss under a suffix the tool reserves without enforcing or escaping it.

   **Suggested requirement text:** “If a conflict-marker destination pre-exists and is not a marker created by this conflict, report `MarkerCollision` and leave it untouched; do not overwrite or delete it. Test a user-created suffix file as the negative control.”

9. **Citation:** [REQUIREMENTS.md:47](.planning/REQUIREMENTS.md:47) says only “more … than a set threshold.” Research defines materially different triggers—missing root versus count threshold—at [FEATURES.md:70](.planning/research/FEATURES.md:70).

   **Problem:** SAFE-04 has no threshold source, direction/counting definition, mixed-plan behavior, or boundary tests. It is not falsifiable.

   **Why it matters:** An implementation can choose a useless threshold, count only local deletes, or copy half the plan before discovering the brake—each technically “implements” the prose while defeating the safeguard.

   **Suggested requirement text:** “Define a documented nonzero default/configured threshold; count selected `delete-local` and `delete-remote` actions before apply; if `count > threshold`, apply zero mutations or base updates unless the exact invocation has `--force`. Test threshold−1, threshold, threshold+1, missing-root, and forced cases.”

## SUSPECTED

1. **SPECULATION — Citation:** The README markets `lww` for SQLite at [README.md:27](README.md:27), while research defers SQLite-aware copy semantics and labels live-copy corruption risk unverified at [FEATURES.md:123](.planning/research/FEATURES.md:123).

   **Problem:** M1 parity retains raw per-file `lww`, but its public description implies it is safe for live SQLite state.

   **Why it matters:** A database plus WAL/SHM can be copied from inconsistent moments. The research has not reproduced this with wsx, so this is not a confirmed defect.

   **Suggested requirement text:** “M1 `lww` is byte-copy only and does not support live SQLite databases; when `-wal` or `-shm` siblings are present, block and report rather than copy. Remove SQLite from M1 documentation unless an atomic database-consistent protocol is tested.”
