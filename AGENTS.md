# AGENTS.md (workspace branch only)

Rules for agents working on `workspace/denniyahh`. This file never reaches develop: the PR cut
strips it. Repo rules every contributor needs stay in `CLAUDE.md`.

## PR-cut traps

Shapes that make a commit the PR cut cannot replay.

- Add new ignore lines at the top of `.gitignore`: the workspace branch appends its own block at
  the end, and a line beside it conflicts when the PR cut replays the commit.
- Scripts that must reach develop never go under `scripts/workspace/`, which is workspace-only
  and stripped from PRs.
- Commit subjects carry the phase scope, `type(NN-MM): ...`, because the PR cut replays only
  phase-scoped commits.
