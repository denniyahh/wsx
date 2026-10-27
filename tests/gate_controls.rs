//! Gate controls as repeatable tests: each one checks that a guard in
//! `scripts/check.sh` or `clippy.toml` refuses the case it exists for.
//!
//! - A tool missing from PATH is named on stderr and fails the gate (DEV-01).
//! - `check.sh coverage` sets `CARGO_LLVM_COV_SETUP=no` itself, so
//!   cargo-llvm-cov never installs a toolchain component.
//! - The nextest pin is checked on the binary cargo runs, not on another copy
//!   found by a different search (T-01-07).
//! - A checkout's `.tools/bin` wins over a shadowing nextest earlier on PATH,
//!   with the counterpart showing the same shadow is refused without it.
//! - A stale nextest in `.tools/bin` is refused with a fix that installs into
//!   `.tools/bin`; without one, the fix is the plain install command.
//! - A stale nextest in `$CARGO_HOME/bin`, which cargo searches first while it
//!   is not on PATH, is refused with the plain install command naming that
//!   directory, even with a pinned `.tools/bin`; with `$CARGO_HOME/bin` on
//!   PATH, the `.tools/bin` copy runs.
//! - `scripts/check-in-container.sh` gates the checkout that holds it, not the
//!   caller's working directory.
//! - It hides the checkout's `.tools` from the container, with the
//!   counterpart that a checkout without one gets no mask.
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

/// A scratch checkout holding copies of the gate scripts and the toolchain
/// pin, unique per caller. check.sh puts its checkout's `.tools/bin` first on
/// PATH, so a test that sets PATH itself runs a copy: in this checkout a
/// developer's installed tools would join the PATH the test controls.
fn scratch_checkout(name: &str) -> PathBuf {
    let checkout = fresh_dir(name);
    fs::create_dir_all(checkout.join("scripts")).expect("create the scratch scripts dir");
    for file in [
        "scripts/check.sh",
        "scripts/install-tools.sh",
        "rust-toolchain.toml",
    ] {
        fs::copy(Path::new(REPO_ROOT).join(file), checkout.join(file))
            .unwrap_or_else(|err| panic!("copy {file}: {err}"));
    }
    checkout
}

/// check.sh in a scratch checkout named `name` (see `scratch_checkout`).
fn check_sh(name: &str) -> PathBuf {
    scratch_checkout(name).join("scripts/check.sh")
}

