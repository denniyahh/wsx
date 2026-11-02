# Phase 4: Repo model - Research

**Researched:** 2026-11-02
**Domain:** gix 0.90.0 config, reference, tree and quote reads; the `.wsx.conf` grammar; one resolved repo model in the binary crate
**Confidence:** HIGH for gix and git behaviour and the grammar (each executed this session with a control). MEDIUM for the type layout (a working prototype, but layout is a judgement call).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Config keys and sources
- **D-01:** wsx reads its `wsx.*` keys from local repo config only. A global or system value is
  ignored, so it can't reconfigure unrelated repos or make them "configured" for the machine-wide
  hook.
- **D-02:** The remote keys are `wsx.baseRemote` and `wsx.pushRemote`, resolved independently. Each
  defaults to `origin` only when unset. git's `remote.pushDefault` is not read: it is normally
  global, and it means "any `git push`", not "where cut PR branches go".
- **D-03:** A key naming a remote that doesn't exist is an error naming the key and the remote.
- **D-04:** With no key and no `origin` remote, the repo has no remote of that kind. This holds even
  if it has a remote under another name.
- **D-05:** `upstream` is never auto-detected. A fork layout sets `wsx.baseRemote=upstream`, which is
  how Success Criterion 2's "both default to `origin`" is read.

#### Integration-branch chain
- **D-06:** The chain reads the base remote's tracking refs only:
  `refs/remotes/<baseRemote>/develop`, then `.../next`, then the remote default from the local
  `refs/remotes/<baseRemote>/HEAD`. A stray local `develop` can't redirect the base.
- **D-07:** Local branches count only when there is no base remote (D-04): local `develop`, then
  local `next`.
- **D-08:** `wsx.baseRef` is a bare branch name, resolved on the base remote (or locally when there
  is no remote). `upstream/next` means a branch literally named that. If the ref is missing, it's an
  error naming the ref tried, with no fallback to the chain.
- **D-09:** If nothing in the chain exists, it's an error listing the refs tried and the fixes
  (`git remote set-head <remote> --auto`, or set `wsx.baseRef`). Resolution makes no network call and
  never guesses `main` or `master`.
- **D-10:** A stale `<remote>/HEAD` is accepted. The owner reconfirmed this after the correction
  under Specific Ideas: it only matters in a repo with no develop, no next and no `wsx.baseRef`, and
  that repo can set `wsx.baseRef`.

#### Manifest grammar (`.wsx.conf`)
- **D-11:** A line starting with `#` is a comment, and an empty line is ignored. There are no trailing
  comments. Any other unknown line is an error naming the line.
- **D-12:** An invalid manifest fails closed for every command including the push guard, so only
  ambiguity is invalid, never staleness. A cross-kind overlap is an error naming both lines: `only a`
  with `modified a`, or `only a` with `modified a/b` (either direction). Exact duplicates and nested
  lines of the same kind parse fine.
- **D-13:** A declaration already covered by a built-in must still parse, because Phase 5 reports it
  as a named `wsx check` failure (DIV-02).
- **D-14:** Paths are literal, with no glob matching, so `*` is an ordinary byte. One trailing `/` is
  stripped, as the oracle does. These are errors naming the line, since they could never match a git
  tree path: an empty path, `.`, a `.` or `..` component, a leading `/`, and `a//b`.

#### Workspace-branch fallback
- **D-15:** A fallback match names the branch, and the repo stays **not configured**. "Configured"
  means only that local config has `wsx.sourceBranch` (CONF-07). This resolves the tension between
  Success Criteria 3 and 5: the model reports "not configured" plus the inferred branch.
- **D-16:** In a not-configured repo, commands may use the inferred branch's manifest if it's
  readable. A missing or invalid manifest there reads as "not configured, run `wsx init`", not
  fail-closed. Failing closed (Success Criterion 5) applies only when `wsx.sourceBranch` is set. The
  guard exits 0 in a not-configured repo, so a stray `personal/foo` in an unrelated repo can't block
  pushes.
- **D-17:** Candidate selection takes the set of branches recorded by `wsx new` as an input, proven by
  a test that passes one in. Production passes an empty set until Phase 8. No `refs/wsx/` record
  format is defined here.
  - **Obligation on Phase 8:** supply the real set. Until then, CONF-05's exclusion is proven only at
    the input, not end to end.

### Claude's Discretion
Calls stated to the owner before they chose to move on. They can be challenged at plan review.
- **D-18:** An empty or repeated `wsx.*` key in local config is an error, not git's last-one-wins.
- **D-19:** Manifest line edges:
  - Everything after the keyword and one space is the path, so `only my dir` means `my dir`.
  - A path with leading or trailing whitespace must be C-quoted; unquoted, it's an error.
  - A line ending in CR is an error.
  - An empty or comments-only manifest is valid and leaves only the built-ins.
  - A `.wsx.conf` at the tip that isn't a blob (symlink, directory) is invalid.
- **D-20:** Phase 4 reads `wsx.worktreeDir` (local config, default `.worktrees/`, same path rules as
  D-14), because the built-in list needs it. The rest of CONF-06 stays in Phase 8.
- **D-21:** Fallback edges:
  - Zero or several candidates is an error listing them for commands that need a workspace branch.
    The guard still sees "not configured" and exits 0.
  - Candidates are local `refs/heads/workspace/*` and `refs/heads/personal/*` only. Remote-only
    branches don't count, and the checked-out branch gets no preference.
  - `wsx.sourceBranch` is a short branch name. If that branch is missing locally, wsx fails closed.

The module layout, the model's types and how errors extend `git::Error` are left to research and
planning.

### Deferred Ideas (OUT OF SCOPE)
- Exit codes and `--json` for every error here: Phase 5 (IF-01).
- The `refs/wsx/` record format and the real recorded set for D-17: Phase 8 (WTREE-05).
- The guard's behaviour on "not configured": Phase 6, reading the model's state.

Reviewed Todos (not folded):
- **Re-check Dependabot after first merge to main:** repo tooling, matched on keywords only.
- **Replace oracle-observed exit 128 under IF-01:** needs Phase 5's exit codes and is applied in
  Phases 7 and 9.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CONF-01 | `.wsx.conf` declares `only`/`modified`, raw-byte paths C-quoted as git does, proptest round trip, path-boundary matching, unknown kinds name the line | gix's `quote`/`undo` match git byte for byte, so no dependency. Pattern 5, Pitfalls 6-7. Prototype proptest passed and caught a planted defect |
| CONF-02 | Built-ins are only `.wsx.conf` and the worktree dir; one resolved list | `Model::is_personal` over built-ins plus `only` lines. `modified` is not personal (Pattern 7) |
| CONF-03 | Integration branch from `wsx.baseRef`, else `develop`, `next`, remote default | Exact full-name reads and the symbolic target of `<remote>/HEAD` (Pattern 3). Seven shapes of `<remote>/HEAD` measured |
| CONF-04 | Separate base and push remotes, both defaulting to `origin` | "Exists" is measured against `git remote add`, not `remote_names()` (Pattern 2, Pitfall 3) |
| CONF-05 | `wsx.sourceBranch`, else the only local `workspace/*` or `personal/*`; zero or several lists them | `prefixed` with a trailing slash plus a one-level filter (Pattern 4, Pitfall 4) |
| CONF-07 | Manifest from `<tip>:.wsx.conf` in the object database; configured means fail closed | Tree-entry read that tells blob from symlink, directory and submodule (Pattern 6). Working-tree edit and clean checkout measured |
</phase_requirements>

## Summary

Every read the phase needs exists in gix 0.90.0 under the current feature set, so **the only new crate is `proptest`, as a dev-dependency**. C-quoting needs nothing new: `gix::quote::ansi_c::{quote, undo}` matched git's own output byte for byte on 270 real file names: every byte value that can appear in a name (1-255 except `/`), plus 16 edge names. A scratch copy of the repo, with a working prototype of the whole phase and 11 tests, passed clippy `-D warnings` and the pinned nextest. Those tests and the rest of the unit suite passed there (131 run). `cargo deny` stayed clean with proptest added and bit on a planted ban.

The gix defaults are the danger. Five natural calls give a wrong or silently weak answer here:
- `string_filter` returns the last value and reads a no-`=` key as absent.
- `Source::Local` includes `include.path` files.
- `remote_names()` counts global sections and keyless sections.
- `prefixed("refs/heads/workspace")` matches `workspace-x`.
- `rev_parse_single("<tip>:.wsx.conf")` and `find_blob` read a symlink as a blob.

Each has a measured replacement below.

The phase also has an integration problem nobody has listed yet. **The harness fixture writes `wsx.baseRef=origin/develop`, and D-08 reads that as a branch literally named `origin/develop`.** The fork fixture also sets no `wsx.baseRemote`. Phase 5's first enabled Rust scenarios would refuse on every fixture unless this is fixed. It is a small change, and it belongs in this phase, which defines the contract.

