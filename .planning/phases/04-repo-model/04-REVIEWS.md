---
phase: 4
reviewers: [codex, chatgpt]
reviewed_at: 2026-11-02T15:29:21Z
plans_reviewed: [04-01-PLAN.md, 04-02-PLAN.md, 04-03-PLAN.md, 04-04-PLAN.md, 04-05-PLAN.md, 04-06-PLAN.md, 04-07-PLAN.md, 04-08-PLAN.md, 04-09-PLAN.md, 04-10-PLAN.md]
models:
  codex: "gpt-6.1-sol (reasoning=high)"
  chatgpt: "gpt-6-astra (reasoning=high)"
model_sources:
  codex: "banner"
  chatgpt: "banner"
---

# Cross-AI Plan Review — Phase 4

Lens: adversarial. Snapshot: 50153d4. Run through the external-review skill, not `/gsd-review`.

<!-- gsd:plan-revision-conflicts:begin -->
## Plan-Revision Conflicts
<!-- gsd:plan-revision-conflicts:end -->

## Codex Review

## CONFIRMED

1. **Major — Chained replacement refs produce the wrong manifest bytes.**

   **Citations:** `.planning/phases/04-repo-model/04-RESEARCH.md:482`: “gix and git both read the replacement. Same answer, nothing to handle.” This becomes an untested assumption in `.planning/phases/04-repo-model/04-09-PLAN.md:95`: “gix and git read the replacement alike; not tested”.

   **What breaks:** The prescribed blob read can construct a different personal-path list from Git’s view of the same manifest.

   **Reproduction and verification:** Using Git 2.56.0 and the pinned gix 0.90.0, I created blobs A=`only original`, B=`only intermediate`, C=`only secret`, then installed replacements A→B and B→C. `git cat-file blob A` returned `only secret`; the planned `repo.find_blob(A)` call returned `only intermediate`. With no replacement and with only A→B, both agreed.

   **Why planned checks miss it:** None exercises a replacement chain. All three manifests parse successfully.

   **Smallest fix:** Refuse unsupported replacement chains before resolving the model, or implement Git-equivalent bounded replacement traversal in the git layer. Add no-replacement, single-replacement, chained-replacement and cycle tests.

2. **Major — The remote-prefix check can be bypassed through another symbolic ref.**

   **Citations:** `.planning/phases/04-repo-model/04-07-PLAN.md:125`: “`target` must start with the prefix, and the target must exist.” `.planning/phases/04-repo-model/04-RESEARCH.md:366` calls a symref to another symref “hand-made, harmless”.

   **What breaks:** Checking the first target’s prefix does not establish where commit resolution ends. This violates `.planning/phases/04-repo-model/04-CONTEXT.md:40`: “Local branches count only when there is no base remote”.

   **Reproduction and verification:** I created this chain using `git symbolic-ref`:

   `refs/remotes/origin/HEAD → refs/remotes/origin/alias → refs/heads/side`

   The first target passed the prescribed prefix and existence checks. The resolver returned the local `side` commit, while the model would report integration branch `alias`. A direct tracking-ref control resolved the expected remote commit.

   **Why planned checks miss it:** The outside-remote test covers a direct outside target. The indirect case is explicitly left untested at `04-07-PLAN.md:202`.

   **Smallest fix:** Validate every symbolic hop, with bounded traversal and cycle detection, before peeling the final object. Test both an entirely remote chain and a chain escaping into `refs/heads/`.

3. **Major — Constructing a `FullName` does not make this lookup exact.**

   **Citation:** `.planning/phases/04-repo-model/04-01-PLAN.md:170`: “build `gix::refs::FullName::try_from` the full name, `self.repo.try_find_reference(name.as_ref())`, map `target()` … `ref_commit` is `self.resolve_commit(full)`”.

   **What breaks:** The proposed wrapper discards the returned reference’s name without checking it. Both proposed reads can resolve a differently named loose reference.

   **Reproduction and verification:** With `refs/remotes/origin/ghost` absent, I placed a valid commit ID in the crafted loose file `.git/heads/refs/remotes/origin/ghost`. The prescribed gix lookup returned a reference named **`heads/refs/remotes/origin/ghost`**, and revision resolution returned its commit. `git show-ref --verify refs/remotes/origin/ghost` exited 128. Removing the crafted file changed gix’s answer to `None`.

   **Why planned checks miss it:** `04-07-PLAN.md:27` plants `refs/heads/refs/remotes/origin/ghost`. That exercises a different path from the fallback gix actually probes.

   **Smallest fix:** Require the returned reference name to equal the requested full name. Resolve its validated target object rather than passing the name through another revision lookup. Add the crafted loose-file case as a negative control.

