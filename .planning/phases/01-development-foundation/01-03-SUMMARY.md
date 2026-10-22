---
phase: 01-development-foundation
plan: 03
subsystem: infra
tags: [devcontainer, docker, dockerfile, dependabot, bash, check-in-container, worktree, nextest]

requires:
  - phase: 01-01
    provides: "scripts/check.sh (fmt, clippy, test, all; prints `==> rustc ...`), rust-toolchain.toml pinning 1.99.1, .config/nextest.toml writing a JUnit report"
  - phase: 01-02
    provides: "scripts/install-tools.sh (pinned, sha256-checked cargo-nextest and cargo-deny installs), scripts/check.sh deps"
provides:
  - ".devcontainer/Dockerfile: the single exact image pin, FROM mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie (a Dependabot docker-ecosystem target)"
  - ".devcontainer/devcontainer.json: builds from the Dockerfile; no image key, no Features; volumes wsx-cargo-registry, wsx-cargo-git, wsx-target-${localWorkspaceFolderBasename}"
  - "scripts/check-in-container.sh [TARGET] [ARGS...]: runs scripts/check.sh in the pinned image from any checkout or linked worktree; source and git dirs bound read-only; per-checkout volumes wsx-ci-target-<12 hex> (at /workspace/target) and wsx-ci-registry-<12 hex>; exit 1 on an unreadable FROM line or unreachable daemon"
affects: [01-04 CI jobs and assert-image-parity.sh read the Dockerfile FROM line, 01-06 docs (CLAUDE.md/CONTRIBUTING describe the container gate), 01-07 removes the wsx-ci-* volumes at phase end]

actuals:
  tokens: 2143
  tasks: 2
  commits: 2
plan_head_before: 487a32eeb730e0e66a82ec065e1ed1f1561a3855
plan_head_after: 0c45ee2a13ae5f196caa55f665fef0ca11d3a5ae

tech-stack:
  added: [mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie (digest sha256:269f7cd057eaccb52c17e9461f42ac5cc1bff564b0723be49ea91726f5753bb7)]
  patterns:
    - "The image is named once, in the Dockerfile FROM line; every consumer reads it from there"
    - "Containerised verification mounts the source and git directories read-only; only named volumes are writable"
    - "Writable paths that a tool fixes relative to the workspace root (nextest's store) get a volume mounted over them, not an env-var redirect"

key-files:
  created:
    - .devcontainer/Dockerfile
    - .devcontainer/devcontainer.json
    - scripts/check-in-container.sh
  modified: []

key-decisions:
  - "The container's target volume is mounted at /workspace/target (over the read-only checkout) instead of /ctarget with CARGO_TARGET_DIR, because nextest writes its store and JUnit report under <workspace root>/target/nextest regardless of CARGO_TARGET_DIR (orchestrator-approved Option A)"
  - "check-in-container.sh runs `mkdir -p \"$REPO_ROOT/target\"` on the host before docker run, because Docker cannot create a mount point inside a read-only bind; target/ is git-ignored and the host gate creates it anyway"
  - "No rustup volume: each run installs 1.99.1 into the throwaway container (about 17 s, network), as the plan chose"

patterns-established:
  - "Changing where a cargo target volume is mounted requires a fresh volume: fingerprints keep the old CARGO_BIN_EXE_* paths and the tests fail"

requirements-completed: [DEV-06, DEV-02, DEV-01]

