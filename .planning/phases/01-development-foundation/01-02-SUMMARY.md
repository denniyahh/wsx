---
phase: 01-development-foundation
plan: 02
subsystem: infra
tags: [supply-chain, cargo-deny, cargo-nextest, gitleaks, sha256, bash, check.sh, deny.toml]

requires:
  - phase: 01-01
    provides: "scripts/check.sh (fmt, clippy, test, all; require_tool), Cargo.toml with license = \"MIT\" and a [dependencies] header, committed Cargo.lock"
provides:
  - "scripts/install-tools.sh: single pin site for cargo-nextest 0.9.148, cargo-deny 0.20.2, gitleaks 8.30.1; `install TOOL...` (sha256 checked before extraction, into WSX_TOOLS_DIR / CARGO_HOME/bin / ~/.cargo/bin) and `verify TOOL...`; exit 2 usage, exit 1 failure"
  - "deny.toml: advisories (unmaintained all, yanked deny), licenses [MIT], bans (wildcards deny), sources (crates.io only)"
  - "scripts/check.sh deps: verifies the pinned cargo-deny (searching CARGO_HOME/bin before PATH, as cargo does), then runs `cargo deny --locked check`; not part of `all`"
affects: [01-03 container runner installs tools through install-tools.sh, 01-04 CI Test and advisory deny jobs, 01-05 gitleaks pre-commit hook, later phases extending the deny.toml license list]

actuals:
  tokens: 2678
  tasks: 2
  commits: 3
plan_head_before: 352f336a4bfaea6f184c497dd4f8908062977d0d
plan_head_after: 68bf289db25f871f00b2ee69cc5b807e070bf25a

tech-stack:
  added: [cargo-deny 0.20.2 (release tarball, sha256-pinned), cargo-nextest 0.9.148 and gitleaks 8.30.1 (same installer)]
  patterns:
    - "Tool versions and sha256s live only in scripts/install-tools.sh; every other file refers to the script, never to a version"
    - "Checksum is checked before extraction; a mismatch exits 1 with nothing installed and the work dir removed by an EXIT trap"
    - "curl runs with --disable first and no header or agent options, so neither the script nor a .curlrc adds anything to the request"
    - "A gate that verifies a tool must verify the binary that will actually run (cargo resolves subcommands in CARGO_HOME/bin before PATH)"

key-files:
  created:
    - scripts/install-tools.sh
    - deny.toml
  modified:
    - scripts/check.sh

key-decisions:
  - "check.sh deps runs `install-tools.sh verify cargo-deny` with CARGO_HOME/bin prepended to PATH, because `cargo deny` runs CARGO_HOME/bin/cargo-deny ahead of PATH; without this an unverified shadow binary scanned and the gate printed deps OK"
  - "install-tools.sh passes curl --disable so a .curlrc on a contributor machine, in the container or in CI cannot add headers, a user agent or a proxy (T-01-06)"
  - "Download and extraction failures exit 1 with an error line; tar's own fatal code (2) would otherwise read as a usage error"
  - "deny.toml's header names install-tools.sh rather than the cargo-deny version, so the version stays written in exactly one tracked file"

patterns-established:
  - "Discriminating controls for every hardening fix: the fixed script refuses, and a scratch copy with the fix reverted is shown to pass the bad case"

requirements-completed: [SUP-01, SUP-04, SUP-05, DEV-01]

