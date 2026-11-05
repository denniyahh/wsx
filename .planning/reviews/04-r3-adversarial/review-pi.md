## CONFIRMED

### 1. The gate every plan is verified by cannot run on this tree (medium — blocks execution, not design)

**Where:** `.planning/phases/04-repo-model/04-01-PLAN.md:197` and the identical `<verify>` in all 11 plans:
`mkdir -p target/gsd && scripts/check.sh all > target/gsd/04-01-all.log 2>&1; rc=$?; tail -n 3 target/gsd/04-01-all.log; test "$rc" -eq 0`
plus `04-01-PLAN.md:245`: “`verify-cargo` exited 1 without the prefix and 0 with it”, and `04-01-PLAN.md:244`'s control prefix `PATH="$R/.tools/bin:$PATH"`.

**Evidence checked:** `scripts/check.sh:30` prepends `.tools/bin` only `if [ -d .tools/bin ]`; `scripts/check.sh:197-198` runs `scripts/install-tools.sh verify-cargo cargo-nextest` unconditionally; `scripts/install-tools.sh:39` pins `CARGO_NEXTEST_VERSION=0.9.148`; host `cargo-nextest --version` = `0.9.150`. Neither `.tools/bin` nor `target/gsd-tools` exists in this tree. `.planning/STATE.md:187` records a *third* location: “phase-3 gates run with PATH=$PWD/target/gsd-tools:$PATH (pinned tools installed there …)”, which `scripts/check.sh` does not consult automatically.

**Reproduce:**
```
scripts/install-tools.sh verify-cargo cargo-nextest
# error: cargo-nextest run as 'cargo nextest' reports 'cargo-nextest 0.9.150', expected 0.9.148; ...
scripts/check.sh test -E 'test(=x)'
# ==> rustc 1.99.1 ...
# ==> scripts/install-tools.sh verify-cargo cargo-nextest
# error: ... expected 0.9.148 ...   (exit 1, before any compile)
```

**What breaks:** every `<verify>` gate exits 1 before a test runs, so `target/nextest/default/junit.xml` is never written and no `RED_EVIDENCE_OK` record in any plan can be produced. The scratch controls (`04-01:244`, `04-03`'s encoder and deny controls, `04-10:103`) abort at `verify-cargo ... || exit 1`, and their own `<fails_when>` text (`04-01:245`) classifies that exact abort as “the nextest this control would run is not the pin its statuses were measured with” — i.e. the plans call their own unstated precondition “evidence invalid”. The plans never state the prerequisite; `CONTRIBUTING.md:17-40` does document `WSX_TOOLS_DIR=.tools/bin scripts/install-tools.sh install …`, so this is a missing precondition plus a stale repo record, not a design error. **Not** established by this: that the plans would fail on a correctly provisioned host.

### 2. A stated threat mitigation rests on a source grep that the plan itself says can pass a violating implementation (medium)

**Where:** `04-01-PLAN.md:277` (T-04-33): “the race half rests on the code shape alone, which Task 1's source check makes falsifiable (one `find_ref` call site in `ref_commit`, no revspec call in the file's code)”; `04-01-PLAN.md:195` is that check (`awk`/`rg` over `src/git/refs.rs`); `04-01-PLAN.md:196` concedes: “It bounds call sites, not how often one runs, and a read moved into a helper `ref_commit` calls would pass it”. `04-07-PLAN.md:227` repeats the same concession for T-04-28's second half.

**Evidence checked:** `CONTRIBUTING.md:178` — a test “is rejected outright … 4. Greps implementation text as a substitute for a runtime contract that could have been asserted directly.” The check is purely textual: `rg -o 'find_ref\(' <<<"$body" | wc -l` and `rg -q 'peel_commit\(&id\)'`. I read `src/git/refs.rs:1-28` (`peel_commit`/`peeled_commit`) and `src/git/testing.rs:1052-1166` for the suite-shape context. I also verified the check's own baseline behaviour (`rg` exits 0 on `pub fn peel_commit`/`fn peeled_commit`, 1 on `SYMREF_MAX_DEPTH`, matching `04-01:acceptance_criteria`), so the check runs; the issue is what it establishes.

**What breaks:** a variant that moves the read into `fn read_rung(...)` called once from `ref_commit` satisfies the text check while reintroducing the exact “second lookup of the name” hazard T-04-33 is marked `mitigate` for. The plans are honest about it in three places, but the mitigation status (“T-04-33 … mitigate”) is not supported by the evidence attached to it, and CONTRIBUTING's bar says such a test cannot be the acceptance evidence at all. Either the threat row drops to `accept`/`transfer`, or the seam is made testable (a read counter around the ref store) so the property is asserted at a boundary.

### 3. `04-10`'s coverage `<verify>` cannot fail on the condition its own `<fails_when>` names (low)

**Where:** `04-10-PLAN.md:161-162`:
```
… scripts/check.sh coverage > target/gsd/04-10-coverage.log 2>&1; rc=$?; rm -rf target/llvm-cov-target; rg '^Filename|^TOTAL|^(model/(manifest|mod)|git/(config|refs|quote|mod))\.rs' target/gsd/04-10-coverage.log; (exit "$rc")
```
`<fails_when>`: “… or no `TOTAL` row in the output”.

