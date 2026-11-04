# Phase 4: Repo model - Pattern Map

**Mapped:** 2026-11-04 (regenerated after inserted phase 03.1 landed)
**Files analyzed:** 12 (4 new, 8 modified)
**Analogs found:** 10 / 12 (`Cargo.lock` is generated; `.planning/STATE.md` is prose). Four sub-patterns have no analog; see "No Analog Found".

Every excerpt below was read from the working tree on branch `workspace/denniyahh` at `6fbea02`, not from 04-RESEARCH.md's description of the code. Every analog path was checked with `git ls-files` and is tracked. Code is cited as symbol plus current line range. The gix calls for the new reads are not repeated here: 04-RESEARCH.md Patterns 1-8 hold them, subject to the corrections in the next section.

## Corrections to RESEARCH.md after phase 03.1

04-RESEARCH.md (2026-11-02) predates 03.1. Parts of it were later patched (Pattern 3, Pattern 6 and the Code Examples already use `peel_commit`/`peeled_commit` and `refuse_unfollowable_replacements`), but these statements no longer match the code:

1. **`Repo::resolve` and `Repo::resolve_commit` no longer exist** (`rg 'fn resolve|resolve_commit' src/` finds nothing). CONTEXT.md's Reusable Assets still lists them. The replacement is in `src/git/refs.rs`:
   - `Repo::peel_commit(&self, id: &ObjectId) -> Result<ObjectId, Error>` (lines 7-13), `pub`.
   - `Repo::peeled_commit(&self, id: &ObjectId) -> Result<gix::Commit<'_>, Error>` (lines 15-28), **private** (`fn`, no `pub`). It is callable only from code in `src/git/refs.rs` itself, so `root_entry` must live in `refs.rs` (as planned) or `peeled_commit` must become `pub(super)`.
   - Both take an `ObjectId` and decode it as hex (`ObjectId::to_gix`, `src/git/mod.rs` 46-64). A name or short prefix is refused. Nothing in `src/git/` resolves a name any more.
2. **gix's revspec parsers are clippy errors.** `clippy.toml` lines 52-66 ban `gix::Repository::rev_parse`, `gix::Repository::rev_parse_single`, `gix::revision::Spec::from_bstr` and `gix::revision::plumbing::spec::parse`. RESEARCH Pattern 3 (line 354), Pattern 6's table (`rev_parse_single("<tip>:.wsx.conf")`, line 465), Pitfall 5 and Pitfall 8 describe measurements made with them. Those measurements stand as evidence, but no production code may call them. A test that calls one as a gix control needs the reasoned expectation shown under "Test-only use of a banned call" below; the alternative is to drop the gix control and cite `banned_gix_revspec_parsers_read_a_describe_shaped_ref_as_another_object` (`src/git/repo.rs` 985-1057), which already pins the divergence.
3. **`src/git/refs.rs` exists.** RESEARCH's project structure (line 261) marks it `# NEW`. It is a modification: 04-01 adds `find_ref`, `ref_commit`, `root_entry`, `is_branch_name` and `RefTarget`/`RootEntry` to the existing file, and must not re-stub `peel_commit`. `mod refs;` is already declared (`src/git/mod.rs` line 26).
4. **Line references that moved:**
   - `src/git/mod.rs:25-29` (RESEARCH line 270) is now lines 25-30: `mod reads; mod refs; mod repo; mod spawn; #[cfg(test)] mod testing;`.
   - `#[cfg(test)] mod testing;` (Pitfall 12, `:28-29`) is now lines 29-30.
   - `ref_storage_in_global_config_does_not_refuse_a_files_repo` (Pitfall 13, `src/git/repo.rs:2221`) is now at 2231 (doc 2227-2229).
   - Unchanged and still correct: `src/git/mod.rs:23` (dead_code attribute), `src/git/mod.rs:52`-area text "Why the git layer could not answer" (now line 73), `refuse_ref_storage` called at `src/git/repo.rs:37`, `tests/harness/fixture.rs:263-266`, `:670`, `:828`.