coverage:
  - id: D1
    description: "The Dockerfile has exactly one FROM line with the exact tag, devcontainer.json builds from it with no image key and no Features, and the tag resolves to the researched digest"
    requirement: DEV-06
    verification:
      - kind: other
        ref: "Task 1 verify #1-#3: FROM value equals the tag and `rg -c '^FROM'` = 1; jq prints true (and false for copies with an image key or a Feature); RepoDigests[0] equals sha256:269f7cd0...; image rustc prints `rustc 1.99.0 (b940084d7 2026-09-28)`"
        status: pass
    human_judgment: false
  - id: D2
    description: "check-in-container.sh all passes from the linked phase worktree in the pinned image, binding the worktree gitdir and common dir, with rustc 1.99.1 selected by rust-toolchain.toml, no CR in non-TTY output, no `command not found`, and git status unchanged"
    requirement: DEV-02
    verification:
      - kind: other
        ref: "Task 2 verify #1 (exit 0, `==> rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `==> check.sh: all OK`) plus git status --porcelain diff before/after (identical)"
        status: pass
    human_judgment: false
  - id: D3
    description: "check-in-container.sh deps installs the pinned tools in the container and passes cargo-deny"
    requirement: DEV-01
    verification:
      - kind: other
        ref: "Task 2 verify #2: exit 0, `advisories ok, bans ok, licenses ok, sources ok`, `==> check.sh: deps OK`"
        status: pass
    human_judgment: false
  - id: D4
    description: "With the Docker daemon unreachable the script exits 1 with `Docker daemon is not reachable`"
    requirement: DEV-06
    verification:
      - kind: other
        ref: "Task 2 verify #3: DOCKER_HOST=unix:///nonexistent/docker.sock scripts/check-in-container.sh test -> exit 1"
        status: pass
    human_judgment: false
  - id: D5
    description: "Per-checkout volumes exist under wsx-ci-target-<hash> and wsx-ci-registry-<hash>"
    requirement: DEV-06
    verification:
      - kind: other
        ref: "Task 2 verify #4: docker volume inspect wsx-ci-target-4fa63246f2d5 wsx-ci-registry-4fa63246f2d5"
        status: pass
    human_judgment: false
  - id: D6
    description: "The source checkout and git directories cannot be written from the container, including with the nested writable target mount in place"
    requirement: DEV-06
    verification:
      - kind: other
        ref: "Task 2 verify #5 (touch /workspace/.ro-probe -> Read-only file system; no probe on host; script has :/workspace:ro and :$dir:ro)"
        status: pass
      - kind: other
        ref: "Added control with the script's exact mount layout: touch Cargo.toml, src/new.rs, gitdir/probe, commondir/probe all -> Read-only file system; touch /workspace/target/probe succeeds into the volume and nothing appears in host target/"
        status: pass
    human_judgment: false
  - id: D7
    description: "A fresh clone with no target/ passes check-in-container.sh all and stays clean"
    requirement: DEV-06
    verification:
      - kind: other
        ref: "Scratch clone of feature/phase-1 at 0c45ee2: no target/ before, exit 0 with `==> check.sh: all OK`, target/ created, git status --porcelain empty"
        status: pass
    human_judgment: false

duration: 9min
completed: 2026-10-22
status: complete
---

# Phase 1 Plan 03: Pinned devcontainer image and the read-only container gate Summary

**The image `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie` is pinned once, in `.devcontainer/Dockerfile`, and `devcontainer.json` builds from it. `scripts/check-in-container.sh` runs `scripts/check.sh` in that image from any checkout or linked worktree. The source and git directories are mounted read-only, and the pinned rustc 1.99.1 and tools are used. To make that work, the build volume is mounted over `/workspace/target` instead of the plan's `/ctarget`.**

## What these results do and do not establish

