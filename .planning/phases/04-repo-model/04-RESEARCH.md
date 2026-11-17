# Phase 4: Repo model - Research

**Researched:** 2026-11-02 (gix era). **Revised:** 2026-11-17 for the git-binary pivot (D-23 to D-32), for the 04-13 replan.
**Domain:** the model's inputs read through the `git` binary (config, for-each-ref, cat-file) with gix kept for object work by id; the `.wsx.conf` grammar; one resolved repo model in the binary crate
**Confidence:** HIGH for the git behaviour the pivot rests on (spikes 001/002 plus probes run this session on git 2.56.1, each with a control). HIGH for the grammar (unchanged, 2026-11-02). MEDIUM for the proposed API shapes (no prototype of the Rust side was built).

> **How to read this file after the pivot.** Sections written on 2026-11-02 that describe gix reads carry a
> "Superseded by D-23" note under their heading and must not be implemented; the behaviour they pin (D-01 to D-21)
> stays the contract. The grammar (Patterns 5 and 7), the model's shape and error model (Pattern 8), and the
> Pitfalls not marked superseded are still valid. Everything the replan needs that is new is in
> **§ Git-binary pivot (D-23 to D-32)** and the rewritten **§ Validation Architecture**.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Config keys and sources
- **D-01:** wsx reads its `wsx.*` keys from local repo config only. A global or system value is
  ignored, so it can't reconfigure unrelated repos or make them "configured" for the machine-wide
  hook.
- **D-02:** The remote keys are `wsx.baseRemote` and `wsx.pushRemote`, resolved independently. Each
  defaults to `origin` only when unset. git's `remote.pushDefault` is not read: it is normally
  global, and it means "any `git push`", not "where cut PR branches go".
- **D-03:** A key naming a remote that doesn't exist is an error naming the key and the remote.
- **D-04:** With no key and no `origin` remote, the repo has no remote of that kind. This holds even
  if it has a remote under another name.
- **D-05:** `upstream` is never auto-detected. A fork layout sets `wsx.baseRemote=upstream`, which is
  how Success Criterion 2's "both default to `origin`" is read.

#### Integration-branch chain
- **D-06:** The chain reads the base remote's tracking refs only:
  `refs/remotes/<baseRemote>/develop`, then `.../next`, then the remote default from the local
  `refs/remotes/<baseRemote>/HEAD`. A stray local `develop` can't redirect the base.
- **D-07:** Local branches count only when there is no base remote (D-04): local `develop`, then
  local `next`.
- **D-08:** `wsx.baseRef` is a bare branch name, resolved on the base remote (or locally when there
  is no remote). `upstream/next` means a branch literally named that. If the ref is missing, it's an
  error naming the ref tried, with no fallback to the chain.
- **D-09:** If nothing in the chain exists, it's an error listing the refs tried and the fixes
  (`git remote set-head <remote> --auto`, or set `wsx.baseRef`). Resolution makes no network call and
  never guesses `main` or `master`.
- **D-10:** A stale `<remote>/HEAD` is accepted. The owner reconfirmed this after the correction
  under Specific Ideas: it only matters in a repo with no develop, no next and no `wsx.baseRef`, and
  that repo can set `wsx.baseRef`.

#### Manifest grammar (`.wsx.conf`)
- **D-11:** A line starting with `#` is a comment, and an empty line is ignored. There are no trailing
  comments. Any other unknown line is an error naming the line.
- **D-12:** An invalid manifest fails closed for every command including the push guard, so only
  ambiguity is invalid, never staleness. A cross-kind overlap is an error naming both lines: `only a`
  with `modified a`, or `only a` with `modified a/b` (either direction). Exact duplicates and nested
  lines of the same kind parse fine.
- **D-13:** A declaration already covered by a built-in must still parse, because Phase 5 reports it
  as a named `wsx check` failure (DIV-02).
- **D-14:** Paths are literal, with no glob matching, so `*` is an ordinary byte. One trailing `/` is
  stripped, as the oracle does. These are errors naming the line, since they could never match a git
  tree path: an empty path, `.`, a `.` or `..` component, a leading `/`, and `a//b`.

#### Workspace-branch fallback
- **D-15:** A fallback match names the branch, and the repo stays **not configured**. "Configured"
  means only that local config has `wsx.sourceBranch` (CONF-07). This resolves the tension between
  Success Criteria 3 and 5: the model reports "not configured" plus the inferred branch.
- **D-16:** In a not-configured repo, commands may use the inferred branch's manifest if it's
  readable. A missing or invalid manifest there reads as "not configured, run `wsx init`", not
  fail-closed. Failing closed (Success Criterion 5) applies only when `wsx.sourceBranch` is set. The
  guard exits 0 in a not-configured repo, so a stray `personal/foo` in an unrelated repo can't block
  pushes.
- **D-17:** Candidate selection takes the set of branches recorded by `wsx new` as an input, proven by
  a test that passes one in. Production passes an empty set until Phase 8. No `refs/wsx/` record
  format is defined here.
  - **Obligation on Phase 8:** supply the real set. Until then, CONF-05's exclusion is proven only at
    the input, not end to end.

### Claude's Discretion
Calls stated to the owner before they chose to move on. They can be challenged at plan review.
- **D-18:** An empty or repeated `wsx.*` key in local config is an error, not git's last-one-wins.
- **D-19:** Manifest line edges:
  - Everything after the keyword and one space is the path, so `only my dir` means `my dir`.
  - A path with leading or trailing whitespace must be C-quoted; unquoted, it's an error.
  - A line ending in CR is an error.
  - An empty or comments-only manifest is valid and leaves only the built-ins.
  - A `.wsx.conf` at the tip that isn't a blob (symlink, directory) is invalid.
- **D-20:** Phase 4 reads `wsx.worktreeDir` (local config, default `.worktrees/`, same path rules as
  D-14), because the built-in list needs it. The rest of CONF-06 stays in Phase 8.
- **D-21:** Fallback edges:
  - Zero or several candidates is an error listing them for commands that need a workspace branch.
    The guard still sees "not configured" and exits 0.
  - Candidates are local `refs/heads/workspace/*` and `refs/heads/personal/*` only. Remote-only
    branches don't count, and the checked-out branch gets no preference.
  - `wsx.sourceBranch` is a short branch name. If that branch is missing locally, wsx fails closed.

The module layout, the model's types and how errors extend `git::Error` are left to research and
planning.

### Replacement refs
The owner decided this at plan review, after the planner raised the refusal's cost.

- **D-22:** A chained or symbolic replacement ref anywhere in the repo stops the model in a configured
  repo, even among objects the manifest read never touches, and the error says the replacement ref is
  at fault rather than saying `.wsx.conf` cannot be read. A loud stop in a very rare case costs little,
  and a silent mismatch with git is the worse failure. A repo with no `wsx.sourceBranch` stays not
  configured (D-16).
  Amended 2026-11-16 by operator: when replace refs are in use at all, the model refuses, whatever
  their shape. In a configured repo, any entry under `refs/replace/` in the common dir's loose refs,
  any `packed-refs` record naming that base (found by a raw byte scan, not a parser), or a replace
  base other than `refs/replace/` stops the model before any object is read, also where
  `core.useReplaceRefs=false` or `GIT_NO_REPLACE_OBJECTS` turns replace refs off. Three review rounds
  of plan 04-12 each found a new way git and gix read replace refs differently, so refusing only the
  chained or symbolic ones above is replaced by refusing them all. The error blames the replace refs
  and names the remedy: remove them, or make the replaced history permanent with `git filter-repo`.
  D-16 is unchanged.

### Model inputs through the git binary
The operator decided the pivot on 2026-11-17, after round 2 of 04-13's Tier A review. The owner then
left D-24 to D-28 to Claude ("you decide on all of these").

- **D-23 (operator, 2026-11-17):** The model reads its inputs through the `git` binary with the
  existing spawn helper (`src/git/spawn.rs` `Git::run`): ref resolution (`wsx.sourceBranch`, the
  integration chain, `wsx.baseRef`), the workspace-candidate listing, the `.wsx.conf` blob at the tip
  with its type, the `wsx.*` keys, and the remote list. gix keeps object work by commit id (merge-base,
  walks, tree diffs) for later phases. This amends Phase 3's "reads through gix" (`03-CONTEXT.md`
  § Phase Boundary, and Established Patterns below) for these inputs only. Writes stay with the
  binary. Four review rounds each found another way gix's ref layer differs from git's, and CONF-07
  requires the bytes git reads, so these inputs match git by construction, not by refusing shapes one
  by one. `04-13-PLAN.md` as written (4e79ac6) is superseded and must not be executed.
- **D-24:** D-22 as amended on 2026-11-16 stays in force, unchanged. With git reading the manifest it
  no longer guards those bytes, but it still guards the gix object reads by id that later phases make,
  where gix and git can apply replacements differently. The spike checks whether today's detection
  misses a replace ref that either git or gix applies, including a linked worktree's own
  `refs/replace/` (CR-01 of `04-REVIEW.md`). A miss is a gap for the replan.
- **D-25:** Code whose only job is to make a gix ref or config read agree with git (the linked-worktree,
  symlink, alias and listing checks planned in 04-13, and any 04-11/04-12 code that serves only the
  manifest read) is deleted once its read moves to git, not kept as a backstop. Refusals that state a
  product rule stay: D-22's replace-ref stop, the refs-namespace refusal (04-05) and D-18. The replan
  lists each item and the side it falls on.
- **D-26:** One `resolve` spawns a small fixed number of git children whatever the repo's size: batched
  reads (one config read, one ref read covering the chain and the candidates, one object read for the
  blob and its type), never one child per ref. Provisional budget, Claude's call with no measurement
  behind it yet: at most 4 children, and `resolve` p95 under 100 ms on the four R3 repos as the spike
  measures it. A miss goes to the operator before the replan.
- **D-27:** The D-01 to D-21 behaviour tests are the contract and stay unchanged: they pass before and
  after the swap. Every shape from 04-13's two review rounds (replace refs, worktree-private stores,
  symlinked refs, `main-worktree/` aliases, a symlinked `HEAD` with `includeIf.onbranch`, dangling
  symrefs) becomes a regression test that asserts git's answer first and then that wsx gives the same
  one, with a control that changes git's answer and must change wsx's.
- **D-28:** Order: the spike (every D-27 shape with negative controls, D-26's child count and latency,
  and the open checks below), then the 04-13 replan under Tier A, since it moves a fail-closed
  boundary.

Open checks for the spike (unmeasured facts, not decisions):
- Whether `git config --local` gives D-01's meaning: it skips `include.*` by default and leaves out
  `config.worktree`; compare with what 04-05 does today in both cases.
- Whether gix honours `core.useReplaceRefs` and `GIT_NO_REPLACE_OBJECTS` for object reads.
- Whether gix reading `HEAD` at open (round 2, codex 2) still changes anything gix does for object
  reads once config comes from git.
- A fifo loose ref hangs both git and gix (round 2, pi): parity, but the spike reports whether the guard
  then hangs a push.

Spikes 001 (VALIDATED) and 002 (PARTIAL) answered these; see `.planning/spikes/`.

- **D-29 (operator, 2026-11-17):** `wsx.sourceBranch`, the integration rungs and the workspace
  candidates are all read through `git for-each-ref`, never by name (`rev-parse`, `cat-file
  --batch-check`). A fifo loose ref is skipped, so the guard never hangs a push. A ref git can reach
  only by name, such as one written as a filesystem symlink under the deprecated
  `core.preferSymlinkRefs`, reads as absent: a missing `wsx.sourceBranch` stays a loud error, and a
  symlinked `<remote>/HEAD` with no other rung gives D-09's error. Chosen over by-name reads (match
  `rev-parse` but hang on a fifo) and by-name reads with a timeout (one arbitrary limit).
