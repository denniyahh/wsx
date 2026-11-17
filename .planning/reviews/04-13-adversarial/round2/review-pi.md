## CONFIRMED

### 1. major — the `git_dir` half of Rule B is never exercised, and cannot change any outcome except which message appears; T-04-51 and the artifact list credit it anyway

**`04-13-PLAN.md:160-162`**
> "`fn refuse_symlinked_ref(&self, full: &[u8]) -> Result<(), Error>` (private): for `dir` in `[self.common_dir(), self.git_dir()]` (the two stores gix reads loose refs from; the same directory twice in a main worktree)"

**`04-13-PLAN.md:580`** (T-04-51)
> "`refuse_symlinked_ref` refuses a symlink at `<store>/<full>` in either store before gix's lookup … pinned by `symlinked_source_branch_fails_closed` … `symlinked_ref_on_the_way_to_the_tip_fails_closed`, their RED logs and the deletion check on that call"

**Evidence checked.** All three symlink fixtures write into `work.join(".git/…")` where `work` is the *main* worktree (`04-13-PLAN.md:472-477`), so `git_dir == common_dir` and the second loop iteration is the same path; **no symlink test runs from a linked worktree at all**. gix sends `LocalBranch`, `RemoteBranch` and `Tag` lookups to the common dir only (`gix-ref-0.69.1/src/store/file/find.rs:264-286`, `to_base_dir_and_relative_name`), and every name wsx ever passes to `find_ref` is `refs/heads/<branch>` or `refs/remotes/<remote>/…` (`src/model/mod.rs:391`, `:557-597`; `src/git/refs.rs:166-176`). So in a linked worktree the `<git_dir>/<full>` lstat can only fire when `<git_dir>/refs/heads/…` exists, which `refuse_linked_worktree_refs` refuses in the same `build` call (`04-13-PLAN.md:143-151`); the second store only changes the error the user sees (`SymlinkedRef` instead of `LinkedWorktreeRefsPresent`). Likewise `refuse_symlinked_entries`'s `git_dir` half is dead behind Rule A's stray rule.

**Reproduce.** `git init -b main r; …; git worktree add --detach ../linked`; plant `<linked>/.git/worktrees/linked/refs/heads/workspace/dev` as a symlink (or the whole `heads` dir); call `resolve`. The answer is `Error::Git(LinkedWorktreeRefsPresent { path: …/refs/heads })`, never `SymlinkedRef`. The Task 3 control mutant `is_symlink_at -> bool with true` (`04-13-PLAN.md:530`) fires on the common store, so it cannot distinguish the stores either.

**What breaks.** Nothing at runtime. The plan's Tier A evidence does not cover the arm its threat row and artifact list ("the symlink checks … in either store") claim, Task 3's coverage block has no threshold (`04-13-PLAN.md:533-541`), and the untested arm is a boundary check in a fail-closed guard — the exact "a real check sitting beside the question" failure the plan's own RED/control machinery exists to prevent. Either drop the second store from the rule and its docs, or add one linked-worktree fixture whose only symlink is in the private store.

---

### 2. minor — `refuse_symlinked_entries` introduces an error kind that is not a ref-store refusal, is unreachable in practice, and is passed through `require` unchanged, contradicting the plan's own routing truth

**`04-13-PLAN.md:167-168`**
> "an entry that fails to read is `Error::Io { what: "list <dir>", source }`; otherwise the least entry name in byte order whose path `is_symlink_at` accepts"

**`04-13-PLAN.md:33`**
> "Every refusal `build` and `require` route apart from manifest failures is one of the six variants `git::Error::is_ref_store_refusal` lists"

**Evidence checked.** `Io` exists (`src/git/mod.rs:137-140`), the six-variant predicate excludes it (`src/git/mod.rs:169-177`), and `require`'s `NotConfigured` arm ends `other => other` (`src/model/mod.rs:124-129`). `refs_directly_under` is the only feed of `candidates` (`src/model/mod.rs:395`), which is the only producer of `inferred`. On Linux the arm is unreachable: an unreadable directory fails `read_dir` itself, which the plan maps to `Ok(None)` — I verified gix then errors (`ERR Could not traverse reference directory` with `<common>/refs/heads/workspace` at mode 000), so the fail-closed path is `Error::Gix`, not `Io`. Only a `DirEntry` iteration failure reaches it. No `<behavior>` case produces it.

