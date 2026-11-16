## CONFIRMED

**1. Non-default bases can fail before the promised refusal, producing a false repository diagnosis.**

`.planning/phases/04-repo-model/04-12-PLAN.md:106–111` promises `ReplaceBaseSet`; line 162 claims replacement loading cannot make discovery fail. However, `src/git/repo.rs:33` opens gix first, and lines 64–67 map its failure to `NotARepo`.

On a configured scratch repo whose manifest contained `only a\n`:

```text
GIT_REPLACE_REF_BASE=/tmp/replace/ git cat-file blob refs/heads/workspace/dev:.wsx.conf
→ only a                         [exit 0]

GIT_REPLACE_REF_BASE=/tmp/replace/ wsx_probe "$r"
→ WSX ERROR: not a git repository: <r>
  (A RelativePath is not allowed to be absolute)
```

`wsx_probe` directly called the snapshot’s unmodified `Repo::discover`. Setting `gitoxide.objects.replaceRefBase=/tmp/replace/` produced the same error. Neither message identifies the setting or supplies the planned remedy.

Control: `refs/other/` opened successfully and reached the modeled guard, which returned `BASE`; the default base returned `ADMIT`. The plan changes no discovery code, so its proposed guard cannot intercept these failures.

**2. A FIFO `packed-refs` blocks instead of yielding a replacement refusal.**

Plan lines 119–123 prescribe unrestricted `std::fs::read`; line 277 tests only a directory. A FIFO without a writer exposes another outcome:

```bash
mkfifo "$r/.git/packed-refs"
timeout 2 wsx_probe "$r"
# exit 124; no output

timeout 2 read_probe "$r/.git/packed-refs"
# exit 124; no output
```

`read_probe` called exactly `std::fs::read(path)`. Thus discovery already blocks, and the proposed read would also block if reached. The external two-second timeout fired; neither operation returned a refusal.

Control: replacing only the FIFO with an empty regular file made discovery succeed, the modeled guard return `ADMIT`, and `read_probe` print `Ok([])`. The plan needs a bounded rejection path for this file kind.

## SUSPECTED

**The early-guard placement lacks a dedicated failing-direction check.** Plan lines 249–268 primarily assert eventual refusal; line 289 retains the guard inside `root_entry`. Those assertions may remain green if the new `build` call is omitted, allowing earlier object reads. I did not execute that mutant. A workspace commit replaced by a blob, tested with and without the early call, would distinguish replacement refusal from the existing `.wsx.conf` attribution.

## Scope

No new admitted byte divergence was reproduced for loose, nested, packed, linked-worktree, or custom-base replacements. Active loose and packed controls tripped the modeled guard. Reftable admitted through that guard but was rejected by existing discovery.

The planned disabled-replacement wording remained true in the checked cases; no remaining `.wsx.conf` attribution path was identified for the three planned refusal variants. Shell blocks passed syntax checks, and cited source symbols were spot-checked; mutation and container gates were not executed.

All scratch directories were deleted, probe processes finished or were reaped, and the snapshot’s file-content fingerprint remained unchanged.


