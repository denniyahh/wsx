## Deferred Items

- `cargo fmt` (and so `scripts/check.sh fmt`) never checks the scenario modules
  status: resolved
  **Resolved (2026-11-04):** `scripts/check.sh fmt` now also runs `rustfmt --edition 2024 --check`
  on every `.rs` file git does not ignore (check.sh `run_fmt`), so macro-declared modules are
  read. Checked 2026-11-04: `check.sh fmt` exits 0; all 18 `tests/harness/*.rs` pass rustfmt;
  `gate_controls::fmt_checks_an_untracked_rs_file` runs and passes (a mis-formatted file must fail).
  **Found during:** 02-10, Task 2.
  **What:** `tests/harness/main.rs` declares `sync`, `worktree`, `check`, `cut`, `guard` and
  `signing` inside the `harness_modules!` macro invocation. rustfmt does not follow `mod`
  declarations inside a macro, so `cargo fmt --check` passes without reading those files.
  **Evidence (2026-10-26):** `rustfmt --edition 2024 --check` passes `scenario.rs`,
  `outcome.rs`, `fixture.rs`, `action.rs`, `oracle.rs`, `registry.rs` and `env.rs` (the files
  `cargo fmt` reaches), and fails `sync.rs`, `worktree.rs` and `cut.rs`; `guard.rs` failed at
  02-09's tip too. 02-10 formatted `guard.rs` with rustfmt directly and left the others alone.
  **Fix options:** have `check.sh fmt` also run `rustfmt --edition 2024 --check` on every
  `tests/harness/*.rs`, or declare the scenario modules as plain `mod` items outside the macro
  and keep the macro for the registry only. Owner: whichever plan next
  touches `check.sh` or `main.rs` (02-15 is the phase-end rules plan).

  **Resolved b3994c4 by the orchestrator:** `scripts/check.sh fmt` now also runs `rustfmt --check` on every tracked `.rs` file; WINDOWS entry 1 marked fixed.
