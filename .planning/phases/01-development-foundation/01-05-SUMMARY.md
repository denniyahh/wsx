---
phase: 01-development-foundation
plan: 05
subsystem: infra
tags: [git, gitleaks, pre-commit, config-hooks, ssh-signing, git-flow, supply-chain, bash]

requires:
  - phase: 01-01
    provides: "scripts/check.sh all (commit gate); shared bash script skeleton"
  - phase: 01-02
    provides: "scripts/install-tools.sh install gitleaks (named in setup-repo.sh's missing-scanner error); pinned gitleaks 8.30.1"
provides:
  - ".gitconfig: tracked, public-safe repo policy with exactly ten keys (gpg.format=ssh, commit/tag.gpgsign=true, gitflow.branch.{master=main,develop=develop}, gitflow.prefix.{feature,release,hotfix,support,versiontag})"
  - "scripts/setup-repo.sh: idempotent, local-config-only; registers hook.gitleaks-staged (command `gitleaks git --pre-commit --staged --redact`, event pre-commit) and include.path ../.gitconfig; exit 1 on git < 2.54, missing gitleaks or a hook git does not list; exit 2 on any argument"
  - "This repository's .git/config now carries the gitleaks-staged hook and the include (local, untracked)"
affects: [01-06 CONTRIBUTING.md and CLAUDE.md should document scripts/setup-repo.sh as the one-time clone setup, 01-07 phase verification of DEV-05/SUP-03/SUP-05]

actuals:
  tokens: 1487
  tasks: 2
  commits: 2
plan_head_before: b18c4379beefa5cd7edd13fd2d4444cff27c5e71
plan_head_after: ef90a6944039fee7ebf5e9f0dc8def7621ada9c6

tech-stack:
  added: []
  patterns:
    - "Repo policy that must survive a clone lives in tracked .gitconfig, pulled in by include.path; per-person values (signing key) stay in local config"
    - "Repo hooks are git 2.54+ config hooks (hook.<friendly-name>.command/.event), added beside core.hooksPath and global hooks, never replacing them"
    - "A setup script confirms its result through git's own view (`git hook list`), not by re-reading the config it just wrote"
    - "Multi-valued config keys (include.path) are read into a variable and tested, never piped into grep -q under pipefail (SIGPIPE would read as 'absent' and duplicate the value)"

key-files:
  created:
    - .gitconfig
    - scripts/setup-repo.sh
  modified: []

key-decisions:
  - "setup-repo.sh treats `git config --get-all include.path` exit 1 as 'unset' and any other non-zero status as an error, and reads it into a variable instead of piping to grep -q, so a SIGPIPE cannot cause a duplicate include"
  - "setup-repo.sh compares `git rev-parse --is-inside-work-tree` output to `true` rather than trusting its exit status, which is 0 inside .git/ too"
  - "setup-repo.sh rejects any argument with exit 2, matching the shared script skeleton (the plan specifies no arguments)"
  - "C6 isolates global config with a scratch file that sets commit.gpgsign=false and tag.gpgsign=false instead of CONTEXT's GIT_CONFIG_GLOBAL=/dev/null: still isolated from the host, and it also proves the include wins over a contrary global setting"

patterns-established:
  - "Secret-scan controls isolate global config (GIT_CONFIG_GLOBAL=<empty scratch file>, GIT_CONFIG_NOSYSTEM=1) and pair the refusal with a hook-disabled counterpart, so the refusal is attributed to the repo hook"

requirements-completed: [DEV-05, SUP-03, SUP-05]