- **Read this first: the plan's container interface changed.** The plan's verification artifacts named `CARGO_TARGET_DIR=/ctarget`. That cannot work with a read-only checkout, because nextest writes its JUnit report under `/workspace/target/nextest` whatever `CARGO_TARGET_DIR` says. The target volume is now mounted at `/workspace/target`, and `CARGO_TARGET_DIR` is not set (Deviation 1).
- **Delete any `wsx-ci-target-*` volume that predates this change.** A target volume reused from the old `/ctarget` layout produces a false failure. Cargo treats the build as fresh, the test binaries still carry `CARGO_BIN_EXE_wsx=/ctarget/debug/wsx`, and both CLI tests fail with `spawn the wsx binary: ... No such file or directory`. I saw exactly this once during the prototype. Remove the volumes with `docker volume rm` by name (`docker volume ls --filter name=wsx-ci-`). The script's comment says the same.
- **Parity is proven for this host's rootless Docker only.** Rootless Docker maps the host uid to container root, so git accepts the read-only mounts without `safe.directory`. A rootful daemon maps uids differently. Phase 1's check.sh never calls git. Git-dependent checks in later phases may need `safe.directory` under rootful Docker, and that was not tested.
- **CI has not run this image yet.** 01-04 uses the same image, and its first run is owner-gated (01-07).
- **Each control ran once.** One run shows each guard can fire. It is not a reliability measurement.
- **The fresh-clone run is a normal clone, not a linked worktree.** It proves the host `mkdir -p target` path. Worktree binding was proven separately, on this phase worktree.
- The estimate was 52000 tokens. The realized diff is about 2143 (chars/4 over the 3 files), so the estimate was about 24 times too high. That matches the misses in 01-01 and 01-02.

## Performance

- **Duration:** about 9 min of execution (20:00:07Z to 20:09:08Z), split around a checkpoint for a decision. Each container `all` run takes about 20 s, most of it rustup downloading 1.99.1.
- **Started:** 2026-10-22T20:00:07Z
- **Completed:** 2026-10-22T20:09:08Z
- **Tasks:** 2 of 2
- **Files modified:** 3 created

## Accomplishments

- `.devcontainer/Dockerfile` is a comment header plus one line, `FROM mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`. The header explains:
  - why the pin lives in a Dockerfile;
  - that Dependabot's docker ecosystem bumps it;
  - that `scripts/assert-image-parity.sh` must fail CI until the workflows match.
- `.devcontainer/devcontainer.json` sets:
  - `"build": {"dockerfile": "Dockerfile"}`, with no image key and no Features;
  - the volumes `wsx-cargo-registry`, `wsx-cargo-git` and `wsx-target-${localWorkspaceFolderBasename}`;
  - devflow's chown and `safe.directory` in `postCreateCommand`, without its `rustup component add` and `cargo build` steps;
  - the extensions rust-analyzer and vscode-lldb, and `remoteUser: vscode`.
- `scripts/check-in-container.sh` (mode 100755, shellcheck and `bash -n` clean):
  - reads the image from the first FROM line, and exits 1 with `error: could not read the FROM image from .devcontainer/Dockerfile` when that is empty or missing;
  - checks `docker info` reachability and exits 1 with devflow's message if it fails;
  - uses per-checkout volumes named from sha256 of REPO_ROOT;
  - binds the gitdir and common dir read-only, with dedup;
  - passes `-t` only when stdout is a TTY;
  - installs cargo-nextest and cargo-deny through install-tools.sh inside the container, then `exec scripts/check.sh "$@"`;
  - has no CPU pinning and no Flatpak routing.

## Task Commits

1. **Task 1: pin the devcontainer image in a Dockerfile** - `220aa38` (feat), gated on `scripts/check.sh all`
2. **Task 2: run the gate in the pinned image from any worktree** - `0c45ee2` (feat), gated on `shellcheck && bash -n && scripts/check.sh all`

**Plan metadata:** recorded in the docs commit that adds this SUMMARY.

## Control evidence (quoted from the runs)

**Image (Task 1):**
- `docker image inspect` RepoDigests: `["mcr.microsoft.com/devcontainers/rust@sha256:269f7cd057eaccb52c17e9461f42ac5cc1bff564b0723be49ea91726f5753bb7"]`
- The image's default compiler, run outside the workspace: `rustc 1.99.0 (b940084d7 2026-09-28)`
- The jq check prints `true` for the committed file, and `false` (exit 1) for copies with an `image` key or a Feature added. A Dockerfile copy with a second FROM appended gives an `rg -c '^FROM'` count of 2.