coverage:
  - id: D1
    description: "install-tools.sh install downloads the three pinned tarballs, checks sha256 before extracting, installs one binary each into WSX_TOOLS_DIR, leaves ~/.cargo/bin and TMPDIR untouched"
    requirement: SUP-04
    verification:
      - kind: other
        ref: "Task 1 verify #1: WSX_TOOLS_DIR=$S/bin TMPDIR=$S/tmp scripts/install-tools.sh install cargo-nextest cargo-deny gitleaks && env PATH=$S/bin:/usr/bin:/bin ... verify (exit 0; ~/.cargo/bin listing unchanged; TMPDIR empty)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A tarball whose sha256 differs from the pin is refused: exit 1, `checksum mismatch`, nothing installed, no temp dir left"
    requirement: SUP-04
    verification:
      - kind: other
        ref: "Task 1 verify #2: GITLEAKS_SHA256 zeroed in a scratch copy -> exit 1, `error: checksum mismatch for gitleaks 8.30.1: refusing to install`"
        status: pass
    human_judgment: false
  - id: D3
    description: "verify names tool, expected and found versions on mismatch and exits 1; missing tool exits 1; unknown tool or missing tool argument exits 2"
    requirement: SUP-04
    verification:
      - kind: other
        ref: "Task 1 verify #3 (fake cargo-deny 0.19.0 -> exit 1 naming 0.20.2 and 0.19.0; host copy passes) and #4 (verify no-such-tool -> 2, install with no tool -> 2)"
        status: pass
    human_judgment: false
  - id: D4
    description: "scripts/check.sh deps passes on the clean tree and fails on a scratch copy depending on smallvec =1.6.0 with RUSTSEC-2021-0003 and `advisories FAILED` (C5)"
    requirement: SUP-05
    verification:
      - kind: other
        ref: "Task 2 verify #1 (clean: `advisories ok, bans ok, licenses ok, sources ok`, `==> check.sh: deps OK`) and #2 (C5: exit 1, RUSTSEC-2021-0003, `advisories FAILED, bans ok, licenses ok, sources ok`)"
        status: pass
    human_judgment: false
  - id: D5
    description: "deps fails closed on a missing scanner (C1) and on a wrong-version scanner, checking the version before any scan line"
    requirement: SUP-01
    verification:
      - kind: other
        ref: "Task 2 verify #3 (PATH=$HOME/.cargo/bin:/usr/bin:/bin -> exit 1 naming cargo-deny) and #4 (fake cargo-deny 0.19.0 first on PATH -> exit 1, no `==> cargo deny --locked check` line)"
        status: pass
      - kind: other
        ref: "Added shadow control: fake cargo-deny in scratch CARGO_HOME/bin -> exit 1 before scan; same scratch tree with the fix reverted -> exit 0 and `FAKE SCAN RAN: deny --locked check`"
        status: pass
    human_judgment: false
  - id: D6
    description: "`scripts/check.sh all` still runs fmt, clippy, test only and never runs cargo deny"
    requirement: DEV-01
    verification:
      - kind: other
        ref: "Task 2 verify #5: check.sh all exit 0, output has no `cargo deny`"
        status: pass
    human_judgment: false
  - id: D7
    description: "Versions pinned in exactly one tracked file outside .planning; no header or user-agent options in curl; .curlrc ignored"
    requirement: SUP-04
    verification:
      - kind: other
        ref: "rg -l --hidden -F -g '!.planning/**' -g '!.git' for 0.9.148, 0.20.2, 8.30.1 -> only scripts/install-tools.sh; header rg exits 1 on the script and 0 on `curl --header X`"
        status: pass
      - kind: other
        ref: "curlrc control: CURL_HOME/.curlrc with `max-filesize = 1` -> current script installs gitleaks (exit 0); copy without --disable fails with `curl: (63) Maximum file size exceeded` (exit 1)"
        status: pass
    human_judgment: false

duration: 6min
completed: 2026-10-22
status: complete
---

# Phase 1 Plan 02: Pinned, checksum-verified tool installs and the cargo-deny gate Summary

**`scripts/install-tools.sh` is now the one place cargo-nextest 0.9.148, cargo-deny 0.20.2 and gitleaks 8.30.1 are pinned, and it refuses a tarball whose sha256 differs. `scripts/check.sh deps` verifies the pinned cargo-deny, then runs `cargo deny --locked check`. It is shown to fail on a known-vulnerable crate, on a missing scanner, on a wrong-version scanner, and on a wrong-version scanner hidden in CARGO_HOME/bin.**

## Performance

- **Duration:** about 6 min of execution (19:43:55Z to 19:49:25Z; excludes reading the plan and references)
- **Started:** 2026-10-22T19:43:55Z
- **Completed:** 2026-10-22T19:49:25Z
- **Tasks:** 2 of 2, plus one hardening fix commit
- **Files modified:** 3 (2 created, 1 modified)

## What these results do and do not establish

