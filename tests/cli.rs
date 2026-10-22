//! CLI boundary tests: the binary's observable output (stdout, stderr and exit
//! code) is the contract under test, not any function inside it. Each test
//! spawns the real `wsx` executable that cargo built for this test run.

use std::process::{Command, Output};

/// Path to the `wsx` binary cargo built alongside these tests.
fn wsx_bin() -> &'static str {
    env!("CARGO_BIN_EXE_wsx")
}

fn run_wsx(args: &[&str]) -> Output {
    Command::new(wsx_bin())
        .args(args)
        .output()
        .expect("spawn the wsx binary")
}

#[test]
fn version_flag_prints_package_version() {
    let output = run_wsx(&["--version"]);

    let stdout = String::from_utf8(output.stdout).expect("wsx stdout is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("wsx stderr is UTF-8");
    let expected = format!("wsx {}\n", env!("CARGO_PKG_VERSION"));

    assert_eq!(stdout, expected, "`wsx --version` stdout");
    assert_eq!(stderr, "", "`wsx --version` must not write to stderr");
    assert_eq!(output.status.code(), Some(0), "`wsx --version` exit code");
}

#[test]
fn missing_argument_exits_with_usage() {
    let output = run_wsx(&[]);

    let stdout = String::from_utf8(output.stdout).expect("wsx stdout is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("wsx stderr is UTF-8");

    assert!(
        stderr.contains("usage: wsx --version"),
        "`wsx` with no argument must print usage on stderr, got {stderr:?}"
    );
    assert_eq!(stdout, "", "`wsx` with no argument must not write to stdout");
    assert_eq!(output.status.code(), Some(2), "`wsx` with no argument exit code");
}