**What breaks.** If it is ever reached from a repo with no `wsx.sourceBranch`, the user is shown `list <dir>: <os error>` instead of D-16's `Run wsx init.`, and `require` returns `Err(Git(Io))` from a repo `resolve` called not configured. Otherwise it is an uncovered line in a new boundary check that no gate can fail on. Both outcomes violate `04-13-PLAN.md:33` as written.

---

### 3. minor — the "six controls" / "three controls" acceptance is false; the `held` lists contain three and one real controls

**`04-13-PLAN.md:435`**
> "it records that the six controls passed at RED"

**`04-13-PLAN.md:588-589`**
> "its eight RED tests have failed alone … and its six controls passed" / "its three RED tests have failed alone … and its three controls passed"

**Evidence checked.** Task 1's `held` list (`04-13-PLAN.md:418`) is: `common_replace_ref_from_a_linked_worktree_fails_closed`, `per_worktree_refs_in_a_linked_worktree_resolve`, `linked_worktree_without_a_refs_dir_resolves` (controls) **plus** `git::tests::linked_worktree_refs_message_names_the_entry_and_the_fix`, `git::tests::unreadable_linked_worktree_refs_message_names_the_directory`, `git::tests::ref_store_refusals_are_recognised` — three new/changed tests that are added and pass at the RED commit, not opposite cases. Task 2's `held` (`04-13-PLAN.md:492`) is `textual_symbolic_source_branch_resolves` plus two new tests, i.e. one control, not three.

**Reproduce.** Read the two `held` invocations and the two acceptance lines; nothing in either block computes a control count, so the SUMMARY's number comes from the prose.

**What breaks.** The plan's falsifiability unit is "a control that must pass" (`04-13-PLAN.md:588-589`, `<verification>`). Recording six where three exist is the same class of error the round-1 review raised against count-based gates, and it will be quoted into the SUMMARY as evidence.

---

### 4. minor — a fifo loose ref is neither refused nor listed, and the guard hangs where the plan's premise says it refuses before reading; git hangs identically, so this is parity, not a divergence

**`04-13-PLAN.md:480`**
> "add to its doc that a symlinked loose ref is refused as `Error::SymlinkedRef` before it is read"

**`04-13-PLAN.md:66`**
> "`find_ref`, the one exact ref read (`ref_commit` and every integration rung read through it), runs `self.refuse_symlinked_ref(full)?;` right after its name check, before gix's lookup"

**Evidence checked (git 2.56.1, gix 0.90.0, scratch repo).** With `.git/refs/heads/workspace/dev` a fifo: `timeout 5 git rev-parse --verify -q refs/heads/workspace/dev` → exit 124 (hang); a gix probe calling `find` on the same name hung until `timeout 10`; `git for-each-ref` exit 0 listing only `refs/heads/main`, and gix's `prefixed` listing skipped it. So a configured `wsx.sourceBranch = workspace/dev` with a fifo ref makes `build`'s `find_ref` block forever: `is_symlink_at` returns false (a fifo is not a symlink) and gix's `File::open` (`gix-ref-0.69.1/src/store/file/find.rs:338`) blocks. Nothing in Rule B covers it; `04-13-PLAN.md:151`'s §6 asks about it and the plan answers only for `refs/`.

**Reproduce.** `mkfifo .git/refs/heads/workspace/dev; timeout 5 git rev-parse refs/heads/workspace/dev` (124), then the same read through `Repo::find_ref`.

**What breaks.** No parity break (git hangs too), which is why I do not call it major. But "refuses by presence before any object read" is false for this shape, a hang in a guard is not a fail-closed refusal, and `CLAUDE.md`'s "a regression proof must show a real assertion failure, not a hang" means any future test asserting this shape will hang the suite rather than fail it. Say in `find_ref`'s doc that a fifo blocks as git's own read does, or refuse non-regular loose refs by `file_type` (that also closes the `.lock`, directory and device-node shapes).

---

## SUSPECTED

### 5. major if true — `Error::Io`'s absence from the predicate may also mis-route a *configured* repo's listing failure, which nothing tests

