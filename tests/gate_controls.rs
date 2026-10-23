//! Gate controls as repeatable tests: each one checks that a guard in
//! `scripts/check.sh` or `clippy.toml` refuses the case it exists for.
//!
//! - A tool missing from PATH is named on stderr and fails the gate (DEV-01).
//! - The nextest pin is checked on the binary cargo runs, not on another copy
//!   found by a different search (T-01-07).
//! - A filter that matches nothing fails even with `NEXTEST_NO_TESTS=pass`
//!   (HARN-04), with the bare-nextest counterpart showing that the env var
//!   would otherwise turn it green.
//! - `std::env::set_var` and `remove_var` fail clippy through the repository
//!   `clippy.toml` (HARN-04), with an empty-config counterpart.
//!
//! Unlike tests/gate.rs, the last two start nested cargo builds. Each gets its
//! own target directory under `CARGO_TARGET_TMPDIR`: the outer run's build
//! lock is never contended, and the container's read-only source mount is
//! never written.

use std::env;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");

/// Exit code check.sh's `require_tool` uses for a missing tool.
const MISSING_TOOL_EXIT: i32 = 1;
/// nextest's exit code for "no tests to run".
const NEXTEST_NO_TESTS_EXIT: i32 = 4;
/// A filterset expression that matches no test in this package.
const NO_MATCH_FILTER: &str = "test(=no_such_test_xyz)";

fn text(bytes: Vec<u8>, what: &str) -> String {
    String::from_utf8(bytes).unwrap_or_else(|err| panic!("{what} is not UTF-8: {err}"))
}

/// A fresh, empty directory under CARGO_TARGET_TMPDIR, unique per caller.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = Path::new(TMP_ROOT).join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap_or_else(|err| panic!("remove {}: {err}", dir.display()));
    }
    fs::create_dir_all(&dir).unwrap_or_else(|err| panic!("create {}: {err}", dir.display()));
    dir
}

/// First `name` on this test process's PATH, as check.sh would find it.
fn on_path(name: &str) -> PathBuf {
    let path = env::var_os("PATH").expect("PATH is set for the test process");
    env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("'{name}' is not on the test's PATH"))
}

/// A directory holding symlinks to exactly `tools`, to use as the whole PATH.
/// Built explicitly rather than by filtering PATH: this host has several
/// tools in more than one PATH directory, so filtering one out leaves another.
fn minimal_path(name: &str, tools: &[&str]) -> PathBuf {
    let dir = fresh_dir(name);
    for tool in tools {
        symlink(on_path(tool), dir.join(tool))
            .unwrap_or_else(|err| panic!("symlink {tool} into {}: {err}", dir.display()));
    }
    dir
}

fn check_sh() -> PathBuf {
    Path::new(REPO_ROOT).join("scripts/check.sh")
}

/// Runs `check.sh <target>` with `PATH` set to `path_dir` alone.
fn check_sh_with_path(target: &str, path_dir: &Path) -> Output {
    Command::new("/bin/bash")
        .arg(check_sh())
        .arg(target)
        .env("PATH", path_dir)
        .output()
        .expect("spawn /bin/bash scripts/check.sh")
}

/// check.sh `target` with only `present` on PATH must exit 1, name `missing`
/// on stderr, and never report success.
fn assert_missing_tool_named(target: &str, present: &[&str], missing: &str) {
    let dir_name = format!("missing-tool-{target}-{missing}");
    let path_dir = minimal_path(&dir_name, present);
    let output = check_sh_with_path(target, &path_dir);
    let stdout = text(output.stdout, "check.sh stdout");
    let stderr = text(output.stderr, "check.sh stderr");

    assert_eq!(
        output.status.code(),
        Some(MISSING_TOOL_EXIT),
        "check.sh {target} with PATH={present:?} exit code; stderr: {stderr:?}"
    );
    let named = format!("'{missing}' is required for 'check.sh {target}'");
    assert!(
        stderr.contains(&named),
        "check.sh {target} with PATH={present:?} must name {missing:?} on stderr, got {stderr:?}"
    );
    assert!(
        !stdout.contains("OK"),
        "check.sh {target} with a missing tool must not report success, got stdout {stdout:?}"
    );
}

#[test]
fn missing_cargo_fails_every_cargo_target_by_name() {
    // `dirname` is the only external command check.sh runs before its first
    // tool check (it locates the repository root with it).
    for target in ["all", "fmt", "clippy", "test", "deps"] {
        assert_missing_tool_named(target, &["dirname"], "cargo");
    }
}