4. **Major — A compile failure satisfies the quoting negative control.**

   **Citation:** `.planning/phases/04-repo-model/04-03-PLAN.md:173`: `test "$rc" -ne 0 && test "$rc" -ne 124 && rg -qF 'any_git_path_round_trips' <<<"$out"`.

   **What breaks:** This accepts any non-timeout failure containing the test’s name. It does not establish that the property executed or caught missing edge-whitespace quoting.

   **Reproduction and verification:** I ran a minimal scratch crate through `cargo nextest`: the baseline test passed. I then introduced an undefined function inside `any_git_path_round_trips`. Nextest exited 101; the compiler diagnostic included that function’s name. The plan’s acceptance predicate reported success although no test ran.

   **Contradiction:** `CONTRIBUTING.md:162` requires failure for the intended reason, explicitly excluding “a setup error, a compile failure, or an unrelated panic”.

   **Smallest fix:** Require a structured result showing this exact test executed and failed its property assertion, including the counterexample. Reject build and setup failures separately. Apply the same distinction to `04-01-PLAN.md:222`.

5. **Minor — The survivor check reports success when ripgrep cannot read the allowlist.**

   **Citation:** `.planning/phases/04-repo-model/04-10-PLAN.md:98`: `touch "$A"; if rg -vxF -f "$A" "$M"; then … exit 1; fi`.

   **What breaks:** Ripgrep exit 2 is treated like exit 1. An unreadable comparison becomes “no unexplained survivors”.

   **Reproduction and verification:** I ran the exact control flow with scratch paths. `missed.txt` contained an unexplained mutant; the allowlist path was a directory. Ripgrep printed `Is a directory` and the block exited **0**. Controls behaved differently: an empty regular allowlist exited 1; a regular allowlist containing the mutant exited 0.

   **Smallest fix:** Validate the allowlist as a readable regular file, check its creation, and distinguish ripgrep’s three outcomes explicitly: matches, no matches, execution error.

## SUSPECTED

None retained. The five findings above were reproduced against the relevant primitives or acceptance predicates.

## RESIDUAL RISKS AND TEST GAPS

1. **Read consistency remains unproved.** `04-09-PLAN.md:216` tests “resolve, commit a changed manifest” sequentially. That does not exercise updates *during* resolution. The repository-byte comparison proves neither an atomic view across config and refs nor safe downstream use after those refs move.

2. **These were primitive-level reproductions.** Phase 4 has no implementation. I did not run a completed model, command-level behavior, the full repository gate, or the container gate. The supplied snapshot has no Git metadata. The probes establish the documented algorithms’ defects, not an existing production exploit.

3. No additional confirmed scope leak was found. Probe processes finished, and the probe directory and build artifacts were removed; cleanup was checked.

## COVERAGE

1. **Read in full:** `04-01-PLAN.md` through `04-10-PLAN.md`, `04-CONTEXT.md`, `04-RESEARCH.md`, `04-PATTERNS.md`, `Cargo.toml`, `CLAUDE.md`, `CONTRIBUTING.md`, `src/main.rs`, `scripts/check.sh`, `scripts/workspace/cut-pr-branch.sh`, `.workspace-divergence`.

2. **Read in part:** `.planning/REQUIREMENTS.md` around the configuration requirements and downstream contracts; `.planning/ROADMAP.md` Phase 4; all five `src/git/*.rs` files, focusing on discovery, resolution, reads and test isolation. Also inspected relevant harness fixture code, container configuration, scanner configuration, lockfile metadata, and cached gix dependency source.

3. **Not read in full:** The remaining requirements and roadmap sections, complete git-adapter test bodies, and the complete harness suite. No listed primary target was left unread.

---

## ChatGPT Review

## CONFIRMED