- **Install was proven on this host's x86_64 Linux only.** The pinned container (01-03) and CI (01-04) run `install` and `verify` for the first time in those plans. The non-x86_64 refusal and the missing curl, sha256sum or tar paths were not exercised, because this host has all of them and is x86_64.
- **C5 proves the advisory check bites on one known advisory** (RUSTSEC-2021-0003). The licenses, bans and sources checks are proven only by their clean pass here. No control planted a disallowed license, a wildcard or a git source.
- **Each control ran once.** That shows each guard can fire, not that it is reliable across environments.
- **"Nothing identifying the operator is sent" was not measured on the wire.** It rests on three things: the script passes no header, agent or proxy options; curl skips any .curlrc (control below); and curl's default user agent is `curl/<version>`. No packet capture was taken.
- The estimate was 44000 tokens; the realized diff is about 2678 (chars/4), so the estimate was about 16 times too high. That matches 01-01's miss.

## Accomplishments

- `scripts/install-tools.sh` (mode 100755, shellcheck and `bash -n` clean):
  - The pin variables `CARGO_NEXTEST_VERSION`/`_SHA256`, `CARGO_DENY_VERSION`/`_SHA256` and `GITLEAKS_VERSION`/`_SHA256` each start their own line. Each URL is built from its version.
  - `install`:
    - refuses anything but x86_64 Linux;
    - requires curl, sha256sum and tar;
    - downloads into one `mktemp -d` work dir, which an EXIT trap removes;
    - uses `curl --disable --fail --silent --show-error --location --proto '=https' --tlsv1.2`;
    - runs `sha256sum --check --status` before extracting;
    - extracts only the pinned archive member and installs it with `install -m 0755`;
    - re-checks the installed binary's version by its absolute path.
  - `verify`: the tool must be on PATH and its first version line must match the pin. Otherwise it exits 1 and names the tool, its path, the version found and the version expected.
  - Usage errors (missing mode, unknown mode, no tool, unknown tool) exit 2. Every tool name is checked before any work starts.
- `deny.toml`: the validated policy from RESEARCH, unchanged except for comments.
- `scripts/check.sh deps`:
  - takes no arguments (`deps extra` exits 2);
  - requires `cargo` and `cargo-deny`;
  - runs `install-tools.sh verify cargo-deny` against the binary cargo will run, then `cargo deny --locked check`;
  - is not part of `all`.

## Task Commits

1. **Task 1: pin and checksum-verify tool installs** - `c866a68` (feat)
2. **Task 2: add the cargo-deny advisory and license gate** - `8eea48f` (feat)
3. **Hardening fix: ignore curlrc and map tool failures to exit 1** - `68bf289` (fix)

**Plan metadata:** recorded in the docs commit that adds this SUMMARY.

## Control evidence (quoted from the runs)

**Install (SUP-04):** all three tools were installed into scratch and passed `verify` under `PATH=$S/bin:/usr/bin:/bin`. None of the three is in /usr/bin or /bin, so `verify` checked the scratch copies. The `~/.cargo/bin` listing was unchanged and TMPDIR was empty afterwards.
```
==> installed cargo-deny 0.20.2 to .../exec-01-02/tools.o3jnCa/bin
==> cargo-nextest 0.9.148 OK
==> cargo-deny 0.20.2 OK
==> gitleaks 8.30.1 OK
```

**Tamper (T-01-05):** a scratch copy with `GITLEAKS_SHA256` zeroed exits **1**, installs no `gitleaks` binary, and leaves no temp dir.
```
error: checksum mismatch for gitleaks 8.30.1: refusing to install
```

**Wrong version (verify):** a fake `cargo-deny` that prints `cargo-deny 0.19.0`, first on PATH, makes verify exit **1**:
```
error: cargo-deny at .../fake.KWf2Tr/cargo-deny reports 'cargo-deny 0.19.0', expected 0.20.2; run scripts/install-tools.sh install cargo-deny
```
Counterpart: the host copy gives `==> cargo-deny 0.20.2 OK` (exit 0). Usage: `verify no-such-tool` exits 2, `install` with no tool exits 2, and `bogus cargo-deny` exits 2. With cargo-deny off PATH, verify exits 1 with `error: cargo-deny is not on PATH; run scripts/install-tools.sh install cargo-deny`.

