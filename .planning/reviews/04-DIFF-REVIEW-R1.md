# Phase 04 Tier A diff review (D-28), round 1

Target: `383c74b..5765862` over `src/` (plans 04-13 to 04-18). Lens: adversarial. Date: 2026-11-18.

| Lane | Result |
|------|--------|
| agy (gemini-3.8-flash-high) | DROPPED: print timeout after 30m, turn still compiling tests; 49 bytes of partial output, counted as no review |
| codex (gpt-6.1-sol, high) | 1 claimed critical, 3 major, 1 minor; each reproduced by the lane |
| pi (deepseek-flash) | no critical; 5 minor, 3 suspected; probes on git 2.56.1 |

Each finding below was checked against the cited source lines by the orchestrator.

## Critical after verification

None. codex's critical (C1) is downgraded, with the reason given below.

## Findings

- **C1, claimed critical, downgraded to known limitation.** `src/git/refs.rs:244-250`: the replace-ref stores are checked after gix built its replacement map, so a replace ref removed between `Repo::discover` and `resolve` in one run leaves gix's `tree_diff` applying it. Already documented at `src/git/refs.rs:222-225`. It needs a concurrent mutation mid-run, and `tree_diff` (`src/git/reads.rs:123`) is not called by the model, whose object reads now go through `git cat-file`. Backlog: whichever phase first calls `tree_diff` re-checks the stores after the read, or moves the read to git.
- **C2 / P5, major: unbounded child on a fifo.** `src/git/spawn.rs:78` (`command.output()`, no deadline). A fifo loose ref under `HEAD` plus an `includeIf "onbranch:"` (codex), or a fifo `.git/config` / `packed-refs` (pi), hangs `resolve`. git hangs the same way (pi: `git status` too), so the answer matches git, but D-29's aim "the guard never hangs a push" is met for loose refs only (`fifo_ref_is_skipped_without_hanging`). Found by both lanes.
- **C3, major: exit 1 with empty stderr is read as "no keys" whatever stdout held.** `src/git/config.rs:73-78`; `src/git/spawn.rs:88-98` drops stdout on failure. Reachable only through a substituted `git` (fault injection); pi rates the same shape speculative (S1).
- **C4, major: malformed scope or origin metadata gives fewer keys, not `GitBadOutput`.** `src/git/config.rs:135` and `:163`. Fault injection only: real git emits neither shape.
- **C5 / P4, minor: `parse_batch` panics on a size near `usize::MAX`.** `src/git/objects.rs:103` (`size + 1`). The panic stops the run (not fail-open). Both lanes found it; the fix is `checked_add`.
- **P1, minor: `.git/config` absent plus a matching global key gives `Error::Io`.** `src/git/config.rs:82-86`. Fail-closed wrong refusal.
- **P2, minor: a bare repository spawns 5 children, not D-26's 4.** `src/git/spawn.rs:170-177`. No test covers bare repos.
- **P3, minor: a successful child's stderr is discarded.** `src/git/spawn.rs:88-98`. git's "ignoring broken ref" warning never reaches the user when D-35 skips the ref.
- **S2, test wording:** `gitlink_whose_commit_is_absent_is_not_a_blob`'s control message reads inverted.

## Disposition

The stop rule (no critical remains) is met after round 1. The operator chose to fold the majors (2026-11-18).

- C3 and C4: fixed, test first. The RED tests failed at their intended assertions and their controls passed; `scripts/check.sh all` passes after the fix.
- C2: fixed (D-39, the operator chose a 10 s limit on every git child). The RED tests ran the full 5 s sleep and returned Ok; after the fix they stop at the deadline. A real-git regression pins the fifo plus `includeIf "onbranch:"` shape, where `timeout 3 git config` itself exits 124. `scripts/check.sh all` and `scripts/check-in-container.sh all` pass (704 tests). The per-child exit poll is 50 µs: 1 ms added about 0.8 ms per child; 50 µs matches a plain spawn (one run of 200 calls each).
- C1, C5, P1 to P3 and S2: backlog.

Raw lane outputs are kept at /tmp/claude-1000/-var-home-denniyahh-Github-wsx/cde60fea-b986-4913-8ee2-9f1cc80fb5a1/scratchpad/review-r1/review-{codex,pi,agy}.md.