5. **Pitfall 10's backstop is gone.** It says "the `is_unsupported()` backstop in `Repo::resolve` does not fire". There is no `is_unsupported` call and no `Repo::resolve`. The conclusion still holds: `refuse_ref_storage` (`src/git/repo.rs` 541-554) runs in `Repo::discover` (line 37) before any read.
6. **`DISCOVERING_SUITES` already has length 3**, not 2: `git::reads::tests::`, `git::refs::tests::`, `git::repo::tests::` (`src/git/testing.rs` 1052-1058). 04-01 Task 2's baseline text "the array has length 2" is stale.
7. **Defect in 04-01 Task 2 as written: adding `model::tests::` to `DISCOVERING_SUITES` fails an existing test.** `discovering_suites_are_the_modules_that_rerun_scrubbed` (`src/git/testing.rs` 1120-1166) scans only `src/git` (line 1126, `Path::new(env!("CARGO_MANIFEST_DIR")).join("src/git")`) for files containing `rerun_scrubbed() {` and asserts that set equals `DISCOVERING_SUITES` (lines 1162-1165). `model::tests::` would be listed but never found as a caller. The plan must also extend the scan to `src/model` (mapping `src/model/mod.rs` to `model::tests::`), or the suite goes red. Likewise, `git::config::tests::` is accepted only if `src/git/config.rs` contains the literal text `rerun_scrubbed() {`; a config test module that uses only `testing::reexec` children would make the two sets disagree. This was read from source, not run.
8. **Model tests cannot use `Git` from `spawn`.** `mod spawn;` is private to `git` (line 28) and not re-exported. Model tests use `testing::git_ok` (the `refs.rs`/`repo.rs` style), not `Git::new(...).run` (the `reads.rs` style, `src/git/reads.rs` 263-270).
9. **`.as_bstr()` needs an import nothing in `src/git/` has yet.** There is no `ByteSlice` import; production code names `gix::bstr::BString` only (`src/git/repo.rs` line 165) and tests import `gix::bstr::BStr` (`src/git/repo.rs` line 589). RESEARCH Patterns 2, 4 and 5 call `.as_bstr()`, so `config.rs`, `refs.rs` and `quote.rs` need `use gix::bstr::ByteSlice;` (or `BStr::new(bytes)`, as the pin test does at line 1003).
10. **Ref differentials must not ask `git rev-parse`.** `testing::object_id` (`src/git/testing.rs` 308-313) runs `git rev-parse --verify`, which resolves by git's DWIM rules (RESEARCH Pitfall 5). Use it to get an id for a revision the test made; to ask whether a full ref name exists, use `git show-ref --verify <full>` through `testing::git`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/git/config.rs` (new) | service, git read | request-response | `src/git/repo.rs` `ref_storage` (160-171) and `implicit_worktree` filters (490-495); `src/git/reads.rs` for module shape | role-match: same read API, but RESEARCH Pattern 1 swaps `string_filter` for `strings_filter` |
| `src/git/refs.rs` (modified) | service, git read | request-response | itself (`peel_commit`/`peeled_commit` 6-29, tests 31-461); `src/git/reads.rs` `tree_of` (162-179), `file_change`/`entry_class` (194-235) | exact |
| `src/git/quote.rs` (new) | utility | transform | `src/git/mod.rs` `ObjectId::from_gix`/`to_gix` (41-64), a thin gix wrapper | partial |
| `src/model/mod.rs` (new) | service (policy) plus error type | transform over git reads | `src/git/mod.rs` header (1-23) and `Error` (73-231); `src/git/refs.rs` tests for repo-backed fixtures | role-match: errors exact, no policy layer exists yet |
| `src/model/manifest.rs` (new) | utility, pure parser | transform | `src/git/spawn.rs` `version_part`/`parse_git_version` (34-55); oracle `scripts/workspace/cut-pr-branch.sh` 163-164 for behaviour | partial |
| `src/main.rs` (modified) | config, module list | none | itself, line 10 | exact |
| `src/git/mod.rs` (modified) | config, module list, error enum | none | itself, lines 25-30 and 73-231 | exact |
| `src/git/testing.rs` (modified) | test support | none | itself, lines 1052-1166 | exact |
| `tests/harness/fixture.rs` (modified) | test fixture | batch | itself, lines 263-272, 668-671, 827-830 | exact |
| `Cargo.toml` (modified) | config | none | itself, lines 31-37 (`tempfile`) | exact |
| `Cargo.lock` (modified) | config | none | generated by cargo | not applicable |
| `.planning/STATE.md` (modified, 04-10) | docs | none | none needed | not applicable |

## Pattern Assignments

### `src/git/refs.rs` (service, request-response) — modified

**Analog:** the file itself plus `src/git/reads.rs`.

**Module header and imports today** (`refs.rs` 1-4). The header must grow to describe exact-name ref reads and the root-entry read; keep `reads.rs`'s style of stating what the read never does (`reads.rs` 1-9):
```rust
//! Peeling object ids to the commits git names for them.

use super::repo::Repo;
use super::{Error, ObjectId};
```
`reads.rs` 1-9, the model for the expanded header:
```rust
//! Read queries answered by gix; callers never hold a gix type.
//! Each read takes object ids, never a name. The commit queries
//! (`merge_base`, `ahead_behind`, `range_walk`) peel their ids to commits
//! with `Repo::peel_commit`, and `tree_diff` peels its ids to trees, so no
//! read resolves a name or passes one to gix's revspec parser. The
//! `&ObjectId` signatures are the guard against a name reaching that parser;
//! the clippy ban in `clippy.toml` and its pin test in `repo.rs`
//! (`banned_gix_revspec_parsers_read_a_describe_shaped_ref_as_another_object`)
//! keep it that way.
```

