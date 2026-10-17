# Architecture Research

**Domain:** Rust CLI for three-way sync of personal untracked files across git worktrees (rewrite of wsx 0.1.0 at parity, then hardened)
**Researched:** 2026-10-17
**Confidence:** MEDIUM overall. The current-behaviour claims are HIGH: every `bin/wsx:N` citation below comes from lines read in this session. The Rust structure is a design recommendation (MEDIUM). gix capability claims are MEDIUM where context7 and the raw `crate-status.md` agree, and LOW where they rest on a docs.rs method-name listing only. Those need a spike.

## Things that cut against the obvious design (read first)

1. **The proposed `base.json` v2 shape (`path -> {size, mtime_ns, sha256}`) cannot cache both sides.** `base.json` is per dev worktree (`bin/wsx:144-145`), but `plan()` hashes two files per path, source and local (`bin/wsx:373-374`). They have different inodes, ctimes and often mtimes. One stat record lets you skip hashing only one of them. The recommended v2 entry keeps one agreed hash plus one stat record per side (see Data Flow, base.json v2).
2. **A stat cache must never authorise a destructive action.** If a racy stat says "local == base" and the planner then pulls over local, a user edit is lost. The cache may only short-circuit to "unchanged". Every overwrite or delete re-hashes its target immediately before acting (compare-and-swap; see Pattern 3).
3. **The planner does not need a git trait at all.** `plan()` reads only the manifest, base, directory walks, digests and mtimes (`bin/wsx:356-416`). It never calls git. The git trait belongs to the command layer. Its value there is choosing gix or the CLI for each operation, and allowing differential tests between the two. It is not what makes the planner testable.
4. **Today's `apply()` makes decisions.** Each of these must move into the plan:
   - lww backup (`if action.policy == "lww" and digest(...)`, `bin/wsx:461-462, 467-468`)
   - whether to write a conflict marker (`if not dry_run and digest(src_path)`, `bin/wsx:437`)
   - the converged base value (`digest(dst_path) or ""`, `bin/wsx:430`, the source of the poisoned-`""` defect)
   - marker cleanup after every applied action (`bin/wsx:478-480`)
   - a second base load, separate from the planner's (`bin/wsx:421` vs `bin/wsx:358`), which leaves a plan/apply TOCTOU gap
5. **`wsx resolve` duplicates executor logic.** It has its own backup, copy and base update (`bin/wsx:699-717`), and that copy is why it skips `materialise_parents()` (`bin/wsx:710`). In Rust, resolve should emit actions and run them through the one executor, so the executor's invariants hold there automatically.
6. **Husky hooks call `$HOME/.local/bin/wsx`, not `wsx` on PATH** (`integrations/husky-init.sh:28-29`; the Claude and Copilot snippets do the same). The sandbox's PATH shim (`tests/sandbox.sh:19-23`) therefore never reaches hook-driven runs. A black-box harness has to install the binary under test into the fake `$HOME/.local/bin/wsx`.
7. **Hook names must stay open-ended.** Python accepts any hook name and returns 0 for unknown ones (`bin/wsx:1153-1154, 1066`). A clap `ValueEnum` for the hook name would turn an unrecognised name into clap's exit 2, which blocks the commit. Parse `HookName` with an `Other(String)` fallback.

## Standard Architecture

### System Overview

```
┌────────────────────────────────────────────────────────────────────────┐
│ Invokers: shell, husky init.sh, Claude/Copilot session hooks, fish wrap│
└───────────────────────────────┬────────────────────────────────────────┘
                                ▼
┌────────────────────────────────────────────────────────────────────────┐
│ crate `wsx` (bin)  — ALL I/O lives here                                 │
│  main.rs: clap → Mode{Interactive|Hook} → exit-code mapping, hook driver│
│  cmd/*:   one module per subcommand (orchestration only, no decisions)  │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐    │
│ │ context  │ │  scan    │ │  apply   │ │  store   │ │ ignore/husky │    │
│ │(Git trait│ │(walk,stat│ │(executor,│ │(base.json│ │(exclude file,│    │
│ │ roots,   │ │ hash,    │ │ CAS,     │ │ load/save│ │ worktree cfg,│    │
│ │ source)  │ │ cache)   │ │ atomic)  │ │ + lock)  │ │ npm arm)     │    │
│ └────┬─────┘ └────┬─────┘ └────▲─────┘ └────┬─────┘ └──────┬───────┘    │
│      │ git/{gix,cli}│ Snapshot  │ Plan       │ Base         │            │
└──────┼────────────┼───────────┼────────────┼──────────────┼────────────┘
       │            ▼           │            ▼              │
┌──────┼────────────────────────┴───────────────────────────┼────────────┐
│      │  crate `wsx-core` (lib) — PURE: no fs, no git, no clock, no env  │
│      │  RelPath · Manifest/Policy · Obs/Snapshot · Base(v1,v2,upgrade)  │
│      │  plan() · select() · plan_resolve() · guard matching · exclude   │
│      │  rendering · (test-only) model::simulate()                       │
└──────┼──────────────────────────────────────────────────────────────────┘
       ▼
┌────────────────────────────────────────────────────────────────────────┐
│ External: git repo (gix reads; `git` CLI writes/worktree add+remove/   │
│ fetch), source + dev worktree files, <git-dir>/wsx/, <common-dir>/wsx/ │
└────────────────────────────────────────────────────────────────────────┘

crate `wsx-blackbox` (test-only, depends on NO wsx crate): spawns a binary
path (Python oracle or Rust) inside hermetic fixtures.
```

### Component Responsibilities

