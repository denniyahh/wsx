# Phase 4: Repo model - Context

**Gathered:** 2026-11-01
**Updated:** 2026-11-17 (D-23 to D-28: model inputs through the git binary)
**Status:** Spike, then replan 04-13

<domain>
## Phase Boundary

Phase 4 delivers one resolved repo model that every later command reads (CONF-01, CONF-02, CONF-03,
CONF-04, CONF-05, CONF-07):
- the integration branch, the workspace branch, and the base and push remotes;
- whether the repo is configured;
- a single personal-path list read from `<workspace tip>:.wsx.conf` in the object database.

It builds no `wsx` command. The first command is Phase 5's `wsx check`, which also sets exit codes
and `--json` (IF-01). CONF-06 stays in Phase 8, apart from reading `wsx.worktreeDir` (D-20).

</domain>

<decisions>
## Implementation Decisions

### Config keys and sources
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

### Integration-branch chain
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

### Manifest grammar (`.wsx.conf`)
- **D-11:** A line starting with `#` is a comment, and an empty line is ignored. There are no trailing
  comments. Any other unknown line is an error naming the line.
- **D-12:** An invalid manifest fails closed for every command including the push guard, so only
  ambiguity is invalid, never staleness. A cross-kind overlap is an error naming both lines: `only a`
  with `modified a`, or `only a` with `modified a/b` (either direction). Exact duplicates and nested
  lines of the same kind parse fine.
  *Operator ruling 2026-11-05:* "only ambiguity is invalid" contrasts ambiguity with staleness; it does
  not forbid an explicit size limit. A manifest over 64 KiB is invalid (third adversarial review,
  finding 8: the overlap check is quadratic and the blob can arrive through a merge).
- **D-13:** A declaration already covered by a built-in must still parse, because Phase 5 reports it
  as a named `wsx check` failure (DIV-02).
- **D-14:** Paths are literal, with no glob matching, so `*` is an ordinary byte. One trailing `/` is
  stripped, as the oracle does. These are errors naming the line, since they could never match a git
  tree path: an empty path, `.`, a `.` or `..` component, a leading `/`, and `a//b`.

### Workspace-branch fallback
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
- **D-33 (operator, 2026-11-17):** The `.wsx.conf` read uses `git cat-file --batch` through one spawn-helper
  method that writes a small fixed request to stdin and closes it at once, so git can never wait for input.
  This amends Phase 3's "every child gets a null stdin" (`03-CONTEXT.md`:59) for that method only; `Git::run`
  keeps a null stdin. Chosen over null stdin with `ls-tree` plus `cat-file blob`, which costs a fifth child.
