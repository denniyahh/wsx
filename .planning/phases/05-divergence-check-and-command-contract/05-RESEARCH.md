# Phase 5: Divergence check and command contract - Research

**Researched:** 2026-11-18
**Domain:** Rust CLI contract (argv, exit classes, versioned JSON), git tree comparison, IF-02 enforcement
**Confidence:** MEDIUM-HIGH (code and git behaviour measured this session; crate choices are recommendations)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
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
  tree. A `120000` entry passes only when its target resolves, within the tree, to a `100755` blob.
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
- **D-12:** The time of the last fetch is the newest reflog entry of the integration ref's
  remote-tracking ref (`refs/remotes/<baseRemote>/<branch>`), read through the git binary. That entry is
  specific to the ref; `FETCH_HEAD`'s mtime is repo-wide and changes with any fetch. The report gives
  the time and the age. With no reflog, `fetched_at` is null and the reason is stated ("no reflog for
  <ref>"). When the integration ref is a local branch (no base remote, Phase 4 D-07), the report says
  staleness doesn't apply.
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

### Deferred Ideas (OUT OF SCOPE)
### Reviewed Todos (not folded)
- **Re-verify Phases 1-3 and 03.1 before milestone audit:** the owner picked it, then confirmed on
  2026-11-18 to keep it deferred until after the last phase that edits shared files.
- **Replace oracle-observed exit 128 under IF-01:** acts in phases 7 (sync) and 9 (cut). For reference,
  under D-06 the "base remote gone" case is code 4 (a key naming a missing remote, Phase 4 D-03), or 3
  when no remote is left and nothing can be resolved.
- **Re-check Dependabot after first merge to main:** waits on that merge.
- **Shell-quote ForeignOwner's printed fix command:** a `git::` fix, unrelated to the check.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DIV-01 | Undeclared added path, or modified/deleted shared file not declared `modified`, fails `wsx check` | Algorithm in Pattern 2 over `Repo::tree_diff` + `covers`; scenario list in Pattern 6 |
| DIV-02 | Stale `only`, stale `modified`, declaration covered by a built-in fail | Pattern 2 (stale rules computed without pathspecs), Pitfall 4 |
| DIV-03 | Under a tracked `core.hooksPath`, every tracked file committed `100755` (or `120000` to a tracked executable); `scripts/*.sh` not checked | Pattern 3, git probes in Code Examples |
| DIV-04 | Offline against the local ref; says how stale the ref is | Pattern 4; **Pitfall 1 is evidence against D-12's wording** |
| IF-01 | `--json` with a versioned documented schema + documented exit codes; contract test pins schema | Patterns 1, 5; exit-class table; Standard Stack |
| IF-02 | No read of GSD/devflow/agent files; source scan + no-agent-files scenario | Pattern 7 |
</phase_requirements>

## Summary

**Evidence against a locked call, first.** D-12 reads "time of the last fetch" from the newest reflog entry of `refs/remotes/<baseRemote>/<branch>`. Measured on git 2.56.1 this session: a fresh `git clone` writes **no** reflog for `refs/remotes/origin/develop` (`git reflog exists` → no; `.git/logs/refs/remotes/origin/` holds only `HEAD`), and a `git fetch` that finds the ref already current writes **no** entry, while `FETCH_HEAD` is rewritten (mtime = wall clock, ignoring `GIT_COMMITTER_DATE`). Only a fetch that moves the ref writes an entry (`origin/develop@{1300000000}|fetch -q origin: fast-forward`). So the reflog gives "when this ref last moved", and is null on every fresh clone until upstream moves. A user who fetches daily against a quiet upstream would be told the ref is weeks stale. This needs the operator's call before the staleness field is named (Open Question 1).

The rest is a well-bounded build on Phase 4: `model::resolve` already yields the workspace tip, integration tip and parsed manifest; `Repo::tree_diff` gives tip-to-tip changes with `Added/Deleted/Modified/TypeChanged`; `manifest::covers` gives literal prefix coverage. The check engine can be a pure function over those values plus three new git-binary reads (tree lookups for stale `only`, the hooks subtree listing with modes, and the reflog/hooksPath reads). No pathspec is ever needed, which removes the oracle's glob-magic behaviour from the port.

The contract work has two traps measured here: removing the module-level `dead_code` expectations (a STATE obligation) produces 15 clippy errors with a check-shaped `main`, so item-level expectations are part of the work; and the binary-only crate means tests cannot import the command table, so the table must be a data-only module that tests include by `#[path]`.

**Primary recommendation:** Build `check` as a pure evaluator over resolved inputs, emit JSON through `serde_json::Value`, parse argv with `lexopt` over a data-only `COMMANDS` table, classify every error through two exhaustive `match`es into the seven D-06 classes, and pin schema files by `git hash-object --no-filters` exactly as `provenance.rs` already does.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| argv parsing, `--json` switch, usage errors (exit 2) | CLI boundary (`src/main.rs`, `src/cli/`) | — | Only place that knows the command table; tests enumerate the same table |
| Exit class per error (D-06/D-07) | CLI boundary (`src/exit.rs`) | model/git error types | Exhaustive `match` lives beside the types it classifies, consumed by main |
| JSON envelope, byte encoding (D-08/D-09) | CLI boundary (`src/json.rs`) | — | One serializer; per-command payload builders call it |
| Divergence/stale/hook evaluation (DIV-01..03) | Domain (`src/check/`) | model (`covers`, `personal_paths`) | Pure function over resolved values: unit-testable without git |
| Tree diff, tree lookups, hooks listing, reflog, hooksPath | Git adapter (`src/git/`) | — | D-23: inputs through the git binary; gix only by object id |
| Model resolution before any other read | Model (`model::resolve`) | — | STATE obligation (read order, replace-ref guard) |
| Parity, contract, source scan | Tests (`tests/harness`, `tests/contract.rs`, `tests/if02_scan.rs`) | `docs/json/` | Integration tests see only the binary plus `#[path]`-included data modules |

## Standard Stack

### Core
| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| serde_json | 1.0.154 (crates.io API, published 2026-10-11) | Emit the one JSON object per run; tests parse it | Correct RFC 8259 escaping of every string; build `Value`/`json!`, no derive needed. Normal deps per crates.io: `itoa`, `memchr`, `serde`, `serde_core` (all already in `Cargo.lock`: itoa 1.0.18, memchr 2.8.3, serde/serde_core 1.0.229 [VERIFIED: rg Cargo.lock]) plus `zmij ^1.0` (new; 1.0.23, dtolnay). Choice [ASSUMED]; registry and legitimacy verified |
| lexopt | 0.3.2 (published 2026-02-28) | argv lexing over `OsString` | Zero dependencies, MIT; handles `--`, `--flag=value`, non-UTF-8 args; the table-driven dispatch stays wsx's own. Choice [ASSUMED] |

### Supporting (dev-only)
| Library | Version | Purpose | When |
|---------|---------|---------|------|
| jsonschema | 0.61.2 (published 2026-11-12), `default-features = false` | Validate `--json` output against `docs/json/<cmd>.v<N>.schema.json` (D-10a) | Contract test only. Default features are `resolve-http`, `resolve-file`, `tls-aws-lc-rs`, `idna`; even without them its non-optional graph is large (ahash, bytecount, data-encoding, email_address, fancy-regex, fraction, getrandom, regex, referencing, strum, uuid-simd, …) [VERIFIED: crates.io dependencies API]. Licenses of that graph are [ASSUMED] compatible with `deny.toml`'s `["Apache-2.0", "MIT", "Unicode-3.0", "Zlib"]` until `scripts/check.sh deps` runs |
| proc-macro2 | 1.0.107 (already in `Cargo.lock`) | Tokenise `src/**/*.rs` for the IF-02 scan, which drops comments for free | Adds no crate to the graph. Behaviour outside a proc macro (fallback lexer; doc comments become `#[doc = "…"]` literal tokens) is [ASSUMED]; the scan's planted-read control proves it |

API confirmed in docs this session: `jsonschema::validator_for(&schema) -> Result<Validator, ValidationError>`, `validator.iter_errors(&instance)` [CITED: context7 /stranger6667/jsonschema].

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| jsonschema | boon 0.6.1 | Fewer deps but still regex/url/idna; last release 2025-01-07 |
| jsonschema | hand-written subset validator | No deps, but "validates against the schema file" then means "against what we chose to implement"; rejected under Don't Hand-Roll |
| lexopt | clap 4.6.8 | Gives `get_subcommands()` enumeration and help text, costs clap_builder/clap_lex/anstyle (+ syn with derive) and owns error printing that must be bent to the JSON-on-failure rule |
| lexopt | hand-rolled `match` on `args_os` (today's main) | Fine for `check [--json]`; grows badly once `cut --scope X [pr]`, `hook pre-push <remote> <url>` land |
| serde_json | hand-written JSON writer | ~60 lines, but the tests need a parser anyway, so the crate is coming in regardless |
| schema hash by sha2 | `git hash-object --no-filters` | sha2 0.11.1 is in the lock already, but `tests/harness/provenance.rs:129` already hashes with git; reuse that |

**Installation (Cargo.toml):**
```toml
[dependencies]
serde_json = "1.0.154"
lexopt = "0.3.2"
[dev-dependencies]
jsonschema = { version = "0.61.2", default-features = false }
proc-macro2 = "1.0.107"
```
Each needs the justification comment this `Cargo.toml` gives every dependency (CLAUDE.md), and a clean `scripts/check.sh deps`.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads (recent, crates.io) | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| serde_json | crates | since 2015-08 | 444.9M | github.com/serde-rs/json | OK | Approved |
| lexopt | crates | since 2021-07 | 3.46M | github.com/blyxxyz/lexopt | OK | Approved |
| jsonschema | crates | since 2020-03 | 43.2M | github.com/Stranger6667/jsonschema | OK | Approved (dev-only) |
| boon | crates | since 2023-02 | 0.46M | github.com/santhosh-tekuri/boon | OK | Alternative only |
| zmij (transitive of serde_json) | crates | since 2025-12 | 366.9M | github.com/dtolnay/zmij | not run through seam | Arrives with serde_json; `check.sh deps` covers it |

Verdicts from `gsd-tools query package-legitimacy check --ecosystem crates` this session; age/downloads from the crates.io API. Crates have no postinstall; `build.rs` of new crates is unreviewed [ASSUMED none harmful].
**Packages removed due to [SLOP]:** none. **Flagged [SUS]:** none. All package *choices* are [ASSUMED] (training knowledge), so the planner gates each addition behind `check.sh deps` and the Cargo.toml justification.

## Architecture Patterns

### System Architecture Diagram

```
argv ──► cli::parse (lexopt over COMMANDS table) ──usage error──► exit 2 (+ JSON envelope if --json)
            │ Command::Check{json}
            ▼
   git::Repo::discover ──refusal──┐
            ▼                     │
   model::resolve (first read) ───┤  (Configured | NotConfigured{inferred})
            │ Model (tips, integration ref, manifest)   │
            ▼                                           │
   git reads by id: tree_diff(int_tip, ws_tip) ─────────┤
                    lookup int_tree:<only path> (-Z)    │
                    core.hooksPath (git config --get)   │
                    ls-tree -r -z ws_tip:<hooksdir>     │
                    reflog of refs/remotes/<r>/<b>      │
            ▼                                           ▼
   check::evaluate (pure) ──► Report{problems, staleness} │ exit::classify(err) ──► class 3/4/5/6
            ▼                                           │
   render: human → stdout/stderr ; --json → one object on stdout
            ▼
   exit 0 (no problems) | 1 (problems)
```

### Recommended Project Structure
```
src/
├── main.rs            # collect args_os, call cli::run, return ExitCode
├── cli/mod.rs         # lexopt parsing, dispatch, --json handling, usage text
├── cli/commands.rs    # DATA ONLY: COMMANDS table (name, schema_version); #[path]-included by tests
├── exit.rs            # ExitClass {Ok=0, Problems=1, Usage=2, NotConfigured=3, InvalidConfig=4, Refused=5, GitFailed=6}; classify()
├── json.rs            # envelope {command, schema_version, ...}; bytes_value() (D-09)
├── check/mod.rs       # evaluate(): undeclared, stale, redundant, ordering (D-05)
├── check/hooks.rs     # hooksPath rule, symlink resolution within the tree
├── git/…              # new reads: tree lookups, ls-tree listing, reflog time, hooksPath
└── model/…            # unchanged except dead_code expectations
docs/
├── exit-codes.md      # D-06 table
└── json/check.v1.schema.json (+ error envelope, see Open Question 3)
tests/
├── contract.rs        # D-10 a/b/c
├── if02_scan.rs       # D-14 (+ planted-read control)
└── harness/check.rs   # enable Rust sides, add DIV scenarios
```

### Pattern 1: Exhaustive exit classification (D-07)
Two functions, no wildcard arm, each nested enum matched variant by variant so a new variant fails to compile.

Variants, quoted from the definitions read this session:
- `git::Error` [VERIFIED: src/git/mod.rs:102-205]: `NotARepo`, `ForeignOwner`, `Reftable`, `GitTooOld`, `GitFailed`, `GitTimedOut`, `OpenTimedOut`, `GitEmptyOutput`, `GitBadOutput`, `RelativeRepoEnv`, `ObjectStoreEnv`, `RefsNamespace`, `ImplicitWorkTree`, `StaleWorktree`, `Io`, `Gix`, `ObjectMissing`, `ReplaceRefsPresent`, `ReplaceBaseSet`, `ReplaceRefsUnreadable`; helper `is_replacement_refusal` (src/git/mod.rs:207-215).
- `model::Error` [VERIFIED: src/model/mod.rs:134-177]: `Git(git::Error)`, `KeyRepeated`, `KeyEmpty`, `NotABranchName`, `NoSuchRemote`, `BaseRefMissing`, `NoIntegrationBranch`, `IntegrationRefSymbolic`, `WorkspaceBranchMissing`, `WorkspaceCandidates`, `ManifestMissing`, `ManifestNotABlob`, `ManifestUnreadable { branch, source: git::Error }`, `ManifestInvalid`, `WorktreeDirInvalid`, `NotConfigured { reason: Box<Error> }`.

Recommended mapping (D-06 names classes; rows marked * are not named by D-06 and are this research's recommendation):

| Class | Code | Variants |
|-------|------|----------|
| NotConfigured | 3 | `WorkspaceCandidates`; `NotConfigured{reason}` unless `reason` holds a replacement refusal; `NotARepo`* (no repository, so nothing is configured or inferable) |
| InvalidConfig | 4 | `KeyRepeated`, `KeyEmpty`, `NotABranchName`, `NoSuchRemote`, `BaseRefMissing`, `NoIntegrationBranch`, `IntegrationRefSymbolic`*, `WorkspaceBranchMissing`*, `ManifestMissing`, `ManifestNotABlob`, `ManifestInvalid`, `WorktreeDirInvalid` |
| Refused | 5 | `ForeignOwner`, `Reftable`, `RelativeRepoEnv`, `ObjectStoreEnv`, `RefsNamespace`, `ImplicitWorkTree`, `StaleWorktree`, the three replacement refusals wherever wrapped (`Error::Git`, `NotConfigured`'s reason, `ManifestUnreadable`'s source) |
| GitFailed | 6 | `GitFailed`, `GitTimedOut`, `GitEmptyOutput`, `GitBadOutput`, `ObjectMissing`, `Gix`*, `Io`*, `OpenTimedOut`*, `GitTooOld`* |
| (recurse) | — | `model::Error::Git(inner)` → class of `inner`; `ManifestUnreadable{source}` → class of `source` (so a missing object is 6, a replace ref 5) |

```rust
// Shape only; names [ASSUMED]
pub fn of_git(err: &git::Error) -> ExitClass {
    use git::Error as G;
    match err {
        G::ForeignOwner { .. } | G::Reftable { .. } | G::RelativeRepoEnv { .. }
        | G::ObjectStoreEnv { .. } | G::RefsNamespace { .. } | G::ImplicitWorkTree { .. }
        | G::StaleWorktree { .. } | G::ReplaceRefsPresent { .. } | G::ReplaceBaseSet { .. }
        | G::ReplaceRefsUnreadable { .. } => ExitClass::Refused,
        G::NotARepo { .. } => ExitClass::NotConfigured,
        G::GitTooOld { .. } | G::GitFailed { .. } | G::GitTimedOut { .. } | G::OpenTimedOut { .. }
        | G::GitEmptyOutput { .. } | G::GitBadOutput { .. } | G::Io { .. } | G::Gix { .. }
        | G::ObjectMissing { .. } => ExitClass::GitFailed,
    }
}
```
Unit-test the table against one constructed value per variant; the `error.kind` string (D-08) is the variant name in snake_case from the same match.

### Pattern 2: Check evaluation without pathspecs
Inputs: `changes = repo.tree_diff(&model.integration.tip, &model.workspace_tip)` (order: from integration to workspace, so `Added` = only on workspace), `builtins = [".wsx.conf", model.worktree_dir]` (first two of `personal_paths()`, src/model/mod.rs:731-741: `let mut paths = vec![MANIFEST_NAME, self.worktree_dir.as_slice()];`), `entries = model.manifest.entries` (`EntryKind::Only | EntryKind::Modified`, src/model/manifest.rs:20-26).

```
for change in changes:
    if any builtin covers change.path: continue                      # oracle: under BUILTIN → continue
    match change.kind:
        Added                         -> ok iff some Only entry covers path      else undeclared_added
        Modified | TypeChanged        -> ok iff some Modified entry == path      else undeclared_modified
        Deleted                       -> ok iff some Modified entry == path      else undeclared_deleted
for entry in entries:
    if any builtin covers entry.path:                 redundant_builtin  (D-13 of phase 4 kept it parseable)
    Only:     if integration tree has entry.path:     stale_only         (tree lookup, any entry kind)
    Modified: if no change.path is covered by entry:  stale_modified     (oracle: git diff --quiet -- entry)
then hook problems; sort each group by path bytes (D-05)
```
`covers` is literal prefix-or-equal (src/model/manifest.rs:315-318: `path.strip_prefix(entry).is_some_and(|rest| rest.is_empty() || rest.starts_with(b"/"))`). The `modified` match is exact equality because the oracle uses `grep -Fxq`; the stale-`modified` test uses `covers` because the oracle's `git diff -- entry` pathspec matches the path and everything below it. `TypeChanged` has no kind of its own in D-11, and the oracle's `T` status falls into its `*)` arm, so it reports as `undeclared_modified`. Whether a `redundant_builtin` entry should also be skipped for the stale checks is a small planning choice; recommend reporting only `redundant_builtin` for it.

### Pattern 3: Hook modes from the workspace tree (D-03)
1. `git config --get core.hooksPath` run in the worktree root (git 2.56.1: multi-valued → last value, exit 0; verified). Exit 1 with empty output means unset → no hook check.
2. Accept only a relative value. Strip one leading `./` and one trailing `/`, then `manifest::normalize_path` (refuses empty, absolute, `.`/`..` components, NUL). Anything else: no hook check, and say why in the report (never silently skip).
3. Request `<ws_tip>:<dir>` in a `cat-file --batch-check -Z` child; it counts only when that entry's mode is `040000`.
4. `git ls-tree -r -z <dir tree id>`: records `<mode> SP <type> SP <oid> TAB <path> NUL` (verified, newline in a name survives). `100755` passes; `120000` → read the blob (target), resolve it against the link's directory within the tree (`.` skip, `..` pop, popping past the root or an absolute target fails), look the result up at `ws_tip`, follow further `120000` up to a hop limit (40 [ASSUMED], the Linux MAXSYMLINKS value), pass only on `100755`. Every other mode (`100644`, `160000`, a dangling or escaping link) is `hook_not_executable` with path and mode.
`scripts/*.sh` outside the directory is never listed.

### Pattern 4: Staleness (DIV-04), as D-12 currently reads
- Integration ref under `refs/remotes/` only; a local integration branch → `applies: false`.
- `git reflog show -n1 --no-show-signature --date=unix --format=%gd <full_ref>` prints `<short>@{<unix>}`; parse the number after the last `@{` (ref names cannot contain `@{`). No output → `fetched_at: null`, reason `no reflog for <ref>`.
- Never `%ct`: that is the commit's time, not the reflog entry's.
- Age = `SystemTime::now()` − entry time; tests pin the field's presence and the timestamp (the harness fixes `GIT_COMMITTER_DATE`, which the reflog entry honours, verified), never the age.
See Pitfall 1 before naming fields.

### Pattern 5: JSON contract (D-08..D-10)
- One object on stdout whenever `--json` is given, success or failure; text to stderr always. `Map` keys are sorted by serde_json without `preserve_order` [ASSUMED]; nothing depends on order.
- D-09 encoding (recommendation): a byte field is a JSON string when `std::str::from_utf8` succeeds, else `{"hex": "<lowercase hex of the raw bytes>"}`. Schema: `"oneOf": [{"type": "string"}, {"type": "object", "required": ["hex"], "properties": {"hex": {"type": "string", "pattern": "^([0-9a-f]{2})+$"}}, "additionalProperties": false}]`. Hex needs no crate and decodes in one call in any orchestrator language; serde_json already escapes control bytes inside valid UTF-8 (`a\nb.md` stays a string).
- Contract test (a): run the binary for each table entry's success and failure fixture with `--json`, parse stdout as exactly one value, validate. (b): `git hash-object --no-filters docs/json/<cmd>.v<N>.schema.json` equals a pinned constant per `(command, version)`; control: a scratch copy with one field renamed must trip it with the intended message. (c): every `COMMANDS` entry has a schema file and a pinned hash; every schema file belongs to an entry.
- The table: `src/cli/commands.rs` holds only `pub const COMMANDS: &[CommandSpec]` (name, schema version); `cli` parses against it; `tests/contract.rs` and the harness include it with `#[path = "../src/cli/commands.rs"] mod commands;` [CITED: doc.rust-lang.org/reference/items/modules.html, the path attribute]. Keep it free of `crate::` imports so it compiles in both places.

### Pattern 6: Scenarios to add or enable in `tests/harness/check.rs`
Existing: `self_test`, `quoted_personal_names`, `fork_layout`, `no_gsd_repo`, `no_gsd_custom_personal_path` (Rust-only), `reftable_refused`, all `rust = "Rust side not enabled until phase 5"` (read this session). Under `rust_enabled`, every case expecting a non-zero exit needs `rust_stderr` (macro doc, tests/harness/scenario.rs:642-650). `reftable_refused` moves from `verdict(1)` to exit 5 with `rust_stderr` naming reftable (STATE).

| New scenario | Fixture | Oracle | Class |
|---|---|---|---|
| undeclared modified | workspace edits `README.md` | exit 1 | Parity |
| undeclared deleted | workspace deletes a shared file | exit 1 | Parity |
| stale `only` | integration gains a path the workspace declares `only`; workspace merged it (else a `Deleted` problem also fires) | exit 1 | Parity |
| stale `modified` | `modified README.md` with README unchanged | exit 1 | Parity |
| redundant built-in | `only .worktrees/x` | exit 0 (oracle has no such rule) | Regression (wsx refuses on purpose) |
| hook `100644` executable on disk | `core.hooksPath=scripts/hooks`, commit 100644, then `chmod +x` unstaged | exit 1 | Parity |
| hook symlink `120000` → tracked `100755` | same hooksPath | exit 1 (oracle demands 100755) | Fixed |
| `scripts/x.sh` `100644`, no hooksPath coverage | — | exit 1 | Fixed (devflow convention) |
| remote unreachable | `git remote set-url origin <nonexistent path>` | exit 0 | Parity; Rust side pins the staleness line in `rust_stderr` or via `--json` test |
| run from a non-workspace checkout (D-02) | HEAD on another branch with an undeclared difference | uses HEAD | Rust-only or Fixed |
Each guard keeps a known-positive and a clean control (HARN-05). `RepoConfig::with_manifest_entry` appends the line to both `.wsx.conf` and `.workspace-divergence` (tests/harness/fixture.rs:368-386), so one call declares for both sides. Hook files must themselves be declared or shared, or they surface as undeclared added paths.

### Pattern 7: IF-02 source scan (D-14) and no-agent-files run (D-15)
- Walk `src/**/*.rs`, tokenise each with `proc_macro2::TokenStream::from_str`, skip `#[doc = …]` groups, and test every string/byte-string literal for: `ROADMAP.md`, `STATE.md`, `.planning/config.json`, `devflow.toml`, `.claude/` with `settings`, `.codex/`. Today `src/` has zero hits for `ROADMAP\.md|STATE\.md|config\.json|devflow\.toml|\.claude|\.codex` [VERIFIED: rg, empty result].
- Control: copy `src/` to a scratch dir, plant `std::fs::read(".planning/config.json")`, run the same scan function, assert the failure text names the file and literal; and a comment-only mention must not trip it (second control).
- D-15: iterate `COMMANDS`; the existing `RepoConfig::no_gsd()` fixture already has no GSD/devflow/agent files (`no_gsd_layout_has_no_gsd_devflow_or_agent_files`, fixture.rs:724).

### Anti-Patterns to Avoid
- **Pathspecs from manifest paths** (`git diff -- <entry>`, `ls-tree -- <entry>`): glob magic and the 04-04 pathspec surprise; compute from `tree_diff` + `covers` and look up by `<tree>:<path>` with `-Z`.
- **gix reads before `resolve`**: bypasses the replace-ref guard (STATE).
- **A wildcard arm in the exit mapping**: defeats D-07.
- **Reading the index or working tree for modes**: D-02/D-03 read committed trees only.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON string escaping | a writer | serde_json | Control bytes, surrogates, number formats |
| JSON Schema validation | a subset validator | jsonschema (dev) | "valid against the file" must mean the spec |
| Rust comment stripping | a regex/state machine | proc-macro2 tokens | raw strings, nested block comments, `'"'` chars |
| Content hash of schema files | a SHA routine | `git hash-object --no-filters` (as provenance.rs) | already the repo's pattern |
| C-quoting paths in prose | new quoting | `git::c_quote` / `shell_word` (src/git/quote.rs) | escape-byte safety already tested |

## Common Pitfalls

### Pitfall 1: The reflog is not a fetch log
**What goes wrong:** `fetched_at` is null after every fresh clone and stays at the last *upstream move*, not the last fetch.
**Evidence (git 2.56.1, this session):** after clone, `git reflog exists refs/remotes/origin/develop` → `no`; after a no-op `git fetch`, still `no`, while `FETCH_HEAD` was rewritten; after a moving fetch: `origin/develop@{1300000000}|fetch -q origin: fast-forward`.
**How to avoid:** name the field for what it measures, or change the source (Open Question 1). Also: a `git push` to that branch moves the remote-tracking ref with an "update by push" entry, which is not a fetch.

### Pitfall 2: Removing the `dead_code` expectations breaks clippy
**Evidence:** scratch copy with both module attributes removed and a `main` calling `discover`, `resolve`, `tree_diff`, `personal_paths`: `cargo clippy -- -D warnings` failed with 15 errors, e.g. `src/git/reads.rs:58:12: methods 'merge_base', 'ahead_behind', 'range_walk', 'commit_ids', and 'commit_id' are never used`, `src/git/repo.rs:152:12: methods 'index_path', 'siblings', and 'list_failed' are never used`, `src/model/mod.rs:113:12: method 'require' is never used`, `src/model/manifest.rs:292:8: function 'encode_path' is never used`, `src/git/mod.rs:173:5: variant 'StaleWorktree' is never constructed`.
**How to avoid:** item-level `#[cfg_attr(not(test), expect(dead_code, reason = "first caller in phase N"))]` on what `check` still does not reach; the real `check` will use `require`/`is_replacement_refusal` via the classifier, so the final list is smaller than 15.

### Pitfall 3: Line-based batch input
Paths can hold newlines. Use `cat-file --batch-check -Z` (NUL in and out; verified with a path `hooks/a\nb`) or the existing batch helper's framing.

### Pitfall 4: Stale-`only` fixtures produce two problems
A path present on integration and declared `only` on an unsynced workspace also shows as `Deleted` → `undeclared_deleted`. Merge integration into the workspace in the fixture if the case is about staleness alone.

### Pitfall 5: `--json` output contaminated
Any `println!` of human text in JSON mode breaks "exactly one object". Route human text through one sink that is stderr in JSON mode.

### Pitfall 6: nextest filters
One `--no-tests=fail -E 'test(=module::name)'` per test, chained with `&&` (CLAUDE.md).

### Pitfall 7: hooksPath forms
`~/hooks` (git expands `~` for path config), absolute paths into the worktree, `./scripts/hooks/`. D-03 counts only relative values naming a tracked directory; report the reason when the check does not apply.

## Code Examples

```rust
// Byte field per D-09 (recommendation); serde_json API [ASSUMED from training, verify with cargo doc]
fn bytes_value(raw: &[u8]) -> serde_json::Value {
    match std::str::from_utf8(raw) {
        Ok(text) => serde_json::Value::String(text.to_owned()),
        Err(_) => serde_json::json!({ "hex": raw.iter().map(|b| format!("{b:02x}")).collect::<String>() }),
    }
}
```
```rust
// Contract test (a), validator API from context7 /stranger6667/jsonschema
let schema: serde_json::Value = serde_json::from_slice(&std::fs::read(schema_path)?)?;
let validator = jsonschema::validator_for(&schema)?;
let output: serde_json::Value = serde_json::from_slice(&run.stdout)?; // exactly one value
let errors: Vec<String> = validator.iter_errors(&output).map(|e| e.to_string()).collect();
assert!(errors.is_empty(), "{cmd} --json drifts from {schema_path}: {errors:#?}");
```
```sh
# git probes measured this session (git 2.56.1)
git config --get core.hooksPath                      # last of several values, exit 0; exit 1 + empty when unset
git ls-tree -r -z <tree-id>                          # "<mode> <type> <oid>\t<path>\0"
printf '<tip>:<path>\0' | git cat-file --batch-check='%(objectname) %(objecttype) %(objectmode)' -Z
git reflog show -n1 --date=unix --format=%gd <full-ref>   # "<short>@{<unix>}" or nothing
```

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| Oracle: `git diff BASE HEAD`, index modes, `scripts/*.sh` rule, devflow FORBIDDEN_PATHS built-ins | wsx: workspace-branch tip (D-02), tree modes (D-03), hooksPath-only rule, built-ins `.wsx.conf` + worktree dir | Recorded differences in Pattern 6 |
| main.rs `EXIT_USAGE = 2` only (src/main.rs:13, read) | seven classes D-06 | `tests/cli.rs` usage test keeps passing at 2 |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | serde_json/lexopt/jsonschema are the right picks | Standard Stack | Swap crate; small rework |
| A2 | jsonschema's graph passes `deny.toml` licenses/bans | Standard Stack | Use boon or another validator |
| A3 | proc-macro2 lexes files outside a proc macro and turns doc comments into `#[doc]` literals | Pattern 7 | Scan misses or over-matches; the controls catch it |
| A4 | Exit classes for rows marked * | Pattern 1 | Contract changes before M1 are cheap (D-06 reversibility note) |
| A5 | Hex form for non-UTF-8 bytes | Pattern 5 | Schema v1 changes once |
| A6 | Symlink hop limit 40 | Pattern 3 | Edge case only |
| A7 | serde_json Map sorts keys without preserve_order | Pattern 5 | None functional |

## Open Questions (RESOLVED)

1. **Staleness source (needs the operator; challenges D-12).** What we know: reflog = last ref move, null after clone; FETCH_HEAD = last fetch of anything, repo-wide, wall-clock mtime. Options: (a) keep D-12, name the field `ref_updated_at` with reason text "no reflog for <ref>; it has not moved since clone"; (b) report both, `ref_updated_at` (reflog) and `last_fetch_at` (FETCH_HEAD mtime, labelled repo-wide); (c) FETCH_HEAD only. Recommendation: (b) — it is the only one that answers "how stale" on a fresh clone without overstating precision; the harness can pin (a)'s timestamp but only the presence of (b)'s.
   **RESOLVED** by D-12 as amended (owner, 2026-11-18): option (b), fields `ref_moved_at` (reflog) and `fetched_at` (the newest `FETCH_HEAD` mtime across the main and every linked worktree), each null with its reason; planned in 05-03 Task 2 and 05-07.
2. **Exit class for `NotARepo`, `GitTooOld`, `Io`, `OpenTimedOut`, `Gix`, `IntegrationRefSymbolic`, `WorkspaceBranchMissing`.** D-06 does not name them; Pattern 1 recommends. Inside Claude's discretion.
   **RESOLVED** in 05-01's context: `NotARepo` 3; `GitTooOld`, `Io`, `OpenTimedOut`, `Gix` 6; `IntegrationRefSymbolic`, `WorkspaceBranchMissing` 4.
3. **JSON for usage errors and `--version`.** `wsx bogus --json` has no command schema. Recommendation: one shared error-envelope schema (`docs/json/error.v1.schema.json`, `command` = the word given or null) referenced by every command schema; `--version` stays a flag outside the table. Inside Claude's discretion.
   **RESOLVED**, the shared error schema not adopted: `--version` stays outside `COMMANDS` (05-01's grammar); a usage error after a recognised command with `--json` prints that command's failure object (kind `usage`, exit_code 2), and no argument or an unknown first word prints no JSON (05-02's grammar, documented in `docs/json/README.md` by 05-08).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| git | everything | ✓ | 2.56.1 (min enforced `MIN_GIT_VERSION: (u32, u32) = (2, 54)`, src/git/spawn.rs:29, rg) | — |
| rustc/cargo | build | ✓ | 1.99.1 (pinned in rust-toolchain.toml, read) | — |
| cargo-nextest | tests | ✓ host 0.9.152; pinned 0.9.148 in install-tools.sh | — | container gate |
| cargo-deny | `check.sh deps` for new crates | pinned 0.20.2 | — | — |
| crates.io network | first fetch of new crates | ✓ (API reached this session) | — | none: adding a crate needs one online fetch |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test via cargo-nextest (pinned 0.9.148), proptest 1.12 |
| Config file | `scripts/check.sh` |
| Quick run command | `scripts/check.sh test -E 'test(=<module::name>)'` |
| Full suite command | `scripts/check.sh all` (and `scripts/check-in-container.sh all`) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DIV-01 | undeclared add/modify/delete fail; declared passes | harness scenario (both sides) + unit `check::` | `scripts/check.sh test -E 'test(=check::self_test::rust)'` | ✅ check.rs (enable) / ❌ new scenarios |
| DIV-02 | stale only/modified, redundant built-in | harness + unit | per scenario | ❌ Wave 0 |
| DIV-03 | 100644 hook fails, 120000→100755 passes, scripts/*.sh not checked | harness + unit (symlink resolution) | per scenario | ❌ Wave 0 |
| DIV-04 | runs with remote unreachable; staleness reported | harness + cli `--json` test | per test | ❌ Wave 0 |
| IF-01 | schema validation, hash pin + control, table coverage, exit table | integration `tests/contract.rs` + unit `exit::` | per test | ❌ Wave 0 |
| IF-02 | source scan + planted control; every command in no-GSD repo | integration `tests/if02_scan.rs`, harness | per test | ❌ Wave 0 |

### Sampling Rate
- Per task commit: the named tests touched, one filter each.
- Per wave merge: `scripts/check.sh all`.
- Phase gate: `scripts/check-in-container.sh all` and `scripts/check.sh deps` green.

### Wave 0 Gaps
- [ ] `tests/contract.rs`, `tests/if02_scan.rs`
- [ ] `docs/json/check.v1.schema.json`, `docs/exit-codes.md`
- [ ] new scenarios in `tests/harness/check.rs`; `tests/harness/rust.rs` argv for `check` stays `["check"]`

## Security Domain

### Applicable ASVS Categories (L1)
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | manifest parser (phase 4), `normalize_path` for hooksPath, exact-byte paths, no pathspecs |
| V6 Cryptography | no | (schema hash is integrity pinning, via git) |
| V12 Files | yes | symlink targets resolved only inside the committed tree; no filesystem reads |
| V14 Configuration | yes | config read through git; replace-ref guard before object reads |

### Known Threat Patterns
| Pattern | STRIDE | Mitigation |
|---------|--------|-----------|
| Terminal escape bytes in path names on stderr | Tampering | `git::c_quote` for every path in prose |
| JSON injection via path bytes | Tampering | serde_json escaping; D-09 encoding for non-UTF-8 |
| Pathspec/glob injection through manifest paths | Tampering | no pathspec built from data |
| Argument injection into git | Elevation | only object ids and `<id>:<path>` on stdin, never data as argv |
| Symlink loop or escape in hooks dir | DoS / Spoofing | hop limit; escape fails the entry |
| Replace refs altering what is read | Spoofing | `resolve` first (refuse_replace_refs) |
| Network access while "offline" | Info disclosure | no fetch code path; unreachable-remote scenario |

## Sources

### Primary (HIGH)
- Repo source read this session: src/main.rs, src/git/mod.rs:95-229, src/model/mod.rs:1-420 and 720-760, src/model/manifest.rs:1-60 and 255-335, src/git/reads.rs:15-75, src/git/config.rs:40-110, tests/harness/{check,rust,registry}.rs, tests/harness/scenario.rs:586-830, tests/oracle/check-workspace-divergence.sh, oracle.patch, Cargo.toml, clippy.toml
- git 2.56.1 probes in a scratch repo (reflog, FETCH_HEAD, hooksPath, ls-tree -z, cat-file -Z)
- clippy run on a scratch copy (dead_code)
- crates.io API (versions, dependencies, downloads); gsd-tools package-legitimacy (all OK)
### Secondary (MEDIUM)
- context7 /stranger6667/jsonschema: `validator_for`, `iter_errors`
- Rust Reference, path attribute
### Tertiary (LOW)
- Crate selection rationale, proc-macro2 fallback behaviour, symlink hop limit

## Metadata

**Confidence breakdown:**
- Standard stack: MEDIUM — registry facts verified, choices are recommendations
- Architecture: HIGH — built on code read this session and existing patterns
- Pitfalls: HIGH for 1-3 (measured), MEDIUM for the rest

**Research date:** 2026-11-18
**Valid until:** 2026-12-18 (gix and jsonschema release often)