**C5, vulnerable crate (SUP-05)** in a scratch copy with `smallvec = "=1.6.0"` and CARGO_HOME isolated in scratch, exit **1**:
```
==> cargo-deny 0.20.2 OK
==> cargo deny --locked check
error[vulnerability]: Buffer overflow in SmallVec::insert_many
  ├ ID: RUSTSEC-2021-0003
advisories FAILED, bans ok, licenses ok, sources ok
```
Counterpart, the clean tree, exit **0**:
```
==> cargo deny --locked check
advisories ok, bans ok, licenses ok, sources ok
==> check.sh: deps OK
```
`git diff --exit-code HEAD -- Cargo.toml Cargo.lock` was clean after the control. The vulnerable crate never touched the tracked crate.

**C1, missing scanner** (`env PATH="$HOME/.cargo/bin:/usr/bin:/bin"`), exit **1**:
```
error: 'cargo-deny' is required for 'check.sh deps' but is not on PATH; run scripts/install-tools.sh install cargo-deny
```

**Ordering edge (T-01-07):** a fake `cargo-deny 0.19.0` first on PATH. `deps` exits **1** after `==> scripts/install-tools.sh verify cargo-deny` and the version error. No `==> cargo deny --locked check` line is printed.

**Shadow control (added, see Deviation 1):** with a fake `cargo-deny` in a scratch `CARGO_HOME/bin` and the real PATH, the committed `check.sh deps` exits **1** with `... ch/bin/cargo-deny reports 'cargo-deny 0.19.0', expected 0.20.2`. The same tree with the fix reverted exits **0**:
```
==> cargo-deny 0.20.2 OK
==> cargo deny --locked check
FAKE SCAN RAN: deny --locked check
==> check.sh: deps OK
```

**`all` stays out of deps:** `scripts/check.sh all` exits 0 with the lines `==> rustc`, `cargo fmt --check`, `cargo clippy ...`, `cargo nextest run ...` and `check.sh: all OK`. Its output has no `cargo deny`.

**curlrc control (added, see Deviation 2):** `CURL_HOME/.curlrc` contains `max-filesize = 1`. The committed script ignores it and installs gitleaks (exit 0). A copy without `--disable` reads it and fails:
```
curl: (63) Maximum file size exceeded
error: download failed for gitleaks 8.30.1
```
That copy exits 1. Before the exit-code fix it exited 63.

