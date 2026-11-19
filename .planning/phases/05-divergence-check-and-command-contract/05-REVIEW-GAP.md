---
phase: 05-divergence-check-and-command-contract
reviewed: 2026-11-19T02:20:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - tests/if02_scan.rs
  - tests/harness/if02_runtime.rs
findings:
  critical: 0
  warning: 2
  info: 1
  total: 3
  necessity: 0
status: issues_found
---

# Phase 05: Code Review Report (gap plan 05-10)

**Reviewed:** 2026-11-19T02:20:00Z
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Reviewed the IF-02 source scan (literal decoding, `concat!` and `.join` chain joining, finding dedup) and the
IF-02/SC5 run-time comparison harness, diff base 27bd3ba. Nothing was found that makes the scan
pass when it should fail on the shapes it claims to cover. The chain walk (`end += 3`), the
`covers` dedup, the nested-`concat!` ordering and the raw/byte/C-string prefix stripping all
check out when traced by hand. The harness's HOME channel is real: `hermetic_env` sets `HOME` to
`dirs.home` (tests/harness/env.rs:139), the same path the harness writes, and `mod if02_runtime`
is registered (tests/harness/main.rs:32).

There are two defects. The scan's decoder panics on a well-formed escape that the lexer accepts,
which goes against its documented invariant. The run-time states cannot catch a lenient GSD reader
that falls back to defaults. Tests were not run for this review; the findings come from reading
the code and the vendored proc-macro2 1.0.107 lexer.

No necessity findings: every guard checked either sits on a file-system or trust boundary or
is exercised by a test.

## Warnings

### WR-01: Decoder panics on a CRLF line continuation, which the lexer accepts

**File:** `tests/if02_scan.rs:391-397` (invariant stated at lines 31-33 and 368-369)
**Issue:** The doc comment says "the lexer refuses every malformed escape, so the decoder panics on
an escape it does not know". proc-macro2 1.0.107's fallback lexer accepts a backslash followed by
`\r` as a string continuation: `cooked_string` matches `ch @ ('\n' | '\r')` and then calls
`trailing_backslash`, which accepts `\r\n` (proc-macro2-1.0.107/src/parse.rs:400-404).
`Literal::to_string()` returns the source text unchanged, so a source file with CRLF line endings
that continues a string with a backslash reaches `unescape` as `\\`, `\r`, `\n`. `unescape` has
an arm for `'\n'` only, so `\r` falls to `other => panic!("... cannot decode the escape")`. The
repository has no `.gitattributes` to force LF, so a checkout with `core.autocrlf=true`, or one
file saved with CRLF, turns `real_tree_reads_no_agent_file` into a panic that blames a decoder
defect. The gate still fails rather than passing, but on valid source, with a misleading message,
and the "impossible" branch is reachable.
**Fix:** Treat `\r\n` after a backslash as a continuation, and add a CRLF case beside
`escaped-continuation`:
```rust
Some('\n') | Some('\r') => {
    while chars
        .next_if(|next| matches!(next, ' ' | '\t' | '\r' | '\n'))
        .is_some()
    {}
}
```
(The lexer already guarantees that a `\r` here is followed by `\n`.) Optionally also add
`* text=auto eol=lf` for `*.rs` in a `.gitattributes`.

### WR-02: State-A sentinels carry no keys a GSD or devflow reader acts on, so a lenient reader goes undetected

**File:** `tests/harness/if02_runtime.rs:6-9, 72-86`
**Issue:** State A writes `{"note": "if02 sentinel-A"}`, `note = "if02 sentinel-A"` and a minimal
roadmap. State B writes malformed text. The module doc says "a parse that fails on it also
differs", but that holds only when the failure reaches the output. A regression shaped like
`serde_json::from_slice::<GsdConfig>(&bytes).unwrap_or_default()` (or `toml::from_str(..).ok()`
with defaults) returns the default value in both states. In A the `note` key is ignored and every
real key is missing; in B the parse fails and falls back to the default. `check` then gives the
same output in both states and `check_output_does_not_depend_on_agent_files` passes. The four
reader-variant controls do not cover this, because they append raw bytes on the test side and
never go through a parser. Of all regressions, a quiet read of `.planning/config.json` or
`devflow.toml` for one setting is among the most likely, and it is the one this run-time check
misses.
**Fix:** In state A, write files whose content would change behaviour if read: a
`.planning/config.json` with real GSD keys set to non-default values (for example a
non-default `git.branching_strategy` or branch template), a `devflow.toml` with real keys set
to non-default values, and `.claude/settings.json` with real fields. Keep B malformed. Also
consider a third state with well-formed but different non-default values, so that a reader which
ignores malformed input and one that uses defaults both produce a difference. Narrow the
sentence at line 8 to what the states actually prove.

## Info

### IN-01: "What it leaves: ... consts and statics" reads as if const literals are not scanned

**File:** `tests/if02_scan.rs:19`
**Issue:** A literal in a `const` or `static` initialiser is a literal token and is reported.
What the scan misses is a value composed through a const binding, such as
`const P: &str = ".planning"; Path::new(P).join("config.json")`. As written, the sentence
undersells the scan and could mislead a later edit into adding a redundant check.
**Fix:** Reword to "values composed through a const or static binding (`Path::new(P).join(...)`)".

---

_Reviewed: 2026-11-19T02:20:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