coverage:
  - id: D1
    description: "Tracked .gitconfig holds exactly the ten policy keys and no personal data (raw text, comments included)"
    requirement: DEV-05
    verification:
      - kind: other
        ref: "Task 1 verify #1: `git config -f .gitconfig --list` sorted equals the ten expected keys (baseline: file absent, exit 128)"
        status: pass
      - kind: other
        ref: "Task 1 verify #2: personal-data pattern matches all five positive samples and does not match .gitconfig; extra negative control: a copy with an email in a comment trips it"
        status: pass
    human_judgment: false
  - id: D2
    description: "The include signs commits and annotated tags over a contrary global setting, sets git-flow names byte-exactly, and signing stops when the include is removed; with no signing key the commit is refused"
    requirement: DEV-05
    verification:
      - kind: integration
        ref: "Task 1 verify #3 (C6): signature lines 1/1 with the include, 0/0 without; gitflow.branch.master `m a i n`, develop `d e v e l o p` (od -c)"
        status: pass
      - kind: integration
        ref: "Task 1 verify #4 (no-key edge): commit exit 128, `fatal: either user.signingkey or gpg.ssh.defaultKeyCommand needs to be configured`, no HEAD"
        status: pass
    human_judgment: false
  - id: D3
    description: "The repo's own config hook refuses a staged secret with global config emptied, passes a clean commit, and is the hook doing the refusing"
    requirement: SUP-05
    verification:
      - kind: integration
        ref: "Task 2 verify #1 (C4): planted runtime token -> exit 1, `WRN leaks found: 1`, HEAD unchanged; same commit with hook.gitleaks-staged.enabled=false -> exit 0; clean commit -> `INF no leaks found`"
        status: pass
    human_judgment: false
  - id: D4
    description: "setup-repo.sh coexists with a core.hooksPath hook, is idempotent and order-preserving, and refuses old git or a missing scanner before writing config"
    requirement: SUP-03
    verification:
      - kind: integration
        ref: "Task 2 verify #2 (coexistence): `git hook list pre-commit` = gitleaks-staged + `hook from hookdir`; hooksPath marker `ran`; commit with gitleaks off PATH refused"
        status: pass
      - kind: integration
        ref: "Task 2 verify #3 (idempotency): two runs -> include.path `../other.inc` then `../.gitconfig` once; one event value; command string exact. Extra negative control: a mutant without the add-if-absent guard fails this check (`../.gitconfig` twice)"
        status: pass
      - kind: integration
        ref: "Task 2 verify #4 (refusals): reported git 2.53 -> exit 1 naming 2.54; PATH=/usr/bin:/bin -> exit 1 naming gitleaks; no hook config written. Extra boundary: 2.54.0 and 3.0.0 accepted, 2.9.5 refused"
        status: pass
    human_judgment: false
  - id: D5
    description: "This repository's real config carries the hook and the include, and a real commit passed through the new hook"
    requirement: SUP-03
    verification:
      - kind: other
        ref: "Task 2 verify #5: `git hook list pre-commit` lists gitleaks-staged (phase worktree and main checkout); include.path ../.gitconfig exactly once; push protection `enabled`"
        status: pass
      - kind: other
        ref: "commit ef90a69 output: two gitleaks scans (repo hook + global hooksPath hook), both `INF no leaks found`"
        status: pass
    human_judgment: false

duration: 5min
completed: 2026-10-22
status: complete
---

# Phase 1 Plan 05: Tracked git policy and the repo's own gitleaks hook Summary

**Public-safe `.gitconfig` (SSH signing for commits and tags, git-flow names) plus `scripts/setup-repo.sh`, which registers a git 2.54+ config hook `gitleaks-staged` beside any global hook; both proven in isolated scratch repos and applied to this repository's real `.git/config`.**

## Performance

- **Duration:** about 5 min
- **Started:** 2026-10-22T19:52:49Z
- **Completed:** 2026-10-22T19:57:30Z
- **Tasks:** 2
- **Files created:** 2 (plus a local, untracked change to `.git/config`)

## Needs your awareness

