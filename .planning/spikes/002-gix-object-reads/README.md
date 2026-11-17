---
spike: 002
idea: model-inputs-via-git
name: gix-object-reads
type: standard
validates: "Given replace refs, a linked worktree's private store, a symlinked HEAD with conditional config, grafts and a shallow file, when gix 0.90 (opened as wsx opens it) reads objects by id, then it returns what git returns, or the difference is caught by D-22's detection"
verdict: PARTIAL
related: [001]
tags: [gix, replace-refs, merge-base, phase-04, d-24]
---

# Spike 002: gix object reads by id

## What This Validates
D-24 and two open checks: does gix honour `core.useReplaceRefs` and `GIT_NO_REPLACE_OBJECTS`; does today's
detection miss a replace ref either reader applies, including a linked worktree's own `refs/replace/` (CR-01);
does gix reading HEAD at open still change object reads. Added: grafts and shallow for merge-base, since gix keeps
object work by id.

## How to Run
```bash
CARGO_TARGET_DIR=<scratch>/probe-target cargo build --offline --manifest-path probe/Cargo.toml
bash gix-cases.sh <scratch-dir> <scratch>/probe-target/debug/gix-object-probe
```
`probe/` pins gix =0.90.0 with wsx's features and copies `open_gix` from `src/git/repo.rs`.

## Results
13 pass, 4 differ. Each difference has a control that agrees.

| Case | git | gix | Meaning |
|---|---|---|---|
| B1 replace ref; `GIT_NO_REPLACE_OBJECTS`; `core.useReplaceRefs=false` local and global | | same | gix honours both switches |
| **B2 loose replace ref only in `.git/worktrees/X/refs/replace`, read from that worktree** | ignored | **applied** | **CR-01 confirmed: a gap.** `refuse_replace_refs` reads only the common dir, so a configured repo opened from that worktree gets replaced objects with no refusal (gap read from the code; the refusal itself was not run) |
| B2 same, read from the main worktree | ignored | ignored | |
| B2b replace ref in the private dir's `packed-refs` | ignored | ignored | |
| **B3 symlinked HEAD, `includeIf.onbranch` include sets `core.useReplaceRefs=false`** | off | **applied** | HEAD at open still changes object reads, through the replacement switch; text HEAD and HEAD elsewhere agree |
| **B4 `info/grafts` making a root** | merge-base none | **finds a base** | gix ignores grafts |
| **B4b `shallow` boundary** | none | **finds a base** | gix merge_base ignores the shallow file (objects past the boundary were present here; a real shallow clone, where they are missing, is untested) |

## Signal for the replan
- **D-22 detection needs one more store:** entries in `<git_dir>/refs/replace` when the git dir is a linked
  worktree's. That closes B2, and with it B3, since B3 only bites while a replace ref exists. Whether any other
  config gix takes from an `onbranch` include changes object bytes was not tested (guess: no).
- **Later phases (merge-base, walks):** gix diverges from git on grafts and shallow. That is outside phase 4's
  inputs, but it sits under "gix keeps object work by id" (D-23), so it belongs in the STATE obligations.