| Component | Owns | Python origin (read) | Talks to |
|---|---|---|---|
| `wsx-core::relpath` | `RelPath` newtype: relative, normalised, no `..`, no leading `/`; the only path type the planner accepts | validation `bin/wsx:202-203`; `resolve`'s `startswith("..")` defect `bin/wsx:690-692` | everything in core |
| `wsx-core::manifest` | `.wsx.conf` grammar → `Manifest`; `policy_for` longest match; managed-path list | `bin/wsx:184-207`, `274-282`, `600-606` | plan, guard, exclude |
| `wsx-core::base` | `Base` model, v1/v2 serde from bytes, v1→v2 upgrade | `bin/wsx:211-229` | store (bin), plan |
| `wsx-core::plan` | `plan(&Manifest, &Snapshot, &Base) -> Plan`; every sync decision | `bin/wsx:356-416` plus the decisions currently in `apply()` | select, model |
| `wsx-core::select` | direction filtering; the hook "should apply" rule | `bin/wsx:442-444`, `1069-1083` | cmd, apply |
| `wsx-core::resolve` | `plan_resolve(rel, Side, obs, base) -> Plan` | `bin/wsx:686-718` | apply |
| `wsx-core::guard` | `GuardVerdict` from changed paths + manifest + branch identity | `bin/wsx:985-1019` | cmd/guard, cmd/hook |
| `wsx-core::exclude` | render exclude-file text from manifest + inherited ignore text | `bin/wsx:512-536` | ignore (bin) |
| `wsx::git` | `trait Git` + `GixGit`, `CliGit`, `HybridGit` (per-op routing) | `bin/wsx:77-114` and the git calls in `cmd_*` | context, ignore, cmd |
| `wsx::context` | `Workspace { root, git_dir, common_dir, branch, source_branch, source }` | `bin/wsx:122-182` | git |
| `wsx::scan` | candidate set, `Obs` per side, `via_symlink`, stat cache use | `bin/wsx:237-271`, `285-298`, `361-365` | fs, base |
| `wsx::apply` | executor: runs `Action`s exactly, CAS preconditions, atomic writes, backups, returns `Applied` records | `bin/wsx:419-488`, `301-338` | fs, store |
| `wsx::store` | base.json load/save (atomic), lock file | `bin/wsx:211-229` | core::base |
| `wsx::ignore` | install/ensure/prune exclude files; worktree config writes via Git | `bin/wsx:496-597` | git, core::exclude |
| `wsx::husky` | `arm_husky` (npm), guard report | `bin/wsx:721-747`, `815-849` | subprocess |
| `wsx::cmd::*` | orchestration only: resolve context → lock → load → scan → plan → select → apply/render → save → report | `bin/wsx:614-1092` | all |
| `wsx::main` | clap, `Mode`, exit mapping, hook driver (`catch_unwind`) | `bin/wsx:1100-1181` | cmd |

## Recommended Project Structure

```
Cargo.toml                  # [workspace] resolver 3, workspace.lints, profile (keep panic = "unwind")
deny.toml
crates/
├── wsx-core/               # pure library; deps: serde, serde_json, thiserror (hashing itself happens in the bin; core holds a Sha256 newtype)
│   └── src/
│       ├── lib.rs
│       ├── relpath.rs      # RelPath newtype + proptest Arbitrary
│       ├── manifest.rs     # parser, Policy, policy_for, managed()
│       ├── obs.rs          # Obs, SideObs, PathObs, Snapshot
│       ├── base.rs         # Base, BaseEntry, StatKey, v1/v2 (de)serialise, upgrade
│       ├── plan.rs         # Action, ActionKind, Reason, plan()
│       ├── select.rs       # Direction, select(), hook_should_apply()
│       ├── resolve.rs      # plan_resolve()
│       ├── guard.rs        # GuardVerdict, guard_paths()
│       ├── exclude.rs      # render_exclude()
│       └── model.rs        # #[cfg(any(test, feature = "model"))] pure simulate(Snapshot, Plan) -> Snapshot
├── wsx/                    # the binary; deps: wsx-core, gix, clap, sha2, tempfile(?), ...
│   └── src/
│       ├── main.rs         # clap parse, Mode, exit mapping, hook driver
│       ├── exit.rs         # Exit enum, HookExit
│       ├── git/{mod,gix_impl,cli_impl}.rs
│       ├── context.rs
│       ├── scan.rs
│       ├── fsops.rs        # atomic_copy, materialise_parents, remove, backup, fsync helpers
│       ├── apply.rs
│       ├── store.rs        # base.json + lock
│       ├── ignore.rs
│       ├── husky.rs
│       ├── report.rs       # TTY-aware colour, fallible writes (never println!)
│       └── cmd/{setup,pull,push,sync,status,list,doctor,new,rm,resolve,guard,hook}.rs
│   └── tests/              # tempdir integration: scan/apply/fault-injection, git differential
└── wsx-blackbox/           # publish = false; NO dependency on wsx or wsx-core
    ├── src/{env,fixture,runner,observe,divergence,invariants}.rs
    └── tests/scenarios.rs
bin/wsx                     # Python oracle — deleted at cutover
tests/sandbox.sh            # retired once its 28 cases are ported into wsx-blackbox
```

### Structure Rationale

- **Workspace with a pure core crate, not a single lib+bin package.** The dependency graph then enforces purity: `wsx-core/Cargo.toml` has no gix and no I/O crates, so planner code cannot quietly start reading the disk. Add `clippy::disallowed_methods` for `std::fs::*`, `std::env::*` and `SystemTime::now` in core as a second fence. It also helps mutation testing. cargo-mutants by default runs only the mutated package's tests, and `--package` restricts what gets mutated (mutants.rs workspaces page; seam tier LOW, official docs). So `cargo mutants -p wsx-core` neither rebuilds nor re-tests the gix-heavy binary for each mutant. The author's devflow project uses the same core+cli workspace (`~/Github/devflow/Cargo.toml`).
- **`wsx-blackbox` must not link the implementation.** A harness that cannot import `wsx-core` cannot test internals by accident, and it can run the Python oracle and the Rust binary through the same code path. One trade-off: `CARGO_BIN_EXE_<name>` is set only for integration tests and benches (Cargo reference). For a crate other than the binary's own it is very likely not available (same-package scope, MEDIUM). So the harness takes the binary path from an env var (`WSX_BIN`, `WSX_ORACLE`), and a small script builds the binary and then runs the harness.
- **Git adapter lives in the binary crate.** It is a seam for the command layer, not a reusable library. Do not create a fourth crate for it.
- **One module per `cmd_*`** mirrors the Python layout (`bin/wsx:614-1092`), so a parity review can go one subcommand at a time.

## Architectural Patterns

### Pattern 1: Pure planner over an observation snapshot

**What:** `plan(&Manifest, &Snapshot, &Base) -> Plan` with no I/O. The scanner produces `Snapshot`, with every filesystem fact the planner may use already gathered.

**When to use:** Always. It is the load-bearing rule from `CLAUDE.md`, and the existing `--dry-run` guarantee (`bin/wsx:457-458`) depends on it.

