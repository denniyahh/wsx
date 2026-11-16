## CONFIRMED

References: [04-12-PLAN.md](.planning/phases/04-repo-model/04-12-PLAN.md), [04-13-PLAN.md](.planning/phases/04-repo-model/04-13-PLAN.md). Statuses below assess the revised instructions.

Probes used Git **2.56.1**, gix **0.90.0**, and isolated scratch repositories. `A` was the committed manifest blob containing `only a\n`; `B` contained `only b\n`.

1. **Critical — a falsely “sorted” packed-refs file passes the proposed guard while Git aborts.**

   `04-12-PLAN.md:190–193` trusts gix’s prefixed listing; `04-13-PLAN.md:175–176` adds only a loose-file walk. Neither validates packed-record ordering.

   I wrote this `.git/packed-refs`:

   ```text
   # pack-refs with: peeled fully-peeled sorted
   <B> refs/replace/<A>
   <A> refs/heads/z
   ```

   Running `git cat-file blob main:.wsx.conf` terminated with SIGABRT and printed:

   ```text
   BUG: refs/packed-backend.c:1020: packed-refs backend yielded reference preceding its prefix
   ```

   Gix listed the direct replacement, read `only b\n`, and the snapshot model resolved `Configured`. That single valid, direct replacement passes the proposed name, duplicate, chain and filesystem checks.

   **Control:** reversing the records into lexical order made Git print `only b\n`; gix and the model retained that result. This is a stationary case where wsx reads while Git produces no manifest bytes.

2. **Major — invalid packed replacement names still blame `.wsx.conf`, expressly preserving the wrong classification.**

   `04-13-PLAN.md:105` explicitly leaves an invalid packed replacement as `Error::Gix`. `04-12-PLAN.md:239` explicitly maps that guard error to `ManifestUnreadable`.

   With the sorted header above and this sole record:

   ```text
   <B> refs/replace/a..b/<A>
   ```

   `git cat-file blob main:.wsx.conf` exited 128:

   ```text
   fatal: git cat-file main:.wsx.conf: bad file
   ```

   Gix’s iterator reported `Invalid packed reference`. The snapshot model rendered:

   ```text
   .wsx.conf at the tip of main cannot be read: list the refs directly under refs/replace/: Invalid packed reference, …
   ```

   The revisions preserve that manifest-blaming wrapper; only the listing description changes. This violates the requested D-22 classification.

   **Control:** changing the packed name to `refs/replace/<A>` made Git and the model read `only b\n`.

3. **Critical — the guard still validates a different replacement state from the map used to read.**

   The listing in `04-12-PLAN.md:190–192` and disk walk in `04-13-PLAN.md:175–176` inspect current refs; neither validates gix’s cached map.

   I ran this deterministic sequence:

   ```text
   git replace A B
   Repo::discover(...)
   remove .git/refs/replace/A
   model::resolve(the_same_repo, ...)
   git cat-file blob main:.wsx.conf
   ```

   The replacement directory was empty. The held repository resolved `Configured` with personal entry `b`; the subsequent Git command exited 0 and printed `only a\n`. Both proposed guard checks see an empty replacement set and admit this state.

   **Control:** leaving the replacement installed makes both readers return `only b\n`; reopening after removal makes both return `only a\n`.

   This is an **inherited, previously accepted exception**, recorded at `src/model/mod.rs:350–355` and `04-11-PLAN.md:184`, rather than a newly introduced race. It nevertheless defeats the absolute rule posed in this review.

4. **Minor — R6’s replacement-message correction remains false when replacements are disabled.**

   `04-12-PLAN.md:180` requires the message to say Git “applies it, or stops with ‘duplicate replace ref’.” The refusal at `04-12-PLAN.md:192` is unconditional.

   For `refs/replace/team/<A> → B`, I ran:

   ```text
   git cat-file blob main:.wsx.conf
       → exit 0, only b
   git -c core.useReplaceRefs=false cat-file blob main:.wsx.conf
       → exit 0, only a
   GIT_NO_REPLACE_OBJECTS=1 git cat-file blob main:.wsx.conf
       → exit 0, only a
   ```

   The proposed guard refuses all three cases with the same message. In the disabled cases Git neither applies the replacement nor stops on a duplicate. Over-refusal is permitted; the asserted explanation is still false.