- **D-30 (Claude's call, 2026-11-17):** A dangling symbolic rung is absent, as git reads it, and the
  chain moves on (spike 001 S7); a new test pins it. A symbolic rung that resolves is still
  `IntegrationRefSymbolic`, using for-each-ref's `%(symref)`.
- **D-31 (Claude's call, 2026-11-17):** D-26's "at most 4 children" counts one `resolve` aimed at
  wsx's own worktree, which spike 001 measured at 4. Aimed at another worktree, the spawn helper adds
  one `rev-parse --local-env-vars`, once per process; a test pins both counts.
- **D-32 (from spike 002, Claude's call):** D-22's detection also scans `<git_dir>/refs/replace` when
  the git dir is a linked worktree's (closes B2, and B3 with it). gix's divergence on `info/grafts` and
  `shallow` is outside Phase 4's inputs; it goes into the STATE obligations for the phases that use
  gix merge-base and walks.

### Deferred Ideas (OUT OF SCOPE)
- Exit codes and `--json` for every error here: Phase 5 (IF-01).
- The `refs/wsx/` record format and the real recorded set for D-17: Phase 8 (WTREE-05).
- The guard's behaviour on "not configured": Phase 6, reading the model's state.

Reviewed Todos (not folded):
- **Re-check Dependabot after first merge to main:** repo tooling, matched on keywords only.
- **Replace oracle-observed exit 128 under IF-01:** needs Phase 5's exit codes and is applied in
  Phases 7 and 9.
- **Re-verify Phases 1-3 before milestone audit** and **Shell-quote the path in ForeignOwner's printed
  fix command:** reviewed 2026-11-17, not folded; neither touches the pivot.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CONF-01 | `.wsx.conf` declares `only`/`modified`, raw-byte paths C-quoted as git does, proptest round trip, path-boundary matching, unknown kinds name the line | gix's `quote`/`undo` match git byte for byte, so no dependency. Pattern 5, Pitfalls 6-7. Prototype proptest passed and caught a planted defect |
| CONF-02 | Built-ins are only `.wsx.conf` and the worktree dir; one resolved list | `Model::is_personal` over built-ins plus `only` lines. `modified` is not personal (Pattern 7) |
| CONF-03 | Integration branch from `wsx.baseRef`, else `develop`, `next`, remote default | Exact full-name reads and the symbolic target of `<remote>/HEAD` (Pattern 3). Seven shapes of `<remote>/HEAD` measured |
| CONF-04 | Separate base and push remotes, both defaulting to `origin` | "Exists" is measured against `git remote add`, not `remote_names()` (Pattern 2, Pitfall 3) |
| CONF-05 | `wsx.sourceBranch`, else the only local `workspace/*` or `personal/*`; zero or several lists them | `prefixed` with a trailing slash plus a one-level filter (Pattern 4, Pitfall 4) |
| CONF-07 | Manifest from `<tip>:.wsx.conf` in the object database; configured means fail closed | Tree-entry read that tells blob from symlink, directory and submodule (Pattern 6). Working-tree edit and clean checkout measured |

**After the pivot** the "Research Support" column above describes the gix-era reads. The behaviour each row
names is unchanged; the reads behind CONF-03, CONF-04, CONF-05 and CONF-07 move to § Git-binary pivot (P1 config,
P2 refs, P3 objects), and CONF-01/CONF-02 (grammar, personal list) are untouched. The test map per requirement is
in § Validation Architecture.
</phase_requirements>

## Summary

**2026-11-17 (pivot):** the paragraphs below are the gix-era summary. After D-23, one `resolve` reads its inputs
with four `git` children (spike 001), and the 2026-11-17 probes found five things the replan must design for before
it can execute: the spawn helper gives every child a null stdin, which `cat-file --batch` cannot work with (a locked
Phase 3 rule; Open Question P1); `%(symref)` reports a chain's final target, so three D-06 tests and one candidate
test cannot keep their assertions (P2, P3); spike 001's `%(objecttype)` makes one broken ref fail the whole listing;
the default `cat-file` format reads a symlink manifest as a blob; and in a partial clone `cat-file` fetches over the
network unless `--no-lazy-fetch` is passed (measured). See § Git-binary pivot.

Every read the phase needs exists in gix 0.90.0 under the current feature set, so **the only new crate is `proptest`, as a dev-dependency**. C-quoting needs nothing new: `gix::quote::ansi_c::{quote, undo}` matched git's own output byte for byte on 270 real file names: every byte value that can appear in a name (1-255 except `/`), plus 16 edge names. A scratch copy of the repo, with a working prototype of the whole phase and 11 tests, passed clippy `-D warnings` and the pinned nextest. Those tests and the rest of the unit suite passed there (131 run). `cargo deny` stayed clean with proptest added and bit on a planted ban.

The gix defaults are the danger. Five natural calls give a wrong or silently weak answer here:
- `string_filter` returns the last value and reads a no-`=` key as absent.
- `Source::Local` includes `include.path` files.
- `remote_names()` counts global sections and keyless sections.
- `prefixed("refs/heads/workspace")` matches `workspace-x`.
- `rev_parse_single("<tip>:.wsx.conf")` and `find_blob` read a symlink as a blob.

Each has a measured replacement below.

The phase also has an integration problem nobody has listed yet. **The harness fixture writes `wsx.baseRef=origin/develop`, and D-08 reads that as a branch literally named `origin/develop`.** The fork fixture also sets no `wsx.baseRemote`. Phase 5's first enabled Rust scenarios would refuse on every fixture unless this is fixed. It is a small change, and it belongs in this phase, which defines the contract.

**Primary recommendation:** Add `src/git/{config,refs,quote}.rs` (reads, gix inside) and `src/model/{mod,manifest}.rs` (policy, no gix). Give `model::resolve` one signature that returns `RepoModel::Configured` or `RepoModel::NotConfigured { inferred }`. Use a separate hand-written `model::Error` with `From<git::Error>`. Add `proptest` with `default-features = false, features = ["std"]`, and `failure_persistence: None` in every property block.

### What this evidence does and does not establish

- **Scope of the measurements:** one host, git 2.56.0, gix 0.90.0. SHA-256 and bare repos were probed for the new reads, and hook-style `GIT_DIR` for config and listing. They were not run through the prototype's tests.
- **The prototype is a feasibility proof.** It has 11 tests, not the phase's test set. It settles that the APIs compile and answer as stated. It does not settle error wording, and it has no global-config test of its own (D-01's control was measured with the probe binary, against `git config`).
- **Not run this session:** `scripts/check.sh all` (the prototype ran clippy and unit tests only, on a `git archive` copy with no git repo), the harness suite against the changed fixture, and the container gate.
- **Proptest strength:** 2048 random cases per property in about 0.2 s. That is a good search for encoding defects and not a proof, and only when the generator can reach the defect: a uniform byte generator missed a bare edge space in 36 of 60 runs, so the generator is weighted (Code Examples). The all-bytes differential test is the stronger evidence for quoting.

## Git-binary pivot (D-23 to D-32)

Research for the 04-13 replan. Spikes 001 (VALIDATED) and 002 (PARTIAL) are the primary evidence and are cited,
not re-measured. Rows marked **probe** were measured on 2026-11-17 on git 2.56.1 in a scratch repo under
`target/gsd/` (removed after), each with a control; their reproductions are in "Probe reproductions" below. Code
facts were read this session from the files cited.

### What the replan builds, in one paragraph

`model::resolve` stops reading refs, listings, config and the manifest through gix. On the configured path in
wsx's own worktree it runs four `git` children through the spawn helper (spike 001, D-26, D-31): `git --version`
(once per helper), one config read, one `for-each-ref`, one `cat-file --batch`. The model's policy code (chain
order, candidate rule, D-18, routing in `require`) is rewired to those answers, not rewritten. gix keeps
`Repo::discover`, its refusals, `peel_commit` and `reads.rs` (object work by id for later phases). D-22's presence
refusal stays and also scans a linked worktree's private `refs/replace` (D-32).

### Facts the spikes did not cover (probe, 2026-11-17)

| # | Fact | Evidence | Consequence for the plan |
|---|------|----------|--------------------------|
| F1 | Every child of the spawn helper gets a null stdin, and that is a locked Phase 3 decision. `cat-file --batch` reads its requests from stdin, so spike 001's fourth child cannot run through `Git::run` as it is | [VERIFIED: src/git/spawn.rs:102 `.stdin(Stdio::null())`; 03-CONTEXT.md:59 "**Every child gets a null stdin and an explicit working directory**, whichever worktree it targets."] | Open Question P1 (operator) |
| F2 | `cat-file --batch='%(objectname) %(objecttype) %(objectsize) %(objectmode)'` on `<commit>:<path>` reports `blob 100644`, `blob 100755`, `blob 120000` (symlink), `tree 040000`; a request that is not `<tree-ish>:<path>` prints an empty mode. The default format has no mode, so a symlink's link text reads as a `blob` | probe | `%(objectmode)` is required for D-19's not-a-blob rule (Pitfall G2). Spike 001 used the default format and did not test this |
| F3 | An absent path, a present entry whose blob object is missing, and an unsorted root tree all print `<request> missing`, exit 0. `<commit>^{tree}` in the same batch still returns the root tree, which lists the entry in the last two cases | probe | Classify a `missing` path with the root tree (P3), or accept `ManifestMissing` for all three |
| F4 | `<id>^{commit}` peels an annotated and a nested tag to the commit; on a blob or a tree it prints `<request> missing` on stdout and `error: ... expected commit type, but the object dereferences to blob type` on stderr, exit 0 | probe | Tips are peeled inside the batch; a rung holding a blob is still an error (`integration_ref_holding_a_blob_fails_closed` expects `Error::Git(_)`) |
| F5 | In a `--filter=blob:none` partial clone, plain `cat-file --batch` **fetched the missing blob from the remote** (control). `git --no-lazy-fetch cat-file` and `GIT_NO_LAZY_FETCH=1` each printed `missing` and fetched nothing | probe | D-09 ("Resolution makes no network call") needs `--no-lazy-fetch` on every child that reads objects. New with the pivot: gix never fetches |
| F6 | `for-each-ref --format='%(refname) %(objecttype)'` exits 128 (`fatal: missing object <id> for refs/heads/main`) when one listed ref names a missing object. With `%(refname)%00%(objectname)%00%(symref)` it lists the ref, exit 0 | probe | Drop spike 001's `%(objecttype)`: otherwise one broken ref anywhere under the patterns fails every `resolve`. Types come from `cat-file` |
| F7 | `%(symref)` prints the **final** target of a chain: for HEAD→a→b→x it prints `refs/remotes/origin/x` for HEAD, a and b. A chain of 5 refs (4 hops) is listed; one of 6 refs, and a cycle, are omitted with no message. `git help for-each-ref`: "The ref which the given symbolic ref refers to" | probe | D-30's `IntegrationRefSymbolic` names the final ref; the HEAD rung sees only its final target. Open Question P2 |
| F8 | Patterns: `refs/remotes/origin/develop` does not list `refs/remotes/origin/developer`; `refs/heads/workspace/` lists nested `refs/heads/workspace/a/b`; `refs/remotes/a*/develop` lists `refs/remotes/ab/develop` (the pattern is a glob) | probe | Filter every record by exact bytes after the listing (Pitfall G1). A remote name holding `*`, `?` or `[` otherwise reads another remote's refs |
| F9 | A loose ref holding `garbage` and a ref with a bad name are skipped with `warning: ignoring broken ref …` / `warning: ignoring ref with broken name …` on stderr, exit 0, also with `GIT_REF_PARANOIA=1`. A dangling symref, a fifo and a `refs/…` symlink are skipped with empty stderr | probe | `corrupt_candidate_ref_stays_in_inferred` cannot keep its assertion if the listing follows git (Open Question P3) |
| F10 | `config -z --show-scope --show-origin --get-regexp` prints records `scope NUL origin NUL key [LF value] NUL`: a bare key has no LF, `key =` is `key LF`; section and variable are lowercased, the subsection keeps its case (`wsx.sub.sourcebranch`, `remote.a.b.url`). The origin is `file:.git/config` from the main worktree (relative to the child's cwd), an absolute path from a linked worktree, `file:config` in a bare repo; an `include.path` file is scope `local` with its own origin. No match: exit 1 and no output | probe; spike 001 C1-C3 | Parser rules in P1; exit 1 means "no keys", not a failure (Pitfall G5) |
| F11 | A full 40-hex request is read as that object even when `refs/heads/<the same hex>` exists | probe | Feeding `for-each-ref`'s ids to `cat-file` cannot be redirected by a ref name |

### Read flow

```
resolve(repo, git, recorded)
  │
  ├─ child 1  git --version                     (once per Git helper; spawn.rs ensure_version)
  ├─ child 2  git config -z --show-scope --show-origin --get-regexp '^(wsx|remote)\.'
  │             parse ─► wsx keys: scope=local AND origin == <common_dir>/config   (D-01)
  │                   ─► remotes:  scope=local|worktree, any origin                 (A2: as git remote add)
  │           single(SOURCE_BRANCH) ─► Some(branch) = configured | None = not configured   (D-15, D-18)
  │           base/push remote (D-02..D-05), baseRef value, worktreeDir value: decided now, errors raised later
  │
  ├─ child 3  git --no-lazy-fetch for-each-ref --format=%(refname)%00%(objectname)%00%(symref) <patterns>
  │             configured:     refs/heads/<branch>                              + rung patterns
  │             not configured: refs/heads/workspace/  refs/heads/personal/      + rung patterns
  │             rung patterns:  <prefix>develop <prefix>next [<prefix>HEAD] | <prefix><baseRef>
  │             exact-byte filter ─► branch present? (WorkspaceBranchMissing) ─► candidates (one level, less recorded)
  │
  ├─ D-22 presence refusal (filesystem, no child): common refs/replace, packed-refs, and
  │             <git_dir>/refs/replace when git_dir != common_dir (D-32)   ── before any object is read
  │
  ├─ child 4  git --no-lazy-fetch cat-file --batch='%(objectname) %(objecttype) %(objectsize) %(objectmode)'
  │             stdin: <ws id>^{commit}  <ws id>^{tree}  <ws id>:.wsx.conf  <rung id>^{commit}
  │             ─► workspace_tip, integration tip, manifest bytes + mode, root tree (only to classify "missing")
  │
  └─ policy, in today's error order ─► RepoModel::Configured | NotConfigured { inferred }
```

With zero or several candidates, child 4 is not spawned (3 children, spike 001). Aimed at a worktree that is not
the helper's own, the helper first runs `git rev-parse --local-env-vars` once (D-31) [VERIFIED: src/git/spawn.rs
`removal_list`, read this session]. A bare repo has no own worktree, so every child there counts as "another
worktree" and pays that fifth child [VERIFIED: src/git/spawn.rs `from_process`: "`Some(worktree) => git.with_own_worktree(...)`, `None => git`"].

### P1: the config read (D-01, D-02 to D-05, D-18, D-20)

**Command:** `git config -z --show-scope --show-origin --get-regexp '^(wsx|remote)\.'` (spike 001 child 2).

**Parse, as bytes** (F10): split stdout on NUL into triples (scope, origin, entry); split entry at the first LF
into key and value; no LF means a bare key, which today's read returns as one empty value
(`source_branch_without_a_value_fails_closed` pins `KeyEmpty`), so map it to an empty value. A trailing record that
is not a full triple is an output error, never a shorter result.

- **wsx keys (D-01):** keep a record only when scope is `local` **and** the origin, with its `file:` prefix removed
  and resolved against the child's cwd, is the common dir's `config`. Compare canonical paths (`Repo::common_dir`
  is canonical). This drops `include.path` files (scope `local`, other origin), `config.worktree` (scope
  `worktree`), `GIT_CONFIG_PARAMETERS`/`-c` (scope `command`), global and system: spike 001 C1 measured that this
  filter equals `git config --local --get-all` in all three cases and from a linked worktree. Compare the key
  lowercased against the lowercased constant (`wsx.sourcebranch`); a three-part key (`wsx.sub.sourcebranch`) never
  matches. Keep every value in file order, so `single` (D-18) keeps returning `KeyRepeated` with all values.
- **Remotes (A2, D-03, D-04):** a remote is defined when some key `remote.<name>.<var>` has scope `local` or
  `worktree`, from any origin, includes too. This is today's `remote_configured` rule, which spike 001 C3 measured
  against `git remote add` (global-only: free; keyless local section: free, since it prints no record; included:
  defined). `<name>` is everything between `remote.` and the **last** dot (remote names may hold dots and
  slashes); a two-part key such as `remote.pushdefault` names no remote (`push_default_is_not_read`).
- **Exit status:** 1 with empty stdout is "no `wsx.*` and no `remote.*` key" (F10). `Git::run` maps any non-zero
  exit to `Error::GitFailed` [VERIFIED: src/git/spawn.rs `finish`], so the caller matches `status: Some(1)` with
  empty stdout and treats it as empty; any other failure (a malformed config file is exit 3 or 128 [ASSUMED]) stays
  `GitFailed`.

Proposed shape (names are the planner's): `Repo::read_config(&self, git: &Git) -> Result<ConfigRead, Error>` with
a pure `parse_config_z(stdout, cwd, own_config) -> Result<ConfigRead, Error>` unit-tested on byte fixtures, and
`ConfigRead::values(key) -> &[Vec<u8>]` / `ConfigRead::remote_defined(name) -> bool` feeding today's `single`,
`remote` and `default_remote` unchanged.

### P2: the ref read (D-06 to D-10, D-17, D-21, D-29, D-30)

**Command:** `git --no-lazy-fetch for-each-ref --format=%(refname)%00%(objectname)%00%(symref) <patterns…>`.
Records end in LF; a refname holds no NUL or LF (git skips bad names, F9), so split on LF then on NUL. Do not
use `%(objecttype)` (F6).

**Patterns, decided from P1:**
- configured: `refs/heads/<wsx.sourceBranch>` (already checked by `is_branch_name`, which refuses `*`, `?`, `[`);
- not configured: the two `CANDIDATE_PREFIXES` [VERIFIED: src/model/mod.rs:386 `[b"refs/heads/workspace/", b"refs/heads/personal/"]`];
- rungs, with prefix `refs/remotes/<base>/` or `refs/heads/` (D-07): `<prefix><wsx.baseRef>` alone when the key is
  set (D-08), else `<prefix>develop`, `<prefix>next` [VERIFIED: src/model/mod.rs:529 `CHAIN: [&[u8]; 2] = [b"develop", b"next"]`]
  and, with a base remote, `<prefix>HEAD`.
- Validate each built rung name as a full ref name before listing (the gix `FullName` check is a pure function,
  not a read), so an invalid remote name stays an error as it is today rather than reading as absent.

**Then filter by exact bytes** (F8, Pitfall G1): a branch or rung is present only when a record's name equals the
expected full name; a candidate is a record whose name is one of the two prefixes plus a rest with no `/`
(`nested_branch_is_not_a_candidate`, `name_extension_is_not_a_candidate`). Sort candidates by bytes across both
prefixes, then drop `recorded` (D-17), as `candidates` does now.

**What a record means** (spike 001 S2-S8, F7, F9):
- No record: absent. That covers a missing ref, a dangling symref (S6, S7; D-30), a fifo (S8; D-29), a
  `refs/…`-target filesystem symlink (S3; D-29), a `main-worktree/` alias (S4), a ref only in a linked worktree's
  private store (S2, WR-01), a corrupt or badly named ref (F9), and a chain deeper than 5 refs or a cycle (F7).
- `%(symref)` empty: a direct ref; `%(objectname)` is its value (maybe a tag; peeled in P3).
- `%(symref)` set on the workspace branch: accepted, with the final object as its tip, as `ref_commit` follows
  symrefs today (04-13 planned `textual_symbolic_source_branch_resolves`).
- `%(symref)` set on develop, next or the `baseRef` rung: `IntegrationRefSymbolic { full_ref, target }` (D-30),
  with `target` the **final** ref (F7). The single-hop tests (`symbolic_develop_is_refused`,
  `symbolic_next_is_refused`, `symbolic_local_develop_is_refused`, `symbolic_base_ref_is_refused`) keep their
  assertions, since final and immediate coincide.
- The `<base>/HEAD` rung: a default only when `%(symref)` starts with the prefix; the branch is the rest and the tip
  is HEAD's own `%(objectname)` (the final target's). An object-id HEAD (`object_remote_head_is_not_a_default`),
  a target outside the remote (`remote_head_outside_the_remote_is_not_a_default`) and a dangling HEAD
  (`dangling_remote_head_is_not_a_default`) stay in `tried`. Multi-hop HEAD chains cannot be told from one hop
  (F7): Open Question P2.

The listing is one child whatever the number of refs (D-26). It reads no object, so it may run before D-22's
refusal; `--no-lazy-fetch` is on it only because it costs nothing.

### P3: the object read (CONF-07, D-16, D-19, D-23)

**Command:** `git --no-lazy-fetch cat-file --batch='%(objectname) %(objecttype) %(objectsize) %(objectmode)'`, its
requests written to stdin and stdin then closed (needs Open Question P1). Requests, one per line, in this order,
all built from ids `for-each-ref` printed (F11):

1. `<ws id>^{commit}` → `workspace_tip`; `missing` means the tip commit is unreadable (`missing_tip_commit_fails_closed` keeps `ManifestUnreadable`).
2. `<ws id>^{tree}` → the root tree's raw bytes, read **only** to classify a `missing` answer to request 3.
3. `<ws id>:.wsx.conf` → type, size, mode and bytes. `<tag>:path` peels through the tag to the same commit as request 1 (probe).
4. `<rung id>^{commit}` → the integration tip; `missing` is a git-layer error (F4), preserving `Error::Git(_)` for a rung holding a blob.

**Output parse:** for each request in order, read a header up to LF. A header ending in ` missing` is that
request's "missing" (parse positionally; the echoed request text is not parsed back). Otherwise split it into
four fields (the mode may be empty), read exactly `objectsize` bytes, then one LF. Any other shape
(`ambiguous`, short read, bad size) is an output error. Ids are lowercase hex of length 40 or 64; build
`ObjectId` through a new hex-checking constructor (today's `ObjectId::from_gix` is private to gix ids
[VERIFIED: src/git/mod.rs:49-52]).

**Classifying `.wsx.conf` (request 3):**

| cat-file answer | root tree (request 2) | Model result | Today's test it keeps |
|---|---|---|---|
| `blob`, mode `100644` or `100755` | — | parse the bytes (D-11 to D-14, 64 KiB limit) | `configured_repo_resolves_from_the_object_database`, `executable_manifest_resolves` |
| `blob`, mode `120000` | — | `ManifestNotABlob { kind: "symlink" }` | `symlink_manifest_fails_closed` |
| `tree`, mode `040000` | — | `ManifestNotABlob { kind: "directory" }` | `directory_manifest_fails_closed` |
| mode `160000`, or `missing` with a `160000` entry in the tree | gitlink entry | `ManifestNotABlob { kind: "submodule" }` | `submodule_manifest_fails_closed` (that a gitlink whose commit is not in this repo reads as `missing` is [ASSUMED]; the test settles it) |
| `missing` | no entry named `.wsx.conf` | `ManifestMissing` | `missing_manifest_fails_closed` |
| `missing` | entry present (blob object gone, a partial clone, or an unsorted tree git's lookup misses) | `ManifestUnreadable` with a new git-layer source | `missing_blob_object_fails_closed`; `unsorted_tree_manifest_fails_closed` keeps the outer variant but not its inner `TreeNotSorted` |

The root-tree scan reads entries as `<ascii mode> SP <name> NUL <raw id>`, the id `hex length / 2` bytes long. It
only picks an error variant, never bytes git did not return. Dropping it is simpler and makes the last two rows
`ManifestMissing` (both still fail closed when configured, and both read as not configured otherwise); that
changes two more test assertions and tells a partial-clone user "no `.wsx.conf`", which is wrong. Recommendation:
keep the scan.

### P4: the spawn helper (D-23, D-26, D-31)

- `Git::run` is the only way children start [VERIFIED: src/git/spawn.rs:276 `pub fn run<I, S>(&self, worktree: &Path, args: I) -> Result<Vec<u8>, Error>`].
  It returns stdout only and drops stderr on success, which is why F9's warnings are invisible to callers.
- `resolve` needs a `&Git`: add it to the signature (`resolve(repo, git, recorded)`), so tests can hand in a
  helper whose PATH holds a counting wrapper (D-31) and production uses `Git::from_process(repo)`.
- The worktree argument is `repo.worktree()`, or the git dir for a bare repo (`bare_repo_resolves_the_same_model`
  must still pass).
- For P3, either a sibling of `run` that pipes a fixed byte buffer and closes it (Open Question P1, option A) or
  two null-stdin children (option B). Under A: write the whole request buffer (under 1 KiB, far below a pipe
  buffer, so no deadlock) and drop stdin before `wait_with_output`; keep the version check and the env removal
  exactly as `run` has them. `children_get_null_stdin` stays for `run`.

### P5: replace-ref detection (D-22, D-24, D-32)

`Repo::refuse_replace_refs` stays, in today's order: `GIT_REPLACE_REF_BASE`, gix's
`gitoxide.objects.replaceRefBase`, the common dir's `refs/replace`, then `packed-refs` [VERIFIED:
src/git/refs.rs:337-366]. D-32 adds one store: when `git_dir() != common_dir()`, an entry in
`<git_dir>/refs/replace` is `ReplaceRefsPresent { path, packed: false }`, read with `holds_an_entry` so a fifo or
unreadable entry there is `ReplaceRefsUnreadable` (04-REVIEW CR-01's fix). Spike 002 B2 measured gix applying such
a ref and git ignoring it, and B2b measured both ignoring the private dir's `packed-refs`, so no packed scan is
added there. It runs before child 4, at today's place in `build` (after the `WorkspaceBranchMissing` check), in
both states, and `require`'s routing is unchanged.

With git reading the manifest, D-22 no longer protects those bytes; git applies or ignores replacements itself
(spike 001 S1). It is kept as a product rule (D-24) for the gix reads by id later phases make. Fix the two docs the
review named while touching this code: the "Only the common dir is read" paragraph of `refuse_replace_refs`
(CR-01) and `resolve`'s "a later check that lists replace refs" (IN-01). IN-02 (`ReplaceBaseSet`'s remedy per
setting) is independent and small.

Spike 002's other findings stay out of Phase 4: B3 (an `onbranch` include that turns replace refs off) only bites
while a replace ref exists, which D-32 now refuses; B4/B4b (grafts, shallow) go into STATE's obligations for the
phases that call `merge_base` and walks (D-32).

### D-25 deletion inventory

Read from `src/git/{refs,config,mod,repo}.rs` and `src/model/mod.rs` this session. "Parity" means its only job is
to make a gix read agree with git; "product rule" means it states a refusal wsx wants whatever reads the bytes.

| Item | Where | Side | Disposition |
|------|-------|------|-------------|
| `Repo::find_ref` and its `found_name != full` check (gix's extra loose paths `.git/heads/<full>` …) | refs.rs:123 | parity | Delete. Its tests `find_ref_tells_direct_from_symbolic`, `find_ref_refuses_a_ref_read_under_another_name`, `full_name_read_is_exact`, `tag_does_not_satisfy_a_branch_name` go; the model test `ref_under_another_path_does_not_satisfy_a_rung` stays and passes by construction. `repo.rs`'s `without_a_namespace_refs_read_as_git_reads` calls it (repo.rs:3278) and must be re-pointed |
| `Repo::ref_commit`, `SYMREF_MAX_DEPTH = 5` | refs.rs:166, :47 | parity (mirrors `rev-parse`'s depth; `for-each-ref` applies the same limit, F7) | Delete with tests `ref_commit_matches_show_ref`, `ref_commit_reads_the_commit_a_describe_shaped_ref_holds`, `ref_commit_follows_symbolic_refs_as_far_as_git_does` |
| `Repo::refs_directly_under` | refs.rs:267 | parity | Delete. Port the behaviour its tests pin to the new listing: sorted, non-UTF-8 by bytes, packed and loose, linked worktree (`listing_is_sorted`, `non_utf8_branch_is_listed_by_its_bytes`, `listing_survives_packing`, `linked_worktree_lists_the_same_branches`, `refs_directly_under_matches_for_each_ref`); `corrupt_ref_under_the_prefix_is_an_error` waits on Open Question P3 |
| `Repo::root_entry`, `Error::TreeNotSorted` and its message test | refs.rs:216; mod.rs:146; `tree_not_sorted_message_names_the_tree_and_the_fix` | parity (the tree-order check made gix's scan agree with git's lookup) | Delete. Port `root_entry_reads_*`, `root_entry_reports_*`, `root_entry_is_absent_without_the_file`, `root_entry_ignores_a_nested_file` and `manifest_read_stays_on_the_commit_it_was_given` to P3; delete `root_entry_refuses_a_tree_out_of_git_order`, `root_entry_reads_a_tree_in_git_order`, `root_entry_reads_a_tree_git_wrote_where_name_order_differs` |
| `RootEntry`, `RefTarget` (types, re-exported from `git`) | refs.rs:24, :33 | neither: shapes | Keep or rename as P2/P3's result types; planner's call |
| `Repo::local_config_values` | config.rs:29 | parity | Delete; its 13 behaviour tests (`included_key_is_not_read` … `subsection_key_is_not_read`) are ported to `parse_config_z` plus a repo-backed differential each, keeping their names where possible |
| `Repo::remote_configured` | config.rs:59 | parity | Delete; port `global_remote_section_is_not_a_remote` … `undefined_remote_is_not_a_remote` the same way |
| 04-13's planned linked-worktree, symlink, alias and listing checks (`refuse_linked_worktree_refs`, `refuse_symlinked_ref`, `refuse_symlinked_entries`, `is_per_worktree_ref_dir`, `SymlinkedRef` …) | 04-13-PLAN.md only, never built | parity | Not built (D-25) |
| `lossy` | refs.rs:80 | helper | Delete if orphaned by the above |
| `Repo::refuse_replace_refs`, `refuse_replace_base_env`, `holds_an_entry`, `packed_refs_name_the_base`, `ReplaceRefsPresent`, `ReplaceBaseSet`, `ReplaceRefsUnreadable`, `is_replacement_refusal` | refs.rs:337-410; mod.rs:151-176 | product rule (D-22, D-24) | Stay; extended by D-32 |
| `refuse_refs_namespace`, `Error::RefsNamespace` | repo.rs:576; mod.rs:127 | product rule (D-25 names it) | Stay. Its doc ("which git's own reads ignore, so wsx would read every ref … from inside the namespace") describes the gix model reads that leave; reword to the gix reads that remain |
| `model::single`, `KeyRepeated`, `KeyEmpty` | model/mod.rs:414 | product rule (D-18) | Stay; fed by P1 |
| `is_branch_name` | refs.rs:71 | product rule (03.1 D-04; `NotABranchName`) | Stay; also keeps glob bytes out of P2's patterns |
| `peel_commit`, `peeled_commit` | refs.rs:88, :96 | object work by id | Stay: `reads.rs:190` calls `peel_commit`. The model stops calling it (P3 peels) |
| `refuse_ref_storage`, `ref_storage`, `Error::Reftable` and the other `discover` refusals | repo.rs | Phase 3 rules, outside D-25 | Stay. Changing them would reverse Phase 3 decisions, which needs the operator |
| `IntegrationRefSymbolic`, every other `model::Error` variant, `require` | model/mod.rs:135-176, :114 | product rules | Stay (D-30 keeps `IntegrationRefSymbolic`) |

### D-27 regression shapes

Each test asserts git's answer first, then that wsx gives the same one, then applies a control that moves git's
answer and must move wsx's. Source: round 2 SUMMARY, 04-REVIEW, spike 001/002. Proposed names are in
`model::tests`.

| Shape (source) | git's answer, asserted first | wsx equals | Control that moves git's answer | Proposed test |
|---|---|---|---|---|
| Replace ref in the common dir (round 1; S1) | `git cat-file blob <ws>:.wsx.conf` prints the replaced bytes | refuses (`ReplaceRefsPresent`), D-22 | `git replace -d`: git prints the original bytes, wsx resolves them | existing `single_replacement_fails_closed` … `replace_dir_left_empty_resolves` |
| Replace ref only in `.git/worktrees/<id>/refs/replace`, read from that worktree (CR-01; S1b; B2) | git ignores it: prints `only .planning` | refuses naming `<git_dir>/refs/replace` (D-32 over-refuses, the product rule) | remove the entry: git unchanged, wsx resolves `only .planning`; and the same entry read from the main worktree resolves | `private_replace_ref_in_a_linked_worktree_fails_closed`, `common_replace_ref_from_a_linked_worktree_fails_closed` |
| Branch only in a linked worktree's private `refs/heads/workspace/` (WR-01; S2) | `git for-each-ref 'refs/heads/workspace/*'` from the worktree lists only `workspace/dev` | `inferred` is `workspace/dev` | `git branch workspace/shared`: both list two | `private_branch_in_a_linked_worktree_is_not_a_candidate` |
| Relative filesystem symlink as the workspace branch (round 1 codex 1; S3) | `git rev-parse --verify` and `for-each-ref` give the target's id | same tip | retarget the link: both move | `relative_symlinked_source_branch_resolves_as_git_does` |
| `refs/…`-target symlink (`core.preferSymlinkRefs`) as `wsx.sourceBranch` (S3; D-29) | `git branch --list` / `for-each-ref` list nothing (assert `rev-parse` resolves it too, to pin the split) | `WorkspaceBranchMissing` | rewrite it as a text symref with `git symbolic-ref`: listed, wsx resolves | `git_style_symlinked_source_branch_reads_as_missing` |
| Same, as `<remote>/HEAD` with no develop or next (D-29) | `for-each-ref` omits HEAD | `NoIntegrationBranch` listing HEAD in `tried` (D-09) | `git remote set-head origin main` (text symref): default found | `symlinked_remote_head_gives_no_integration_branch` |
| `main-worktree/` alias symref as the workspace branch, from a linked worktree (round 2 codex 1; S4) | `git rev-parse --verify 'refs/heads/workspace/dev^{commit}'` fails; `for-each-ref` omits it | `WorkspaceBranchMissing` | make it a plain symref to `refs/heads/keep`: both resolve the same tip | `main_worktree_alias_source_branch_reads_as_missing` |
| Symlinked `HEAD` + `includeIf.onbranch` defining `origin` (round 2 codex 2; S5) | `git remote` lists `origin` | base remote `origin` | point `HEAD` elsewhere: `git remote` lists nothing, wsx has no base remote | `onbranch_remote_under_a_symlinked_head_counts_as_git_does` |
| Dangling text symref under `refs/heads/workspace/` (round 2 codex 3; S6) | `git branch --list 'workspace/*'` omits it | not a candidate | create its target: both list it | `dangling_symref_is_not_a_candidate` |
| Dangling symbolic `develop` rung (S7; D-30) | `git rev-parse --verify` fails, `for-each-ref` omits it | chain moves on to `next` | create the target: rung present and symbolic → `IntegrationRefSymbolic` | `dangling_symbolic_rung_is_absent` |
| Fifo loose ref under `refs/heads/workspace/` (round 2 pi; S8; D-29) | `for-each-ref` exits 0 and skips it | resolves (`inferred` = the other candidate), within a bound | the fifo as `wsx.sourceBranch`: `WorkspaceBranchMissing`, still bounded | `fifo_ref_is_skipped_without_hanging` (bound it: Pitfall G7) |
| `include.path`, `config.worktree`, `GIT_CONFIG_PARAMETERS` `wsx.sourceBranch` (C1; D-01) | `git config --local --get-all` exits 1 | not configured | `git config --add wsx.sourceBranch dev` in the own file: both configured | existing `included_source_branch_does_not_configure`, `global_source_branch_does_not_configure` + ported `worktree_config_key_is_not_read`, `command_line_config_key_is_not_read` |
| Bare, empty, repeated keys (C2; D-18) | `git config --local --get-all` prints them | `KeyEmpty` / `KeyRepeated` | remove the extra line: single value | existing `repeated_source_branch_fails_closed`, `empty_source_branch_fails_closed`, `source_branch_without_a_value_fails_closed` |
| Global-only, keyless local, included remote section (C3; A2) | `git remote add <name>` succeeds / succeeds / exits 3 | free / free / defined | add a key to the keyless section: both defined | ported `global_remote_section_is_not_a_remote`, `keyless_local_remote_section_is_not_a_remote`, `included_remote_is_a_remote` |
| Remote name with a glob byte, `[remote "o*"]` with a `refs/remotes/ox/develop` (F8) | `git for-each-ref refs/remotes/o*/develop` lists `ox`'s ref; `git rev-parse --verify 'refs/remotes/o*/develop'` fails | `refs/remotes/o*/develop` is absent | none needed beyond the listing assertion; the exact filter is the subject | `glob_bytes_in_a_remote_name_match_only_themselves` |
| Partial clone, manifest blob not local (F5) | `git --no-lazy-fetch cat-file -e <ws>:.wsx.conf` fails; plain `git cat-file -p` fetches it (the control, run last) | `ManifestUnreadable`, and the blob is still absent after `resolve` | the control fetch: wsx then resolves the bytes | `partial_clone_manifest_is_unreadable_without_a_fetch` |

### Contract tests that cannot keep their assertion

D-27 says the D-01 to D-21 tests stay unchanged. These seven assert a gix-era answer that git does not give, so
reading through git (D-23, D-29) changes them; each still fails closed or stays not configured unless noted.

| Test | Asserts today | Through git | Why |
|---|---|---|---|
| `remote_head_chain_into_a_local_branch_is_refused` | `IntegrationRefSymbolic { alias, refs/heads/workspace/dev }` | `NoIntegrationBranch` (HEAD's final target is outside the remote) | F7 |
| `remote_head_chain_inside_the_remote_is_refused` | `IntegrationRefSymbolic { alias, refs/remotes/origin/main }` | **resolves** `main` (final target inside the remote) | F7; the only row that stops refusing |
| `remote_head_cycle_is_refused` | `IntegrationRefSymbolic { alias, HEAD }` | `NoIntegrationBranch` (cycle omitted) | F7 |
| `corrupt_candidate_ref_stays_in_inferred` | `Err(Git(Gix { what: "list the refs directly under refs/heads/workspace/" }))` | infers `workspace/dev` (git skips the corrupt ref) | F9 |
| `unsorted_tree_manifest_fails_closed` | `ManifestUnreadable { source: TreeNotSorted { .. } }` | `ManifestUnreadable` with the new source (with P3's scan) | `TreeNotSorted` deleted |
| `git::refs::tests::corrupt_ref_under_the_prefix_is_an_error` | listing error | git skips it | F9 |
| `refs_namespace` test `without_a_namespace_refs_read_as_git_reads` | through `find_ref` | re-pointed | `find_ref` deleted |

### Pitfalls specific to the pivot

- **G1, prefix and glob matching in `for-each-ref` patterns.** A pattern matches by glob or as a prefix up to a
  slash (F8), so the listing is a superset. Every decision reads the exact name from the record; nothing trusts
  that a record matched a pattern.
- **G2, the default `cat-file` format hides the mode.** A symlink is `blob` (F2). Use `%(objectmode)`; a test
  asserting `ManifestNotABlob { kind: "symlink" }` already exists and goes RED if the mode is dropped.
- **G3, lazy fetch.** Without `--no-lazy-fetch` a partial clone turns `resolve` into a network call (F5) and makes
  CONF-07's "missing object fails closed" depend on the network. Pass it as the global option before the
  subcommand (`git --no-lazy-fetch cat-file …`).
- **G4, origin paths.** `--show-origin` prints a path relative to the child's cwd in the main worktree and absolute
  in a linked worktree (F10). Canonicalise both sides before comparing; a repo path with a space or a non-UTF-8 byte
  was not probed, so add one fixture with such a path (unquoted with `-z` is [ASSUMED]).
- **G5, `git config --get-regexp` exits 1 on no match.** Read as "no keys", not `GitFailed`, but only with empty
  stdout (F10).
- **G6, error precedence.** Today's order is: `wsx.sourceBranch` key errors, `NotABranchName`,
  `WorkspaceBranchMissing`, replace refusal, base remote, push remote, `wsx.baseRef` and the chain (rung peel
  included), `wsx.worktreeDir`, tip, manifest [VERIFIED: src/model/mod.rs `build` and `integration`, read this
  session]. The batched reads gather everything first; raise the errors afterwards in that order, so a repo with two
  faults reports the same one as today.
- **G7, a fifo test must not hang the suite.** `.config/nextest.toml` sets no `slow-timeout` [VERIFIED: read this
  session, only a junit path and one `success-output` override], so a regression that reads the fifo by name would
  hang forever. Run that resolve in a child (`testing::reexec`) with a kill after a few seconds and assert it
  finished, so the failing direction prints an assertion, not a hang.
- **G8, `Git::run` drops stderr on success.** Warnings (F9) and `cat-file`'s `error:` lines (F4) never reach the
  caller. Decide from stdout and the exit code only, or add a variant that returns stderr if Open Question P3 goes
  that way.
- **G9, locale.** The parsed tokens (`missing`, field order) are machine formats and the user's locale is passed
  through; that `missing` is never translated is [ASSUMED].
- **G10, a huge `.wsx.conf`.** The whole blob comes back through stdout before the 64 KiB check, as `find_blob`
  read it whole before the pivot. Unchanged risk, accepted at ASVS L1 (Security Domain).
- **G11, the child count has no slack** (spike 001). A bare repo, or `resolve` aimed at another worktree, is five
  (D-31). A sixth child anywhere on the configured path is a D-26 miss and goes to the operator.

### Open questions for the replan (RESOLVED 2026-11-17)

Resolved in 04-CONTEXT.md: P1 by D-33 (operator); P2 by D-34 and P3 by D-35, both Claude's calls because they amend
D-27, which the operator delegated ("you decide on all of these"); the operator was told of both and can reverse them.
The original framing is kept below for the record.

1. **P1, stdin for `cat-file --batch`.** Phase 3 locked "Every child gets a null stdin" (03-CONTEXT.md:59), and spike
   001's fourth child needs its requests on stdin.
   - **Option A (recommended):** one `Git` method that writes a fixed buffer to stdin and closes it. The rule's
     purpose, that git can never wait for input, holds because stdin reaches EOF at once. Four children, as spike
     001 measured. Amends a Phase 3 decision, so it needs the operator's word.
   - **Option B:** keep null stdin and use two children, `git ls-tree -z --format=...` for mode and id, then `git
     cat-file blob <id>`, plus a second peel for annotated-tag tips. Five children on the configured path: a D-26
     miss, which also goes to the operator. `ls-tree` looks entries up differently from `<tip>:path` on unsorted
     trees, a split inside git.
2. **P2, multi-hop `<remote>/HEAD` chains (F7).** Under D-29, `for-each-ref` shows only the final target, so three
   D-06 tests change (table above), and `HEAD -> alias -> <remote>/main` stops being refused.
   - **Recommended:** accept git's final-target reading and rewrite the three tests to it. D-06's aim, never
     landing on a local commit, still holds: a final target outside the remote is not a default.
   - **Alternative:** read each symbolic rung's immediate target by name with `git symbolic-ref --no-recurse`, which
     breaks D-29 (by-name reads) and adds a child.
3. **P3, corrupt and badly named refs (F9).** git skips them with a warning. Following git, `corrupt_candidate_ref_stays_in_inferred` infers the remaining candidate.
   - **Recommended:** follow git; D-29 already reads every unlistable ref as absent, and a configured repo still
     fails closed (`WorkspaceBranchMissing`) when the corrupt ref is `wsx.sourceBranch`.
   - **Alternative:** refuse when `for-each-ref`'s stderr is non-empty. That needs a stderr-returning `Git` method
     (G8). Measured: a dangling symref, a fifo and a `refs/…` symlink print nothing, so it would not change D-29's
     cases.

P1 blocks the plan's object-read task; P2 and P3 change test assertions D-27 calls the contract. The rest of the
replan (P1 config, P2 refs, P5, the deletions, the regression tests) does not depend on them.

### Probe reproductions

Run in a scratch dir with `HOME`, `XDG_CONFIG_HOME` and `GIT_CONFIG_GLOBAL` there and `GIT_CONFIG_NOSYSTEM=1`, as
spike 001's `parity.sh` does. `C2` is a commit holding `.wsx.conf`, a symlink `lnk`, a directory `d` and an
executable `ex`; `TIP` is `workspace/dev`'s commit.

```bash
FMT='%(objectname) %(objecttype) %(objectsize) %(objectmode)'
# F2: mode per entry kind
printf '%s\n' "$C2:.wsx.conf" "$C2:lnk" "$C2:d" "$C2:ex" "$C2:nope" | git cat-file --batch="$FMT"
# F3: absent path vs a deleted blob object vs an unsorted tree: all "<request> missing"; ^{tree} still answers
printf '%s\n' "$C:.wsx.conf" "$C^{tree}" | git cat-file --batch="$FMT"
# F5: partial clone; the control (no flag) fetches
git -C src config uploadpack.allowFilter true
git clone -q --no-checkout --filter=blob:none "file://$PWD/src" pc
printf '%s\n' "$TIP:.wsx.conf" | git -C pc --no-lazy-fetch cat-file --batch="$FMT"   # ... missing
printf '%s\n' "$TIP:.wsx.conf" | git -C pc cat-file --batch="$FMT"                    # blob, fetched
# F6: objecttype dies on a ref to a missing object, the plain format does not
git for-each-ref --format='%(refname) %(objecttype)' refs/heads/                     # exit 128
git for-each-ref --format='%(refname)%00%(objectname)%00%(symref)' refs/heads/      # exit 0
# F7: final target, depth and cycle
git symbolic-ref refs/remotes/origin/b refs/remotes/origin/x
git symbolic-ref refs/remotes/origin/a refs/remotes/origin/b
git symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/a
git for-each-ref --format='%(refname) [%(symref)]' refs/remotes/origin/   # HEAD, a, b all print .../x
# F8: glob in a pattern
git update-ref refs/remotes/ab/develop "$C"
git for-each-ref --format='%(refname)' 'refs/remotes/a*/develop'          # lists refs/remotes/ab/develop
# F9: corrupt ref, with paranoia on
printf 'garbage\n' > .git/refs/heads/workspace/bad
GIT_REF_PARANOIA=1 git for-each-ref refs/heads/workspace/                 # exit 0, warning on stderr
# F10: record shape, and exit 1 on no match
git config -z --show-scope --show-origin --get-regexp '^(wsx|remote)\.' | od -c
```

## Project Constraints (from CLAUDE.md)

- One package, binary crate `wsx`, edition 2024, no library crate. So all tests of this phase are unit tests inside `src/`, not in `tests/`.
- `rust-toolchain.toml` pins rustc; tool versions live only in `scripts/install-tools.sh`.
- `scripts/check.sh all` is the gate for every change. `scripts/check.sh deps` is required when dependencies change (CONTRIBUTING design constraint 5).
- A nextest filter that matches nothing must fail. Gate one test with `--no-tests=fail -E 'test(=module::name)'`, one test per command chained with `&&`. Never `-E 'test(=a) | test(=b)'`.
- `rg -c` prints nothing on zero matches. Check absence with a Rust test or `rg -q`. Prefer a named Rust test to any shell pipeline.
- Run gate commands under bash. Capture the exit code of the command that matters, not the pipeline tail.
- `std::env::set_var`/`remove_var` are banned by `clippy.toml`. Scope environment to the child with `Command::env`.
- Never run `cargo init`/`cargo new` in the repo. New `.gitignore` lines go at the top.
- Commit subjects carry the phase scope, `type(04-NN): ...`, because the PR cut replays only phase-scoped commits.
- No dependency without a justification and a clean `scripts/check.sh deps`.
- The tool never prompts, never resolves a conflict on the user's behalf. No push, PR or repo-settings change from an agent session.
- The container mounts the checkout read-only, so a check that needs to write must use a scratch directory.
- Python 0.1.0 (`bin/wsx`, `tests/sandbox.sh`) is frozen: read for reference only.
- CONTRIBUTING's acceptance bar applies: a test that fails before the change, for the intended reason (not a compile failure); an assertion at a stable boundary; no test that reproduces the production algorithm in its body, compares a call with itself, asserts constants, or greps implementation text.
- Global rules: Rust clippy `-D warnings`, `?` not `.unwrap()` outside tests, `///` on public items, functions around 40 lines, behaviour-led test names with no review narrative, atomic tests (no chained scenarios), no regex-alternation assertions.

## Architectural Responsibility Map

wsx is a CLI, so the tiers are its own layers rather than browser/server.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Local-scope config values, remote-exists check | Git layer (`src/git/config.rs`) | — | gix types never leave `src/git/` |
| Exact ref reads, branch listing, root-tree entry | Git layer (`src/git/refs.rs`) | — | Same boundary; returns `RefTarget`, `RootEntry`, raw bytes |
| C-quote and unquote | Git layer (`src/git/quote.rs`) | Model (`encode_path` adds edge-whitespace quoting) | The gix call stays in `git`; the grammar rule stays in the model |
| Manifest grammar, path matching | Model (`src/model/manifest.rs`) | — | Pure bytes, no repository, so proptest and mutation testing run on it directly |
| Chain, fallback, configured state, built-ins | Model (`src/model/mod.rs`) | Git layer | Policy from D-01 to D-21 lives in one place |
| Exit codes, `--json`, rendering for a command | Phase 5 consumer | Model errors | Deferred to IF-01 |
| Guard's exit 0 when not configured | Phase 6 consumer | `RepoModel::NotConfigured` | Deferred; the model only reports the state |

## Standard Stack

> **Pivot (2026-11-17):** no crate is added or removed. The model's reads move to the `git` binary through the
> existing spawn helper; gix 0.90.0 stays for discovery, its refusals and object work by id. Package Legitimacy:
> no new packages, so no audit rows.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| gix | `=0.90.0` (already pinned) | Config, refs, trees, blobs | Project stack. Every call below compiled and ran under `features = ["sha1", "sha256", "revision"]`, no new feature [VERIFIED: prototype build, Cargo.toml] |
| gix-quote via `gix::quote` | 0.10.1 (already in the lock) | C-quote and unquote | `pub use gix_quote as quote;` at `gix-0.90.0/src/lib.rs:162`, not feature-gated. Matches git on every byte (measured below) [VERIFIED: gix source and differential run] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| proptest | 1.12.0, dev-dependency | SC 1 requires a proptest; `.planning/PROJECT.md` names it for the manifest and matcher | Round trip and matcher properties [VERIFIED: crates.io API, Context7 `/proptest-rs/proptest`, legitimacy seam OK] |
| tempfile | already present | Scratch repos through `testing::Scratch` | Every repo-backed test |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `gix::quote` | Hand-written unquote | Rejected: seven named escapes plus octal, and gix already matches git on all of them |
| `gix::quote` | A direct `gix-quote` dependency | Rejected: same crate, one more manifest line to keep in step with gix |
| proptest `default-features = false` | proptest defaults | Defaults add `fork`, `timeout` and `bit-set` (rusty-fork, tempfile, bit-vec) that nothing here uses [VERIFIED: crates.io features list] |
| `failure_persistence: None` | proptest's default file persistence | Default writes `proptest-regressions/` beside the source, which fails against the read-only container checkout and adds an ignored path. A printed minimal input becomes a named regression test instead |
| `strings_filter` | `raw_values_filter` | `raw_values_filter` returns `Err` for an absent key, so absence must be told apart from a real error. `strings_filter` gives `None` directly |

**Installation** (in `Cargo.toml` `[dev-dependencies]`, with a justification comment like the existing `tempfile` one):
```toml
proptest = { version = "1.12.0", default-features = false, features = ["std"] }
```

**Version verification:** `cargo search proptest` printed `1.12.0`; the crates.io API reports 1.12.0 published 2026-10-10, `MIT OR Apache-2.0`, `rust_version` 1.88 (the pin is rustc 1.99.1), not yanked.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| proptest | crates.io | 9 yrs (created 2017-06-18); 1.12.0 is 23 days old | 5,277,031/wk (seam) | github.com/proptest-rs/proptest | OK | Approved |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

**Crates proptest adds to `Cargo.lock`** [VERIFIED: lock diff in a scratch copy]: `chacha20` 0.10.2, `num-traits` 0.2.19, `proptest` 1.12.0, `rand` 0.10.3, `rand_core` 0.10.1, `rand_xorshift` 0.5.0, `regex-syntax` 0.8.11, `unarray` 0.1.4. Dev-only, so nothing reaches the shipped binary.

**cargo-deny with these added** (0.20.2, the pin, `cargo deny --locked check`): `advisories ok, bans ok, licenses ok, sources ok`, exit 0, with no `deny.toml` change. Duplicate-crate warnings stayed at one (`hashbrown`) before and after. **Control:** banning `unarray`, a crate only proptest brings, made it exit 2 with `crate 'unarray = 0.1.4' is explicitly banned`, so the run did see the new crates. Not checked: that each crate's licence is correctly declared upstream; cargo-deny trusts declarations.

**Build scripts among the new crates:** only `num-traits` has a `build.rs`, a compile-time `autocfg` probe with no network or filesystem use outside the build directory (read this session). crates.io has no postinstall.

## Architecture Patterns

### System Architecture Diagram

> **Superseded by D-23 (2026-11-17):** the reads in this diagram move to the `git` binary; see § Git-binary pivot, Read flow. The policy boxes (chain order, candidate rule, grammar, fail-closed routing) are unchanged.

```text
Phase 5+ command or Phase 6 guard
        |
        v
Repo::discover (Phase 3: trust, reftable, env refusals)
        |
        v
model::resolve(&repo, &recorded)
        |
        +--> local wsx.sourceBranch? -- repeated/empty/not a branch name --> Err (configured, fails closed)
        |         |
        |    set  |  unset
        |         |      \
        |         |       v
        |         |     candidates: refs/heads/{workspace,personal}/<one level> minus recorded
        |         |       |  exactly one                     | zero or several
        |         |       v                                  v
        |         |     build(inferred)                 inferred = Err(lists them)
        |         v       |                                  |
        |     build(branch)                                  |
        |         |       |                                  |
        v         v       v                                  v
      build: base and push remote (config key or `origin`, must be defined in this repo)
          --> integration: wsx.baseRef exact, else develop, next, <remote>/HEAD target
              (tracking refs only; local refs only when there is no base remote)
          --> worktree dir: wsx.worktreeDir or .worktrees/, D-14 rules
          --> tip of refs/heads/<branch> --> root tree entry `.wsx.conf`
                  absent / not a blob / unreadable / bad grammar --> Err
                  blob --> manifest::parse --> Manifest
        |
        v
RepoModel::Configured(model)  or  RepoModel::NotConfigured { inferred: Ok(model) | Err(why) }
```

### Recommended Project Structure

```text
src/
├── main.rs               # add `mod model;`
├── git/
│   ├── mod.rs            # add the mods and re-exports; make `testing` pub(crate)
│   ├── config.rs         # NEW: local_config_values, remote_configured
│   ├── refs.rs           # NEW: find_ref, ref_commit, refs_directly_under, root_entry, is_branch_name
│   └── quote.rs          # NEW: c_quote, c_unquote
└── model/
    ├── mod.rs            # NEW: RepoModel, Model, Error, resolve
    └── manifest.rs       # NEW: parse, covers, normalize_path, encode_path
tests/harness/fixture.rs  # wsx.baseRef and fork-remote values (Pitfall 14)
Cargo.toml, Cargo.lock    # proptest
```

`Repo` must also be re-exported from `git` (`pub use repo::Repo;`) for the model to name it. Existing `pub use` is absent: `src/git/mod.rs:25-29` declares `mod reads; mod repo; mod spawn; #[cfg(test)] mod testing;` [VERIFIED: src/git/mod.rs:25-29]. `ObjectId` and `Error` are already public there.

### Pattern 1: The local-scope, multi-value config read (D-01, D-18)

> **Superseded by D-23 (2026-11-17):** the read moves to one `git config -z --show-scope --show-origin` child (§ Git-binary pivot, P1). What it must return is unchanged: own-file local values in file order, bare and empty keys as one empty value, repeated keys kept for D-18.

**What:** `strings_filter` with `source == Local && level == 0`. Length 0 is absent, 2+ is repeated, and a single empty value is empty.

```rust
// Compiled and run in the prototype.
pub fn local_config_values(&self, key: &str) -> Vec<Vec<u8>> {
    self.repo
        .config_snapshot()
        .plumbing()
        .strings_filter(key, |meta| {
            meta.source == gix::config::Source::Local && meta.level == 0
        })
        .unwrap_or_default()
        .into_iter()
        .map(Vec::from)
        .collect()
}
```

**Measured against `git config --local --get-all`** (git 2.56.0, gix-config 0.62.2) [VERIFIED: probe run]:

| Setup | `strings_filter` (Local, level 0) | `string_filter` (`ref_storage`'s call) | git `--local` |
|---|---|---|---|
| Key only in global | `None` | `None` | exit 1 (the D-01 control; plain `--get` returns it) |
| `baseRef=a`, then `baseRef=b` | `["a","b"]` | `"b"` (last wins) | `a` `b` |
| Repeated across two `[wsx]` sections | `["1","2"]` | not measured | `1` `2` |
| `key =` | `[""]` | `""` | empty, exit 0 |
| `key` with no `=` | `[""]` | **`None`** | empty, exit 0 |
| `"a b # c"`, `a\tb`, `v # comment`, `   v   ` | normalised: `a b # c`, tab, `v`, `v` | same | same |
| `[wsx "sub"]` | `None` | `None` | empty (`wsx.sub.key` finds it) |
| Non-UTF-8 value | raw `caf\xe9` | not measured | raw `caf\351` |
| Key case `WSX.QUOTED` | matches | not measured | matches |
| `config.worktree` | `None` (`Source::Worktree`) | `None` | empty (plain `--get` returns it) |
| `GIT_CONFIG_COUNT` | `None` (filtered) | `None` | not applicable |
| Linked worktree, or hook-style `GIT_DIR`+`GIT_INDEX_FILE` | finds the common dir's value | not measured | not compared |
| SHA-256 and bare repos | finds the value | not measured | not compared |

**Included files need the `level` filter:** a value from `include.path` reads as `source=Local level=1`. `git config --local --get` does not see it (exit 1); `--local --includes` and plain `--get` do. So `level == 0` matches the control CONTEXT names, and keeps a tracked include (the trap in CLAUDE.md) from making a repo "configured". [ASSUMED: this reading of D-01, see A1 and Open Question 1]

### Pattern 2: A remote "exists" when this repo defines it (D-03, D-04)

> **Superseded by D-23 (2026-11-17):** the meaning (A2: as `git remote add` decides) stays; the remote set is parsed from the same config child (P1).

**What:** Any key in a `remote.<name>` section from `Source::Local` or `Source::Worktree`. This is git's own in-repo test.

```rust
pub fn remote_configured(&self, name: &[u8]) -> bool {
    let snapshot = self.repo.config_snapshot();
    let Some(sections) = snapshot.plumbing().sections_by_name("remote") else {
        return false;
    };
    sections
        .filter(|section| matches!(
            section.meta().source,
            gix::config::Source::Local | gix::config::Source::Worktree
        ))
        .filter(|section| section.value_names().next().is_some())
        .any(|section| section.header().subsection_name() == Some(name.as_bstr()))
}
```

**Measured** [VERIFIED: probe run]. `git remote add` is the differential: it refuses with exit 3 only when the name is already defined in the repo.

| Section | `gix remote_names()` | `git remote` | `git remote add` |
|---|---|---|---|
| Local `url`+`fetch` | listed | listed | refuses, exit 3 |
| **Global only** (`[remote "x"] prune = true`) | listed | listed | **succeeds** |
| **Local, no keys** (`[remote "empty"]`) | **listed** | not listed | **succeeds** |
| Local, keys but no url (`prune = true`) | listed | listed | not measured |
| Name with a slash (`a/b`) | listed | listed | not measured |
| Via `include.path` (`Source::Local`, level 1) | listed | not compared | refuses, exit 3 |
| `config.worktree` (`Source::Worktree`) | listed | not compared | refuses, exit 3 |
| Legacy `.git/remotes/<name>` file | not listed | not listed | not measured |

So `remote_names()` is wrong twice for D-04: a global `[remote "origin"]` would make a repo with no origin look like it has one, and then neither the chain nor `wsx.baseRef` could ever resolve there. [ASSUMED: this reading of D-03 and D-04, see A2 and Open Question 2]

### Pattern 3: Exact ref reads and the chain (D-06 to D-10)

> **Superseded by D-23 (2026-11-17):** the chain order and the refusals stay; every rung is read from one `git for-each-ref` listing, never by name (D-29), and `%(symref)` gives the final target, not the immediate one (P2, Open Question P2).

**What:** Build the full name, `FullName::try_from`, then `try_find_reference(name.as_ref())`, reading `target()` without following it, and treat a returned name other than the one asked for as `None`. Report a found ref's commit by peeling the object id that read returned (`peel_commit`), never by resolving the name again (corrected at the second-review replan; see the next list's last item).

**Measured** [VERIFIED: probe run]:
- **A full name is exact under `refs/`, not outside it.** With `refs/heads/refs/remotes/origin/ghost` present and `refs/remotes/origin/ghost` absent, `try_find_reference("refs/remotes/origin/ghost")` is `None`. Control: the longer name is found. A `refs/tags/refs/heads/ghost2` tag does not satisfy `refs/heads/ghost2` either. But a commit id in the loose file `.git/heads/refs/remotes/origin/ghost` (likewise `.git/tags/...`, `.git/remotes/...` and `.git/remotes/<full>/HEAD`) is returned under the name `heads/refs/remotes/origin/ghost`, `rev_parse_single` resolves it too, and `git show-ref --verify` fails. So `find_ref` compares the returned name. When the exact ref exists it wins over such a file, loose and packed (measured).
- **A short name is not.** `try_find_reference("develop")` found `refs/tags/develop`, and `origin/develop` found the remote branch. Never pass a short name.
- `rev_parse_single(full)` on names shaped like describe output or holding `@` (`workspace/x-1-gabc1234`, `workspace/a@b`, `workspace/@`, non-UTF-8) gave git's answer on all six shapes tried, so `resolve_commit` is safe for a full name once `find_ref` has found it. **Corrected at the second-review replan (measured on gix 0.90.0 and git 2.56.0):** that holds only when the hex after `-g` prefixes no object. With `refs/remotes/origin/rel-1-g<first 7 hex of commit Y>` holding commit X, `rev_parse_single` gave Y, while `git rev-parse --verify`, `git show-ref --verify` and `try_find_reference` gave X; the same through a symbolic `refs/heads/alias-1-g<same hex>`. Control: with a hex that prefixes no object, `rev_parse_single` gave X. gix tries a describe-shaped name as an object prefix before it tries the ref (read in `gix-revision-0.52.0/src/spec/parse/function.rs`), while git, measured, reads the ref. So `ref_commit` never resolves a name: it peels the id each `find_ref` read returns, following a symbolic ref hop by hop through `find_ref`, at most 5 refs, which matches git (four symbolic hops resolve, five fail; measured).

**`<remote>/HEAD` shapes:**

| Shape | `target()` | Treatment |
|---|---|---|
| `git clone`, or `git remote set-head origin --auto` | `Symbolic(refs/remotes/origin/main)` | Strip `refs/remotes/<base>/`, require the target to exist and hold an object, branch `main` |
| Target ref deleted (dangling) | `Symbolic(...)` still returned; `git rev-parse --verify` fails | Exists check fails, so D-09's error lists it |
| Packed | Same as loose | Same |
| `remote.origin.followRemoteHEAD=never`, after deletion, then a fetch | `None` | D-09's error |
| Object id, no symref (hand-made) | `Object(id)` | No branch name, so D-09's error |
| Symref outside the remote (`refs/heads/main`) | `Symbolic(refs/heads/main)` | Prefix strip fails, so D-09's error |
| Symref to another symref under the remote | `Symbolic(refs/remotes/origin/alias)` | Refused, naming both refs: `alias` can point anywhere, and `rev_parse` follows it, so `HEAD -> alias -> refs/heads/side` would make a local commit the base against D-06 (measured). The same holds for a symbolic `develop`, `next` or `wsx.baseRef` target |

**Branch-name validity for `wsx.baseRef` and `wsx.sourceBranch`:** `FullName::try_from("refs/heads/<name>")` agreed with `git check-ref-format --branch` on 25 names except `HEAD` and `-x`, which gix accepts and git refuses. So `is_branch_name` adds those two checks. `refs/heads/x` as a value is accepted by both and then simply is not found, which D-21 already covers.

### Pattern 4: Workspace candidates (D-17, D-21)

> **Superseded by D-23 (2026-11-17):** candidates come from the same `for-each-ref` listing, filtered to one level by exact bytes (P2). The trailing-slash rule of Pitfall 4 still applies to the patterns.

```rust
pub fn refs_directly_under(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
    // ... `failed` as elsewhere in this module ...
    let platform = self.repo.references().map_err(|err| failed(err.to_string()))?;
    let refs = platform.prefixed(prefix.as_bstr()).map_err(|err| failed(err.to_string()))?;
    let mut names = Vec::new();
    for reference in refs {
        let reference = reference.map_err(|err| failed(err.to_string()))?;
        let full = reference.name().as_bstr().to_vec();
        if !full[prefix.len()..].contains(&b'/') {
            names.push(full);
        }
    }
    names.sort();
    Ok(names)
}
```

**Measured** [VERIFIED: probe run, with branches `workspace/{a,café,nested/deep,raw\xe9,sym}`, `workspace-x`, `workspaceY`, `personal/workspace`, `personal/workspace-backup`, `personal-z`, `notworkspace/q`]:

| Query | Result |
|---|---|
| `prefixed("refs/heads/workspace/")` | The five under `workspace/`, including `nested/deep` |
| `prefixed("refs/heads/workspace")` no slash | **Also `workspace-x` and `workspaceY`** |
| `git for-each-ref 'refs/heads/workspace/*'` (the oracle's call, `scripts/workspace/cut-pr-branch.sh:140`) | The four at one level: **not** `nested/deep` |
| After `pack-refs --all` plus one new loose ref | gix 6, git 6 |
| From a linked worktree | Same list |
| Non-UTF-8 name | Listed and readable by exact name |
| Corrupt loose ref under the prefix | gix yields `Err` naming it; git warns and skips |
| Dangling symref under the prefix | gix lists it; `git for-each-ref` omits it |

The prototype applies one level, as D-21's glob reads in git. [ASSUMED: this reading of D-21, see A3 and Open Question 3] A corrupt ref becomes an error, which fails closed in the only case it can matter. A dangling symref counts as a candidate, so it produces the "several candidates" error listing it, which is safe and names the fix.

### Pattern 5: C-quoted paths (D-14, D-19)

```rust
// src/git/quote.rs: all the gix there is.
pub fn c_quote(raw: &[u8]) -> Vec<u8> {
    gix::quote::ansi_c::quote(raw.as_bstr()).to_vec()
}

pub fn c_unquote(token: &[u8]) -> Result<Vec<u8>, String> {
    let (raw, consumed) =
        gix::quote::ansi_c::undo(token.as_bstr()).map_err(|err| err.to_string())?;
    if consumed != token.len() {
        return Err("text after the closing quote".to_owned());
    }
    Ok(raw.to_vec())
}
```

**Measured differential** [VERIFIED: probe run, 270 real files: `p<byte>q` for every byte 1-255 except `/`, plus ` lead`, `trail `, `\tleadtab`, `trailtab\t`, `my dir`, `"q"`, `back\slash`, `caf\xc3\xa9`, `raw\xe9`, `#hash`, `*star`, `nl\nnl`, `cr\r`, `\x7f`, `-dash`, `a  b`]:
- `quote(raw)` equals `git ls-files` output, byte for byte across the whole list. Control: one changed byte is detected.
- `undo(git ls-files output)` equals `git ls-files -z`. Same for `-c core.quotePath=false` output.

**Malformed forms**, against `git check-ignore --stdin`, which calls git's own unquote:

| Token | git | gix `undo` | Required outcome |
|---|---|---|---|
| `"abc`, `"`, `"a\`, `"\3` | badly quoted | `Err` | Error naming the line |
| `"a\q"`, `"\e"`, `"\'"`, `"\?"` | badly quoted | `Err` | Error |
| `"\400"`, `"\38"`, `"\37"`, `"\1a1"` | badly quoted | `Err` | Error |
| `"a"b` | **accepted as `a`** | **`Ok`, consumed 3 of 4** | **Must error: check `consumed`** |
| `"\000"` | truncated to empty | **`Ok("\0")`** | **Must error: a git path has no NUL** |
| `""` | empty refused | `Ok("")`, consumed 2 | Error, empty path |
| `"\101"`, `" lead"`, `"a b"` | accepted | `Ok` | Parses |
| `a"b`, `plain` | literal | Unchanged | Parses (quoted only when it starts with `"`) |

**`gix quote` leaves leading and trailing spaces bare, exactly as git does** (` lead` and `trail ` printed unquoted; tab-edged names are quoted). D-19 refuses those bare, so `encode_path` must add the quotes itself: that is the function `wsx init` and `wsx migrate` will use in Phase 12, and the one the proptest round-trips.

### Pattern 6: The manifest blob (CONF-07, D-19)

> **Superseded by D-23 (2026-11-17):** the blob, its mode and the tip commit come from one `git --no-lazy-fetch cat-file --batch` child with `%(objectmode)` (P3). The tree-order check (`TreeNotSorted`) is deleted (D-25 inventory).

```rust
// src/git/refs.rs, the final shape: 04-09 adds the mode check, 04-11 the first call (line 482). `RootEntry` is { Absent, Blob(Vec<u8>), NotABlob { kind } }; `commit` is the id `root_entry` takes.
self.refuse_unfollowable_replacements()?; let commit = self.peeled_commit(commit)?; // peel_commit's peel: no revspec parse (checker, 2026-11-03)
let tree = commit
    .tree().map_err(|err| failed(err.to_string()))?;
let Some(entry) = tree.find_entry(name) else {
    return Ok(RootEntry::Absent);
};
let mode = entry.mode();
if !mode.is_blob() {
    let kind = if mode.is_link() { "symlink" }
        else if mode.is_tree() { "directory" }
        else { "submodule" };
    return Ok(RootEntry::NotABlob { kind });
}
let blob = self.repo.find_blob(entry.oid().to_owned()).map_err(|err| failed(err.to_string()))?;
Ok(RootEntry::Blob(blob.data.clone()))
```

**Measured against `git ls-tree` and `git cat-file -t`** [VERIFIED: probe run]:

| Entry at `.wsx.conf` | `mode()` | `is_blob()` | `find_blob` | `rev_parse_single("<tip>:.wsx.conf")` | `git ls-tree` |
|---|---|---|---|---|---|
| Regular file | `100644` | true | the bytes | Blob | `100644 blob` |
| Executable | `100755` | true | the bytes | Blob | `100755 blob` |
| **Symlink** | `120000` | **false** | **Ok: the link target's text** | **Blob** | `120000 blob` |
| Directory | `40000` | false | Err: kind tree | Tree | `040000 tree` |
| Gitlink | `160000` | false | Err: kind commit | Commit | `160000 commit` |
| Absent | `find_entry` is `None` | — | — | Err | empty |
| Empty file | `100644` | true | 0 bytes | Blob | `100644 blob` |
| Only `sub/.wsx.conf` | `None` | — | — | Err | empty |

**So only the tree-entry mode can tell a symlink apart.** A symlink's blob holds its target's text, so a read that skips the mode check succeeds, and that text is then parsed as a manifest.

**Other measured cases:**
- Uncommitted edit to the working file: unchanged read. Clean checkout without the file: same read. Both also pass as prototype tests.
- Missing blob object (loose file deleted): `find_blob` `Err`, "could not be found"; `git cat-file` also fails. Missing tree: `commit.tree()` `Err`. Missing commit: the ref still exists, peel `Err`. Each is a fail-closed reason, not a "missing manifest".
- Ref holding a blob id: peel `Err`.
- `refs/replace`: one replacement reads alike in gix and git, on a blob, a tree and a commit. A chain (A->B->C) does not: git follows it and gix stops after one hop, on all three kinds. A cycle or a self-replacement makes git fail while gix still reads. A symbolic replace ref is followed by git and ignored by gix. So `root_entry` first calls `refuse_unfollowable_replacements`, which refuses all of those repo-wide with `git::Error::ReplacementRef`, and `build` keeps that as `Error::Git` rather than `ManifestUnreadable`, so the error names the ref and does not say `.wsx.conf` cannot be read (D-22); it agreed with git, or over-refused safely, on all but one of 25 shapes measured, including uppercase hex names, packed refs, SHA-256 and `GIT_REPLACE_REF_BASE` (the exception: that variable without a trailing slash, where git itself dies). The second review (0a0dc55) found two more shapes that disagree, and the refusal covers both: two replace refs whose names differ only in hex case, where git stops with `duplicate replace ref` and gix reads one (`ReplacementRefDuplicate`), and `gitoxide.objects.replaceRefBase` set in config with `GIT_REPLACE_REF_BASE` unset, where gix reads replacements under the key and git under `refs/replace/` (`ReplaceBaseDiffers`, refused only while a ref's name starts with either base, as narrowed on 2026-11-03, so the key alone stops nothing). It also found that gix loads the replacement map when the repository is opened, while the refusal lists the refs later, so a replace ref changed in between, during one run, can make them disagree; the owner accepted that on 2026-11-03.
- SHA-256 and bare repos: same read.

`find_entry` drops malformed tree entries by `filter_map(Result::ok)` (`gix-0.90.0/src/object/tree/mod.rs:57-58`), so a corrupt tree could read as `Absent`. Not measured. It still fails closed when configured; only the reason would be imprecise.

### Pattern 7: The manifest grammar, and what "personal" means

```rust
pub fn covers(entry: &[u8], path: &[u8]) -> bool {
    path.strip_prefix(entry)
        .is_some_and(|rest| rest.is_empty() || rest[0] == b'/')
}

pub fn normalize_path(path: &[u8]) -> Result<Vec<u8>, Problem> {
    if path.contains(&0) { return Err(Problem::Nul); }
    let stripped = path.strip_suffix(b"/").unwrap_or(path);
    if stripped.is_empty() { return Err(Problem::EmptyPath); }
    if stripped[0] == b'/' { return Err(Problem::AbsolutePath); }
    for component in stripped.split(|byte| *byte == b'/') {
        match component {
            b"" => return Err(Problem::EmptyComponent),
            b"." | b".." => return Err(Problem::DotComponent),
            _ => {}
        }
    }
    Ok(stripped.to_vec())
}
```

**Line rules the prototype's table test passed** (`grammar_table`, `overlap_table`) [VERIFIED: prototype tests]:

| Line | Result |
|---|---|
| `sync a`, ` only a`, `only\ta`, whitespace only | Unknown kind |
| `only a\r`, `# c\r` | Line ends in CR. A CRLF file fails on line 1 with the reason |
| `only`, `only `, `only /` | Empty path |
| `only  a`, `only a `, `only a /` | Edge whitespace, must be quoted |
| `only /a` | Leading `/` |
| `only a//b`, `only a//` | Empty component |
| `only .`, `only a/../b` | Dot component |
| `only "\000"` | NUL |
| `only my dir` | `my dir` |
| `only a/` | `a` |
| `only a*b` | `a*b`, literal |
| `only #x` | `#x`: a comment only when `#` starts the line |
| `only a"b` | `a"b` |
| `only " a "` | ` a ` |
| `only café` (raw UTF-8), `only raw\xe9`, `only a\x01b` | Raw bytes pass unquoted |
| `only "caf\303\251"` | The UTF-8 bytes |
| No trailing newline; empty manifest; comments only | Valid |
| `only a` with `modified a`, `only a` with `modified a/b`, `modified a/b` then `only a`, `only a/` with `modified a` | Overlap, naming both lines |
| `only a`, `only a`, `only a/b` with `modified b` | Parses: same-kind duplicates and nesting are allowed |
| `only a` with `modified ab` | Parses: `ab` is not under `a` |

**`modified` is not personal.** PRCUT-02 replays `modified` changes "like any other change", DIV-01 requires an added path to be under "a built-in or `only` entry", and the oracle collects only `only` lines into `FORBIDDEN_PATHS` (`scripts/workspace/cut-pr-branch.sh:163-165`: `if [ "$KIND" = "only" ] && [ -n "$ENTRY" ]; then`). So `is_personal` covers `.wsx.conf`, the worktree dir and `only` entries, and an entry's kind stays available for Phase 5's `modified` checks.

**Keep every entry, with its line number.** D-13 needs `only .wsx.conf` to parse, and Phase 5 reports it, so the parser must not filter built-ins or dedupe.

**A deliberate difference from the oracle:** `read -r KIND ENTRY _` splits on any whitespace and drops a third word, so the oracle reads `only a b` as `a` and `only\ta` as `a`. D-19 reads the first as `a b` and refuses the second. `.workspace-divergence` in this repo has neither shape.

### Pattern 8: The model's shape and errors

```rust
pub enum RepoModel {
    Configured(Box<Model>),
    NotConfigured { inferred: Result<Box<Model>, Error> },
}

pub fn resolve(repo: &Repo, recorded: &BTreeSet<Vec<u8>>) -> Result<RepoModel, Error> {
    match single(repo, SOURCE_BRANCH)? {
        Some(value) => {
            let branch = branch_name(SOURCE_BRANCH, value)?;
            Ok(RepoModel::Configured(Box::new(build(repo, branch)?)))
        }
        None => {
            let inferred = candidates(repo, recorded).and_then(|mut names| {
                if names.len() != 1 {
                    return Err(Error::WorkspaceCandidates { candidates: names });
                }
                build(repo, names.remove(0))
            });
            Ok(RepoModel::NotConfigured { inferred: inferred.map(Box::new) })
        }
    }
}
```

**Why this shape:**
- `Err` from `resolve` means exactly "configured and failing closed". A repeated or empty `wsx.sourceBranch` lands there, because the key is present (D-15).
- `NotConfigured` never errors, however broken the rest is, so the Phase 6 guard reads it and exits 0 (D-16, D-21).
- Commands needing a workspace branch take `inferred` and report its `Err`, wrapped as "not configured, run `wsx init`" for a manifest failure (D-16) or as the candidate list (D-21). Add one `require()` method for that.
- `recorded` is the D-17 input. Production passes an empty set until Phase 8.

**Model fields:** `workspace_branch`, `workspace_tip: ObjectId`, `integration { branch, full_ref, tip }`, `base_remote: Option<Vec<u8>>`, `push_remote: Option<Vec<u8>>`, `worktree_dir`, `manifest`. Recording the tips means every later read in one command uses the same commits the manifest came from.

**Errors: a separate `model::Error`, not new `git::Error` variants, with one exception, the replacement refusals.** `git::Error` documents itself as "Why the git layer could not answer" (`src/git/mod.rs:52`). These are policy refusals, and one variant per refusal follows Phase 3's pattern and gives Phase 5 one exit code per refusal. Hand-written `Display`, `From<git::Error>` (variant `Git`), no error crate. Variants used by the prototype: `Git`, `KeyRepeated`, `KeyEmpty`, `NotABranchName`, `NoSuchRemote`, `BaseRefMissing`, `NoIntegrationBranch`, `WorkspaceBranchMissing`, `WorkspaceCandidates`, `ManifestMissing`, `ManifestNotABlob`, `ManifestInvalid`, `WorktreeDirInvalid`. The plans add more, among them `ManifestUnreadable { branch, source }`, for a manifest read that failed rather than found nothing. The exception is 04-11's `git::Error::ReplacementRef { name, target }`, joined at the second-review replan by `ReplacementRefDuplicate { first, second }` and `ReplaceBaseDiffers { value, git_base }`, which `git::Error::is_replacement_refusal` names together: they are not policy refusals but what that doc comment says, since the git layer cannot answer as git would under a replacement the two follow differently (line 482). `build` keeps them as `model::Error::Git` rather than `ManifestUnreadable`, because the refusal does not know whether the ref touches `.wsx.conf` (D-22). So they add no `model::Error` variant, and they have no exit code of their own unless Phase 5 matches inside `Git`, and inside `NotConfigured`'s `reason`, where `require` wraps them with no key; 04-10 records that in STATE.md.

**Messages must name the fix** (CONTEXT, CONTRIBUTING style). `BaseRefMissing` should say the value is a bare branch name, since `origin/develop` was the old form (Pitfall 14). `NoIntegrationBranch` lists the refs tried and both D-09 fixes, and says none of them names an integration branch, not that none exists, because a `<remote>/HEAD` that holds an object id, points outside the remote or dangles exists and still names none (second review, finding 5).

### Anti-Patterns to Avoid

- **Letting gix leak into `src/model/`.** The first prototype draft called `FullName::try_from` from the model; it moved to `git::is_branch_name`.
- **`Err` for "not configured".** The guard would have to tell apart "not configured" from a real failure by variant. Keep it a value.
- **Filtering built-ins out of the parsed manifest.** Phase 5's stale-declaration check needs them (D-13).
- **Reusing `ref_storage`'s `string_filter`** for D-18. It hides repeats and no-`=` keys (Pattern 1).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| C-quote and unquote | A byte table | `gix::quote::ansi_c` | Already equal to git on every byte value |
| Config parsing and value normalisation | Reading `.git/config` | `strings_filter` | Quotes, escapes, comments, case and trailing space all matched git |
| Ref name validity | A character list | `gix::refs::FullName::try_from`, plus the two extra checks | Agreed with `git check-ref-format --branch` on 23 of 25 names; the other two are in `is_branch_name` |
| Packed and loose ref merge | Reading `packed-refs` | `references().prefixed()` | Measured equal after `pack-refs --all` |

**Key insight:** Don't hand-roll the reads, but do not trust gix's convenience defaults either. Each read in this phase needs an explicit filter or check, and each has a named differential test against git.

## Runtime State Inventory

Not a rename or migration phase. One related item: `tests/harness/fixture.rs` writes `wsx.baseRef` in a form D-08 changes (Pitfall 14). No real repo on this machine has a `wsx.*` key today (`git config --local --get-regexp '^wsx\.'` printed nothing for wsx, devflow, gsd-core and gsd-pi), so no stored config needs migrating. The 0.1.0 projects on the owner's work PC could hold `wsx.baseRef=origin/...`: `bin/wsx:845` reads it as a revision. That is Phase 12's concern (MIG-01 writes `wsx.baseRef`), and the error message covers an unmigrated value.

## Common Pitfalls

### Pitfall 1: `string_filter` hides both D-18 cases

> **Superseded by D-23 (2026-11-17):** no gix config read remains in the model (P1).
**What goes wrong:** Copying `ref_storage`'s call gives the last value of a repeated key, and `None` for a key with no `=`.
**Why it happens:** `string_filter` is last-one-wins; `string_implicit_filter` is its implicit-aware twin, returning `Some(None)`.
**How to avoid:** `strings_filter` (Pattern 1).
**Warning signs:** A test for a repeated key that resolves instead of erroring.

### Pitfall 2: `Source::Local` includes `include.path` files

> **Superseded by D-23 (2026-11-17):** the git equivalent is that `--show-scope` reports an included file as `local`; filter on the origin as well (P1, Pitfall G4).
**What goes wrong:** A tracked include sets `wsx.sourceBranch`, read live from whatever branch is checked out, and the repo reads as configured.
**How to avoid:** Add `meta.level == 0`. Negative control: the include file's value must be invisible while `git config --local --includes --get` sees it.

### Pitfall 3: `remote_names()` over-counts

> **Superseded by D-23 (2026-11-17):** no gix remote read remains (P1).
**What goes wrong:** A global `[remote "origin"] prune = true` makes a repo without an origin look like it has one. Local refs are then never tried (D-07), and `wsx.baseRef` cannot rescue it, because it resolves against the phantom remote. An empty `[remote "x"]` also counts, though git disagrees.
**How to avoid:** `remote_configured` (Pattern 2). Run the D-04 test under a global `remote.origin` section through `testing::reexec`.

### Pitfall 4: Prefix without a slash, and nesting
**What goes wrong:** `prefixed("refs/heads/workspace")` matches `workspace-x`, so an unrelated branch makes the fallback fail. Without a depth filter, `workspace/a/b` counts though git's glob excludes it.
**How to avoid:** A trailing `/` and the one-level filter. Differential test against `git for-each-ref 'refs/heads/workspace/*' 'refs/heads/personal/*'`.

### Pitfall 5: Short names, and `git rev-parse` in differential tests
**What goes wrong:** `try_find_reference("develop")` found a tag. Separately, with `refs/heads/refs/heads/ghost` present and `refs/heads/ghost` absent, **`git rev-parse --verify refs/heads/ghost` succeeds**, by git's DWIM rules, while gix's exact read and its revspec both say missing.
**How to avoid:** Always full names in gix. In a differential test, ask git with `git show-ref --verify <full>`, which refused such a name, not `rev-parse`.

### Pitfall 6: `undo` stops at the closing quote and passes NUL
**What goes wrong:** `only "a"b` parses as `a`, and `only "\000"` gives a path holding NUL, which could never match.
**How to avoid:** `c_unquote` checks `consumed`; `normalize_path` refuses NUL. NUL is not in D-14's list, but D-14's reason ("could never match a git tree path") covers it, so this is applied without a new decision.

### Pitfall 7: Git's output is not valid under D-19 for edge spaces
**What goes wrong:** Feeding `git ls-files` output directly into a manifest fails for ` lead` and `trail `, because git prints them bare. This is by design (CONTEXT Specific Ideas).
**How to avoid:** Write through `encode_path`. The all-bytes differential test asserts exactly two such refusals, which pins this.

### Pitfall 8: A symlink reads as a valid manifest

> **Superseded by D-23 (2026-11-17):** still true, now of `cat-file --batch` with its default format: a symlink is reported as `blob`. `%(objectmode)` (`120000`) is the tell (P3, Pitfall G2).
**What goes wrong:** `rev_parse_single("<tip>:.wsx.conf")` and `find_blob` both succeed on a symlink.
**How to avoid:** Check `entry.mode().is_blob()` first (Pattern 6). Named tests for symlink, directory and submodule, each against `git ls-tree`.

### Pitfall 9: `Repo`'s config is frozen at open; refs are not

> **Superseded by D-23 (2026-11-17):** config is read by a git child during `resolve`, so it is as fresh as the refs. gix's own snapshot still drives D-22's `gitoxide.objects.replaceRefBase` check.
**What goes wrong:** A test opens `Repo`, runs `git config`, reads through the same `Repo`, and sees the old value. Measured: config stale (empty) until reopened, while a new branch and a pack-then-new-branch were both seen immediately.
**How to avoid:** Set config, then discover. Production is unaffected unless a future command writes config and re-reads through one handle.

### Pitfall 10: On reftable, these reads say "missing", not "unsupported"

> **Superseded by D-23 (2026-11-17):** git reads reftable; the refusal at `Repo::discover` (`refuse_ref_storage`, Phase 3) still runs first, so the model never reaches a reftable repo. Not in D-25's scope.
**What goes wrong:** On a reftable repo opened without Phase 3's refusal, the exact read returned `None` and the listing was empty. So the `is_unsupported()` backstop in `Repo::resolve` does not fire for them: a reftable repo would read as "no develop, no candidates".
**How to avoid:** Nothing new: `refuse_ref_storage` runs first in `Repo::discover` (`src/git/repo.rs:37`), and it is tested. Do not add an open path that skips it.

### Pitfall 11: `dead_code` fires on the test target for unread error fields
**What goes wrong:** `src/git/mod.rs:23` is `#![cfg_attr(not(test), expect(dead_code, reason = "no caller until phase 5"))]`, off on the test target, so every item must be used by a test. Measured: `#[derive(Debug)]` does not count, and error variant fields read nowhere failed clippy with `fields ... are never read`.
**How to avoid:** The new `src/model/mod.rs` carries the same attribute (measured: non-test build clean), and `Display` reads the fields. Implement `Display` in the same plan as the variants. Phase 5 removes both attributes.

### Pitfall 12: `testing` is private to `git`
**What goes wrong:** `src/git/mod.rs:28-29` has `#[cfg(test)] mod testing;`, so model tests can't reach `Scratch`.
**How to avoid:** `#[cfg(test)] pub(crate) mod testing;` (measured: compiles and runs).

### Pitfall 13: In-process `Repo::discover` reads the runner's environment
**What goes wrong:** A test that discovers in-process sees the runner's `GIT_DIR` when the suite runs from a hook, and always sees the runner's real global config.
**How to avoid:** Call `if testing::rerun_scrubbed() { return; }` first (the Phase 3 convention). Any test that plants global config must run through `testing::reexec` with the scratch env, as `ref_storage_in_global_config_does_not_refuse_a_files_repo` does (`src/git/repo.rs:2221`). Assert a `WSXCHILD` marker, never the exit status alone. `local_config_values` and `remote_configured` ignore host global config by design, so other tests stay hermetic.

### Pitfall 14: The harness fixture contradicts D-08, and has no `wsx.baseRemote`
**What goes wrong:** `tests/harness/fixture.rs:263-266` reads:
```rust
let base_ref = format!("{}/{}", config.base_remote, config.integration);
let mut local = vec![
    ("wsx.sourceBranch".to_owned(), config.workspace.clone()),
    ("wsx.baseRef".to_owned(), base_ref),
```
and `:670` asserts `("wsx.baseRef", "origin/develop\n")`, `:828` asserts `("wsx.baseRef", "upstream/develop\n")`. Under D-08 those resolve to `refs/remotes/origin/origin/develop`, which is missing, so it is an error with no fallback. `tests/harness/` also never sets `wsx.baseRemote` or `wsx.pushRemote`, so the fork fixture would resolve its base to `origin`, the wrong remote.
**Why it happens:** Phase 2 predates D-08; 0.1.0 treated `wsx.baseRef` as a revision (`bin/wsx:845`).
**How to avoid:** In this phase, set `wsx.baseRef` to `config.integration`, add `wsx.baseRemote` and `wsx.pushRemote` from `config.base_remote` and `config.push_remote`, and update the two assertions. The oracle reads no `wsx.*` key (`rg` over `tests/oracle` and `tests/harness/oracle.rs` found none), and `tests/harness/oracle.rs:242` builds its own `base_ref`, so the oracle-side records should not change. **Unverified:** the harness suite has not been run with this change; the plan's verification must do so.
**Warning signs:** Phase 5's first scenario refusing with `BaseRefMissing`.

### Pitfall 15: D-18 as written conflicts with GUARD-05
**What goes wrong:** D-18 says "An empty or repeated `wsx.*` key" is an error, while GUARD-05 says of `wsx.protectedBranches`, "an empty value turns it off". A blanket `wsx.*` scan would break that.
**How to avoid:** Apply D-18 per key, to the keys this phase reads: `wsx.sourceBranch`, `wsx.baseRef`, `wsx.baseRemote`, `wsx.pushRemote`, `wsx.worktreeDir`. Phase 11 handles its own. Do not scan the section.

### Pitfall 16: Manifest bytes reach the terminal through error messages
**What goes wrong:** An error naming a line prints raw bytes, including ESC sequences, from a blob that may have arrived through a merge. `String::from_utf8_lossy` (the current message pattern) passes those through.
**How to avoid:** Render manifest lines, paths and config values through `c_quote` in `Display`, as git does for paths. Test: a line with `\x1b` renders `\033`.

### Pitfall 17: Proptest in the repo's environment
**What goes wrong:** Default persistence writes `proptest-regressions/main.txt` on failure (measured), which would hit the container's read-only mount. Also, `PROPTEST_CASES=50` overrode an explicit `cases: 3` (measured), so the host environment can change the count, and `PROPTEST_CASES=0` reports a property as passed with no case run (measured).
**How to avoid:** `failure_persistence: None` (measured: writes nothing, still prints the minimal input). Turn any failure into a named regression test. `property_runs_its_configured_cases` counts the bodies `proptest!` runs under the shared `property_config()` and fails below 2048, so a lowered count is a named failure under every runner (measured: 0 and 2047 fail, unset and 4096 pass). `scripts/check-in-container.sh` forwards no host variable (read, and probed with the same `docker run` shape with a `-e` control), so the container run cannot be lowered from the host.

### Pitfall 18: RED evidence for a new module
**What goes wrong:** With `tdd_mode: true`, and CONTRIBUTING rejecting a compile failure as evidence, a test against a function that does not exist yet proves nothing.
**How to avoid:** Commit the signature with a wrong-answer stub, so the test compiles and fails on its assertion. Phase 3 opened the same way (`test(03-01): add failing merge-base parity test`). The JUnit report at `target/nextest/default/junit.xml` is what the TDD gate reads (`.config/nextest.toml`).

## Code Examples

### Local config with D-18

> **Superseded by D-23 (2026-11-17):** see § Git-binary pivot, P1.

```rust
fn single(repo: &Repo, key: &'static str) -> Result<Option<Vec<u8>>, Error> {
    let mut values = repo.local_config_values(key);
    match values.len() {
        0 => Ok(None),
        1 => {
            let value = values.remove(0);
            if value.is_empty() {
                return Err(Error::KeyEmpty { key });
            }
            Ok(Some(value))
        }
        _ => Err(Error::KeyRepeated { key, values }),
    }
}
```

### Remotes and the chain

> **Superseded by D-23 (2026-11-17):** see § Git-binary pivot, P1 and P2.

```rust
fn remote(repo: &Repo, key: &'static str) -> Result<Option<Vec<u8>>, Error> {
    match single(repo, key)? {
        Some(name) if repo.remote_configured(&name) => Ok(Some(name)),
        Some(remote) => Err(Error::NoSuchRemote { key, remote }),
        None => Ok(repo.remote_configured(DEFAULT_REMOTE).then(|| DEFAULT_REMOTE.to_vec())),
    }
}

fn integration(repo: &Repo, base: Option<&[u8]>) -> Result<Integration, Error> {
    let prefix = match base {
        Some(remote) => [b"refs/remotes/", remote, b"/"].concat(),
        None => b"refs/heads/".to_vec(),
    };
    let found = |branch: &[u8]| -> Result<Option<Integration>, Error> {
        let full_ref = join(&prefix, branch);
        match repo.find_ref(&full_ref)? {
            None => Ok(None),
            Some(RefTarget::Symbolic(target)) => Err(Error::IntegrationRefSymbolic { full_ref, target }),
            Some(RefTarget::Object(id)) => Ok(Some(Integration { branch: branch.to_vec(), tip: repo.peel_commit(&id)?, full_ref })),
        }
    };
    if let Some(value) = single(repo, BASE_REF)? {
        let value = branch_name(BASE_REF, value)?;
        let tried = join(&prefix, &value);
        return match found(&value)? {
            Some(integration) => Ok(integration),
            None => Err(Error::BaseRefMissing { value, tried }),
        };
    }
    let mut tried = Vec::new();
    for branch in CHAIN { // [b"develop", b"next"]
        let full = join(&prefix, branch);
        if let Some(integration) = found(branch)? {
            return Ok(integration);
        }
        tried.push(full);
    }
    if base.is_some() {
        let head = join(&prefix, b"HEAD");
        if let Some(RefTarget::Symbolic(target)) = repo.find_ref(&head)?
            && let Some(branch) = target.strip_prefix(prefix.as_slice())
            && let Some(integration) = found(branch)?
        {
            return Ok(integration);
        }
        tried.push(head);
    }
    Err(Error::NoIntegrationBranch { remote: base.map(<[u8]>::to_vec), tried })
}
```

A ref that exists but does not peel to a commit gives `Error::Git` from `peel_commit`, with no fall-through. Each rung's tip is peeled from the id its one `find_ref` read returned, so the ref found not symbolic is the ref whose commit is used; a second lookup of the name let a ref made symbolic between the reads move the base (second review, finding 6), and no deterministic test can place that change. That matches "with no fallback" in D-08, and fails closed. A symbolic ref on any rung is `Error::IntegrationRefSymbolic`, with no fall-through either: `ref_commit` follows symrefs, so a rung that accepted one could land on a local commit (measured).

### Encoding a path for the manifest

```rust
pub fn encode_path(path: &[u8]) -> Vec<u8> {
    let quoted = c_quote(path);
    let bare = quoted.as_slice() == path; let edge = path.strip_suffix(b"/").unwrap_or(path); // parse strips one `/` (D-14) before its edge check
    if bare
        && (edge.first().copied().is_some_and(is_edge_whitespace)
            || edge.last().copied().is_some_and(is_edge_whitespace))
    {
        let mut wrapped = vec![b'"'];
        wrapped.extend_from_slice(path);
        wrapped.push(b'"');
        return wrapped;
    }
    quoted
}

fn is_edge_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | 0x0b | 0x0c)
}
```

### The proptest (SC 1)

```rust
use proptest::prelude::*;
use proptest::test_runner::Config;

fn component() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(prop_oneof![2 => 1u8..=255u8, 2 => 0x21u8..=0x7eu8, 1 => Just(b' ')], 1..12).prop_filter("a git tree name", |name| {
        !name.contains(&b'/') && name != b"." && name != b".."
    })
}

fn git_path() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(component(), 1..4).prop_map(|parts| parts.join(&b'/'))
}

proptest! {
    #![proptest_config(property_config())] // Config { cases: 2048, failure_persistence: None, ..Config::default() }

    #[test]
    fn any_git_path_round_trips(path in git_path(), trailing_slash in any::<bool>()) { // the slash, which D-14 strips, added at the second-review replan
        let mut text = b"only ".to_vec();
        text.extend_from_slice(&encode_path(&[&path[..], if trailing_slash { &b"/"[..] } else { &b""[..] }].concat()));
        text.push(b'\n');
        let manifest = parse(&text).expect("an encoded path parses");
        prop_assert_eq!(&manifest.entries[0].path, &path);
    }
}
```

**Controls measured on the prototype:** making `covers` ignore the path boundary failed `name_extension_is_not_covered` and `overlap_table`. Making `encode_path` leave edge whitespace bare failed `any_git_path_round_trips` on a uniform byte generator only some of the time: re-measured, the defect passed in 36 of 60 runs, because a bare edge space needs every other byte of the path to be printable too. With the weighted generator above, the defect failed in all 200 runs and the correct encoder passed all 50 (measured on a stub parser, not the plan's). Both restored after. The generator still draws from all of 1-255 except `/`, so newline, quote, backslash, space, non-ASCII and non-UTF-8 bytes all appear, but it does not guarantee each in every run. The all-bytes differential test does. The second review (0a0dc55, finding 4) found that the encoder once tested `path.last()`, which is `/` for `a /`, so it left that path bare and `parse` refused it as edge whitespace, and that the generator made no trailing `/`, so the property could not see it. Both are fixed above; 04-03's `encoded_space_before_a_trailing_slash_round_trips` pins the shape, and how often the property now reaches it is not measured.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `.workspace-divergence` read with `read -r KIND ENTRY _` | C-quoted raw-byte grammar, unknown lines are errors | This phase | The oracle silently truncates `only a b`; wsx does not |
| `origin/develop` style `wsx.baseRef` (0.1.0, harness fixture) | Bare branch name plus `wsx.baseRemote` (D-08, D-02) | This phase | Fixture and migration must write the new form |
| `origin/HEAD` as the default base | `develop`, then `next`, then `<remote>/HEAD`, with stale accepted (D-06, D-10) | This phase | `.planning/research/WORKSPACE-MODE.md` R3 records devflow's `origin/HEAD` as `main`; it reads `develop` today (measured), which the chain handles either way |

**Deprecated/outdated:** STATE.md's blocker note that gates run with `PATH=$PWD/target/gsd-tools:$PATH`: that directory does not exist now. `.tools/bin` holds the pins, and `scripts/check.sh` already prefers it.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | D-01's "local repo config" excludes `include.path` files (`level == 0`), so a `wsx.*` key in an included file is unset | Pattern 1 | Someone who deliberately includes wsx config reads as not configured. Low, and the fix is one filter term |
| A2 | "A remote exists" (D-03, D-04) means defined in this repo, as `git remote add` decides, not as `git remote` lists | Pattern 2 | A remote defined only in global config counts as missing. Low: such a remote has no repo-specific tracking refs in practice |
| A3 | D-21's `refs/heads/workspace/*` is one level, as git's glob reads it | Pattern 4 | `workspace/a/b` alone gives "zero candidates", naming the fix, instead of being picked. Fails safe either way |
| A4 | Each `wsx.*` rule in D-18 applies per key read by this phase, not across the section | Pitfall 15 | Reading it as a section scan would break GUARD-05's empty value |
| A5 | `wsx.baseRef=HEAD`, a leading `-` and other values `git check-ref-format --branch` refuses are errors, an extension of D-08 and D-21 | Pattern 3 | Harmless: git can't create such branches |
| A6 | Whitespace in D-19 means space, tab, CR, VT, FF | Code Examples | Only space can reach the unquoted path from git's own output, so only hand-written lines are affected |

| A7 | `%(objectmode)` and `--no-lazy-fetch` exist in every git from the 2.54 floor up (from memory: added in 2.45 and 2.44); both were verified only on 2.56.1 | P3, G3 | A git between 2.54 and 2.56 without them would fail every `resolve` loudly (unknown option or format), not silently. No test runs git 2.54 (Environment) |
| A8 | A gitlink at `.wsx.conf` whose commit is not in this repo answers `missing` from `<tip>:.wsx.conf` | P3 table | The tree scan still finds mode `160000` either way; `submodule_manifest_fails_closed` settles it |
| A9 | `git config` exits 3 or 128, never 1, on a malformed config file | P1 | A malformed file read as "no keys" would leave a configured repo not configured. The parser test should plant one and assert `GitFailed` |
| A10 | `--show-origin -z` prints a path with a space or non-UTF-8 byte unquoted | G4 | The origin comparison would fail and every key read as unset (not configured). One fixture with such a path settles it |
| A11 | cat-file's `missing` token is never translated | G9 | A translated token would make the parser fail with an output error (loud) |

These are readings of decisions the owner left to research; each was measured against git where git has an opinion. The plan can state them as decided (they fall under Claude's Discretion in CONTEXT); plan review can challenge them.

## Open Questions (RESOLVED, gix era)

The pivot's open questions, P1 to P3, are in § Git-binary pivot and are **not** resolved.

1. **Does an included file count as "local repo config" (D-01)?**
   - What we know: `git config --local --get` says no, and that is the control CONTEXT names. Plain `git config --get` and gix's `Source::Local` say yes. CLAUDE.md explains why a live include is dangerous.
   - What's unclear: whether anyone wants wsx config through an include.
   - RESOLVED: No. 04-05 adopts A1: D-01's "local repo config" excludes `include.path` files (`meta.level == 0`), pinned by `git::config::tests::included_key_is_not_read`. If wrong, someone who deliberately includes wsx config reads as not configured, and the fix is one filter term.

2. **Should a remote defined only in global config exist for D-03 and D-04?**
   - What we know: git has both answers. `git remote` lists it, and `git remote add` treats the name as free.
   - What's unclear: nothing that blocks work.
   - RESOLVED: No. 04-06 adopts A2: "a remote exists" (D-03, D-04) means defined in this repo, as `git remote add` decides, not as `git remote` lists, so a remote defined only in global config counts as missing. Pinned by `git::config::tests::global_remote_section_is_not_a_remote`, `keyless_local_remote_section_is_not_a_remote` and `local_remote_is_a_remote`.

3. **Do nested branches count as fallback candidates?**
   - What we know: the oracle's `for-each-ref 'refs/heads/workspace/*'` excludes them. `git branch --list` was not measured.
   - RESOLVED: No. 04-08 adopts A3: D-21's `refs/heads/workspace/*` is one level, as git's glob reads it, pinned by `git::refs::tests::refs_directly_under_matches_for_each_ref`. If wrong, `workspace/a/b` alone gives "zero candidates", naming the fix, instead of being picked, which fails safe either way.

4. **Which order for reads when several are broken?**
   - RESOLVED: The first failure only. 04-01's `build` keeps the prototype's order: the branch, then remotes, then integration, then the worktree dir, then the manifest. D-09 and D-12 require naming the refs and lines of one failure, not every failure, so this meets them. 04-04 adopts the same for lines: a manifest with two bad lines reports the first only, which meets D-11 and D-12. Phase 5 can revisit this if `wsx check` wants a full list.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| git | Fixtures, differential tests, and now every model read | ✓ | 2.56.1 on the host (2026-11-17; was 2.56.0 on 2026-11-02) | — |
| rustc, cargo | Build | ✓ | 1.99.1 (pinned) | — |
| cargo-nextest | `scripts/check.sh test` | ✓ | 0.9.148 in `.tools/bin` (the pin); host PATH has 0.9.150 | `check.sh` puts `.tools/bin` first |
| cargo-deny | `scripts/check.sh deps` | ✓ | 0.20.2 | — |
| Docker | `scripts/check-in-container.sh` | ✓ | 29.9.2 | — |
| cargo-mutants | REL-03, later | ✓ | 27.1.0 (`~/.cargo/bin`, not pinned here) | — |
| shellcheck | Not needed: no script changes | ✓ | — | — |
| crates.io | Fetching proptest on first build | ✓ this session | — | Fails the build, doesn't skip |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none.

**Pivot (2026-11-17):** the pivot adds no tool. It needs `%(objectmode)` in `cat-file --batch` and the `--no-lazy-fetch` global option (A7), both verified on the host's 2.56.1. The container's git was 2.56.0 on 2026-11-02 and was not re-checked this session; the phase gate in the container settles it.

The container image is `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie` (`.devcontainer/Dockerfile:14`). Its git, measured 2026-11-02 with `git --version` in the image and not through the test suite: `/usr/local/bin/git` is 2.56.0, `/usr/bin/git` is 2.47.3, and `HERMETIC_PATH` in `src/git/testing.rs` selects 2.56.0, equal to the host, so the container adds no second git version and no real git at the 2.54 floor runs. The prototype's behaviour was not run in it, so the phase gate must.

## Validation Architecture

Rewritten 2026-11-17 for the 04-13 replan. The 2026-11-02 map (every row "❌ Wave 0") is replaced: those tests now
exist, and the D-01 to D-21 ones are the contract the swap must keep green (D-27), except the seven listed under
"Contract tests that cannot keep their assertion". Names marked ❌ are proposals; the planner fixes them.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | libtest unit tests in the binary crate (`src/model/`, `src/git/`), proptest 1.12.0, run by the pinned cargo-nextest |
| Config file | `.config/nextest.toml` (exists; no `slow-timeout`, see Pitfall G7); `--no-tests=fail` is fixed by `scripts/check.sh` |
| Quick run command | `scripts/check.sh test -E 'test(=<module>::tests::<name>)'`, one test per command, chained with `&&` |
| Full suite command | `scripts/check.sh all`; `scripts/check-in-container.sh all` at the phase gate |
| Runtime | Not timed this session |

Module paths used below: model tests are `model::tests::`, the grammar `model::manifest::tests::`, the spawn helper
`git::spawn::tests::`, the config parser `git::config::tests::`, the ref listing `git::refs::tests::`. A new file for
the object read (for example `git::objects::tests::`) is the planner's call.

### Requirements to tests

| Req | Behaviour | Type | Command | Exists? |
|-----|-----------|------|---------|---------|
| CONF-01 | Any git path round-trips; unknown kinds name the line (grammar untouched by the pivot) | proptest, unit | `scripts/check.sh test -E 'test(=model::manifest::tests::any_git_path_round_trips)' && scripts/check.sh test -E 'test(=model::manifest::tests::unknown_kind_names_the_line)' && scripts/check.sh test -E 'test(=model::manifest::tests::name_extension_is_not_covered)'` | ✅ |
| CONF-02 | Only built-ins and declared `only` paths are personal | unit, repo | `scripts/check.sh test -E 'test(=model::tests::undeclared_path_is_not_personal)' && scripts/check.sh test -E 'test(=model::tests::declared_path_is_personal)' && scripts/check.sh test -E 'test(=model::tests::custom_worktree_dir_replaces_the_default)'` | ✅ |
| CONF-03 | Chain order, `baseRef`, D-09 error, single-hop symbolic refusal | unit, repo | `scripts/check.sh test -E 'test(=model::tests::develop_beats_the_remote_default)' && scripts/check.sh test -E 'test(=model::tests::next_follows_develop)' && scripts/check.sh test -E 'test(=model::tests::remote_default_follows_next)' && scripts/check.sh test -E 'test(=model::tests::base_ref_overrides_the_chain)' && scripts/check.sh test -E 'test(=model::tests::no_integration_branch_lists_the_refs_tried)' && scripts/check.sh test -E 'test(=model::tests::symbolic_develop_is_refused)'` | ✅ |
| CONF-03 | A stray local `develop` does not redirect; a rung under another path does not count | unit, repo | `scripts/check.sh test -E 'test(=model::tests::local_develop_does_not_redirect_a_remote_base)' && scripts/check.sh test -E 'test(=model::tests::ref_under_another_path_does_not_satisfy_a_rung)'` | ✅ |
| CONF-04 | Fork layout, defaults, missing remote, `pushDefault` unread | unit, repo | `scripts/check.sh test -E 'test(=model::tests::fork_layout_resolves_each_remote)' && scripts/check.sh test -E 'test(=model::tests::remotes_default_to_origin)' && scripts/check.sh test -E 'test(=model::tests::missing_base_remote_names_the_key_and_the_remote)' && scripts/check.sh test -E 'test(=model::tests::push_default_is_not_read)' && scripts/check.sh test -E 'test(=model::tests::remote_name_with_a_slash_resolves)'` | ✅ |
| CONF-05 | Key names the branch; one candidate; several listed; recorded excluded; nested and remote-only ignored | unit, repo | `scripts/check.sh test -E 'test(=model::tests::source_branch_names_the_workspace)' && scripts/check.sh test -E 'test(=model::tests::only_candidate_is_inferred)' && scripts/check.sh test -E 'test(=model::tests::several_candidates_are_listed)' && scripts/check.sh test -E 'test(=model::tests::recorded_branches_are_not_candidates)' && scripts/check.sh test -E 'test(=model::tests::nested_branch_is_not_a_candidate)' && scripts/check.sh test -E 'test(=model::tests::remote_only_branch_is_not_a_candidate)'` | ✅ |
| CONF-07 | From the object database in any checkout; fails closed once configured; not configured otherwise | unit, repo | `scripts/check.sh test -E 'test(=model::tests::working_tree_edit_changes_nothing)' && scripts/check.sh test -E 'test(=model::tests::linked_worktree_resolves_the_same_list)' && scripts/check.sh test -E 'test(=model::tests::bare_repo_resolves_the_same_model)' && scripts/check.sh test -E 'test(=model::tests::hook_environment_resolves_the_same_list)' && scripts/check.sh test -E 'test(=model::tests::missing_manifest_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::symlink_manifest_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::missing_blob_object_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::invalid_inferred_manifest_is_not_configured)'` | ✅ |
| CONF-07 | `resolve` writes nothing | unit, repo | `scripts/check.sh test -E 'test(=model::tests::resolve_changes_nothing_in_the_repository)' && scripts/check.sh test -E 'test(=model::tests::repository_snapshot_sees_a_ref_change)'` | ✅ |

### Pivot decisions to tests

| Decision | Behaviour | Type | Command | Exists? |
|----------|-----------|------|---------|---------|
| D-23 | The model reads no ref, config or manifest through gix: the parsers on byte fixtures | unit | `scripts/check.sh test -E 'test(=git::config::tests::parse_config_z_keeps_only_the_own_local_file)' && scripts/check.sh test -E 'test(=git::config::tests::parse_config_z_bare_key_is_one_empty_value)' && scripts/check.sh test -E 'test(=git::refs::tests::parse_ref_listing_reads_raw_bytes)' && scripts/check.sh test -E 'test(=git::objects::tests::parse_batch_reads_missing_and_sized_records)'` | ❌ |
| D-23 | Each source moved to git keeps its differential against git (config scope, remotes, listing) | unit, repo | `scripts/check.sh test -E 'test(=git::config::tests::included_key_is_not_read)' && scripts/check.sh test -E 'test(=git::config::tests::worktree_config_key_is_not_read)' && scripts/check.sh test -E 'test(=git::config::tests::command_line_config_key_is_not_read)' && scripts/check.sh test -E 'test(=git::config::tests::global_remote_section_is_not_a_remote)' && scripts/check.sh test -E 'test(=git::refs::tests::listing_survives_packing)' && scripts/check.sh test -E 'test(=git::refs::tests::non_utf8_branch_is_listed_by_its_bytes)'` | ✅ names, ported bodies |
| D-23 | `%(objectmode)` tells the entry kind; a missing entry is classified by the root tree | unit, repo | `scripts/check.sh test -E 'test(=model::tests::symlink_manifest_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::directory_manifest_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::submodule_manifest_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::missing_blob_object_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::unsorted_tree_manifest_fails_closed)'` | ✅ (last one rewritten) |
| D-23, D-09 | No network read: a partial clone's blob is not fetched | unit, repo | `scripts/check.sh test -E 'test(=model::tests::partial_clone_manifest_is_unreadable_without_a_fetch)'` | ❌ |
| D-24, D-22 | Every existing replace-ref refusal still fires, and resolves once removed | unit, repo | `scripts/check.sh test -E 'test(=model::tests::single_replacement_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::replace_ref_with_replace_objects_off_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::replace_ref_leaves_a_not_configured_repo_not_configured)' && scripts/check.sh test -E 'test(=model::tests::replace_dir_left_empty_resolves)'` | ✅ |
| D-25 | Deleted items are gone (a source check, not a test): `rg -n -e 'fn find_ref' -e 'fn ref_commit' -e 'fn refs_directly_under' -e 'fn root_entry' -e 'fn local_config_values' -e 'fn remote_configured' -e TreeNotSorted src` exits 1 | lint | as written, with a negative control: run it once before the deletion and see it match | — |
| D-26, D-31 | Four children in the own worktree, five aimed at another; a repo with two candidates spawns three (the control that the counter counts) | unit, PATH wrapper | `scripts/check.sh test -E 'test(=model::tests::resolve_spawns_four_git_children_in_its_own_worktree)' && scripts/check.sh test -E 'test(=model::tests::resolve_aimed_at_another_worktree_adds_the_env_var_query)' && scripts/check.sh test -E 'test(=model::tests::resolve_without_one_candidate_skips_the_object_read)'` | ❌ |
| D-26 | p95 under 100 ms on the four R3 repos | manual measurement | not a test: machine-dependent and reads the operator's real repos (read-only); a release build timing `resolve` 200 times per repo, as spike 001's `bench-git.sh` did | ❌ manual |
| D-27 | The round 1/2 and review shapes (table "D-27 regression shapes") | unit, repo | `scripts/check.sh test -E 'test(=model::tests::private_replace_ref_in_a_linked_worktree_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::common_replace_ref_from_a_linked_worktree_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::private_branch_in_a_linked_worktree_is_not_a_candidate)' && scripts/check.sh test -E 'test(=model::tests::relative_symlinked_source_branch_resolves_as_git_does)' && scripts/check.sh test -E 'test(=model::tests::main_worktree_alias_source_branch_reads_as_missing)' && scripts/check.sh test -E 'test(=model::tests::onbranch_remote_under_a_symlinked_head_counts_as_git_does)' && scripts/check.sh test -E 'test(=model::tests::dangling_symref_is_not_a_candidate)' && scripts/check.sh test -E 'test(=model::tests::glob_bytes_in_a_remote_name_match_only_themselves)'` | ❌ |
| D-29 | A `refs/…` symlink reads as absent: missing source branch, and D-09 for a symlinked HEAD | unit, repo | `scripts/check.sh test -E 'test(=model::tests::git_style_symlinked_source_branch_reads_as_missing)' && scripts/check.sh test -E 'test(=model::tests::symlinked_remote_head_gives_no_integration_branch)'` | ❌ |
| D-29 | A fifo loose ref never hangs `resolve` | unit, child with a kill | `scripts/check.sh test -E 'test(=model::tests::fifo_ref_is_skipped_without_hanging)'` | ❌ |
| D-30 | A dangling symbolic rung is absent; once its target exists it is `IntegrationRefSymbolic` | unit, repo | `scripts/check.sh test -E 'test(=model::tests::dangling_symbolic_rung_is_absent)' && scripts/check.sh test -E 'test(=model::tests::symbolic_next_is_refused)'` | ❌ / ✅ |
| D-32 | A linked worktree's private `refs/replace` entry refuses, an unreadable one is `ReplaceRefsUnreadable`, and the main worktree is unaffected | unit, repo | `scripts/check.sh test -E 'test(=model::tests::private_replace_ref_in_a_linked_worktree_fails_closed)' && scripts/check.sh test -E 'test(=model::tests::unreadable_private_replace_dir_blames_the_replace_refs)'` | ❌ |
| IN-02 | `ReplaceBaseSet`'s remedy per setting | unit | `scripts/check.sh test -E 'test(=git::tests::replace_base_set_message_names_the_setting_and_the_fix)' && scripts/check.sh test -E 'test(=git::tests::replace_base_variable_message_names_only_the_variable)'` | ✅ / ❌ |

D-28 (order: spike, then replan under Tier A) is a process decision with no test.

### Sampling Rate
- **Per task commit:** the task's named tests through the quick run command, each RED first where the task adds
  behaviour; a deletion task runs the D-25 source check and the D-01 to D-21 commands above.
- **Per wave merge:** `scripts/check.sh all`.
- **Phase gate:** `scripts/check.sh all` and `scripts/check-in-container.sh all` green on the final tip, then a fresh
  `cargo mutants` pass over the changed files (04-10's practice), before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `Git` method for fixed stdin input (only under Open Question P1, option A), with its own test that the child
  reads exactly the buffer and then EOF; `git::spawn::tests::children_get_null_stdin` stays for `run`.
- [ ] A counting `git` wrapper usable from `model::tests`: `git::spawn::tests` has `fake_git` and `log_lines`, which
  are private to that module; move a shared one into `git::testing`.
- [ ] A bounded child run for the fifo test (`testing::reexec` has no timeout today).
- [ ] A partial-clone fixture (`uploadpack.allowFilter` on a local `file://` source, as in the probe).

### Suggested implementation order (fine granularity)
1. The D-25 source check's negative control (it matches today), then the D-01 to D-21 suite green as the baseline.
2. P1: `parse_config_z` plus the ported config tests; switch `single`, `remote`, `default_remote`; delete `local_config_values` and `remote_configured`.
3. P2: the listing plus its parser and the ported listing tests; switch `build`, `integration`, `candidates`; delete `find_ref`, `ref_commit`, `refs_directly_under`; apply the P2/P3 decisions to the affected tests.
4. P4 then P3: the stdin method (or option B), the batch read, the classification; delete `root_entry` and `TreeNotSorted`.
5. P5: D-32 and the CR-01/IN-01/IN-02 doc and message fixes.
6. The D-27 regression tests and the D-31 count tests; the STATE obligation for grafts and shallow (D-32).
7. The phase gate, the mutation pass, and the D-26 latency measurement (manual).

## Security Domain

`security_enforcement: true`, ASVS level 1.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No credentials; no network in resolution (D-09) |
| V3 Session Management | no | — |
| V4 Access Control | yes | Phase 3's trust and ownership refusals run in `Repo::discover` before any read here; local-scope config (D-01); fail closed once configured (CONF-07) |
| V5 Input Validation | yes | The grammar parser refuses unknown lines; path rules (D-14) plus NUL; `is_branch_name`; `FullName::try_from` for every built ref name |
| V6 Cryptography | no | SHA-1 and SHA-256 object ids are gix's |
| V7 Error Handling and Logging | yes | One typed refusal per cause, each naming the reason; manifest and config bytes escaped in messages (Pitfall 16) |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Global or included config silently configures an unrelated repo | Elevation of privilege | `source == Local && level == 0`; negative controls against git |
| A symlink at `.wsx.conf` smuggling a manifest that reads as its link text | Tampering | Tree-entry mode check before the blob read |
| A path hiding through quoting or a name prefix, so a personal path looks clean | Information disclosure | Raw-byte comparison on a boundary (`covers`); all-bytes differential; proptest |
| An ambiguous manifest leaving a path both replayed and stripped | Tampering | Cross-kind overlap refusal (D-12) |
| Terminal escape sequences in a manifest line or value reach the user's terminal | Spoofing | `c_quote` in `Display` |
| A stray branch blocks pushes in an unrelated repo | Denial of service | `NotConfigured` is a value, and the guard exits 0 (D-16) |
| A partial clone turns `resolve` into a fetch from the remote (pivot, F5) | Information disclosure, availability | `git --no-lazy-fetch` on object-reading children (verified with a control) |
| A remote name holding glob bytes makes the rung pattern read another remote's tracking refs (pivot, F8) | Tampering | Exact-byte filter on every listed record |
| A git child waits on stdin and hangs a push (pivot) | Denial of service | `run` keeps a null stdin; a stdin-fed child gets a fixed buffer closed before waiting (Open Question P1) |
| A fifo loose ref hangs the guard (round 2 pi) | Denial of service | Every ref is read through `for-each-ref`, which skips it (D-29, spike 001 S8) |
| A hostile manifest size | Denial of service | Not bounded: `find_blob` reads it whole, and overlap checking is quadratic in entries. Accepted for ASVS L1: the blob is on the user's own branch |

## Sources

### Primary (HIGH confidence)
- Scratch probe binary and prototype on gix 0.90.0 (gix-config 0.62.2, gix-ref 0.69.1, gix-object 0.66.1, gix-quote 0.10.1), against git 2.56.0: every "measured" claim above
- gix sources read this session: `gix-0.90.0/src/lib.rs:155-165`, `src/repository/config/remote.rs:23-34`, `src/object/tree/mod.rs:56-100`, `src/reference/iter.rs:45-87`; `gix-quote-0.10.1/src/ansi_c.rs`; `gix-ref-0.69.1/src/store/file/find.rs:88-133`; `gix-config-0.62.2/src/file/access/comfort.rs:238`
- Repo files read this session: `src/git/{mod,repo,reads,testing}.rs`, `tests/harness/fixture.rs`, `scripts/workspace/cut-pr-branch.sh`, `.workspace-divergence`, `clippy.toml`, `deny.toml`, `.config/nextest.toml`, `Cargo.toml`, `CLAUDE.md`, `CONTRIBUTING.md`, `.planning/{REQUIREMENTS,ROADMAP,STATE}.md`, 04-CONTEXT.md

- **Pivot (2026-11-17):** `.planning/spikes/001-git-binary-model-reads/README.md` (and its `parity.sh`, `bench.sh`), `.planning/spikes/002-gix-object-reads/README.md`, `.planning/reviews/04-13-adversarial/round2/SUMMARY.md`, `04-REVIEW.md`, `04-13-PLAN.md` (test shapes), `03-CONTEXT.md` § Running the git binary; code read this session: `src/git/{mod,refs,config,spawn}.rs` (production parts), `src/model/mod.rs` (production part and the tests quoted), test names of `src/git/{config,refs,repo,mod}.rs` and `src/model/{mod,manifest}.rs`; probes F1 to F11 on git 2.56.1 with controls (scratch, removed); `git help for-each-ref` (the `symref` atom)

### Secondary (MEDIUM confidence)
- Context7 `/proptest-rs/proptest`: `Config` fields, `failure_persistence`, `rng_seed`, `PROPTEST_RNG_SEED`, file persistence. Confirmed where it matters by running 1.12.0
- crates.io API and the `package-legitimacy` seam: proptest metadata and features

### Tertiary (LOW confidence)
- None used.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH. Built and gated in a scratch copy; `cargo deny` with a control.
- gix reads: HIGH. Each measured with a control or a git differential, on one git version.
- Grammar: HIGH for byte handling. MEDIUM for the readings A3, A5, A6.
- Model shape and errors: MEDIUM. The prototype works, but this is a judgement call.
- Harness fixture impact: MEDIUM. The contradiction was read from source; the fix was not run.

- Pivot reads: HIGH for F2 to F11 (each measured with a control on one git version). MEDIUM for the proposed shapes in P1 to P4 (no Rust prototype). The three open questions are decisions, not unknowns.

**Research date:** 2026-11-02, revised 2026-11-17
**Graph context:** the knowledge graph is 326 commits behind (built at `9e96bc8`) and returned only Python 0.1.0 and planning nodes, so it was not used.
**Graph context (2026-11-17):** `graphify explain "refuse_replace_refs"` located the guard and its callers; the code was then read directly.
**Valid until:** the next gix or git upgrade, or 30 days.