**The peel every new read builds on** (`refs.rs` 6-29). `ref_commit` peels the id a `find_ref` read returned through `peel_commit`; `root_entry` calls `peeled_commit` to get the `gix::Commit` and then `.tree()`:
```rust
impl Repo {
    /// The commit `id` names, peeling annotated tags as git's
    /// `<id>^{commit}` does. No ref is read and no revspec is parsed: `id`
    /// is decoded as hex, so a name or a short prefix is refused.
    pub fn peel_commit(&self, id: &ObjectId) -> Result<ObjectId, Error> {
        self.peeled_commit(id)
            .map(|commit| ObjectId::from_gix(&commit.id))
    }

    /// The commit `id` peels to, as gix's commit. A decode failure is
    /// `ObjectId::to_gix`'s error; a missing object, or one that does not
    /// peel to a commit, is `Error::Gix` naming `id`.
    fn peeled_commit(&self, id: &ObjectId) -> Result<gix::Commit<'_>, Error> {
        let gix_id = id.to_gix(self.repo.object_hash())?;
        self.repo
            .find_object(gix_id)
            .map_err(|err| err.to_string())
            .and_then(|object| object.peel_to_commit().map_err(|err| err.to_string()))
            .map_err(|message| Error::Gix {
                what: format!("peel {id} to a commit"),
                message,
            })
    }
}
```

**Public result types: documented variants, derives, raw bytes** (`reads.rs` 27-48). `RefTarget { Object(ObjectId), Symbolic(Vec<u8>) }` and `RootEntry { Absent, Blob(Vec<u8>), NotABlob { kind } }` follow this:
```rust
/// How a path differs between two trees.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeKind {
    /// The path exists only in the second tree.
    Added,
    ...
}

/// One changed file between two trees: what changed and at which path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TreeChange {
    /// How the path changed.
    pub kind: ChangeKind,
    /// The path from the tree root, as raw bytes with `/` separators.
    pub path: Vec<u8>,
}
```

**Error wrapping: a `failed` closure naming the operation** (`reads.rs` 101-110), the shape for every gix call in `find_ref`, `refs_directly_under` and `root_entry`:
```rust
        let failed = |message: String| Error::Gix {
            what: format!("walk {} --not {}", render_ids(tips), render_ids(hidden)),
            message,
        };
        let walk = self
            .repo
            .rev_walk(tip_ids)
            .with_hidden(hidden_ids)
            .all()
            .map_err(|err| failed(err.to_string()))?;
```

**Tree-entry mode, read without following** (`reads.rs` 222 and 228-235). `root_entry` classifies `.wsx.conf`'s entry the same way, through `EntryMode`, never through `find_blob` alone (RESEARCH Pitfall 8):
```rust
    (!mode.is_tree()).then(|| TreeChange {
        kind,
        path: path.into(),
    })
...
fn entry_class(mode: gix::object::tree::EntryMode) -> gix::object::tree::EntryKind {
    match mode.kind() {
        gix::object::tree::EntryKind::BlobExecutable => gix::object::tree::EntryKind::Blob,
        kind => kind,
    }
}
```

**Test module imports** (`refs.rs` 31-38). New tests extend this list; no `Git` import (correction 8):
```rust
#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Output;

    use super::super::testing::{self, Scratch, T0, TICK, commit_all, git_ok, write};
    use super::super::{Error, ObjectId};
    use super::Repo;
```

**Fixture builder that checks git agrees before the test relies on it** (`refs.rs` 40-65, `tagged_repo`): build with `git_ok`, then assert git's own view of what was built (`cat-file -t`) so a broken fixture fails as a fixture fault. The SHA-256 variant is `repo_with_format` (`refs.rs` 125-145).

**Git oracle helper that returns the whole `Output`** (`refs.rs` 73-79), the shape for `git show-ref --verify <full>`, `git ls-tree`, `git cat-file -e`:
```rust
    fn git_peel(scratch: &Scratch, dir: &Path, hex: &str) -> Output {
        let rev = format!("{hex}^{{commit}}");
        testing::git(scratch, dir, ["rev-parse", "--verify", rev.as_str()])
            .unwrap_or_else(|err| panic!("{err}"))
    }
```

**Refusal asserted against git's refusal** (`refs.rs` 96-123, `assert_refused_as_git_does`): git must exit non-zero, and wsx must return the specific variant with a rendered prefix. Use for missing blob, missing tree, symlink, directory and submodule `root_entry` cases.

**Test body shape** (`refs.rs` 156-180):
```rust
    #[test]
    fn peel_commit_of_a_commit_is_that_commit() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-commit").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let head = id(&scratch, &repo_dir, "HEAD");
        let from_git = git_peeled(&scratch, &repo_dir, &head.to_string());
        ...
        assert_eq!(
            peeled.to_string(),
            from_git,
            "peel_commit({head}) against git's {from_git}"
        );
    }
```