Other requested probes produced these specific results:

| Case and command | Observed result |
|---|---|
| Valid packed replacement with an added valid `^<C>` peeled line; `git cat-file blob main:.wsx.conf` | Git and gix both read `only b\n`; the peeled value did not redirect the replacement. |
| `git symbolic-ref refs/replace/<A> refs/targets/b` | Git read `b`, gix read `a`; the existing guard returned `ReplacementRef`. |
| Direct replacement read from a linked worktree | Git and gix both read `b` from the common ref store. |
| Linked-worktree base `refs/worktree/replacements/` | Direct replacement: both read `b`; symbolic replacement: existing refusal; filesystem symlink: Git read `b`, gix read `a`, covered by the proposed git-dir walk at `04-13-PLAN.md:111`. |
| `git init --ref-format=reftable`, then replacement | Git read `b`; wsx’s existing discovery guard refused reftable storage. |
| Explicit `GIT_COMMON_DIR` | Existing discovery guard refused it. |
| Replacement target available through `objects/info/alternates` | Git and gix read the alternate blob. |
| Target supplied through `GIT_ALTERNATE_OBJECT_DIRECTORIES` | Existing discovery guard refused the environment override. |

## SUSPECTED

No additional concrete defect is asserted for case-insensitive filesystems: the scratch filesystem was case-sensitive.

Unicode normalization does expose another instance of finding 2’s classification problem: with `core.precomposeUnicode=true` and an NFD base `refs/replace\u0301/`, Git printed `only b\n`, while gix failed to read the normalized NFC path and the model blamed `.wsx.conf`. Setting the option to `false` made both readers return `b`. This does not establish behavior on a case-insensitive filesystem.

## R1–R13 status

| Finding | Status in revised plans | Plan evidence |
|---|---|---|
| R1 | **Closed for the original symlink cases.** Direct symlink, symlinked directory and linked-worktree tests; walk covers common and git dirs. | `04-13-PLAN.md:158–162,175–176` |
| R2 | **Closed for the slashless base.** Explicit refusal, Git premise and positive/negative base cases. | `04-12-PLAN.md:227,230,238` |
| R3 | **Closed.** Guard moves ahead of integration and workspace object reads, with both failing-direction tests. | `04-12-PLAN.md:225–226,235–239` |
| R4 | **Closed.** Full suffix/nesting key rule belongs to the first GREEN commit. | `04-12-PLAN.md:189–193` |
| R5 | **Closed.** Claims are narrowed to name refusal; individual RED evidence is required, and the permissive control gets a targeted mutant. | `04-12-PLAN.md:25,187,197,277` |
| R6 | **Not closed in full.** Duplicate handling is corrected, but disabled replacements still falsify the mandated wording; finding 4. | `04-12-PLAN.md:180,185,192` |
| R7 | **Closed.** Documentation states gix accepts either hash length; SHA-256 test includes a direct-replacement control. | `04-12-PLAN.md:179,185,193` |
| R8 | **Closed.** Both plans require their new variants in the phase-5 obligation. | `04-12-PLAN.md:272`; `04-13-PLAN.md:210` |
| R9 | **Closed.** Identity comparison replaces counts; timeouts require disposition; doctored outputs must fail. | `04-13-PLAN.md:212–214,221–224` |
| R10 | **Closed.** Coverage execution and modified-file measurements are restored. | `04-13-PLAN.md:215,225` |
| R11 | **Closed.** Dependency added; six mutation files declared; separate output and allowlist. | `04-12-PLAN.md:6`; `04-13-PLAN.md:7–14,211` |
| R12 | **Closed.** Both widenings are recorded while CONTEXT remains unchanged. | `04-12-PLAN.md:105`; `04-13-PLAN.md:99` |
| R13 | **Closed.** Quoting discrepancy is explicitly assigned to WR-02. | `04-12-PLAN.md:124` |

All original snapshot files remained byte-identical. Probe scratch was deleted and probe processes exited; an unattributed `target/` directory that appeared during review was preserved.


