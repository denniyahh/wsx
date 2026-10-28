---
phase: 03-git-adapter
reviewed: 2026-10-27T18:21:14Z
depth: deep
iteration: 3
files_reviewed: 14
files_reviewed_list:
  - Cargo.toml
  - clippy.toml
  - deny.toml
  - src/git/mod.rs
  - src/git/reads.rs
  - src/git/repo.rs
  - src/git/spawn.rs
  - src/git/testing.rs
  - src/main.rs
  - tests/harness/check.rs
  - scripts/check.sh
  - scripts/install-tools.sh
  - scripts/check-in-container.sh
  - tests/gate_controls.rs
findings:
  critical: 1
  warning: 2
  info: 3
  total: 6
status: issues_found
---

# Phase 3: Code Review Report (iteration 3)

**Reviewed:** 2026-10-27T18:21:14Z
**Depth:** deep
**Files Reviewed:** 14
**Status:** issues_found

## Summary

This re-review covers iteration 2's five fixes (b6102ed, 64bd096, 4f3b68c, 9589841, 54c0576),
plus anything the fixes broke or newly exposed.

**Verdict on the five fixes.** Four are correct for the cases they target. One of them made
things worse:

- **Ceiling fix (b6102ed): correct.** `discovery_dir` was traced against git's rule that a
  ceiling equal to the start is ignored, for all of these cases, and agrees with git in each:
  - starting inside a non-bare `.git`;
  - starting inside a bare repository;
  - a nested invalid `.git` directory;
  - a relative `gitdir:` file. gix-discover 0.57's `from_gitdir_file` joins the value with the
    file's parent.
- **Implicit-worktree refusal (64bd096): incomplete.** It is correct when gix reports a
  worktree. It returns early when gix reports none, and that is exactly where the blocker
  below lives.
- **Object-store refusal (4f3b68c): correct for its two variables.** A third variable that
  changes git's object store, `GIT_COMMON_DIR`, is still not refused (WR-01).
- **Container `.tools` mask (9589841): correct.** A real-docker run of the mask was done by
  the orchestrator, not here.
- **Stale-tool install hint (54c0576): fixes the reported case, regresses another.** The
  hint is now wrong whenever cargo resolves `$CARGO_HOME/bin` ahead of `.tools/bin` (WR-02).

**How the findings were checked.** Every finding was measured, each with a control that must
give the opposite result:

- The git side used real git 2.56.0.
- The wsx side used the unit-test binary's own child tests (`ceiling_child`,
  `object_env_child`), re-executed with a hand-built environment.
- The cargo side used the real rustc 1.99.1 cargo.
- Scratch repositories were under `target/review-iter3-scratch` and were removed afterwards.
  No source file was modified.

Neither `scripts/check.sh all` nor the container gate was re-run. The orchestrator's tip
results (300 passed, 56 skipped) were taken as given.

## Critical Issues

### CR-01: A `GIT_DIR` that gix opens as bare, but git gives the current directory as its worktree, is accepted, including inside a hook

**Files:** `src/git/repo.rs:464-468` (`refuse_implicit_worktree` returns `Ok` when
`repo.workdir()` is `None`), with `src/git/spawn.rs:155-160` (`Git::from_process`)

**Issue:** suppose `GIT_DIR` names a non-bare git dir that is not called `.git`, and there is
no `GIT_WORK_TREE` and no `core.worktree`. Then:

- **git** takes the current directory as the worktree.
- **gix 0.90** reports no worktree at all, as if the repository were bare.
- **wsx:** `refuse_implicit_worktree` treats "gix has no workdir" as "a repository without a
  worktree is not checked". So `Repo::discover` opens the repository with
  `worktree() == None`, from any directory.

This is the normal shape of a repository made with `git init --separate-git-dir` or
`git clone --separate-git-dir`. git writes a `.git` file in the worktree and sets **no**
`core.worktree` (measured). Inside that repository's hooks, git exports
`GIT_DIR=<separate dir>`.

Measured with git 2.56.0. A real `pre-commit` hook in such a repository re-executed
`ceiling_child`; the commit was run from `r/sub`.

