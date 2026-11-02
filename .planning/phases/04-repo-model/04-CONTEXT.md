# Phase 4: Repo model - Context

**Gathered:** 2026-11-01
**Status:** Ready for planning

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
- `src/git/repo.rs` `Repo::resolve` and `Repo::resolve_commit`: revision resolution, `pub(super)`
  today.
- `src/git/mod.rs` `Error`: the hand-written enum, one variant per refusal, with no new error crate
  (Phase 3).

### Established Patterns
- gix types never leave `src/git/`: callers get wsx's own types, and paths are raw bytes.
- Reads go through gix and writes through the `git` binary, with differential tests against git for
  each read and a control that must change the answer.
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

</deferred>

---

*Phase: 04-repo-model*
*Context gathered: 2026-11-01*
