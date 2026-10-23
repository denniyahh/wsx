//! Gate boundary tests: `scripts/check.sh test` must refuse every nextest
//! option that could let it print "OK" without running the suite. Each case
//! is refused before any tool runs, so these tests never start a nested build.

use std::process::{Command, Output};

/// Exit code check.sh uses for a refused invocation.
const USAGE_EXIT: i32 = 2;

fn check_sh_test(args: &[&str]) -> Output {
    Command::new(concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/check.sh"))
        .arg("test")
        .args(args)
        .output()
        .expect("spawn scripts/check.sh")
}

fn assert_refused(args: &[&str], expected_stderr: &str) {
    let output = check_sh_test(args);
    let stdout = String::from_utf8(output.stdout).expect("check.sh stdout is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("check.sh stderr is UTF-8");

    assert_eq!(
        output.status.code(),
        Some(USAGE_EXIT),
        "check.sh test {args:?} exit code; stderr: {stderr:?}"
    );
    assert!(
        stderr.contains(expected_stderr),
        "check.sh test {args:?} stderr must contain {expected_stderr:?}, got {stderr:?}"
    );
    assert!(
        !stdout.contains("OK"),
        "check.sh test {args:?} must not report success, got stdout {stdout:?}"
    );
}

#[test]
fn help_flag_is_refused() {
    assert_refused(&["--help"], "not '--help'");
    assert_refused(&["-h"], "not '-h'");
}

#[test]
fn no_run_is_refused() {
    assert_refused(&["--no-run"], "not '--no-run'");
}

#[test]
fn no_tests_override_is_refused() {
    assert_refused(&["--no-tests=pass"], "--no-tests is fixed");
    assert_refused(&["--no-tests", "pass"], "--no-tests is fixed");
}

#[test]
fn option_after_filterset_is_refused() {
    assert_refused(&["-E", "test(=a)", "--no-run"], "not '--no-run'");
}

#[test]
fn filterset_flag_without_expression_is_refused() {
    assert_refused(&["-E"], "-E needs a filterset expression");
}

/// A second selector is refused: nextest ORs selectors, so one that matches
/// nothing would pass on the other's tests.
const SECOND_SELECTOR: &str = "takes at most one -E/--filterset expression or test name";

#[test]
fn second_filterset_is_refused() {
    assert_refused(
        &["-E", "test(=a)", "-E", "test(=no_such_xyz)"],
        SECOND_SELECTOR,
    );
    assert_refused(
        &["--filterset=test(=a)", "--filterset", "test(=no_such_xyz)"],
        SECOND_SELECTOR,
    );
}

#[test]
fn second_test_name_is_refused() {
    // Names that match no test: a name filter is a substring match, so if
    // the refusal regressed, a short one would run this test inside itself.
    assert_refused(&["no_such_xyz_one", "no_such_xyz_two"], SECOND_SELECTOR);
    assert_refused(
        &["-E", "test(=no_such_xyz_one)", "no_such_xyz_two"],
        SECOND_SELECTOR,
    );
}
