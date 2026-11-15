# Research brief: one set of hooks across coding-agent harnesses

Written 2026-11-15 from a DevFlow session, at the operator's request, for a separate session started in this
repository. This file is a brief, not a finding: nothing in it has been built, and the claims under "Starting
points" are leads to check, not facts.

Suggested start: `/gsd-spike` with this file as its input, so the work is recorded under `.planning/spikes/`.

## The question

The operator runs several coding agents (Claude Code, Codex, Gemini CLI, agy, Pi) and relies on hooks to block
bad commands, format files and inject reminders. Today each harness is wired separately and most have nothing.

1. Can one definition of a hook run under every harness, with blocking that actually blocks?
2. Does anything that exists do this reliably enough to adopt?
3. If not, what would the feature be in wsx?

## Decided: wsx owns this

The operator decided on 2026-11-15 that agent hooks are wsx's, not devflow's. `PROJECT.md` (What This Is, Key
Decisions) and `REQUIREMENTS.md` (Boundary amendment, IF-02, STAT-02) record it. Do not reopen the question.

What the research still has to settle is the size of the exception. IF-02 says wsx reads no agent-runtime files and
is enforced by a source-scan test; a hooks feature has to read and write harness hook configuration. Propose the
narrowest carve-out that works: which files wsx may touch, in which commands, and how the source-scan test and the
"repo with no agent files" scenario change so that everything outside the carve-out stays forbidden.

## Hard rules for this research

Phase 52.1 of DevFlow is executing on this machine while this research runs. Its executors run under the live
harness configuration, and the phase tests timing-sensitive flakes.

- Write nothing under `~/.claude`, `~/.codex`, `~/.gemini` or `~/.pi`. Read them freely.
- Test every harness through a throwaway configuration directory under the session scratch directory. Find each
  harness's documented way to point at one (an environment variable or a flag) and prove it took effect before
  trusting any result: a marker hook that fires in the sandbox and does not appear in the real config.
- No long builds or heavy parallel work. Install candidate tools into the scratch directory only (`cargo install
  --root`, `npm --prefix`, `GOBIN`), never globally.
- Never suggest `dnf`: the host is immutable Fedora Kinoite.
- Do not send the operator's name, email, repository names or file contents to any external service.
- Stage 2 (changing any real configuration, or writing wsx code) is not part of this brief. It needs the
  operator's word.

## What to produce

One report, under `.planning/spikes/` or `.planning/research/`, in this order.

### 1. Capability matrix, from official documentation and the installed versions

For Claude Code, Codex, Gemini CLI, agy and Pi, one row each:

- hook events that exist, and which fire before a tool runs, after it, at session start and at stop;
- whether a pre-tool hook can block, and how a block is signalled (exit code, JSON on stdout, both);
- how the payload arrives (stdin JSON, environment, arguments) and the field names for the tool name and the
  command or file path;
- where hooks are configured, per user and per project;
- timeout units and defaults;
- the installed version it was checked against, and the documentation URL.

Mark every cell as read from documentation, observed in a run, or unknown. Do not fill a cell from a blog post.

### 2. Captured payloads

In each sandboxed harness, register a hook that writes its stdin, arguments and relevant environment to a file,
then trigger one shell command and one file edit. Keep the captures: they are the test fixtures for whatever comes
next. Where a harness cannot be run headless or has no hook, say so and how it was established.

### 3. Each candidate, tested, not described

Candidates found by one web search on 2026-11-15, none evaluated:

- `agent-hooks`, a Rust crate, 0.1.0 at the time
- `agenthooks`, Go, by Speakeasy, 0.1.0 at the time
- `ai-hooks`, npm
- `hawk-hooks`
- hand-written adapters around shared scripts (the baseline to beat)

Search again for others before starting. For each: install it in the sandbox, register one guard, and run two
cases per harness it claims to support:

- must block: a command the guard refuses. Assert the harness did not run it (a marker file was not created), and
  that the refusal text reached the agent. An exit code alone is not proof.
- must pass: an ordinary command. Assert it ran.

Also record: last release date, open issue count, whether it has tests for each harness, licence, and what it
writes into the harness configuration (and whether it can be removed cleanly).

A candidate that fails the must-block case on any harness the operator uses fails, whatever else it does.

### 4. Verdict

One of: adopt a named candidate; write thin adapters; build it. State the evidence against the verdict near the
top. If the verdict is to build, outline the feature for wsx: the IF-02 carve-out, the commands,
the normalised payload, the per-harness installers, and what `wsx doctor` (or its equivalent) would verify.

## Starting points, unverified

Observed on this machine on 2026-11-15:

- Claude Code 2.1.325: hooks in `~/.claude/settings.json`. Payload is JSON on stdin (`tool_input.command`,
  `tool_input.file_path`); exit code 2 with text on stderr blocks; a pre-tool hook can also add context through
  `hookSpecificOutput.additionalContext`. Two environment variables that older examples use
  (`CLAUDE_TOOL_INPUT`, `CLAUDE_FILE_PATHS`) are not set: two hooks written against them had never worked.
- Codex CLI 0.169.0: `~/.codex/hooks.json` exists with `PreToolUse` (matcher `Bash`) and `SessionStart` entries, in
  the same shape as Claude Code's, and `hooks = true` in `config.toml`. Payload fields and blocking semantics were
  not checked.
- Pi: TypeScript extensions under `~/.pi/agent/extensions/`. Whether an extension can veto a tool call was not
  checked.
- Gemini CLI: `~/.gemini/settings.json` has `enableHooks` and no hooks defined.
- agy: its settings file has no hooks key. Whether it supports hooks at all was not checked.

From web write-ups, not confirmed: the three largest harnesses pass similar JSON on stdin with different field
names (`tool_input` in Codex, `toolInput` in Gemini), different event names and timeout units, and different block
signals; one estimate puts a normaliser at under twenty lines. Sources disagree on whether Codex has real hooks;
the local `hooks.json` suggests it does.

The shared scripts that any solution has to carry, all plain Python reading JSON on stdin:

- `~/.claude/hooks/bash-command-guard.py`: blocks a filtered `cargo test`, `rm -rf /`, `dd` onto a device, `mkfs`,
  a fork bomb and `git push --force`; advises on `rg -c` and `gh run list --branch`.
- `~/.claude/hooks/format-on-edit.py`: formats the file just written.
- `~/.claude/hooks/harness-retro-due.py`: a session-start note.

mise 2026.11.8 is installed. Its own hooks fire on directory and tool-install events, not on agent tool calls, so
it cannot be the unifier; mise tasks could be the single entry point each adapter calls, and would pin tool
versions.

## Out of scope

Git hooks. wsx already manages those through `wsx hook <name>`, and they run whatever agent made the change.
Note in the report which of the operator's checks could move there and stop needing an agent hook at all.
