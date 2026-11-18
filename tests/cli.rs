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
    assert_eq!(
        stdout, "",
        "`wsx` with no argument must not write to stdout"
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "`wsx` with no argument exit code"
    );
}

#[test]
fn check_with_an_extra_argument_exits_with_usage() {
    let output = run_wsx(&["check", "extra"]);

    let stdout = String::from_utf8(output.stdout).expect("wsx stdout is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("wsx stderr is UTF-8");

    assert!(
        stderr.contains("usage: wsx check"),
        "`wsx check extra` must print usage on stderr, got {stderr:?}"
    );
    assert_eq!(stdout, "", "`wsx check extra` must not write to stdout");
    assert_eq!(output.status.code(), Some(2), "`wsx check extra` exit code");
}

/// `output`'s stdout as one JSON object followed by one newline.
fn one_object(output: &Output) -> serde_json::Map<String, serde_json::Value> {
    let shown = output.stdout.escape_ascii().to_string();
    assert!(
        output.stdout.ends_with(b"\n")
            && output.stdout.iter().filter(|byte| **byte == b'\n').count() == 1,
        "stdout is not one line ending in a newline: {shown}"
    );
    match serde_json::from_slice(&output.stdout) {
        Ok(serde_json::Value::Object(object)) => object,
        Ok(other) => panic!("stdout is JSON but not an object: {other}"),
        Err(err) => panic!("stdout is not one JSON value ({err}): {shown}"),
    }
}

/// `object` is check's usage failure: `error.kind` usage, exit code 2.
fn assert_usage_object(object: &serde_json::Map<String, serde_json::Value>) {
    assert_eq!(object["command"], serde_json::json!("check"));
    assert_eq!(object["schema_version"], serde_json::json!(1));
    assert_eq!(object["error"]["kind"], serde_json::json!("usage"));
    assert_eq!(object["error"]["exit_code"], serde_json::json!(2));
}

#[test]
fn check_json_usage_error_is_one_object() {
    let output = run_wsx(&["check", "--bogus", "--json"]);

    let stderr = String::from_utf8(output.stderr.clone()).expect("wsx stderr is UTF-8");
    assert_eq!(output.status.code(), Some(2), "exit code, stderr: {stderr}");
    assert_usage_object(&one_object(&output));
    assert!(
        stderr.contains("usage: wsx check [--json]"),
        "usage lines on stderr, got {stderr:?}"
    );
}

#[test]
fn unknown_command_with_json_prints_no_json() {
    let output = run_wsx(&["bogus", "--json"]);

    let stdout = String::from_utf8(output.stdout).expect("wsx stdout is UTF-8");
    assert_eq!(stdout, "", "an unknown command has no JSON contract");
    assert_eq!(
        output.status.code(),
        Some(2),
        "`wsx bogus --json` exit code"
    );
}

#[test]
fn unknown_command_is_named_c_quoted_on_stderr() {
    let output = run_wsx(&["bo\tgus"]);

    let stderr = String::from_utf8(output.stderr).expect("wsx stderr is UTF-8");
    assert!(
        stderr.contains(r#"unknown command "bo\tgus""#),
        "the refusal names the command C-quoted, got {stderr:?}"
    );
    assert_eq!(output.status.code(), Some(2), "`wsx bo<TAB>gus` exit code");
}

#[test]
fn duplicate_json_flag_is_a_usage_error() {
    let output = run_wsx(&["check", "--json", "--json"]);

    assert_eq!(
        output.status.code(),
        Some(2),
        "`wsx check --json --json` exit code"
    );
    assert_usage_object(&one_object(&output));
}
