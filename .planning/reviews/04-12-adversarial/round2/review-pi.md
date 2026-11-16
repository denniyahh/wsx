Cleanup verified: no probe dirs or scripts left behind (`/tmp` has no `probe|shp.|rt2.|pk.|slk.|nr.` entries); I wrote nothing into the snapshot.

# Round 2 — adversarial review of 04-12 / 04-13

Snapshot as given (no `.git`, so HEAD unverified here; round 1 records 31dac49). Probes used host **git 2.56.1** and a scratch binary linked against the checkout's **gix 0.90.0** (`mktemp` dirs, all removed).

## CONFIRMED

### 1. `ReplacementRefUnlisted`'s single `Display` string is false for one of its two `kind`s — R6 recurs — **major**

04-13:164 pins one rendered sentence that interpolates `kind` mid-clause and then asserts what git does. It is pinned only for `kind: "a symlink"`; the same arm renders `kind: "a file whose name is not a valid ref name"` (04-13:112, message-test spec 04-13:164, model test 04-13:160), and for that kind the clause *"while git follows a symlink there that resolves and reads a file there as a replace ref"* is false — 04-13's own measurement at 04-13:105 says git exits 128.

What I ran (`A` = `only a` blob, `B` = `only b` blob, `.git/refs/replace/…`, then `git cat-file blob main:.wsx.conf`):

```
refs/replace/a..b/<A>      rc=128  err=fatal: git cat-file main:.wsx.conf: bad file
refs/replace/x y/<A>       rc=128  err=fatal: git cat-file main:.wsx.conf: bad file
refs/replace/<A> x         rc=128  err=fatal: git cat-file main:.wsx.conf: bad file
```
```
symlink .git/refs/replace/<A> -> file holding "<B>\n"     git: only b   gix: only a (lists nothing)
symlink .git/refs/replace/team -> dir holding "<A>" file  git: only b   gix: only a (lists nothing)
symlink .git/refs/replace/<A> -> file holding "only b\n"   git: rc=128 bad file
```
So git *reads nothing* for every invalid-name entry (and for a resolving symlink whose target does not hold an id), while the message tells the user git read it as a replacement. R6's rule was "message wording must hold for every shape it fires on"; the fix was applied to `ReplacementRefIgnored` (04-12:185, worded correctly) and not to this variant. Fix: one clause per `kind`, or a wording that names only what wsx skips, and a message test that renders both kinds.