| setup | git | wsx |
|---|---|---|
| hook env: cwd `r`, `GIT_DIR=<store.git>` | `--show-toplevel` = `r`, `--is-bare-repository` = `false` | `opened`, `worktree=<none>` |
| control: cwd `r`, no `GIT_DIR` | `r` | `opened`, `worktree=r` |
| `GIT_DIR=<meta.git>` (core.bare=false or unset), cwd `r/sub` | toplevel `r/sub` | `opened`, `worktree=<none>` |
| control: same with `core.bare=true` | bare | `opened`, `worktree=<none>` (agree) |
| control: same with `GIT_WORK_TREE=r` | `r` | `opened`, `worktree=r` |
| `GIT_DIR=r1/.git GIT_IMPLICIT_WORK_TREE=0`, cwd `r1` | no worktree ("must be run in a work tree") | `opened`, `worktree=r1` |

**Failure scenario, traced in the code.** wsx is run from the pre-commit hook of a
separate-git-dir repository during a partial commit (`git commit -- path`). There, git exports
a temporary `GIT_INDEX_FILE`. Then:

1. `Repo::worktree()` is `None`.
2. `Git::from_process` builds a helper with no own worktree.
3. `Git::command` (`spawn.rs:239`) therefore counts the hook's own worktree `r` as "another
   worktree" and strips `GIT_DIR` and `GIT_INDEX_FILE`.
4. A wsx child aimed at `r` reads the real index, not the partial commit's index, while wsx's
   gix side (`index_path`) uses the hook's index.
5. A check run in the hook then judges content that is not the content being committed.

This breaks the locked decision that "a child aimed at wsx's own worktree keeps the hook's
environment". It also breaks the specific idea that "inside a hook, wsx must behave as git
does for its own worktree". The last table row is the reverse divergence, with an
internal-only variable: wsx opens a worktree where git has none.

**Fix:** decide the worktree the way git does before trusting gix's `workdir()`. With
`GIT_DIR` set, `GIT_WORK_TREE` unset and no local `core.worktree`:

- if local `core.bare` is `true`, or `GIT_IMPLICIT_WORK_TREE` is false, git has no worktree;
- otherwise git's worktree is the current directory.

Refuse whenever gix's answer differs from that:

```rust
fn refuse_implicit_worktree(repo: &gix::Repository) -> Result<(), Error> {
    let Some(value) = std::env::var_os("GIT_DIR") else { return Ok(()) };
    let local = |key| repo.config_snapshot().plumbing()
        .string_filter(key, |meta| meta.source == gix::config::Source::Local);
    if std::env::var_os("GIT_WORK_TREE").is_some() || local("core.worktree").is_some() {
        return Ok(());
    }
    let bare = local("core.bare").is_some_and(|v| v.eq_ignore_ascii_case(b"true"))
        || std::env::var_os("GIT_IMPLICIT_WORK_TREE").is_some_and(|v| v == "0" || v == "false");
    let cwd = canonical(repo.current_dir())?;
    let git_worktree = (!bare).then_some(cwd.clone());
    let gix_worktree = repo.workdir().map(canonical).transpose()?;
    if git_worktree == gix_worktree { Ok(()) } else {
        Err(Error::ImplicitWorkTree { git_dir: PathBuf::from(value), cwd, worktree: gix_worktree })
        // `worktree` becomes Option<PathBuf>; the message says "none (gix opens it as bare)".
    }
}
```

- Read `core.bare` with git's boolean parser, not by comparing strings; the snippet only shows
  the shape.
- Add a git-oracle test: a `--separate-git-dir` repository with `GIT_DIR=<store>` at the
  worktree root must be refused. git says not bare, with toplevel = cwd.
- Controls:
  - the same with `GIT_WORK_TREE=<root>` opens, with worktree `<root>`;
  - `core.bare=true` opens bare;
  - the existing hook tests in linked worktrees stay green. There gix derives the worktree from
    the `gitdir` back-link.

Opening the repository with the current directory as its worktree, rather than refusing it,
would also restore hook support for these repositories. That needs gix's open options to take
an explicit worktree, which this review did not check.

## Warnings

### WR-01: gix ignores `GIT_COMMON_DIR`, so git and wsx read different object stores, and only a relative value is refused

**File:** `src/git/repo.rs:315` (`OBJECT_STORE_VARS`), `src/git/repo.rs:410-415`
(`REPO_PATH_VARS` covers `GIT_COMMON_DIR` only when it is relative and wsx runs below the root)

**Issue:** this is the same class as iteration 2's object-store warning. gix 0.90's
`discover_with_environment_overrides_opts` documents the variables it honours:

- `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_SHALLOW_FILE` and `GIT_NAMESPACE`, once
  `GIT_DIR` has selected the repository;