- **D-34 (Claude's call, 2026-11-17, amends D-27 for three tests):** `for-each-ref`'s `%(symref)` gives a
  chain's final target, and the model follows that reading. A `<remote>/HEAD` chain whose final target is inside
  the base remote resolves; one ending outside it is not a default. D-06's aim, never landing on a local commit,
  holds. The three D-06 chain tests are rewritten to git's answer.
- **D-35 (Claude's call, 2026-11-17, amends D-27 for one test):** A corrupt or badly named ref that git skips is
  absent, as D-29 already treats every ref git cannot list. A configured repo whose `wsx.sourceBranch` is such a
  ref still fails closed (`WorkspaceBranchMissing`).
- **D-36 (Claude's call, 2026-11-17):** "A remote exists" (D-03, D-04) means what `git remote add` refuses as
  already existing, as spike 001 C3 measured; a `wsx.*` key from an `include.path` file is unset (D-01), as
  today's tests already pin.
- **D-37 (Claude's call, 2026-11-17, amends D-27 for one test):** `unsorted_tree_manifest_fails_closed` keeps its
  outer `ManifestUnreadable` and fail-closed result; its inner cause becomes what `cat-file` reports (object
  missing) instead of gix's `TreeNotSorted`.
  Widened 2026-11-18 (Claude's call, after the phase verifier's ruling) to the same shape in two more tests 04-16
  changed: `require_passes_a_git_failure_through` now expects git's `ObjectMissing`, and
  `require_wraps_an_unreadable_manifest_as_not_configured` names `<tip>:.wsx.conf`; both keep their outer behaviour.
- **Tier A diff review (D-28):** after the last plan's gates pass, the orchestrator runs the external-review diff
  lanes (at least two, in parallel) on the final tip, up to three rounds until no critical finding remains (then the operator decides), before
  the phase is verified.
- **D-38 (operator, 2026-11-17):** When `GIT_CONFIG` is set, the config read drops it for that one child, so
  `git config` reads the repository's own config as every other git command does. This amends Phase 3's rule that a
  child aimed at wsx's own worktree gets the whole environment, for `GIT_CONFIG` on the config child only. Chosen
  over refusing to run while it is set, and over accepting that a configured repo then reads as not configured.
- **D-39 (operator, 2026-11-18, widens D-29):** wsx never hangs on a read git itself would block on. Every git
  child is killed once it runs past 10 s and the run fails with `GitTimedOut` naming the command (a fifo under a ref
  an `includeIf "onbranch:"` evaluates, for one). gix's in-process open in `Repo::discover` is probed on a thread
  held to the same limit and fails with `OpenTimedOut` (a fifo at `.git/config` or `packed-refs`), and the replace
  guard opens `packed-refs` only when it is a regular file. A fifo planted between the probe and the real open is not
  caught: it needs a concurrent writer of the user's own git dir during one run (as T-04-37). Chosen over accepting
  that wsx hangs where git does. From the Tier A diff review round 1 (C2); the in-process half from the phase code
  review (CR-01), which found the first wording claimed the config and packed-refs cases for the child limit alone.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Scope and requirements
- `.planning/ROADMAP.md` § Phase 4 — goal and the five success criteria
- `.planning/REQUIREMENTS.md` — CONF-01 to CONF-05 and CONF-07 for this phase; CONF-06, WTREE-05,
  GUARD-03, GUARD-04 and DIV-02 for the later phases that consume the model
- `.planning/PROJECT.md` § Key Decisions — new grammar, fail-closed once configured, no public library
  crate

### Repo layouts the model must handle
- `.planning/research/WORKSPACE-MODE.md` § R3 — the four repos on this machine and what each needs
  configured (devflow and wsx on `develop`, gsd-core with `upstream/next`, gsd-pi on `main`)

### Oracle behaviour
- `scripts/workspace/cut-pr-branch.sh` — manifest reading (`read -r KIND ENTRY _`), literal matching
  with a stripped trailing `/`, `BASE_REMOTE`/`BASE_BRANCH` handling
- `.workspace-divergence` — the current manifest, with comments and `only`/`modified` lines

### The git-binary pivot
- `.planning/reviews/04-13-adversarial/round2/SUMMARY.md` — the shapes D-27 must cover and the operator decision
- `.planning/phases/04-repo-model/04-REVIEW.md` — post-04-12 review (CR-01, WR-01, IN-01, IN-02)
- `.planning/phases/04-repo-model/04-13-PLAN.md` — superseded (D-23); read for its test shapes only

### Prior phase
- `.planning/phases/03-git-adapter/03-CONTEXT.md` — git-layer boundaries; config and blob reads were
  deferred to this phase

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/git/repo.rs` `ref_storage`: the local-scope config read to copy for D-01,
  `config_snapshot().plumbing().string_filter(key, |meta| meta.source == gix::config::Source::Local)`.
  It returns a single value, so research must find the multi-value read that D-18 needs.
- `src/git/refs.rs` `peel_commit` and `peeled_commit`: peel an exact ref or object id to a commit
  (phase 03.1 replaced `Repo::resolve` and `Repo::resolve_commit`, which no longer exist).
- `src/git/mod.rs` `Error`: the hand-written enum, one variant per refusal, with no new error crate
  (Phase 3).

### Established Patterns
- gix types never leave `src/git/`: callers get wsx's own types, and paths are raw bytes.
- Reads go through gix and writes through the `git` binary, with differential tests against git for
  each read and a control that must change the answer. Amended by D-23: the model's inputs (refs,
  listings, manifest blob, config) are read through the `git` binary; gix reads objects by id.
- Tests are unit tests in the binary crate, built on real temporary repos, because there is no
  library crate.
- `src/git/mod.rs` carries a `dead_code` expectation until Phase 5, and new Phase 4 code has no
  production caller either.

### Integration Points
- Phase 5 `wsx check`, Phase 6 guard, Phase 9 cut and `status` all read this model.
- `src/git/` still needs a blob read at `<tip>:.wsx.conf`, a local-branch listing, and a way to tell
  a blob from a symlink or directory at that path.

</code_context>

<specifics>
## Specific Ideas

- **Git's actual behaviour, measured on git 2.56.0 (2026-11-01):**
  - `git remote add` plus `git fetch` creates `refs/remotes/origin/HEAD`. It stayed unset with
    `remote.origin.followRemoteHEAD=never` (the control).
  - Docs excerpts, not tested here: the default mode leaves an existing `<remote>/HEAD` alone, and
    fetches outside the configured refspecs don't set it.
  - **Correction on record:** the owner was first told this ref is commonly missing after a fetch,
    which is false on the git version wsx requires. They reconfirmed D-09 and D-10 with the corrected
    facts.
- **Quoting:** `git ls-files` printed `my dir` and a name with a trailing space unquoted, and quoted
  the non-ASCII name as `"caf\303\251"`. So D-19's quoting rule is stricter than git's own output, and
  `wsx init`/`wsx migrate` must quote edge whitespace themselves.
- **Config scope:** `git config --get` returned a global value that `git config --local --get` did
  not. Use this as the negative control for D-01.

</specifics>

<deferred>
## Deferred Ideas

- Exit codes and `--json` for every error here: Phase 5 (IF-01).
- The `refs/wsx/` record format and the real recorded set for D-17: Phase 8 (WTREE-05).
- The guard's behaviour on "not configured": Phase 6, reading the model's state.

### Reviewed Todos (not folded)
- **Re-check Dependabot after first merge to main:** repo tooling, matched on keywords only.
- **Replace oracle-observed exit 128 under IF-01:** needs Phase 5's exit codes and is applied in
  Phases 7 and 9.
- **Re-verify Phases 1-3 before milestone audit** and **Shell-quote the path in ForeignOwner's printed
  fix command:** reviewed 2026-11-17, not folded; neither touches the pivot.

</deferred>

---

*Phase: 04-repo-model*
*Context gathered: 2026-11-01*
