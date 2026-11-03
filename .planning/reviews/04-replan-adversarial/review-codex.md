## CONFIRMED

1. **Major — The symbolic-ref fix still permits a local workspace commit to become the integration base.**

   **Citations:** `.planning/phases/04-repo-model/04-07-PLAN.md:134`: “`RefTarget::Object(_)` is an `Integration` whose tip comes from `ref_commit`.”  
   `.planning/phases/04-repo-model/04-01-PLAN.md:174`: “otherwise `self.resolve_commit(full)`”.  
   This contradicts `04-09-PLAN.md:235`: “it reads each ref once”.

   **Evidence checked:** `src/git/repo.rs:162–187` resolves the name again and peels its current target. The new wrapper’s existence check does not prohibit following a symbolic target.

   **Reproduce:** Read a direct `refs/remotes/origin/develop`, then run:
   ```bash
   git symbolic-ref refs/remotes/origin/develop refs/heads/workspace/dev
   ```
   Continue through the prescribed `ref_commit`. With gix 0.90.0, the first lookup returned the remote commit; the second returned the distinct workspace commit. Without the update, it returned the remote commit.

   **What breaks:** D-06’s remote-only base restriction. This is an **unfixed part of the earlier review**: `04-REVIEWS.md:64` specifically recommended resolving the validated object instead of looking up the name again. The rewrite checks the name but still discards the validated object.

2. **Major — Lowercasing replacement names conceals duplicate replacements that make Git abort.**

   **Citation:** `.planning/phases/04-repo-model/04-11-PLAN.md:132`: “collects the lower-cased names after the base into a `BTreeSet`, because git and gix both accept uppercase hex”.

   **Evidence checked:** Executed the prescribed listing, set construction and target checks against gix 0.90.0 and Git 2.56.0.

   **Reproduce:** For manifest blob A and two different valid manifest blobs B and C:
   ```bash
   git update-ref refs/replace/<lowercase-A> <B>
   git update-ref refs/replace/<uppercase-A> <C>
   ```
   Neither target is itself replaced. The planned guard returned success; gix read B. Git exited **128** with `fatal: duplicate replace ref`. Removing the uppercase ref restored agreement.

   **What breaks:** The model accepts a manifest where Git refuses to read one. Case-insensitive object identity requires duplicate detection; collapsing names into a set removes exactly that evidence. The uppercase-chain tests do not exercise this collision.

3. **Major — The replacement guard validates live refs against an object reader using an older replacement map.**

   **Citations:** `.planning/phases/04-repo-model/04-11-PLAN.md:75`: “gix loads them at open”.  
   The same plan, line 132: “It lists the refs directly under the base”.

   **Evidence checked:** gix’s cached replacement-map construction in `gix-0.90.0/src/open/repository.rs:531–560`; wsx’s repository opening in `src/git/repo.rs:32–67`; the proposed guard and manifest-read calls.

   **Reproduce:** Install A→B→C, open the repository, then delete both replacement refs **before any manifest object read**. Run the planned guard and read through that existing handle.

   Measured:
   ```text
   guard: success
   existing gix handle: only intermediate
   git: only original
   freshly opened gix handle: only original
   ```

   **What breaks:** The earlier chained-replacement defect remains reachable through a controlled interleaving. Scanning an empty live namespace cannot certify the cached replacement map. The replacement tests create everything before discovery and therefore miss this mismatch.

4. **Major — A gix-only config key produces wrong manifest bytes with no chain, cycle or symbolic ref.**

   **Citation:** `.planning/phases/04-repo-model/04-11-PLAN.md:132`: “Its base is the string at `gitoxide.objects.replaceRefBase` from `self.repo.config_snapshot()`”.

   **Evidence checked:** gix’s replacement-base selection in `gix-0.90.0/src/open/repository.rs:668–687`, the proposed guard, and the manifest read path. `src/git/repo.rs:32–67` adds no normalization of this setting to Git’s behavior.

   **Reproduce:** With `GIT_REPLACE_REF_BASE` absent:
   ```bash
   git config --local gitoxide.objects.replaceRefBase refs/other/
   git update-ref refs/other/<manifest-blob-A> <manifest-blob-C>
   ```
   The planned guard accepted the single replacement. gix read `only secret`; Git read `only original`. Unsetting the config key restored agreement.

   **What breaks:** The guard validates gix’s namespace without establishing that Git uses that namespace. This violates the plan’s line-25 prohibition against reading different manifest bytes. Testing the environment-variable override does not test the reader-specific config key.

5. **Medium — The rewritten mutation gate can certify a survivor on the final source.**

   **Citation:** `.planning/phases/04-repo-model/04-10-PLAN.md:103`: “Re-run with `--iterate` added, which skips mutants already caught”. The same step permits production fixes between runs.

   **Evidence checked:** Reproduced with cargo-mutants **27.1.0**, nextest, the plan’s test filter, and its survivor predicate.

   **Reproduce:** Start with:
   ```rust
   pub fn bump(value: u32) -> u32 { value + 1 }
   // Test: bump(2) == 3
   ```
   All four generated mutants were caught. Change the implementation to `value + 2` and update the expected result to `4`, preserving source locations.

   Results:
   ```text
   --iterate: exit 0; zero mutants tested; four previously caught
   planned survivor gate: exit 0
   fresh run: exit 2; “replace + with * in bump” survived
   ```
   The cached count and current `--list` count both equalled four, so the proposed count audit also passed.

   **What breaks:** The final `missed.txt` does not establish mutation coverage of the final source. Line 190 acknowledges this limitation but leaves the acceptance gate unchanged. A fresh pass after production changes is required to support its claim.

## SUSPECTED

None retained. The concurrency findings use deterministic interleavings; their frequency during ordinary use was not measured.

## COVERAGE

- **Read in full:** All eleven plans, `04-CONTEXT.md`, `04-RESEARCH.md`, `04-VALIDATION.md`, `04-REVIEWS.md`, the Phase 4 roadmap section, `CLAUDE.md`, and `CONTRIBUTING.md`.
- **Read selectively:** Requirements covering configuration and downstream consumers; Rust discovery, resolution, reads and test isolation; frozen `bin/wsx` configuration/manifest/base resolution; harness fixtures and oracle setup; relevant script sections. `scripts/check.sh` and `scripts/check-in-container.sh` were read in full.
- **Verification limits:** Ran isolated reproductions of the prescribed algorithms and mutation gate. No completed Phase 4 model exists to exercise. The snapshot lacks Git metadata; no commit identity, full project gate or container gate was verified.
- **Cleanup:** Snapshot file bytes matched the pre-probe hashes. Review scratch was removed, probe processes exited, and no cargo-mutants temporary directories remained.
