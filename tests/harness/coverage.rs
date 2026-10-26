//! Coverage of the spawned binary (plan 02-06).
//!
//! Proves that a coverage run counts code that only the spawned `wsx` binary
//! executes, so black-box scenarios contribute to coverage even though no
//! test calls that code in-process.
//!
//! Isolation: the binary runs inside the test's own `harness-coverage-` tree
//! under `CARGO_TARGET_TMPDIR`, with the hermetic environment, whose only
//! inherited keys are the `LLVM_` ones coverage needs.
//!
//! The proof itself is a one-time run of `cargo llvm-cov nextest` recorded in
//! the 02-06 SUMMARY; `scripts/check.sh coverage` reruns it, and neither `all`
//! nor CI runs coverage. These tests are its subject and
//! its no-passthrough control, and both also hold in an ordinary test run.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use crate::env::{Env, ScenarioDirs, command, hermetic_env, inherited_env};

/// The profile a coverage run asks every instrumented process to write. An
/// instrumented child that does not see it writes `default_*.profraw` into
/// its working directory instead.
const PROFILE_FILE_KEY: &str = "LLVM_PROFILE_FILE";

/// Spawns the built `wsx --version` with `env`, working in the scenario root.
fn spawn_wsx_version(dirs: &ScenarioDirs, env: &Env) -> Output {
    command(env!("CARGO_BIN_EXE_wsx"), &dirs.root, env)
        .arg("--version")
        .output()
        .expect("spawn the wsx binary")
}

/// The output `wsx --version` must produce, however its environment is built.
fn assert_reports_version(output: Output) {
    let stdout = String::from_utf8(output.stdout).expect("wsx stdout is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("wsx stderr is UTF-8");
    let expected = format!("wsx {}\n", env!("CARGO_PKG_VERSION"));
    assert_eq!(stdout, expected, "`wsx --version` stdout");
    assert_eq!(stderr, "", "`wsx --version` must not write to stderr");
    assert_eq!(output.status.code(), Some(0), "`wsx --version` exit code");
}

/// Every file under `root`, at any depth, whose name ends in `.profraw`.
fn profraw_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries =
            fs::read_dir(&dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
        for entry in entries {
            let entry = entry.unwrap_or_else(|err| panic!("read in {}: {err}", dir.display()));
            let path = entry.path();
            let kind = entry
                .file_type()
                .unwrap_or_else(|err| panic!("file type of {}: {err}", path.display()));
            if kind.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "profraw") {
                found.push(path);
            }
        }
    }
    found
}

/// The coverage proof's subject: `wsx --version` spawned through the hermetic
/// environment reports its version, and leaves no `*.profraw` anywhere in the
/// scenario tree. Under `cargo llvm-cov` the passed-through `LLVM_*` keys send
/// the child's profile to the coverage run, so a profile file here means the
/// passthrough was lost.
#[test]
fn spawned_wsx_reports_version() {
    let dirs = ScenarioDirs::new("coverage", "reports_version").expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);

    assert_reports_version(spawn_wsx_version(&dirs, &env));

    let stray = profraw_files(&dirs.root);
    assert!(
        stray.is_empty(),
        "the hermetic env must carry LLVM_* to the child; profile files in the scenario: {stray:?}"
    );
}

/// Control B of the coverage proof: the same spawn with every `LLVM_*` key
/// removed from the child's environment (and nothing changed in this process).
/// The child still reports its version. Under `cargo llvm-cov`, where this
/// process has `LLVM_PROFILE_FILE`, the child drops a stray `*.profraw` into its
/// working directory and its run is not counted; that first branch only runs
/// under coverage. In an ordinary run there is no profile to write, so none
/// may appear.
#[test]
fn spawned_wsx_without_llvm_passthrough() {
    let dirs = ScenarioDirs::new("coverage", "without_llvm").expect("scenario dirs");
    let env: Env = hermetic_env(&inherited_env(), &dirs)
        .into_iter()
        .filter(|(key, _)| !key.as_encoded_bytes().starts_with(b"LLVM_"))
        .collect();

    assert_reports_version(spawn_wsx_version(&dirs, &env));

    let stray = profraw_files(&dirs.root);
    if std::env::var_os(PROFILE_FILE_KEY).is_some() {
        assert!(
            !stray.is_empty(),
            "under coverage, a child without LLVM_* must drop a profile file in its working \
             directory {}",
            dirs.root.display()
        );
    } else {
        assert!(
            stray.is_empty(),
            "outside coverage no profile file may appear; found {stray:?}"
        );
    }
}
