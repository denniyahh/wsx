# Contributing to wsx

## Status

wsx is being rewritten in Rust. The Python 0.1.0 (`bin/wsx`, `tests/sandbox.sh`)
is frozen and is deleted at cutover: read it for reference if you need to, but
do not change it.

## Getting started

Do these once per clone, in order. Linux on x86_64 and git 2.54 or newer are
required.

1. Install [rustup](https://rustup.rs). It installs the compiler pinned by
   `rust-toolchain.toml` the first time you run `cargo` in the repository.
2. Install the pinned tools the gates and the commit hook use:

   ```bash
   scripts/install-tools.sh install cargo-nextest cargo-deny gitleaks
   ```

   Each download is checked against a pinned sha256 before anything is
   extracted. The binaries go into `$WSX_TOOLS_DIR`, or cargo's `bin`
   directory by default, which must be on your `PATH`. If you already have
   them, `scripts/install-tools.sh verify cargo-nextest cargo-deny gitleaks`
   fails unless each one is the pinned version. The versions are pinned in
   that script and nowhere else.
3. Run the one-time repository setup:

   ```bash
   scripts/setup-repo.sh
   ```

   It registers the repository's own gitleaks pre-commit hook, which runs
   beside any global hook you already have, and includes the tracked
   `.gitconfig` (see [Repository git policy](#repository-git-policy)). It
   refuses to run, before changing anything, on git older than 2.54 or when
   `gitleaks` is not on `PATH`. From then on a commit made where `gitleaks` is
   not on `PATH` is refused rather than committed unscanned.
4. Set your own signing key. The included policy signs every commit and tag
   with SSH, so a commit is refused until a key is configured:

   ```bash
   git config --local user.signingkey <your SSH public key or its path>
   ```

5. Confirm the gates pass: `scripts/check.sh all`.

## Repository git policy

Git cannot version `.git/config`, so settings that must survive a fresh clone
live in the tracked [`.gitconfig`](.gitconfig) and are pulled in once, either
by `scripts/setup-repo.sh` or by hand:

```bash
git config --local include.path ../.gitconfig
```

The path is relative to the common git directory (`.git/`), so linked
worktrees read the main checkout's copy of `.gitconfig`, not their own. This
turns on SSH-format signing for both commits and tags, and configures the
git-flow branch model. It deliberately does **not** set `user.signingkey` —
that is per-contributor and this repository is public.

Because the model is supplied by that include, **do not run `git flow init -d`**:
its default production branch is `master` and this repository's is `main`, so
the defaults would misconfigure git-flow against a branch that does not exist,
surfacing only later at release or hotfix time. With the include in place,
`git flow feature start <name>` works directly.

## Development commands

`scripts/check.sh` is the single definition of a green change. CI runs the
same script, so a clean local run means the same thing as a clean CI run.

| Command | Runs |
|---|---|
| `scripts/check.sh` or `scripts/check.sh all` | fmt, then clippy, then test (the default) |
| `scripts/check.sh fmt` | `cargo fmt --check` |
| `scripts/check.sh clippy` | clippy over all targets with `-D warnings` |
| `scripts/check.sh test [-E <filterset> \| name]` | the nextest suite; `--no-tests=fail` is always set, at most one filterset or test name is accepted, and no other nextest option |
| `scripts/check.sh deps` | `cargo deny` advisories, licenses, bans and sources; not part of `all` |

Run one test by its full name with `scripts/check.sh test -E 'test(=name)'`.
Run one test per command: a filter that combines names with `|` passes even
when one of the names matches nothing.

`scripts/check-in-container.sh [target]` runs the same target inside the
pinned devcontainer image that CI uses. It needs a reachable Docker daemon and
works from the main checkout and from linked worktrees. The source and git
directories are mounted read-only; build output goes to per-checkout Docker
volumes named `wsx-ci-target-*` and `wsx-ci-registry-*`, with the target
volume mounted over `target/`. Remove them with `docker volume rm` when you no
longer need the cache, and always after changing where the target volume is
mounted: a volume built under another layout keeps stale binary paths and
fails the tests.

`scripts/assert-image-parity.sh` fails unless every workflow job runs in the
image pinned by `.devcontainer/Dockerfile`.

See [AI change acceptance](#ai-change-acceptance) for what a test must
demonstrate to be accepted, and which test shapes are rejected outright.

## CI and required checks

The CI workflow runs three jobs, `Test`, `Clippy` and `Format`, each in the
pinned image and each through `scripts/check.sh`. All three are required
checks on `develop` and `main`, so renaming or removing one blocks every merge
until the branch rulesets change with it.

The cargo-deny workflow is advisory: it goes red on any finding so the finding
is seen, but it is never a required check, because a newly published advisory
can turn it red on a change that touched nothing.

A Dependabot PR that bumps the image in `.devcontainer/Dockerfile` must update
every workflow image in the same PR; the parity check in `Test` fails it until
they match. Scheduled scans and Dependabot run from `main` only, so a change
to either takes effect once it is released there.

## Design constraints

These are the rules the tool is built around. Breaking one is a bug, even if
the tests still pass.

1. **History changes go through the `git` binary.** Merge, cherry-pick,
   branch and worktree creation, commit and fetch all run `git`, so the user's
   hooks, signing, merge drivers and credentials apply. gix only reads.
2. **Never write shared branches, never push.** wsx commits only on the
   workspace branch, on branches it creates, and on PR branches it cuts.
3. **Never prompt.** wsx runs inside hooks and scripts.
4. **Linux and git 2.54 or newer** for milestone 1.
5. **Every dependency is justified,** and `scripts/check.sh deps` stays clean.
6. **Tests never mutate the process environment.** `clippy.toml` bans
   `std::env::set_var` and `remove_var`; scope environment to the child
   process with `Command::env` instead.

## AI change acceptance

Effectively every change to this repository is authored or substantially
modified by an AI agent — that is the default, not a special case — so the
acceptance bar for a change is written down here rather than assumed.

A change is accepted only when it demonstrates all five of the following:

1. A regression test that fails before the change.
2. At least one assertion at a public or otherwise stable boundary, not only
   a private helper.
3. Evidence the test fails for the intended reason — not a setup error, a
   compile failure, or an unrelated panic.
4. `scripts/check.sh all` clean (formatting, clippy over all targets, and the
   full test suite), plus `scripts/check.sh deps` clean when dependencies
   change.
5. Independent review of both the implementation and the test signal — a
   correct-looking implementation reviewed alone is exactly how a test that
   cannot fail gets through.

A test is rejected outright, regardless of whether it currently passes, if it
does any of the following:

1. Asserts only constants, never an implementation's actual output.
2. Reproduces the production algorithm inside the test body, so the test and
   the code fail together and pass together.
3. Compares a function call with itself.
4. Greps implementation text as a substitute for a runtime contract that
   could have been asserted directly.

Reviewers apply this bar to every change.

## Supply-chain scanning

- **Secrets at commit time.** The repository's own pre-commit hook, registered
  by `scripts/setup-repo.sh`, runs gitleaks over the staged changes. GitHub
  push protection is the backstop for a commit made with `--no-verify`.
- **Dependencies.** `scripts/check.sh deps` runs cargo-deny locally, and the
  advisory cargo-deny workflow runs it on every push, on pull requests to
  `develop` and `main`, and weekly.
- **CI dependencies.** Dependabot opens grouped weekly updates for the
  workflow actions and the devcontainer image.

The gitleaks hook and cargo-deny were each shown to refuse a planted problem,
a fake secret and a dependency with a known advisory, before either counted as
enabled.

## Style

- Comments explain *why*, never *what*. If a line needs a "what" comment, it
  probably needs a better name instead.
- Every non-obvious constraint gets a comment naming the failure it prevents —
  the stdin redirect and the symlink-parent materialisation are the model here.
- User-facing output is aligned, coloured only when stdout is a TTY, and says
  what to do next when it reports a problem.

## Commits

Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`).
The body should explain the failure mode being fixed, not restate the diff.
Subjects carry the phase scope, `type(NN-MM): ...`, so the PR cut replays them.
