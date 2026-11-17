# Agent hooks: findings from wiring shared hooks across harnesses (2026-11-17)

Input: `AGENT-HOOKS-UNIFICATION-BRIEF.md`. This is not the brief's report. It records what a Claude Code session
in this repository built and measured on 2026-11-17, at the operator's request, while making the operator's own
hooks run in Codex, agy, Hermes and pi. It follows the brief's section order so the spike can reuse it.

**Evidence against using this as the brief's answer, first:**

- The brief's hard rules (no writes under `~/.claude`, `~/.codex`, `~/.gemini`, `~/.pi`; test only in a sandbox
  config dir) were **not** followed. The operator asked for live global wiring, so the real configs were changed.
  This is Stage 2 work done by hand, ahead of the research.
- No harness was driven end to end with a real model turn. "Observed" below means the harness's own hook runner
  (`hermes hooks test`) or the adapter run directly with a synthetic payload, never a captured payload from a live
  session. Section 2 of the brief is therefore still open.
- None of the brief's candidates (`agent-hooks`, `agenthooks`, `ai-hooks`, `hawk-hooks`) was evaluated. What was
  built is the brief's baseline, hand-written adapters, so section 3 has its baseline and nothing to compare it to.
- Gemini CLI was not covered: `gemini` is not on PATH on this machine.

Source key: **src** read from the installed binary, source or bundled docs; **doc** official docs via ctx7;
**obs** observed in a run (named); **unk** not established.

## 1. Capability matrix

| | Codex CLI 0.169.0 | agy 1.5.10 | Hermes 0.23.2 | pi 1.5.2 |
|---|---|---|---|---|
| Global config | `~/.codex/hooks.json`, Claude-shaped `{"hooks":{Event:[{matcher,hooks:[{type,command,timeout}]}]}}` (src, doc) | `~/.gemini/config/hooks.json`, named top-level hooks `{"<name>":{Event:[...]}}`; a top-level `hooks` key is rejected (src) | `hooks:` in `~/.hermes/config.yaml`, `{event:[{matcher,command,timeout,fail_closed}]}` (src, obs: `hermes hooks list`) | TS/JS extension auto-loaded from `~/.pi/agent/extensions/` (src) |
| Consent gate | Each entry needs a trusted hash in `config.toml` `[hooks.state."<file>:<event>:<group>:<index>"]`, keyed by position (src) | none found (unk) | first use per (event, command) needs approval, stored in `~/.hermes/shell-hooks-allowlist.json`; without a TTY an unapproved hook is skipped, not prompted (src) | none (src) |
| Session start | `SessionStart` (src) | `SessionStart`, flat handler list, no matcher wrapper (src) | no usable event: `on_session_start` ignores output; use `pre_llm_call` once per `session_id` (src) | `session_start` (`event.reason`: startup/reload/new/resume/fork) (src) |
| Before tool / after tool | `PreToolUse` / `PostToolUse` (src) | `PreToolUse` / `PostToolUse`; also `PreInvocation`, `PostInvocation`, `Stop`; no `BeforeTool`/`AfterTool` (src) | `pre_tool_call` / `post_tool_call` (src) | `tool_call` / `tool_result` (src) |
| Shell tool name | `Bash` (src) | `run_command` (matcher is a regex on the lowercased step type) (src) | `terminal` (src) | `bash` (also `powershell`) (src) |
| Edit tool name | `apply_patch`; matcher aliases `Write`, `Edit` (src, doc) | **unk** (`code_action` or `file_change` guessed from binary strings) | `write_file`, `patch` (src) | `edit`, `write` (src) |
| Payload | stdin JSON, `cwd`, `tool_name`, `tool_input.command` (src) | stdin JSON camelCase: `cwd`, `workspacePaths[]`, `toolCall.{name,args}`, shell text at `toolCall.args.CommandLine`; SessionStart documents `workspacePaths`, not `cwd` (src) | stdin JSON `{hook_event_name, tool_name, tool_input, session_id, cwd, extra}`; `tool_input` is the tool's args; file path at `tool_input.path` (src, obs) | handler args: `event.input.command`, `event.input.path`, `ctx.cwd` (src) |
| Edited file path | not a field: parse `*** Add File:` / `*** Update File:` / `*** Move to:` lines out of the patch in `tool_input.command` (doc; **unk** in a live run) | **unk** | `tool_input.path`; `patch` in patch mode carries it in the patch text (src) | `event.input.path`, relative to `ctx.cwd` (src) |
| Block a shell command | exit 2 + stderr, or JSON `decision:block` / `permissionDecision:deny` (src, doc) | stdout JSON `{"decision":"deny","reason":...}`; exit-code semantics undocumented (src) | exit 2 + stderr (first 400 chars), or `{"action":"block","message":...}`; only `pre_tool_call` can block (src, obs) | return `{block:true, reason}` (src; whether `reason` reaches the model: **unk**) |
| Note into context | `hookSpecificOutput.additionalContext` on SessionStart **and** PreToolUse (schema in binary), or plain stdout on SessionStart (src) | SessionStart stdout `{"injectSteps":[{"systemMessage":{"systemMessage":...}}]}`; none known for PreToolUse (src) | `pre_llm_call` stdout `{"context":...}`, appended to the user message (src, obs) | return `{message:{customType,content,display}}` from `before_agent_start` (src; delivery to the model **unk**) |
| Timeout | `timeout`, seconds, default unk (src) | `timeout`, seconds, default 30 (src) | `timeout`, seconds, default 60, clamp 1–300 (src) | no field for extensions (src) |
| Headless check without a model turn | none found | none found (`agy` has no hooks subcommand) | `hermes hooks list|test|doctor` (src, obs) | none; extension logic testable under Node with a stand-in `pi` (obs) |

