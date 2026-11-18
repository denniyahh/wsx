# Phase 5: Divergence check and command contract - Context

**Gathered:** 2026-11-18
**Status:** Ready for planning

<domain>
## Phase Boundary

`wsx check`, the first real command: offline, it reports every way the workspace branch differs from
the integration branch that `.wsx.conf` does not declare (DIV-01), every stale or redundant
declaration (DIV-02), and every tracked file under a tracked `core.hooksPath` directory that is not
committed executable (DIV-03). It also says how stale the local integration ref is (DIV-04). It sets the
`--json` and exit-code contract that every later command joins (IF-01), and the source-scan and
no-agent-files scenario that enforce IF-02. Building any other command is out of scope.

</domain>

<decisions>
## Implementation Decisions

The owner left all four areas to Claude ("you decide on all of these", 2026-11-18). Each call below
can be challenged at plan review.

### What the check compares
- **D-01:** Two-dot, tip to tip: the tree of the integration ref's tip against the tree of the
  workspace branch's tip, as the oracle does (`git diff --name-status --no-renames BASE_REF HEAD`).
  A merge-base comparison would pass an undeclared difference the oracle catches, which success
  criterion 1 forbids. One consequence is intended: when the integration branch changes a shared file
  the workspace hasn't merged yet, the check fails until the workspace syncs.
- **D-02:** The workspace side is the tip of the workspace branch the repo model names
  (`wsx.sourceBranch`, or the inferred branch under D-04), not `HEAD`. Run from a feature worktree,
  the check still checks the workspace branch. This is a recorded difference from the oracle, which uses
  `HEAD`; the oracle scenarios check out the workspace branch, so their outcomes are unchanged. Only
  committed state counts: the working tree and the index are never read.
- **D-03:** DIV-03 reads modes from the workspace tip's tree, not the index (the oracle uses
  `ls-files -s`, i.e. the index). `core.hooksPath` is git's effective value, read through the git
  binary (Phase 4 D-23). It counts only when it is a relative path naming a directory tracked in that
  tree. A `120000` entry passes only when its target resolves, within the tree, to a `100755` blob;
  resolution follows symlinked directories inside the tree, with a hop limit (owner, 2026-11-18, at plan
  review).
  Everything else under the directory that is not `100755` fails, naming the path and its mode.
  `scripts/*.sh` outside the hooks directory is not checked (DIV-03, success criterion 3).
- **D-04:** In a repo that is not configured (no `wsx.sourceBranch`), `wsx check` follows Phase 4 D-16
  and D-21. With exactly one candidate workspace branch and a readable manifest, it runs against that
  branch and reports `configured: false` with the branch it inferred. Otherwise it exits with
  the not-configured code (D-06), naming the candidates or saying to run `wsx init`. Unlike the push
  guard, `check` is something a person runs on purpose, so silence would mislead.
- **D-05:** Every problem is collected and reported in one run, in a fixed order: undeclared paths,
  stale declarations, hook modes, then by path in byte order within each group.
  Phase 5 needs no merge base, so the grafts and shallow gap (STATE, D-32) does not apply here. The model
  is resolved before any other Repo read (STATE obligation).