**In-container gate from this linked worktree (Task 2 verify #1), exit 0:**
```
==> worktree detected: gitdir component /var/home/denniyahh/Github/wsx/.git/worktrees/phase-1 is outside the mount; binding it through
==> worktree detected: gitdir component /var/home/denniyahh/Github/wsx/.git is outside the mount; binding it through
==> image:  mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie
==> target: all
==> rustc 1.99.1 (58cf8f9c3 2026-10-13)
     Summary [   0.008s] 2 tests run: 2 passed, 0 skipped
==> check.sh: all OK
```
In the same image, the default is `rustc 1.99.0` and the gate reports `rustc 1.99.1`, so rust-toolchain.toml selected the compiler. The output has no `\r` and no `command not found`. `git status --porcelain` is identical before and after.

**deps (verify #2), exit 0:** `==> cargo-deny 0.20.2 OK`, `advisories ok, bans ok, licenses ok, sources ok`, `==> check.sh: deps OK`.

**Unreachable daemon (verify #3):** with `DOCKER_HOST=unix:///nonexistent/docker.sock` the script exits **1** with `error: Docker daemon is not reachable.`

**Volumes (verify #4):** `wsx-ci-target-4fa63246f2d5` and `wsx-ci-registry-4fa63246f2d5`. The suffix is the first 12 hex of sha256 of `/var/home/denniyahh/Github/wsx/.worktrees/phase-1`.

**Read-only source (verify #5):** `touch: cannot touch '/workspace/.ro-probe': Read-only file system`. No `.ro-probe` appears on the host.

**Read-only source with the nested target mount (added at the orchestrator's request).** This used the script's exact mount layout: gitdir and common dir `:ro`, `/workspace:ro`, and a scratch volume at `/workspace/target`.
```
touch: cannot touch '/workspace/Cargo.toml': Read-only file system
touch: cannot touch '/workspace/src/new.rs': Read-only file system
touch: cannot touch '/var/home/denniyahh/Github/wsx/.git/worktrees/phase-1/probe': Read-only file system
touch: cannot touch '/var/home/denniyahh/Github/wsx/.git/probe': Read-only file system
target rc=0
```
The positive half: writing into `/workspace/target` succeeded, and the file went into the volume, not the host. Host `target/probe`, `Cargo.toml`'s mtime and `src/` were all unchanged. `git status` worked inside the container through the read-only mounts.

**Fresh clone (added at the orchestrator's request).** A scratch `git clone --branch feature/phase-1` at `0c45ee2` started with no `target/` and an empty `git status --porcelain`. `scripts/check-in-container.sh all` exited **0** with `==> rustc 1.99.1 ...`, `2 tests run: 2 passed` and `==> check.sh: all OK`. Afterwards `target/` existed and `git status --porcelain` was still empty.

**Plan-as-written failure (why Deviation 1 exists).** With `-v vol:/ctarget -e CARGO_TARGET_DIR=/ctarget`, check.sh exited 110 after both tests passed:
```
     Summary [   0.006s] 2 tests run: 2 passed, 0 skipped
error: error reporting results: error operating on path /workspace/target/nextest/default/junit.xml
```

**Acceptance criteria (Task 2):**
- `git ls-files -s` gives `100755 ... scripts/check-in-container.sh`.
- `shellcheck` and `bash -n` exit 0.
- `rg -e taskset -e cpuset -e ci-cpus` finds nothing in the script, and the same search matches devflow's script.

## Files Created/Modified

- `.devcontainer/Dockerfile`: the single image pin.
- `.devcontainer/devcontainer.json`: the dev container definition, built from the Dockerfile.
- `scripts/check-in-container.sh`: runs the gate in the pinned image with read-only source mounts.

## Decisions Made

- The target volume is mounted at `/workspace/target`, and the host gets `mkdir -p target` (Deviation 1). The orchestrator chose Option A: the locked constraint is a read-only *source*, and target/ is git-ignored.
- The script also handles a missing Dockerfile. sed's error is shown and the run exits 1 with the plan's message, rather than with sed's exit code 2.
- devcontainer.json keeps only devflow's two Rust extensions and none of its VS Code settings, as the plan listed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking, escalated and decided] nextest cannot write its report into a read-only checkout**
- **Found during:** Task 2, the first `scripts/check-in-container.sh all` run.
- **Issue:** nextest's store directory defaults to `target/nextest` under the workspace root (nextest's `default-config.toml`), not under `CARGO_TARGET_DIR`. 01-01's `.config/nextest.toml` writes a JUnit report there on every run, so with `/workspace:ro` the run exits 110. RESEARCH Pattern 3's "nextest passed with /workspace:ro" was checked before that config existed. No environment variable moves the store.
- **Fix:** I stopped at a decision checkpoint, and the orchestrator chose Option A:
  - `-v "$TARGET_VOLUME":/workspace/target`;
  - `/ctarget` and `CARGO_TARGET_DIR` dropped;
  - host `mkdir -p "$REPO_ROOT/target"` before `docker run`, because Docker cannot create a mount point inside a read-only bind (`mkdirat ... read-only file system`, exit 125, without it).
  - This replaces the `CARGO_TARGET_DIR=/ctarget` interface in the plan's verification artifacts.
- **Files modified:** scripts/check-in-container.sh
- **Verification:**
  - all five Task 2 verify commands, rerun with fresh volumes;
  - the nested-mount write control and the fresh-clone run above.
- **Committed in:** 0c45ee2
- **Side finding:** reusing a target volume across the layout change gives a false failure (stale `CARGO_BIN_EXE_wsx` path), so old `wsx-ci-target-*` volumes must be deleted. That is recorded in the script's comment and at the top of this SUMMARY.

---

**Total deviations:** 1 (blocking; it changed a mount interface, so it was escalated and decided by the orchestrator).
**Impact on plan:** the source stays read-only, and the added control shows that. The only host-side write is creating the git-ignored `target/` directory.

## Issues Encountered

- My first prototype of the fix reused the existing target volume and failed, because of the stale binary path described above. All volumes were deleted and the verified runs used fresh ones.

## Docker objects

- **Kept, as the plan requires:** `wsx-ci-target-4fa63246f2d5` and `wsx-ci-registry-4fa63246f2d5`, this phase worktree's per-checkout volumes. 01-07 removes them at the end of the phase.
- **Removed by exact name:**
  - `wsx-ci-target-4fa63246f2d5-proto` (prototype);
  - `wsx-negctl-0103` (nested-mount control);
  - `wsx-ci-target-0a7f0244dd6f` and `wsx-ci-registry-0a7f0244dd6f` (fresh clone);
  - the first-run pair, which held the contaminated pre-change target cache.
- No containers remain, since every run used `--rm`.

## User Setup Required

None.

## Next Phase Readiness

- 01-04 can read the image from `.devcontainer/Dockerfile`'s FROM line for its parity check and its job `container.image`. In CI the checkout is writable, so check.sh runs directly. That needs `scripts/install-tools.sh install cargo-nextest` (and `cargo-deny` for the advisory job) first.
- 01-06 docs should describe `scripts/check-in-container.sh`, its volumes, and the instruction to delete the volumes after a mount-layout change.

## Self-Check: PASSED

- `.devcontainer/Dockerfile`, `.devcontainer/devcontainer.json` and `scripts/check-in-container.sh` exist and are tracked (the script as mode 100755).
- Commits 220aa38 and 0c45ee2 are ancestors of HEAD. `git rev-list --count 487a32e..HEAD` = 2 before this docs commit.
- All Task 1 and Task 2 verify commands and acceptance criteria passed on the committed tree, as quoted above.

---
*Phase: 01-development-foundation*
*Completed: 2026-10-22*
