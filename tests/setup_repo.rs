//! `scripts/setup-repo.sh` copies the tracked git policy into local config
//! instead of including it (DEV-05), so a branch's `.gitconfig` is never read
//! live and can never set a key outside the policy.
//!
//! Each test copies the script and `.gitconfig` into a fresh repository under
//! CARGO_TARGET_TMPDIR, because the script always configures the checkout that
//! holds it. git runs with no global or system config and a scratch HOME.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");

/// Exit code for a refusal or, under `check`, drift.
const FAILURE_EXIT: i32 = 1;
/// Exit code for a usage error.
const USAGE_EXIT: i32 = 2;
/// The include clones carried before the policy was copied.
const LEGACY_INCLUDE: &str = "../.gitconfig";

fn text(bytes: Vec<u8>, what: &str) -> String {
    String::from_utf8(bytes).unwrap_or_else(|err| panic!("{what} is not UTF-8: {err}"))
}

/// A command that sees no inherited `GIT_*` variable and no global or system
/// config. `stub_bin` is put first on PATH.
fn hermetic(program: &str, dir: &Path, stub_bin: &Path) -> Command {
    let home = dir.join("home");
    fs::create_dir_all(&home).expect("create scratch HOME");
    let path = std::env::var_os("PATH").expect("PATH is set");
    let mut paths = vec![stub_bin.to_path_buf()];
    paths.extend(std::env::split_paths(&path));
    let mut command = Command::new(program);
    command
        .current_dir(dir.join("repo"))
        .env_clear()
        .env("PATH", std::env::join_paths(paths).expect("join PATH"))
        .env("HOME", &home)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("LC_ALL", "C");
    command
}

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    /// A fresh repository holding a copy of the script and of `policy`.
    fn new(name: &str, policy: &str) -> Self {
        let dir = Path::new(TMP_ROOT).join(format!("setup-repo-{name}"));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove old fixture");
        }
        let scripts = dir.join("repo/scripts");
        fs::create_dir_all(&scripts).expect("create fixture scripts dir");
        fs::create_dir_all(dir.join("bin")).expect("create stub bin dir");
        fs::copy(
            Path::new(REPO_ROOT).join("scripts/setup-repo.sh"),
            scripts.join("setup-repo.sh"),
        )
        .expect("copy setup-repo.sh");
        fs::write(dir.join("repo/.gitconfig"), policy).expect("write fixture .gitconfig");
        let fixture = Self { dir };
        fixture.git_ok(&["init", "--quiet"]);
        fixture
    }

    fn tracked_policy() -> String {
        fs::read_to_string(Path::new(REPO_ROOT).join(".gitconfig")).expect("read .gitconfig")
    }

    /// A gitleaks stand-in: the script only checks that one is on PATH.
    fn stub_gitleaks(&self) {
        let stub = self.dir.join("bin/gitleaks");
        fs::write(&stub, "#!/bin/sh\nexit 0\n").expect("write gitleaks stub");
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    }

    fn setup(&self, args: &[&str]) -> Output {
        hermetic("bash", &self.dir, &self.dir.join("bin"))
            .arg("scripts/setup-repo.sh")
            .args(args)
            .output()
            .expect("spawn setup-repo.sh")
    }

    fn git(&self, args: &[&str]) -> Output {
        hermetic("git", &self.dir, &self.dir.join("bin"))
            .args(args)
            .output()
            .expect("spawn git")
    }

    fn git_ok(&self, args: &[&str]) -> String {
        let output = self.git(args);
        let stderr = text(output.stderr, "git stderr");
        assert_eq!(
            output.status.code(),
            Some(0),
            "git {args:?}; stderr: {stderr:?}"
        );
        text(output.stdout, "git stdout")
    }

    /// Every `key=value` in the repository's local config, as git reports it.
    fn local_config(&self) -> Vec<String> {
        self.git_ok(&["config", "--local", "--list"])
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

/// Every `key=value` the tracked .gitconfig sets, as git parses it.
fn tracked_policy_entries(fixture: &Fixture) -> Vec<String> {
    fixture
        .git_ok(&["config", "--no-includes", "-f", ".gitconfig", "--list"])
        .lines()
        .map(str::to_owned)
        .collect()
}

fn assert_exit(output: &Output, expected: i32, what: &str) -> (String, String) {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{what}: stdout {stdout:?}, stderr {stderr:?}"
    );
    (stdout, stderr)
}

#[test]
fn policy_copies_every_tracked_key_and_adds_no_include() {
    let fixture = Fixture::new("copy", &Fixture::tracked_policy());
    let expected = tracked_policy_entries(&fixture);
    assert!(!expected.is_empty(), "tracked .gitconfig sets no keys");

    assert_exit(&fixture.setup(&["policy"]), 0, "setup-repo.sh policy");

    let local = fixture.local_config();
    for entry in &expected {
        assert!(
            local.contains(entry),
            "{entry} missing from local config {local:#?}"
        );
    }
    assert!(
        !local.iter().any(|entry| entry.starts_with("include.path=")),
        "policy must not add an include: {local:#?}"
    );
    assert!(
        !local.iter().any(|entry| entry.starts_with("hook.")),
        "policy must not register the hook: {local:#?}"
    );
}

