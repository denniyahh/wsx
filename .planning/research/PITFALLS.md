# Pitfalls Research

**Domain:** Rewriting a git-worktree personal-file sync CLI (wsx 0.1.0, Python) in Rust on gix, with the Python binary as a black-box oracle that is deleted afterwards
**Researched:** 2026-10-17
**Confidence:** MEDIUM-HIGH overall. Most items below were either reproduced on this host with a control case, or read in the published source of the exact crate version. The GSD `classify-confidence` seam rates anything that came from WebFetch as LOW; those items are tagged [DOCS] and flagged where they matter.

## Read this first: what is new compared with CONCERNS.md

`.planning/codebase/CONCERNS.md` already lists 3 reproduced defects, the first-push guard gap, the lost global `core.excludesFile`, the lack of locking, the full rehash and the non-hermetic suite. Those are not repeated here, only sharpened where needed. **Five defects are new. Each was reproduced today in a hermetic scratch repo (with `HOME`, `XDG_CONFIG_HOME`, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1` and the `GIT_*` variables unset), and each probe had a control case that behaved correctly:**

| # | New defect in 0.1.0 | Impact | Control that behaved correctly |
|---|---|---|---|
| N1 | An **unreadable local directory** (`chmod 000 .p/sub`) makes `wsx push` print `delete .p/sub/A (sync: changed locally)` and delete the **whole subtree** from the source. `os.walk` silently skips directories it cannot read, and `digest()` maps the resulting `EACCES` to "absent". | Data loss, at directory scale. This is a separate code path from the known file-level bug, so fixing `digest()` alone does not close it. | Readable sibling `.p/B` was left alone. |
| N2 | **The guard misses managed paths that contain non-ASCII or special characters.** `git diff --cached --name-only` quotes them (`".p/caf\303\251.md"`, because of `core.quotePath`), so `is_managed()` never matches. `wsx guard` exits 0. | A personal file can be committed to a team branch. | A staged ASCII managed path made the guard exit 1. |
| N3 | **The pre-push guard compares only the two tips** (`git diff upstream..HEAD`). A commit that adds a managed file, followed by a commit that removes it, passes. The file is still in pushed history. | A personal file can be pushed to the team remote. | A managed file present at the tip made the guard exit 1. |
| N4 | **Git exports `GIT_DIR` to every hook** (pre-commit, pre-push, post-checkout, post-rewrite; `GIT_INDEX_FILE` too in pre-commit). Inside a hook, `prune_orphan_excludes()` then runs `git rev-parse --absolute-git-dir` in each sibling worktree and gets the hook's own git dir back every time. It concludes the siblings are dead and **deletes their `exclude.wsx-*` files**. This fires whenever the exclude is stale, for example after any edit to `.wsx.conf`. | In every other worktree the personal files reappear as untracked (`?? .p/B` observed), so a `git add -A` sweeps them in. | Running the same regeneration as `wsx setup` outside a hook kept both exclude files. |
| N5 | Python 3.15 runs in UTF-8 mode (`surrogateescape`) and stores a non-UTF-8 filename in `base.json` as a lone surrogate, `".p/bad\udcff.md"`. **serde_json 1.0.154 rejects lone surrogates when it deserialises a `String`** (`LoneLeadingSurrogateInHexEscape`, read.rs:962). | One such filename makes the whole v1 file unparseable for a naive Rust upgrader. | Python's behaviour was reproduced. The serde_json behaviour is [SOURCE], read in the code but not run. |

Probe scripts were run from the session scratchpad and have been deleted.

Evidence tags used below:
- **[REPRODUCED]**: executed on this host with a control case.
- **[SOURCE]**: read in published source: gix 0.89.0 crate (crates.io, 2026-10-08), gitoxide main `ad0202e`, serde_json 1.0.154, husky `main`.
- **[DOCS]**: official docs via WebFetch (seam tier LOW).
- **[CODE-READ]**: follows from reading `bin/wsx`. Not executed.
- **[UNVERIFIED]**: plausible, not checked.

Phase labels are suggestions for the roadmapper, in dependency order:
- **H**: harness and oracle
- **G**: git access layer and path identity
- **C**: core port (manifest, walk, digest, copy, planner/apply parity)
- **S**: safety (data-loss fixes, locking, crash and concurrency safety)
- **K**: guard and hooks
- **V**: state v2 (stat cache, `base.json` upgrade)
- **X**: cutover (Python deleted)

---

## Critical Pitfalls

### Pitfall 1: Rust's standard library brings back the "absent vs unreadable" data-loss bug by default

**What goes wrong:**
A faithful port of `digest()`, `walk()` and `is_file()` checks produces the same deletions as the known bug and as N1, in new places.

**Why it happens:**
`Path::is_file()`, `Path::exists()` and `Path::is_dir()` return `bool` and turn every error into `false`. The docs say so: "If you cannot access the metadata of the file, e.g. because of a permission error ... this will return `false`" [DOCS]. Probed on this host [REPRODUCED]:
- For a file inside a `chmod 000` directory: `is_file=false`, `exists=false`, `try_exists=Err(PermissionDenied)`.
- For a **missing** file inside that directory: also `PermissionDenied`. Absence cannot be known there.
- Control, a readable directory: `Ok(true)`, and `NotFound` for the missing file.

`walkdir` and `ignore` yield `Err` entries for unreadable directories. Code that writes `filter_map(Result::ok)` behaves exactly like Python's silent `os.walk`.

**How to avoid:**
- Model each side of a file as a closed enum: `Absent | Regular{hash,…} | Symlink | Dir | Special | Unreadable(io::ErrorKind)`.
- Treat only `NotFound` from `symlink_metadata()` as `Absent`. Decide `ENOTDIR` explicitly.
- Never call the bool predicates in `plan()`, `walk()` or `apply()`. Add a `clippy.toml` `disallowed-methods` entry for `Path::exists`, `Path::is_file`, `Path::is_dir` and `Result::ok` inside the sync crate.
- A walk error under a managed root must mark that whole subtree `Unreadable`. Every base key under it then becomes "skip or conflict", never a delete.
- A FIFO, socket or device in a managed directory is `Special`, not `Absent`. The current `is_file()` check also maps those to absent.

**Warning signs:**
- `.ok()` or `.unwrap_or(false)` near filesystem calls.
- A planner function that takes `Option<Hash>` instead of a state enum.
- No test with an unreadable directory.

**Phase to address:** C (the type design), S (the regression cases). The proptest planner model must generate `Unreadable` and `Special` states.

---

### Pitfall 2: Plan-to-apply race and "hash after copy" quietly lose edits

**What goes wrong:**
- **Window 1.** `plan()` hashes both sides, then `apply()` acts later. If an agent writes the local file in between, a `pull` the plan approved ("changed upstream") overwrites the fresh edit. There is no backup, because the policy is `sync`.
- **Window 2.** `apply()` records `base[rel] = digest(dst_path)` by re-reading the destination after the copy (`bin/wsx:465,470`). If the file changes between the copy and that read, the base records the new edit as already synced. The next pull then sees `local == base` and overwrites it [CODE-READ].

**Why it happens:**
Agent session hooks run while agents are still writing files, and Claude Code runs every matching hook in parallel [DOCS]. A rewrite that keeps the plan/apply split, which it must, keeps both windows unless it adds preconditions.

**How to avoid:**
- Every action carries the observed state of both sides. Immediately before acting, `apply()` re-checks them (compare-and-swap) and downgrades a mismatch to a conflict or a skip. This is a precondition check, not a new decision, so it does not break the "all decisions in `plan()`" rule.
- Hash the bytes while copying them into the temp file. Record **that** hash in the base, never a re-read of the destination.

**Warning signs:**
- `apply()` calls `digest()` on a path it just wrote.
- Actions carry no expected hashes.

**Phase to address:** S. It needs a deterministic interleaving test: a test-only hook between plan and apply, or an injected file-mutation step.

---

### Pitfall 3: Locking in the wrong scope, or deadlocking itself

**What goes wrong:**
**(a) Lost update across worktrees** [CODE-READ, a sharper version of CONCERNS]. Worktrees A and B both edit F (base h0, source h0).
1. B plans `push` (source h0 matches its base).
2. A pushes hA.
3. B applies and writes hB over hA.
4. A's next pull sees `local == base(hA)`, calls the source change "changed upstream", and overwrites hA.

A's edit is now gone everywhere. A lock that covers only `apply()`, or a per-worktree lock, does not prevent this.

**(b) Self-deadlock.** `File::lock()` is `flock(LOCK_EX)` on Linux. The docs say "No *other* file handle to this file, in this or any other process, may acquire another lock" [DOCS]. wsx opens several `Context`s in one process: `cmd_new` applies into the child, and `cmd_rm` and `cmd_list` plan every child. If each `Context` takes the lock, the second one blocks forever.

**(c) Cross-process deadlock.** `wsx new` holds the lock and runs `git worktree add`, which runs the post-checkout hook, which runs `wsx hook post-checkout`. That child waits on its parent. Under husky the hook currently exits early in a fresh worktree, but with plain `core.hooksPath`, planned for milestone 2, it does not.

**How to avoid:**
- One lock at `<common_dir>/wsx/lock`, held by a guard object that the top-level command acquires once and passes down. It covers plan and apply together.
- Hooks use `try_lock` with a bounded retry (well under the 30 s SessionEnd budget). On timeout they print "wsx skipped (another sync running)" and exit 0.
- Never hold the lock across a `git` subprocess that can run hooks. If unavoidable, mark the child with an environment token.
- The precondition re-check from Pitfall 2 still applies under the lock. It protects against non-wsx writers.

**Warning signs:**
- The lock is acquired inside `Context::new` or `plan()`.
- Tests never run two wsx processes against one common dir.

**Phase to address:** S. The deadlock cases belong in the harness, covering `new`, `rm` and `list` with a held lock, and a hook while `new` runs.

---

### Pitfall 4: Repository environment variables leak into git and gix calls inside hooks

**What goes wrong:**
N4 above [REPRODUCED]. In Rust it can get worse, in two opposite ways [SOURCE, gix 0.89.0]:
- `gix::discover()` **ignores** `GIT_DIR`. `discover_with_environment_overrides()` and `open_with_environment_overrides()` honour it.
- Default `open::Options` set `use_repository_local_environment = true`, so `GIT_INDEX_FILE` (mapped to `gitoxide.core.indexFile`) and `GIT_WORK_TREE` are applied during config init even by a plain `gix::open(sibling)`. Only `Repository::main_repo()`, `worktree::Proxy::into_repo*()`, submodules and clone switch it off (`without_repository_environment_overrides`).

Two consequences:
- In pre-commit, a guard that opens with plain discover reads `.git/worktrees/<id>/index` while `git commit -a` is building `index.lock`. That is the wrong index. `GIT_INDEX_FILE=…/index.lock` was observed.
- Code that opens a sibling worktree with `gix::open(path)` inside a hook inherits the hook's index or worktree overrides.

**How to avoid:**
- Open the current repository once with `discover_with_environment_overrides`, so the guard sees what git will commit.
- Reach siblings only through `repo.worktrees()` / `Proxy::into_repo()` / `main_repo()`.
- Every `std::process::Command` that spawns `git` for a *different* worktree must `env_remove` `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_COMMON_DIR`, `GIT_OBJECT_DIRECTORY`, `GIT_ALTERNATE_OBJECT_DIRECTORIES`, `GIT_PREFIX` and `GIT_NAMESPACE`. Use one helper that makes this impossible to forget.
- Fix `prune_orphan_excludes` to list live worktrees from the common dir (`worktrees/*/gitdir`), not by running git in each one.

**Warning signs:**
- `Command::new("git")` without the helper.
- `gix::open(` on a path that came from a worktree list.
- No harness case that runs a real hook in worktree A while worktree B exists.

**Phase to address:** G (the helper and open strategy), K (hook cases). This is the most important "differences from git binary" item for this project.

---

### Pitfall 5: The guard has more bypasses than the first-push gap

**What goes wrong:**
Taken together, these make "the guard blocks managed paths" untrue in common cases:
1. **Quoted paths (N2) [REPRODUCED].** Any path with non-ASCII or special characters passes. Python fix: `-z`. With gix: compare raw `BStr` paths.
2. **Tip-to-tip diff (N3) [REPRODUCED].** Use per-commit checks instead. For each commit in the range, check whether its **tree contains** any managed path. That covers merges too, and it is cheap with gix tree lookups.
3. **Husky exits before calling wsx [SOURCE, husky `main`].** husky's `h` runner does `[ ! -f "$s" ] && exit 0` *before* it sources `~/.config/husky/init.sh`. In a repo whose team ships only `.husky/pre-commit`, wsx's pre-push guard and the post-merge, post-checkout and post-rewrite syncs **never run**. `HUSKY=0` is checked after `init.sh`, so it does not disable wsx.
4. **wsx cannot read pre-push stdin.** `init.sh` is *sourced* before the team's `.husky/pre-push`, and both share stdin. A team hook such as `git lfs pre-push "$@"` needs the ref list, so consuming stdin would break LFS uploads. The first-push fix therefore has to compute its range from git state (`<base_ref>..HEAD`, or `HEAD --not --remotes`). Residual gap: `git push origin other-branch` checks HEAD, not `other-branch`.
5. **Fail-open on error.** Any error in a hook exits 0, including the guard, and a gix "unsupported" error (Pitfall 9) is such an error. In Rust, `panic = "abort"` in the release profile turns every panic in a hook into SIGABRT, so a commit is **blocked**. That violates constraint 2 in the other direction.

**How to avoid:**
- Guard logic gets its own module and its own tests for each bypass, each with a positive control.
- `doctor` reports whether `.husky/pre-push` exists and calls wsx.
- Keep `panic = "unwind"`, wrap hook dispatch in `catch_unwind`, and decide explicitly whether guard errors fail closed (CONCERNS already recommends this).

**Warning signs:**
- Guard tests that use only ASCII paths.
- Guard tests that only stage files, never commit and then remove.
- No test that runs through real husky `h`.

**Phase to address:** K.

---

### Pitfall 6: Oracle parity tests that encode the oracle's bugs, or detect nothing

**What goes wrong:**
"Rust output == Python output" passes on every defect the two share. Fixed defects show up as parity failures and get "fixed" back. Tests for permission defects pass vacuously when run as **root**, because `chmod 000` does not stop root (a likely case for container checks, `scripts/check-in-container.sh`).

**Why it happens:**
A black-box oracle has no notion of intent. Defects N1-N5 plus the three in CONCERNS mean at least eight known cases where Python is wrong.

**How to avoid:**
- Every harness case declares its expectation as one of:
  - `parity`: both binaries must agree.
  - `fixed:<defect>`: the **oracle must fail** this case and Rust must pass it. The oracle failure is the negative control that proves the test detects the defect.
  - `rust-only`.
- Each case asserts its own precondition first, for example `cat` of the unreadable file must fail. If it does not (running as root), the case **fails**; it does not skip.
- Assert on filesystem end state and exit codes. Compare stdout only for a small set of golden cases.
- Pin the oracle (Python 3.15, git 2.56). Python behaviour around non-UTF-8 stdout changed with UTF-8 mode, which is default in 3.15 and observed here with `sys.flags.utf8_mode=1`, `stdout.errors=surrogateescape`. An older Python would crash on the same file.

**Warning signs:**
- A case list with no `fixed:` entries.
- All cases green on the oracle.
- Tests using `sleep 1` for mtime ordering (use `touch -d`).

**Phase to address:** H. This must exist before any Rust code is compared.

---

### Pitfall 7: The harness tests the wrong binary, or the host

**What goes wrong:**
- `integrations/husky-init.sh` and the Claude and Copilot snippets call **`$HOME/.local/bin/wsx` by absolute path**, so the sandbox's `PATH` shim does not apply to hooks. Once hook cases exist, they exercise whatever is installed. With a non-isolated `HOME`, that is the user's real binary. (Checked: nothing is installed at `~/.local/bin/wsx` on this host today.)
- This host has a global `core.hooksPath = /home/denniyahh/.config/git/hooks` (the gitleaks hook CONCERNS saw).
- A harness run from inside a git hook, such as a pre-commit that runs tests, inherits `GIT_DIR` and `GIT_INDEX_FILE` and operates on the wsx repo itself.
- Coverage: `cargo llvm-cov` collects data from spawned binaries through environment variables (`LLVM_PROFILE_FILE` and others). A harness that calls `env_clear()` for hermeticity silently gets zero coverage [UNVERIFIED mechanism; verify in phase H].
- `cargo mutants` runs only `cargo test`. A bash harness outside `cargo test` never runs against mutants, so every surviving mutant looks like a weak unit test, not a harness gap.

**How to avoid:**
- The harness takes the binary under test as an argument and installs it as `$HOME/.local/bin/wsx` inside a temporary `HOME`.
- Set `GIT_CONFIG_GLOBAL` and `XDG_CONFIG_HOME`, `GIT_CONFIG_NOSYSTEM=1`, and unset all `GIT_*`, then assert that a canary hook in the real global hooks dir did **not** fire.
- Wire the harness into `cargo test` as an integration test, so mutants and coverage see it. Allow-list the `LLVM_*` variables through the scrub.

**Warning signs:**
- A hook test passes the same on the Python and Rust binaries before the Rust hook code exists.

**Phase to address:** H.

---

### Pitfall 8: The `base.json` v1→v2 upgrade destroys the only record of sync history

**What goes wrong:**
- **(a)** N5: a lone-surrogate key makes serde_json reject the whole file [SOURCE].
- **(b)** Python's `load_base` treats unparseable JSON as `{}` (`bin/wsx:212`). If the port copies that, then rewrites the file as v2, the base is gone for good. With an empty base every edit becomes a conflict, and every deletion since the last sync is **resurrected**. A missing base never deletes, so this is resurrection rather than loss.
- **(c)** During cutover both binaries may run against the same worktree, because hooks call whatever is at `~/.local/bin/wsx`. If v2 keeps the name `base.json` with a different `entries` shape, Python compares a hash with a dict, flags everything as a conflict, and rewrites v1 over the v2 stat cache.

**How to avoid:**
- Write v2 under a **new filename** and treat `base.json` as read-only upgrade input. Keep it until X.
- The upgrade reads v1 with a tolerant decoder that maps `\udcXX` back to byte `0xXX` (reversing surrogateescape), or else aborts loudly naming the offending keys.
- A parse failure is never an empty base. It is an error. In hooks that means skip with a warning.
- Store paths in v2 as bytes (base64 or a hex escape), not as `String`, so non-UTF-8 names round-trip.

**Warning signs:**
- `serde_json::from_str::<V1>(…).unwrap_or_default()`.
- The upgrade overwrites in place.

**Phase to address:** V. The decision about the filename and path encoding is needed before the first Rust build writes state on a real machine (phase C/S dogfooding).

---

### Pitfall 9: gix API churn, very new APIs, and silent divergence from git

**What goes wrong / facts [SOURCE, crates.io]:**
- **Churn.** gix has shipped a breaking 0.x minor about once a month: 0.76 (2025-12-22) through 0.89 (2026-10-08). Each release carries 2-3 `(BREAKING)` changelog sections. Main is mid-way through migrating the whole public API to `gix::Result`. MSRV is 1.88 (local rustc 1.99).
- **New APIs.** `Repository::config_file_mut()` (transactional config edit) first shipped in 0.88.0 (2026-09-25). `add_worktree()` and `remove_worktree()` (feature `worktree-mutation`) are in the 0.89.0 crate, though main's CHANGELOG still lists them as Unreleased and `crate-status.md` still marks remove as unimplemented. They are weeks old.
- **`add_worktree` behaves differently from `git worktree add`.** It requires an **existing** local branch or a detached commit (no `-b`), and it runs **no hooks**.
- **Reduced trust is silent.** In a repo not owned by the current user (containers, sudo), gix applies `filter_config_section = is_trusted`, `lenient_config = true` and `bail_if_untrusted = false`. Repository and worktree config is **silently dropped**, so `wsx.sourceBranch` falls back to `personal-workspace`. The git binary refuses with "dubious ownership" instead.
- **Unsupported repository formats.** Reftable repos fail ("unsupported storage backend, such as reftable"). SHA-256 repos need the `sha256` feature, and the defaults enable only `sha1`. In hook mode, either failure means the guard is silently off.
- **The `interrupt` feature** installs signal handlers through `gix-tempfile`. They would compete with wsx's own SIGINT policy [SOURCE, feature definition; behaviour UNVERIFIED].

**How to avoid:**
- Pin an exact gix version (`=0.89.x`) with `default-features = false` plus only `index`, `revision` (or tree access) and `sha1`. Upgrade gix deliberately, in its own commit, with the full harness.
- **Shell out to `git` for `worktree add/remove` and `config --worktree` in milestone 1.** The new gix APIs do not match the semantics wsx needs, and they are untested in the field.
- Open repositories with explicit trust handling: if trust is not `Full`, stop with a clear message rather than running on defaults.
- Map gix "unsupported" errors to a distinct exit path that the guard can treat as fail-closed.
- Keep all gix calls behind one small `GitRepo` trait. That keeps the churn contained, and the harness can then compare that layer against `git` output directly.

**Warning signs:**
- gix types appear in planner signatures.
- `gix = "0.89"` (caret) in `Cargo.toml`.
- A cold build that pulls gix's network or credential crates.

**Phase to address:** G.

---

### Pitfall 10: Path identity through symlinks (`/home` → `/var/home` on this host)

**What goes wrong:**
On this host `/home` is a symlink to `/var/home`. `HOME=/home/denniyahh`, but git reports `/var/home/...` for `--show-toplevel` and `worktree list` [REPRODUCED]. Rust `current_dir()` is physical, while user arguments and `$HOME`-derived paths are logical. Consequences:
- **`is_source` comparisons fail.** If Rust then treats the source as a dev worktree, `ensure_installed` excludes the managed paths in the *source*, and the personal branch can no longer commit them.
- **The excludesFile string compare churns.** `doctor` and `ensure_installed` compare the `core.excludesFile` **string** with a computed path (`bin/wsx:582`, `:761`). A different but equivalent spelling forces a reinstall on every hook. That reinstall triggers pruning (N4).
- **`canonicalize()` fails on paths that do not exist yet.** It errors if the path does not exist [DOCS], unlike Python's `Path.resolve()` (non-strict). `wsx new <path>` breaks.
- **gix and git spell the same path differently.** gix keeps the spelling it was given, while git canonicalises. Mixing gix paths with git-fallback output breaks equality.

**How to avoid:**
- Have one `canonical(path)` helper: it canonicalises the longest existing ancestor and appends the rest.
- Compare paths only after that helper, or by `(dev, ino)` for existing directories.
- Write config values in exactly the spelling the oracle wrote, so existing installs do not churn on cutover.

**Warning signs:**
- `==` between `PathBuf`s from different sources.
- The harness never runs from a symlinked cwd. Add a case where the sandbox is entered through a symlinked parent.

**Phase to address:** G.

---

### Pitfall 11: The stat cache trusts `(size, mtime)` and misses real changes

**What goes wrong:**
The planned `(size, mtime, hash)` cache can miss a content change. If a local change is missed, a later source change is planned as "changed upstream" and **overwrites the local edit**. That is data loss, not just a stale status.

**Why it happens:**
- `copy2`, `cp -p`, `rsync -t` and tar all **preserve mtime**, and a same-size edit through such a tool leaves `(size, mtime)` unchanged.
- Linux file timestamps come from the coarse clock, so two writes within one tick share an mtime [UNVERIFIED tick size on this kernel (7.2), which has multigrain timestamps].
- This is git's "racy clean" problem.

**How to avoid:**
- Key the cache on `(dev, ino, size, mtime_ns, ctime_ns)`. `ctime` cannot be set from userspace, and the atomic rename gives every write a new inode.
- Apply git's racy rule: an entry whose mtime is not strictly older than the time the snapshot was written must be re-hashed.
- Fall back to hashing on any doubt.
- Add proptest and mutation cases that write same-size content and then `touch -d` back to the old mtime.

**Phase to address:** V.

---

### Pitfall 12: Copy semantics that `shutil.copy2` provided for free

**What goes wrong:**
`std::fs::copy` copies content and permission bits only [DOCS]. It does **not** preserve mtime. Today the `lww` policy, the mtime-tie behaviour and the stat cache all rely on `copy2`'s mtime preservation. Dropping it changes which side wins `lww`.
- Python compares `st_mtime` as a float, which is below nanosecond precision. Rust's `SystemTime` is exact, so near-ties resolve differently. The harness must not build nanosecond-level ties as `parity` cases.
- `copy2` also copies xattrs (`user.*`). Copying them is a choice to make deliberately.

**How to avoid:**
- Copy through a temp file **in the destination directory**, so a rename across filesystems fails loudly with `EXDEV` rather than happening silently. Use a unique name (`O_EXCL`), hash while copying, then `set_times` (stable since 1.75), `fsync` the file, rename, and `fsync` the directory for state files.
- Skip `*.wsx-tmp*` in the walk. That part is already known.

**Phase to address:** C (parity), S (fsync and unique temp names).

---

### Pitfall 13: Managed paths that the team also tracks

**What goes wrong:**
Nothing stops a manifest entry such as `sync AGENTS.md` from naming a file that is **tracked** on the dev branch [CODE-READ: `plan()` never looks at the index]. A pull then overwrites the team's tracked file with personal content. The exclude file does not apply to tracked paths, so the change shows as a modification and the guard blocks every commit of it. This breaks "never write tracked files" in the way that matters most.

**How to avoid:**
`plan()` gets the set of tracked paths (from the gix index) and turns a managed path that is tracked into `skip` plus a loud warning. `doctor` reports it too.

**Phase to address:** C or S. It needs index access from phase G.

---

## Moderate Pitfalls

- **Exit-code and stream parity.**
  - Python: `Problem` exits 2, conflicts exit 1, an uncaught exception exits 1 with a traceback, Ctrl-C exits 130, and argparse usage errors exit 2.
  - Rust: a panic exits 101. `println!` **panics on EPIPE** (`wsx status | head -1`), because Rust ignores SIGPIPE by default. clap uses exit 2 for usage errors, which matches, but adding `--version` or `--help` changes behaviour the oracle lacks.
  - Prevention: one `main` that maps every outcome to the table above. Use `writeln!` with a broken-pipe-is-success rule. Add a harness case for each exit code. Phase C.
- **Manifest parser differences** [REPRODUCED both sides].
  - Python's `str.strip()` and `split()` treat `\x1c`-`\x1f` as whitespace. Rust's `char::is_whitespace` does not.
  - `PurePosixPath("./a/./b//c/").parts` gives `('a','b','c')`, while Rust `components()` gives `[CurDir, a, b, c]`. A leading `./` entry would match nothing.
  - Prevention: normalise to a `Vec<Normal>` of path parts in exactly one place, and run a differential proptest of the parser against `python3 bin/wsx` in phase H/C.
- **Displaying non-UTF-8 names.** Python 3.15 writes the raw bytes to stdout. `Path::display()` writes U+FFFD. Sort order differs as well (surrogates sort differently from bytes). Keep non-UTF-8 cases to filesystem-state assertions only. Phase C.
- **Symlink handling is broader than migration** [CODE-READ].
  - Any local symlink under a managed path, including an intentional `.state/big.db -> /scratch/...`, is "replaced" with a copy.
  - Turning a source file into a symlink makes it "absent" in the source, so every worktree deletes its copy.
  - A file on one side and a directory on the other makes `copy_file` fail mid-apply.
  - Prevention: decide the symlink policy explicitly, and port it bit-for-bit or list it as a `fixed:` divergence. Phase C.
- **SQLite under `lww`** [UNVERIFIED as a wsx scenario]. A WAL-mode database is three files (`db`, `db-wal`, `db-shm`). Deciding last-writer-wins per file, or copying while a process has the database open, can pair a database with the wrong WAL and corrupt it. Prevention: at minimum, detect `-wal` siblings and warn. Flag this for deeper research in phase S. It is the headline use case of `lww`.
- **Agent hook budgets** [DOCS]. Claude Code's SessionEnd hooks "share a 1.5-second budget", raised to match a per-hook `timeout` up to 60 s. The shipped snippet uses 30 s. A full rehash or a lock wait can be cancelled mid-apply, and the snippet's `>/dev/null 2>&1; true` hides it. Prevention: crash safety (Pitfall 12), a time-bounded hook mode, and conflict reporting through a file that `wsx status` surfaces. Phase K.
- **Worktree IDs collide.** A linked worktree whose admin dir is named `main` already collides (CONCERNS). Changing the naming scheme counts as an exclude-mechanism compatibility change, so pruning must recognise both old and new names. Phase G.

## Minor Pitfalls

- gix reads split and sparse index files but does not expand sparse-directory entries (`crate-status.md`:868-871) [SOURCE]. A guard that matches index entries by path could miss entries hidden inside a sparse directory. Low relevance for wsx.
- `relativeWorktrees` and `useRelativePaths` (git ≥ 2.48) change the content of `gitdir` files. Code that parses `worktrees/*/gitdir` directly, for example in the N4 fix, must resolve relative links [SOURCE, add.rs docs].
- `save_base` stamps local time with `%z`. Never assert on it, and keep `TZ` fixed in the harness.

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|---|---|---|---|
| Port `digest()` as `Option<Hash>` | Mechanical 1:1 port | Brings back the data-loss bug (Pitfall 1) | Never |
| Corrupt state reads as an empty base | Hooks "just work" | Silent history loss, mass resurrection (Pitfall 8) | Never |
| Plain `gix::open` everywhere | Simple | Hook-environment leakage (Pitfall 4) | Never inside hook paths |
| Shell out to `git` for worktree and config writes | Proven semantics, runs hooks | Process spawn cost, must scrub env | **Milestone 1: yes** |
| Bash harness outside `cargo test` | Reuses `sandbox.sh` | Invisible to mutants and coverage (Pitfall 7) | Only as a thin wrapper invoked from `cargo test` |
| `panic = "abort"` for a smaller binary | Size | Hooks block commits on panic (Pitfall 5) | Never |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|---|---|---|
| husky `h` runner | Assuming `init.sh` runs for every hook | It runs only when the team ships `.husky/<hook>`. `doctor` must check pre-push explicitly |
| pre-push stdin | Reading the refs to compute the range | Off-limits: the team's hook (for example git-lfs) needs them. Compute the range from git state |
| Claude Code hooks | Assuming they run one at a time, with no deadline | They run in parallel; SessionEnd has a ≤60 s budget, and a timed-out hook is cancelled |
| git hooks | Running git in sibling worktrees without scrubbing `GIT_DIR` | Use a scrubbed-env helper. Get the live-worktree list from the common dir |
| gix ↔ git fallback | Comparing their path strings | Canonicalise both sides (Pitfall 10) |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|---|---|---|---|
| Per-commit tree check over a whole history on first push | pre-push takes seconds | Bound the range by `base_ref` and check only managed prefixes | Large repos with no upstream |
| gix default features | Long cold builds, large dependency tree | `default-features = false` with an explicit list | Immediately (developer friction, CI time) |
| Lock held during a full rehash | Parallel agent hooks time out and skip | Stat cache first; hash outside the lock where safe, then re-check under the lock | Many worktrees or large `lww` state |

## Security Mistakes

| Mistake | Risk | Prevention |
|---|---|---|
| Treating the guard's fail-open as harmless | A gix "unsupported" error or a panic disables it silently | Guard errors fail closed (decision, recorded in CONCERNS) |
| Silent reduced-trust config filtering | Wrong source branch in a foreign-owned repo | Abort when trust is not `Full` |
| Writing manifest paths raw into ignore syntax | Pattern injection (CONCERNS, speculative) | Escape or reject `*?[!#` and trailing spaces in the parser |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---|---|---|
| Hook skips print to `/dev/null` (agent snippets) | Conflicts and skips are never seen | Persist the last hook outcome; have `wsx status` and `doctor` show it |
| "already up to date" when files were skipped as unreadable | False reassurance | Report `unreadable`/`skipped` counts on every run |

## "Looks Done But Isn't" Checklist

- [ ] **Unreadable handling:** verify a `chmod 000` **directory** case, not only a file case. Verify the test fails under root instead of passing.
- [ ] **Guard:** verify a non-ASCII path, add-then-remove in range, first push, and a run through real husky `h` with and without `.husky/pre-push`.
- [ ] **Hooks:** verify a hook in worktree A with a stale exclude leaves worktree B's `exclude.wsx-*` in place (N4).
- [ ] **Parity:** verify each `fixed:` case **fails on the oracle**. That is the negative control.
- [ ] **Locking:** verify `wsx new`, `rm` and `list` complete while two other wsx processes run, with no deadlock.
- [ ] **Upgrade:** verify a v1 `base.json` with a `\udcXX` key either upgrades losslessly or aborts and leaves v1 untouched.
- [ ] **Coverage/mutants:** verify the black-box harness runs under `cargo test`, and that `cargo llvm-cov` shows non-zero coverage for `cmd_*`.
- [ ] **Paths:** verify the harness passes when entered through a symlinked parent directory.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---|---|---|
| Source deleted by a false delete (Pitfall 1, N1) | HIGH | Only if the file still exists in another worktree or git history; `lww` backups do not cover the `sync` policy. Hence "back up before every delete" is cheap insurance |
| Pruned sibling excludes (N4) | LOW | Run `wsx setup` in each worktree; check `git status` for swept-in files |
| Lost base snapshot (Pitfall 8) | MEDIUM | Re-derive: treat matching hashes as synced and resolve the rest manually; keep the v1 file until cutover |
| Leaked commit (N2, N3) | HIGH once pushed | History rewrite on the team remote |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---|---|---|
| 6 Oracle encodes bugs, 7 non-hermetic harness | H (first) | `fixed:` cases fail on the oracle; global-hook canary never fires |
| 4 Hook env leakage, 9 gix churn and divergence, 10 path identity | G | Scrubbed-env helper is the only spawn path; harness case through a symlinked cwd; pinned gix |
| 1 Absent/unreadable, 12 copy semantics, 13 tracked overlap, parser and exit-code parity | C | State-enum proptest; directory-unreadable case; exit-code table cases |
| 2 TOCTOU, 3 locking, crash mid-apply, SQLite WAL | S | Interleaving test; concurrent-process test; kill-mid-apply test |
| 5 Guard bypasses, hook budgets | K | One test per bypass, each with a positive control |
| 8 Upgrade, 11 stat cache | V | Surrogate-key upgrade case; same-size `touch -d` case |
| Python deleted while hooks still call it | X | `~/.local/bin/wsx` swapped atomically; v1 file retained until the harness passes on the installed binary |

## Open decisions surfaced by this research (for the roadmapper or operator, not decided here)

1. Should guard internal errors fail closed? (CONCERNS recommends it; Pitfall 5 adds the panic and "unsupported" cases.)
2. The v2 state filename and how paths are encoded in it (Pitfall 8).
3. Whether non-UTF-8 filenames are supported or rejected loudly (Pitfalls 8 and 12).
4. The symlink policy beyond migration (Moderate: symlink handling).
5. What happens when the manifest overlaps tracked paths: skip-with-warning or hard error (Pitfall 13).

## Sources

- Probes on this host, 2026-10-17: Python 3.15.0, git 2.56.0, rustc 1.99.0, kernel 7.2.5 (Fedora Kinoite). Probes N1-N4, N5 (Python side), the Rust std predicates and whitespace, the hook environment dump, and path spellings via `/home`.
- gix 0.89.0 crate source (static.crates.io): `src/discover.rs`, `src/open/{repository,options,mod}.rs`, `src/config/cache/init.rs`, `src/config/mod.rs`, `src/repository/{mod,worktree}.rs`, `src/repository/config/mod.rs`, `src/worktree/{add,remove,proxy}.rs`, `Cargo.toml`.
- gitoxide main `ad0202e` (2026-10-17): `gix/CHANGELOG.md`, `crate-status.md`, `gix-ref/src/store/file/loose/reference/decode.rs`. https://github.com/GitoxideLabs/gitoxide
- crates.io gix version list (release dates, MSRV): https://crates.io/crates/gix/versions
- serde_json 1.0.154 `src/read.rs`:948-1015: https://crates.io/crates/serde_json
- husky runner script and installer: https://github.com/typicode/husky/blob/main/husky, https://github.com/typicode/husky/blob/main/index.js
- Rust std docs [DOCS]: https://doc.rust-lang.org/std/path/struct.Path.html, https://doc.rust-lang.org/std/fs/fn.copy.html, https://doc.rust-lang.org/std/fs/struct.File.html
- Claude Code hooks [DOCS]: https://code.claude.com/docs/en/hooks
- githooks(5) environment semantics: observed directly, not fetched.

---
*Pitfalls research for: Python→Rust (gix) rewrite of a git-worktree personal-file sync CLI*
*Researched: 2026-10-17*