1. **Major — The remote-prefix check permits selecting a local workspace commit as the integration base.**

   **Citations:** `.planning/phases/04-repo-model/04-07-PLAN.md:125`: “`target` must start with the prefix, and the target must exist.” `.planning/phases/04-repo-model/04-RESEARCH.md:366`: “Symref to another symref under the remote … Accepted as branch `alias`; hand-made, harmless.”

   **What breaks:** Checking the first symbolic target does not constrain the final target. With no remote `develop` or `next`, this chain passes the planned checks:

   ```text
   refs/remotes/origin/HEAD
     → refs/remotes/origin/alias
     → refs/heads/workspace/dev
   ```

   The integration tip becomes the workspace tip. This contradicts `04-CONTEXT.md:37`: “The chain reads the base remote's tracking refs only.” Later comparisons or cuts can consequently use the wrong history boundary.

   **Verified:** Created this chain with Git 2.56.0 and distinct remote/workspace commits. The prefix check passed while resolution returned the workspace commit. Redirecting the alias to `origin/main` changed the result. Inspected `src/git/repo.rs:162–183` and gix’s reference-peeling implementation; the planned `ref_commit` delegates to that unrestricted resolution. Phase 4 itself is not implemented.

   **Smallest fix:** Reject symbolic branch targets after `remote/HEAD`, or validate every symbolic hop before resolving the final object. Test both an escaping chain and an ordinary remote default.

2. **Major — The encoder negative control accepts compiler failures as evidence that the property caught a defect.**

   **Citation:** `.planning/phases/04-repo-model/04-03-PLAN.md:173`:

   ```bash
   test "$rc" -ne 0 && test "$rc" -ne 124 &&
   rg -qF 'any_git_path_round_trips' <<<"$out"
   ```

   **What breaks:** A compiler diagnostic containing the function name satisfies this predicate. No test needs to execute. This contradicts the same plan’s line 167, “A failure is a printed minimal input,” and `CONTRIBUTING.md:162–163`, which explicitly rejects compile failures as evidence.

   **Verified:** Ran nextest against a scratch test named `any_git_path_round_trips` containing a type error. Nextest exited 101, the compiler printed that name, and the planned predicate accepted it. Replacing the body with a passing assertion produced exit 0, which the predicate rejected.

   **Smallest fix:** Require a fresh structured report identifying the selected test as executed and failed on its assertion. Reject build, setup, selection and timeout failures separately. Apply the same discrimination to the analogous control in `04-01-PLAN.md:222`.

3. **Minor — An unreadable inferred manifest escapes the required “not configured” diagnostic.**

   **Citations:** `.planning/phases/04-repo-model/04-09-PLAN.md:127` specifies a missing blob as `Err(Error::Git(_))`. `.planning/phases/04-repo-model/04-08-PLAN.md:175` wraps only `ManifestMissing`, `ManifestNotABlob` and `ManifestInvalid`; “every other error passes through unchanged.”

   **What breaks:** Remove the manifest’s blob object while leaving its tree entry, with one candidate branch and no `wsx.sourceBranch`. `resolve` retains `NotConfigured`, but `require()` returns the generic Git error without the required configuration explanation. `04-CONTEXT.md:69–70` says an unreadable inferred manifest cannot be used and a missing or invalid manifest reads as “not configured, run `wsx init`.”

   **Verified:** Traced the explicitly prescribed error mapping across both plans. The missing-object test covers only the configured case. This is a source-confirmed contract gap, not an executed Phase 4 failure. It does **not** demonstrate a push-guard bypass.

   **Smallest fix:** Preserve manifest-read provenance with a typed unreadable-manifest error, then wrap it for inferred models. Exercise the missing-blob fixture with the configuration key both absent and present.

4. **Minor — The mutation-survivor gate succeeds when its comparison fails.**

   **Citation:** `.planning/phases/04-repo-model/04-10-PLAN.md:98`:

   ```bash
   touch "$A";
   if rg -vxF -f "$A" "$M"; then
       echo "unexplained survivors above"; exit 1;
   fi
   ```

   **What breaks:** `rg` exit 1 means no unmatched survivors; exit 2 means an error. Both take the successful path. Failure to read the evidence can therefore certify unexplained survivors.

   **Verified:** Executed this predicate with a real survivor and an empty allowlist: exit 1. Replaced the allowlist file with a directory: `rg` reported “Is a directory,” but the gate exited 0 with the survivor unchanged.

   **Smallest fix:** Check allowlist creation/readability and handle `rg` statuses explicitly: only status 1 passes; statuses 0 and 2 fail. Keep the unreadable-allowlist case as a negative control.

