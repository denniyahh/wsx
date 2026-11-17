## CONFIRMED

### 1. Critical — worktree-qualified names bypass the symlink check

**File:** `.planning/phases/04-repo-model/04-13-PLAN.md:160–164`

> “`<dir>/<full>` … a symlink is `SymlinkedRef { path }`.”

That is not necessarily the path gix opens. `gix-ref-0.69.1/src/store/file/find.rs:270` strips `main-worktree/` before opening the ref. The proposed check retains it.

**Reproduce:** With commits `A` containing `only .planning` and `B` containing `only b`, configure `workspace/dev` and run in the main worktree:

```bash
git update-ref refs/heads/target "$A"
git update-ref refs/heads/refs/heads/target "$B"
git -c core.preferSymlinkRefs=true symbolic-ref \
  refs/heads/keep refs/heads/target
git symbolic-ref refs/heads/workspace/dev main-worktree/refs/heads/keep
```

Resolve from a linked worktree with an empty private `refs/`.

**Checked:** A scratch implementation of the proposed presence checks passed. The unchanged model returned `Configured` with `only b`; Git exited 128, reporting a dangling source symref. The check examined `<common>/main-worktree/refs/heads/keep`; gix opened `<common>/refs/heads/keep`.

**Controls:** Using `refs/heads/keep` directly made the proposed symlink check reject it. Converting that direct chain to text symrefs made both readers select `A`. Separately, a regular-file target behind the `main-worktree/` alias still let gix resolve a source Git rejected.

**What breaks:** CONF-07 remains violated inside `build`. Correcting the symlink pathname alone also leaves the ordinary text-alias discrepancy.

### 2. Major — gix reads symlinked `HEAD` before either proposed placement

**Files:** `.planning/phases/04-repo-model/04-13-PLAN.md:183–190`, `:604`

> “every exact ref the model path reads goes through it”

> “No symlinked ref wsx reads yields a model”

Gix reads `HEAD` outside `find_ref`. `gix-0.90.0/src/open/repository.rs:302–309` reads it while opening the repository and uses its symbolic target to evaluate conditional config.

**Reproduce:** In a main worktree on `main`, define `origin` only through an `includeIf.onbranch:main.path` file, provide `refs/remotes/origin/main`, then run:

```bash
git -c core.preferSymlinkRefs=true symbolic-ref HEAD refs/heads/main
```

Keep the workspace source ref ordinary.

**Checked:** Git still loaded the conditional remote. With textual `HEAD`, wsx returned both remotes as `origin` and selected the remote integration ref. With symlinked `HEAD`, wsx returned both remotes as `None` and selected the local integration ref. Restoring textual `HEAD` restored the original result.

In a linked worktree, a Git-created symlinked `HEAD` instead made `Repo::discover` fail before the new guards. `gix-discover-0.57.1/src/is.rs:66` follows the filesystem link when testing whether `HEAD` exists; its repository-relative target normally has no corresponding private filesystem path.

**What breaks:** A normal Git-supported symlink workflow either silently changes model configuration or fails discovery. Guarding only wsx’s later `find_ref` calls cannot establish the claimed boundary.

### 3. Major — the candidate-parity prohibition remains false for text symrefs

**File:** `.planning/phases/04-repo-model/04-13-PLAN.md:40`

> “MUST NOT list a workspace-branch candidate git does not have”

**Reproduce:** With one valid `workspace/dev` candidate and no `wsx.sourceBranch`:

```bash
git symbolic-ref refs/heads/workspace/ghost refs/heads/absent
git for-each-ref --format='%(refname)' 'refs/heads/workspace/*'
```

**Checked:** Git listed only `workspace/dev`. `refs_directly_under` listed both names, and `resolve` returned `NotConfigured` with `WorkspaceCandidates`, preventing inference. Both proposed presence checks accept this layout: it contains ordinary files and no private stray.

**Control:** Removing only `ghost` left Git’s listing unchanged and restored successful inference in wsx.

`src/git/refs.rs:265–266` explicitly documents this surviving discrepancy:

> “A dangling symbolic ref is listed, where `git for-each-ref` omits it.”

**What breaks:** The plan declares candidate parity resolved while retaining a known counterexample. D-16’s state distinction survives; D-21 inference parity does not.

## SUSPECTED

### 4. Major — the new symlink check has an unaddressed check/open race

**File:** `.planning/phases/04-repo-model/04-13-PLAN.md:157–164`

> “`std::fs::symlink_metadata(path)`”

> “before gix’s lookup”

The metadata check and gix’s `File::open` at `gix-ref-0.69.1/src/store/file/find.rs:338` are separate operations.

**Reproduction hypothesis:** Replace an accepted regular ref atomically with the demonstrated redirecting symlink after the metadata check and before gix opens it.

**Evidence checked:** The specified check and actual open do not share a file descriptor or verify filesystem identity. I did not execute a controlled concurrent swap.

**What could break:** The symlink’s commit could reach peeling despite the presence check. T-04-50 describes the earlier replacement-map race, not this new check/open window.

## COVERAGE

- Read the target plan, gap review, CONTEXT, CLAUDE.md, and both round-1 reviews and their summary in full; inspected relevant requirements and preceding-plan/summary sections.
- Inspected wsx lookup, traversal, candidate, configuration and discovery paths, plus relevant gix-ref, gix and gix-discover sources and installed Git worktree documentation.
- Runtime probes used Git 2.56.1 and unchanged snapshot modules linked against gix 0.90.0; proposed presence checks were implemented separately in scratch. Also checked directory symlinks, packed per-worktree records, permission failures and FIFO targets.
- Did not execute the proposed RED/GREEN tests, full suite, mutation, coverage or container gates; did not runtime-test bare repositories or every normal worktree operation.
- Snapshot paths and file hashes were unchanged. Created scratch files were removed; probe subprocesses were reaped.