**Bad archive member:** a copy with the gitleaks member renamed exits **1** (not tar's 2) with `error: could not extract nope from the gitleaks 8.30.1 archive`. Nothing was installed and TMPDIR was empty.

**Single pin site:** `rg -l --hidden -F -g '!.planning/**' -g '!.git'` for each of `0.9.148`, `0.20.2` and `8.30.1` returns only `scripts/install-tools.sh`. The header check `rg -e '--user-agent' -e '--header'` does not match the script, and it does match the sample line `curl --header X`.

## Files Created/Modified

- `scripts/install-tools.sh`: the pin site and installer (install and verify modes).
- `deny.toml`: cargo-deny policy.
- `scripts/check.sh`: `deps` target, `run_deps`, usage text.

## Decisions Made

- `verify` inside `deps` searches `CARGO_HOME/bin` before PATH, the same order cargo uses for subcommands (Deviation 1).
- `install-tools.sh` reads no `.curlrc` (Deviation 2).
- `deny.toml`'s header points to `install-tools.sh` instead of naming the cargo-deny version. My first draft named it, and that would have broken the single-pin-site check.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] deps verified one cargo-deny and scanned with another**
- **Found during:** Task 2
- **Issue:** `install-tools.sh verify` checks the `cargo-deny` that PATH finds. `cargo deny` runs `$CARGO_HOME/bin/cargo-deny` ahead of PATH when that file exists. A probe confirmed this: a fake in a scratch `CARGO_HOME/bin` won over the brew copy on PATH. A stale or wrong-version binary there would be scanned with, unverified. The reverted-fix control above shows the gate then printing `deps OK` over a fake scan. This defeats T-01-07.
- **Fix:** `PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH" scripts/install-tools.sh verify cargo-deny` in `run_deps`. The plan's literal command `cargo deny --locked check` is kept.
- **Files modified:** scripts/check.sh
- **Verification:** shadow control (fails with the fix, passes without it), plus all of Task 2's verify commands.
- **Committed in:** 8eea48f

**2. [Rule 2 - Missing critical] a .curlrc could add headers or a user agent to the download**
- **Found during:** writing the SUMMARY's threat review (T-01-06)
- **Issue:** curl reads `$CURL_HOME/.curlrc`, `$XDG_CONFIG_HOME/.curlrc` or `~/.curlrc` by default. The plan's acceptance check only covers options written in the script. No curlrc exists on this host, but contributor machines, the container and CI are not controlled.
- **Fix:** `--disable` as curl's first option. It is not a request option; it only stops curl reading config files.
- **Files modified:** scripts/install-tools.sh
- **Verification:** curlrc control (ignored with the fix, honoured without it). The full Task 1 verify block was re-run afterwards, and all four commands passed.
- **Committed in:** 68bf289

**3. [Rule 1 - Bug] curl and tar exit codes leaked through**
- **Found during:** the curlrc control. The script exited 63, but the plan says failures exit 1.
- **Issue:** under `set -e` a failed curl or tar exits with its own code. tar's fatal code 2 collides with the usage-error code.
- **Fix:** each call is wrapped in `if ! ...; then echo error...; exit 1; fi`. Both are still called directly, not in a process substitution or behind `|| true`.
- **Files modified:** scripts/install-tools.sh
- **Verification:** the curlrc copy exits 1, and the bad-member copy exits 1.
- **Committed in:** 68bf289

### Verification procedure adjustment (not a code change)

- I could not run the ordering control in the plan's `mktemp`+`trap 'rm -rf "$S"'` form inside a `bash -c` wrapper. Claude Code's built-in safety check refused that command, because it could not resolve the removal target. I ran it with the same assertions in a fixed scratch directory, `exec-01-02/order.fixed`, and removed that directory afterwards with its literal path. The shadow, curlrc and Task 1 re-run controls used fixed directories the same way. The install, tamper, fake-version, usage and C5 controls had already run in their written `mktemp`+`trap` form; I swapped quote styles where the commands were nested in `bash -c`. All assertions are the plan's own.

---

**Total deviations:** 3 auto-fixed (2 missing critical, 1 bug), plus 1 procedural adjustment.
**Impact on plan:** each change tightens a mitigation the plan already assigned (T-01-05, T-01-06, T-01-07) or its stated exit-code contract. No scope was added.

## Issues Encountered

- `run_deps` checks `require_tool cargo-deny` against PATH only. If cargo-deny exists only in `CARGO_HOME/bin` and not on PATH, `deps` refuses even though cargo could run it. This fails closed, so I left it.
- `check_version`'s path for a binary that exits non-zero on `--version` was not exercised.

## User Setup Required

None. The host's brew copies already match the pins (`verify` passes for all three).

## Next Phase Readiness

- 01-03 can run `scripts/install-tools.sh install cargo-nextest cargo-deny gitleaks` and `verify` in the pinned image. It should set `WSX_TOOLS_DIR` or `CARGO_HOME` as needed, because the default destination is `$CARGO_HOME/bin`.
- 01-04 can call `scripts/install-tools.sh install cargo-deny` and then `scripts/check.sh deps` in the advisory job.
- Requirements: SUP-01, SUP-04, SUP-05 and DEV-01 are all shared with sibling plans. `requirements.ready-ids` reported all four blocked, so none is marked complete yet.

## Self-Check: PASSED

- `scripts/install-tools.sh`, `deny.toml` and `scripts/check.sh` exist. `scripts/install-tools.sh` is mode 100755 in the index.
- Commits c866a68, 8eea48f and 68bf289 are ancestors of HEAD. `git rev-list --count 352f336..HEAD` = 3 before this docs commit.
- The final tree passes both `scripts/check.sh all` and `scripts/check.sh deps` (exit 0). Commit 68bf289 was gated on both.
- Task 1 acceptance criteria (mode, shellcheck, `bash -n`, header check with its positive control, single pin site) and Task 2 acceptance criteria (deny.toml keys, Cargo.toml and Cargo.lock unchanged, no `c5.*` or `order.*` scratch left) were all re-run after the last code commit and passed.

---
*Phase: 01-development-foundation*
*Completed: 2026-10-22*
