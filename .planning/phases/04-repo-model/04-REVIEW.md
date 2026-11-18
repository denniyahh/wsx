---
phase: 04-repo-model
reviewed: 2026-11-18T05:11:00Z
depth: deep
files_reviewed: 8
files_reviewed_list:
  - src/git/config.rs
  - src/git/mod.rs
  - src/git/objects.rs
  - src/git/refs.rs
  - src/git/repo.rs
  - src/git/spawn.rs
  - src/git/testing.rs
  - src/model/mod.rs
findings:
  critical: 1
  warning: 3
  info: 4
  total: 8
  necessity: 1
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-11-18T05:11:00Z
**Depth:** deep
**Files Reviewed:** 8
**Status:** issues_found

## Summary

The review covered the production code of the eight listed files (changes since `375348f`): the spawn helper and its new per-child deadline (D-39), the `git config` read, the `for-each-ref` listing, the `cat-file --batch` reads, the replace-ref guard, and the model built on them. The tests were reviewed only for how reliable they are.

One defect breaks a guarantee this phase promises. D-39 says a fifo at `.git/config` or `packed-refs` stops wsx instead of hanging it. That holds only for reads made by a git child. wsx (through gix at `Repo::discover`) and the replace guard both open those files in-process, with no deadline. This was measured with the compiled test binary (`model::tests::state_child`) against scratch repositories:

- With a fifo at `packed-refs`, `resolve` was still running when `timeout 20` killed it, after 20 s.
- With a fifo at `.git/config`, it was still running when `timeout 6` killed it.
- In both runs no git child existed, and the test thread sat in the kernel's `wait_for_partner`, which is a blocking open of a fifo.
- The control was the same repository with a regular `packed-refs`. It resolved in 0.75 s.

A second measured defect: a repository whose `.git/config` is missing, which git itself still operates, fails `resolve` with a canonicalize error as soon as any global `remote.*` or `wsx.*` key prints.

These parts were traced and hold:

- **Batch output parsing:** `missing` and `<oid> submodule` are git's actual strings, and `%(objectmode)` is empty for a request that is not `<tree-ish>:<path>`.
- **`%(symref)`** gives the end of the chain, as the model assumes.
- **Defined remotes:** local or worktree scope matches `git remote add`'s `configured_in_repo`.
- **Prefix patterns:** `for-each-ref` treats a trailing `/` as a prefix.
- **Model order:** the order `build` raises its errors in matches its doc.

Necessity was assessed against `necessity-review.md`. The replace-ref guard in `build` over-refuses for phase 4's git-only object reads. D-24 keeps it for the gix reads by id in `reads.rs`, which exist now, so it is not a necessity finding.

## Critical Issues

### CR-01: A fifo at `.git/config` or `packed-refs` hangs wsx in-process, though D-39 says it is stopped

**File:** `src/git/repo.rs:34-45`, `src/git/repo.rs:54-72`, `src/git/refs.rs:299-308` (claim at `src/git/spawn.rs:31-35`)
**Issue:** `CHILD_DEADLINE` bounds only git children. Two reads happen in-process before or beside them:

- `Repo::discover` → `open_gix` loads config and lists replace refs while it opens the repository. gix-0.90.0 `src/open/repository.rs:531-548` calls `refs.iter()`, which reaches gix-ref-0.69.1 `src/store/packed/buffer.rs:80-81`: `std::fs::metadata(path)?.len() <= threshold`, then `std::fs::read(&path)`.
- `refuse_replace_refs` → `packed_refs_name_the_base` calls `std::fs::read(path)` itself (`refs.rs:300`).

Opening a fifo with no writer blocks forever, and no deadline applies to this thread. The measurements are in the Summary: no git child was running, and the thread was blocked in `wait_for_partner` for both shapes. D-39 names exactly these two shapes (`04-CONTEXT.md` D-39), so the phase's stated guarantee is false for them.

The one test of the deadline on the config path, `config_read_stops_on_a_fifo_branch_under_an_onbranch_include` (`config.rs:1132-1143`), calls `Repo::discover` before it plants the fifo. It cannot catch this.
**Fix:** Refuse what is not a regular file before any in-process open, and route the guard through the same check. For example, in `discover` before `open_gix` and in `packed_refs_name_the_base`:
```rust
fn refuse_special_file(path: &Path) -> Result<(), Error> {
    match std::fs::metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Io { what: format!("stat {}", path.display()), source }),
        Ok(meta) if meta.is_file() => Ok(()),
        Ok(_) => Err(Error::Io {
            what: format!("read {}", path.display()),
            source: std::io::Error::other("not a regular file; wsx does not open it"),
        }),
    }
}
```
Apply it to the git dir's and the common dir's `config`, `config.worktree` and `packed-refs`. Also cover loose entries under `refs/replace/`, which gix also opens at open time. An alternative is to run `discover` and the guard on a worker thread and wait for it with `recv_timeout(CHILD_DEADLINE)`. Whichever you choose, add a test that plants the fifo *before* `discover`. Use `reexec_within` so that a regression fails the test instead of hanging the suite.

## Warnings

### WR-01: `collect` waits for pipe EOF and kills only the direct child, so a process git leaves behind turns a success into `GitTimedOut` and outlives the kill

**File:** `src/git/spawn.rs:172-201`, `src/git/spawn.rs:246-259`
**Issue:** `collect` waits for both reader threads, which end only at EOF, before it looks at the exit status (`183-201`). EOF needs every holder of the write end closed, and descendants of git inherit it. So a git that exits 0 while a descendant still holds stdout or stderr is reported as `GitTimedOut` after 10 s. On timeout, `child.kill()` (`175`) sends SIGKILL only to git's own pid, because `child()` sets no process group. A hook or helper git started keeps running, and its reader threads leak.

