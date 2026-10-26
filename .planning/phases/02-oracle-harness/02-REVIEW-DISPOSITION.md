---
phase: 02
review: 02-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "The harness never observed HEAD, the index or an in-progress operation (iteration 1; fixed 9c1f77e, widened to Exactly forms a0956e6)"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "Refusals pinned only by exit code (iteration 1; oracle-side stderr pins b835827, Rust-side rust_stderr rule 6bd5229)"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "The git ceiling did not protect commands run in the scenario root (iteration 1; fixed 4c1e4c0)"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "Observer missed remotes, non-standard refs and local config (iteration 1; fixed 18f8dfe, a0956e6)"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "CARGO_LLVM_COV_SETUP=no promised but never set (iteration 1; check.sh coverage target 790a8a2)"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "Adapter stdin writer could lose a run or deadlock (iteration 1; fixed c4841dc)"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "check.sh fmt depended on git silently (iteration 1; fixed 0d64008)"
  - id: IN-01
    severity: info
    disposition: deferred
    title: "The mode check reads the index, not the files the oracle copy executes"
  - id: IN-02
    severity: info
    disposition: deferred
    title: "`is_refusal` ignores files"
  - id: IN-03
    severity: info
    disposition: deferred
    title: "Pre-push input takes the first `ls-remote` line"
  - id: IN-04
    severity: info
    disposition: deferred
    title: "`CommitExpect` omits the committer date"
  - id: IN-05
    severity: info
    disposition: deferred
    title: "The nextest override can silently match nothing, and host runs have no timeout"
  - id: IN-06
    severity: info
    disposition: deferred
    title: "The coverage tests are vacuous outside a coverage run, and control B is not the decided control"
  - id: IN-07
    severity: info
    disposition: fixed
    title: "Stale docs: oracle README ceiling and new_commits (fixed 07c67ee)"
  - id: IN-08
    severity: info
    disposition: deferred
    title: "The index record has no entry flags, and the config record has no per-worktree config"
  - id: IN-09
    severity: info
    disposition: deferred
    title: "Six of the seven older `fixed:` defect texts omit the upstream their records now carry"
  - id: IN-10
    severity: info
    disposition: deferred
    title: "The observer now fails on any checkout whose HEAD is unborn"
---

# Phase 02 — Code Review Disposition

Review ran `--fix --auto`: three review iterations; iteration 3 found 0 critical, 0 warning, 9 info.
Critical/Warning rows above are iteration-1 findings (iteration 2's two warnings were partial fixes of
CR-01 and WR-01 and are folded into those rows). Info findings are triaged by the orchestrator.

| ID | Severity | Disposition | Source | Reason |
|----|----------|-------------|--------|--------|
| CR-01 | critical | fixed | orchestrator | Checkout HEAD, index, in-progress markers compared under Unchanged and Exactly; 8+ negative controls |
| WR-01 | warning | fixed | orchestrator | Oracle refusals pin stderr; enabling a Rust side requires a `rust_stderr` pin |
| WR-02 | warning | fixed | orchestrator | Ceiling at the scenario root's parent; control shows the old ceiling leaked |
| WR-03 | warning | fixed | orchestrator | All ref namespaces, bare remotes and local config observed |
| WR-04 | warning | fixed | orchestrator | `scripts/check.sh coverage` sets `CARGO_LLVM_COV_SETUP=no` |
| WR-05 | warning | fixed | orchestrator | Stdin written from its own thread; early-exit and 1 MiB controls |
| WR-06 | warning | fixed | orchestrator | fmt names git when missing, refuses outside a work tree, covers untracked .rs |
| IN-01 | info | deferred | orchestrator | Copy is made from the work tree; index mode test plus the copy's run cover the practical case. Revisit if a run ever fails on a lost exec bit |
| IN-02 | info | deferred | orchestrator | Refusal shapes are checked by refs/commits plus pinned stderr; file checks land with the Rust sides (phases 5-10) |
| IN-03 | info | deferred | orchestrator | Fixtures have one matching ref per name; becomes relevant only with ambiguous names — phase 6 (guard) |
| IN-04 | info | deferred | orchestrator | Deliberate: HARN-03 omits the committer date (orchestrator decision, STATE) |
| IN-05 | info | deferred | orchestrator | Report test is named in the registry test; a rename fails `rust_sides_not_enabled`. Host timeout belongs with CI-parity work (backlog 999.x) |
| IN-06 | info | deferred | orchestrator | Coverage is a manual, recorded proof by decision (no coverage gate); revisit when a coverage job is added |
| IN-07 | info | fixed | orchestrator | README and doc comment updated |
| IN-08 | info | deferred | orchestrator | No scenario uses skip-worktree or per-worktree config; add when a requirement needs it |
| IN-09 | info | deferred | orchestrator | Defect texts are documentation; the judge compares records, which carry the upstream. Tidy when those cut cases are touched in phase 9 |
| IN-10 | info | deferred | orchestrator | Fails loudly, not a false green; no fixture has an unborn HEAD. Handle when a scenario needs one |