#[test]
fn missing_rustc_fails_the_toolchain_report_by_name() {
    assert_missing_tool_named("fmt", &["dirname", "cargo"], "rustc");
}

#[test]
fn missing_rustfmt_fails_fmt_by_name() {
    assert_missing_tool_named("fmt", &["dirname", "cargo", "rustc"], "cargo-fmt");
}

#[test]
fn missing_clippy_fails_clippy_by_name() {
    assert_missing_tool_named("clippy", &["dirname", "cargo", "rustc"], "cargo-clippy");
}

#[test]
fn missing_nextest_fails_test_by_name() {
    assert_missing_tool_named("test", &["dirname", "cargo", "rustc"], "cargo-nextest");
}

#[test]
fn missing_cargo_deny_fails_deps_by_name() {
    assert_missing_tool_named("deps", &["dirname", "cargo"], "cargo-deny");
}

/// A stand-in `cargo-nextest`. Run directly with `--version` it hands off to
/// the real, pinned binary, so a check that looks the tool up on PATH sees the
/// pin. Run by cargo, as `cargo-nextest nextest --version`, it reports a
/// version that is not the pin. Anything else stands in for a test run that
/// runs nothing and exits 0.
fn nextest_stub(real: &Path) -> String {
    format!(
        "#!/bin/bash\n\
         if [ \"$1\" = --version ]; then exec '{}' --version; fi\n\
         if [ \"$1\" = nextest ] && [ \"$2\" = --version ]; then\n\
         \x20   echo 'cargo-nextest 0.0.0-stub'; exit 0\n\
         fi\n\
         echo \"STUB cargo-nextest ran: $*\"\n",
        real.display()
    )
}

#[test]
fn nextest_pin_is_checked_on_the_binary_cargo_runs() {
    // The documented setup: the pinned nextest in $CARGO_HOME/bin, which is
    // on PATH. cargo then follows PATH order, so an earlier copy (here the
    // stub, in practice a package manager's) is the one `cargo nextest` runs.
    let root = fresh_dir("cargo-resolution");
    let stub_dir = root.join("stub");
    let cargo_home = root.join("cargo-home");
    let cargo_home_bin = cargo_home.join("bin");
    for dir in [&stub_dir, &cargo_home_bin] {
        fs::create_dir_all(dir).unwrap_or_else(|err| panic!("create {}: {err}", dir.display()));
    }
    let real = on_path("cargo-nextest");
    symlink(&real, cargo_home_bin.join("cargo-nextest")).expect("link pinned nextest");
    let stub = stub_dir.join("cargo-nextest");
    fs::write(&stub, nextest_stub(&real)).expect("write cargo-nextest stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    let tools = minimal_path(
        "cargo-resolution-tools",
        &["bash", "cargo", "rustc", "dirname"],
    );
    let path = env::join_paths([&stub_dir, &cargo_home_bin, &tools]).expect("join PATH");

    let output = Command::new("/bin/bash")
        .arg(check_sh())
        .arg("test")
        .env("PATH", path)
        .env("CARGO_HOME", &cargo_home)
        .output()
        .expect("spawn /bin/bash scripts/check.sh test");
    let stdout = text(output.stdout, "check.sh stdout");
    let stderr = text(output.stderr, "check.sh stderr");

    assert_eq!(
        output.status.code(),
        Some(1),
        "check.sh test must refuse the unpinned nextest cargo runs; \
         stdout: {stdout:?}; stderr: {stderr:?}"
    );
    assert!(
        stderr.contains("reports 'cargo-nextest 0.0.0-stub'"),
        "the refusal must name the version cargo's nextest reports, got stderr {stderr:?}"
    );
    assert!(
        !stdout.contains("STUB cargo-nextest ran"),
        "the unverified nextest must never run, got stdout {stdout:?}"
    );
    assert!(
        !stdout.contains("check.sh: test OK"),
        "check.sh test must not report success, got stdout {stdout:?}"
    );
}

/// A command that runs a nested nextest: inherited `NEXTEST_*` variables from
/// the outer run are dropped, then `NEXTEST_NO_TESTS=pass` is set, the
/// override check.sh's `--no-tests=fail` must beat.
fn nested_nextest_command(program: &str, target_dir: &Path) -> Command {
    let mut command = Command::new(program);
    for (key, _) in env::vars_os() {
        if key.to_string_lossy().starts_with("NEXTEST_") {
            command.env_remove(&key);
        }
    }
    command
        .current_dir(REPO_ROOT)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("NEXTEST_NO_TESTS", "pass");
    command
}

