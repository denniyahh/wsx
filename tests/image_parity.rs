//! `scripts/assert-image-parity.sh` boundary tests: every CI job must run in
//! the image the devcontainer Dockerfile pins (DEV-06).
//!
//! Each case copies the script into its own tree under CARGO_TARGET_TMPDIR
//! with a fixture Dockerfile and workflow files. The script finds the
//! repository root from its own location, so the copy reads only the fixture.
//! The real tree is checked in place as the positive case.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");

/// The fixture Dockerfile's pin, independent of the real one so a Dependabot
/// bump of the real Dockerfile never touches these cases.
const PIN: &str = "mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie";
const DRIFTED: &str = "mcr.microsoft.com/devcontainers/rust:2.1.2-1-trixie";

/// Exit code the script uses for every refusal except a usage error.
const REFUSED_EXIT: i32 = 1;

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_parity(root: &Path) -> Run {
    let output = Command::new("/bin/bash")
        .arg(root.join("scripts/assert-image-parity.sh"))
        .output()
        .expect("spawn /bin/bash assert-image-parity.sh");
    Run {
        code: output.status.code(),
        stdout: String::from_utf8(output.stdout).expect("parity stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("parity stderr is UTF-8"),
    }
}

/// A tree holding the real script, a Dockerfile pinning PIN and `workflows`.
fn fixture_tree(name: &str, workflows: &[(&str, &str)]) -> PathBuf {
    let root = Path::new(TMP_ROOT).join("image-parity").join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove stale fixture tree");
    }
    for dir in ["scripts", ".devcontainer", ".github/workflows"] {
        fs::create_dir_all(root.join(dir)).expect("create fixture dir");
    }
    fs::copy(
        Path::new(REPO_ROOT).join("scripts/assert-image-parity.sh"),
        root.join("scripts/assert-image-parity.sh"),
    )
    .expect("copy assert-image-parity.sh");
    fs::write(
        root.join(".devcontainer/Dockerfile"),
        format!("# fixture pin\nFROM {PIN}\n"),
    )
    .expect("write fixture Dockerfile");
    for (file, contents) in workflows {
        fs::write(root.join(".github/workflows").join(file), contents).expect("write workflow");
    }
    root
}

/// A workflow whose `jobs:` block is `jobs`, indented by two spaces.
fn workflow(jobs: &str) -> String {
    format!("name: fixture\non: push\n\njobs:\n{jobs}")
}

fn assert_refused(run: &Run, case: &str) {
    assert_eq!(
        run.code,
        Some(REFUSED_EXIT),
        "{case}: exit code; stdout: {:?}; stderr: {:?}",
        run.stdout,
        run.stderr
    );
    assert!(
        !run.stdout.contains("OK"),
        "{case}: a refusal must not report success, got stdout {:?}",
        run.stdout
    );
}

fn assert_accepted(run: &Run, case: &str, images: usize) {
    assert_eq!(
        run.code,
        Some(0),
        "{case}: exit code; stderr: {:?}",
        run.stderr
    );
    let ok = format!("image parity OK: {PIN} ({images} job container image(s))");
    assert!(
        run.stdout.contains(&ok),
        "{case}: stdout must contain {ok:?}, got {:?}",
        run.stdout
    );
}

#[test]
fn matching_job_image_passes() {
    let ci = workflow(&format!(
        "  test:\n    runs-on: ubuntu-24.04\n    container:\n      image: {PIN}\n\
         \x20   steps:\n      - run: true\n"
    ));
    let run = run_parity(&fixture_tree("matching", &[("ci.yml", &ci)]));
    assert_accepted(&run, "matching pin", 1);
}

#[test]
fn drifted_job_image_fails_naming_both_tags() {
    let ci = workflow(&format!(
        "  test:\n    runs-on: ubuntu-24.04\n    container:\n      image: {PIN}\n\
         \x20   steps:\n      - run: true\n"
    ));
    let drift = workflow(&format!(
        "  deny:\n    runs-on: ubuntu-24.04\n    container:\n      image: {DRIFTED}\n\
         \x20   steps:\n      - run: true\n"
    ));
    let run = run_parity(&fixture_tree(
        "drifted",
        &[("ci.yml", &ci), ("drift.yml", &drift)],
    ));
    assert_refused(&run, "drifted tag");
    for needle in [PIN, DRIFTED, "drift.yml: job 'deny'"] {
        assert!(
            run.stderr.contains(needle),
            "drifted tag: stderr must name {needle:?}, got {:?}",
            run.stderr
        );
    }
    assert!(
        !run.stderr.contains("job 'test'"),
        "drifted tag: the matching job must not be reported, got {:?}",
        run.stderr
    );
}

#[test]
fn bare_job_with_pinned_service_image_fails_naming_the_job() {
    // Regression for fe762f4: a service container's `image:` used the pin, so
    // a job running on the bare runner passed the old line-counting check.
    let ci = workflow(&format!(
        "  lint:\n    runs-on: ubuntu-24.04\n    services:\n      cache:\n        image: {PIN}\n\
         \x20   steps:\n      - run: true\n"
    ));
    let run = run_parity(&fixture_tree("service-only", &[("ci.yml", &ci)]));
    assert_refused(&run, "service image only");
    assert!(
        run.stderr.contains("ci.yml: job 'lint'"),
        "service image only: stderr must name job 'lint', got {:?}",
        run.stderr
    );
}

#[test]
fn second_bare_job_fails_naming_only_that_job() {
    let ci = workflow(&format!(
        "  test:\n    runs-on: ubuntu-24.04\n    container:\n      image: {PIN}\n\
         \x20   steps:\n      - run: true\n\
         \x20 extra:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: true\n"
    ));
    let run = run_parity(&fixture_tree("second-bare", &[("ci.yml", &ci)]));
    assert_refused(&run, "second bare job");
    assert!(
        run.stderr.contains("ci.yml: job 'extra'"),
        "second bare job: stderr must name job 'extra', got {:?}",
        run.stderr
    );
    assert!(
        !run.stderr.contains("job 'test'"),
        "second bare job: the containerised job must not be reported, got {:?}",
        run.stderr
    );
}

#[test]
fn shorthand_and_quoted_images_pass() {
    let ci = workflow(&format!(
        "  short:\n    runs-on: ubuntu-24.04\n    container: {PIN}\n\
         \x20   steps:\n      - run: true\n\
         \x20 double:\n    runs-on: ubuntu-24.04\n    container:\n      image: \"{PIN}\" # pin\n\
         \x20   steps:\n      - run: true\n\
         \x20 single:\n    runs-on: ubuntu-24.04\n    container: '{PIN}'\n\
         \x20   steps:\n      - run: true\n"
    ));
    let run = run_parity(&fixture_tree("shorthand-quoted", &[("ci.yml", &ci)]));
    assert_accepted(&run, "shorthand and quoted", 3);
}

#[test]
fn repository_workflows_match_the_devcontainer_pin() {
    let run = run_parity(Path::new(REPO_ROOT));
    assert_eq!(
        run.code,
        Some(0),
        "real tree: exit code; stderr: {:?}",
        run.stderr
    );
    assert!(
        run.stdout.starts_with("image parity OK: "),
        "real tree: stdout must report parity, got {:?}",
        run.stdout
    );
}