/// Runs `check.sh <target>` with `PATH` set to `path_dir` alone.
fn check_sh_with_path(name: &str, target: &str, path_dir: &Path) -> Output {
    Command::new("/bin/bash")
        .arg(check_sh(name))
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
    let output = check_sh_with_path(&format!("{dir_name}-checkout"), target, &path_dir);
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

#[test]
fn missing_cargo_llvm_cov_fails_coverage_by_name() {
    assert_missing_tool_named("coverage", &["dirname", "cargo", "rustc"], "cargo-llvm-cov");
}

/// The cargo-llvm-cov version `scripts/install-tools.sh` pins, read from the
/// script so the pin stays in one place.
fn pinned_llvm_cov_version() -> String {
    let script = Path::new(REPO_ROOT).join("scripts/install-tools.sh");
    let text = fs::read_to_string(&script).expect("read scripts/install-tools.sh");
    text.lines()
        .find_map(|line| line.strip_prefix("CARGO_LLVM_COV_VERSION="))
        .expect("install-tools.sh pins CARGO_LLVM_COV_VERSION")
        .to_owned()
}

/// A stand-in `cargo`: `cargo llvm-cov --version` reports the pinned version,
/// `cargo llvm-cov nextest ...` records the `CARGO_LLVM_COV_SETUP` it sees
/// (`<unset>` if none) and its arguments in `record`, and anything else runs
/// the real cargo.
fn cargo_stub(real: &Path, pin: &str, record: &Path) -> String {
    format!(
        "#!/bin/bash\n\
         if [ \"$1\" = llvm-cov ] && [ \"$2\" = --version ]; then\n\
         \x20   echo 'cargo-llvm-cov {pin}'; exit 0\n\
         fi\n\
         if [ \"$1\" = llvm-cov ] && [ \"$2\" = nextest ]; then\n\
         \x20   printf '%s\\n' \"${{CARGO_LLVM_COV_SETUP-<unset>}}\" \"$@\" > '{record}'; exit 0\n\
         fi\n\
         exec '{real}' \"$@\"\n",
        real = real.display(),
        record = record.display(),
    )
}

/// `check.sh coverage` runs cargo-llvm-cov with `CARGO_LLVM_COV_SETUP=no`, set
/// by the script itself, so a missing llvm-tools component fails instead of
/// being installed: the variable is removed from the environment check.sh
/// starts in, and the stub must still see `no`.
#[test]
fn coverage_runs_cargo_llvm_cov_with_setup_disabled() {
    let root = fresh_dir("coverage-setup");
    let stub_dir = root.join("stub");
    fs::create_dir_all(&stub_dir).expect("create the stub dir");
    let record = root.join("record");
    let cargo = stub_dir.join("cargo");
    fs::write(
        &cargo,
        cargo_stub(&on_path("cargo"), &pinned_llvm_cov_version(), &record),
    )
    .expect("write the cargo stub");
    let llvm_cov = stub_dir.join("cargo-llvm-cov");
    fs::write(&llvm_cov, "#!/bin/bash\nexit 1\n").expect("write a cargo-llvm-cov stand-in");
    for file in [&cargo, &llvm_cov] {
        fs::set_permissions(file, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    }
    let tools = minimal_path(
        "coverage-setup-tools",
        &["bash", "rustc", "dirname", "cargo-nextest"],
    );
    let path = env::join_paths([&stub_dir, &tools]).expect("join PATH");

    let output = Command::new("/bin/bash")
        .arg(check_sh("coverage-setup-checkout"))
        .args(["coverage", "-E", "test(=some::test)"])
        .env("PATH", path)
        .env_remove("CARGO_LLVM_COV_SETUP")
        .output()
        .expect("spawn /bin/bash scripts/check.sh coverage");
    let stdout = text(output.stdout, "check.sh stdout");
    let stderr = text(output.stderr, "check.sh stderr");
    assert_eq!(
        output.status.code(),
        Some(0),
        "check.sh coverage with the stub; stdout: {stdout:?}; stderr: {stderr:?}"
    );
    let recorded = fs::read_to_string(&record).expect("the stub recorded its run");
    let lines: Vec<&str> = recorded.lines().collect();
    assert_eq!(
        lines.first(),
        Some(&"no"),
        "CARGO_LLVM_COV_SETUP as cargo-llvm-cov sees it; recorded: {recorded:?}"
    );
    assert_eq!(
        lines[1..],
        [
            "llvm-cov",
            "nextest",
            "--locked",
            "--workspace",
            "--no-fail-fast",
            "--no-tests=fail",
            "-E",
            "test(=some::test)"
        ],
        "the cargo-llvm-cov command line"
    );
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
        .arg(check_sh("cargo-resolution-checkout"))
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

/// A stand-in for the pinned nextest installed in a checkout's `.tools/bin`.
/// Version queries, direct or through cargo, go to the real pinned binary;
/// anything else stands in for a test run that runs nothing and exits 0.
fn repo_tools_nextest_stub(real: &Path) -> String {
    format!(
        "#!/bin/bash\n\
         if [ \"$1\" = --version ] || [ \"$2\" = --version ]; then exec '{}' \"$@\"; fi\n\
         echo \"REPO-TOOLS cargo-nextest ran: $*\"\n",
        real.display()
    )
}

/// A scratch checkout (see `scratch_checkout`) with a shadowing
/// `cargo-nextest`, a package manager's newer build, ahead of everything else
/// on PATH. Returns the checkout and that PATH.
fn shadowed_nextest_checkout(name: &str) -> (PathBuf, std::ffi::OsString) {
    let checkout = scratch_checkout(&format!("{name}-checkout"));
    let stub_dir = fresh_dir(&format!("{name}-shadow"));
    let real = on_path("cargo-nextest");
    let stub = stub_dir.join("cargo-nextest");
    fs::write(&stub, nextest_stub(&real)).expect("write shadowing cargo-nextest stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    let tools = minimal_path(
        &format!("{name}-tools"),
        &["bash", "cargo", "rustc", "dirname"],
    );
    let path = env::join_paths([&stub_dir, &tools]).expect("join PATH");
    (checkout, path)
}

fn checkout_check_sh_test(
    checkout: &Path,
    path: &std::ffi::OsStr,
) -> (Option<i32>, String, String) {
    // An empty CARGO_HOME: cargo puts $CARGO_HOME/bin first when it is absent
    // from PATH, so a runner's own installed nextest (the container's
    // /usr/local/cargo/bin) would otherwise beat both stand-ins.
    let cargo_home = checkout.join(".cargo-home");
    fs::create_dir_all(&cargo_home).expect("create the empty CARGO_HOME");
    // No inherited WSX_TOOLS_DIR: an outer check.sh run from a checkout with
    // .tools/bin exports it, and the scratch checkout's hints must not name
    // that checkout's directory.
    let output = Command::new("/bin/bash")
        .arg(checkout.join("scripts/check.sh"))
        .arg("test")
        .env("PATH", path)
        .env("CARGO_HOME", &cargo_home)
        .env_remove("WSX_TOOLS_DIR")
        .output()
        .expect("spawn /bin/bash scripts/check.sh test in the scratch checkout");
    (
        output.status.code(),
        text(output.stdout, "check.sh stdout"),
        text(output.stderr, "check.sh stderr"),
    )
}

#[test]
fn repo_tools_dir_wins_over_a_shadowing_nextest() {
    let (checkout, path) = shadowed_nextest_checkout("repo-tools-wins");
    let repo_tools = checkout.join(".tools/bin");
    fs::create_dir_all(&repo_tools).expect("create .tools/bin");
    let pinned = repo_tools.join("cargo-nextest");
    fs::write(&pinned, repo_tools_nextest_stub(&on_path("cargo-nextest")))
        .expect("write .tools/bin/cargo-nextest");
    fs::set_permissions(&pinned, fs::Permissions::from_mode(0o755)).expect("chmod pinned stub");

    let (code, stdout, stderr) = checkout_check_sh_test(&checkout, &path);

    assert_eq!(
        code,
        Some(0),
        "check.sh test must run the checkout's .tools/bin nextest; \
         stdout: {stdout:?}; stderr: {stderr:?}"
    );
    assert!(
        stdout.contains("REPO-TOOLS cargo-nextest ran: nextest run"),
        "the .tools/bin nextest must be the one cargo runs, got stdout {stdout:?}"
    );
    assert!(
        !stdout.contains("STUB cargo-nextest ran"),
        "the shadowing nextest must never run, got stdout {stdout:?}"
    );
}

#[test]
fn shadowing_nextest_is_refused_without_a_repo_tools_dir() {
    // The counterpart: the same checkout and PATH with no .tools/bin, so the
    // shadowing build is the one cargo would run and the pin check refuses it.
    let (checkout, path) = shadowed_nextest_checkout("repo-tools-absent");

    let (code, stdout, stderr) = checkout_check_sh_test(&checkout, &path);

    assert_eq!(
        code,
        Some(1),
        "check.sh test must refuse the shadowing nextest; stdout: {stdout:?}; stderr: {stderr:?}"
    );
    assert!(
        stderr.contains("reports 'cargo-nextest 0.0.0-stub'"),
        "the refusal must name the shadowing version, got stderr {stderr:?}"
    );
    assert!(
        !stdout.contains("STUB cargo-nextest ran"),
        "the shadowing nextest must never run, got stdout {stdout:?}"
    );
}

/// A stand-in for a stale nextest left in a checkout's `.tools/bin`: every
/// version query, direct or through cargo, reports a version that is not the
/// pin; anything else stands in for a test run that runs nothing and exits 0.
const STALE_NEXTEST_STUB: &str = "#!/bin/bash\n\
     if [ \"$1\" = --version ] || [ \"$2\" = --version ]; then\n\
     \x20   echo 'cargo-nextest 0.0.0-stale'; exit 0\n\
     fi\n\
     echo \"STALE cargo-nextest ran: $*\"\n";

#[test]
fn stale_repo_tools_nextest_hint_installs_into_repo_tools() {
    // A stale nextest in .tools/bin shadows any install into cargo's bin, so
    // the refusal's fix must install into .tools/bin, or following it changes
    // nothing.
    let (checkout, path) = shadowed_nextest_checkout("repo-tools-stale");
    let repo_tools = checkout.join(".tools/bin");
    fs::create_dir_all(&repo_tools).expect("create .tools/bin");
    let stale = repo_tools.join("cargo-nextest");
    fs::write(&stale, STALE_NEXTEST_STUB).expect("write .tools/bin/cargo-nextest");
    fs::set_permissions(&stale, fs::Permissions::from_mode(0o755)).expect("chmod stale stub");

    let (code, stdout, stderr) = checkout_check_sh_test(&checkout, &path);

    assert_eq!(
        code,
        Some(1),
        "check.sh test must refuse the stale .tools/bin nextest; \
         stdout: {stdout:?}; stderr: {stderr:?}"
    );
    assert!(
        stderr.contains("reports 'cargo-nextest 0.0.0-stale'"),
        "the refusal must name the stale version, got stderr {stderr:?}"
    );
    let hint = format!(
        "run WSX_TOOLS_DIR={} scripts/install-tools.sh install cargo-nextest",
        repo_tools.display()
    );
    assert!(
        stderr.contains(&hint),
        "the fix must install into .tools/bin ({hint:?}), got stderr {stderr:?}"
    );
    assert!(
        !stdout.contains("STALE cargo-nextest ran"),
        "the stale nextest must never run, got stdout {stdout:?}"
    );
}

#[test]
fn shadowing_nextest_hint_names_no_tools_dir_without_one() {
    // The counterpart: with no .tools/bin, cargo's bin is the right place to
    // install, so the fix is the plain install command and names no .tools.
    let (checkout, path) = shadowed_nextest_checkout("repo-tools-absent-hint");

    let (code, stdout, stderr) = checkout_check_sh_test(&checkout, &path);

    assert_eq!(
        code,
        Some(1),
        "check.sh test must refuse the shadowing nextest; stdout: {stdout:?}; stderr: {stderr:?}"
    );
    assert!(
        stderr.contains("; run scripts/install-tools.sh install cargo-nextest"),
        "the fix must be the plain install command, got stderr {stderr:?}"
    );
    assert!(
        !stderr.contains(".tools"),
        "the fix must not name a .tools directory, got stderr {stderr:?}"
    );
}

/// A scratch checkout whose `.tools/bin` holds a nextest that reports the
/// pin, and whose `CARGO_HOME` (`<checkout>/.cargo-home`, the one
/// `checkout_check_sh_test` passes) holds a stale one in `bin`, with a PATH of
/// the plain tools only: `$CARGO_HOME/bin` is not on it, so cargo searches
/// that directory first (cargo#11020). Returns the checkout, that
/// `$CARGO_HOME/bin` and the PATH.
fn stale_cargo_home_checkout(name: &str) -> (PathBuf, PathBuf, std::ffi::OsString) {
    let checkout = scratch_checkout(&format!("{name}-checkout"));
    let repo_tools = checkout.join(".tools/bin");
    let cargo_home_bin = checkout.join(".cargo-home/bin");
    for dir in [&repo_tools, &cargo_home_bin] {
        fs::create_dir_all(dir).unwrap_or_else(|err| panic!("create {}: {err}", dir.display()));
    }
    let pinned = repo_tools.join("cargo-nextest");
    fs::write(&pinned, repo_tools_nextest_stub(&on_path("cargo-nextest")))
        .expect("write .tools/bin/cargo-nextest");
    fs::set_permissions(&pinned, fs::Permissions::from_mode(0o755)).expect("chmod pinned stub");
    let stale = cargo_home_bin.join("cargo-nextest");
    fs::write(&stale, STALE_NEXTEST_STUB).expect("write $CARGO_HOME/bin/cargo-nextest");
    fs::set_permissions(&stale, fs::Permissions::from_mode(0o755)).expect("chmod stale stub");
    let tools = minimal_path(
        &format!("{name}-tools"),
        &["bash", "cargo", "rustc", "dirname"],
    );
    (checkout, cargo_home_bin, tools.into_os_string())
}

#[test]
fn stale_cargo_home_nextest_hint_installs_into_cargo_home() {
    // .tools/bin already holds the pin, so installing there again changes
    // nothing: cargo ran the stale copy in $CARGO_HOME/bin, which it searches
    // before PATH while that directory is not on PATH. The fix must name that
    // directory and be the plain install command, which installs there.
    let (checkout, cargo_home_bin, path) = stale_cargo_home_checkout("cargo-home-stale");

    let (code, stdout, stderr) = checkout_check_sh_test(&checkout, &path);

    assert_eq!(
        code,
        Some(1),
        "check.sh test must refuse the stale $CARGO_HOME/bin nextest; \
         stdout: {stdout:?}; stderr: {stderr:?}"
    );
    assert!(
        stderr.contains("reports 'cargo-nextest 0.0.0-stale'"),
        "the refusal must name the stale version, got stderr {stderr:?}"
    );
    assert!(
        stderr.contains(&cargo_home_bin.display().to_string()),
        "the refusal must name $CARGO_HOME/bin ({}), got stderr {stderr:?}",
        cargo_home_bin.display()
    );
    assert!(
        stderr.contains("; run scripts/install-tools.sh install cargo-nextest"),
        "the fix must be the plain install command, got stderr {stderr:?}"
    );
    assert!(
        !stderr.contains("WSX_TOOLS_DIR="),
        "the fix must not install into .tools/bin, got stderr {stderr:?}"
    );
    assert!(
        !stdout.contains("ran: nextest run"),
        "no nextest may run after a refused pin, got stdout {stdout:?}"
    );
}

#[test]
fn repo_tools_nextest_wins_when_cargo_home_bin_is_on_path() {
    // The counterpart: the same checkout with $CARGO_HOME/bin on PATH, after
    // the plain tools. cargo then follows PATH order, check.sh's .tools/bin
    // comes first, and its pinned nextest is the one that runs.
    let (checkout, cargo_home_bin, tools) = stale_cargo_home_checkout("cargo-home-on-path");
    let path = env::join_paths([PathBuf::from(tools), cargo_home_bin]).expect("join PATH");

    let (code, stdout, stderr) = checkout_check_sh_test(&checkout, &path);

    assert_eq!(
        code,
        Some(0),
        "check.sh test must run the .tools/bin nextest; stdout: {stdout:?}; stderr: {stderr:?}"
    );
    assert!(
        stdout.contains("REPO-TOOLS cargo-nextest ran: nextest run"),
        "the .tools/bin nextest must be the one cargo runs, got stdout {stdout:?}"
    );
    assert!(
        !stdout.contains("STALE cargo-nextest ran"),
        "the stale nextest must never run, got stdout {stdout:?}"
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

    // This checkout's own check.sh, not a scratch copy: the nested run needs
    // the real manifest, and it keeps the caller's PATH.
    let gated = nested_nextest_command("/bin/bash", &target_dir)
        .arg(Path::new(REPO_ROOT).join("scripts/check.sh"))
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

/// `program` with every inherited `GIT_*` variable removed and no global or
/// system git config, so only the fixture repositories decide what git sees.
fn without_git_env(program: &str) -> Command {
    let mut command = Command::new(program);
    for (key, _) in env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(&key);
        }
    }
    command
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    command
}

/// A fresh git repository at `repo` whose Dockerfile pins `image`.
fn fixture_repo(repo: &Path, image: &str) {
    let devcontainer = repo.join(".devcontainer");
    fs::create_dir_all(&devcontainer)
        .unwrap_or_else(|err| panic!("create {}: {err}", devcontainer.display()));
    fs::write(devcontainer.join("Dockerfile"), format!("FROM {image}\n"))
        .expect("write fixture Dockerfile");
    let output = without_git_env("git")
        .args(["init", "--quiet"])
        .arg(repo)
        .output()
        .expect("spawn git init");
    assert_eq!(
        output.status.code(),
        Some(0),
        "git init {} exit code; stderr: {:?}",
        repo.display(),
        text(output.stderr, "git init stderr")
    );
}

/// The image a `container_gate_args` fixture's own Dockerfile pins.
const OWN_IMAGE: &str = "example/own:1.0.0-1-trixie";

/// Runs a copy of `check-in-container.sh fmt` from a caller repository, with
/// a stub docker that records its arguments, and returns the canonical root
/// of the checkout holding the copy and the recorded arguments, one per line.
/// With `tools_dir`, that checkout holds a `.tools/bin` directory, as after
/// `WSX_TOOLS_DIR=.tools/bin scripts/install-tools.sh install ...`.
fn container_gate_args(name: &str, tools_dir: bool) -> (PathBuf, String) {
    // Fixture repositories rather than this checkout: they are owned by the
    // test's user, so git trusts them inside the container as well.
    let root = fresh_dir(name);
    let own = root.join("own");
    let caller = root.join("caller");
    fixture_repo(&own, OWN_IMAGE);
    fixture_repo(&caller, "example/caller:1.0.0-1-trixie");
    if tools_dir {
        fs::create_dir_all(own.join(".tools/bin")).expect("create the fixture .tools/bin");
    }
    let script = own.join("scripts/check-in-container.sh");
    fs::create_dir_all(own.join("scripts")).expect("create fixture scripts dir");
    fs::copy(
        Path::new(REPO_ROOT).join("scripts/check-in-container.sh"),
        &script,
    )
    .expect("copy check-in-container.sh");

    // A docker that records its arguments, one per line, and succeeds.
    let stub_dir = root.join("stub");
    fs::create_dir_all(&stub_dir).expect("create stub dir");
    let stub = stub_dir.join("docker");
    fs::write(
        &stub,
        "#!/bin/bash\nprintf '%s\\n' \"$@\" >>\"$DOCKER_LOG\"\n",
    )
    .expect("write docker stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    let log = root.join("docker.log");
    let inherited = env::var_os("PATH").expect("PATH is set for the test process");
    let path = env::join_paths(std::iter::once(stub_dir).chain(env::split_paths(&inherited)))
        .expect("join PATH");

    let output = without_git_env("/bin/bash")
        .arg(&script)
        .arg("fmt")
        .current_dir(&caller)
        .env("PATH", path)
        .env("DOCKER_LOG", &log)
        .output()
        .expect("spawn /bin/bash check-in-container.sh");
    assert_eq!(
        output.status.code(),
        Some(0),
        "check-in-container.sh with a stub docker exit code; stderr: {:?}",
        text(output.stderr, "check-in-container.sh stderr")
    );

    let args = fs::read_to_string(&log).expect("read docker stub log");
    let own_root = fs::canonicalize(&own).expect("canonicalize fixture repo");
    (own_root, args)
}

#[test]
fn container_gate_mounts_its_own_checkout_not_the_callers() {
    let (own_root, args) = container_gate_args("container-gate", false);
    let mount = args
        .lines()
        .find_map(|arg| arg.strip_suffix(":/workspace:ro"))
        .unwrap_or_else(|| panic!("no read-only /workspace mount in docker args {args:?}"));
    assert_eq!(
        Path::new(mount),
        own_root,
        "the gate must mount the checkout holding the script, not the caller's"
    );
    assert!(
        args.lines().any(|arg| arg == OWN_IMAGE),
        "the image must come from the script's own Dockerfile, got docker args {args:?}"
    );
}

/// The `--tmpfs` mount that hides the checkout's `.tools` in the container.
const TOOLS_MASK: [&str; 2] = ["--tmpfs", "/workspace/.tools:ro"];

/// Whether the recorded docker `args` hold `TOOLS_MASK` as consecutive
/// arguments.
fn masks_tools(args: &str) -> bool {
    args.lines()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| pair == TOOLS_MASK)
}

#[test]
fn container_gate_masks_a_checkout_tools_dir() {
    // check.sh puts /workspace/.tools/bin first on PATH, so without the mask
    // the container would run the host's tools ahead of the pinned,
    // checksum-verified builds it installs.
    let (_own_root, args) = container_gate_args("container-gate-tools", true);
    assert!(
        masks_tools(&args),
        "a checkout with .tools must get {TOOLS_MASK:?}, got docker args {args:?}"
    );
}

#[test]
fn container_gate_adds_no_tools_mask_without_a_tools_dir() {
    // The counterpart: docker cannot create a mount point inside the
    // read-only bind, so a checkout without .tools gets no mask.
    let (_own_root, args) = container_gate_args("container-gate-no-tools", false);
    assert!(
        !args.lines().any(|arg| arg.contains("/workspace/.tools")),
        "a checkout without .tools must get no .tools mount, got docker args {args:?}"
    );
}

/// Runs git in `repo` with no inherited git environment or config, a fixed
/// identity and no signing, and requires it to succeed.
fn git_in(repo: &Path, args: &[&str]) {
    let output = without_git_env("git")
        .current_dir(repo)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("spawn git");
    assert_eq!(
        output.status.code(),
        Some(0),
        "git {args:?} in {}; stderr: {:?}",
        repo.display(),
        text(output.stderr, "git stderr")
    );
}

/// A minimal crate, rustfmt-clean, with copies of `scripts/check.sh` and
/// `rust-toolchain.toml`, so `check.sh fmt` runs on it. With `commit`, it is
/// a git repository holding one commit of every file.
fn fmt_fixture(name: &str, commit: bool) -> PathBuf {
    let repo = fresh_dir(name);
    fs::create_dir_all(repo.join("src")).expect("create fixture src");
    fs::create_dir_all(repo.join("scripts")).expect("create fixture scripts");
    fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"fmt-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
         [workspace]\n",
    )
    .expect("write fixture Cargo.toml");
    fs::write(repo.join("src/main.rs"), "fn main() {}\n").expect("write fixture main.rs");
    fs::write(repo.join("src/gone.rs"), "pub fn gone() {}\n").expect("write fixture gone.rs");
    for file in ["scripts/check.sh", "rust-toolchain.toml"] {
        fs::copy(Path::new(REPO_ROOT).join(file), repo.join(file))
            .unwrap_or_else(|err| panic!("copy {file} into the fixture: {err}"));
    }
    if commit {
        git_in(&repo, &["init", "--quiet"]);
        git_in(&repo, &["add", "--all"]);
        git_in(&repo, &["commit", "--quiet", "-m", "fixture"]);
    }
    repo
}

/// `check.sh fmt` of the fixture at `repo`, with git stopped from searching
/// above it, so it never finds this checkout around `target/`.
fn fixture_fmt(repo: &Path) -> (Option<i32>, String, String) {
    let ceiling = repo.parent().expect("the fixture has a parent");
    let output = without_git_env("/bin/bash")
        .arg(repo.join("scripts/check.sh"))
        .arg("fmt")
        .env("GIT_CEILING_DIRECTORIES", ceiling)
        .output()
        .expect("spawn the fixture's check.sh fmt");
    (
        output.status.code(),
        text(output.stdout, "check.sh stdout"),
        text(output.stderr, "check.sh stderr"),
    )
}

#[test]
fn missing_git_fails_fmt_by_name() {
    assert_missing_tool_named("fmt", &["dirname", "cargo", "rustc", "cargo-fmt"], "git");
}

/// Outside a git work tree, such as a `git archive` snapshot, fmt says it
/// needs one instead of failing on git's own message.
#[test]
fn fmt_refuses_outside_a_git_work_tree() {
    let repo = fmt_fixture("fmt-no-work-tree", false);
    let (code, stdout, stderr) = fixture_fmt(&repo);
    assert_eq!(code, Some(1), "stdout: {stdout:?}; stderr: {stderr:?}");
    assert!(
        stderr.contains(
            "'check.sh fmt' lists the .rs files with git, so it must run in a git work tree"
        ),
        "the refusal must say fmt needs a work tree, got {stderr:?}"
    );
    assert!(!stdout.contains("fmt OK"), "got stdout {stdout:?}");
}

/// A tracked .rs file deleted from disk but not staged is skipped, not handed
/// to rustfmt; the rest of the fixture is clean, so fmt passes.
#[test]
fn fmt_skips_a_tracked_file_deleted_from_disk() {
    let repo = fmt_fixture("fmt-deleted-file", true);
    fs::remove_file(repo.join("src/gone.rs")).expect("delete gone.rs from disk");
    let (code, stdout, stderr) = fixture_fmt(&repo);
    assert_eq!(code, Some(0), "stdout: {stdout:?}; stderr: {stderr:?}");
    assert!(stdout.contains("check.sh: fmt OK"), "got stdout {stdout:?}");
}

/// An untracked .rs file that no `mod` reaches, as a new scenario module is
/// before `git add`, is still checked: a misformatted one fails fmt.
#[test]
fn fmt_checks_an_untracked_rs_file() {
    let repo = fmt_fixture("fmt-untracked-file", true);
    fs::write(repo.join("src/extra.rs"), "pub fn  extra( ) {}\n").expect("write extra.rs");
    let (code, stdout, stderr) = fixture_fmt(&repo);
    assert_ne!(code, Some(0), "stdout: {stdout:?}; stderr: {stderr:?}");
    assert!(
        stdout.contains("src/extra.rs"),
        "rustfmt's diff must name src/extra.rs; stdout: {stdout:?}; stderr: {stderr:?}"
    );
    assert!(!stdout.contains("fmt OK"), "got stdout {stdout:?}");
}
