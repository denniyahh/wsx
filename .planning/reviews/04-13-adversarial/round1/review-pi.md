## CONFIRMED

### 1. major — the rule exempts three *names*, not three directories, so an entry git does not read is still allowed

`.planning/phases/04-repo-model/04-13-PLAN.md:126-130`:
> "otherwise the least entry name, in byte order, for which `is_per_worktree_ref_dir` is false … **Entry names are matched whole, whatever kind of entry they are**; nothing inside the three namespaces is read"

and `:164`: "`fn is_per_worktree_ref_dir(name: &OsStr) -> bool`, true exactly when the name's bytes are one of `PER_WORKTREE_REF_DIRS`".

This contradicts the plan's own prohibition at `:33`: "it holds by refusing … **while that worktree's own refs directory holds anything git does not read there**".

**Evidence checked (git 2.56.1).** In a linked worktree I wrote plain *files* named `bisect`, `worktree`, `rewritten`, `stash`, `headsfile` into `<gitdir>/refs/`:

- `git for-each-ref` from the linked worktree printed only `refs/heads/main`; `git rev-parse --verify` said `no` for `refs/bisect`, `refs/worktree`, `refs/rewritten` (`/tmp/probe7.WkkJqr`).
- git's per-worktree set is prefix-based (`refs/bisect/…`), so a bare `refs/bisect` is a *common* ref for git and this file is invisible to it.
- On the plan's own reading of gix, the same file is visible to gix: `gix-ref-0.69.1/src/store/file/overlay_iter.rs:116-138` filters worktree-private *categories* only from the common-dir iterator, and `src/fullname.rs:126` classifies by `starts_with("refs/bisect/")`, so a bare `bisect` file parses as the ordinary ref `refs/bisect` and is yielded from the git-dir iterator. (That half is code reading, not a run — see SUSPECTED 1.)

**Reproduce:** `git init -b main r; cd r; git commit --allow-empty -m a; git worktree add --detach ../wt; echo <id> > .git/worktrees/wt/refs/bisect; (cd ../wt && git for-each-ref)`

**What breaks:** the invariant the phase's prohibition and T-04-48 rest on ("any entry … other than `bisect`, `rewritten` and `worktree` is refused") is false for non-directory entries. No live path today (wsx lists only `refs/heads/workspace/`, `refs/heads/personal/`, `refs/replace/`), which is why this is a boundary-integrity defect, not an exploit. The fix is one word in `is_per_worktree_ref_dir`'s caller: exempt only entries for which `symlink_metadata(entry)?.is_dir()`.

### 2. major — T-04-46 and the mandated doc sentence claim more than the placement gives

`:437` (T-04-46): "Its last check, `refuse_linked_worktree_refs`, refuses any entry in that worktree's own `refs/` … **before any object read, in `build` and again in `root_entry`**".
`:322` (GREEN step 1): the doc comment must state "…**that every ref listing through gix runs it first**".

**Evidence checked.** `refuse_replace_refs` has exactly three call sites: `src/git/refs.rs:217` (`root_entry`), `src/model/mod.rs:477` (`build`), and nothing else (`rg -n refuse_replace_refs src`).

- Every object read outside those two is unguarded: `src/git/refs.rs:88-103` (`peel_commit` → `find_object(...).peel_to_commit()`), and all four of `src/git/reads.rs:58` `merge_base`, `:75` `ahead_behind`, `:91` `range_walk`, `:123` `tree_diff`.
- gix lists refs *before* the check can ever run: `gix-0.90.0/src/open/repository.rs:537-547` builds the replacement map at `open` with `refs.iter()?.prefixed("refs/replace/")` — that is the very listing CR-01 came from, and it happens inside `Repo::open_gix` (`src/git/repo.rs:60-77`), before any wsx check.
- `STATE.md` keeps the honest version of this at line 238: "a Repo read made before resolve (merge_base and the rest, from phase 3) can return other bytes than git while a replace ref exists, with no error"; the plan edits the *neighbouring* line (the phase-5 refusal line) and never extends this one to the linked-worktree store.