**Describe-shaped ref fixture already exists** (`src/git/testing.rs` 423-491, `describe_shaped_ref`): `refs/remotes/origin/rel-1-g<7 hex of Y>` holding ONE. `ref_commit_reads_the_commit_a_describe_shaped_ref_holds` should reuse it; `refs.rs` 437-460 (`peel_commit_refuses_a_describe_shaped_ref_name`) shows the call:
```rust
        let fixture = testing::describe_shaped_ref(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let repo = Repo::discover(&fixture.repo).expect("discover fixture");
```

**Raw-byte differential parsing with `-z`** (`reads.rs` 334-337), for `git for-each-ref` (`candidates_match_for_each_ref`, `refs_directly_under_matches_for_each_ref`) and `git ls-tree -z`:
```rust
        let mut fields: Vec<&[u8]> = raw.split(|byte| *byte == 0).collect();
        if fields.last() == Some(&&b""[..]) {
            fields.pop();
        }
```

**Symlink fixture** (`src/git/testing.rs` 384-385), for `symlink_manifest_fails_closed`:
```rust
    std::os::unix::fs::symlink("a.txt", &p)
        .map_err(|err| format!("symlink {} -> a.txt: {err}", p.display()))?;
```

**Replacement-ref refusal (04-11)** reads the environment like `refuse_object_store_env` (`src/git/repo.rs` 271-293): a `const` list or name, `std::env::var_os`, and a typed `Error` naming the variable and value:
```rust
fn refuse_object_store_env() -> Result<(), Error> {
    match OBJECT_STORE_VARS
        .into_iter()
        .find_map(|name| Some((name, std::env::var_os(name)?)))
    {
        None => Ok(()),
        Some((name, value)) => Err(Error::ObjectStoreEnv {
            name,
            value: PathBuf::from(value),
        }),
    }
}
```
Tests that need `GIT_REPLACE_REF_BASE` set must go through `testing::reexec` with an `Env` (see Shared Patterns, child halves), never `set_var`.

---

### `src/git/config.rs` (service, request-response) — new

**Analog:** `src/git/repo.rs` `ref_storage` (160-171) for the config call, `src/git/reads.rs` for module shape (header, `impl Repo`, `use super::repo::Repo;`).

**The local-scope read to adapt** (`repo.rs` 160-171). Keep the doc comment's habit of recording what git was measured to do:
```rust
/// The `extensions.refStorage` of the repository's own config (gix's
/// `Source::Local`, the common dir's `config`), or `None` when it is not set
/// there. git reads repository extensions from that file alone: measured with
/// git 2.56.0, a `reftable` value in global config, in `config.worktree` or
/// through `GIT_CONFIG_COUNT` leaves a files repository readable.
fn ref_storage(repo: &gix::Repository) -> Option<gix::bstr::BString> {
    repo.config_snapshot()
        .plumbing()
        .string_filter("extensions.refStorage", |meta| {
            meta.source == gix::config::Source::Local
        })
}
```
**Differences:** `local_config_values` is a `pub fn` on `impl Repo` (reading `self.repo`, which is `pub(super)` at `repo.rs` line 12), uses `strings_filter`, and adds `meta.level == 0` (RESEARCH Pattern 1, Pitfalls 1-2). Do not reuse `string_filter`: it returns the last value and reads a no-`=` key as absent.

**Named filter closures over `Metadata`** (`repo.rs` 490-495), the shape for `remote_configured`'s `Source::Local | Source::Worktree` match:
```rust
    let config = repo.config_snapshot();
    let config = config.plumbing();
    let in_worktree_config =
        |meta: &gix::config::file::Metadata| meta.source == gix::config::Source::Worktree;
    let in_repo_config =
        |meta: &gix::config::file::Metadata| meta.source == gix::config::Source::Local;
```

**Global-config tests run in a child** (`repo.rs` 2184-2239). `global_source_branch_does_not_configure` and `global_remote_section_is_not_a_remote` follow `ref_storage_in_global_config_does_not_refuse_a_files_repo`: append to `scratch.home.join(".gitconfig")`, then re-execute a child half:
```rust
    #[test]
    fn ref_storage_in_global_config_does_not_refuse_a_files_repo() {
        let scratch = Scratch::new("ref-storage-global").expect("scratch");
        let (repo, env) = files_repo_and_env(&scratch);
        let global = scratch.home.join(".gitconfig");
        let mut config = std::fs::read(&global).expect("read the scratch global config");
        config.extend_from_slice(b"[extensions]\n\trefStorage = reftable\n");
        write(&global, &config).expect("append extensions.refStorage");
        assert_both_read_files(&scratch, &env, &repo);
    }
```
`files_repo_and_env` (2184-2196) builds the repo and an `Env` carrying `START_VAR`; `assert_both_read_files` (2198-2225) runs git with `testing::git_in_env` and the child with `testing::reexec(&child_path("ref_storage_child"), env, &scratch.root, b"")`, combines both outputs into one failure message, and asserts `testing::marker(&child, "result")`. The child half (2270-2274):
```rust
    #[test]
    #[ignore = "child half of the ref-storage source tests; runs only when re-executed"]
    fn ref_storage_child() {
        print_discovery();
    }
```
`child_path` (1097-1100) builds the libtest path, `format!("git::repo::tests::{name}")`; config.rs needs its own with `git::config::tests::`. `print_discovery` (2153-2176) prints `result=` plus details through `testing::child_print`, replacing `\n` in error text.