**`04-13-PLAN.md:167-168`**, as in finding 2. I could not construct a reachable `DirEntry` failure on this host, so I cannot show the message. What I did verify is the *reachable* half: an unreadable prefix directory gives `Error::Gix` (fail-closed, git silently drops), so the divergence I can prove is in the safe direction. Whether `require` receiving `Git(Io)` for a not-configured repo would ever reach a user is unverified; the path exists in the code (`src/model/mod.rs:124-129`).

### 6. minor — Rule A's exemption is "a real directory", so a symlinked *namespace directory* is refused; I could not show that refusal is wrong, but the plan's over-refusal list does not name the shape

**`04-13-PLAN.md:147-150`**
> "`is_per_worktree_ref_dir(path: &Path) -> bool` is true exactly when the path's file name bytes are one of `PER_WORKTREE_REF_DIRS` … and `std::fs::symlink_metadata(path)` says a directory, so a regular file, a symlink or a fifo of one of those names is a stray"

I planted `<private>/refs/bisect` as a symlink to a directory holding a valid ref: git's `rev-parse --verify refs/bisect/x` from the linked worktree printed nothing, and gix listed nothing, so both readers agree and refusing is defensible. What I could not verify is whether that agreement holds for a symlinked `refs/worktree` or `refs/rewritten`, or under `core.symlinks=false`-style layouts; the plan's over-refusal paragraph (`04-13-PLAN.md:224-229`) names "a symlinked ref" but not "a symlinked namespace directory", so the doc Task 1 mandates will be incomplete relative to the code.

### 7. minor — the round-1 backlog item pi-suspected-3 (the unreadable arm is only ever ENOTDIR) is still unfixed, and now also covers the new variant

**`04-13-PLAN.md:144-146`** requires the message "wsx failed to read `<refs>`, so it cannot tell whether refs git ignores are stored there". The only fixture (`unreadable_linked_worktree_refs_blame_the_refs`, `04-13-PLAN.md:404`) makes `<git_dir>/refs` a fifo, i.e. `ENOTDIR`, where nothing is *stored* there and the sentence's reason is vacuous. Round 1 recorded this as backlog; the revision adds a second variant with the same reason text and no `EACCES` fixture. The plan's §6 asks about "unreadable" refs/; it answers only for "not a directory".

### 8. minor — `T-04-48`'s "git wrote nothing else there" is measured on 18 operations but not on the one that mutates the private store from another worktree

**`04-13-PLAN.md:398-404`** lists the measured operations. I re-ran 18 (`bisect start`/`reset`, `rebase -r`, `rebase --abort`, `stash`, `stash pop`, `cherry-pick`, `pack-refs --all`, `gc`, `update-ref` ×2, `maintenance run`, `switch -c`, `notes add`, `tag`, `replace`, `reflog expire`, `worktree lock`) and confirm only `refs/bisect/`, `refs/rewritten/`, `refs/worktree/` appear; `git stash` writes the common `refs/stash`. What I could not verify is the cross-worktree case the design actually fears: `git worktree repair`, `git worktree move`, or a second `git worktree add` with the same id while wsx is running — none is measured, and `read_dir` between worktree add and its first git command returns a `refs/` directory that is empty (fine) but `worktree add` interrupted mid-way leaves what I did not establish.

---

## COVERAGE

**Read in full:** `04-13-PLAN.md` (all 615 lines), `04-REVIEW.md`, `04-CONTEXT.md`'s decision block (D-12…D-22), `round1/SUMMARY.md`, `round1/review-pi.md`, `round1/review-codex.md`, `CLAUDE.md`, `.config/nextest.toml`, `scripts/check.sh` (usage, selector guard, `run_fmt`/`run_clippy`/`run_test`/`run_deps`/`run_coverage`), `scripts/install-tools.sh` head. **Source:** `src/git/mod.rs:1-40,100-200,270-340`; `src/git/refs.rs:1-20,46-58,85-200,207-420`; `src/git/repo.rs:60-150`; `src/model/mod.rs:90-135,320-420,450-520,555-600,1175-1200,1875-1935,4370-4400,4825-4890,5030-5060,5340-5400,5650-5710`; `src/main.rs`; `src/git/testing.rs:640-700`. **gix:** `gix-ref-0.69.1/src/store/file/{find.rs (1-360), packed.rs (1-80), overlay_iter.rs (380-420), loose/iter.rs (1-120)}`, `gix-fs-0.24.x/src/walkdir.rs` head.

