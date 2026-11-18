# Exit codes

Every wsx command exits with one of seven codes. A code names a class of outcome, not a
particular error, and means the same thing for every command, so a caller can tell "found
problems" from "could not run" from "fix your setup" by the code alone. The error's kind names
the particular error within its class; with `--json` it is the `error.kind` field.

## Classes

| Code | Class | Meaning |
|------|-------|---------|
| 0 | success | The command ran; for `check`, no problems |
| 1 | problems | The command ran and found problems (`check`: divergence, stale declaration, hook mode) |
| 2 | usage | Arguments the command line does not accept |
| 3 | not_configured | The command needs a workspace branch and none is configured or inferable |
| 4 | invalid_config | Invalid configuration: `wsx.*` keys, the manifest, a missing remote or base ref, an empty integration chain |
| 5 | refused | Repository refused: an unsupported state wsx will not read (reftable, the object-store, relative-repo and implicit-worktree environment refusals, a stale worktree, a refs namespace, replace refs) |
| 6 | git_failed | Git failed: a git child failed, timed out or gave empty or bad output, an object is missing, or gix or the operating system failed |

## Kinds

| Kind | Class |
|------|-------|
| `usage` | usage |
| `not_a_repo` | not_configured |
| `workspace_candidates` | not_configured |
| `not_configured` | not_configured |
| `key_repeated` | invalid_config |
| `key_empty` | invalid_config |
| `not_a_branch_name` | invalid_config |
| `no_such_remote` | invalid_config |
| `base_ref_missing` | invalid_config |
| `no_integration_branch` | invalid_config |
| `integration_ref_symbolic` | invalid_config |
| `workspace_branch_missing` | invalid_config |
| `manifest_missing` | invalid_config |
| `manifest_not_a_blob` | invalid_config |
| `manifest_invalid` | invalid_config |
| `worktree_dir_invalid` | invalid_config |
| `manifest_unreadable` | the class of the read that failed: refused for a replace-ref refusal, else git_failed |
| `foreign_owner` | refused |
| `reftable` | refused |
| `relative_repo_env` | refused |
| `object_store_env` | refused |
| `refs_namespace` | refused |
| `implicit_work_tree` | refused |
| `stale_worktree` | refused |
| `replace_refs_present` | refused |
| `replace_base_set` | refused |
| `replace_refs_unreadable` | refused |
| `git_too_old` | git_failed |
| `git_failed` | git_failed |
| `git_timed_out` | git_failed |
| `open_timed_out` | git_failed |
| `git_empty_output` | git_failed |
| `git_bad_output` | git_failed |
| `io` | git_failed |
| `gix` | git_failed |
| `object_missing` | git_failed |

A replace-ref refusal keeps its own kind and class 5 wherever it is reported, including when it
leaves a repository not configured. A new error kind gets its row here in the same commit as the
variant that introduces it.