5. **Minor — A host property-test pass can mean zero generated cases.**

   **Citation:** `.planning/phases/04-repo-model/04-03-PLAN.md:161`: “Do not assert a case count: `PROPTEST_CASES` overrides it.”

   **What breaks:** The plan acknowledges an override but leaves zero cases acceptable. With `PROPTEST_CASES=0`, nextest/libtest can report the named property passed without evaluating any path. `--no-tests=fail` cannot detect this: the test function exists and runs.

   **Verified:** Used proptest **1.12.0**, the specified features and `cases: 2048` configuration, with an always-failing property. `PROPTEST_CASES=0` reported one passed test; `PROPTEST_CASES=1` produced the planted assertion failure.

   **Smallest fix:** Enforce a minimum effective case count or run the properties in a subprocess with an explicit configuration environment. This finding invalidates the host pass as property evidence under that environment; it does not establish that the container run or deterministic tests are bypassed.

## SUSPECTED

None retained beyond the confirmed findings and test gaps below.

## RESIDUAL RISKS AND TEST GAPS

6. **The concurrency evidence establishes neither a consistent multi-ref snapshot nor behavior during updates.**

   `.planning/phases/04-repo-model/04-09-PLAN.md:216` tests “resolve, commit a changed manifest … a fresh `Repo` resolves the new ones.” Line 217 compares repository files before and after resolution.

   These are sequential checks. They cannot detect a fetch or branch update between candidate selection, integration resolution and workspace-tip resolution. `.planning/phases/04-repo-model/04-RESEARCH.md:639–640` explicitly says config is frozen while refs are not.

   No harmful interleaving was reproduced here. Settle the risk with controlled updates between reads and an explicit consistency contract. Retaining one manifest commit ID alone does not establish atomicity of the whole model.

7. **Supported-version behavior remains untested at the minimum Git version.**

   `.planning/phases/04-repo-model/04-10-PLAN.md:177` explicitly states that “no real git at the 2.54 floor runs in this phase.” Host/container agreement on 2.56.0 cannot establish compatibility with 2.54. This review’s Git probes also used 2.56.0.

## COVERAGE

- **Read in full:** All ten `04-01-PLAN.md` through `04-10-PLAN.md`; `04-CONTEXT.md`, `04-RESEARCH.md`, `04-PATTERNS.md`; `Cargo.toml`, `CLAUDE.md`, `CONTRIBUTING.md`, `scripts/check.sh`, `scripts/workspace/cut-pr-branch.sh`, `.workspace-divergence`, and `src/main.rs`.
- **Read in part:** `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`; all five `src/git/` files; supporting harness fixture code. Also inspected relevant installed gix/proptest source and read `scripts/check-in-container.sh` and nextest configuration.
- **Not read in full:** The remaining requirements/roadmap sections, git-layer implementations/tests outside the inspected ranges, and the rest of the harness.
- **Execution limits:** Ran isolated Git, nextest, proptest and shell-predicate probes. Did not run the project’s full gate or container gate. The supplied archive has no `.git`; its claimed commit identity remains unverified.
- **Scope:** I found no confirmed scope leak or inevitable later-plan invalidation of an earlier test.
- **Cleanup:** All probe commands finished; verified that all three sets of temporary probe directories were removed.

---

## Consensus Summary

Both lanes are OpenAI models on one login, so agreement between them is weaker evidence than
cross-vendor agreement. The "ChatGPT" lane is the Codex engine bundled in the ChatGPT desktop app,
not chatgpt.com. Each verdict below comes from a separate re-check by the driving agent on
gix 0.90.0, proptest 1.12.0 and git 2.56.0. These are probes of the primitives the plans prescribe:
Phase 4 is not implemented, so no model code was run, and passing a fix against these probes would
not show the model as a whole is right.

| # | Finding | Raised by | Re-check | Severity |
|---|---------|-----------|----------|----------|
| 1 | A symbolic ref can make the integration base a local branch, against D-06 | both | Reproduced | major |
| 2 | gix follows one replacement hop where git follows the chain, so the manifest bytes differ | Codex | Reproduced | major |
| 3 | `PROPTEST_CASES=0` passes every property test with no case run | ChatGPT | Reproduced | minor |
| 4 | The mutation survivor gate passes when `rg` cannot read the allowlist | both | Reproduced | minor |
| 5 | `find_ref` can return a differently named ref from `.git/heads/` | Codex | Reproduced; downgraded from major | minor |
| 6 | The encoder control accepts a failure that is not a property failure | both | Narrower than claimed; downgraded from major | minor |
| 7 | An unreadable blob on an inferred branch is not reported as "not configured" | ChatGPT | Traced in the plan text, not run | minor |

### Agreed Strengths
Neither lane was asked for strengths. Both reported no confirmed scope leak and no later plan that
invalidates an earlier plan's tests. That is an absence of findings, not proof.