**Primary recommendation:** Add `src/git/{config,refs,quote}.rs` (reads, gix inside) and `src/model/{mod,manifest}.rs` (policy, no gix). Give `model::resolve` one signature that returns `RepoModel::Configured` or `RepoModel::NotConfigured { inferred }`. Use a separate hand-written `model::Error` with `From<git::Error>`. Add `proptest` with `default-features = false, features = ["std"]`, and `failure_persistence: None` in every property block.

### What this evidence does and does not establish

- **Scope of the measurements:** one host, git 2.56.0, gix 0.90.0. SHA-256 and bare repos were probed for the new reads, and hook-style `GIT_DIR` for config and listing. They were not run through the prototype's tests.
- **The prototype is a feasibility proof.** It has 11 tests, not the phase's test set. It settles that the APIs compile and answer as stated. It does not settle error wording, and it has no global-config test of its own (D-01's control was measured with the probe binary, against `git config`).
- **Not run this session:** `scripts/check.sh all` (the prototype ran clippy and unit tests only, on a `git archive` copy with no git repo), the harness suite against the changed fixture, and the container gate.
- **Proptest strength:** 2048 random cases per property in about 0.2 s. That is a good search for encoding defects and not a proof. The all-bytes differential test is the stronger evidence for quoting.

## Project Constraints (from CLAUDE.md)

- One package, binary crate `wsx`, edition 2024, no library crate. So all tests of this phase are unit tests inside `src/`, not in `tests/`.
- `rust-toolchain.toml` pins rustc; tool versions live only in `scripts/install-tools.sh`.
- `scripts/check.sh all` is the gate for every change. `scripts/check.sh deps` is required when dependencies change (CONTRIBUTING design constraint 5).
- A nextest filter that matches nothing must fail. Gate one test with `--no-tests=fail -E 'test(=module::name)'`, one test per command chained with `&&`. Never `-E 'test(=a) | test(=b)'`.
- `rg -c` prints nothing on zero matches. Check absence with a Rust test or `rg -q`. Prefer a named Rust test to any shell pipeline.
- Run gate commands under bash. Capture the exit code of the command that matters, not the pipeline tail.
- `std::env::set_var`/`remove_var` are banned by `clippy.toml`. Scope environment to the child with `Command::env`.
- Never run `cargo init`/`cargo new` in the repo. New `.gitignore` lines go at the top.
- Commit subjects carry the phase scope, `type(04-NN): ...`, because the PR cut replays only phase-scoped commits.
- No dependency without a justification and a clean `scripts/check.sh deps`.
- The tool never prompts, never resolves a conflict on the user's behalf. No push, PR or repo-settings change from an agent session.
- The container mounts the checkout read-only, so a check that needs to write must use a scratch directory.
- Python 0.1.0 (`bin/wsx`, `tests/sandbox.sh`) is frozen: read for reference only.
- CONTRIBUTING's acceptance bar applies: a test that fails before the change, for the intended reason (not a compile failure); an assertion at a stable boundary; no test that reproduces the production algorithm in its body, compares a call with itself, asserts constants, or greps implementation text.
- Global rules: Rust clippy `-D warnings`, `?` not `.unwrap()` outside tests, `///` on public items, functions around 40 lines, behaviour-led test names with no review narrative, atomic tests (no chained scenarios), no regex-alternation assertions.

## Architectural Responsibility Map

wsx is a CLI, so the tiers are its own layers rather than browser/server.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Local-scope config values, remote-exists check | Git layer (`src/git/config.rs`) | — | gix types never leave `src/git/` |
| Exact ref reads, branch listing, root-tree entry | Git layer (`src/git/refs.rs`) | — | Same boundary; returns `RefTarget`, `RootEntry`, raw bytes |
| C-quote and unquote | Git layer (`src/git/quote.rs`) | Model (`encode_path` adds edge-whitespace quoting) | The gix call stays in `git`; the grammar rule stays in the model |
| Manifest grammar, path matching | Model (`src/model/manifest.rs`) | — | Pure bytes, no repository, so proptest and mutation testing run on it directly |
| Chain, fallback, configured state, built-ins | Model (`src/model/mod.rs`) | Git layer | Policy from D-01 to D-21 lives in one place |
| Exit codes, `--json`, rendering for a command | Phase 5 consumer | Model errors | Deferred to IF-01 |
| Guard's exit 0 when not configured | Phase 6 consumer | `RepoModel::NotConfigured` | Deferred; the model only reports the state |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| gix | `=0.90.0` (already pinned) | Config, refs, trees, blobs | Project stack. Every call below compiled and ran under `features = ["sha1", "sha256", "revision"]`, no new feature [VERIFIED: prototype build, Cargo.toml] |
| gix-quote via `gix::quote` | 0.10.1 (already in the lock) | C-quote and unquote | `pub use gix_quote as quote;` at `gix-0.90.0/src/lib.rs:162`, not feature-gated. Matches git on every byte (measured below) [VERIFIED: gix source and differential run] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| proptest | 1.12.0, dev-dependency | SC 1 requires a proptest; `.planning/PROJECT.md` names it for the manifest and matcher | Round trip and matcher properties [VERIFIED: crates.io API, Context7 `/proptest-rs/proptest`, legitimacy seam OK] |
| tempfile | already present | Scratch repos through `testing::Scratch` | Every repo-backed test |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `gix::quote` | Hand-written unquote | Rejected: seven named escapes plus octal, and gix already matches git on all of them |
| `gix::quote` | A direct `gix-quote` dependency | Rejected: same crate, one more manifest line to keep in step with gix |
| proptest `default-features = false` | proptest defaults | Defaults add `fork`, `timeout` and `bit-set` (rusty-fork, tempfile, bit-vec) that nothing here uses [VERIFIED: crates.io features list] |
| `failure_persistence: None` | proptest's default file persistence | Default writes `proptest-regressions/` beside the source, which fails against the read-only container checkout and adds an ignored path. A printed minimal input becomes a named regression test instead |
| `strings_filter` | `raw_values_filter` | `raw_values_filter` returns `Err` for an absent key, so absence must be told apart from a real error. `strings_filter` gives `None` directly |

**Installation** (in `Cargo.toml` `[dev-dependencies]`, with a justification comment like the existing `tempfile` one):
```toml
proptest = { version = "1.12.0", default-features = false, features = ["std"] }
```

**Version verification:** `cargo search proptest` printed `1.12.0`; the crates.io API reports 1.12.0 published 2026-10-10, `MIT OR Apache-2.0`, `rust_version` 1.88 (the pin is rustc 1.99.1), not yanked.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| proptest | crates.io | 9 yrs (created 2017-06-18); 1.12.0 is 23 days old | 5,277,031/wk (seam) | github.com/proptest-rs/proptest | OK | Approved |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

**Crates proptest adds to `Cargo.lock`** [VERIFIED: lock diff in a scratch copy]: `chacha20` 0.10.2, `num-traits` 0.2.19, `proptest` 1.12.0, `rand` 0.10.3, `rand_core` 0.10.1, `rand_xorshift` 0.5.0, `regex-syntax` 0.8.11, `unarray` 0.1.4. Dev-only, so nothing reaches the shipped binary.

**cargo-deny with these added** (0.20.2, the pin, `cargo deny --locked check`): `advisories ok, bans ok, licenses ok, sources ok`, exit 0, with no `deny.toml` change. Duplicate-crate warnings stayed at one (`hashbrown`) before and after. **Control:** banning `unarray`, a crate only proptest brings, made it exit 2 with `crate 'unarray = 0.1.4' is explicitly banned`, so the run did see the new crates. Not checked: that each crate's licence is correctly declared upstream; cargo-deny trusts declarations.

**Build scripts among the new crates:** only `num-traits` has a `build.rs`, a compile-time `autocfg` probe with no network or filesystem use outside the build directory (read this session). crates.io has no postinstall.

## Architecture Patterns

### System Architecture Diagram

```text
Phase 5+ command or Phase 6 guard
        |
        v
Repo::discover (Phase 3: trust, reftable, env refusals)
        |
        v
model::resolve(&repo, &recorded)
        |
        +--> local wsx.sourceBranch? -- repeated/empty/not a branch name --> Err (configured, fails closed)
        |         |
        |    set  |  unset
        |         |      \
        |         |       v
        |         |     candidates: refs/heads/{workspace,personal}/<one level> minus recorded
        |         |       |  exactly one                     | zero or several
        |         |       v                                  v
        |         |     build(inferred)                 inferred = Err(lists them)
        |         v       |                                  |
        |     build(branch)                                  |
        |         |       |                                  |
        v         v       v                                  v
      build: base and push remote (config key or `origin`, must be defined in this repo)
          --> integration: wsx.baseRef exact, else develop, next, <remote>/HEAD target
              (tracking refs only; local refs only when there is no base remote)
          --> worktree dir: wsx.worktreeDir or .worktrees/, D-14 rules
          --> tip of refs/heads/<branch> --> root tree entry `.wsx.conf`
                  absent / not a blob / unreadable / bad grammar --> Err
                  blob --> manifest::parse --> Manifest
        |
        v
RepoModel::Configured(model)  or  RepoModel::NotConfigured { inferred: Ok(model) | Err(why) }
```