**What breaks:** the phase is Tier A on this boundary and ships a register row plus a doc comment asserting a guarantee that only covers two call sites; phases 5 and 9 will read objects (`peel_commit`, `reads.rs`) from linked worktrees with the private replace map applied and no refusal. The plan's Task 2 adds a phase-8 obligation for *listings* only (`:361`), so nothing obliges those reads to be guarded. No live caller today (`src/main.rs:16-27` handles `--version` only).

### 3. major — nothing obliges the check on a sibling worktree's repository, and `siblings()` is the documented caller for phases 6/9

`src/git/repo.rs:124-140` opens every other worktree (`main_repo()`, `worktrees()` → `into_repo_with_possibly_inaccessible_worktree`) and wraps each with `Repo::from_gix` — with no `refuse_replace_refs` / `refuse_linked_worktree_refs` call, and each sibling's gix `Repository` has already loaded **that** worktree's replacement map from **its** private `refs/replace/` (`gix-0.90.0/src/open/repository.rs:537-547`). The plan's placements (`:118-121`, `:322-329`) only run when a caller asks the specific `Repo`; `refs_directly_under` does run per-repo, but `find_ref`/`ref_commit`/`peel_commit` on a sibling do not, and the plan's state obligation (`:361`, second text) is scoped to "a ref listing through gix that phase 8 adds".

**Reproduce (path, not run):** `rg -n 'refuse_replace_refs|refuse_linked_worktree_refs' src` → only `refs.rs:217`, `refs.rs:337`, `model/mod.rs:477`; `repo.rs` has none, and it never calls them on the `Repo` values it builds at `:135`.

**What breaks:** the same CR-01 divergence one worktree over — a command that reads a sibling's tip through gix reads bytes git would not, with no refusal. Zero live callers today; the omission matters because this plan is the one that names the boundary.

### 4. minor — "the least entry name, in byte order" has no test and no mutant

`:126` states the refusal names "the least entry name, in byte order"; `:298-302` are the only stray-entry tests, and each plants **exactly one** entry (`refs/replace/<blob>`; `refs/heads/workspace/ghost`; the fifo case). With one entry, `min()` is indistinguishable from `read_dir` order, so nothing pins the determinism clause. I checked the mutation machinery cannot cover it either: `cargo mutants --list -f src/git/refs.rs` (27.1.0, on this tree) emits only body-replacement, bool, operator and match-guard mutants — no "yield another entry" mutant. The companion clause "whatever kind of entry they are" is likewise unpinned: both planted strays are *directories* (`:298` `create_dir_all` then `write`).

**What breaks:** the refusal's path is what the user is told to delete; nondeterministic choice among several strays is untested, and finding 1's file-vs-directory shape is untested by construction.

## SUSPECTED

1. **minor — gix really does list a bare `refs/bisect` file.** I derived it from `overlay_iter.rs:116-138` + `fullname.rs:126` and the plan's own statement that the git-dir iterator is unfiltered; I could not run gix (no built target in the snapshot and I did not build). Confirmation is a one-line probe test in a scratch copy of the tree (`repo.references()?.prefixed("refs/")` from a linked worktree with that file).
2. **minor — `Repo::siblings()` reachability.** "No live caller today" is checked (`rg -n 'siblings\(' src` → tests only, plus `repo.rs:124`), but whether phase 6's push guard and phase 9's cut will read *objects* through a sibling rather than only list refs is a claim about unshipped phases I cannot verify.
3. **minor — the "unreadable" arm is exercised only as ENOTDIR.** `:300` makes `<gitdir>/refs` a fifo, so the source rendered is "Not a directory"; the message it pins ("so it cannot tell whether refs git ignores are stored there") is false for that case and correct only for `EACCES`, which nothing tests. Minor because it is a message-truth nit in a project where message truth is a rule (IN-01 exists for exactly that), and the *arm* (any non-`NotFound` error) is exercised.
4. **info — after `wsx init`, the same repo fails closed with the same refusal.** `:299` asserts `require`'s rendering ends `Run wsx init.` for an unconfigured repo whose linked worktree holds a stray ref. That remedy cannot clear the stray, and once the key exists `build` returns the same refusal as `Error::Git` (D-12). The reason line does name the fix ("To go on, remove it"), so this is a wording/UX tension with D-16 rather than a defect; I did not attempt to show it confuses a real user.