### Agreed Concerns

**1. Symbolic ref escape (`04-07-PLAN.md:125`, `04-RESEARCH.md:366`).** With
`refs/remotes/origin/HEAD -> refs/remotes/origin/alias -> refs/heads/side`, the planned check sees a
target under the remote, and `rev_parse` then returns the local `side` commit. The control, a direct
tracking ref, returned the remote commit. Research calls this shape "hand-made, harmless", and the
plan leaves it untested. It needs a hand-made symref, but its effect is a wrong base for later
comparisons and cuts.
Re-check addition, not raised by either lane: the planned `found` closure only asks that the ref
exists, then calls `ref_commit`. So a symbolic `refs/remotes/origin/develop`, `next` or `wsx.baseRef`
target reaches the same escape. This is inferred from the same probe and the code at
`04-RESEARCH.md:720-760`.
Fix: in `found`, require the ref under the prefix to be a direct `RefTarget::Object`, on every rung.
Test an escaping chain on the HEAD rung and on a `develop` rung, plus an ordinary `origin/HEAD`.

**4. Mutation gate (`04-10-PLAN.md:98`).** With a directory as the allowlist, `rg` printed
`Is a directory` and the block exited 0 with the survivor unexplained. An empty allowlist exited 1
and a covering allowlist exited 0, as they should. Fix: check `rg`'s status, passing only on 1.

**6. Encoder control (`04-03-PLAN.md:173`, `04-01-PLAN.md:222`).** Both lanes showed a compile error
whose diagnostic prints the test's name satisfying the predicate. The plan's actual mutation does
not reach that: a compile error in `encode_path` printed no test name and the predicate rejected it.
So did a compile error inside the proptest body in the re-check's shape. What remains is that any
failure naming the test passes, including a panic unrelated to quoting. Fix: also require the
property's failure text, such as `minimal failing input`, in the output.

**Residual, both lanes.** `04-09-PLAN.md:216-217` tests only sequential reads, so a ref update during
resolution is untested. Neither lane reproduced a harmful interleaving. `04-10-PLAN.md:177` already
states that the git 2.54 floor is not exercised.

### Divergent Views
Raised by one lane, not contradicted by the other.

**2. Chained replacement (`04-RESEARCH.md:482`: "Same answer, nothing to handle").** That line holds
for one replacement and is false for a chain. With A->B and B->C, git reads C and gix reads B. With a
cycle, git fails with `replace depth too high` while gix still reads B. Controls with no replacement
and with one replacement agreed. A wrong manifest is a wrong personal-path list, with no error. It
needs a user-made replacement chain. Only blob reads were probed; commit and tree reads are untested.
Fix: refuse when the replacement object itself has a replacement, correct the research line, and test
none, single, chain and cycle.

**3. Zero proptest cases (`04-03-PLAN.md:161`).** With the plan's settings
(`cases: 2048, failure_persistence: None`, features `std`) and a property that always fails, unset
and `PROPTEST_CASES=1` failed and `PROPTEST_CASES=0` reported 1 passed. No script, nextest config or
workflow sets the variable, so this needs a polluted shell. An earlier re-check run contradicted this
only because default persistence replayed a saved failure. Fix: clear `PROPTEST_CASES` in
`scripts/check.sh`. Whether the container forwards it is unchecked.

**5. Inexact lookup (`04-01-PLAN.md:170`).** A commit id in `.git/heads/refs/remotes/origin/ghost`
made gix return `heads/refs/remotes/origin/ghost`, and `rev_parse` resolve it, while `git show-ref
--verify` exited 128. Downgraded from major: no git command creates that path. A real branch or tag
with that name, and a file at `.git/refs/refs/...`, all returned `None`. The plan's
`full_name_read_is_exact` tests a path gix does not confuse, so it stays green. Fix: compare the
returned name with the requested one in `find_ref`.

**7. Inferred manifest with a missing blob (`04-08-PLAN.md:175`, `04-09-PLAN.md:127`).** `require`
wraps only the missing, not-a-blob and invalid errors, so a missing object surfaces as `Error::Git`.
D-16 says an inferred manifest is used "if it's readable", and names "missing or invalid" as the
"not configured" cases. It still refuses, so nothing fails open, and the lane says it found no guard
bypass. Proposed disposition: keep the behaviour and record it in the plan, because a corrupt object
store is not a missing manifest and the git error is the more accurate message.