**Differential control for config:** `git config --local --get-all <key>` through `testing::git` (exit 1 means absent). CONTEXT's D-01 control: plain `git config --get` returns the global value, `--local --get` does not.

**Test-module discovery note:** if any config test discovers in-process it must start with `if testing::rerun_scrubbed() { return; }`, and that literal text is what `discovering_suites_are_the_modules_that_rerun_scrubbed` looks for (correction 7).

---

### `src/git/quote.rs` (utility, transform) — new

**Analog:** `src/git/mod.rs` `ObjectId::from_gix` and `ObjectId::to_gix` (41-64): a thin conversion at the gix boundary, wsx types in and out, gix's error crossing only as its message.
```rust
impl ObjectId {
    /// The wsx form of a gix object id: its lowercase hex.
    fn from_gix(id: &gix::oid) -> ObjectId {
        ObjectId(id.to_hex().to_string())
    }

    /// The gix form of this id in a repository whose objects are hashed
    /// with `kind`: hex decoding only, never revspec parsing, so no ref is
    /// read and no prefix or describe output is expanded. ...
    fn to_gix(&self, kind: gix::hash::Kind) -> Result<gix::ObjectId, Error> {
        let failed = |message: String| Error::Gix {
            what: format!("read object id {self}"),
            message,
        };
        let id =
            gix::ObjectId::from_hex(self.0.as_bytes()).map_err(|err| failed(err.to_string()))?;
        ...
    }
}
```
**Differences:** `c_quote` and `c_unquote` are `pub` (the model calls them), take `&[u8]`, return `Vec<u8>` / `Result<Vec<u8>, String>`, and `c_unquote` checks `consumed` (RESEARCH Pattern 5, Pitfall 6). Needs `use gix::bstr::ByteSlice;` (correction 9).

**Tests:** hermetic git children only, no in-process discover, so no `rerun_scrubbed` and no `DISCOVERING_SUITES` entry. Build the all-bytes fixture with `testing::write` and `testing::git_ok`; split `git ls-files -z` with the `-z` pattern above and `git ls-files` (quoted) on `\n`.

---

### `src/model/mod.rs` (policy service and error type) — new

**Analog:** `src/git/mod.rs` for the module header, the `dead_code` expectation and the hand-written `Error`; `src/git/refs.rs` tests for repo-backed fixtures.

**Module header and dead-code expectation** (`git/mod.rs` 1-23). Copy the attribute verbatim; the reason string must stay short enough for rustfmt to keep it on one line (comment at 19-22):
```rust
//! wsx's git layer: repository resolution and reads through gix, history
//! changes through the `git` binary.
...
//! gix types never leave this module: callers get wsx's own types (`ObjectId`,
//! raw-byte paths), and a gix error crosses the boundary only as its message.
//!
//! Every refusal names its cause: an `Error` renders a message a user can act
//! on without reading wsx's source.

// Nothing in `main` calls this module until phase 5's `wsx check`, which
// removes this attribute. `cfg_attr(not(test), ...)`: the tests use every
// item, so on the test target the expectation would be unfulfilled. The
// reason is short so rustfmt keeps the attribute on one line.
#![cfg_attr(not(test), expect(dead_code, reason = "no caller until phase 5"))]
```

**Error enum: one documented variant per refusal, `Debug` only** (`git/mod.rs` 73-131):
```rust
/// Why the git layer could not answer.
#[derive(Debug)]
pub enum Error {
    /// No repository was found from `start`; `reason` is gix's explanation.
    NotARepo { start: PathBuf, reason: String },
    ...
    /// The repository at `path` stores its refs in the `format` backend
    /// (`extensions.refStorage`), which this version of wsx cannot read.
    Reftable { path: PathBuf, format: String },
    ...
    /// A gix operation failed; `what` names the operation, `message` is gix's.
    Gix { what: String, message: String },
}
```

**Hand-written `Display` that names the fix** (`git/mod.rs` 133-221). A multi-line message with the command to run on its own tab-indented line:
```rust
            Error::ForeignOwner { path } => write!(
                f,
                "refusing {path}: it is owned by another user, and git refuses such a repository too\n\
                 To trust it, run:\n\
                 \tgit config --global --add safe.directory {path}",
                path = path.display()
            ),
            ...
            Error::StaleWorktree { worktree, git_dir } => write!(
                f,
                "worktree {} is missing, but {} still registers it (git lists it as prunable)\n\
                 If it is gone for good, run:\n\
                 \tgit worktree prune",
                worktree.display(),
                git_dir.display()
            ),
```
then `impl std::error::Error for Error {}` (line 223). `model::Error` adds `impl From<git::Error> for Error` (variant `Git`), which has no analog in the codebase (see No Analog Found). Render manifest lines, paths and config values with `c_quote`, not `String::from_utf8_lossy` (RESEARCH Pitfall 16); the lossy rendering `git/mod.rs` uses for stderr (lines 171-177) is the pattern not to copy here.