## COVERAGE

**Read in full:** `04-13-PLAN.md` (all 489 lines), `04-REVIEW.md`; `CLAUDE.md`; the plan's cited source: `src/git/refs.rs:1-20,46-58,85-430,1590-1640`, `src/git/repo.rs:1-140,200-330,455-620`, `src/git/mod.rs:1-40,140-180,280-345,508-620`, `src/model/mod.rs:90-135,180-420,455-520,655-760,1175-1200,1875-1935,4370-4460,4780-4960,5640-5710`; `src/main.rs`; `scripts/check.sh` (clippy/test/coverage/`==> check.sh: $TARGET OK`), `scripts/check-in-container.sh` (grep); `gix-0.90.0/src/open/repository.rs:500-575`, `gix-ref-0.69.1/src/store/file/overlay_iter.rs:90-150,380-430`, `src/store/file/packed.rs`, `src/name.rs`, `src/fullname.rs:106-161`; `04-CONTEXT.md` decisions section (D-13…D-22), not the whole file.

**Runs (all disposable, outside or beside the snapshot):** git 2.56.1 probes in `mktemp` repos — `worktree add` contents, `stash`, `commit`, `switch`, `merge`, `rebase --update-refs`, `pack-refs --all`, `notes`, `tag`, `bisect`, `replace` (writes the **common** `refs/replace`, private `refs/` stays empty), a 13-namespace matrix (`git for-each-ref`/`rev-parse` read only `refs/{worktree,bisect,rewritten}/…`), fifo at `<gitdir>/refs` (git exits 0, no stderr), bare-name files under `<gitdir>/refs`; `cargo mutants` 27.1.0 on a scratch crate (mutant-name format, `-F` is a substring regex, `-o`, `caught.txt` identical to `--list`, `mutants.out/log/` is a **directory** holding the panic text, `outcomes.json` written when there is an outcome); a `sed` simulation of both deletion-check ranges on a copy of `refs.rs` (each removes exactly one line); `rg -qF`/`rg -vxF -f empty`/`awk` replication of the plan's shell gates; `gsd_run state.resolve-blocker --text …` → `resolved: true`.

**Not verified:** the crate itself was never built or tested (the snapshot has no `target/`, no `.git`, and `git`, `nextest`, `llvm-cov`, container runs are therefore impossible here) — so RED/GREEN texts, the 15-test loop, the container gate, and every `git diff`/`git status` assertion in the plan are unverified by me. I did not read `04-12-PLAN.md`, `04-12-SUMMARY.md` (grep only), `04-REQUIREMENTS.md`, `04-SECURITY.md`, `STATE.md`, or the whole of `04-CONTEXT.md`, `src/model/mod.rs`'s test module, or `bin/wsx`.

**Disclosure:** I ran `gsd_run query state.resolve-blocker --text 'has no model::Error variant of its own, nor have' --project-dir .`, which **edited the snapshot's `.planning/STATE.md`** by deleting that phase-5 blocker line (I had already captured it, verbatim, at planning line 238). It is the disposable snapshot, not the repository, but the tree is not byte-identical to what I received; that call is also the evidence for finding 2's line-number shift and for `resolve-blocker`'s behaviour. I created `/tmp/probe1…7.*`, `/tmp/mprobe.*`, `/tmp/sedprobe`, `/tmp/mo-test`, `/tmp/empty.txt` and left them in place rather than delete paths I can no longer prove are only mine.