No phase-4 command starts a descendant, so phase 4 does not reach this. `Git` is the one constructor of every git child, though, and later phases run `git push`, `commit` and hooks through it. Here is one case, reasoned rather than measured: ssh with `ControlPersist`, whose background master inherits stderr.
**Fix:** Spawn each child as its own process group (`std::os::unix::process::CommandExt::process_group(0)` in `child()`) and kill the group on timeout (`rustix::process::kill_process_group`). Decide success from the child's exit, not from pipe EOF: once `try_wait` reports an exit, give the readers a short grace period, then stop waiting on them. Add a test whose fake git backgrounds `sleep` with the pipe held, then exits 0.

### WR-02: The `RefsNamespace` refusal gives a cause that is no longer true

**File:** `src/git/mod.rs:295-305` (pinned by the test at `src/git/mod.rs:549`)
**Issue:** The message says "wsx reads refs through gix, which reads every ref inside the namespace … so wsx would not read the refs git reads". After the pivot, the model lists every ref through `git for-each-ref` (`refs.rs:4-5`, `model/mod.rs:339-347`). This phase rewrote the doc of the refusal itself to give the new reason, which is gix's object reads by id and its replacement map (`repo.rs:563-566`), but left the message unchanged. A user who hits the refusal is told a reason the code does not have. That goes against the module's rule that every refusal names its cause (`mod.rs:16-17`).
**Fix:** Reword the message to the reason in `repo.rs:563-566`, for example: "gix, which wsx uses to read objects by id, applies the namespace to those reads and to the replace refs it loads when it opens the repository, while git ignores it". Update the test at `mod.rs:549` to match.

### WR-03: A repository without `.git/config` fails `resolve` as soon as any global `remote.*` or `wsx.*` key prints

**File:** `src/git/config.rs:86-91`
**Issue:** `read_config` canonicalizes `<common dir>/config` whenever git printed any record, whether or not a `wsx.*` record of scope `local` needs it. git operates a repository whose `config` file is missing. When such a record comes from global config, `resolve` fails with `canonicalize …/.git/config: No such file or directory`.

This was measured: a repository with `config` removed and a global `[remote "up"]`, run through `model::tests::state_child`, gave `state=failed` with that error, while `git status` in the same repository worked. The control, the same layout with `config` present, gave `state=not-configured`. The function's own doc limits a failure in a repository that is not configured to config "that git itself cannot read" (`model/mod.rs:367-369`).
**Fix:** Treat a missing own file as "no own records": map `NotFound` from the canonicalize to an `own_config` that matches nothing (for example `Option<PathBuf>`, with `is_own_file` returning `Some(false)` when it is `None`). Or canonicalize lazily on the first local `wsx.*` record. Add the measured case as a test.

## Info

### IN-01: Early error returns in `collect` leave the child running with no deadline

**File:** `src/git/spawn.rs:187`, `src/git/spawn.rs:194-199`, `src/git/spawn.rs:203-205`
**Issue:** A reader's I/O error, a panicked reader (`Disconnected`) and a `try_wait` error all return through `?` or `return` without `kill` and `wait`. Dropping a `Child` neither kills it nor reaps it. These paths are hard to reach (an EIO on a pipe, a thread panic), but on them the D-39 rule that every child is stopped does not hold.
**Fix:** Route every error exit after `spawn` through the `timed_out`-style cleanup (`let _ = child.kill(); let _ = child.wait();`), for example with a small guard struct that kills on `Drop` unless it has been disarmed.

### IN-02: Impossible-state guard: the 4-answer `try_from` check in `read_tips`

**File:** `src/git/objects.rs:303-311`
**Category:** necessity
**Issue:** `parse_batch(&stdout, 4)` either returns `Err` or pushes exactly one answer per index for `count` iterations (`objects.rs:76-113`), so its `Ok` always holds exactly 4 answers. The `map_err` that renders "N answers to 4 requests" can never run. This rests on judgment: it is a cheap way to destructure.
**Fix:** Have `parse_batch` return `[BatchAnswer; N]` through a const generic, or keep the line and drop the error text, which is unreachable (for example `.expect("parse_batch returns count answers")`, if the project allows that in non-test code).

### IN-03: The `git` module's doc still says reads go through gix

**File:** `src/git/mod.rs:1-2`
**Issue:** "repository resolution and reads through gix, history changes through the `git` binary". After the pivot, the config read, the ref listing and the model's object reads all go through the `git` binary (`config.rs`, `refs.rs:4-5`, `objects.rs:1-2`). gix is used only for discovery, the guard's config snapshot and `reads.rs`.
**Fix:** Say which reads go through git and which through gix.

### IN-04: The test helpers find the real git only in `/usr/local/bin`, `/usr/bin` or `/bin`

**File:** `src/git/testing.rs:416-430`
**Issue:** `real_git()` panics on a host whose git lives elsewhere (Homebrew/Linuxbrew, Nix, mise). So `counting_git` and every child-count test fail there for a reason unrelated to the code. This does not happen in the pinned container.
**Fix:** Resolve `git` through the hermetic env's PATH, minus `fakebin`, or note the requirement in `CONTRIBUTING.md` beside `check-in-container.sh`.

---

_Reviewed: 2026-11-18T05:11:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