**Evidence checked:** the block's exit status is the final `(exit "$rc")`, i.e. coverage's status alone; `rg`'s status (1 on no match) is discarded and its stdout is never asserted. `scripts/check.sh` cannot print `TOTAL` itself, so a log with no `TOTAL` row (report format change, empty file, `cargo-llvm-cov` printing to stderr only) exits 0.

**Reproduce:** run the block with `scripts/check.sh coverage` replaced by `true > log` — the command exits 0 and prints nothing. **What breaks:** the SUMMARY's recorded line coverage and the “no threshold” claim can be derived from a run that produced no report, and the `<fails_when>` clause reads as if it were enforced.

### 4. The “final line is `==> check.sh: all OK`” failure clause is unreachable in 21 places (low)

**Where:** the gate `<fails_when>` in every plan, e.g. `04-01-PLAN.md:197-198`, `04-10-PLAN.md:148-149`: `… tail -n 3 …log; test "$rc" -eq 0` with `<fails_when>non-zero exit, or the final line is not `==> check.sh: all OK``.

**Evidence checked:** the only assertion in the command is `test "$rc" -eq 0`; `tail` is printed, not tested. `scripts/check.sh:260` prints `==> check.sh: $TARGET OK` only on the success path, so when `rc` is 0 the line is present by construction and when it is not, `rc` already failed the block. `rg -c` shows 21 occurrences of that clause across the 11 plans.

**What breaks:** nothing at run time; it matters because the clause is the only stated guard against a truncated/garbled log and it can never fire, so `acceptance_criteria`' “the SUMMARY quotes” steps rest on a criterion that is not falsifiable.

### 5. `04-04` accepts a quadratic parser as “low” on a justification its own trust table contradicts (confirmed contradiction; impact in SUSPECTED below)

**Where:** `04-04-PLAN.md:197` — “T-04-10 | Denial of Service | a hostile manifest size | low | accept | `find_blob` reads it whole and the overlap check is quadratic in entries; **the blob is on the user's own branch**, accepted for ASVS level 1”; against `04-04-PLAN.md:189` — “`.wsx.conf` bytes -> the path list | **the blob is on the user's own branch but can arrive through a merge**; a malformed or ambiguous line must not change what is protected” (and `04-01-PLAN.md:266`: “the branch can arrive through a merge”).

**Evidence checked:** `04-04-PLAN.md:163` (GREEN): “compare it with every earlier entry of the other kind using `covers` in both directions; … The check is quadratic in entries”; no size, line-count or entry-count bound exists anywhere in the plan (`rg -n 'MAX_|limit|cap|too many|size' 04-04-PLAN.md` → only T-04-10). The consumers that run this parse on every invocation are named by `REQUIREMENTS.md:73` (“Guard, cut, check and status use it from any checkout”) and `REQUIREMENTS.md:130` (GUARD-03, the pre-push guard).

**What breaks:** the disposition “low / accept” is justified by a premise the same document rejects one table earlier. That is a decision that cannot be defended in review as written; the risk either gets a bound or its justification must change.

## SUSPECTED

### 6. Quadratic parse on the push path with no bound (speculation: not measured)

`04-04-PLAN.md:163` makes every manifest line compare against every earlier opposite-kind entry, and `04-04-PLAN.md:197` accepts it. If a merge brings a large `.wsx.conf` onto the workspace branch (the path `04-04-PLAN.md:189` names), then per push in a configured repo the guard pays ~n²/2 `covers` calls over up to n path bytes: at 100 000 lines that is ~5×10⁹ calls, each scanning bytes — minutes to hours inside a pre-push hook, once per push, with no timeout and no cap in any plan. I did not measure this (no phase-4 code exists); the arithmetic and the “no cap anywhere” grep are real, the wall-clock estimate is not. A `MAX_ENTRIES`/`MAX_MANIFEST_BYTES` refusal with its own named test would close it and is cheaper than re-litigating the threat row.

### 7. Commit-SHA baselines in acceptance criteria have no failure mode for an unreachable commit (not verifiable here)

`04-02-PLAN.md` acceptance (`git diff --name-only 6946ae9 -- tests/harness`), `04-03-PLAN.md` (`git diff 6946ae9 -- Cargo.lock`, `for c in … Cargo.lock`), `04-10-PLAN.md:162` (“baseline at `8b2fb4f`”) and `04-10-PLAN.md:acceptance` (`git log --no-merges … 6946ae9.. -- src/model/mod.rs`) all hard-code commits. Round 2 rejected “`6946ae9` unreachable” by checking ancestry on the reviewer's machine; that answers ancestry, not what the executor does when the commit is absent (a PR-branch cut, a shallow clone, a replay). A missing commit makes `git diff`/`git log` exit 128, which the criteria read as a failed check with no fallback stated. I could not verify reachability at all here — this snapshot has no `.git` (`git status` → `fatal: not a git repository`), so every SHA-referencing criterion was unverifiable in this review.

### 8. `04-06`'s remote check is deliberately less strict than the `wsx.*` read it sits beside