- `GIT_DISCOVERY_ACROSS_FILESYSTEM` and `GIT_CEILING_DIRECTORIES` during discovery.

`GIT_COMMON_DIR` is not among them, and no gix crate reads it. git honours it with or without
`GIT_DIR`.

Measured with two one-commit repositories, `r1` and `r2`, cwd `r1`, and an absolute
`GIT_COMMON_DIR=<r2>/.git`:

| env | git | wsx (`object_env_child`) |
|---|---|---|
| `GIT_COMMON_DIR` only | `--git-common-dir` = `r2/.git`; `cat-file -t <r2 commit>` = `commit` | opens; `HEAD` reads as r1's commit |
| `GIT_DIR=r1/.git` + `GIT_COMMON_DIR` | `cat-file` reads r2's commit; `rev-parse HEAD^{commit}` fails (r1's commit is missing from r2's store) | opens; `HEAD` = r1's commit; r2's commit unreadable |
| control: `GIT_DIR=r1/.git` only | r2's commit unreadable; `HEAD` = r1's commit | same as git |

**Failure scenario:** a user or script has `GIT_COMMON_DIR` set, for example left over from
scripting a linked worktree by hand. wsx then:

- answers merge-base, ahead/behind and range walks from one store, while its own git children
  read and write another;
- reads `extensions.refStorage` from a different config file than git does.

git does **not** export `GIT_COMMON_DIR` to hooks. This was measured in a linked worktree's
`pre-commit`, whose environment held `GIT_DIR` and `GIT_INDEX_FILE` only. So refusing it
costs hooks nothing.

**Fix:** refuse an absolute `GIT_COMMON_DIR` too, unless it canonicalises to the common dir
gix found. That keeps a harmless, matching value working:

```rust
fn refuse_common_dir_env(repo: &gix::Repository) -> Result<(), Error> {
    let Some(value) = std::env::var_os("GIT_COMMON_DIR") else { return Ok(()) };
    match std::fs::canonicalize(&value) {
        Ok(dir) if dir == canonical(repo.common_dir())? => Ok(()),
        _ => Err(Error::ObjectStoreEnv { name: "GIT_COMMON_DIR", value: value.into() }),
    }
}
```

The `ObjectStoreEnv` message says gix "ignores" the variable, which also holds here. Add a
git-oracle test for the first row, where wsx must refuse, and a control with the variable
equal to the repository's own common dir, which must open.

### WR-02: The new install hint is wrong whenever cargo resolves `$CARGO_HOME/bin` first; the old hint was right there

**Files:** `scripts/install-tools.sh:119-129` (`install_hint`), `scripts/check.sh:18-31` (the
comment "cargo follows PATH order for its subcommands, so `cargo nextest` runs the same copy",
and the `WSX_TOOLS_DIR` export)