**Trade-offs:** The scanner has to anticipate every fact the planner needs: mtimes for lww (`bin/wsx:402`), the symlink-ancestor flag (`bin/wsx:378`), whether a conflict marker exists (`bin/wsx:478-480`). In exchange the planner becomes a total function you can enumerate.

The observation type carries the absent-vs-unreadable fix structurally. Python's `digest()` collapses symlink, non-file and any `OSError` into `None` (`bin/wsx:237-248`), which is how an unreadable file becomes a planned deletion:

```rust
pub enum Obs {
    Absent,                                   // ENOENT only
    File { hash: Sha256, mtime_ns: i128 },    // regular file, hashed or cache-confirmed
    Symlink,                                  // leaf is a symlink (Python: treated as absent)
    NotRegular,                               // dir/fifo/socket where a file was expected
    Unreadable(std::io::ErrorKind),           // EACCES, EIO, ... — never "absent"
}
pub struct PathObs {
    pub source: Obs,
    pub local: Obs,
    pub local_via_symlink: bool,   // via_symlink(): any ancestor OR leaf is a link (bin/wsx:285-298)
    pub marker_present: bool,      // <rel>.wsx-remote exists locally
}
pub type Snapshot = BTreeMap<RelPath, PathObs>; // BTreeMap: sorted, as bin/wsx:368
```

`Obs::Symlink` and `Obs::NotRegular` keep the 0.1.0 meaning (absent), but the planner maps them to absent explicitly, in one visible match arm. Today that mapping is a side effect of `digest()`. `Unreadable` on either side produces `ActionKind::Blocked { reason }`: it is reported, it never mutates anything, and its base entry is left untouched.

### Pattern 2: Typed actions carry every decision; the executor only executes

**What:** Each `Action` variant carries all the data the executor needs: backup flags, expected hashes for compare-and-swap, the new base value, whether to clear a marker. The executor may branch only on those fields and on precondition failures. It never inspects policy or file state to pick a different action.

```rust
pub struct Action { pub rel: RelPath, pub policy: Policy, pub kind: ActionKind, pub reason: Reason }

pub enum ActionKind {
    Converged { base: BaseUpdate },                       // Set(hash) | Remove  (fixes the "" entry)
    Pull   { expect_src: Sha256, expect_local: Expect, backup_local: bool, replacing_symlink: bool },
    Push   { expect_local: Sha256, expect_src: Expect, backup_src: bool },
    DeleteLocal  { expect_local: Expect, backup_local: bool },  // lww: backup_local always true
    DeleteRemote { expect_src: Sha256 },
    Conflict { write_marker: bool },                       // false when source absent (bin/wsx:437)
    Blocked  { why: BlockReason },                         // Unreadable, etc. Never mutates.
}
pub enum Expect { Absent, Hash(Sha256), Symlink }
pub enum Direction { In, Out }   // ActionKind::direction(): ONE place (dup today at bin/wsx:442, 1077)
pub enum Reason { ChangedUpstream, ChangedLocally, BothChanged, Mirror, GoneUpstream,
                  AbsentLocally, NewerMtime, ReplacingSymlink }  // replaces free-text detail (bin/wsx:388-414)
```

`select(&Plan, &[Direction]) -> Vec<&Action>` is pure, and it keeps the 0.1.0 semantics: `Converged` and `Conflict` are emitted whatever the direction (`bin/wsx:429-440` run before the direction filter at `bin/wsx:442-444`). `--dry-run` renders `select()`'s output, so the backups and markers a real run would write also appear in the dry run.

**Rejected alternative:** a second layer of executor primitives (`Step::Backup`, `Step::Copy`, ...). Each `ActionKind` already maps to a fixed sequence of primitive operations, and adding the layer doubles the surface cargo-mutants has to cover for no new guarantee.

**Trade-off:** `ActionKind` gets wide. That is the price of moving the hidden decisions out of `apply()`.

### Pattern 3: Compare-and-swap executor

**What:** Before any overwrite or delete, the executor re-hashes the target (no stat-cache shortcut) and compares it with the action's `expect_*`. On a mismatch it skips the action, reports `changed during sync`, and makes no base change for that path. The next run plans it again.

**Why:** The lock serialises wsx runs, but not the agents and editors writing files while a `session-end` or `pre-commit` hook runs. Python deletes or overwrites on the strength of a hash taken earlier (`bin/wsx:460-476`). Re-checking narrows the window from the whole run to microseconds.

**Copy-with-hash:** when copying source to local, stream through sha256 into the temp file. If the bytes copied do not hash to `expect_src`, discard the temp file and skip. This replaces Python's post-copy `digest(dst)` base update (`bin/wsx:465, 470`), which records whatever happened to be copied.

**Trade-off:** the target file is hashed one extra time. It is paid only for files that actually change, which is few per hook run.

### Pattern 4: Exit codes and fail-open policy in types

0.1.0 exit behaviour, as read:

| Situation | Exit | Lines |
|---|---|---|
| success | 0 | |
| conflicts after setup/pull/push/sync, conflicts in status | 1 | `bin/wsx:630, 638, 646, 654, 683` |
| doctor problems | 1 | `bin/wsx:800` |
| guard blocks | 1 | `bin/wsx:1019` |
| `Problem` outside hook mode | 2 | `bin/wsx:1169-1170` |
| `Problem` or any exception in hook mode | 0 | `bin/wsx:1164-1168, 1173-1176` |
| unexpected exception outside hook mode | re-raised: Python traceback, exit 1, which collides with "conflicts" | `bin/wsx:1177` |
| Ctrl-C | 130, even in hook mode (blocks the commit) | `bin/wsx:1171-1172` |
| argparse usage error | 2 (argparse default; clap also uses 2) | |

Rust shape:

```rust
pub enum Exit { Ok = 0, Attention = 1, Problem = 2, Interrupted = 130 }

/// The ONLY way a hook can make git fail. Constructible only from a guard verdict.
pub struct HookExit(Inner); enum Inner { Allow, Block }
impl HookExit { pub fn allow() -> Self { Self(Inner::Allow) } }
impl From<GuardVerdict> for HookExit { /* Blocked(_) => Block, Pass => Allow, Indeterminate => policy */ }

fn hook_main(name: HookName, ws: Result<Workspace, Problem>) -> ExitCode {
    // AssertUnwindSafe: we discard all state on panic.
    match std::panic::catch_unwind(AssertUnwindSafe(|| run_hook(name, ws))) {
        Ok(HookExit(Inner::Block)) => ExitCode::from(1),
        _ => ExitCode::SUCCESS,                 // Allow, Err swallowed inside, or panic
    }
}
fn hook_sync(...) -> SyncReport  // infallible signature: errors become report lines, never Err
```