**Display test: one rendering per variant, exact strings** (`git/mod.rs` 245-364, `error_messages_name_their_cause`):
```rust
        let stale = Error::StaleWorktree {
            worktree: PathBuf::from("/w"),
            git_dir: PathBuf::from("/m/.git/worktrees/w"),
        };
        assert_eq!(
            stale.to_string(),
            "worktree /w is missing, but /m/.git/worktrees/w still registers it (git lists it as prunable)\n\
             If it is gone for good, run:\n\
             \tgit worktree prune"
        );
```
Implement `Display` in the same plan that adds a variant: on the test target the `dead_code` expectation is off, and a field read nowhere fails clippy (RESEARCH Pitfall 11).

**Repo-backed tests:** `use crate::git::testing::{self, Scratch, T0, TICK, commit_all, git_ok, write};` once `testing` is `pub(crate)`. Each test opens with `if testing::rerun_scrubbed() { return; }` (`refs.rs` 158-160). Set config with `git_ok(&scratch, &dir, ["config", "--local", key, value])` **before** `Repo::discover`: config is frozen at open (RESEARCH Pitfall 9). The shared fixture (remote, `develop`, `next`, `workspace/dev` with `.wsx.conf`) follows `tagged_repo` (`refs.rs` 40-65): `git_ok` steps, `write`, `commit_all(..., T0 + n * TICK)`, then a git check that the fixture is what the test assumes.

**Imports from `git`:** `git` is a private module of the crate root (`src/main.rs` line 10), so `model` (a sibling) reaches `crate::git::{Error, ObjectId, Repo, ...}` only for items that are `pub` in `git/mod.rs`. `Repo` is defined in the private `mod repo`, so `git/mod.rs` needs `pub use repo::Repo;` (and likewise `pub use refs::{RefTarget, RootEntry};`, `pub use quote::{c_quote, c_unquote};` or equivalent).

---

### `src/model/manifest.rs` (utility, pure parser, transform) — new

**Analog:** `src/git/spawn.rs` `version_part` and `parse_git_version` (34-55): a small byte/text parser, documented with the exact shape it accepts, returning `None` on anything else. `manifest.rs` returns a typed `ParseError` naming the line instead.
```rust
/// One dot-separated version part as a number: ASCII digits only.
fn version_part(part: Option<&str>) -> Option<u32> {
    let part = part?;
    if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

/// The (major, minor) of `git --version` output: the first line's token after
/// `git version `, its first two dot-separated parts. Anything after them
/// (`.windows.1`, ` (Apple Git-155)`) is ignored. `None` when the output does
/// not have that shape.
fn parse_git_version(output: &[u8]) -> Option<(u32, u32)> {
    let line = first_line(output);
    let token = line
        .strip_prefix("git version ")?
        .split_whitespace()
        .next()?;
    let mut parts = token.split('.');
    Some((version_part(parts.next())?, version_part(parts.next())?))
}
```
**Differences:** operate on `&[u8]`, never `String::from_utf8_lossy` (paths are raw bytes, D-14/D-19). No gix import; quoting goes through `crate::git::c_unquote`/`c_quote`.

**Behavioural reference, the oracle** (`scripts/workspace/cut-pr-branch.sh` 163-164; tracked):
```bash
    while read -r KIND ENTRY _; do
        if [ "$KIND" = "only" ] && [ -n "$ENTRY" ]; then
```
wsx deliberately differs: `only a b` is the path `a b` and `only\ta` is an error (RESEARCH Pattern 7, D-19). Only `only` lines are personal.

**Tests:** no repository, so no `rerun_scrubbed` and no `DISCOVERING_SUITES` entry (04-01 Task 2 already keeps `model::manifest::tests::` out). One atomic test per grammar rule (global rule; RESEARCH Pattern 7 table). Proptest uses `failure_persistence: None` and a shared `property_config()` (RESEARCH Pitfall 17, Code Examples).

---

### `src/main.rs` (config) — modified

**Analog:** itself, line 10:
```rust
mod git;
```
Add `mod model;` beside it. No other change: nothing in `main` calls the model until Phase 5.

---

### `src/git/mod.rs` (config, error enum) — modified

**Module list today** (lines 25-30):
```rust
mod reads;
mod refs;
mod repo;
mod spawn;
#[cfg(test)]
mod testing;
```
Add `mod config;` and `mod quote;` (keep alphabetical order), change the last line to `pub(crate) mod testing;` (RESEARCH Pitfall 12), and add the `pub use` lines the model needs (see `src/model/mod.rs` above). `ObjectId` (line 38) and `Error` (line 75) are already `pub`.