`04-06-PLAN.md` GREEN: “`level` is not filtered here, because git's own in-repo test counts an included section”, while `04-05-PLAN.md` (A1) filters `meta.level == 0` for `wsx.*` because “a tracked include would be read live from whatever branch is checked out”. I verified the git facts both ways (an `include.path` remote makes `git remote add` exit 3; `git config --local --get` on the included wsx key exits 1; a *global* `[remote "x"]` is listed by `git remote` but `git remote add x` still exits 0), so the asymmetry tracks git. I flag it only because the two adjacent reads of the same config snapshot now use different trust rules with no single place stating why that is safe; if the include is what makes a `wsx.baseRemote` value valid, then the value and its target are validated under different policies.

### 9. Not a finding, but the reason this report is thin

I tried to break ~25 factual claims by measurement rather than by reading, and every one held on this host (git 2.56.0, gix 0.90.0, gix-config 0.62.2, gix-ref 0.69.1, gix-quote 0.10.1): `git for-each-ref 'refs/heads/workspace/*'` matches one level only (`workspace/nested/deep` excluded); `:(literal).planning` matches `.planning/x` and `:(literal).plan` does not; `git clone` of a local path does create `refs/remotes/origin/HEAD` → `origin/main` plus `develop`/`next`; a keyless local `[remote]` section is invisible to `git remote`/`git remote add` while a `prune = true` section is not; an implicit (`no =`) config key reads as one empty value from `git config --local --get`, not `true`; `gix::quote::ansi_c` quoting ranges match git's `cq_lookup` table byte for byte; `strings_filter` returns implicit values (via `Event::Value`) while `string_filter` skips them, exactly as `04-05` claims; `gix-config` `Metadata.level` is `u8`; `git update-ref` accepts a blob under `refs/remotes/…` and `^{commit}` then says “expected commit type”; `git symbolic-ref` builds the cycle and `update-ref --no-deref -d` removes only the link; `git clone --bare` leaves `remote.origin.url` so `git remote remove origin` succeeds; the source-check `sed`/`awk` fragments do strip both guard shapes; the `.workspace-divergence` line numbers (11 and 14) and the harness `wsx.*` line numbers (265, 266, 669, 670, 828) are exact; the file/line citations I spot-checked in `src/git/{mod,refs,testing,repo}.rs` all land on the symbol claimed. Given two prior review rounds that probed the same surface, the absence of a factual break is the expected result — the remaining defects are in the executability layer (1–4) and in how two risks are justified (2, 5).

## COVERAGE

**Read in full:** `.planning/phases/04-repo-model/04-CONTEXT.md`; `04-01-PLAN.md`; `04-02-PLAN.md`; `04-03-PLAN.md`; `04-04-PLAN.md`; `04-05-PLAN.md`; `04-06-PLAN.md`; `04-07-PLAN.md`; `04-VALIDATION.md`; `04-REVIEWS.md` lines 1-125 (consensus, findings, rejections) and the grep-extracted disposition ledgers of 04-01/03/04/07/10/11; `.planning/ROADMAP.md` § Phase 03.1 and § Phase 4 and the Progress-adjacent phase list; the CONF/GUARD/DIV/WTREE lines of `.planning/REQUIREMENTS.md`; `clippy.toml`; `deny.toml`; `Cargo.toml`; `CLAUDE.md`; `CONTRIBUTING.md` (both halves); `scripts/check.sh`; `.planning/STATE.md` Blockers section; `.config/nextest.toml`; `.workspace-divergence`.

**Read substantially but not line-by-line:** `04-08-PLAN.md` (must_haves, both tasks' behavior/action/verify/acceptance); `04-09-PLAN.md` (must_haves, Tasks 1-3 behavior/verify/acceptance); `04-10-PLAN.md` (Tasks 1-2, all `<automated>`/`<fails_when>` blocks, threat model, verification); `04-11-PLAN.md` (must_haves, GREEN/verify/acceptance, threat model, both ledgers) — Task 1's full `behavior` block was read only through grep excerpts.

**Not read:** `04-RESEARCH.md` (only the ranges plans quote, never end to end), `04-PATTERNS.md` (only greps), `04-DISCUSSION-LOG.md`, `bin/wsx` (frozen reference), `scripts/check-in-container.sh`, `scripts/install-tools.sh` beyond the version pins and `verify-cargo`, `tests/harness/*` beyond the fixture's `wsx.*` lines, `src/git/{reads,spawn}.rs`, `src/main.rs` beyond the `mod` line.

**Verification limits:** no git metadata in this snapshot, so no SHAs, ancestry, diffs or commit subjects were verified; no phase-4 code exists, so every claim about how planned code behaves is either read from the plan or from the library source; nothing was compiled (`scripts/check.sh clippy` was not run, and would fail at the tool pin anyway); the container and Docker gates were not run; no measurement in this report is a timing or a load measurement. Probes ran in `/tmp/{remoteprobe,ferprobe,pathspec,cfgprobe,gcfg,cloneprobe,bareprobe,symprobe,sedprobe,rgtest}` and `~/.cargo/registry` sources were read only; I created no files inside the tree.
