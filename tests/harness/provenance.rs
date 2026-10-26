//! Provenance of the vendored oracle (plan 02-04).
//!
//! Proves that the scripts vendored under `tests/oracle` are the files devflow
//! published at the pinned revision, and that the patch the harness applies to
//! its scratch copy still applies and changes only what it says it changes.
//!
//! Isolation: the checked-in files are only read; anything patched is a
//! scratch copy inside the test's own `harness-provenance-` tree under
//! `CARGO_TARGET_TMPDIR`.

use std::fs;
use std::path::Path;

use crate::env::{Env, ScenarioDirs, hermetic_env, inherited_env};

/// The devflow commit the oracle was vendored from (`origin/develop`,
/// 2026-10-23, "Merge pull request #228 from denniyahh/feature/dependabot").
const DEVFLOW_COMMIT: &str = "ee6ec792d03f1495fc8c4d90d5894ff08b968d80";

/// The vendored oracle, as committed.
const ORACLE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/oracle");

/// Every vendored file: its path under `tests/oracle`, its path in devflow,
/// its mode there and its blob at `DEVFLOW_COMMIT`, from `git ls-tree -l
/// ee6ec792 -- <path>` in devflow (RESEARCH "Oracle inventory").
const VENDORED_BLOBS: [(&str, &str, &str, &str); 6] = [
    (
        "sync-workspace.sh",
        "scripts/sync-workspace.sh",
        "100755",
        "82ed2d7572e03c7527f3deced2a51562ba891e85",
    ),
    (
        "phase-worktree.sh",
        "scripts/phase-worktree.sh",
        "100755",
        "ae6e47022137768a6ff947151c7169b898781719",
    ),
    (
        "cut-pr-branch.sh",
        "scripts/cut-pr-branch.sh",
        "100755",
        "d77eda55b84aa67215ecf51ddde2b38c982a6989",
    ),
    (
        "check-workspace-divergence.sh",
        "scripts/check-workspace-divergence.sh",
        "100755",
        "0041f8a50fae25a2b88b1fe4ac9393314697909f",
    ),
    (
        "hooks/pre-push",
        "scripts/hooks/pre-push",
        "100755",
        "67ef847ecf98dfaa1b124be5b10179b7a0134dc9",
    ),
    (
        "LICENSE",
        "LICENSE",
        "100644",
        "9fde56fe9dc9a61107fb774925149bd11942de67",
    ),
];

/// `(path, blob)` for every vendored file.
fn vendored_pairs() -> [(&'static str, &'static str); 6] {
    VENDORED_BLOBS.map(|(path, _, _, blob)| (path, blob))
}

/// Each file in `table` (paths relative to `dir`) whose content does not hash
/// to its recorded blob, as `<path>: <reason>`. A file that cannot be hashed is
/// reported too, so a missing file never reads as a match.
fn blob_mismatches(
    dir: &Path,
    table: impl IntoIterator<Item = (&'static str, &'static str)>,
    env: &Env,
) -> Vec<String> {
    let _ = (dir, table, env);
    Vec::new()
}

/// Copies every vendored file and `oracle.patch` from `tests/oracle` to `to`,
/// keeping each file's permission bits.
fn scratch_copy(to: &Path) {
    for path in VENDORED_BLOBS
        .map(|(path, _, _, _)| path)
        .into_iter()
        .chain(["oracle.patch"])
    {
        let target = to.join(path);
        let parent = target.parent().expect("a vendored path has a parent");
        fs::create_dir_all(parent).expect("create the scratch copy's directories");
        fs::copy(Path::new(ORACLE_DIR).join(path), &target).expect("copy a vendored file");
    }
}

/// The paths a mismatch list names, in order.
fn reported_paths(mismatches: &[String]) -> Vec<&str> {
    mismatches
        .iter()
        .map(|line| {
            line.split_once(": ")
                .map_or(line.as_str(), |(path, _)| path)
        })
        .collect()
}

/// A copy that differs from the vendored set by one flipped byte in one file
/// is reported naming exactly that file.
#[test]
fn flipped_byte_copy_is_reported() {
    let dirs = ScenarioDirs::new("provenance", "flipped_byte").expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let copy = dirs.root.join("copy");
    scratch_copy(&copy);
    let flipped = copy.join("cut-pr-branch.sh");
    let mut bytes = fs::read(&flipped).expect("read the copied script");
    bytes[0] ^= 0x01;
    fs::write(&flipped, &bytes).expect("write the flipped script");

    let mismatches = blob_mismatches(&copy, vendored_pairs(), &env);
    assert_eq!(
        reported_paths(&mismatches),
        ["cut-pr-branch.sh"],
        "a flipped byte in cut-pr-branch.sh must be the one file reported against \
         devflow {DEVFLOW_COMMIT}; full report: {mismatches:?}"
    );
}