**Probes run (all disposable, in `/tmp`, nothing in the tree edited):** git 2.56.1 and a gix 0.90.0 scratch binary built offline from the same registry sources, in new `mktemp` repos:
- **Premises reproduced:** a private `refs/heads/workspace/ghost` is listed by gix from the linked worktree while `git for-each-ref` and `git rev-parse` miss it; a private `refs/replace/<blob>` enters gix's replacement map (`prefixed("refs/replace/")` printed it). Both round-1 findings are real at this tip.
- **Rule A's scope:** linked worktrees of a normal repo and of a bare repo both give `git_dir != common_dir` under gix 0.90.0 (`…/worktrees/<id>` vs `…/.git`), a main worktree and a bare repo give equality, so step 1 is correctly scoped. 18 git operations wrote nothing into the private `refs/` but the three namespaces; `git stash` wrote the common `refs/stash`; a planted private `packed-refs` is ignored by git and never read by gix (`packed.rs:50` uses `common_dir_resolved()`), and a controlled `git pack-refs --all` from a linked worktree created none.
- **Rule B's scope:** git and gix *agree* on a symlinked `refs` directory (both list through it), on a symlinked `refs/heads/workspace` prefix directory (both list through it), and on a dangling symlink ref (both omit it from a listing); they *disagree* only on the final component, which is exactly what Rule B checks (git reads `dev` as a symref to `A`, gix reads the link target `B` — verified: git `only .planning`, gix the `B` object). gix's alternative lookup locations produce a name mismatch (`find refs/heads/workspace/dev` returned `name=heads/refs/heads/workspace/dev` for a planted `<common>/heads/refs/...`), so `find_ref`'s name check keeps them out. An unreadable prefix directory gives gix `ERR Could not traverse reference directory` (git silently skips), so the `read_dir`-error arm's "defers to gix's own error" is true.
- **Plan mechanics:** the four deletion-check `sed` ranges remove exactly one line each on copies of the real `refs.rs` (`root_entry`'s live call plus synthetic `find_ref`, `refs_directly_under`, `refuse_replace_refs` insertions); cargo-mutants 27.1.0 emits exactly `replace is_per_worktree_ref_dir -> bool with false`, `replace match guard err.kind() == std::io::ErrorKind::NotFound with false in stray_private_ref`, `replace is_symlink_at -> bool with true`, so Task 3's `-F` selection is sound, and it reuses an existing `-o` directory (so Task 3 step 4's re-run works); the STATE awk guard works on this `STATE.md` (no `###` subheading inside `Blockers/Concerns`, ends at `## Roadmap Evolution`, `is_replacement_refusal` occurs once, `merge_base and the rest, from phase 3` once, `Phase 12: MIG-03` present); 04-12's coverage rows are recorded as `git/refs.rs`, `git/mod.rs`, `model/mod.rs`, so Task 3's row regex matches.

**Not read / not verified:** `04-12-PLAN.md` and `04-12-SUMMARY.md` in full (grep only, for the mutation-pass counts and coverage rows); `04-REQUIREMENTS.md`; `04-SECURITY.md`; most of `src/model/mod.rs`'s test module beyond the helpers and cited tests; `bin/wsx`; `gix-0.90.0/src/open/repository.rs` (relied on round 1's citation).

**Could not run:** the crate itself and every `scripts/check.sh` command. This snapshot has **no `.git`** and no `.target`/`.tools/bin`, and `scripts/check.sh test|coverage` hard-fails at `scripts/install-tools.sh verify-cargo cargo-nextest` (host 0.9.152 vs the pin 0.9.148) before any test runs, while `check.sh fmt` needs a work tree. So every RED/GREEN text, the `scripts/check.sh all` exit, the five deletion checks against the real functions, coverage, the mutation pass and the container gate are **unverified by me**; and in a tree without `.git`, Task 1 block 5's `test -z "$(git status --porcelain -- src)"` would pass on git's error and then `sed` the file with no `git checkout` to restore it. I therefore attacked the two rules and the plan's measurements by direct git/gix probes instead, and could not break either rule at the wsx surface: the residual holes I can name are findings 1-4 above, and the acknowledged ones (`peel_commit`, `reads.rs`, `siblings()`) are already transferred to `T-04-53` and the STATE lines.
