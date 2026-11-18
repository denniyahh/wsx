# The `--json` contract

Every wsx command that takes `--json` prints exactly one JSON object on stdout, followed by one
newline, on every outcome: success, problems found, or an error. Each command's object is
described by a JSON Schema (draft 2020-12) in this directory, one file per command and schema
version: `<command>.v<schema_version>.schema.json`. Exit codes and error kinds are in
[docs/exit-codes.md](../exit-codes.md).

| Command | Schema |
|---------|--------|
| `check` | [check.v1.schema.json](check.v1.schema.json) |

## The envelope

Every object carries two fields first:

- `command`: the command's name, as typed (`"check"`).
- `schema_version`: the version of the command's schema the object follows, an integer.

A run that completed (exit 0 or 1) adds the command's own fields and has no `error` key. A run
that could not complete (exit 2 to 6) carries only the envelope and `error`:

```json
{"command": "check", "schema_version": 1,
 "error": {"kind": "workspace_candidates", "message": "...", "exit_code": 3}}
```

`error.kind` is one of the kinds in [docs/exit-codes.md](../exit-codes.md), `error.message` is the
error's message (stderr shows it too, after a `wsx <command>: ` prefix), and `error.exit_code` is
the process's exit code.

## stdout and stderr

With `--json`, stdout holds the object and nothing else. Everything for a person (diagnostics,
the verdict line, error messages) goes to stderr, whether or not `--json` is given. Without
`--json`, the verdict line goes to stdout and nothing else does.

A usage error before a command is recognised (no command, an unknown command, `--json` as the
first argument) prints no JSON: there is no command whose schema the object would follow. Only
stderr and exit 2 report it. Once the first argument names a command and `--json` follows it,
a usage error is that command's failure object with `error.kind` `usage`.

## Bytes

Git paths and names are bytes, not text. A field holding them (a path, a branch, a ref) is:

- a JSON string when the bytes are valid UTF-8; control characters are escaped as JSON
  requires (`"a\nb.md"`);
- otherwise an object `{"hex": "<lowercase hex of the raw bytes>"}`, two digits per byte: the
  path made of the byte 0xFF and `.md` is `{"hex": "ff2e6d64"}`.

## Key order

Key order is not part of the contract. wsx writes keys in a fixed order today, but a consumer
must look fields up by name.

## Versioning

A schema file, once committed, never changes. Any change to a command's object (a field added,
removed, renamed or retyped, an enum value added) is a new schema version:

1. bump the command's `schema_version` in `src/cli/commands.rs`;
2. add `<command>.v<N+1>.schema.json` beside the old file;
3. pin the new file's blob in `SCHEMA_PINS` in `tests/harness/contract.rs`.

The old file and its pin stay, so a consumer of version N can still validate what it reads.
The contract tests enforce this: every command's output must validate against its current
schema, each schema file must hash to its pinned blob, and every command in the table must have
a schema file and a pin.