#[test]
fn no_match_filter_fails_even_when_env_allows_no_tests() {
    let target_dir = Path::new(TMP_ROOT).join("nested-nextest-target");

    // Counterpart first: bare nextest honours NEXTEST_NO_TESTS=pass and goes
    // green over zero tests. Without this the assertion below could not tell
    // check.sh's flag apart from nextest's own default, which already fails.
    let bare = nested_nextest_command("cargo", &target_dir)
        .args([
            "nextest",
            "run",
            "--locked",
            "--workspace",
            "-E",
            NO_MATCH_FILTER,
        ])
        .output()
        .expect("spawn cargo nextest");
    assert_eq!(
        bare.status.code(),
        Some(0),
        "bare nextest with NEXTEST_NO_TESTS=pass must pass over zero tests, or this control \
         proves nothing; stderr: {:?}",
        text(bare.stderr, "nextest stderr")
    );

    let gated = nested_nextest_command("/bin/bash", &target_dir)
        .arg(check_sh())
        .args(["test", "-E", NO_MATCH_FILTER])
        .output()
        .expect("spawn /bin/bash scripts/check.sh test");
    let stdout = text(gated.stdout, "check.sh stdout");
    let stderr = text(gated.stderr, "check.sh stderr");

    assert_eq!(
        gated.status.code(),
        Some(NEXTEST_NO_TESTS_EXIT),
        "check.sh test with a no-match filter and NEXTEST_NO_TESTS=pass exit code; \
         stdout: {stdout:?}; stderr: {stderr:?}"
    );
    assert!(
        stderr.contains("no tests to run"),
        "the failure must be nextest's no-tests refusal, got stderr {stderr:?}"
    );
    // Not a bare "OK": the tool-pin check above it prints "cargo-nextest ... OK".
    assert!(
        !stdout.contains("check.sh: test OK"),
        "check.sh test over zero tests must not report success, got stdout {stdout:?}"
    );
}

/// `cargo clippy` over the env-ban fixture crate, which plants
/// `std::env::set_var` and `std::env::remove_var`.
/// Each caller passes its own `target_name`: clippy's cached result for one
/// configuration must never be replayed for another.
fn clippy_env_ban_fixture(target_name: &str, extra_env: Option<(&str, &Path)>) -> Output {
    let manifest = Path::new(REPO_ROOT).join("tests/fixtures/env-ban/Cargo.toml");
    let mut command = Command::new("cargo");
    command
        .arg("clippy")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(&manifest)
        .args(["--", "-D", "warnings"])
        .env("CARGO_TARGET_DIR", Path::new(TMP_ROOT).join(target_name))
        // The toolchain must come from rust-toolchain.toml, not the outer run.
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("CLIPPY_CONF_DIR");
    if let Some((key, value)) = extra_env {
        command.env(key, value);
    }
    command.output().expect("spawn cargo clippy")
}

#[test]
fn repo_clippy_toml_bans_env_mutation() {
    let output = clippy_env_ban_fixture("env-ban-target", None);
    let stderr = text(output.stderr, "clippy stderr");

    assert_ne!(
        output.status.code(),
        Some(0),
        "clippy must reject the planted set_var/remove_var; stderr: {stderr:?}"
    );
    for method in ["std::env::set_var", "std::env::remove_var"] {
        let message = format!("use of a disallowed method `{method}`");
        assert!(
            stderr.contains(&message),
            "clippy must report {message:?}, got stderr {stderr:?}"
        );
    }
    // This reason text exists only in the repository's clippy.toml, so it
    // shows that file, found by walking up from the fixture, did the banning.
    assert!(
        stderr.contains("scope it to the child with Command::env instead"),
        "the ban must carry the repository clippy.toml reason, got stderr {stderr:?}"
    );
}

#[test]
fn env_mutation_passes_clippy_without_the_ban() {
    // Counterpart: the same fixture under an empty clippy.toml passes, so the
    // failure above comes from the ban and not from anything else in the
    // fixture (a compile error, another lint).
    let empty_conf = fresh_dir("env-ban-empty-conf");
    fs::write(empty_conf.join("clippy.toml"), "").expect("write empty clippy.toml");

    let output = clippy_env_ban_fixture(
        "env-ban-target-empty-conf",
        Some(("CLIPPY_CONF_DIR", &empty_conf)),
    );
    let stderr = text(output.stderr, "clippy stderr");

    assert_eq!(
        output.status.code(),
        Some(0),
        "the fixture must pass clippy with an empty config; stderr: {stderr:?}"
    );
}