**04-11 adds `git::Error` variants** (`ReplacementRef`, `ReplacementRefDuplicate`, `ReplaceBaseDiffers`) and `is_replacement_refusal`. Follow the variant doc style at 100-110 (`RelativeRepoEnv`: every field named in the doc) and add one exact-string case per variant to `error_messages_name_their_cause` (245-364).

---

### `src/git/testing.rs` (test support) — modified

**`DISCOVERING_SUITES` today** (lines 1052-1058):
```rust
/// The libtest filters for the modules whose tests discover a repository in
/// the test process itself.
const DISCOVERING_SUITES: [&str; 3] = [
    "git::reads::tests::",
    "git::refs::tests::",
    "git::repo::tests::",
];
```
**The scan that guards it** (lines 1120-1166) reads only `src/git`:
```rust
fn discovering_suites_are_the_modules_that_rerun_scrubbed() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/git");
    ...
        if source.contains("rerun_scrubbed() {") {
            callers.insert(if stem == "mod" {
                "git::tests::".to_owned()
            } else {
                format!("git::{stem}::tests::")
            });
        }
    ...
    assert_eq!(
        callers, listed,
        "modules calling rerun_scrubbed() against DISCOVERING_SUITES"
    );
}
```
Adding `model::tests::` requires extending this scan to `src/model` (correction 7). `discovering_tests_pass_with_a_runner_git_dir` (1084-1118) needs no change: it iterates the array and requires each suite to pass at least one test.

---

### `tests/harness/fixture.rs` (test fixture, batch) — modified

**The writes to change** (lines 263-272):
```rust
        let base_ref = format!("{}/{}", config.base_remote, config.integration);
        let mut local = vec![
            ("wsx.sourceBranch".to_owned(), config.workspace.clone()),
            ("wsx.baseRef".to_owned(), base_ref),
            ("commit.gpgsign".to_owned(), "false".to_owned()),
        ];
        local.extend(config.extra_config.iter().cloned());
        for (key, value) in &local {
            self.git_ok(WORK, ["config", "--local", key, value])?;
        }
```
Set `wsx.baseRef` to `config.integration`, and add `wsx.baseRemote` and `wsx.pushRemote` from `config.base_remote` and `config.push_remote` in the same `vec!` (RESEARCH Pitfall 14).

**The two assertions to update** (lines 668-671 and 827-830):
```rust
    for (key, value) in [
        ("wsx.sourceBranch", "workspace/dev\n"),
        ("wsx.baseRef", "origin/develop\n"),
    ] {
```
```rust
    for (key, value) in [
        ("wsx.baseRef", "upstream/develop\n"),
        ("fixture.layout", "fork\n"),
    ] {
```
**Config snapshots do not move:** `tests/harness/outcome.rs` `RefsExpect` (57-72) compares local config before and after an action (`config_added`, entries built by `config_entry`, 84-87, from `config --local --list -z` at line 414). Keys set at build time are in both snapshots. Read from source; the harness has not been run with the change.

---

### `Cargo.toml` (config) — modified

**Analog:** the `tempfile` dev-dependency and its justification (lines 31-37):
```toml
[dev-dependencies]
# Scenario tests build scratch git trees. tempfile's TempDir removes them even
# when a test panics, and `TempDir::new_in` keeps them under
# CARGO_TARGET_TMPDIR rather than the RAM-backed /tmp. Dev-only, so nothing
# ships in the binary; MIT OR Apache-2.0, and every crate in its graph offers
# MIT. Legitimacy checked in phase 2 research (Package Legitimacy Audit: OK).
tempfile = "3.27.0"
```
The `proptest` entry gets the same kind of comment: purpose, why `default-features = false, features = ["std"]`, dev-only, licence, and "Legitimacy checked in phase 4 research". Then `scripts/check.sh deps`.

## Shared Patterns

### Boundary: gix stays in `src/git/`
**Source:** `src/git/mod.rs` 13-14; `src/git/reads.rs` 1-9.
**Apply to:** `src/model/*` (no `gix::` path at all), and every new `pub fn` in `src/git/`, which takes and returns `&[u8]`, `Vec<u8>`, `ObjectId` or wsx enums. Ref and path names are raw bytes. Branch-name validity (`FullName::try_from`) lives in `git::is_branch_name`, not in the model (RESEARCH Anti-Patterns).

### Reads take ids or exact full names, never revspecs
**Source:** `clippy.toml` 52-66; `src/git/refs.rs` 7-13; `src/git/mod.rs` 46-64.
**Apply to:** `find_ref`, `ref_commit`, `root_entry`, every model read. A ref is read by its exact full name (`refs/heads/...`, `refs/remotes/<remote>/...`) and its id is peeled with `peel_commit`; the tip passed to `root_entry` is an `ObjectId`, never `"<tip>:.wsx.conf"`.