### Exit codes (IF-01)
- **D-06:** Exit codes are per class, not per error, and are the same for every command:
  | Code | Meaning |
  |------|---------|
  | 0 | Success; for `check`, no problems |
  | 1 | The command ran and found problems (`check`: divergence, stale declaration, hook mode) |
  | 2 | Usage: arguments the CLI does not accept (keeps today's `EXIT_USAGE`) |
  | 3 | Not configured: the command needs a workspace branch and none is configured or inferable |
  | 4 | Invalid configuration: `wsx.*` keys, the manifest, a missing remote or base ref, an empty integration chain |
  | 5 | Repository refused: an unsupported state wsx will not read (reftable, the object-store, relative-repo and implicit-worktree env refusals, a stale worktree, refs namespace, replace refs) |
  | 6 | Git failed: a git child failed, gave empty or bad output, or an object is missing |
  An orchestrator needs to tell "found problems" from "could not run" from "fix your setup"; codes
  for each variant would freeze the internal error enum into the public contract. The specific
  variant goes in the JSON `error.kind` field (D-08).
  — **Reversibility:** costly — once devflow (backlog 999.162) branches on these codes, changing
  one breaks it. Before the M1 release nobody consumes them.
- **D-07:** Every `git::Error` and `model::Error` variant maps to exactly one class, by an exhaustive
  `match` with no wildcard arm, so a new variant fails to compile until it is classified. This covers
  the STATE obligations: the phase 3 review refusals, `RefsNamespace`, `ObjectMissing`/`GitBadOutput`/
  `GitFailed`, and replacement refusals wherever they are wrapped (inside `Error::Git` or inside
  `NotConfigured`'s boxed reason, via `git::Error::is_replacement_refusal`), which take 5.
  `check::reftable_refused` gets its Rust side enabled, with `rust_stderr` naming reftable and exit 5
  replacing its placeholder 1. The `dead_code` expectations come off `src/model/mod.rs` and
  `src/git/mod.rs`.

### JSON contract (IF-01)
- **D-08:** With `--json`, stdout carries exactly one JSON object, on success and on failure alike.
  Every object has `command` (e.g. `"check"`) and `schema_version` (an integer, versioned per
  command, so one command's change doesn't bump the others). A failure object adds
  `error: {kind, message, exit_code}`. Human-readable text stays on stderr in both modes; the harness
  pins it through `rust_stderr`. Without `--json`, stdout carries the human report.
- **D-09:** Byte-valued fields (paths, ref names) cannot be assumed UTF-8. A valid UTF-8 value is a
  JSON string; anything else is encoded losslessly in a form the schema documents (research picks the
  encoding). A path is never lossily converted.
- **D-10:** The schema is a source-of-truth file per command and version, as JSON Schema under
  `docs/` (for example `docs/json/check.v1.schema.json`). It is documented beside `docs/exit-codes.md`,
  which lists D-06. The contract test (a) runs each command, its `--json` success and failure paths
  included, and validates the output against the schema file; (b) holds a pinned hash for each
  `(command, version)` schema file, so an edit without a version bump fails (the control from success
  criterion 4); (c) enumerates the command tree from the same table the argument parser uses, never a
  hand-kept list, and fails for a command with no pinned schema.
- **D-11:** `check`'s v1 payload reports `configured`, the workspace branch and tip, the integration ref
  and tip, the staleness object (D-12), and `problems: [{kind, path, detail}]`. `kind` is one of
  `undeclared_added`, `undeclared_modified`, `undeclared_deleted`, `stale_only`,
  `stale_modified`, `redundant_builtin` or `hook_not_executable`. The exact field names are left to
  planning, inside these semantics.

### Staleness (DIV-04)
- **D-12:** (Amended 2026-11-18 by the owner at plan time; the reflog-only form below it was replaced.)
  Two times are reported, each with its age. `ref_moved_at`: the newest reflog entry of the integration
  ref's remote-tracking ref (`refs/remotes/<baseRemote>/<branch>`), read through the git binary; null
  with the reason stated ("no reflog for <ref>") when there is none. `fetched_at`: the mtime of
  `FETCH_HEAD`, which is repo-wide and changes with any fetch; null with the reason stated when the file
  is absent. Git keeps one `FETCH_HEAD` per worktree, so "repo-wide" means the newest `FETCH_HEAD` across
  the main worktree and every linked worktree (owner, 2026-11-18, at plan review). Reason for the amendment: on git 2.56.1 a fresh clone writes no reflog entry for the
  remote-tracking ref, and a fetch that brings nothing new writes `FETCH_HEAD` but no reflog entry
  (reproduced in a scratch repo), so reflog alone reports "never fetched" after a clone and
  over-reports staleness against a quiet upstream. When the integration ref is a local branch (no base
  remote, Phase 4 D-07), the report says staleness doesn't apply.
- **D-13:** Staleness is reported, never a failure, and has no threshold. An offline check must not
  fail because the network is down, and the no-network rule (DIV-04) comes from the oracle's pre-push
  use. With the remote unreachable, the check runs against the local ref (success criterion 1); it never
  touches the network.

### IF-02 enforcement
- **D-14:** The source-scan test walks `src/` (comments stripped) for any path literal naming
  `ROADMAP.md`, `STATE.md`, `.planning/config.json`, `devflow.toml`, or settings under `.claude` or
  `.codex`. Its control is a planted read in a scratch copy that must trip it, asserting the intended
  failure text. A tracked `CLAUDE.md` or `.planning` path is still just a manifest path to wsx, and
  path *data* is not a read. The pending agent-hook exception (REQUIREMENTS 2026-11-15 amendment) is not
  built here: no allowlist entry is added until that scope is decided.
- **D-15:** The no-agent-files scenario runs every command in the command table (D-10c) in a fixture
  repo that has none of those files, and each command must behave the same as in the oracle's
  configured fixture.

### Claude's Discretion
All of D-01 to D-15 above, under the owner's "you decide on all of these". Left to research and
planning: argument parsing (hand-rolled or a crate, which needs a justification and a clean
`check.sh deps`), the JSON serializer and schema validator (dev-only where possible, same gate), the
byte-encoding form (D-09), and the module layout.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements and roadmap
- `.planning/ROADMAP.md` § Phase 5: goal and the five success criteria
- `.planning/REQUIREMENTS.md`: DIV-01..04, IF-01, IF-02, and the 2026-11-15 amendment under Boundary
  (agent hook configuration)
- `.planning/STATE.md` § Accumulated Context: the Phase 5 obligations (read order, exit codes for the
  phase 3/4 refusals, `check::reftable_refused`, `dead_code` removal, replacement refusals)

### Oracle
- `tests/oracle/check-workspace-divergence.sh`: the parity reference (tip-to-tip diff, stale
  declarations, mode rule)
- `tests/oracle/README.md`: oracle provenance

### Prior decisions this phase consumes
- `.planning/phases/04-repo-model/04-CONTEXT.md`: D-06..D-10 (chain), D-11..D-14 (manifest), D-15..D-21
  (fallback, not configured), D-22..D-28 (replace refs, inputs through the git binary)
- `.planning/phases/03-git-adapter/03-CONTEXT.md`: error variants that need codes; "Decided during code
  review"
- `.planning/phases/05-divergence-check-and-command-contract/MEMORY-RECALL.md`: MemPalace recall
  (signal checked against a nonsense control)
- `.planning/spikes/002-gix-object-reads`: grafts and shallow gap (not triggered here, D-05)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `model::resolve` (`src/model/mod.rs:372`) and `RepoModel::require` (`src/model/mod.rs:114`):
  give the workspace branch, the integration ref, and the parsed manifest with built-ins.
- `git::Repo::tree_diff` (`src/git/reads.rs:123`): tree-to-tree diff by object id, tested against
  git.
- `src/git/spawn.rs` `Git::run`: git-binary reads (hooksPath, reflog), per D-23.
- `src/git/quote.rs`: `c_quote` / `shell_word` for paths in prose and in printed fix commands.
- `tests/harness` `check.rs`, `registry.rs`: the recorded oracle scenarios whose Rust side this
  phase enables.

### Established Patterns
- Every refusal test keeps a positive control and asserts the intended failure text (04-LEARNINGS).
- One nextest filter per test with `--no-tests=fail` (CLAUDE.md repo traps).
- No `set_var`/`remove_var`; env is set on the child only.

### Integration Points
- `src/main.rs`: today only `--version`, with `EXIT_USAGE = 2`. The command table and the exit
  class mapping land here.

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond the oracle and the success criteria.

</specifics>

<deferred>
## Deferred Ideas

### Reviewed Todos (not folded)
- **Re-verify Phases 1-3 and 03.1 before milestone audit:** the owner picked it, then confirmed on
  2026-11-18 to keep it deferred until after the last phase that edits shared files.
- **Replace oracle-observed exit 128 under IF-01:** acts in phases 7 (sync) and 9 (cut). For reference,
  under D-06 the "base remote gone" case is code 4 (a key naming a missing remote, Phase 4 D-03), or 3
  when no remote is left and nothing can be resolved.
- **Re-check Dependabot after first merge to main:** waits on that merge.
- **Shell-quote ForeignOwner's printed fix command:** a `git::` fix, unrelated to the check.

</deferred>

---

*Phase: 05-divergence-check-and-command-contract*
*Context gathered: 2026-11-18*
