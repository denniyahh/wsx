## CONFIRMED

1. **Critical — The proposed listing misses filesystem symlinks that Git follows.** [04-12-PLAN.md:156](.planning/phases/04-repo-model/04-12-PLAN.md) promises “every ref” through gix’s `references().prefixed()`. However, `gix-ref-0.69.1/src/store/file/loose/iter.rs:47–48` skips entries whose filesystem type is not a regular file.

   In a scratch SHA-1 repository, with A containing `only a\n` and B containing `only b\n`, I ran the equivalent of:

   ```sh
   printf '%s\n' "$B" > "$scratch/target"
   mkdir -p .git/refs/replace
   ln -s "$scratch/target" ".git/refs/replace/$A"
   git cat-file blob "$A"
   ```

   Git 2.56.1 printed `only b`; a gix 0.90.0 probe printed `only a`, and the proposed listing/parser produced no refusal. A symlinked directory containing the replacement reproduced the same mismatch. Replacing the symlink with a regular file made both readers print `only b`.

   This survives the planned fix: neither its new check nor the existing loops sees the omitted ref. Add filesystem-symlink controls before claiming “a replace ref git applies and gix ignores never yields a model” at plan line 290.

2. **Major — Equal bases can still produce a gix-only replacement that the rule admits.** [04-12-PLAN.md:158](.planning/phases/04-repo-model/04-12-PLAN.md) refuses only when the parsed key exists and the stripped name’s length differs from `hex_len`. That assumes stripping the base preserves Git’s last component.

   I planted `refs/replace/pre<A> -> B`, then ran:

   ```sh
   GIT_REPLACE_REF_BASE=refs/replace/pre git cat-file blob "$A"
   ```

   Git 2.56.1 aborted with:

   ```text
   BUG: refs.c:1908: ref pattern must end in a trailing slash when trimming
   ```

   Under that same environment, gix printed `only b`; the proposed rule admitted the ref because its stripped remainder was exactly A. Removing the variable made both readers print `only a`, with Git warning `bad replace ref name`.

   This reproduced in SHA-1 and SHA-256 repositories. Line 102 explicitly defers WR-01, but lines 100 and 290 still claim complete agreement/refusal. Either reject unsupported bases or narrow those claims; deferral does not establish compatibility.

3. **Major — Commit replacement chains can fail before the new guard, blaming the manifest instead of the ref.** [04-12-PLAN.md:159](.planning/phases/04-repo-model/04-12-PLAN.md) proposes claiming refusal “before any object is read through gix’s replacement map.” Actual ordering contradicts this: [src/model/mod.rs:485](src/model/mod.rs) calls `ref_commit` before `root_entry`, where [src/git/refs.rs:217](src/git/refs.rs) invokes the guard.

   I created workspace commit A, blob B, and valid commit C, then planted:

   ```sh
   git update-ref "refs/replace/$A" "$B"
   git update-ref "refs/replace/team/$B" "$C"
   git cat-file blob refs/heads/workspace/dev:.wsx.conf
   ```

   Git followed the chain and printed `only a`. Gix peeling A failed:

   ```text
   Error: Last encountered object 52d6107 was blob while trying to peel to commit
   ```

   Without the replacements, gix successfully peeled A. Source tracing routes this failure through `ManifestUnreadable` at model lines 481–485, before the new refusal can run. That contradicts D-22’s requirement that the error blame the replacement ref ([04-CONTEXT.md:107](.planning/phases/04-repo-model/04-CONTEXT.md)). The planned blob-only chain test misses this ordering failure.

4. **Major — The mutation gate accepts evidence that contradicts its claimed result.** [04-12-PLAN.md:29](.planning/phases/04-repo-model/04-12-PLAN.md) requires testing every mutant named by `--list`. Lines 238–240 check only unexplained entries in `missed.txt` and equality of line counts.

   I executed those exact two verification blocks in a scratch Git repository with controlled lists and a stubbed `cargo mutants --list`:

   - One timeout, zero caught mutants: both checks exited **0**.
   - One caught entry naming a different mutant from `--list`: both exited **0**.
   - Control: an unallowlisted missed mutant made the survivor check exit **1**, printing `unexplained survivors above`.

   Thus the checks enforce neither mutant identity nor the statement at line 237 that exit 3 “is not a pass.” Compare the actual mutant sets and explicitly settle timeout handling instead of reporting count equality as complete coverage.

5. **Minor — The plan silently drops required coverage measurement.** [04-12-PLAN.md:228](.planning/phases/04-repo-model/04-12-PLAN.md) says: “Each caught mutant also shows its line runs under the tests, so no separate coverage run is made.”

   That does not satisfy the supplied Rust rule, “`cargo llvm-cov` for line and branch coverage of modified code.” Nor does it reproduce [04-10-PLAN.md:146](.planning/phases/04-repo-model/04-10-PLAN.md), which separately requires line coverage and records a specific tooling-based branch-coverage waiver. Caught mutations do not measure unmutated lines or branches. Restore the coverage measurement or obtain an explicit waiver.

## SUSPECTED

None retained.

Scratch repositories and probe binaries were removed; no probe processes remained.