### Recommended Project Structure

```text
src/
├── main.rs               # add `mod model;`
├── git/
│   ├── mod.rs            # add the mods and re-exports; make `testing` pub(crate)
│   ├── config.rs         # NEW: local_config_values, remote_configured
│   ├── refs.rs           # NEW: find_ref, ref_commit, refs_directly_under, root_entry, is_branch_name
│   └── quote.rs          # NEW: c_quote, c_unquote
└── model/
    ├── mod.rs            # NEW: RepoModel, Model, Error, resolve
    └── manifest.rs       # NEW: parse, covers, normalize_path, encode_path
tests/harness/fixture.rs  # wsx.baseRef and fork-remote values (Pitfall 14)
Cargo.toml, Cargo.lock    # proptest
```

`Repo` must also be re-exported from `git` (`pub use repo::Repo;`) for the model to name it. Existing `pub use` is absent: `src/git/mod.rs:25-29` declares `mod reads; mod repo; mod spawn; #[cfg(test)] mod testing;` [VERIFIED: src/git/mod.rs:25-29]. `ObjectId` and `Error` are already public there.

### Pattern 1: The local-scope, multi-value config read (D-01, D-18)

**What:** `strings_filter` with `source == Local && level == 0`. Length 0 is absent, 2+ is repeated, and a single empty value is empty.

```rust
// Compiled and run in the prototype.
pub fn local_config_values(&self, key: &str) -> Vec<Vec<u8>> {
    self.repo
        .config_snapshot()
        .plumbing()
        .strings_filter(key, |meta| {
            meta.source == gix::config::Source::Local && meta.level == 0
        })
        .unwrap_or_default()
        .into_iter()
        .map(Vec::from)
        .collect()
}
```

**Measured against `git config --local --get-all`** (git 2.56.0, gix-config 0.62.2) [VERIFIED: probe run]:

| Setup | `strings_filter` (Local, level 0) | `string_filter` (`ref_storage`'s call) | git `--local` |
|---|---|---|---|
| Key only in global | `None` | `None` | exit 1 (the D-01 control; plain `--get` returns it) |
| `baseRef=a`, then `baseRef=b` | `["a","b"]` | `"b"` (last wins) | `a` `b` |
| Repeated across two `[wsx]` sections | `["1","2"]` | not measured | `1` `2` |
| `key =` | `[""]` | `""` | empty, exit 0 |
| `key` with no `=` | `[""]` | **`None`** | empty, exit 0 |
| `"a b # c"`, `a\tb`, `v # comment`, `   v   ` | normalised: `a b # c`, tab, `v`, `v` | same | same |
| `[wsx "sub"]` | `None` | `None` | empty (`wsx.sub.key` finds it) |
| Non-UTF-8 value | raw `caf\xe9` | not measured | raw `caf\351` |
| Key case `WSX.QUOTED` | matches | not measured | matches |
| `config.worktree` | `None` (`Source::Worktree`) | `None` | empty (plain `--get` returns it) |
| `GIT_CONFIG_COUNT` | `None` (filtered) | `None` | not applicable |
| Linked worktree, or hook-style `GIT_DIR`+`GIT_INDEX_FILE` | finds the common dir's value | not measured | not compared |
| SHA-256 and bare repos | finds the value | not measured | not compared |

**Included files need the `level` filter:** a value from `include.path` reads as `source=Local level=1`. `git config --local --get` does not see it (exit 1); `--local --includes` and plain `--get` do. So `level == 0` matches the control CONTEXT names, and keeps a tracked include (the trap in CLAUDE.md) from making a repo "configured". [ASSUMED: this reading of D-01, see A1 and Open Question 1]

### Pattern 2: A remote "exists" when this repo defines it (D-03, D-04)

**What:** Any key in a `remote.<name>` section from `Source::Local` or `Source::Worktree`. This is git's own in-repo test.

```rust
pub fn remote_configured(&self, name: &[u8]) -> bool {
    let snapshot = self.repo.config_snapshot();
    let Some(sections) = snapshot.plumbing().sections_by_name("remote") else {
        return false;
    };
    sections
        .filter(|section| matches!(
            section.meta().source,
            gix::config::Source::Local | gix::config::Source::Worktree
        ))
        .filter(|section| section.value_names().next().is_some())
        .any(|section| section.header().subsection_name() == Some(name.as_bstr()))
}
```

**Measured** [VERIFIED: probe run]. `git remote add` is the differential: it refuses with exit 3 only when the name is already defined in the repo.

| Section | `gix remote_names()` | `git remote` | `git remote add` |
|---|---|---|---|
| Local `url`+`fetch` | listed | listed | refuses, exit 3 |
| **Global only** (`[remote "x"] prune = true`) | listed | listed | **succeeds** |
| **Local, no keys** (`[remote "empty"]`) | **listed** | not listed | **succeeds** |
| Local, keys but no url (`prune = true`) | listed | listed | not measured |
| Name with a slash (`a/b`) | listed | listed | not measured |
| Via `include.path` (`Source::Local`, level 1) | listed | not compared | refuses, exit 3 |
| `config.worktree` (`Source::Worktree`) | listed | not compared | refuses, exit 3 |
| Legacy `.git/remotes/<name>` file | not listed | not listed | not measured |

So `remote_names()` is wrong twice for D-04: a global `[remote "origin"]` would make a repo with no origin look like it has one, and then neither the chain nor `wsx.baseRef` could ever resolve there. [ASSUMED: this reading of D-03 and D-04, see A2 and Open Question 2]

### Pattern 3: Exact ref reads and the chain (D-06 to D-10)

**What:** Build the full name, `FullName::try_from`, then `try_find_reference(name.as_ref())`, reading `target()` without following it. Report a found ref's commit through the existing `resolve_commit`.

**Measured** [VERIFIED: probe run]:
- **A full name is exact.** With `refs/heads/refs/remotes/origin/ghost` present and `refs/remotes/origin/ghost` absent, `try_find_reference("refs/remotes/origin/ghost")` is `None`. Control: the longer name is found. A `refs/tags/refs/heads/ghost2` tag does not satisfy `refs/heads/ghost2` either.
- **A short name is not.** `try_find_reference("develop")` found `refs/tags/develop`, and `origin/develop` found the remote branch. Never pass a short name.
- `rev_parse_single(full)` on names shaped like describe output or holding `@` (`workspace/x-1-gabc1234`, `workspace/a@b`, `workspace/@`, non-UTF-8) gave git's answer on all six shapes tried, so `resolve_commit` is safe for a full name.

**`<remote>/HEAD` shapes:**

| Shape | `target()` | Treatment |
|---|---|---|
| `git clone`, or `git remote set-head origin --auto` | `Symbolic(refs/remotes/origin/main)` | Strip `refs/remotes/<base>/`, require the target to exist, branch `main` |
| Target ref deleted (dangling) | `Symbolic(...)` still returned; `git rev-parse --verify` fails | Exists check fails, so D-09's error lists it |
| Packed | Same as loose | Same |
| `remote.origin.followRemoteHEAD=never`, after deletion, then a fetch | `None` | D-09's error |
| Object id, no symref (hand-made) | `Object(id)` | No branch name, so D-09's error |
| Symref outside the remote (`refs/heads/main`) | `Symbolic(refs/heads/main)` | Prefix strip fails, so D-09's error |
| Symref to another symref under the remote | `Symbolic(refs/remotes/origin/alias)` | Accepted as branch `alias`; hand-made, harmless (D-10) |

**Branch-name validity for `wsx.baseRef` and `wsx.sourceBranch`:** `FullName::try_from("refs/heads/<name>")` agreed with `git check-ref-format --branch` on 25 names except `HEAD` and `-x`, which gix accepts and git refuses. So `is_branch_name` adds those two checks. `refs/heads/x` as a value is accepted by both and then simply is not found, which D-21 already covers.

### Pattern 4: Workspace candidates (D-17, D-21)

```rust
pub fn refs_directly_under(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
    // ... `failed` as elsewhere in this module ...
    let platform = self.repo.references().map_err(|err| failed(err.to_string()))?;
    let refs = platform.prefixed(prefix.as_bstr()).map_err(|err| failed(err.to_string()))?;
    let mut names = Vec::new();
    for reference in refs {
        let reference = reference.map_err(|err| failed(err.to_string()))?;
        let full = reference.name().as_bstr().to_vec();
        if !full[prefix.len()..].contains(&b'/') {
            names.push(full);
        }
    }
    names.sort();
    Ok(names)
}
```

**Measured** [VERIFIED: probe run, with branches `workspace/{a,café,nested/deep,raw\xe9,sym}`, `workspace-x`, `workspaceY`, `personal/workspace`, `personal/workspace-backup`, `personal-z`, `notworkspace/q`]:

| Query | Result |
|---|---|
| `prefixed("refs/heads/workspace/")` | The five under `workspace/`, including `nested/deep` |
| `prefixed("refs/heads/workspace")` no slash | **Also `workspace-x` and `workspaceY`** |
| `git for-each-ref 'refs/heads/workspace/*'` (the oracle's call, `scripts/workspace/cut-pr-branch.sh:140`) | The four at one level: **not** `nested/deep` |
| After `pack-refs --all` plus one new loose ref | gix 6, git 6 |
| From a linked worktree | Same list |
| Non-UTF-8 name | Listed and readable by exact name |
| Corrupt loose ref under the prefix | gix yields `Err` naming it; git warns and skips |
| Dangling symref under the prefix | gix lists it; `git for-each-ref` omits it |

The prototype applies one level, as D-21's glob reads in git. [ASSUMED: this reading of D-21, see A3 and Open Question 3] A corrupt ref becomes an error, which fails closed in the only case it can matter. A dangling symref counts as a candidate, so it produces the "several candidates" error listing it, which is safe and names the fix.

### Pattern 5: C-quoted paths (D-14, D-19)

```rust
// src/git/quote.rs: all the gix there is.
pub fn c_quote(raw: &[u8]) -> Vec<u8> {
    gix::quote::ansi_c::quote(raw.as_bstr()).to_vec()
}

pub fn c_unquote(token: &[u8]) -> Result<Vec<u8>, String> {
    let (raw, consumed) =
        gix::quote::ansi_c::undo(token.as_bstr()).map_err(|err| err.to_string())?;
    if consumed != token.len() {
        return Err("text after the closing quote".to_owned());
    }
    Ok(raw.to_vec())
}
```

**Measured differential** [VERIFIED: probe run, 270 real files: `p<byte>q` for every byte 1-255 except `/`, plus ` lead`, `trail `, `\tleadtab`, `trailtab\t`, `my dir`, `"q"`, `back\slash`, `caf\xc3\xa9`, `raw\xe9`, `#hash`, `*star`, `nl\nnl`, `cr\r`, `\x7f`, `-dash`, `a  b`]:
- `quote(raw)` equals `git ls-files` output, byte for byte across the whole list. Control: one changed byte is detected.
- `undo(git ls-files output)` equals `git ls-files -z`. Same for `-c core.quotePath=false` output.

**Malformed forms**, against `git check-ignore --stdin`, which calls git's own unquote:

| Token | git | gix `undo` | Required outcome |
|---|---|---|---|
| `"abc`, `"`, `"a\`, `"\3` | badly quoted | `Err` | Error naming the line |
| `"a\q"`, `"\e"`, `"\'"`, `"\?"` | badly quoted | `Err` | Error |
| `"\400"`, `"\38"`, `"\37"`, `"\1a1"` | badly quoted | `Err` | Error |
| `"a"b` | **accepted as `a`** | **`Ok`, consumed 3 of 4** | **Must error: check `consumed`** |
| `"\000"` | truncated to empty | **`Ok("\0")`** | **Must error: a git path has no NUL** |
| `""` | empty refused | `Ok("")`, consumed 2 | Error, empty path |
| `"\101"`, `" lead"`, `"a b"` | accepted | `Ok` | Parses |
| `a"b`, `plain` | literal | Unchanged | Parses (quoted only when it starts with `"`) |

**`gix quote` leaves leading and trailing spaces bare, exactly as git does** (` lead` and `trail ` printed unquoted; tab-edged names are quoted). D-19 refuses those bare, so `encode_path` must add the quotes itself: that is the function `wsx init` and `wsx migrate` will use in Phase 12, and the one the proptest round-trips.

### Pattern 6: The manifest blob (CONF-07, D-19)

```rust
// src/git/refs.rs. `RootEntry` is { Absent, Blob(Vec<u8>), NotABlob { kind } }.
let commit = self.resolve_commit(full)?;
let tree = self.repo.find_commit(commit).map_err(|err| failed(err.to_string()))?
    .tree().map_err(|err| failed(err.to_string()))?;
let Some(entry) = tree.find_entry(name) else {
    return Ok(RootEntry::Absent);
};
let mode = entry.mode();
if !mode.is_blob() {
    let kind = if mode.is_link() { "symlink" }
        else if mode.is_tree() { "directory" }
        else { "submodule" };
    return Ok(RootEntry::NotABlob { kind });
}
let blob = self.repo.find_blob(entry.oid().to_owned()).map_err(|err| failed(err.to_string()))?;
Ok(RootEntry::Blob(blob.data.clone()))
```

**Measured against `git ls-tree` and `git cat-file -t`** [VERIFIED: probe run]:

| Entry at `.wsx.conf` | `mode()` | `is_blob()` | `find_blob` | `rev_parse_single("<tip>:.wsx.conf")` | `git ls-tree` |
|---|---|---|---|---|---|
| Regular file | `100644` | true | the bytes | Blob | `100644 blob` |
| Executable | `100755` | true | the bytes | Blob | `100755 blob` |
| **Symlink** | `120000` | **false** | **Ok: the link target's text** | **Blob** | `120000 blob` |
| Directory | `40000` | false | Err: kind tree | Tree | `040000 tree` |
| Gitlink | `160000` | false | Err: kind commit | Commit | `160000 commit` |
| Absent | `find_entry` is `None` | — | — | Err | empty |
| Empty file | `100644` | true | 0 bytes | Blob | `100644 blob` |
| Only `sub/.wsx.conf` | `None` | — | — | Err | empty |

**So only the tree-entry mode can tell a symlink apart.** A symlink's blob holds its target's text, so a read that skips the mode check succeeds, and that text is then parsed as a manifest.

**Other measured cases:**
- Uncommitted edit to the working file: unchanged read. Clean checkout without the file: same read. Both also pass as prototype tests.
- Missing blob object (loose file deleted): `find_blob` `Err`, "could not be found"; `git cat-file` also fails. Missing tree: `commit.tree()` `Err`. Missing commit: the ref still exists, peel `Err`. Each is a fail-closed reason, not a "missing manifest".
- Ref holding a blob id: peel `Err`.
- `refs/replace` on the blob: gix and git both read the replacement. Same answer, nothing to handle.
- SHA-256 and bare repos: same read.

`find_entry` drops malformed tree entries by `filter_map(Result::ok)` (`gix-0.90.0/src/object/tree/mod.rs:57-58`), so a corrupt tree could read as `Absent`. Not measured. It still fails closed when configured; only the reason would be imprecise.

### Pattern 7: The manifest grammar, and what "personal" means

```rust
pub fn covers(entry: &[u8], path: &[u8]) -> bool {
    path.strip_prefix(entry)
        .is_some_and(|rest| rest.is_empty() || rest[0] == b'/')
}

pub fn normalize_path(path: &[u8]) -> Result<Vec<u8>, Problem> {
    if path.contains(&0) { return Err(Problem::Nul); }
    let stripped = path.strip_suffix(b"/").unwrap_or(path);
    if stripped.is_empty() { return Err(Problem::EmptyPath); }
    if stripped[0] == b'/' { return Err(Problem::AbsolutePath); }
    for component in stripped.split(|byte| *byte == b'/') {
        match component {
            b"" => return Err(Problem::EmptyComponent),
            b"." | b".." => return Err(Problem::DotComponent),
            _ => {}
        }
    }
    Ok(stripped.to_vec())
}
```

**Line rules the prototype's table test passed** (`grammar_table`, `overlap_table`) [VERIFIED: prototype tests]:

| Line | Result |
|---|---|
| `sync a`, ` only a`, `only\ta`, whitespace only | Unknown kind |
| `only a\r`, `# c\r` | Line ends in CR. A CRLF file fails on line 1 with the reason |
| `only`, `only `, `only /` | Empty path |
| `only  a`, `only a `, `only a /` | Edge whitespace, must be quoted |
| `only /a` | Leading `/` |
| `only a//b`, `only a//` | Empty component |
| `only .`, `only a/../b` | Dot component |
| `only "\000"` | NUL |
| `only my dir` | `my dir` |
| `only a/` | `a` |
| `only a*b` | `a*b`, literal |
| `only #x` | `#x`: a comment only when `#` starts the line |
| `only a"b` | `a"b` |
| `only " a "` | ` a ` |
| `only café` (raw UTF-8), `only raw\xe9`, `only a\x01b` | Raw bytes pass unquoted |
| `only "caf\303\251"` | The UTF-8 bytes |
| No trailing newline; empty manifest; comments only | Valid |
| `only a` with `modified a`, `only a` with `modified a/b`, `modified a/b` then `only a`, `only a/` with `modified a` | Overlap, naming both lines |
| `only a`, `only a`, `only a/b` with `modified b` | Parses: same-kind duplicates and nesting are allowed |
| `only a` with `modified ab` | Parses: `ab` is not under `a` |

**`modified` is not personal.** PRCUT-02 replays `modified` changes "like any other change", DIV-01 requires an added path to be under "a built-in or `only` entry", and the oracle collects only `only` lines into `FORBIDDEN_PATHS` (`scripts/workspace/cut-pr-branch.sh:163-165`: `if [ "$KIND" = "only" ] && [ -n "$ENTRY" ]; then`). So `is_personal` covers `.wsx.conf`, the worktree dir and `only` entries, and an entry's kind stays available for Phase 5's `modified` checks.

**Keep every entry, with its line number.** D-13 needs `only .wsx.conf` to parse, and Phase 5 reports it, so the parser must not filter built-ins or dedupe.

**A deliberate difference from the oracle:** `read -r KIND ENTRY _` splits on any whitespace and drops a third word, so the oracle reads `only a b` as `a` and `only\ta` as `a`. D-19 reads the first as `a b` and refuses the second. `.workspace-divergence` in this repo has neither shape.

### Pattern 8: The model's shape and errors

```rust
pub enum RepoModel {
    Configured(Box<Model>),
    NotConfigured { inferred: Result<Box<Model>, Error> },
}

pub fn resolve(repo: &Repo, recorded: &BTreeSet<Vec<u8>>) -> Result<RepoModel, Error> {
    match single(repo, SOURCE_BRANCH)? {
        Some(value) => {
            let branch = branch_name(SOURCE_BRANCH, value)?;
            Ok(RepoModel::Configured(Box::new(build(repo, branch)?)))
        }
        None => {
            let inferred = candidates(repo, recorded).and_then(|mut names| {
                if names.len() != 1 {
                    return Err(Error::WorkspaceCandidates { candidates: names });
                }
                build(repo, names.remove(0))
            });
            Ok(RepoModel::NotConfigured { inferred: inferred.map(Box::new) })
        }
    }
}
```

**Why this shape:**
- `Err` from `resolve` means exactly "configured and failing closed". A repeated or empty `wsx.sourceBranch` lands there, because the key is present (D-15).
- `NotConfigured` never errors, however broken the rest is, so the Phase 6 guard reads it and exits 0 (D-16, D-21).
- Commands needing a workspace branch take `inferred` and report its `Err`, wrapped as "not configured, run `wsx init`" for a manifest failure (D-16) or as the candidate list (D-21). Add one `require()` method for that.
- `recorded` is the D-17 input. Production passes an empty set until Phase 8.

**Model fields:** `workspace_branch`, `workspace_tip: ObjectId`, `integration { branch, full_ref, tip }`, `base_remote: Option<Vec<u8>>`, `push_remote: Option<Vec<u8>>`, `worktree_dir`, `manifest`. Recording the tips means every later read in one command uses the same commits the manifest came from.

**Errors: a separate `model::Error`, not new `git::Error` variants.** `git::Error` documents itself as "Why the git layer could not answer" (`src/git/mod.rs:52`). These are policy refusals, and one variant per refusal follows Phase 3's pattern and gives Phase 5 one exit code per refusal. Hand-written `Display`, `From<git::Error>` (variant `Git`), no error crate. Variants used by the prototype: `Git`, `KeyRepeated`, `KeyEmpty`, `NotABranchName`, `NoSuchRemote`, `BaseRefMissing`, `NoIntegrationBranch`, `WorkspaceBranchMissing`, `WorkspaceCandidates`, `ManifestMissing`, `ManifestNotABlob`, `ManifestInvalid`, `WorktreeDirInvalid`.

**Messages must name the fix** (CONTEXT, CONTRIBUTING style). `BaseRefMissing` should say the value is a bare branch name, since `origin/develop` was the old form (Pitfall 14). `NoIntegrationBranch` lists the refs tried and both D-09 fixes.

### Anti-Patterns to Avoid

- **Letting gix leak into `src/model/`.** The first prototype draft called `FullName::try_from` from the model; it moved to `git::is_branch_name`.
- **`Err` for "not configured".** The guard would have to tell apart "not configured" from a real failure by variant. Keep it a value.
- **Filtering built-ins out of the parsed manifest.** Phase 5's stale-declaration check needs them (D-13).
- **Reusing `ref_storage`'s `string_filter`** for D-18. It hides repeats and no-`=` keys (Pattern 1).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| C-quote and unquote | A byte table | `gix::quote::ansi_c` | Already equal to git on every byte value |
| Config parsing and value normalisation | Reading `.git/config` | `strings_filter` | Quotes, escapes, comments, case and trailing space all matched git |
| Ref name validity | A character list | `gix::refs::FullName::try_from`, plus the two extra checks | Agreed with `git check-ref-format --branch` on 23 of 25 names; the other two are in `is_branch_name` |
| Packed and loose ref merge | Reading `packed-refs` | `references().prefixed()` | Measured equal after `pack-refs --all` |

**Key insight:** Don't hand-roll the reads, but do not trust gix's convenience defaults either. Each read in this phase needs an explicit filter or check, and each has a named differential test against git.

## Runtime State Inventory

Not a rename or migration phase. One related item: `tests/harness/fixture.rs` writes `wsx.baseRef` in a form D-08 changes (Pitfall 14). No real repo on this machine has a `wsx.*` key today (`git config --local --get-regexp '^wsx\.'` printed nothing for wsx, devflow, gsd-core and gsd-pi), so no stored config needs migrating. The 0.1.0 projects on the owner's work PC could hold `wsx.baseRef=origin/...`: `bin/wsx:845` reads it as a revision. That is Phase 12's concern (MIG-01 writes `wsx.baseRef`), and the error message covers an unmigrated value.

## Common Pitfalls

### Pitfall 1: `string_filter` hides both D-18 cases
**What goes wrong:** Copying `ref_storage`'s call gives the last value of a repeated key, and `None` for a key with no `=`.
**Why it happens:** `string_filter` is last-one-wins; `string_implicit_filter` is its implicit-aware twin, returning `Some(None)`.
**How to avoid:** `strings_filter` (Pattern 1).
**Warning signs:** A test for a repeated key that resolves instead of erroring.

### Pitfall 2: `Source::Local` includes `include.path` files
**What goes wrong:** A tracked include sets `wsx.sourceBranch`, read live from whatever branch is checked out, and the repo reads as configured.
**How to avoid:** Add `meta.level == 0`. Negative control: the include file's value must be invisible while `git config --local --includes --get` sees it.

### Pitfall 3: `remote_names()` over-counts
**What goes wrong:** A global `[remote "origin"] prune = true` makes a repo without an origin look like it has one. Local refs are then never tried (D-07), and `wsx.baseRef` cannot rescue it, because it resolves against the phantom remote. An empty `[remote "x"]` also counts, though git disagrees.
**How to avoid:** `remote_configured` (Pattern 2). Run the D-04 test under a global `remote.origin` section through `testing::reexec`.

### Pitfall 4: Prefix without a slash, and nesting
**What goes wrong:** `prefixed("refs/heads/workspace")` matches `workspace-x`, so an unrelated branch makes the fallback fail. Without a depth filter, `workspace/a/b` counts though git's glob excludes it.
**How to avoid:** A trailing `/` and the one-level filter. Differential test against `git for-each-ref 'refs/heads/workspace/*' 'refs/heads/personal/*'`.

### Pitfall 5: Short names, and `git rev-parse` in differential tests
**What goes wrong:** `try_find_reference("develop")` found a tag. Separately, with `refs/heads/refs/heads/ghost` present and `refs/heads/ghost` absent, **`git rev-parse --verify refs/heads/ghost` succeeds**, by git's DWIM rules, while gix's exact read and its revspec both say missing.
**How to avoid:** Always full names in gix. In a differential test, ask git with `git show-ref --verify <full>`, which refused such a name, not `rev-parse`.

### Pitfall 6: `undo` stops at the closing quote and passes NUL
**What goes wrong:** `only "a"b` parses as `a`, and `only "\000"` gives a path holding NUL, which could never match.
**How to avoid:** `c_unquote` checks `consumed`; `normalize_path` refuses NUL. NUL is not in D-14's list, but D-14's reason ("could never match a git tree path") covers it, so this is applied without a new decision.

### Pitfall 7: Git's output is not valid under D-19 for edge spaces
**What goes wrong:** Feeding `git ls-files` output directly into a manifest fails for ` lead` and `trail `, because git prints them bare. This is by design (CONTEXT Specific Ideas).
**How to avoid:** Write through `encode_path`. The all-bytes differential test asserts exactly two such refusals, which pins this.

### Pitfall 8: A symlink reads as a valid manifest
**What goes wrong:** `rev_parse_single("<tip>:.wsx.conf")` and `find_blob` both succeed on a symlink.
**How to avoid:** Check `entry.mode().is_blob()` first (Pattern 6). Named tests for symlink, directory and submodule, each against `git ls-tree`.

### Pitfall 9: `Repo`'s config is frozen at open; refs are not
**What goes wrong:** A test opens `Repo`, runs `git config`, reads through the same `Repo`, and sees the old value. Measured: config stale (empty) until reopened, while a new branch and a pack-then-new-branch were both seen immediately.
**How to avoid:** Set config, then discover. Production is unaffected unless a future command writes config and re-reads through one handle.

### Pitfall 10: On reftable, these reads say "missing", not "unsupported"
**What goes wrong:** On a reftable repo opened without Phase 3's refusal, the exact read returned `None` and the listing was empty. So the `is_unsupported()` backstop in `Repo::resolve` does not fire for them: a reftable repo would read as "no develop, no candidates".
**How to avoid:** Nothing new: `refuse_ref_storage` runs first in `Repo::discover` (`src/git/repo.rs:37`), and it is tested. Do not add an open path that skips it.

### Pitfall 11: `dead_code` fires on the test target for unread error fields
**What goes wrong:** `src/git/mod.rs:23` is `#![cfg_attr(not(test), expect(dead_code, reason = "no caller until phase 5"))]`, off on the test target, so every item must be used by a test. Measured: `#[derive(Debug)]` does not count, and error variant fields read nowhere failed clippy with `fields ... are never read`.
**How to avoid:** The new `src/model/mod.rs` carries the same attribute (measured: non-test build clean), and `Display` reads the fields. Implement `Display` in the same plan as the variants. Phase 5 removes both attributes.

### Pitfall 12: `testing` is private to `git`
**What goes wrong:** `src/git/mod.rs:28-29` has `#[cfg(test)] mod testing;`, so model tests can't reach `Scratch`.
**How to avoid:** `#[cfg(test)] pub(crate) mod testing;` (measured: compiles and runs).

### Pitfall 13: In-process `Repo::discover` reads the runner's environment
**What goes wrong:** A test that discovers in-process sees the runner's `GIT_DIR` when the suite runs from a hook, and always sees the runner's real global config.
**How to avoid:** Call `if testing::rerun_scrubbed() { return; }` first (the Phase 3 convention). Any test that plants global config must run through `testing::reexec` with the scratch env, as `ref_storage_in_global_config_does_not_refuse_a_files_repo` does (`src/git/repo.rs:2221`). Assert a `WSXCHILD` marker, never the exit status alone. `local_config_values` and `remote_configured` ignore host global config by design, so other tests stay hermetic.

### Pitfall 14: The harness fixture contradicts D-08, and has no `wsx.baseRemote`
**What goes wrong:** `tests/harness/fixture.rs:263-266` reads:
```rust
let base_ref = format!("{}/{}", config.base_remote, config.integration);
let mut local = vec![
    ("wsx.sourceBranch".to_owned(), config.workspace.clone()),
    ("wsx.baseRef".to_owned(), base_ref),
```
and `:670` asserts `("wsx.baseRef", "origin/develop\n")`, `:828` asserts `("wsx.baseRef", "upstream/develop\n")`. Under D-08 those resolve to `refs/remotes/origin/origin/develop`, which is missing, so it is an error with no fallback. `tests/harness/` also never sets `wsx.baseRemote` or `wsx.pushRemote`, so the fork fixture would resolve its base to `origin`, the wrong remote.
**Why it happens:** Phase 2 predates D-08; 0.1.0 treated `wsx.baseRef` as a revision (`bin/wsx:845`).
**How to avoid:** In this phase, set `wsx.baseRef` to `config.integration`, add `wsx.baseRemote` and `wsx.pushRemote` from `config.base_remote` and `config.push_remote`, and update the two assertions. The oracle reads no `wsx.*` key (`rg` over `tests/oracle` and `tests/harness/oracle.rs` found none), and `tests/harness/oracle.rs:242` builds its own `base_ref`, so the oracle-side records should not change. **Unverified:** the harness suite has not been run with this change; the plan's verification must do so.
**Warning signs:** Phase 5's first scenario refusing with `BaseRefMissing`.

### Pitfall 15: D-18 as written conflicts with GUARD-05
**What goes wrong:** D-18 says "An empty or repeated `wsx.*` key" is an error, while GUARD-05 says of `wsx.protectedBranches`, "an empty value turns it off". A blanket `wsx.*` scan would break that.
**How to avoid:** Apply D-18 per key, to the keys this phase reads: `wsx.sourceBranch`, `wsx.baseRef`, `wsx.baseRemote`, `wsx.pushRemote`, `wsx.worktreeDir`. Phase 11 handles its own. Do not scan the section.

### Pitfall 16: Manifest bytes reach the terminal through error messages
**What goes wrong:** An error naming a line prints raw bytes, including ESC sequences, from a blob that may have arrived through a merge. `String::from_utf8_lossy` (the current message pattern) passes those through.
**How to avoid:** Render manifest lines, paths and config values through `c_quote` in `Display`, as git does for paths. Test: a line with `\x1b` renders `\033`.

### Pitfall 17: Proptest in the repo's environment
**What goes wrong:** Default persistence writes `proptest-regressions/main.txt` on failure (measured), which would hit the container's read-only mount. Also, `PROPTEST_CASES=50` overrode an explicit `cases: 3` (measured), so the host environment can change the count.
**How to avoid:** `failure_persistence: None` (measured: writes nothing, still prints the minimal input). Turn any failure into a named regression test. Do not assert a case count.

### Pitfall 18: RED evidence for a new module
**What goes wrong:** With `tdd_mode: true`, and CONTRIBUTING rejecting a compile failure as evidence, a test against a function that does not exist yet proves nothing.
**How to avoid:** Commit the signature with a wrong-answer stub, so the test compiles and fails on its assertion. Phase 3 opened the same way (`test(03-01): add failing merge-base parity test`). The JUnit report at `target/nextest/default/junit.xml` is what the TDD gate reads (`.config/nextest.toml`).

## Code Examples

### Local config with D-18

```rust
fn single(repo: &Repo, key: &'static str) -> Result<Option<Vec<u8>>, Error> {
    let mut values = repo.local_config_values(key);
    match values.len() {
        0 => Ok(None),
        1 => {
            let value = values.remove(0);
            if value.is_empty() {
                return Err(Error::KeyEmpty { key });
            }
            Ok(Some(value))
        }
        _ => Err(Error::KeyRepeated { key, values }),
    }
}
```

### Remotes and the chain

```rust
fn remote(repo: &Repo, key: &'static str) -> Result<Option<Vec<u8>>, Error> {
    match single(repo, key)? {
        Some(name) if repo.remote_configured(&name) => Ok(Some(name)),
        Some(remote) => Err(Error::NoSuchRemote { key, remote }),
        None => Ok(repo.remote_configured(DEFAULT_REMOTE).then(|| DEFAULT_REMOTE.to_vec())),
    }
}

fn integration(repo: &Repo, base: Option<&[u8]>) -> Result<Integration, Error> {
    let prefix = match base {
        Some(remote) => [b"refs/remotes/", remote, b"/"].concat(),
        None => b"refs/heads/".to_vec(),
    };
    let found = |branch: &[u8]| -> Result<Integration, Error> {
        let full_ref = join(&prefix, branch);
        Ok(Integration {
            branch: branch.to_vec(),
            tip: repo.ref_commit(&full_ref)?,
            full_ref,
        })
    };
    if let Some(value) = single(repo, BASE_REF)? {
        let value = branch_name(BASE_REF, value)?;
        let tried = join(&prefix, &value);
        return match repo.find_ref(&tried)? {
            Some(_) => found(&value),
            None => Err(Error::BaseRefMissing { value, tried }),
        };
    }
    let mut tried = Vec::new();
    for branch in CHAIN { // [b"develop", b"next"]
        let full = join(&prefix, branch);
        if repo.find_ref(&full)?.is_some() {
            return found(branch);
        }
        tried.push(full);
    }
    if base.is_some() {
        let head = join(&prefix, b"HEAD");
        if let Some(RefTarget::Symbolic(target)) = repo.find_ref(&head)?
            && let Some(branch) = target.strip_prefix(prefix.as_slice())
            && repo.find_ref(&target)?.is_some()
        {
            return found(branch);
        }
        tried.push(head);
    }
    Err(Error::NoIntegrationBranch { remote: base.map(<[u8]>::to_vec), tried })
}
```

A ref that exists but does not peel to a commit gives `Error::Git` from `ref_commit`, with no fall-through. That matches "with no fallback" in D-08, and fails closed.

### Encoding a path for the manifest

```rust
pub fn encode_path(path: &[u8]) -> Vec<u8> {
    let quoted = c_quote(path);
    let bare = quoted.as_slice() == path;
    if bare
        && (path.first().copied().is_some_and(is_edge_whitespace)
            || path.last().copied().is_some_and(is_edge_whitespace))
    {
        let mut wrapped = vec![b'"'];
        wrapped.extend_from_slice(path);
        wrapped.push(b'"');
        return wrapped;
    }
    quoted
}

fn is_edge_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | 0x0b | 0x0c)
}
```

### The proptest (SC 1)

```rust
use proptest::prelude::*;
use proptest::test_runner::Config;

fn component() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(1u8..=255u8, 1..12).prop_filter("a git tree name", |name| {
        !name.contains(&b'/') && name != b"." && name != b".."
    })
}

fn git_path() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(component(), 1..4).prop_map(|parts| parts.join(&b'/'))
}

proptest! {
    #![proptest_config(Config { cases: 2048, failure_persistence: None, ..Config::default() })]

    #[test]
    fn any_git_path_round_trips(path in git_path()) {
        let mut text = b"only ".to_vec();
        text.extend_from_slice(&encode_path(&path));
        text.push(b'\n');
        let manifest = parse(&text).expect("an encoded path parses");
        prop_assert_eq!(&manifest.entries[0].path, &path);
    }
}
```

**Controls measured on the prototype:** making `covers` ignore the path boundary failed `name_extension_is_not_covered` and `overlap_table`. Making `encode_path` leave edge whitespace bare failed `any_git_path_round_trips`. Both restored after. The generator draws from all of 1-255 except `/`, so newline, quote, backslash, space, non-ASCII and non-UTF-8 bytes all appear, but it does not guarantee each in every run. The all-bytes differential test does.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `.workspace-divergence` read with `read -r KIND ENTRY _` | C-quoted raw-byte grammar, unknown lines are errors | This phase | The oracle silently truncates `only a b`; wsx does not |
| `origin/develop` style `wsx.baseRef` (0.1.0, harness fixture) | Bare branch name plus `wsx.baseRemote` (D-08, D-02) | This phase | Fixture and migration must write the new form |
| `origin/HEAD` as the default base | `develop`, then `next`, then `<remote>/HEAD`, with stale accepted (D-06, D-10) | This phase | `.planning/research/WORKSPACE-MODE.md` R3 records devflow's `origin/HEAD` as `main`; it reads `develop` today (measured), which the chain handles either way |

**Deprecated/outdated:** STATE.md's blocker note that gates run with `PATH=$PWD/target/gsd-tools:$PATH`: that directory does not exist now. `.tools/bin` holds the pins, and `scripts/check.sh` already prefers it.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | D-01's "local repo config" excludes `include.path` files (`level == 0`), so a `wsx.*` key in an included file is unset | Pattern 1 | Someone who deliberately includes wsx config reads as not configured. Low, and the fix is one filter term |
| A2 | "A remote exists" (D-03, D-04) means defined in this repo, as `git remote add` decides, not as `git remote` lists | Pattern 2 | A remote defined only in global config counts as missing. Low: such a remote has no repo-specific tracking refs in practice |
| A3 | D-21's `refs/heads/workspace/*` is one level, as git's glob reads it | Pattern 4 | `workspace/a/b` alone gives "zero candidates", naming the fix, instead of being picked. Fails safe either way |
| A4 | Each `wsx.*` rule in D-18 applies per key read by this phase, not across the section | Pitfall 15 | Reading it as a section scan would break GUARD-05's empty value |
| A5 | `wsx.baseRef=HEAD`, a leading `-` and other values `git check-ref-format --branch` refuses are errors, an extension of D-08 and D-21 | Pattern 3 | Harmless: git can't create such branches |
| A6 | Whitespace in D-19 means space, tab, CR, VT, FF | Code Examples | Only space can reach the unquoted path from git's own output, so only hand-written lines are affected |

These are readings of decisions the owner left to research; each was measured against git where git has an opinion. The plan can state them as decided (they fall under Claude's Discretion in CONTEXT); plan review can challenge them.

## Open Questions

1. **Does an included file count as "local repo config" (D-01)?**
   - What we know: `git config --local --get` says no, and that is the control CONTEXT names. Plain `git config --get` and gix's `Source::Local` say yes. CLAUDE.md explains why a live include is dangerous.
   - What's unclear: whether anyone wants wsx config through an include.
   - Recommendation: `level == 0`, as in the prototype. Recorded as A1.

2. **Should a remote defined only in global config exist for D-03 and D-04?**
   - What we know: git has both answers. `git remote` lists it, and `git remote add` treats the name as free.
   - What's unclear: nothing that blocks work.
   - Recommendation: in-repo only, which keeps D-01's reasoning and makes the phantom-origin case resolvable. Recorded as A2.

3. **Do nested branches count as fallback candidates?**
   - What we know: the oracle's `for-each-ref 'refs/heads/workspace/*'` excludes them. `git branch --list` was not measured.
   - Recommendation: one level. Recorded as A3.

4. **Which order for reads when several are broken?**
   - The prototype reports the first failure only: the branch, then remotes, then integration, then the worktree dir, then the manifest. D-09 and D-12 require naming the refs and lines of one failure, not every failure, so this meets them. A manifest with two bad lines reports the first. Phase 5 can revisit this if `wsx check` wants a full list.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| git | Fixtures, differential tests | ✓ | 2.56.0 | — |
| rustc, cargo | Build | ✓ | 1.99.1 (pinned) | — |
| cargo-nextest | `scripts/check.sh test` | ✓ | 0.9.148 in `.tools/bin` (the pin); host PATH has 0.9.150 | `check.sh` puts `.tools/bin` first |
| cargo-deny | `scripts/check.sh deps` | ✓ | 0.20.2 | — |
| Docker | `scripts/check-in-container.sh` | ✓ | 29.9.2 | — |
| cargo-mutants | REL-03, later | ✓ | 27.1.0 (`~/.cargo/bin`, not pinned here) | — |
| shellcheck | Not needed: no script changes | ✓ | — | — |
| crates.io | Fetching proptest on first build | ✓ this session | — | Fails the build, doesn't skip |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none.

The container image is `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie` (`.devcontainer/Dockerfile:14`). Its git version was not checked this session. The prototype's behaviour was not run in it, so the phase gate must.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | libtest unit tests in the binary crate (`src/model/`, `src/git/`), proptest 1.12.0, run by cargo-nextest 0.9.148 |
| Config file | `.config/nextest.toml` (exists); `--no-tests=fail` is fixed by `scripts/check.sh` |
| Quick run command | `scripts/check.sh test -E 'test(=model::manifest::tests::<name>)'`, one test per command, chained with `&&` |
| Full suite command | `scripts/check.sh all`, plus `scripts/check.sh deps` (dependency added) |
| Estimated runtime | The prototype's 11 tests took 0.24 s; all unit tests with the prototype added took 4.4 s (131 run, 16 ignored child halves). The whole `check.sh test`, with the harness, was not timed this session |

### Phase Requirements to Test Map

Names are proposals; the planner fixes them. Each has a control that must change the answer. Repo-backed tests call `testing::rerun_scrubbed` first unless noted. `(child)` means run through `testing::reexec` with a scratch env, asserting a `WSXCHILD` marker.

| Req ID | Behavior | Test Type | Proposed test and control | File Exists? |
|--------|----------|-----------|---------------------------|-------------|
| CONF-01 | Any git path round-trips C-quoted | proptest | `model::manifest::tests::any_git_path_round_trips`. Control: the planted bare-edge encoder fails it | ❌ Wave 0 |
| CONF-01 | git's own quoted output parses to git's raw bytes on every byte value; exactly ` lead` and `trail ` are refused bare | unit, differential | `model::manifest::tests::git_quoted_output_parses_to_git_raw_names` | ❌ |
| CONF-01 | `.planning` covers `.planning/x`, not `.planningX`; a literal `a*b` covers only itself | proptest and differential | `name_extension_is_not_covered`; `covers_matches_a_literal_pathspec` against `git ls-files -- ':(literal)<path>'` (measured: `.planning` and `.planning/` give `.planning/x`, `.plan` nothing, `a*b` only `a*b/c`) | ❌ |
| CONF-01 | Unknown line kind names the line | unit | `unknown_kind_names_the_line`: number and text. Control: `only a` parses | ❌ |
| CONF-01 | D-11, D-12, D-14, D-19 line rules | unit, one per rule | One atomic test per row of Pattern 7's table, not a chained table (global rule) | ❌ |
| CONF-02 | Undeclared `.planning` or `CLAUDE.md` is not personal; once declared `only`, it is | unit, repo-backed | `undeclared_path_is_not_personal`, `declared_path_is_personal`: the same path before and after one manifest line | ❌ |
| CONF-02 | Built-ins are `.wsx.conf` and the worktree dir; `wsx.worktreeDir` moves the second | unit, repo-backed | `custom_worktree_dir_replaces_the_default`: `.worktrees/x` stops being personal | ❌ |
| CONF-02 | A built-in declaration still parses | unit | `declaration_covered_by_a_built_in_parses` (D-13) | ❌ |
| CONF-03 | `develop` beats `origin/HEAD=main`, then `next`, then the default | unit, repo-backed | Three tests, deleting one tracking ref at a time | ❌ |
| CONF-03 | A stray local `develop` doesn't redirect | unit, repo-backed | `local_develop_does_not_redirect_a_remote_base`. Control: no remotes, and the same local `develop` wins (D-07) | ❌ |
| CONF-03 | `wsx.baseRef` overrides; missing is an error with no fallback | unit, repo-backed | `base_ref_overrides_the_chain`, `missing_base_ref_names_the_ref`: `upstream/next` tries `refs/remotes/upstream/upstream/next` | ❌ |
| CONF-03 | D-09 error lists refs and fixes | unit, repo-backed | `no_integration_branch_lists_the_refs_tried`, plus dangling `<remote>/HEAD` and non-symbolic `<remote>/HEAD` variants | ❌ |
| CONF-04 | Fork: base `upstream`, push `origin`; both default to `origin` | unit, repo-backed | `fork_layout_resolves_each_remote`, `remotes_default_to_origin` | ❌ |
| CONF-04 | A key naming a missing remote names both (D-03); no origin, no key, a remote under another name means none (D-04) | unit, repo-backed | `missing_remote_names_the_key_and_remote`, `no_origin_means_no_remote` | ❌ |
| CONF-04 | A global `remote.origin` section doesn't create a remote | unit (child) | `global_remote_section_is_not_a_remote`. Control: `git remote` lists it, `git remote add` succeeds | ❌ |
| CONF-05 | `wsx.sourceBranch` names the branch; a missing branch fails closed | unit, repo-backed | `source_branch_names_the_workspace`, `missing_source_branch_fails_closed` | ❌ |
| CONF-05 | The only candidate is used; several or zero lists them; `personal/workspace` with `-backup` | unit, repo-backed | `only_candidate_is_inferred`, `several_candidates_are_listed`, `zero_candidates_are_an_error` | ❌ |
| CONF-05 | A recorded branch is excluded (D-17) | unit, repo-backed | `recorded_branches_are_not_candidates`: the same repo with and without the set | ❌ |
| CONF-05 | `workspace-x`, `workspaceY`, remote-only and the checked-out branch get no credit | unit, differential | `candidates_match_for_each_ref` against `git for-each-ref` | ❌ |
| CONF-07 | Uncommitted edit changes nothing; clean checkout resolves the same | unit, repo-backed | `working_tree_edit_changes_nothing`, `clean_checkout_resolves_the_same_list` | ❌ |
| CONF-07 | Configured plus missing, symlink, directory, submodule, missing object or invalid manifest fails closed with the reason | unit, one each | `missing_manifest_fails_closed`, `symlink_manifest_fails_closed`, `directory_manifest_fails_closed`, `submodule_manifest_fails_closed`, `missing_blob_object_fails_closed`, `invalid_manifest_fails_closed`. Differential for kind: `git ls-tree`; for the object: `git cat-file -e` fails | ❌ |
| CONF-07 | Not configured is reported; a bad inferred manifest stays "not configured" (D-16) | unit, repo-backed | `no_source_branch_is_not_configured`, `invalid_inferred_manifest_is_not_configured`. Control: the same manifest with the key set fails closed | ❌ |
| D-01 | A global `wsx.sourceBranch` is ignored | unit (child) | `global_source_branch_does_not_configure`. Control: `git config --get` returns it, `--local --get` does not | ❌ |
| D-18 | Repeated, empty and no-`=` keys are errors | unit, repo-backed | `repeated_key_is_an_error`, `empty_key_is_an_error`, `key_without_a_value_is_an_error`, each against `git config --local --get-all` | ❌ |
| D-01 | Include and `config.worktree` values are not read | unit, repo-backed | `included_key_is_not_read`. Control: `git config --local --includes --get` returns it | ❌ |
| Fixture | Harness scenarios still pass with bare `wsx.baseRef` | harness | Existing `devflow_layout_matches_the_preset` and the fork test at `tests/harness/fixture.rs:828`, updated | ✅ update |

### Sampling Rate
- **Per task commit:** the task's named tests, through the quick run command.
- **Per wave merge:** `scripts/check.sh all`.
- **Phase gate:** `scripts/check.sh all`, `scripts/check.sh deps` and `scripts/check-in-container.sh all`, green before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `Cargo.toml`, `Cargo.lock`: add proptest with a justification comment, `scripts/check.sh deps` clean.
- [ ] `src/git/mod.rs`: `#[cfg(test)] pub(crate) mod testing;`, the new `mod` lines, `pub use repo::Repo;`.
- [ ] `src/main.rs`: `mod model;`.
- [ ] `src/model/mod.rs`, `src/model/manifest.rs`: signatures with wrong-answer stubs, for RED by assertion. `src/model/mod.rs` carries the `dead_code` expectation.
- [ ] Shared model-test fixtures: a helper building a repo with a remote, `develop`, `next`, and a `workspace/dev` with a `.wsx.conf` (the prototype's `repo_with_remote`).

No new test framework config is needed. `.gitignore` needs no change, since persistence is off.

### Suggested implementation order (fine granularity)
1. proptest dependency, visibility changes, harness fixture values, with the harness run.
2. `git/quote.rs` with the all-bytes differential.
3. `model/manifest.rs`: `covers`, `normalize_path`, `parse`, `encode_path`, the proptests and one test per grammar rule. No repository, so nothing else is required first.
4. `git/config.rs` with the scope tests.
5. `git/refs.rs`: `find_ref`, `refs_directly_under`, `root_entry`, each against git.
6. `model/mod.rs`: remotes, then integration, then workspace and manifest, then `resolve`, `RepoModel`, `Error`, `Display`.
7. Phase gate including the container.

## Security Domain

`security_enforcement: true`, ASVS level 1.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No credentials; no network in resolution (D-09) |
| V3 Session Management | no | — |
| V4 Access Control | yes | Phase 3's trust and ownership refusals run in `Repo::discover` before any read here; local-scope config (D-01); fail closed once configured (CONF-07) |
| V5 Input Validation | yes | The grammar parser refuses unknown lines; path rules (D-14) plus NUL; `is_branch_name`; `FullName::try_from` for every built ref name |
| V6 Cryptography | no | SHA-1 and SHA-256 object ids are gix's |
| V7 Error Handling and Logging | yes | One typed refusal per cause, each naming the reason; manifest and config bytes escaped in messages (Pitfall 16) |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Global or included config silently configures an unrelated repo | Elevation of privilege | `source == Local && level == 0`; negative controls against git |
| A symlink at `.wsx.conf` smuggling a manifest that reads as its link text | Tampering | Tree-entry mode check before the blob read |
| A path hiding through quoting or a name prefix, so a personal path looks clean | Information disclosure | Raw-byte comparison on a boundary (`covers`); all-bytes differential; proptest |
| An ambiguous manifest leaving a path both replayed and stripped | Tampering | Cross-kind overlap refusal (D-12) |
| Terminal escape sequences in a manifest line or value reach the user's terminal | Spoofing | `c_quote` in `Display` |
| A stray branch blocks pushes in an unrelated repo | Denial of service | `NotConfigured` is a value, and the guard exits 0 (D-16) |
| A hostile manifest size | Denial of service | Not bounded: `find_blob` reads it whole, and overlap checking is quadratic in entries. Accepted for ASVS L1: the blob is on the user's own branch |

## Sources

### Primary (HIGH confidence)
- Scratch probe binary and prototype on gix 0.90.0 (gix-config 0.62.2, gix-ref 0.69.1, gix-object 0.66.1, gix-quote 0.10.1), against git 2.56.0: every "measured" claim above
- gix sources read this session: `gix-0.90.0/src/lib.rs:155-165`, `src/repository/config/remote.rs:23-34`, `src/object/tree/mod.rs:56-100`, `src/reference/iter.rs:45-87`; `gix-quote-0.10.1/src/ansi_c.rs`; `gix-ref-0.69.1/src/store/file/find.rs:88-133`; `gix-config-0.62.2/src/file/access/comfort.rs:238`
- Repo files read this session: `src/git/{mod,repo,reads,testing}.rs`, `tests/harness/fixture.rs`, `scripts/workspace/cut-pr-branch.sh`, `.workspace-divergence`, `clippy.toml`, `deny.toml`, `.config/nextest.toml`, `Cargo.toml`, `CLAUDE.md`, `CONTRIBUTING.md`, `.planning/{REQUIREMENTS,ROADMAP,STATE}.md`, 04-CONTEXT.md

### Secondary (MEDIUM confidence)
- Context7 `/proptest-rs/proptest`: `Config` fields, `failure_persistence`, `rng_seed`, `PROPTEST_RNG_SEED`, file persistence. Confirmed where it matters by running 1.12.0
- crates.io API and the `package-legitimacy` seam: proptest metadata and features

### Tertiary (LOW confidence)
- None used.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH. Built and gated in a scratch copy; `cargo deny` with a control.
- gix reads: HIGH. Each measured with a control or a git differential, on one git version.
- Grammar: HIGH for byte handling. MEDIUM for the readings A3, A5, A6.
- Model shape and errors: MEDIUM. The prototype works, but this is a judgement call.
- Harness fixture impact: MEDIUM. The contradiction was read from source; the fix was not run.

**Research date:** 2026-11-02
**Graph context:** the knowledge graph is 326 commits behind (built at `9e96bc8`) and returned only Python 0.1.0 and planning nodes, so it was not used.
**Valid until:** the next gix upgrade (monthly, per `Cargo.toml`), or 30 days.
