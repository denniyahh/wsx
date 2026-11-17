# Phase 04 deferred items

## RUSTSEC-2026-0486: rustix 1.1.5 is unsound (found during 04-12 Task 3)

- `scripts/check.sh deps` exits 1 on 04-12's tip: `advisories FAILED, bans ok, licenses ok, sources ok`,
  the one error `RUSTSEC-2026-0486` (`xattr functions mark unwritten bytes as initialized when given a
  zero-length buffer`) against `rustix 1.1.5`, pulled in by `gix-index 0.58.0` and `tempfile 3.27.0`.
  Log: `target/gsd/04-12-deps.log`.
- Not caused by 04-12: `Cargo.lock` is unchanged since before the phase; the advisory is new in the
  database. The advisory's fix is rustix `>=1.1.6` (`cargo update -p rustix`).
- Not applied by the executor: a lockfile update is a package-manager change the executor does not make
  on its own. It needs the operator's word. Bumping `Cargo.lock` after 04-12's mutation pass makes that
  pass's pass-head check (Task 3's third block) fail until the pass is re-run on the new tip.
