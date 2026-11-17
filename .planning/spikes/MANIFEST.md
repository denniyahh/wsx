# Spike Manifest

## Ideas

### model-inputs-via-git
Phase 04's model reads refs, listings, the `.wsx.conf` blob and `wsx.*`/remote config through the git binary
(D-23), with gix kept for object work by id. The spikes check that the batched reads equal git's own answer on every
shape from 04-13's two review rounds, the D-26 child and latency budget, and what gix still does differently by id.

**Requirements:**
- The read set is four batched children: version check, one config read, one for-each-ref, one cat-file --batch (D-26).
- Every shape check compares against the command a person runs, with a control that moves git's answer.

## Spikes

| # | Idea | Name | Type | Validates | Verdict | Tags |
|---|------|------|------|-----------|---------|------|
| 001 | model-inputs-via-git | git-binary-model-reads | standard | batched git reads equal git's answer on every review shape; 4 children, p95 under 100 ms on the R3 repos | VALIDATED | git-binary, refs, config, latency |
| 002 | model-inputs-via-git | gix-object-reads | standard | gix object reads by id match git or are caught by D-22 | PARTIAL | gix, replace-refs, merge-base |