1. **Every checkout and worktree of this repository now runs the repo hook, in addition to your global one.** The change is in the shared `.git/config`, so the main checkout (`workspace/denniyahh`) lists it too (checked). Each commit now runs gitleaks twice, once per hook.
2. **A commit made where `gitleaks` is not on PATH is now refused.** Your global hook skips with a warning in that case; the repo hook does not skip. Shown in scratch with `PATH=/usr/bin:/bin` (`gitleaks: command not found`, commit refused). A GUI git client that does not inherit the Homebrew PATH would hit this. This fail-closed behaviour is intended.
3. **The include does nothing yet on this machine.** `../.gitconfig` resolves against the common git dir, so it reads the **main checkout's** file, and `workspace/denniyahh` has no `.gitconfig` (checked). It takes effect once the file reaches that branch. At that point it changes nothing about signing for you, because your global config already sets the same three signing keys. The git-flow names would be new.
4. **Exactly what changed in `.git/config`** (diff against a copy taken just before the run; nothing else changed, and global config was not touched, as its unchanged mtime and the absence of any `hook.*`/`include.*` key in it show):
   ```
   [hook "gitleaks-staged"]
   	command = gitleaks git --pre-commit --staged --redact
   	event = pre-commit
   [include]
   	path = ../.gitconfig
   ```
   **To undo:**
   ```
   git config --local --remove-section hook.gitleaks-staged
   git config --local --unset include.path '^\.\./\.gitconfig$'
   ```

## Accomplishments

- `.gitconfig` with exactly ten keys, no `user.*`, key path, email or home path, comments included.
- C6: with the include, commits and annotated tags are signed even though global config sets gpgsign to false. Without the include, they are not signed. With no signing key, the commit is refused.
- `scripts/setup-repo.sh` (mode 100755, shellcheck and `bash -n` clean): registers the hook, adds the include only if it is absent, refuses before writing on old git or missing gitleaks, and confirms the result with `git hook list`.
- C4: the repo hook alone refuses a planted secret, and disabling only that hook lets the same commit through.
- Applied to this repository. The commit of the script itself passed through the new hook.

## Evidence (quoted output; the planted token was generated at runtime and is scrubbed)

**Task 1, key list** (baseline before the task: `fatal: unable to read config file '.gitconfig'`, exit 128):
```
gpg.format=ssh  commit.gpgsign=true  tag.gpgsign=true  gitflow.branch.master=main
gitflow.branch.develop=develop  gitflow.prefix.feature=feature/  gitflow.prefix.release=release/
gitflow.prefix.hotfix=hotfix/  gitflow.prefix.support=support/  gitflow.prefix.versiontag=v
```
**Task 1, personal-data check.** It runs on the raw text, so it covers the header comments, which the key-list check never sees. All five positive samples match (home path, `~/` path, `.ssh/` path, email address, `ssh-ed25519 AAAA` key material) and `.gitconfig` does not. An extra negative control appended a comment containing an email address to a scratch copy, and the check flagged it.

**C6** (global config is a scratch file setting `commit.gpgsign=false` and `tag.gpgsign=false`; the throwaway ed25519 key had comment `wsx-control` and was deleted with its scratch directory):
```
with include:    commit sig lines=1 tag sig lines=1
master bytes:    m   a   i   n  \n        (trailing \n is od's view of git's output line)
develop bytes:   d   e   v   e   l   o   p  \n
without include: commit sig lines=0 tag sig lines=0
C6 rc=0
```
**No-key edge** (empty global config, include present):
```
commit rc=128
fatal: either user.signingkey or gpg.ssh.defaultKeyCommand needs to be configured
```

**C4** (global config is an empty file):
```
clean commit:   INF no leaks found  -> [master (root-commit) ...] clean
planted token:  WRN leaks found: 1  -> rc=1, HEAD unchanged     (token: ghp_<scrubbed>)
same commit with -c hook.gitleaks-staged.enabled=false -> exit 0
```

**Coexistence** (`core.hooksPath` holds a marker-writing pre-commit hook):
```
$ git hook list pre-commit
gitleaks-staged
hook from hookdir
marker: ran
PATH=/usr/bin:/bin commit -> "gitleaks git --pre-commit --staged --redact: line 1: gitleaks: command not found", refused
```

**Idempotency and ordering** (repo started with `include.path ../other.inc`, script run twice):
```
include.path: ../other.inc, ../.gitconfig      hook.gitleaks-staged.event: pre-commit
```

