## CONFIRMED

**Nothing was found at critical or major.** No remaining shape that reads different bytes with `Ok`, no guard error still blamed on `.wsx.conf`, no step that cannot run, no commit ordering that reopens a closed shape.

What I ran and what it printed (probes in `mktemp -d`, since deleted; no build, no `target/`, no tracked file touched; git 2.56.1):

- **Replace divergences the plans claim, re-measured** (the control that my search can see a real divergence, so "nothing found" is not blindness): `refs/replace/<tree>` → a commit printed `only c` (04-13:31's premise holds); `refs/replace/<commit>` → a blob made git fail. The second only says `does not exist in` because `repo_with_remote` asserts `.wsx.conf` is absent from the worktree (`src/model/mod.rs:697-700`); with the file on disk my probe got `exists on disk, but not in` instead, i.e. the planned assertion survives only on that fixture assertion — test premise, not a guard hole.
- **`info/grafts`, an unguarded mechanism in the same read path**: git 2.56.1 honours the graft for parentage (`git rev-list --parents -1 main` printed the grafted parent) but `git cat-file blob main:.wsx.conf` printed the original bytes — grafts change parents, not content, so no manifest divergence and nothing to fold in.
- **gix's map builder, read from source**: `gix-0.90.0/src/open/repository.rs:531-553` keys on `ObjectId::from_hex(name.strip_prefix(base))`, skips symbolic refs, and drops listing errors (`.ok()?`) — matching 04-12's rule step 2 exactly; `gix-hash-0.28.1/src/object_id.rs:64-97` makes a 40-digit name a SHA-1 key and a 64-digit name a SHA-256 key *whatever the repository's hash*, so 04-12's SHA-256 refusal is the safe side; `gix-0.90.0/src/config/tree/sections/gitoxide.rs:505-507` (`with_environment_override("GIT_REPLACE_REF_BASE")`) plus the env injection at `src/config/cache/init.rs:355,406` make S4's revision-3 measurement (`answered "configured", not`) consistent with source, not just a probe; `core.rs:103-104` maps `GIT_NO_REPLACE_OBJECTS` onto `core.useReplaceRefs`, so gix honours the same switch as git and 04-12:31's "both ignore it" measurement is sound — no silent divergence hides behind that switch in the *unrefused* plain-single-replacement case.
- **The same class already refused elsewhere**, i.e. the "gix ignores what git honours" family is reachable and closed by earlier phases: reftable (`src/git/mod.rs:192-196`), `GIT_OBJECT_DIRECTORY` / `GIT_ALTERNATE_OBJECT_DIRECTORIES` / `GIT_COMMON_DIR` (`src/git/repo.rs:264-290`). I checked these because an unreported member of that family would be the one thing the two plans structurally cannot cover.
- **Test/helper existence** behind every loop in both plans: all twelve pre-existing test names cited in the verify blocks exist, and `git::shell_word`, `refusal`, `replacement_ref`, `assert_blames_the_ref`, `sides`, `chain_repo`, `child_state_with`, `set_config`, `REPLACE_BASE_BASE_VAR`-style consts are all present, so no loop dies on exit 4. `scripts/check.sh` does print `==> check.sh: all OK` last and does accept one `-E`, so the `all` gates in both plans are runnable as written.

## SUSPECTED

1. **`ReplacementRefIgnored`'s "git … applies it" clause is true only where git's *iteration* returns the name** — `04-12-PLAN.md:199`, rule at `04-12-PLAN.md:212`. The refusal fires on any ref gix lists whose name after the base is longer than `hex_len`, and gix lists a name only if its own validator accepts it; if a name exists that gix accepts and `git check-ref-format` rejects, git skips it and the message is false for that shape. *Speculation*: 04-13's measurements put git's answer for an invalid directory component at `bad file`, and gix lists nothing there, so no such shape is known — the refusal still fires closed, only the claim would be wrong. What would confirm or kill it: one ref name gix's `FullName::try_from` accepts and `git check-ref-format` rejects, planted under a valid base; if none exists, the clause is sound and no change is needed.

## S1–S15 status

All fifteen are closed in the plan text; none needs another round.

| # | Status | Where |
|---|--------|-------|
| S1 | closed | `04-12-PLAN.md:261` splits the guard and wraps every non-refusal as `ReplaceRefsUnreadable`, `04-12-PLAN.md:28` states it, `04-12-PLAN.md:257` pins it (`invalid_packed_replace_ref_name_blames_the_replace_refs`); 04-13's walk and packed reads return `Error::Io` into the same wrap (`04-13-PLAN.md:125-135`). Verified `is_replacement_refusal` is a flat `matches!` (`src/git/mod.rs:167-174`), so `Io`/`Gix` are always wrapped. |
| S2 | closed | `04-12-PLAN.md:31` and `04-12-PLAN.md:296` scope 04-11's three texts to `when replace refs are in use`; new texts at `04-12-PLAN.md:199`, `04-12-PLAN.md:253-254`; each 04-13 kind rendered (`04-13-PLAN.md:32`, `139`). |
| S3 | closed | `04-13-PLAN.md:30`, rule at `04-13-PLAN.md:131-135`, task at `04-13-PLAN.md:257-262` (git control `only .planning`, in-order control `only b`). |
| S4 | closed, by measurement | `04-12-PLAN.md:170`; the RED text `answered "configured", not` is reachable because gix injects `GIT_REPLACE_REF_BASE` as its own base (`gix-0.90.0/src/config/tree/sections/gitoxide.rs:505-507`). |
| S5 | closed | `04-13-PLAN.md:31`, loop at `04-13-PLAN.md:137`, kept apart from the chain loop at `04-13-PLAN.md:261` so chains stay `ReplacementRef`. |
| S6 | closed | `04-13-PLAN.md:127` — append components, never join with another `/`. |
| S7 | closed | `04-12-PLAN.md:212(b)` scopes the sentence to `among the refs the listing returns`; `04-13-PLAN.md:33` and `04-13-PLAN.md:262` restate it as `a regular loose file or a packed record` once the skips are refused. |
| S8 | closed | `04-12-PLAN.md:298` second text replaces `runs only inside root_entry`; `04-12-PLAN.md:33` and the STATE block gate it. |
| S9 | closed | `04-12-PLAN.md:298` and `04-13-PLAN.md:298` both use `state.resolve-blocker` then `state.add-blocker`; the two target lines exist today at `.planning/STATE.md` Blockers/Concerns lines 11 and 13, and `Phase 12: MIG-03` is there as the positive control (`04-13-PLAN.md:319`). |
| S10 | closed | `04-13-PLAN.md:315` — the `deps` block now reads only the exit code. |
| S11 | closed by dropping the claim | `04-12-PLAN.md:25` ends "they do not pin the order … and do not test those checks". |
| S12 | closed | `04-13-PLAN.md:127` refuses every non-directory non-regular entry, `04-13-PLAN.md:194` pins the fifo. |
| S13 | closed | `04-12-PLAN.md:260` refuses `refs/`, `refs/worktree/`, `refs/bisect/`, `refs/rewritten/`; `04-13-PLAN.md:126` walks the common dir alone and drops the untested arm. |
| S14 | closed | `04-13-PLAN.md:319`. |
| S15 | closed | `04-12-PLAN.md:90` now says thirteen model tests, one unit test, three message tests — the three lists agree. |

## Scope

Proportionate: every added mechanism maps to a divergence that was measured, not hypothesised — the walk to the symlink and invalid-name shapes (`04-13-PLAN.md:112-124`), the packed-refs read to the sorted-header miss and the duplicate name, the target-kind loop to the tag and tree-by-commit reads, `is_replace_base` to git's `BUG:` abort and to the common-dir-only walk. The one thing that could be cut without reopening a measured divergence is the **whole-file reach of `PackedRefsDuplicate`** (`04-13-PLAN.md:133-135`): the measured duplicate divergence is replace-record only (git stops with `duplicate replace ref`), so narrowing that scan to records under the base would save a sort over every ref in the file; the unsorted rule must stay whole-file, since git's lookup of any name trusts the order. Everything else is either a measured divergence or an over-refusal that *is* the guard's accepted cost (special files, dangling symlinks, `refs/`-prefixed odd bases, replace refs turned off). `git_skips` (`04-13-PLAN.md:127`, `208`) is not a divergence rule either — it exists so ordinary `.lock` files do not stop every command. Two smaller costs worth naming but not cutting: the guard now runs twice per resolve (`build` and `root_entry`, `04-12-PLAN.md:262`) and 04-13's three checks run on both calls.