#[test]
fn rerunning_policy_changes_nothing() {
    let fixture = Fixture::new("idempotent", &Fixture::tracked_policy());
    assert_exit(&fixture.setup(&["policy"]), 0, "first run");
    let first = fixture.local_config();
    assert_exit(&fixture.setup(&["policy"]), 0, "second run");
    assert_eq!(
        fixture.local_config(),
        first,
        "second run changed local config"
    );
}

#[test]
fn policy_removes_the_legacy_include_and_keeps_others() {
    let fixture = Fixture::new("legacy", &Fixture::tracked_policy());
    fixture.git_ok(&["config", "--local", "--add", "include.path", "other.inc"]);
    fixture.git_ok(&["config", "--local", "--add", "include.path", LEGACY_INCLUDE]);

    let (stdout, _) = assert_exit(&fixture.setup(&["policy"]), 0, "setup-repo.sh policy");
    assert!(
        stdout.contains("removed include.path"),
        "stdout: {stdout:?}"
    );

    let includes = fixture.git_ok(&["config", "--local", "--get-all", "include.path"]);
    assert_eq!(
        includes, "other.inc\n",
        "only the legacy include may be removed"
    );
}

#[test]
fn key_outside_the_policy_is_refused_before_anything_is_written() {
    let marker = Path::new(TMP_ROOT).join("setup-repo-refuse/fsmonitor-ran");
    let policy = format!(
        "{}\n[core]\n    fsmonitor = touch {}\n",
        Fixture::tracked_policy(),
        marker.display()
    );
    let fixture = Fixture::new("refuse", &policy);
    let before = fixture.local_config();

    let (_, stderr) = assert_exit(&fixture.setup(&["policy"]), FAILURE_EXIT, "policy");
    assert!(
        stderr.contains("'core.fsmonitor', which is not a policy key"),
        "stderr must name the refused key: {stderr:?}"
    );
    assert_eq!(fixture.local_config(), before, "a refused run wrote config");

    // Run git the way a user would next: had the key been applied, this would
    // run the fsmonitor command.
    fixture.git_ok(&["status", "--porcelain"]);
    assert!(!marker.exists(), "core.fsmonitor ran: {}", marker.display());
}

#[test]
fn refused_key_control_the_legacy_include_would_have_run_it() {
    // Counterpart to the refusal test: through the old include, the same
    // tracked file runs its command on `git status`. This is what copying
    // prevents, so the refusal test is not passing for an unrelated reason.
    let marker = Path::new(TMP_ROOT).join("setup-repo-include-control/fsmonitor-ran");
    let policy = format!("[core]\n    fsmonitor = touch {}\n", marker.display());
    let fixture = Fixture::new("include-control", &policy);
    fixture.git_ok(&["config", "--local", "include.path", LEGACY_INCLUDE]);

    fixture.git(&["status", "--porcelain"]);
    assert!(marker.exists(), "the include did not apply core.fsmonitor");
}

#[test]
fn check_reports_drift_and_passes_once_applied() {
    let fixture = Fixture::new("check", &Fixture::tracked_policy());

    let (_, stderr) = assert_exit(
        &fixture.setup(&["check"]),
        FAILURE_EXIT,
        "check before apply",
    );
    assert!(stderr.contains("commit.gpgsign"), "stderr: {stderr:?}");

    assert_exit(&fixture.setup(&["policy"]), 0, "policy");
    assert_exit(&fixture.setup(&["check"]), 0, "check after apply");

    fixture.git_ok(&["config", "--local", "commit.gpgsign", "false"]);
    let (_, stderr) = assert_exit(&fixture.setup(&["check"]), FAILURE_EXIT, "check after edit");
    assert!(
        stderr.contains("commit.gpgsign: local 'false', policy 'true'"),
        "stderr must name the drifted key: {stderr:?}"
    );
}

#[test]
fn check_reports_a_remaining_legacy_include() {
    let fixture = Fixture::new("check-legacy", &Fixture::tracked_policy());
    assert_exit(&fixture.setup(&["policy"]), 0, "policy");
    fixture.git_ok(&["config", "--local", "--add", "include.path", LEGACY_INCLUDE]);

    let (_, stderr) = assert_exit(&fixture.setup(&["check"]), FAILURE_EXIT, "check");
    assert!(
        stderr.contains("still reads .gitconfig live"),
        "stderr: {stderr:?}"
    );
}

#[test]
fn all_registers_the_hook_and_applies_the_policy() {
    let fixture = Fixture::new("all", &Fixture::tracked_policy());
    fixture.stub_gitleaks();

    assert_exit(&fixture.setup(&[]), 0, "setup-repo.sh");

    let hooks = fixture.git_ok(&["hook", "list", "pre-commit"]);
    assert!(
        hooks.lines().any(|line| line == "gitleaks-staged"),
        "hook list: {hooks:?}"
    );
    let local = fixture.local_config();
    assert!(
        local.contains(&"commit.gpgsign=true".to_owned()),
        "{local:#?}"
    );
}

#[test]
fn unknown_mode_is_a_usage_error() {
    let fixture = Fixture::new("usage", &Fixture::tracked_policy());
    let (_, stderr) = assert_exit(&fixture.setup(&["polcy"]), USAGE_EXIT, "typo mode");
    assert!(stderr.contains("usage:"), "stderr: {stderr:?}");
    assert_exit(&fixture.setup(&["policy", "extra"]), USAGE_EXIT, "two args");
}
