# Phase 4: Repo model - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-01
**Phase:** 04-repo-model
**Areas discussed:** Config keys & sources, Integration-branch chain, Manifest grammar edges, Workspace-branch fallback

---

## Config keys & sources

| Option | Description | Selected |
|--------|-------------|----------|
| Local repo config only | Per-repo; a global value can't reconfigure unrelated repos | ✓ |
| Local for sourceBranch; all scopes for the rest | One global default for baseRef and remotes | |
| All scopes for everything | Simplest, but a global sourceBranch configures every repo | |

**User's choice:** Local repo config only

| Option | Description | Selected |
|--------|-------------|----------|
| `wsx.baseRemote`/`wsx.pushRemote`; missing remote is an error | Explicit config for forks | ✓ |
| Same keys; auto-detect `upstream` | Zero config for forks, but a silent guess | |
| Different names | Owner names them | |
| Reuse `remote.pushDefault` for the push remote | One fewer key, coupled to a global git setting | |

**User's choice:** Own keys, after asking for the reasoning and then a comparison with
`remote.pushDefault`.
**Notes:** Default `origin` only when unset. No `origin` and no key means no remote.

---

## Integration-branch chain

| Option | Description | Selected |
|--------|-------------|----------|
| Base remote's tracking refs only | Local branches only when there is no remote | ✓ |
| Tracking refs first, then local branches | Also finds an unpushed local develop | |
| Local branches first | Follows checkouts, but a stale branch becomes the base | |

**User's choice:** Base remote's tracking refs only

| Option | Description | Selected |
|--------|-------------|----------|
| Bare branch name, on the base remote | One source of truth for the remote | ✓ |
| Remote-qualified, like 0.1.0 | States the remote twice | |
| Accept both | Meaning depends on which refs exist | |

**User's choice:** Bare branch name

| Option | Description | Selected |
|--------|-------------|----------|
| Error listing what was tried, no guessing | No network; never main or master | ✓ |
| Query the remote for its default | Network in a read that must work offline | |
| Fall back to main, then master | A guess about the base | |

**User's choice:** Error listing what was tried
**Notes:** The question first claimed `<remote>/HEAD` is commonly missing after `git remote add` plus
fetch. A later probe on git 2.56.0 showed fetch creates it. The owner was told, and reconfirmed the
answer with the corrected facts.

---

## Manifest grammar edges

| Option | Description | Selected |
|--------|-------------|----------|
| Allow full-line comments and blank lines | No trailing comments | ✓ |
| Strict: declarations only | Nowhere to record why | |
| Also allow trailing comments | A typo can change a path silently | |

**User's choice:** Full-line comments and blank lines

| Option | Description | Selected |
|--------|-------------|----------|
| Only cross-kind overlaps are invalid | Duplicates and same-kind nesting parse | ✓ |
| Also reject any redundancy | A harmless duplicate blocks every push | |
| Reject only exact same-path conflicts | Adds a longest-match rule | |

**User's choice:** Only cross-kind overlaps

| Option | Description | Selected |
|--------|-------------|----------|
| Literal paths; strip a trailing `/`; reject the rest | Unmatchable declarations are errors | ✓ |
| Same, but reject a trailing `/` too | One spelling, but blocks pushes over `.planning/` | |
| Normalize instead of rejecting | The file differs from what wsx acts on | |

**User's choice:** Literal paths, strip a trailing `/`, reject the rest

---

## Workspace-branch fallback

| Option | Description | Selected |
|--------|-------------|----------|
| Names the branch, repo stays not configured | Guard exits 0 | ✓ |
| A fallback match counts as configured | The hook could block unrelated repos | |
| Fallback only for read-only reporting | Conflicts with criterion 3's wording | |

**User's choice:** Names the branch, repo stays not configured

| Option | Description | Selected |
|--------|-------------|----------|
| Takes the recorded set as an input, Phase 8 supplies it | Exclusion complete only at Phase 8 | ✓ |
| Define the record format and reader now | Locks a format early | |
| Move the exclusion to Phase 8 entirely | CONF-05 traceability needs a partial note | |

**User's choice:** Input now, Phase 8 supplies it

---

## Claude's Discretion

Stated to the owner before they moved on; see D-18 to D-21 in CONTEXT.md.
- Empty or repeated `wsx.*` keys are errors.
- Manifest line edges: rest of line is the path, edge whitespace must be quoted, CR is an error, an
  empty manifest is valid, a non-blob is invalid.
- Phase 4 reads `wsx.worktreeDir` for the built-in list.
- Fallback edges: candidate listing, local refs only, short-name `wsx.sourceBranch`.

## Deferred Ideas

- Two pending todos were reviewed and not folded: Dependabot re-check, and oracle-observed exit 128.

## Process notes

- `workflow.research_before_questions` is on, but the search ran only after the questions, so answers
  were given without it. Apart from the correction above, it changed nothing.
- The todo match, the MemPalace recall and the spike check also ran late, after the four areas.


---

# Update 2026-11-17: model inputs through the git binary

Recorded the operator's pivot of 2026-11-17 (`.planning/reviews/04-13-adversarial/round2/SUMMARY.md`) as D-23.

## Areas presented
| Area | Question |
|---|---|
| Replace-ref policy | Retire D-22, keep it, or move it to gix object reads? |
| Fate of 04-11/04-12 guards | Delete or keep as backstop? |
| Spawn budget & batching | Children per resolve and latency ceiling |
| Test model after pivot | What proves parity once git is the reader |

**User's choice:** "you decide on all of these". Claude's calls are D-24 to D-28 in CONTEXT.md: keep D-22 unchanged
(it now guards gix object reads), delete parity-only code and keep the refusals that state a product rule, at most 4 batched
children with p95 under 100 ms (provisional), D-01 to D-21 tests unchanged plus every review shape as a git-first
regression test, then spike before the Tier A replan.

## Todos
Two new keyword matches (re-verify phases 1-3, ForeignOwner quoting) were reviewed and not folded (operator's choice).

## Process notes
- MemPalace recall (MCP) returned only the 04-11/04-12 plan and summary drawers already read; nothing new.
- Phase 3's CONTEXT.md was not edited; D-23 records the amendment of its "reads through gix" here.