Claude Code 2.1.326 behaves as the brief's starting points say (exit 2 + stderr; `additionalContext`).

## 2. Payloads

Not captured from live sessions. The one agy probe (a temporary log-everything hook plus `agy -p ...
--dangerously-skip-permissions` in a scratch dir) was refused by the session's permission classifier and did not
run. Settling agy's edit tool name and arguments needs that probe run by the operator, or a sandboxed config dir
as the brief asks.

## 3. Baseline built: one shared copy plus a thin adapter

- Shared scripts moved to `~/.config/agents/hooks/` (bash-command-guard, format-on-edit, graphify-auto-refresh,
  harness-retro-due, learnings-due); `~/.claude/hooks/<name>` are symlinks to them, so Claude Code's registrations
  did not change.
- `agent-hook-adapter.py <codex|agy|hermes|pi> <pre-shell|post-shell|post-edit|session> <hook argv...>` (~230
  lines, stdlib only) turns each payload into Claude Code's shape (`cwd`, `tool_input.command`,
  `tool_input.file_path`), runs the unchanged hook in the session cwd, and maps the answer back: exit 2 becomes
  each harness's block signal; `additionalContext` becomes `injectSteps` (agy), `{"context"}` (Hermes), plain
  stdout (pi), or passes through (Codex). A note a harness has no channel for goes to stderr. For Hermes the
  session hooks run once per `session_id` (marker files under `~/.cache/agent-hooks/`, pruned after 30 days).
  The brief's estimate of a "twenty-line normaliser" holds for field renames only. Output mapping, Codex patch
  parsing and Hermes's once-per-session rule are most of the code.
- pi gets a ~110-line extension (`~/.pi/agent/extensions/agent-hooks.ts`) that calls the same adapter.
- Tests: `test_agent_hook_adapter.py`, 17 cases (pytest via `uvx`): block with the guard's reason text for every
  harness, benign pass, per-harness context mapping, Codex multi-file patch paths, Hermes once-per-session, bad
  payload. Negative control: with exit-2 handling removed, all four block tests failed.
- Measured per harness:
  - Codex: each registered command string run directly with a synthetic payload. The guard blocked
    `git push --force` with its reason; the learnings note was produced; the others exited 0 silently.
  - Hermes: `hermes hooks test pre_tool_call --for-tool terminal` blocked the force push (`exit=2`, parsed as
    `{"action":"block",...}`) and passed `ls`; `hermes hooks test pre_llm_call` produced the `{"context"}` note.
    Trap: `--payload-file` merges into the call **kwargs**, so a tool's input goes under `args`, not
    `tool_input`. A payload with `tool_input` reaches the hook as nothing and the guard passes it.
  - pi: the handlers, run under Node with a stand-in `pi`, sent the note once and hidden, skipped it on
    `reload`, blocked the force push, passed `ls`, and formatted a written Python file in place. Node needs the
    copy named `.mts`, because `~/.pi/agent/package.json` makes a `.ts` file load as CommonJS.
  - agy: registered under the `agents-shared` key. No run of any kind. Format-on-edit is not registered there
    (edit tool name unknown).
- What it writes into each harness, and how to remove it: one appended group per event in `~/.codex/hooks.json`
  (positions of existing groups kept, because trust is keyed by position); one `agents-shared` key in
  `~/.gemini/config/hooks.json`; one commented `hooks:` block at the end of `~/.hermes/config.yaml`; one extension
  file for pi. Each is self-contained and can be removed by deleting it.

## 4. Points for the wsx feature, if the verdict is "build"

- **Payload normalisation is the small part.** The work is in the per-harness output contract, the consent gates
  (Codex trust hashes keyed by position, Hermes allowlist) and session-start delivery, which only Codex and agy
  support directly.
- **Installers must be position-stable and idempotent.** Codex trust breaks if an installer inserts before
  existing groups. Append, and detect an existing install by command text.
- **`wsx doctor` needs per-harness probes.** Only Hermes has a native hook tester. Elsewhere the proof is either a
  live turn or a synthetic run of the registered command string.
- **Third-party installers can register hooks that never fire.** GSD 1.21.0 writes Claude-shaped hook blocks into
  `~/.gemini/antigravity/settings.json` and `~/.hermes/settings.json`, which neither CLI reads for hooks, under event
  and tool names they do not use (logged in the operator's GSD incident log). A doctor check that a registration
  sits in the file the harness reads, with names it recognises, would have caught it.
- **IF-02 carve-out, as measured:** the files a hooks command would have to write are exactly the four above, plus
  reading `~/.codex/config.toml` (trust state) and `~/.hermes/shell-hooks-allowlist.json` (consent state) for
  doctor. Nothing else in the harness directories was needed.
- **Herdr manages its own per-harness hook** (`herdr-agent-state.*`, overwritten on herdr updates). A wsx feature
  should leave such entries alone and not count them as its own.

## Still open (for the spike)

1. Captured live payloads for every harness, in sandboxed config dirs (brief section 2).
2. agy edit tool name and argument names, and whether agy honours exit 2.
3. Whether pi's `before_agent_start` message reaches the model, and whether a blocked call's `reason` does.
4. Codex: where the trust prompt for new hooks appears, and the default timeout.
5. The candidates in brief section 3, compared against this baseline.
6. Gemini CLI, once installed.