### Test-only use of a banned call
**Source:** `src/git/repo.rs` 1004-1011 (and the same shape at 794-871 for the open/discover ban):
```rust
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's revspec ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let single = found
            .repo
            .rev_parse_single(name)
            .map(|id| Some(id.to_string()));
```
**Apply to:** any test that keeps a gix revspec call as a control (03.1 Phase 4 Impact item 5). `expect` on the smallest item (the `let`), with a reason, never `allow`.

### Git-layer error handling
**Source:** `src/git/reads.rs` 101-110 and 168-179; `src/git/mod.rs` 129-130, 218.
**Apply to:** every gix call in `config.rs`, `refs.rs`, `quote.rs`. `Error::Gix { what, message }`, where `what` names the operation and subject (`format!("peel {id} to a commit")`) and `message` is `err.to_string()`. Policy refusals go in `model::Error`, not in `git::Error`, except the 04-11 replacement-ref refusals (RESEARCH Pattern 8).

### Test hermeticity
**Source:** `src/git/testing.rs` `Scratch` (81-120), `rerun_scrubbed` (735-785), `reexec` (640-680), `marker` (682-703), `MARKER` (633).
**Apply to:** every repo-backed test in `src/git/{config,refs}.rs` and `src/model/mod.rs`.
- `Scratch::new("<name>")` per test; every git child gets `scratch.env` (cleared env, scratch HOME with `GLOBAL_CONFIG`).
- In-process `Repo::discover` tests start with `if testing::rerun_scrubbed() { return; }`, and their module is listed in `DISCOVERING_SUITES`.
- Global config or environment variables go only to a re-executed child: `testing::reexec(&child_path("..._child"), &env, &scratch.root, b"")`, child marked `#[ignore = "child half of ...; runs only when re-executed"]`, results read with `testing::marker`, never the exit status alone.
- No `std::env::set_var`/`remove_var` (`clippy.toml` 24-25).

### Every read has a differential and a control
**Source:** `src/git/reads.rs` `tree_diff_matches_git` (655-690) and `tree_diff_changes_with_the_pair` (692-724):
```rust
        let from_gix = repo
            .tree_diff(
                &id(&scratch, &repo_dir, "main"),
                &id(&scratch, &repo_dir, "feat"),
            )
            .expect("gix diff main feat");
        let from_git = git_tree_diff(&scratch, &repo_dir, "main", "feat");
        assert_eq!(
            from_gix, from_git,
            "diff main feat: gix against git diff-tree --no-renames"
        );
```
and the control, a different input that must give a different answer:
```rust
        assert_ne!(
            moved, main_feat,
            "diff main feat~3 {moved:?} against main feat {main_feat:?}"
        );
```
**Apply to:** `local_config_values` (vs `git config --local --get-all`), `remote_configured` (vs `git remote add` exit 3), `find_ref` (vs `git show-ref --verify`), `refs_directly_under` (vs `git for-each-ref`), `root_entry` (vs `git ls-tree` / `git cat-file -e`), `c_quote`/`c_unquote` (vs `git ls-files` / `-z`).

### Doc comments carry the measurement
**Source:** `src/git/repo.rs` `ref_storage` doc (160-164); `refuse_at_ceiling` doc with a measured table (173-200); `implicit_worktree`/`apply_implicit_worktree` doc (444-457).
**Apply to:** each new read's doc states what git was measured to do and which gix default it avoids (for example, why `level == 0`, why the trailing `/` on `prefixed`).

## No Analog Found

| File / sub-pattern | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/model/mod.rs` `From<git::Error> for model::Error` | error conversion | transform | No error type in the crate wraps another; use RESEARCH Pattern 8 |
| `src/model/mod.rs` `resolve`/`build`/chain | policy service | transform | No policy layer exists yet; use RESEARCH Pattern 8 and Code Examples ("Remotes and the chain") |
| `src/model/manifest.rs` proptest blocks | test | property | No proptest in the crate; use RESEARCH Code Examples ("The proptest") and Pitfall 17 |
| `src/git/refs.rs` `refs_directly_under` | git read | iteration over refs | No ref listing exists in `src/git/`; use RESEARCH Pattern 4 |

## Metadata

**Analog search scope:** `src/` (all of `src/git/*.rs`, `src/main.rs`), `tests/harness/{fixture,outcome}.rs`, `Cargo.toml`, `clippy.toml`, `scripts/workspace/cut-pr-branch.sh`.
**Files scanned:** 12 read in part or whole; line numbers verified against the files as read on 2026-11-04.
**Not verified here:** nothing was compiled or run. Corrections 7-9 come from reading source; correction 7 in particular predicts a test failure that has not been observed.
**Plan cross-references:** 04-01, 04-03, 04-04 and 04-08 cite "PATTERNS line N" (lines 14, 15, 97, 148-154, 257) from the previous version of this file. Those line numbers do not carry over; the content they named is under correction 8 (no `Git` in model tests), the `-z` parsing pattern, and the `refs.rs` peel section above.
