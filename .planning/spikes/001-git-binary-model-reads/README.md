---
spike: 001
idea: model-inputs-via-git
name: git-binary-model-reads
type: standard
validates: "Given every shape from 04-13's two review rounds, when the model's inputs are read with four batched git children, then each answer equals the git command a person would run, a control that changes git's answer changes the batch's, and resolve stays within D-26's budget on the four R3 repos"
verdict: VALIDATED
related: [002]
tags: [git-binary, refs, config, latency, phase-04]
---

# Spike 001: git-binary model reads

## What This Validates
D-23/D-26/D-27 and the D-01 open check (04-CONTEXT.md): the batched reads

1. `git --version` (the spawn helper's version check)
2. `git config -z --show-scope --show-origin --get-regexp '^(wsx|remote)\.'`
3. `git for-each-ref --format='%(refname) %(objectname) %(objecttype) %(symref)' refs/heads/workspace/ refs/heads/personal/ refs/heads/<branch> refs/remotes/<base>/{develop,next,HEAD}`
4. `git cat-file --batch` fed `<tip>:.wsx.conf`

give the answer of the reference command (`git show`, `git branch --list`, `git remote`, `git remote add`,
`git config --local --get-all`, `git rev-parse --verify`).

## How to Run
```bash
NODE=<real node binary, not a mise shim> bash parity.sh <scratch-dir>      # 37 checks
bash bench-git.sh 200 ~/Github/devflow ~/Github/wsx ~/Github/gsd-core ~/Github/gsd-pi
bash bench.sh 100 <repos...>     # whole-prototype timing + child count by PATH wrapper
```
git 2.56.1. Fixtures run with HOME, XDG_CONFIG_HOME and GIT_CONFIG_GLOBAL in scratch and GIT_CONFIG_NOSYSTEM=1.

## Investigation Trail
- First run: the config cases read nothing. Cause: `node` is a mise shim and fails once HOME moves; the script now
  takes NODE as a real binary and exits 2 otherwise.
- S3 first failed: a git-style symlinked ref (target `refs/heads/other`, the deprecated `core.preferSymlinkRefs`
  format) is resolved by `rev-parse` and `show-ref --verify` but hidden by `for-each-ref`, `branch --list` and
  `show-ref`. That is a split inside git, not a batch error; the case now asserts the batch sides with the listings.
- S8 first showed no hang for `for-each-ref`; probing further, only reads by name hang on a fifo
  (`rev-parse`, `cat-file --batch-check`), listings skip it.
- Whole-prototype timing (bench.sh) came out at 36-45 ms p50 for 4 children and about the same for the 13-child
  control, with one child at 4 ms: the shell's own sed/awk/cut forks dominated. bench-git.sh times the git
  children alone; there the control is about 3x slower, so that measurement responds to child count.

## Results
**Parity: 37/37 pass, every control moved.**

| Shape | Reference | Batch | Note |
|---|---|---|---|
| S1 replace ref; `GIT_NO_REPLACE_OBJECTS`; `core.useReplaceRefs=false` | `git show` | cat-file | cat-file applies and drops replacements exactly as git show does |
| S1b replace ref only in `.git/worktrees/X/refs/replace` | git ignores it | ignores it | today's detection scans only the common dir (see 002: gix applies it) |
| S2 branch only in a worktree-private `refs/heads` | `branch --list` hides it | hidden | |
| S3 relative symlinked ref | resolves | resolves | |
| S3 git-style (`refs/…`) symlinked ref | **split**: rev-parse resolves, listings hide | hidden | batch reads "missing" |
| S4 `main-worktree/` alias symref, from a linked worktree | rev-parse fails | absent | control (plain symref) agrees on the tip |
| S5 symlinked HEAD + `includeIf.onbranch` remote | `git remote` lists origin | lists origin | control (HEAD elsewhere) moves both |
| S6 dangling symref under `refs/heads/workspace/` | `branch --list` omits | omits | |
| S7 dangling symbolic integration rung | rev-parse: absent | absent | **behaviour change**: today `find_ref` reports Symbolic -> `IntegrationRefSymbolic`; via git it is absent and the chain moves on. No D-01..D-21 test pins the dangling case (only `dangling_remote_head_is_not_a_default`) |
| S8 fifo loose ref | listings skip it; by-name reads hang (killed at 5 s) | for-each-ref exit 0 | push with a pre-push hook that lists candidates: exit 0, no hang |
| C1 D-01: include.path, config.worktree, `GIT_CONFIG_PARAMETERS` | `config --local` skips all three | same (filter: scope local and origin = common dir's config) | matches today's read, already pinned by `included_key_is_not_read`, `worktree_config_key_is_not_read`, `command_line_config_key_is_not_read`; also right from a linked worktree |
| C2 D-18: bare, empty, repeated keys | | distinguishable in `-z` output | bare key prints no newline |
| C3 remote defined (global-only, keyless local, included) | `git remote add` exit | same set | |

**Child count and latency (D-26).** 4 children on the configured path (3 when the candidate count is not 1),
counted by the prototype and independently by a PATH wrapper that logs each `git` exec; the per-ref control
counts 12-13 both ways. Git children only, 200 runs each, load average about 5:

| Repo | batched (4) p50 / p95 | per-ref control (13) p50 / p95 |
|---|---|---|
| devflow | 20.0 / 40.7 ms | 56.5 / 138.4 ms |
| wsx | 15.2 / 26.7 ms | 42.3 / 74.2 ms |
| gsd-core | 22.0 / 38.8 ms | 46.8 / 79.3 ms |
| gsd-pi | 15.4 / 23.0 ms | 41.6 / 67.4 ms |

These are bash-launched children; Rust's spawn and parsing cost is not measured (expected smaller than bash's,
unverified). None of the four repos has `wsx.*` config or a `.wsx.conf`; devflow and wsx read their real
`.workspace-divergence` blob in its place, so the blob read cost is real there.

## Signal for the replan
- **The S3/S8 trade-off is one choice.** Reading the workspace branch and rungs through `for-each-ref` never hangs on
  a fifo and hides a legacy symlinked ref. For the workspace branch that is a loud `WorkspaceBranchMissing`; for an
  integration rung it silently falls through to the next rung while `git rev-parse origin/develop` resolves.
  Reading by name (`cat-file --batch-check`, `rev-parse`) resolves the symlink but hangs on a fifo.
- **S7 changes a behaviour no test pins**: a dangling symbolic rung stops being an error. The replan says which
  answer is intended and adds the test.
- **The child budget has no slack.** In a hook aimed at another worktree, the spawn helper adds
  `rev-parse --local-env-vars`, a 5th child (read from `src/git/spawn.rs`, not measured here).
