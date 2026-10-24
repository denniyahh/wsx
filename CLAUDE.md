# CLAUDE.md

Agent guidance for the **wsx** repository, deliberately minimal. `README.md` explains the tool;
`CONTRIBUTING.md` holds the workflow, the design constraints and the AI change acceptance bar.

The Python 0.1.0 (`bin/wsx`, `tests/sandbox.sh`) is frozen: read it for reference only and do
not change it. It is deleted at cutover.

## Shape of the code

- One package at the repository root: binary crate `wsx`, edition 2024, no library crate.
- `rust-toolchain.toml` pins rustc. Tool versions live only in `scripts/install-tools.sh`.

## Canonical commands

- `scripts/check.sh all`: fmt, clippy, test. The default target and the gate for every change.
- `scripts/check.sh fmt|clippy|test|deps`: one gate; `deps` (cargo-deny) is not part of `all`.
- `scripts/check.sh test -E 'test(=name)'`: one named test (`--no-tests=fail` is always set).
- `scripts/check-in-container.sh [target]`: the same gate in the pinned image CI uses.
- `scripts/assert-image-parity.sh`: every workflow image matches the devcontainer pin.
- `scripts/setup-repo.sh [all|policy|check]`: once per clone, and after `.gitconfig` changes;
  registers the gitleaks pre-commit hook and copies the tracked `.gitconfig` into local config.

## Repo traps

Shapes that ship a false green, or a commit the PR cut cannot replay.

- A nextest filter that matches nothing must fail. Gate one test with
  `--no-tests=fail -E 'test(=module::name)'`, one test per command chained with `&&`:
  `-E 'test(=a) | test(=b)'` skips a wrong name and still passes.
- `rg -c` and `--count` print nothing, not `0`, on zero matches. Check absence with a Rust test
  or `rg -q` exit codes, not a shell count.
- zsh has no `PIPESTATUS` (its array is `pipestatus`). Capture the exit code of the command that
  matters, not the pipeline tail, and run gate commands under bash.
- Prefer a named Rust test to any shell pipeline; keep shell to the invocation.
- `std::env::set_var` and `remove_var` are banned by `clippy.toml`. Scope environment to the
  child with `Command::env`.
- Never run `cargo init` or `cargo new` here, and add new ignore lines at the top of
  `.gitignore`: the workspace branch appends its own block at the end, and a line beside it
  conflicts when the PR cut replays the commit.
- Scripts that must reach develop never go under `scripts/workspace/`, which is workspace-only
  and stripped from PRs.
- Commit subjects carry the phase scope, `type(NN-MM): ...`, because the PR cut replays only
  phase-scoped commits.
- Never `include.path` the tracked `.gitconfig`: git would read it live from any checked-out
  branch. `setup-repo.sh` copies an allowlist of keys; a new policy key goes in its
  `POLICY_KEYS` first.
- The container mounts the checkout read-only and builds into a volume over `target/`, so a
  check that needs to write must use a scratch directory.

## Do not

- Add a dependency without a justification and a clean `scripts/check.sh deps`.
- Make the tool interactive: it never prompts.
- Resolve a conflict on the user's behalf.
- Push, open PRs or change repository settings from an agent session.