**Issue:** cargo puts `$CARGO_HOME/bin` **first** in its subcommand search when that
directory is not on `PATH` (cargo#11020). `check-in-container.sh`, `install-tools.sh`'s
header and `checkout_check_sh_test`'s comment all document this. In that configuration
`.tools/bin` does not win for `cargo nextest`. `verify-cargo` then reports the copy in
`$CARGO_HOME/bin`, yet the hint, since 54c0576, says to install into `.tools/bin`. Following
it changes nothing.

Before 54c0576 the plain hint installed into `$CARGO_HOME/bin`, which would have fixed this
case. So the fix moved the dead-end advice from one configuration to another.

Measured with real cargo (rustc 1.99.1) and copies of `check.sh` and `install-tools.sh` in a
scratch checkout:

- **Fixture:**
  - `.tools/bin/cargo-nextest` reports the pin, 0.9.148. This is the state after following
    the hint.
  - `$CARGO_HOME/bin/cargo-nextest` reports `0.0.1-stale`.
  - `PATH` holds the toolchain bin, `/usr/bin` and `/bin`, but not `$CARGO_HOME/bin`.
- **Result:** `check.sh test` exits 1 with:

  ```
  cargo-nextest run as 'cargo nextest' reports 'cargo-nextest 0.0.1-stale', expected 0.9.148;
  run WSX_TOOLS_DIR=<checkout>/.tools/bin scripts/install-tools.sh install cargo-nextest
  ```

  That is the directory that already holds the pinned copy.
- **Control:** the same fixture with `$CARGO_HOME/bin` appended to `PATH`. `verify-cargo`
  passes and the `.tools/bin` copy runs (`check.sh: test OK`). A bare-cargo probe confirmed the
  lookup order: `HOME_BIN` without it on PATH, `PATH_DIR` with it.

The gate still fails closed, so there is no false green, but the advice cannot work. The
check.sh comment is false in this configuration.

**Fix:** have the hint name the directory cargo actually resolves. The rule is deterministic,
so `install_hint` can reproduce it for `verify-cargo`:

```bash
install_hint() {
    local home_bin="${CARGO_HOME:-$HOME/.cargo}/bin" dir="${WSX_TOOLS_DIR:-}"
    case ":$PATH:" in
        *":$home_bin:"* | *":$home_bin/:"*) ;;
        *) if [ -e "$home_bin/$1" ]; then dir="$home_bin"; fi ;;  # cargo searches it first
    esac
    if [ -n "$dir" ]; then
        printf 'WSX_TOOLS_DIR=%q scripts/install-tools.sh install %s' "$dir" "$1"
    else
        printf 'scripts/install-tools.sh install %s' "$1"
    fi
}
```

Also correct the check.sh comment: `.tools/bin` wins for `cargo <sub>` only when
`$CARGO_HOME/bin` is on `PATH`. Add a scratch-checkout test for the fixture above. stderr must
name `$CARGO_HOME/bin`, not `.tools/bin`. The existing
`stale_repo_tools_nextest_hint_installs_into_repo_tools` is the counterpart. Its empty
`.cargo-home` has no `bin/cargo-nextest`, so its hint must keep naming `.tools/bin`.

## Info

### IN-01: The maintenance counterpart test still starts a detached background writer (carried from iteration 2)

**File:** `src/git/testing.rs:1174-1184` (with `commit_starts_auto_maintenance`, 1141-1160)

**Issue:** this was out of scope for the iteration-2 fix and is unchanged. With
`maintenance.auto=true`, the commit starts `git maintenance run --auto --detach` in a
`Scratch` that is deleted straight away. `TempDir` drop swallows removal errors, so a race
leaves a tree under `target/tmp` rather than failing the test.

**Fix:** pass `maintenance.autoDetach=false` alongside `maintenance.auto=true`. The trace2
assertion is unchanged.

### IN-02: check.sh's own "tool missing" hints ignore `WSX_TOOLS_DIR`

**File:** `scripts/check.sh:189,207,229,230` (`require_tool cargo-nextest "run
scripts/install-tools.sh install cargo-nextest"`, and the same for deny and llvm-cov)

**Issue:** 54c0576 made `install-tools.sh`'s three hints destination-aware. check.sh's own
`require_tool` hints still print the plain command. The two can disagree:

- `require_tool` checks with `command -v`, which searches PATH only.
- The plain command installs into `$CARGO_HOME/bin`.
- When that directory is not on `PATH`, the tool is still "not on PATH" after following the
  advice.

This is the same dead end as WR-02 for a missing tool rather than a stale one. It predates this
iteration.

**Fix:** have `require_tool` callers for the pinned tools defer to
`scripts/install-tools.sh verify-cargo`, which already prints the destination-aware hint. Or
build the hint from the same `install_hint` logic.

### IN-03: Three new refusals, and refusing wsx in receive-pack hooks, are not recorded for phase 5

**File:** `src/git/mod.rs:84-103` (`RelativeRepoEnv`, `ObjectStoreEnv`, `ImplicitWorkTree`)

**Issue:** CONTEXT names five error variants, "one variant per refusal: reftable, foreign
owner, git too old, not a repo, and a git failure", and leaves exit codes to phase 5 (IF-01).
The review-fix iterations added three more refusal variants. 4f3b68c also makes wsx refuse
inside `pre-receive`/`update` hooks: git exports `GIT_OBJECT_DIRECTORY` to receive-pack's
quarantine.

None of this is recorded in CONTEXT, STATE.md or deferred-items: a search of `.planning/`,
outside the review files and research, finds no mention. Phase 5 assigns an exit code to each
refusal, and nothing tells it these variants exist or that server-side hooks are refused by
design.

**Fix:** add these to CONTEXT's "Decided after research" list, or to the STATE.md concern
phase 5 reads:

- the three variants;
- the receive-pack refusal.

Whether refusing in receive-pack hooks is acceptable is a scope decision for the owner, not
for a review fix.

---

_Reviewed: 2026-10-27T18:21:14Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
_Iteration: 3_