Rules the types enforce:

- `run_hook` returns `HookExit`, not `Result`. The sync half has an infallible signature, so no `?` can carry a sync error out to the exit code. `Block` can only be built from `GuardVerdict`, because the constructor is private to the module.
- `catch_unwind` needs `panic = "unwind"`. The workspace profile must never set `panic = "abort"`, or every panic in a hook blocks the commit. Add a test that runs a hook with a forced panic (behind a test-only cargo feature) and asserts exit 0. A missing `catch_unwind` must make that test fail.
- **Output in hook mode must be fallible.** `println!` panics when the write fails, and Rust ignores SIGPIPE by default, so a closed stdout (a common situation for hook output) becomes a panic. Route all output through `report.rs` using `writeln!` and ignore write errors.
- **All child processes get `Stdio::null()` for stdin.** Python's `subprocess.run` calls never pass `stdin`, so `git` and `npm` inherit the hook's stdin (`bin/wsx:78-83, 834-839`). Today the only guard against them swallowing pre-push refs is `</dev/null` in `husky-init.sh:29`. Rust should not rely on the caller for that (design constraint 3).
- **Guard verdict:** `GuardVerdict { Pass, Blocked(Vec<RelPath>), Indeterminate(Problem) }`. 0.1.0 maps an internal guard error to allow, because the guard runs inside the hook catch-all (`bin/wsx:1163-1168`; CONCERNS "guard fails open"). Keeping `Indeterminate` as a separate variant lets that be a one-line policy decision. See the open decisions below.
- **Ordering in hooks:** the guard runs before the lock is taken and never waits on it. A guard skipped because of lock contention would be a silent hole.

## Data Flow

### Request flow (sync / pull / push / setup / hook-sync)

```
argv ─▶ clap ─▶ Mode ─▶ Workspace::resolve(&dyn Git)            [git reads: gix]
                          │
                          ▼
                 Lock::acquire(<common-dir>/wsx/lock)            [exclusive; hook: bounded try]
                          │
       ┌──────────────────┼─────────────────────┐
       ▼                  ▼                     ▼
 Manifest::parse   store::load_base()     (Base also feeds scan's stat cache)
 (source/.wsx.conf)  v1→v2 in memory
       │                  │
       └────────┬─────────┘
                ▼
        scan(&Workspace, &Manifest, &Base) ─▶ Snapshot          [fs reads; hash only on stat miss]
                ▼
        plan(&Manifest, &Snapshot, &Base) ─▶ Plan               [PURE]
                ▼
        select(&Plan, directions) ─▶ Vec<&Action>               [PURE]
                ▼
     ┌── dry-run? ──▶ render ─▶ exit
     ▼
  apply(&Workspace, actions) ─▶ Vec<Applied{rel, base_update, side_stats}>   [fs writes, CAS]
                ▼
  base' = base.merge(applied)  ─▶ store::save_base() (atomic)   [PURE merge, then 1 write]
                ▼
           Lock::release ─▶ report ─▶ Exit
```

Data only moves downward. Nothing below `plan()` feeds a decision back up. `apply()` returns observations (what it wrote, with fresh stats), never new decisions.

### State: base.json v2

0.1.0 v1 (`bin/wsx:219-229`): `{version: 1, source, updated, entries: {rel: sha256 | ""}}`. The `""` values come from `bin/wsx:430, 465, 470, 704, 711`.

Recommended v2:

```json
{
  "version": 2,
  "source": "/abs/path/to/source",
  "updated": "2026-10-17T12:00:00+0000",
  "entries": {
    ".planning/PLAN.md": {
      "sha256": "<agreed content at last sync>",
      "local":  { "size": 812, "mtime_ns": 1760700000123456789, "ctime_ns": 1760700000223456789, "ino": 4194305 },
      "source": { "size": 812, "mtime_ns": 1760700000123456789, "ctime_ns": 1760699999000000000, "ino": 4194999 }
    }
  }
}
```