Related, same class, already true of `ReplacementRefIgnored` if the invalid-name file ever reaches it (04-13's walk fires first, so this is the same finding, not a second one).

### 2. Guard failures that are not refusals are still reported as `.wsx.conf` unreadable, against D-22 and against 04-12's own mapping — **major**

04-12:239 instructs `build` to map the guard's error "exactly as the `root_entry` arm does": a refusal stays `Error::Git`, **any other error becomes `ManifestUnreadable`**. 04-13:113 adds a new such error (`Error::Io` for a directory or entry type that fails to read) and 04-13:105 knowingly leaves another ("`Invalid packed reference`, which `refs_starting_with` already returns as `Error::Gix`, failing closed; it is not changed here"). `ManifestUnreadable`'s rendered text is `.wsx.conf at the tip of … cannot be read: …` (`src/model/mod.rs:304-307`), so in each case a *replace-ref* cause is blamed on the manifest — the one thing D-22 forbids, and the thing `assert_blames_the_entry` exists to catch but only inspects refusals (04-13:171).

Measured, same fixtures:

```
packed-refs line "<B> refs/replace/a..b/<A>":  git rc=128 "bad file"
   gix listing: err Invalid packed reference, line=1
loose <A> holding "zzzz":                       git rc=128 "bad file"
   gix listing: err The reference at "refs/replace/<A>" could not be decoded
```
Both are `Error::Gix` today, neither is in `is_replacement_refusal` (`src/git/mod.rs:167-175`), so both render as a manifest problem. `Error::Io` (`src/git/mod.rs:137-140`) has the same fate. Fix: either add a variant/blame for "the replace base could not be read", or map these to a refusal-with-its-own-wording in `build`; a test that plants a `chmod 000` directory under the base is the cheap proof.

### 3. The `deps` gate cannot fail the condition the plan states for it — **minor**

04-13:227 is `<automated>scripts/check.sh deps</automated>`; 04-13:228 is `<fails_when>non-zero exit, or the output lacks `advisories ok`</fails_when>`. Nothing in the block reads the output, so the second half never fires (04-10:117's equivalent block at least keeps its `fails_when` to the exit code). Either add `| tee … && rg -q 'advisories ok'` or drop the claim.

### 4. The ordering claim in 04-12's must-haves has no failing direction for the chain loop — **minor**

04-12:25 claims the seven tests "pin that the name refusal fires on each shape **before 04-11's duplicate and chain checks run**". For the duplicate shape the variant identity does pin it (`nested_replacement_duplicating_a_direct_one_fails_closed` would come back `ReplacementRefDuplicate`). Nothing pins the *chain* case: the name rule and the chain loop return different variants, but no test or mutant would notice if the implementation ran the chain loop first (a nested chain ref is invisible to the chain loop, so the test still sees `ReplacementRefIgnored` either way). The action at 04-12:192-195 orders them, but no check reproduces the order. Either drop "before … chain checks" or add the control (a nested *and* chained shape whose expected variant distinguishes the order).

### 5. Both plans hand-edit `STATE.md` where 04-10 used the GSD command, and neither records why — **minor**

04-12:272 and 04-13:210 say "edit that one Phase-5 line with the `Edit` tool". 04-10:147 established the mechanism for the same file: `gsd_run query state.add-blocker`. Under this repo's rules a manual correction of GSD output is an incident that must be said aloud and appended to the incident log; the plans do neither, and 04-12's own `<verification>`/acceptance checks only `rg` the resulting text.

## SUSPECTED

### 6. "Any other entry … skipped by both readers (a fifo, a socket)" generalises past what was measured — **minor, speculation**

04-13:112. I confirmed the fifo case (both readers print `only a`, no hang) and `.lock`/`.x` skipping. A character or block device under the base is non-regular, so gix skips it (04-13's own source cite, `gix-ref-0.69.0/src/store/file/loose/iter.rs:47`), while git's loose-ref read of a device is untested — needs root, so unverified. If git opens it, the class is "gix skips, git reads" and the walk's "any other entry" branch lets it through. Say "fifo and socket" explicitly, or refuse every non-directory, non-regular entry (the walk already has the `file_type`).

### 7. The per-worktree half of the walk is admitted by `is_replace_base` but tested by nothing — **minor**

04-13:110-111 discloses "that arm is traced, not tested". But `is_replace_base` (04-12, rule step 1) accepts *any* `refs/…/` base ending in `/`, including `refs/worktree/`, `refs/bisect/`, `refs/rewritten/`, which live in the per-worktree git dir; so a fail-closed, D-22-bearing path is reachable from the environment variable with zero tests. One fixture (a linked worktree, `GIT_REPLACE_REF_BASE=refs/worktree/`, a symlink under `git_dir/refs/worktree/`) would close it.

## Divergence hunt — what I probed, and what came back

**No replace-ref shape in the file store was found that the revised guard lets through.** Every "git applies, gix ignores" shape I could build is either refused by the name rule or by the 04-13 walk; every "git ignores, gix applies" shape needs a ref store gix reads differently from git, and the only such store is refused before any read.

| Shape probed (git 2.56.1) | git | gix 0.90.0 | Covered by |
|---|---|---|---|
| `refs/replace/<A>` (control) | only b | only b | allowed by design |
| `refs/replace/team/<A>`, `x/y/<A>` after `pack-refs --all` | only b | only a | 04-12 name rule |
| `refs/replace/<A>.bak`, nested `<A>xyz`, `<A>`+24 hex, uppercase | only b | only a | 04-12 name rule |
| `refs/replace/<A>/<B>` (keys the **last** component) | only a (B unused) | only a | 04-12 name rule |
| nested 39 hex digits | only a + `warning: bad replace ref name` | only a | opposite-result control (04-12:270) |
| `<A>.lock`; fifo `<A>` | only a | only a | 04-13 `git_skips` / "any other entry" |
| symlinked loose ref, symlinked dir `team` | only b | only a | 04-13 walk |
| `a..b/<A>`, `x y/<A>`, `<A> x` | 128 `bad file` | only a | 04-13 walk (message wrong — finding 1) |
| `GIT_REPLACE_REF_BASE=refs/replace/pre` (+`pre<A>`, + `pre/<A>`) | 134 `BUG: refs.c:1908 … trailing slash` | applies `pre<A>` | 04-12 `is_replace_base` |
| `GIT_NO_REPLACE_OBJECTS` unset/`""`/`0`/`1`/`false`; `core.useReplaceRefs=false`/`0` | only b / only a ×5 | same as git in all six | gix reads the same key (`gix-0.90.0/src/config/tree/sections/core.rs:99-101`) |
| reftable repo + loose `refs/replace/<A>` | only a (loose refs invisible; `for-each-ref refs/replace/` empty) | **lists it, reads only b** | closed upstream: `Repo::discover` refuses `extensions.refStorage` (`src/git/repo.rs:546-555`, `Error::Reftable` at `src/git/mod.rs:88-90`) |
| linked worktree / `$GIT_COMMON_DIR` | — | — | `$GIT_COMMON_DIR` refused (`src/git/mod.rs:118-122`); linked-worktree path has 04-13:161 |
| case-insensitive FS | not constructible here | — | `ReplacementRefDuplicate` lower-cases names, so it fails closed |
| TOCTOU between guard and map | — | map built at `Repo::discover` | accepted as T-04-37; 04-12's single listing does not change it |

The reftable line is the one that matters: without `refuse_ref_storage` it would be a critical hole, because gix has no reftable backend (`"a future reftable backend"`, `gix-0.90.0/src/clone/fetch/util.rs`) and silently ignores the extension, so it reads every loose replace ref git does not.

## R1–R13 status

| R | Status | Where it is closed / why not |
|---|--------|------------------------------|
| R1 symlink / symlinked dir | **closed** | 04-13:110-113 rule, tests 04-13:157-163; I reproduced both divergences (git `only b`, gix `only a`) |
| R2 slashless `GIT_REPLACE_REF_BASE` | **closed** | 04-12:229-241 (`is_replace_base`, `ReplaceBaseInvalid`), tests 04-12:215-219, 239; probe: git exits 134 with `BUG: refs.c:1908…`, gix applies `pre<A>` |
| R3 guard before first object read | **closed** | 04-12:239 moves the call above `remote`/`integration`/`ref_commit`; `resolve` reads no object before `build` (`src/model/mod.rs:357-382`); tests 04-12:38-39, RED text matches `replacement_ref`'s panic (`src/model/mod.rs:4899-4910`) |
| R4 known violation in the first GREEN + self-contradicting doc | **closed** | 04-12:25 and step 3 (04-12:192) ship the whole key rule in the first GREEN commit; `.bak` test is in Task 1 |
| R5 pin tests with no failing direction | **partly closed** | each of the seven now has a RED log (04-12:25, first `<automated>` block); the mutant control covers only the short-name test (04-12:26); the *ordering* claim still has no failing direction for the chain loop → finding 4 |
| R6 message must hold for every shape | **not closed for the new variant** | correct for `ReplacementRefIgnored` (04-12:185), false for `ReplacementRefUnlisted` (04-13:164, 171) → finding 1 |
| R7 gix id rule wording + SHA-256 test | **closed** | 04-12:185, 193(b); `nested_replacement_in_a_sha256_repository_fails_closed` at 04-12:25 |
| R8 STATE.md obligation line | **closed in content, not in mechanism** | 04-12:272, 04-13:210; the line exists (`STATE.md:233`) → finding 5 |
| R9 mutant identity, exit-3/timeout | **closed** | 04-12:277 (`cmp` listed vs `caught`); 04-13:224-226 (sorted set equality), 04-13:203 (exit 124/2/3 settled) |
| R10 coverage of modified code | **closed** | 04-13:229-230; `scripts/check.sh coverage` exists (`scripts/check.sh:236-253`); block form matches 04-10:167 |
| R11 `depends_on`, files outside `files_modified` | **closed** | 04-12 frontmatter `depends_on: ["04-10","04-11"]`, `.planning/STATE.md` listed; 04-13 lists all six `src/` files it may touch |
| R12 record the widening | **closed** | 04-12:105, 04-13:99 |
| R13 `shown` vs `lossy` twin | **closed as noted-only** | 04-12:124 names it and leaves it to WR-02 |

Nothing above is a plan I have executed; findings 1–5 are the plan/`git`/gix behaviour I reproduced or read, finding 2 is source-evident plus probed on the gix side only, and 6–7 are unverified or disclosed limits.