**Refusals** (no hook config written in either case):
```
reported git 2.53.0 -> rc=1  error: git 2.53 is older than 2.54, which ignores hook.<name> config and would leave commits unscanned
PATH=/usr/bin:/bin  -> rc=1  error: gitleaks is required by the pre-commit hook; run scripts/install-tools.sh install gitleaks
```

**Real repository** (Task 2 verify #5 exit 0): `git hook list pre-commit` prints `gitleaks-staged` and `hook from hookdir` in both the phase worktree and the main checkout. `include.path ../.gitconfig` appears once. Push protection is `enabled`. The commit of the script (`ef90a69`) ran gitleaks twice, both `INF no leaks found`.

## What these checks do not establish

- C4 proves the hook refuses one well-known token shape (a GitHub classic PAT). It does not test which secrets gitleaks' rules cover.
- The signing proof uses a throwaway key. It shows that the policy turns signing on, not that any particular contributor's key is valid.
- On the real repository, registration was checked, and one clean commit was seen passing through the hook. No refusal was triggered there, by design: no planted secret is allowed outside scratch.
- `--no-verify` skips the hook. Server-side push protection (`enabled`, checked) is the only backstop for that.

## Task Commits

1. **Task 1: Tracked .gitconfig, proven to sign and to carry nothing personal (C6)**, `01b3984` (feat)
2. **Task 2: scripts/setup-repo.sh registers the repo's own gitleaks hook (C4), then applied to this repo**, `ef90a69` (feat)

## Files Created/Modified

- `.gitconfig`: tracked repo policy (signing, git-flow names) and a header saying how to include it and what must never go in it.
- `scripts/setup-repo.sh`: one-time, idempotent local setup for the gitleaks config hook and the include.

## Decisions Made

See `key-decisions` in the frontmatter. All four were implementation choices within the task's remit: three hardening points in `setup-repo.sh` and the stronger C6 isolation the plan itself specified.

## Deviations from Plan

**1. [Procedural] Scratch cleanup without `trap 'rm -rf "$S"' EXIT`**
- **Found during:** Tasks 1 and 2 (every scratch control).
- **Issue:** the owner warned that the harness may block `rm -rf "$VAR"`, and asked for deletion by explicit full path.
- **Fix:** each control prints `S=<path>`, and the directory is removed afterwards by that literal path. The checks themselves ran verbatim. Afterwards, no `c6.*`, `nokey.*`, `c4.*`, `coexist.*`, `idem.*`, `refuse.*` (or the extra `neg.*`/`dis.*`) directory remained.
- **Side note:** in the first C6 run I added echo lines joined with `;`, which broke the `&&` chain's exit code. The plan's uninstrumented chain was rerun and exited 0.

**2. [Added verification] Extra negative controls beyond the plan**
- A mutant `setup-repo.sh` without the add-if-absent guard fails the idempotency check, so the check can fail.
- The version gate accepts 2.54.0 and 3.0.0 and refuses 2.9.5 (boundary).
- With `hook.gitleaks-staged.enabled=false` already in local config, `git hook list` prints `disabled<TAB>gitleaks-staged`, and setup-repo.sh exits 1 instead of reporting a registered hook. That is fail-closed, though its message says "not visible" rather than "disabled".

**Total deviations:** 1 procedural, plus added controls. **Impact:** none on the deliverables.

## Issues Encountered

None.

## User Setup Required

None. The setup already ran on this repository.

## Next Phase Readiness

- 01-06 (CONTRIBUTING.md/CLAUDE.md rewrite) should tell contributors to run `scripts/setup-repo.sh` once per clone, after `scripts/install-tools.sh install gitleaks`.
- No blockers.

---
*Phase: 01-development-foundation*
*Completed: 2026-10-22*

## Self-Check: PASSED

Files: .gitconfig, scripts/setup-repo.sh, 01-05-SUMMARY.md present. Commits 01b3984 and ef90a69 are ancestors of HEAD. Evidence token scan `ghp_[A-Za-z0-9]{36}` over the phase directory exits 1, and the same pattern matches a runtime-generated 36-character sample (exit 0). No scratch control directory remains. The real repo reports the locked hook command.