- **Meaning of a stat record:** "when this side had exactly this (size, mtime_ns, ctime_ns, ino), its content was `sha256`". If the stat matches, the scanner returns `Obs::File { hash: entry.sha256 }` without reading the file. Otherwise it hashes. Each record is optional.
- **`ino` and `ctime_ns` are essential.** Atomic replace (temp + rename) gives a new inode, and every in-place write bumps ctime. Copies preserve mtime, as `shutil.copy2` does (`bin/wsx:320`), and parity needs that because lww compares mtimes (`bin/wsx:402`). So mtime alone collides often.
- **Racy-stat rule (git's racy-index problem):** do not store a stat record whose `mtime_ns` or `ctime_ns` falls within a safety margin (about 2 s) of the scan time. Leave it empty and hash again next run. A just-written file is therefore cached on the second run, not the first.
- **v1→v2 upgrade (in memory, on load):** `sha` becomes `{sha256: sha}` with no stat records. `""` is dropped, which also fixes existing poisoned entries. That is safe: with no base entry, a later one-sided recreate reads as "changed locally" and pushes, per `bin/wsx:406-412`. Before the first v2 write, copy the original to `base.v1.json.bak` once, for rollback.
- **Unknown future version:** refuse with a `Problem` (hook mode: skip, exit 0). Never overwrite it.
- **Corrupt JSON:** 0.1.0 silently treats it as empty (`bin/wsx:214-215`). Keep that, because an empty base is safe (see the invariants), but warn and keep the bad file as `base.json.corrupt-<stamp>`.
- **Python cannot read v2.** `load_base` would accept the dict values (`bin/wsx:216-217`), and every non-equal path would then read as a conflict, since a dict never equals a digest string (`bin/wsx:406-414`). That is spurious conflicts, not data loss, but it means **the harness must never let the oracle and Rust share state**, and rolling back means restoring `base.v1.json.bak`.

### Locking scope

- **One lock per repository, at `<common-dir>/wsx/lock`.** Per worktree is not enough: the source worktree is found among this repo's own worktrees (`bin/wsx:139-142`), so every dev worktree writes into the same source tree. `<common-dir>` is git metadata, not a tracked file (design constraint 4).
- **Primitive:** `std::fs::File::lock` / `try_lock`, stable since Rust 1.89 and implemented with `flock` on Unix. Verified on this host (rustc 1.99): a second descriptor in the same process got `TryLockError::WouldBlock` while the first held the lock, then acquired it after unlock. No lock crate is needed.
- **Held across:** the manifest and base load, scan, plan, apply and base save, for `setup pull push sync resolve`, the child sync inside `new` and `rm`, and hook sync. Python's plan and apply each load base separately (`bin/wsx:358, 421`). In Rust, one `Base` value flows through under the lock.
- **Not held across subprocesses that can fire git hooks** (`npm install` in `arm_husky`, `bin/wsx:834-839`; `git worktree add`, `bin/wsx:877`). flock locks belong to an open file description, so a nested `wsx hook` would block on its own parent's lock. With bounded waits it would time out and silently skip its sync. This is the reentrancy hazard the probe above demonstrated.
- **Interactive commands** wait a bounded time (e.g. 10 s), then fail with a `Problem` that says which run holds the lock (write the PID and command into the lock file).
- **Hooks** retry `try_lock` for a short bounded time (about 1-2 s), then skip the sync with a one-line note and exit 0, failing open.
- **Read-only commands** (`status list doctor guard`) take no lock in milestone 1. A torn read can only misreport, never mutate. The lock exists to serialise writers.
- **`ensure_installed`** writes the exclude file, `extensions.worktreeConfig` and the worktree config, and `prune_orphan_excludes` deletes other worktrees' files (`bin/wsx:539-570`). Run it under the same lock. In hooks, if the lock is busy, skip it and still run the guard.

### Atomic writes and crash-recovery invariants

Mechanics: write a temp file in the same directory, named `.<name>.wsx-tmp-<pid>-<rand>` and created with `O_EXCL`. Copy while hashing, set permissions and mtime as `copy2` does, `fsync` the file, then `rename` over the target. For `base.json` and backups, also `fsync` the parent directory. This fixes the fixed `name.wsx-tmp` / `base.json.tmp` names (`bin/wsx:227, 319`) that two concurrent runs share.

| # | Invariant | Why it holds / how it is enforced |
|---|---|---|
| I1 | A managed path holds either its old or its new content, never a partial write | temp + rename. `scan` skips both the legacy `*.wsx-tmp` suffix and the new pattern. 0.1.0's `walk()` does not, so leftover temp files get synced (`bin/wsx:264-266`). An apply run holding the lock deletes orphan temp files it finds. |
| I2 | `base.json` may lag the files but never leads them | Base is saved once, after all actions (as `bin/wsx:483-484`), and only from successful `Applied` records. Every completed pull/push leaves the two sides byte-equal, and every completed delete leaves both absent. The planner classifies equal sides as converged without consulting base (`bin/wsx:380-382`). So a crash after any subset of actions recovers on the next run: done actions read as converged, and undone ones plan again unchanged. |
| I3 | A backup is durable before the overwrite or delete it protects | Backup, fsync, then the replace. Backup names need sub-second uniqueness (ns stamp plus counter); today's per-second stamps collide (`bin/wsx:333-338`). |
| I4 | Conflict markers never affect base | marker write/remove is a separate field on the action |
| I5 | An empty or corrupt base can never produce a delete under `sync` or `lww` | Property-tested. True of the 0.1.0 `sync` logic (`bin/wsx:406-412`), and true of `lww` once a local delete requires a base entry (CONCERNS fix). `pull` mirrors and may delete by design (`bin/wsx:391-393`). |
| I6 | No local mutation, write or delete, goes through a symlinked ancestor | The executor checks this as a precondition after `materialise_parents`. 0.1.0 checks only before pull writes (`bin/wsx:463`); `delete-local` goes through `remove_file` and its parent `rmdir` loop (`bin/wsx:324-330`). |
| I7 | Migration from a symlinked layout never leaves siblings "absent" | **[CODE-READ, not reproduced]** `materialise_parents` unlinks the symlinked dir and creates an empty one (`bin/wsx:307-312`). A crash before every sibling is pulled leaves them locally absent. If base still holds `== source` entries for them, the next `push` plans `delete-remote` (`bin/wsx:410-412`), which is source data loss. Rust should materialise by copy-and-swap: build a real temp dir holding copies of the source files, then rename it over the link. Pitfalls research should try to reproduce this. |

Ctrl-C in hook mode: install a SIGINT flag (std has no signal API; this needs `signal-hook` or `rustix`, a STACK decision). The executor checks the flag between actions, stops, saves base for what completed, and exits 0. Invariant I2 makes stopping between actions safe.

### Git access: trait and per-operation routing

```rust
pub trait Git {
    fn discover(&self, cwd: &Path) -> Result<RepoPaths>;                 // root, git_dir, common_dir
    fn head_branch(&self, root: &Path) -> Result<Option<String>>;
    fn config_get(&self, root: &Path, key: &str, scope: Scope) -> Result<Option<String>>;
    fn config_set(&self, root: &Path, key: &str, val: &str, scope: Scope) -> Result<()>;
    fn worktrees(&self, root: &Path) -> Result<Vec<WorktreeInfo>>;        // path + branch
    fn staged_paths(&self, root: &Path) -> Result<Vec<String>>;
    fn changed_paths(&self, root: &Path, from: &str, to: &str) -> Result<Vec<String>>;
    fn upstream(&self, root: &Path) -> Result<Option<String>>;
    fn rev_exists(&self, root: &Path, rev: &str) -> Result<bool>;
    fn worktree_add(&self, root: &Path, path: &Path, branch: &str, base: &str) -> Result<()>;
    fn worktree_remove(&self, root: &Path, path: &Path, force: bool) -> Result<()>;
    fn fetch(&self, root: &Path, remote: &str) -> Result<bool>;
    fn tracked_dirty(&self, root: &Path) -> Result<Vec<String>>;
}
```

| Operation | 0.1.0 call (read) | Route | Basis |
|---|---|---|---|
| discover, branch, common dir | `bin/wsx:124-131` | gix | standard gix open (MEDIUM) |
| config reads incl. worktree scope | `bin/wsx:134, 497, 560, 586, 804` | gix | crate-status: "read per-worktree config if `extensions.worktreeConfig` is enabled" (MEDIUM: context7 + raw file agree) |
| global/system `core.excludesFile` (defect fix) | `bin/wsx:496-501` | gix with source filtering, or CLI `--global` | spike: check gix honours `GIT_CONFIG_GLOBAL`/`HOME` the way git does (unverified) |
| config writes (`extensions.worktreeConfig`, `--worktree core.excludesFile`) | `bin/wsx:563, 567` | **CLI** | crate-status: gix "write configuration back [ ]". gix-config can edit losslessly, but the CLI owns `config.lock` (MEDIUM) |
| worktree list | `bin/wsx:98-114` | gix `worktrees()` + main | API name present in gix 0.89 (LOW); differential test against `--porcelain` |
| staged paths | `bin/wsx:980-982` | gix `tree_index_status`, else CLI | method listed on docs.rs gix 0.89, but crate-status still shows "tree with index [ ]" (LOW, conflicting) → **spike** |
| range paths for pre-push | `bin/wsx:997, 1046-1048` | gix `diff_tree_to_tree` + `branch_remote_tracking_ref_name` | names present (LOW); crate-status "handle upstream and push resolution [x]" |
| worktree add | `bin/wsx:877` | **CLI** | gix lists "create linked worktrees [x]", but parity with `-b` + checkout + hooks is cheaper via CLI |
| worktree remove | `bin/wsx:946` | **CLI** | crate-status: "move, remove, and repair linked worktrees [ ]" (MEDIUM) |
| fetch | `bin/wsx:874` | **CLI** | credential helpers, ssh config |
| dirty check before rm | `bin/wsx:936` | **CLI** | `status --porcelain` semantics (untracked + ignore rules) are parity-critical |

`HybridGit` holds a `GixGit` and a `CliGit` and routes each method. The differential tests in `crates/wsx/tests/git_parity.rs` run both implementations of every read method against the same fixture repo and assert equal results. Each needs a control that must differ: for example a worktree-scoped config value, which a reader that ignores `config.worktree` would miss. **Pin guard path semantics before switching to gix.** `git diff --name-only` includes deletions and, with rename detection on, reports only the new name of a rename. gix's tree/index diff must reproduce exactly that set, or the guard's coverage changes silently.

## Test Architecture

### Layers

| Layer | Target | Tooling | Proves |
|---|---|---|---|
| L1 core unit + property | `wsx-core` | proptest, cargo-mutants `-p wsx-core` | decision table correctness, invariants I5, never-lose-an-edit |
| L2 executor/scan integration | `wsx` with tempdirs | std tests, test-only fault injection | I1-I4, I6, CAS, stat-cache correctness |
| L3 git differential | `wsx::git` | real temp repos, both impls | gix ≡ CLI per operation |
| L4 black-box | built binary | `wsx-blackbox` | parity with the oracle, the fixed defects, end-to-end hooks |

**L1 properties (planner).** Generators: an arbitrary `Manifest` over a small path alphabet, so nested and `skip` overrides collide often; an arbitrary `Snapshot` with `Obs` drawn over all five variants; an arbitrary `Base`, including empty.

1. **Never lose an edit (`sync`).** Any action that overwrites or deletes a side must expect that side's hash to equal base. Equivalently: if `local != base` and `source != base`, the action is `Conflict`.
2. **`Unreadable` on either side gives `Blocked`, with no mutation and no base change.** Defect #1.
3. **Empty base never yields `DeleteLocal` or `DeleteRemote` under `sync`/`lww`.** I5; covers defect #2 for lww.
4. **Both sides absent gives `Converged { base: Remove }`.** Defect #3.
5. **Convergence.** `plan(simulate(snap, plan(snap)))` contains only `Converged`, `Conflict` and `Blocked`. `model::simulate` is the pure reference for the executor, and L2 checks the real executor against it on tempdirs.
6. **Determinism and coverage.** Output is sorted, with exactly one action per non-`skip` candidate (`bin/wsx:368-371`).
7. **Selection.** `select` never returns an `In` action for an `[Out]` run, and still includes every `Conflict`/`Converged`.

Plus table-driven unit tests transcribing each branch of `bin/wsx:376-414`, so cargo-mutants has exact expectations to kill against. Run `cargo mutants -p wsx-core -f 'crates/wsx-core/src/plan.rs'` with zero surviving mutants as the bar. Run the manifest parser under proptest against the grammar from `bin/wsx:189-206`: round-trip, rejects `..` and absolute paths, and the line number appears in errors.

**L2 fault injection.** A cargo feature `fault-injection` (off in release) reads `WSX_FAULT=after-action:N` or `WSX_FAULT=before-rename:N` and calls `std::process::abort()` at that point. Tests loop N over every action of a multi-action plan, rerun wsx without the fault, and assert three things: the end state equals the no-fault end state, nothing is lost, and no temp file survives. Negative control: a deliberately broken variant (saving base before the file write) must fail at least one N.

**Unreadable-file tests need a precondition check.** As root, `chmod 000` does not make a file unreadable (CAP_DAC_OVERRIDE). The test must first confirm the file is actually unreadable and fail loudly, not pass vacuously, when it is not.

### L4 black-box harness

**Hermetic environment per scenario** (`Command::env_clear()`, then set only these):

- `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` → per-scenario tempdir
- `GIT_CONFIG_GLOBAL=<tmp>/gitconfig`, `GIT_CONFIG_NOSYSTEM=1`. This drops host `core.hooksPath`, the global hooks that fired during the CONCERNS probing, and a host `core.excludesFile` (the sandbox inherits all of these, `tests/sandbox.sh:21-26`).
- `GIT_AUTHOR_*`/`GIT_COMMITTER_*` with fixed dates, `TZ=UTC`, `LC_ALL=C`, `PATH=<shim>:<dir of git>:<dir of python3>`
- binary under test installed as **both** `<shim>/wsx` and `$HOME/.local/bin/wsx` (husky and agent snippets use the latter, `integrations/husky-init.sh:28-29`)
- the real `integrations/husky-init.sh` installed at `$HOME/.config/husky/init.sh`, plus a fake `.husky/_/` shim set, so `pre-commit`/`pre-push`/`post-*` hooks are exercised through git itself (today they have zero coverage; TESTING.md)
- `GIT_*` repo variables are gone by construction (`env_clear`)

**Every scenario starts from a fresh fixture.** `sandbox.sh` sections share state in order (`tests/sandbox.sh:70-152`), so one divergence cascades into every later section. Port the 28 checks as independent scenarios built from shared setup functions.

**Observed outcome** (compared structurally, not as text):

- exit code
- content map `rel -> sha256` of both worktrees' managed trees, including `*.wsx-remote` markers
- backup count per rel
- semantic base map `rel -> sha256`. The harness reads both v1 and v2: it is reading external state, not linking the implementation.
- `wsx status` classification as a set of `(kind, rel)` pairs, parsed from the `kind rel` columns (`bin/wsx:673-676`)

**Universal invariants checked after every scenario, for both implementations:**

- tracked files unchanged: `git status --porcelain` on tracked paths and the index, versus the pre-run snapshot (constraint 4)
- no temp file anywhere in either tree
- **stdin untouched.** Give the child a regular file as stdin through a dup'd descriptor that shares the file offset. After exit, the parent's offset must still be 0 (constraint 3). Negative control: one test-only scenario runs `cat` in place of wsx and must move the offset.

**Known-divergence encoding:**

```rust
pub enum Divergence {            // one variant per defect deliberately fixed in Rust
    UnreadableLocalDeletesSource,   // CONCERNS Known Bugs #1; bin/wsx:237-248, 406-412
    LwwDeletesLocalOnlyFile,        // #2; bin/wsx:396-398
    BothDeletedLeavesEmptyBase,     // #3; bin/wsx:380-382, 429-431
    FirstPushSkipsGuard,            // bin/wsx:1045-1051
    GlobalExcludesFileDropped,      // bin/wsx:496-501
    LwwMtimeTieGoesToPush,          // bin/wsx:402
    ResolveTheirsSkipsMaterialise,  // bin/wsx:710
    BackupStampCollision,           // bin/wsx:333-338
    NewDerivesRemoteFromLocalRef,   // bin/wsx:873
    RmMainWorktreeDetection,        // bin/wsx:918
    CtrlCBlocksHook,                // bin/wsx:1171-1172
    TempFilesSynced,                // bin/wsx:264-266, 319
}
pub enum Expect {
    Same(Outcome),                                  // both impls must match it
    Diverges { d: Divergence, oracle: Outcome, rust: Outcome },
}
```

Rules:

- **Expectations are explicit data, never "whatever the oracle did".** Pure A-vs-B differential testing loses all of its assertions the day Python is deleted. With explicit outcomes, the oracle column becomes documentation at cutover and the Rust column keeps running.
- **For `Diverges`, the oracle must still show the defect.** This is the negative control. It proves the scenario actually reproduces the bug. If the oracle stops reproducing it (a scenario that rotted), the test fails.
- **Completeness.** A test iterates every `Divergence` variant and requires at least one scenario to reference it. Any scenario where the two implementations differ without a `Divergence` tag fails.
- The harness is selected with `WSX_IMPL=oracle|rust|both`. CI after cutover runs `rust` only.

## Suggested Build Order

Each step lists what it depends on. Steps 3 and 5 can run in parallel.

1. **Black-box harness, against the Python oracle only.** Depends on nothing in Rust. Deliverables: hermetic env, fresh-fixture scenarios porting the 28 sandbox checks, hook paths via the real `husky-init.sh`, universal invariants, and the `Divergence` registry, with every defect scenario proven to reproduce on the oracle. This is the executable spec. It also closes the "green 28/28 says nothing" gap before any Rust exists.
2. **Workspace skeleton + `wsx-core` types.** `RelPath`, `Manifest` parser (compatible grammar, proptest), `Policy`/`policy_for`, `Base` v1/v2 serde + upgrade, `Obs`/`Snapshot`. Depends on: nothing. Sets up lints, `deny.toml`, `clippy -D warnings`.
3. **Pure planner.** `plan`, `select`, `plan_resolve`, `model::simulate`, `guard_paths`, `render_exclude`, plus the L1 properties and cargo-mutants at zero survivors. Depends on: 2. This is the core of the milestone, and Python's decision logic gets transcribed here, branch by branch.
4. **FS layer: scan + executor + store.** Walk/skip rules, `via_symlink`, the stat cache with the racy rule, atomic copy-with-hash, `materialise_parents` (copy-and-swap), backups, CAS, base save, lock, fault-injection tests. Depends on: 3 (consumes `Action`, produces `Snapshot`).
5. **Git adapter.** The trait, `CliGit` first (fast parity), then `GixGit` for each read op behind differential tests, and the `Workspace` resolution. Depends on: 2 only. Includes the gix spikes: `tree_index_status`, worktree enumeration, global-scope config read.
6. **Core commands: `setup pull push sync status resolve`.** Orchestration only. Run the L4 harness in `both` mode for these scenarios. Depends on: 4, 5.
7. **Ignore + guard + hook driver.** `ensure_installed`/`install_ignore`/prune, the global excludesFile fix, `GuardVerdict`, the first-push range, `HookExit` + `catch_unwind`, SIGINT handling, the hook lock policy. Depends on: 5, 6. This is the step with the most hook-environment risk; CLAUDE.md requires a manual pass against a real repo here.
8. **Lifecycle: `new rm list doctor` + `arm_husky`.** Depends on: 6, 7. The remote-derivation and main-worktree-detection fixes land here.
9. **Cutover.** The whole harness green with `WSX_IMPL=both`; every `Divergence` confirmed; `install.sh` points at the Rust binary; manual pass (`wsx doctor`, `wsx new`, `wsx rm`) on a real repo; one real-machine base.json v1→v2 upgrade with `base.v1.json.bak` kept; delete `bin/wsx` and `tests/sandbox.sh`; switch the harness to `rust`. Depends on: all.

**Ordering rationale:** the harness comes first because every later phase needs a parity signal, and building it against the oracle alone keeps it from being bent to fit the Rust code. The planner comes before the FS layer because the executor's input type is the planner's output. Git work is independent of the core and can overlap with steps 3-4. Hooks come late because they combine everything, and they are where fail-open mistakes show up.

## Scaling Considerations

The unit of scale is managed files, worktrees and hook frequency. It is not users.

| Scale | Adjustment |
|---|---|
| tens of planning docs, a few worktrees | 0.1.0's full rehash is fine; the stat cache is the only change needed |
| large `lww` binaries (multi-GB SQLite) | the stat cache avoids hashing twice per hook (`bin/wsx:356-416`); the CAS re-hash costs only for files actually changing; backup retention is deferred to milestone 2 |
| many worktrees (`wsx list`) | `list` plans every worktree (`bin/wsx:958-976`). With the cache it is mostly stats. If needed later, parallelise per worktree with scoped threads; no async runtime |

**First bottleneck:** hook latency from spawned git processes. Context alone runs 6 or more git calls (`bin/wsx:124-139`), and `ensure_installed` adds more (`bin/wsx:585-597`). Reading through gix removes most of them. **Second:** hashing large files. The stat cache handles that.

## Anti-Patterns

### Branching on file state in `apply()`
**What people do:** "just check if the file exists before backing up". 0.1.0 already does this (`bin/wsx:461, 467, 437`).
**Why it's wrong:** `--dry-run` no longer shows what a real run does, and cargo-mutants cannot reach the decision from pure tests.
**Do this instead:** add a field to `ActionKind`, decide it in `plan()`, and let `apply()` read the field.

### Treating every read error as absent
**What people do:** `fs::read(...).ok()`, or matching only `Ok`/`Err`.
**Why it's wrong:** this is exactly defect #1, where an unreadable file becomes a planned deletion of the source.
**Do this instead:** map `ErrorKind::NotFound` to `Obs::Absent`, every other error to `Obs::Unreadable`, and let the planner block.

### Trusting the stat cache for destructive actions
**What people do:** skip hashing because the stat matches, then overwrite on that basis.
**Why it's wrong:** racy timestamps and coarse filesystem mtime let edits through unseen.
**Do this instead:** CAS re-hash before every overwrite or delete (Pattern 3).

### `println!` / `?`-to-exit in hook paths
**Why it's wrong:** a broken pipe panics, and an `Err` that reaches `main` maps to non-zero. Either blocks the commit and violates constraint 2.
**Do this instead:** fallible writes, an infallible `hook_sync` signature, and `catch_unwind` in `hook_main`.

### Differential-only harness
**What people do:** assert `run(oracle) == run(rust)`.
**Why it's wrong:** at cutover the oracle is deleted and every assertion goes with it. A defect present in both implementations passes.
**Do this instead:** explicit `Expect` values plus a check that the oracle agrees.

### Holding the lock across hook-firing subprocesses
**Why it's wrong:** a nested `wsx hook` blocks on its parent's flock (reentrancy is not per process).
**Do this instead:** release the lock before `npm install` and `git worktree add`, and take it again for the child sync.

## Integration Points

### External services

| Service | Pattern | Notes |
|---|---|---|
| git (repo data) | gix for reads; `git` CLI for config writes, worktree add/remove, fetch, porcelain status | all subprocesses get `stdin(Stdio::null())` and an explicit `cwd`; parse `--porcelain`/`-z` output only |
| husky | `~/.config/husky/init.sh` → `$HOME/.local/bin/wsx hook <name> </dev/null` | installed path is load-bearing (`integrations/husky-init.sh:28-29`) |
| npm | `arm_husky` subprocess (`bin/wsx:815-849`) | outside the lock; it runs arbitrary `prepare` scripts (CONCERNS) |
| Claude/Copilot hooks | `wsx hook session-start|session-end` | output discarded by the callers; must exit 0 |

### Internal boundaries

| Boundary | Communication | Notes |
|---|---|---|
| `cmd` ↔ `wsx-core` | plain values (`Manifest`, `Snapshot`, `Base` → `Plan`) | core never sees `Path`, only `RelPath` |
| `scan` ↔ `store` | `&Base` (read-only stat cache) | scan never writes base |
| `apply` → `store` | `Vec<Applied>`; merged purely, saved once | base never leads files (I2) |
| `cmd` ↔ `git` | `&dyn Git` | not a hot path; dynamic dispatch is fine |
| `wsx-blackbox` ↔ binaries | process boundary only | no crate dependency, by construction |

## Open decisions (for requirements/discuss, not settled here)

These are design choices the evidence does not settle. They are not defects.

1. **Should the guard fail closed on an internal error?** Today it fails open (`bin/wsx:1163-1168`). CONCERNS recommends failing closed, and it is not in the PROJECT.md Active list. The `GuardVerdict::Indeterminate` variant keeps this a one-line policy choice either way.
2. **How should an lww mtime tie resolve?** Today a tie goes to push (`bin/wsx:402`, strict `>`). Options: fall back to the three-way base comparison, or report a conflict. The fix requirement exists; the rule does not.
3. **base.json v2 shape.** The recommendation above is two stat records per entry, versus the single record the question proposed (see "Things that cut against the obvious design", item 1).
4. **Stdout text parity.** The recommendation is that the contract covers only exit codes, filesystem effects and the status classification, not message wording.

## Sources

- `bin/wsx` lines 50-1181, read in windows this session. Every `bin/wsx:N` citation above (HIGH).
- `tests/sandbox.sh:1-45` and section headers; `integrations/husky-init.sh:20-30`; `integrations/claude/settings-snippet.json`, `integrations/copilot/wsx-session.json` (rg hits for `.local/bin/wsx`) (HIGH)
- `.planning/PROJECT.md`, `CLAUDE.md`, `CONTRIBUTING.md`, `.planning/codebase/{ARCHITECTURE,STRUCTURE,TESTING,CONCERNS}.md`
- gitoxide `crate-status.md` (context7 `/gitoxidelabs/gitoxide`, cross-checked against raw GitHub main) for worktree create/remove, worktreeConfig read and config write-back (MEDIUM)
- docs.rs `gix` 0.89.0 `Repository` method list: `tree_index_status`, `diff_tree_to_tree`, `worktrees`, `branch_remote_tracking_ref_name`, `rev_parse_single` (LOW: names only, summarised by the fetch model) → spike in build step 5
- Rust std `File::lock`/`try_lock` docs (since 1.89, flock on Unix), plus a local probe on rustc 1.99 with a contention control (verified)
- mutants.rs "Workspaces" (per-package test scoping, `--package`, `--file`) (LOW, official docs via webfetch)
- Cargo reference, environment variables, `CARGO_BIN_EXE_<name>` (LOW via webfetch; same-package scope is inferred, not stated)
- `~/Github/devflow/Cargo.toml`: the author's existing core+cli workspace convention

---
*Architecture research for: wsx Rust rewrite (milestone 1)*
*Researched: 2026-10-17*
