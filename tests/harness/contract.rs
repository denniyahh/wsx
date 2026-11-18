//! The `--json` contract (D-08, D-09, D-11) over real fixtures: every
//! outcome of a command prints exactly one JSON object on stdout, with the
//! command's name and schema version from the binary's own `COMMANDS`
//! table, and human text goes to stderr.
//!
//! Isolation: every test builds its fixture in its own
//! `harness-contract-<name>-` tree under `CARGO_TARGET_TMPDIR`, in the
//! hermetic environment, and spawns the built `wsx` there.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Output;
use std::time::UNIX_EPOCH;

use serde_json::{Map, Value, json};

use crate::check::{fetch_moved_develop, lose_origin};
use crate::commands::{COMMANDS, CommandId, CommandSpec};
use crate::env::{Env, ScenarioDirs, command, hermetic_env, inherited_env};
use crate::fixture::{Fixture, ManifestKind, RepoConfig, WORK};
use crate::provenance::hash_blob;

/// The `wsx` binary cargo built for this test run.
const WSX: &str = env!("CARGO_BIN_EXE_wsx");

/// A fixture and the scenario tree holding it, removed when this is dropped
/// (fields drop in order, so the fixture goes first).
struct Contract {
    fixture: Fixture,
    _dirs: ScenarioDirs,
}

/// The devflow preset in its own `harness-contract-<name>-` tree.
fn contract(name: &str) -> Contract {
    contract_with(name, RepoConfig::devflow())
}

/// `config` in its own `harness-contract-<name>-` tree.
fn contract_with(name: &str, config: RepoConfig) -> Contract {
    let dirs = ScenarioDirs::new("contract", name).unwrap_or_else(|err| panic!("{err}"));
    let env = hermetic_env(&inherited_env(), &dirs);
    let fixture = Fixture::new(&dirs, env, config).unwrap_or_else(|err| panic!("{err}"));
    Contract {
        fixture,
        _dirs: dirs,
    }
}

impl Contract {
    /// Runs git in `work` and returns its stdout; a failing git fails the test.
    fn git(&self, args: &[&str]) -> Vec<u8> {
        self.fixture
            .git_ok(WORK, args)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    /// The commit `full_ref` names, as hex.
    fn tip(&self, full_ref: &str) -> String {
        let out = self.git(&["rev-parse", "--verify", full_ref]);
        String::from_utf8(out)
            .expect("rev-parse prints hex")
            .trim_end()
            .to_owned()
    }

    /// The workspace branch gains `path` in a commit of its own.
    fn add_file(&mut self, path: &[u8]) {
        self.fixture
            .write(WORK, path, b"personal\n")
            .unwrap_or_else(|err| panic!("{err}"));
        self.fixture
            .commit(WORK, b"chore(ws): add a personal file\n")
            .unwrap_or_else(|err| panic!("{err}"));
    }

    /// Unsets `wsx.sourceBranch`, and checks git no longer answers it.
    fn unset_source_branch(&self) {
        self.git(&["config", "--local", "--unset", "wsx.sourceBranch"]);
        let get = self
            .fixture
            .git(WORK, ["config", "--get", "wsx.sourceBranch"])
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            (get.status.code(), get.stdout.escape_ascii().to_string()),
            (Some(1), String::new()),
            "wsx.sourceBranch is still set after --unset"
        );
    }

    /// Sets `wsx.sourceBranch` to `branch`, and checks git answers it.
    fn set_source_branch(&self, branch: &str) {
        self.git(&["config", "--local", "wsx.sourceBranch", branch]);
        let got = self.git(&["config", "--get", "wsx.sourceBranch"]);
        assert_eq!(got, format!("{branch}\n").into_bytes(), "wsx.sourceBranch");
    }

    /// Runs the built `wsx` with `args` in `work`, in the fixture's
    /// environment.
    fn wsx(&self, args: &[&str]) -> Output {
        command(WSX, &self.fixture.checkout(WORK), &self.fixture.env)
            .args(args)
            .output()
            .expect("spawn the wsx binary")
    }
}

/// The preset whose workspace branch adds the undeclared `notes/todo.md`.
fn with_note(name: &str) -> Contract {
    let mut contract = contract(name);
    contract.add_file(b"notes/todo.md");
    contract
}

/// The preset with `wsx.sourceBranch` unset and a second candidate,
/// `workspace/other`.
fn two_candidates(name: &str) -> Contract {
    let contract = contract(name);
    contract.unset_source_branch();
    contract.git(&["branch", "workspace/other"]);
    contract
}

/// `output`'s stderr, for assertions on prose.
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The whole of `output`'s stdout as one JSON object followed by one
/// newline; anything else fails the test, showing stdout escaped.
fn one_object(output: &Output) -> Map<String, Value> {
    let shown = output.stdout.escape_ascii().to_string();
    let newlines = output.stdout.iter().filter(|byte| **byte == b'\n').count();
    assert!(
        output.stdout.ends_with(b"\n") && newlines == 1,
        "stdout is not one line ending in a newline: {shown}"
    );
    match serde_json::from_slice::<Value>(&output.stdout) {
        Ok(Value::Object(object)) => object,
        Ok(other) => panic!("stdout is JSON but not an object: {other}"),
        Err(err) => panic!("stdout is not one JSON value ({err}): {shown}"),
    }
}

/// `object` names `spec` and its schema version.
fn assert_envelope(object: &Map<String, Value>, spec: &CommandSpec) {
    assert_eq!(object.get("command"), Some(&json!(spec.name)), "command");
    assert_eq!(
        object.get("schema_version"),
        Some(&json!(spec.schema_version)),
        "schema_version"
    );
}

/// The `check` entry of `COMMANDS`.
fn check_spec() -> &'static CommandSpec {
    COMMANDS
        .iter()
        .find(|spec| spec.id == CommandId::Check)
        .expect("COMMANDS has check")
}

/// A failure object: the envelope and `error` only; returns `error`.
fn failure(object: &Map<String, Value>) -> &Map<String, Value> {
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["command", "error", "schema_version"], "failure keys");
    object["error"].as_object().expect("error is an object")
}

/// An undeclared path: exit 1, one object with the D-11 fields and the
/// problem, no `error`; the problem and the verdict go to stderr.
#[test]
fn check_json_reports_problems_as_one_object() {
    let contract = with_note("reports_problems");
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    let object = one_object(&output);
    assert_envelope(&object, check_spec());
    assert_eq!(object.get("configured"), Some(&json!(true)));
    assert_eq!(
        object.get("workspace"),
        Some(&json!({
            "branch": "workspace/dev",
            "tip": contract.tip("refs/heads/workspace/dev"),
        }))
    );
    assert_eq!(
        object.get("integration"),
        Some(&json!({
            "branch": "develop",
            "ref": "refs/remotes/origin/develop",
            "tip": contract.tip("refs/remotes/origin/develop"),
        }))
    );
    assert_eq!(
        object.get("problems"),
        Some(&json!([
            {"kind": "undeclared_added", "path": "notes/todo.md", "detail": null}
        ]))
    );
    assert!(!object.contains_key("error"), "a problem is not an error");
    let err = stderr(&output);
    assert!(
        err.contains("undeclared workspace-only path: notes/todo.md"),
        "stderr: {err}"
    );
    assert!(
        err.contains("wsx check: 1 problem between workspace/dev and refs/remotes/origin/develop"),
        "stderr: {err}"
    );
}

/// The preset: exit 0, `problems` present and empty, the verdict on stderr.
#[test]
fn check_json_passes_as_one_object() {
    let contract = contract("passes");
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let object = one_object(&output);
    assert_envelope(&object, check_spec());
    assert_eq!(object.get("problems"), Some(&json!([])));
    assert!(!object.contains_key("error"));
    let err = stderr(&output);
    assert!(
        err.contains(
            "wsx check: every difference between workspace/dev and refs/remotes/origin/develop is declared"
        ),
        "stderr: {err}"
    );
}

/// The hooks object of a run whose stdout is one object.
fn hooks_object(output: &Output) -> Value {
    one_object(output)
        .get("hooks")
        .cloned()
        .unwrap_or_else(|| panic!("no hooks key: {}", output.stdout.escape_ascii()))
}

/// No `core.hooksPath`: exit 0, and the hooks object says it is not set.
#[test]
fn check_json_hooks_not_set() {
    let contract = contract("hooks_not_set");
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert_eq!(
        hooks_object(&output),
        json!({"path": null, "checked": false, "reason": "core.hooksPath is not set"})
    );
    assert!(
        !stderr(&output).contains("core.hooksPath"),
        "stderr: {}",
        stderr(&output)
    );
}

/// `core.hooksPath` naming a directory at the tip, holding a 100755 hook:
/// exit 0, the hooks object names the directory as checked, and the
/// object writes `hooks` before `problems`.
#[test]
fn check_json_hooks_checked() {
    let mut contract = contract("hooks_checked");
    let hook = b".planning/hooks/pre-push";
    contract
        .fixture
        .write(WORK, hook, b"#!/bin/sh\nexit 0\n")
        .unwrap_or_else(|err| panic!("{err}"));
    let full = contract
        .fixture
        .checkout(WORK)
        .join(".planning/hooks/pre-push");
    fs::set_permissions(&full, fs::Permissions::from_mode(0o755))
        .unwrap_or_else(|err| panic!("chmod {}: {err}", full.display()));
    contract
        .fixture
        .commit(WORK, b"chore(ws): add a pre-push hook\n")
        .unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(
        contract.git(&[
            "ls-tree",
            "--format=%(objectmode)",
            "HEAD",
            "--",
            ".planning/hooks/pre-push"
        ]),
        b"100755\n"
    );
    contract.git(&["config", "core.hooksPath", ".planning/hooks"]);
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert_eq!(
        hooks_object(&output),
        json!({"path": ".planning/hooks", "checked": true, "reason": null})
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let hooks_at = text.find("\"hooks\"").expect("hooks key");
    let problems_at = text.find("\"problems\"").expect("problems key");
    assert!(hooks_at < problems_at, "hooks after problems: {text}");
}

/// `core.hooksPath` relative but naming no directory at the tip: exit 0, the
/// hooks object gives the value and the reason, and stderr says it.
#[test]
fn check_json_hooks_not_in_the_tree() {
    let contract = contract("hooks_not_in_the_tree");
    contract.git(&["config", "core.hooksPath", "missing/hooks"]);
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let reason = "missing/hooks is not a directory at the tip of workspace/dev";
    assert_eq!(
        hooks_object(&output),
        json!({"path": "missing/hooks", "checked": false, "reason": reason})
    );
    let err = stderr(&output);
    assert!(
        err.contains(&format!(
            "wsx check: core.hooksPath missing/hooks is not checked: {reason}"
        )),
        "stderr: {err}"
    );
}

/// A path that is not UTF-8 is its hex; a path holding a newline is a JSON
/// string; problems keep path-byte order (`a` before 0xFF).
#[test]
fn check_json_encodes_a_non_utf8_path_as_hex() {
    let mut contract = contract("non_utf8_path");
    contract.add_file(b"\xff.md");
    contract.add_file(b"a\nb.md");
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    let object = one_object(&output);
    let paths: Vec<&Value> = object["problems"]
        .as_array()
        .expect("problems is an array")
        .iter()
        .map(|problem| &problem["path"])
        .collect();
    assert_eq!(paths, [&json!("a\nb.md"), &json!({"hex": "ff2e6d64"})]);
}

/// Two candidates and no `wsx.sourceBranch`: exit 3, one failure object
/// naming both; the message on stderr.
#[test]
fn check_json_failure_is_one_object() {
    let contract = two_candidates("failure");
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(3), "stderr: {}", stderr(&output));
    let object = one_object(&output);
    assert_envelope(&object, check_spec());
    let error = failure(&object);
    assert_eq!(error.get("kind"), Some(&json!("workspace_candidates")));
    assert_eq!(error.get("exit_code"), Some(&json!(3)));
    let message = error["message"].as_str().expect("message is a string");
    assert!(message.contains("workspace/dev"), "message: {message}");
    assert!(message.contains("workspace/other"), "message: {message}");
    assert!(
        stderr(&output).starts_with("wsx check: "),
        "stderr: {}",
        stderr(&output)
    );
}

/// D-04: no `wsx.sourceBranch` and one candidate: the check runs against it,
/// reports `configured` false and says so on stderr; with the key set
/// (control) it reports `configured` true and no such line.
#[test]
fn check_in_an_unconfigured_repo_checks_the_one_candidate() {
    let contract = contract("one_candidate");
    contract.unset_source_branch();
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let object = one_object(&output);
    assert_eq!(object.get("configured"), Some(&json!(false)));
    assert_eq!(object["workspace"]["branch"], json!("workspace/dev"));
    let err = stderr(&output);
    assert!(err.contains("wsx.sourceBranch is not set"), "stderr: {err}");
    assert!(err.contains("workspace/dev"), "stderr: {err}");
    assert!(err.contains("run wsx init"), "stderr: {err}");

    contract.set_source_branch("workspace/dev");
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert_eq!(one_object(&output).get("configured"), Some(&json!(true)));
    let err = stderr(&output);
    assert!(
        !err.contains("wsx.sourceBranch is not set"),
        "stderr: {err}"
    );
}

/// No `wsx.sourceBranch` and no candidate: exit 3, nothing on stdout
/// without `--json`, and with it one failure object saying to run wsx init.
#[test]
fn check_with_no_workspace_branch_is_not_configured() {
    let contract = contract("no_candidate");
    contract.unset_source_branch();
    contract.git(&["branch", "-m", "workspace/dev", "dev-notes"]);
    let listed = contract.git(&[
        "for-each-ref",
        "refs/heads/workspace/",
        "refs/heads/personal/",
    ]);
    assert_eq!(listed.escape_ascii().to_string(), "", "a candidate is left");

    let plain = contract.wsx(&["check"]);
    assert_eq!(plain.status.code(), Some(3), "stderr: {}", stderr(&plain));
    assert_eq!(
        plain.stdout.escape_ascii().to_string(),
        "",
        "stdout without --json"
    );
    assert!(
        stderr(&plain).starts_with("wsx check: "),
        "stderr: {}",
        stderr(&plain)
    );

    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(3), "stderr: {}", stderr(&output));
    let object = one_object(&output);
    assert_envelope(&object, check_spec());
    let error = failure(&object);
    assert_eq!(error.get("kind"), Some(&json!("workspace_candidates")));
    assert_eq!(error.get("exit_code"), Some(&json!(3)));
    let message = error["message"].as_str().expect("message is a string");
    assert!(message.contains("Run wsx init"), "message: {message}");
    assert!(
        stderr(&output).starts_with("wsx check: "),
        "stderr: {}",
        stderr(&output)
    );
}

/// Every command's success and failure cases print one object naming the
/// command and its schema version; a failure (exit 2 to 6) carries `error`
/// with the process's exit code, a run that found problems (exit 1) none.
/// The match on `CommandId` is exhaustive, so a new command needs cases here
/// to compile.
#[test]
fn every_command_prints_one_json_object() {
    for spec in COMMANDS {
        let cases: Vec<Contract> = match spec.id {
            CommandId::Check => vec![
                contract("every_check_passes"),
                with_note("every_check_problems"),
                two_candidates("every_check_failure"),
            ],
        };
        for case in &cases {
            let output = case.wsx(&[spec.name, "--json"]);
            let code = output.status.code().expect("wsx exits with a code");
            let object = one_object(&output);
            assert_envelope(&object, spec);
            if code >= 2 {
                assert_eq!(failure(&object).get("exit_code"), Some(&json!(code)));
            } else {
                assert!(!object.contains_key("error"), "exit {code} carries error");
            }
        }
    }
}

/// The staleness object of a run whose stdout is one object.
fn staleness_object(output: &Output) -> Map<String, Value> {
    match one_object(output).get("staleness") {
        Some(Value::Object(staleness)) => staleness.clone(),
        other => panic!("staleness is not an object: {other:?}"),
    }
}

/// `git reflog exists refs/remotes/origin/develop`'s exit code in `work`.
fn reflog_exists(contract: &Contract) -> Option<i32> {
    contract
        .fixture
        .git(WORK, ["reflog", "exists", "refs/remotes/origin/develop"])
        .unwrap_or_else(|err| panic!("{err}"))
        .status
        .code()
}

/// The `FETCH_HEAD` path git names for `work`.
fn fetch_head(contract: &Contract) -> std::path::PathBuf {
    let path = contract.git(&["rev-parse", "--git-path", "FETCH_HEAD"]);
    contract
        .fixture
        .checkout(WORK)
        .join(OsStr::from_bytes(path.trim_ascii_end()))
}

/// The preset whose `work` fetched a moved `origin/develop`.
fn fetched(name: &str) -> Contract {
    let mut contract = contract(name);
    fetch_moved_develop(&mut contract.fixture).unwrap_or_else(|err| panic!("{err}"));
    contract
}

/// `work` is the only worktree `git worktree list --porcelain` lists.
fn assert_only_work(contract: &Contract) {
    let list = String::from_utf8(contract.git(&["worktree", "list", "--porcelain"]))
        .expect("worktree list prints text");
    let worktrees = list
        .lines()
        .filter(|line| line.starts_with("worktree "))
        .count();
    assert_eq!(worktrees, 1, "worktrees: {list}");
}

/// A fresh clone: git keeps no reflog for `origin/develop` and `work` has no
/// `FETCH_HEAD`, so both times are null with their reasons.
#[test]
fn check_json_staleness_after_a_clone() {
    let contract = contract("staleness_after_a_clone");
    assert_eq!(
        reflog_exists(&contract),
        Some(1),
        "reflog exists after a clone"
    );
    assert_only_work(&contract);
    assert!(!fetch_head(&contract).exists(), "FETCH_HEAD after a clone");
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert_eq!(
        Value::Object(staleness_object(&output)),
        json!({
            "applies": true,
            "reason": null,
            "ref_moved_at": null,
            "ref_moved_age_seconds": null,
            "ref_moved_reason": "no reflog for refs/remotes/origin/develop",
            "fetched_at": null,
            "fetched_age_seconds": null,
            "fetched_reason": "no FETCH_HEAD",
        })
    );
}

/// After a fetch that moved `origin/develop`: `ref_moved_at` is the time git
/// prints for the newest reflog entry, `fetched_at` the mtime of the only
/// worktree's `FETCH_HEAD`, and both ages are 0 or more.
#[test]
fn check_json_staleness_after_a_fetch() {
    let contract = fetched("staleness_after_a_fetch");
    assert_eq!(
        reflog_exists(&contract),
        Some(0),
        "reflog exists after a fetch"
    );
    let selector = String::from_utf8(contract.git(&[
        "reflog",
        "show",
        "-n1",
        "--date=unix",
        "--format=%gd",
        "refs/remotes/origin/develop",
    ]))
    .expect("reflog prints text");
    let moved: i64 = selector
        .trim_end()
        .rsplit_once("@{")
        .and_then(|(_, time)| time.strip_suffix('}'))
        .and_then(|time| time.parse().ok())
        .unwrap_or_else(|| panic!("no unix time in {selector:?}"));
    assert_only_work(&contract);
    let modified = fs::metadata(fetch_head(&contract))
        .and_then(|meta| meta.modified())
        .expect("FETCH_HEAD's mtime");
    let fetched = i64::try_from(
        modified
            .duration_since(UNIX_EPOCH)
            .expect("an mtime after the epoch")
            .as_secs(),
    )
    .expect("seconds fit i64");
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(!one_object(&output).contains_key("error"), "no error key");
    let staleness = staleness_object(&output);
    assert_eq!(staleness["applies"], json!(true), "applies");
    assert_eq!(staleness["ref_moved_at"], json!(moved), "ref_moved_at");
    assert_eq!(staleness["fetched_at"], json!(fetched), "fetched_at");
    for age in ["ref_moved_age_seconds", "fetched_age_seconds"] {
        assert!(
            staleness[age].as_i64().is_some_and(|seconds| seconds >= 0),
            "{age}: {}",
            staleness[age]
        );
    }
    assert_eq!(
        staleness["ref_moved_reason"],
        Value::Null,
        "ref_moved_reason"
    );
    assert_eq!(staleness["fetched_reason"], Value::Null, "fetched_reason");
}

/// The preset with no remote, and no `wsx.baseRemote` or `wsx.pushRemote`
/// naming one, so the integration ref is the local `develop` (phase 4 D-07).
fn local_branch(name: &str) -> Contract {
    let contract = contract(name);
    contract.tip("refs/heads/develop");
    contract.git(&["remote", "remove", "origin"]);
    contract.git(&["config", "--local", "--unset", "wsx.baseRemote"]);
    contract.git(&["config", "--local", "--unset", "wsx.pushRemote"]);
    contract
}

/// No remote, and no `wsx.baseRemote` or `wsx.pushRemote` naming one, so
/// the integration ref is the preset's local `develop` (phase 4 D-07):
/// staleness does not apply and no time is read.
#[test]
fn check_json_staleness_does_not_apply_to_a_local_branch() {
    let contract = local_branch("staleness_local_branch");
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let object = one_object(&output);
    assert_eq!(
        object["integration"]["ref"],
        json!("refs/heads/develop"),
        "integration ref"
    );
    assert_eq!(
        Value::Object(staleness_object(&output)),
        json!({
            "applies": false,
            "reason": "refs/heads/develop is a local branch; it is not fetched",
            "ref_moved_at": null,
            "ref_moved_age_seconds": null,
            "ref_moved_reason": null,
            "fetched_at": null,
            "fetched_age_seconds": null,
            "fetched_reason": null,
        })
    );
}

/// The remote unreachable after a fetch that moved `origin/develop`: exit 0,
/// and both times and ages are integers with no reason (D-13).
#[test]
fn check_json_staleness_with_the_remote_unreachable() {
    let mut contract = fetched("staleness_remote_unreachable");
    lose_origin(&mut contract.fixture).unwrap_or_else(|err| panic!("{err}"));
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let staleness = staleness_object(&output);
    assert_eq!(staleness["applies"], json!(true), "applies");
    for key in [
        "ref_moved_at",
        "ref_moved_age_seconds",
        "fetched_at",
        "fetched_age_seconds",
    ] {
        assert!(staleness[key].is_i64(), "{key}: {}", staleness[key]);
    }
    assert_eq!(
        staleness["ref_moved_reason"],
        Value::Null,
        "ref_moved_reason"
    );
    assert_eq!(staleness["fetched_reason"], Value::Null, "fetched_reason");
}

/// A `FETCH_HEAD` that cannot be stat'ed (a regular file at
/// `<common>/worktrees/stray`) makes `fetched_at` null with its reason and
/// leaves the exit code and `ref_moved_at` as they were (D-13); the same
/// repository without the stray file is the control.
#[test]
fn check_json_staleness_read_failure_keeps_the_exit_code() {
    let contract = fetched("staleness_read_failure");
    let control = contract.wsx(&["check", "--json"]);
    assert_eq!(
        control.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&control)
    );
    let before = staleness_object(&control);
    assert!(
        before["fetched_at"].is_i64(),
        "control fetched_at: {}",
        before["fetched_at"]
    );
    let common = contract.git(&["rev-parse", "--git-common-dir"]);
    let worktrees = contract
        .fixture
        .checkout(WORK)
        .join(OsStr::from_bytes(common.trim_ascii_end()))
        .join("worktrees");
    fs::create_dir_all(&worktrees).expect("create <common>/worktrees");
    fs::write(
        worktrees.join("stray"),
        b"not a worktree
",
    )
    .expect("write the stray file");
    let stat = fs::metadata(worktrees.join("stray").join("FETCH_HEAD"));
    assert!(
        stat.as_ref()
            .is_err_and(|err| err.kind() != std::io::ErrorKind::NotFound),
        "stat of stray/FETCH_HEAD: {stat:?}"
    );
    contract.git(&["worktree", "list", "--porcelain"]);
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let staleness = staleness_object(&output);
    assert_eq!(staleness["fetched_at"], Value::Null, "fetched_at");
    assert_eq!(
        staleness["fetched_age_seconds"],
        Value::Null,
        "fetched_age_seconds"
    );
    assert!(
        staleness["fetched_reason"]
            .as_str()
            .is_some_and(|reason| reason.starts_with("FETCH_HEAD could not be read: ")),
        "fetched_reason: {}",
        staleness["fetched_reason"]
    );
    assert!(
        staleness["ref_moved_at"].is_i64(),
        "ref_moved_at: {}",
        staleness["ref_moved_at"]
    );
    assert_eq!(
        staleness["ref_moved_reason"],
        Value::Null,
        "ref_moved_reason"
    );
}

/// The schema files, as committed.
const SCHEMA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/json");

/// The schema file of version `version` of command `name`.
fn schema_file(name: &str, version: u32) -> String {
    format!("{name}.v{version}.schema.json")
}

/// The validator for `spec`'s committed schema at its current version.
fn validator(spec: &CommandSpec) -> jsonschema::Validator {
    let path = Path::new(SCHEMA_DIR).join(schema_file(spec.name, spec.schema_version));
    let bytes = fs::read(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    let schema: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|err| panic!("{} is not JSON: {err}", path.display()));
    jsonschema::validator_for(&schema)
        .unwrap_or_else(|err| panic!("{} is not a valid schema: {err}", path.display()))
}

/// Every error `validator` finds in `instance`, each after the location
/// it names.
fn schema_errors(validator: &jsonschema::Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|err| format!("at {:?}: {err}", err.instance_path().as_str()))
        .collect()
}

/// The seven problem kinds, by their JSON names.
const PROBLEM_KINDS: [&str; 7] = [
    "undeclared_added",
    "undeclared_modified",
    "undeclared_deleted",
    "stale_only",
    "stale_modified",
    "redundant_builtin",
    "hook_not_executable",
];

/// One fixture with every problem kind: `origin/develop` gains
/// `src/b.txt` and `shared/x.md`, which `work` fetches and merges, with
/// `only shared` and `modified src/b.txt` declared (stale); `only
/// .worktrees/x` (redundant); an undeclared `0xFF.md`, an edited
/// `README.md` and a deleted `src/a.txt`; and `scripts/hooks/pre-push`
/// committed 100644 under `core.hooksPath scripts/hooks`, with `only
/// scripts` declared.
fn all_kinds(name: &str) -> Contract {
    let config = RepoConfig::devflow()
        .with_manifest_entry(ManifestKind::Only, b"shared")
        .with_manifest_entry(ManifestKind::Modified, b"src/b.txt")
        .with_manifest_entry(ManifestKind::Only, b".worktrees/x")
        .with_manifest_entry(ManifestKind::Only, b"scripts");
    let mut contract = contract_with(name, config);
    contract
        .fixture
        .advance_remote("origin", "develop", |fixture, scratch| {
            fixture.write(scratch, b"src/b.txt", b"b\n")?;
            fixture.write(scratch, b"shared/x.md", b"x\n")?;
            fixture.commit(scratch, b"feat: add two shared files\n")
        })
        .unwrap_or_else(|err| panic!("{err}"));
    contract.git(&["fetch", "--quiet", "origin"]);
    contract
        .fixture
        .git_tick(
            WORK,
            ["merge", "--quiet", "--no-ff", "--no-edit", "origin/develop"],
        )
        .unwrap_or_else(|err| panic!("{err}"));
    contract.add_file(b"\xff.md");
    let hook = b"scripts/hooks/pre-push";
    for (path, content) in [
        (&b"README.md"[..], &b"# Fixture, edited\n"[..]),
        (hook, b"#!/bin/sh\nexit 0\n"),
    ] {
        contract
            .fixture
            .write(WORK, path, content)
            .unwrap_or_else(|err| panic!("{err}"));
    }
    let full = contract
        .fixture
        .checkout(WORK)
        .join("scripts/hooks/pre-push");
    fs::set_permissions(&full, fs::Permissions::from_mode(0o644))
        .unwrap_or_else(|err| panic!("chmod {}: {err}", full.display()));
    contract
        .fixture
        .remove(WORK, b"src/a.txt")
        .unwrap_or_else(|err| panic!("{err}"));
    contract
        .fixture
        .commit(WORK, b"chore(ws): edit, delete and add a hook\n")
        .unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(
        contract.git(&[
            "ls-tree",
            "--format=%(objectmode)",
            "HEAD",
            "--",
            "scripts/hooks/pre-push"
        ]),
        b"100644\n"
    );
    contract.git(&["config", "core.hooksPath", "scripts/hooks"]);
    contract
}

/// The preset with `wsx.baseRemote` naming a remote that does not exist.
fn bad_base_remote(name: &str) -> Contract {
    let contract = contract(name);
    contract.git(&["config", "--local", "wsx.baseRemote", "nowhere"]);
    contract
}

/// `object`'s problems hold every kind at least once and its hooks object
/// says the hooks were checked.
fn assert_all_kinds(object: &Value) {
    let kinds: Vec<&Value> = object["problems"]
        .as_array()
        .expect("problems is an array")
        .iter()
        .map(|problem| &problem["kind"])
        .collect();
    for kind in PROBLEM_KINDS {
        assert!(kinds.contains(&&json!(kind)), "no {kind} in {kinds:?}");
    }
    assert_eq!(object["hooks"]["checked"], json!(true), "hooks.checked");
}

/// One `--json` run for the schema test: its fixture, the arguments after
/// `<name> --json`, the exit code it stands for and any further check.
struct SchemaCase {
    name: &'static str,
    contract: Contract,
    extra: &'static [&'static str],
    exit: i32,
    also: Option<fn(&Value)>,
}

impl SchemaCase {
    fn new(name: &'static str, contract: Contract, exit: i32) -> SchemaCase {
        SchemaCase {
            name,
            contract,
            extra: &[],
            exit,
            also: None,
        }
    }
}

/// D-10a: every command's success and failure cases each print one object
/// that validates against the command's schema at its current version. For
/// check: exit 0, exit 1 with every problem kind, exit 2, 3 and 4, and the
/// local-branch staleness form. The match on `CommandId` is exhaustive, so
/// a new command needs cases here to compile.
#[test]
fn every_command_output_matches_its_schema() {
    for spec in COMMANDS {
        let cases = match spec.id {
            CommandId::Check => vec![
                SchemaCase::new("passes", contract("schema_passes"), 0),
                SchemaCase {
                    also: Some(assert_all_kinds),
                    ..SchemaCase::new("all_kinds", all_kinds("schema_all_kinds"), 1)
                },
                SchemaCase {
                    extra: &["--bogus"],
                    ..SchemaCase::new("usage", contract("schema_usage"), 2)
                },
                SchemaCase::new("candidates", two_candidates("schema_candidates"), 3),
                SchemaCase::new(
                    "bad_base_remote",
                    bad_base_remote("schema_bad_base_remote"),
                    4,
                ),
                SchemaCase::new("local_branch", local_branch("schema_local_branch"), 0),
            ],
        };
        let validator = validator(spec);
        for case in &cases {
            let mut args = vec![spec.name, "--json"];
            args.extend(case.extra);
            let output = case.contract.wsx(&args);
            assert_eq!(
                output.status.code(),
                Some(case.exit),
                "{} {}: stderr: {}",
                spec.name,
                case.name,
                stderr(&output)
            );
            let object = Value::Object(one_object(&output));
            let errors = schema_errors(&validator, &object);
            assert!(
                errors.is_empty(),
                "{} {}: {object} fails {}:\n{}",
                spec.name,
                case.name,
                schema_file(spec.name, spec.schema_version),
                errors.join("\n")
            );
            if let Some(also) = case.also {
                also(&object);
            }
        }
    }
}

/// The schema is strict: a real check object with `problems` renamed, a
/// problem kind unknown, or a hex path of odd length each fails validation;
/// the unmutated object (control) passes.
#[test]
fn schema_rejects_a_renamed_field() {
    let validator = validator(check_spec());
    let contract = with_note("schema_rejects");
    let output = contract.wsx(&["check", "--json"]);
    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    let real = Value::Object(one_object(&output));
    assert_eq!(
        schema_errors(&validator, &real),
        Vec::<String>::new(),
        "the unmutated object"
    );
    let mut renamed = real.clone();
    let object = renamed.as_object_mut().expect("an object");
    let problems = object.remove("problems").expect("a problems key");
    object.insert("problem".to_owned(), problems);
    let mut unknown_kind = real.clone();
    unknown_kind["problems"][0]["kind"] = json!("undeclared_renamed");
    let mut odd_hex = real.clone();
    odd_hex["problems"][0]["path"] = json!({"hex": "abc"});
    for (mutation, mutated) in [
        ("problems renamed problem", renamed),
        ("kind undeclared_renamed", unknown_kind),
        ("path hex abc", odd_hex),
    ] {
        assert!(
            !schema_errors(&validator, &mutated).is_empty(),
            "{mutation}: the schema accepts {mutated}"
        );
    }
}

/// Every schema file's pinned blob: `(command, schema version, blob)`, the
/// blob being what `git hash-object --no-filters
/// docs/json/<command>.v<N>.schema.json` prints. A schema file never changes
/// once pinned: a change bumps the command's `schema_version` in
/// `src/cli/commands.rs`, adds `<command>.v<N+1>.schema.json` and a row
/// here, and keeps the old file and row (docs/json/README.md).
const SCHEMA_PINS: [(&str, u32, &str); 1] =
    [("check", 1, "4dddf8bbb55fbcde054f2946b76105761202bfc1")];

/// Each pinned schema file under `dir` whose blob is not its pin, as a line
/// saying a change needs a new version. A file that cannot be hashed is
/// reported too, so a missing file never reads as a match. `git
/// hash-object` runs in the scenario HOME with `env`.
fn pin_mismatches(
    dir: &Path,
    pins: &[(&str, u32, &str)],
    dirs: &ScenarioDirs,
    env: &Env,
) -> Vec<String> {
    pins.iter()
        .filter_map(|&(name, version, pinned)| {
            let file = schema_file(name, version);
            match hash_blob(&dir.join(&file), dirs, env) {
                Ok(actual) if actual == pinned => None,
                Ok(actual) => Some(format!(
                    "docs/json/{file}: blob {actual}, pinned {pinned} for ({name}, {version}): \
                     a schema change needs a new version: bump schema_version in \
                     src/cli/commands.rs, add {} and its pin, and keep the old file and pin",
                    schema_file(name, version + 1)
                )),
                Err(reason) => Some(format!("docs/json/{file}: {reason}")),
            }
        })
        .collect()
}

/// `file`'s command name and version, when it is named
/// `<name>.v<N>.schema.json`.
fn parse_schema_file(file: &str) -> Option<(&str, u32)> {
    let (name, version) = file.strip_suffix(".schema.json")?.rsplit_once(".v")?;
    let version = version.parse().ok()?;
    (!name.is_empty()).then_some((name, version))
}

/// The names of the `*.schema.json` files in `dir`, sorted, or why `dir`
/// could not be listed.
fn schema_files(dir: &Path) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(dir).map_err(|err| format!("list {}: {err}", dir.display()))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("list {}: {err}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".schema.json") {
            files.push(name);
        }
    }
    files.sort_unstable();
    Ok(files)
}

/// What keeps `commands`, `pins` and the schema files in `dir` from
/// covering each other: a command with no file or no pin for its current
/// version, a schema file that names no version of a command (`N` from 1
/// to its `schema_version`) or has no pin, and a pin with no file.
fn coverage_gaps(commands: &[CommandSpec], pins: &[(&str, u32, &str)], dir: &Path) -> Vec<String> {
    let pinned = |name: &str, version: u32| {
        pins.iter()
            .any(|&(pin_name, pin_version, _)| pin_name == name && pin_version == version)
    };
    let mut gaps = Vec::new();
    for spec in commands {
        let file = schema_file(spec.name, spec.schema_version);
        if !dir.join(&file).is_file() {
            gaps.push(format!("command {}: no docs/json/{file}", spec.name));
        }
        if !pinned(spec.name, spec.schema_version) {
            gaps.push(format!(
                "command {}: no pin for ({}, {}) in SCHEMA_PINS",
                spec.name, spec.name, spec.schema_version
            ));
        }
    }
    match schema_files(dir) {
        Ok(files) => {
            for file in files {
                let known = parse_schema_file(&file).filter(|&(name, version)| {
                    commands.iter().any(|spec| {
                        spec.name == name && (1..=spec.schema_version).contains(&version)
                    })
                });
                match known {
                    Some((name, version)) if pinned(name, version) => {}
                    Some(_) => gaps.push(format!("docs/json/{file}: no pin in SCHEMA_PINS")),
                    None => gaps.push(format!(
                        "docs/json/{file}: names no version of a command in COMMANDS"
                    )),
                }
            }
        }
        Err(reason) => gaps.push(reason),
    }
    for &(name, version, _) in pins {
        let file = schema_file(name, version);
        if !dir.join(&file).is_file() {
            gaps.push(format!("pin ({name}, {version}): no docs/json/{file}"));
        }
    }
    gaps
}

/// A fresh copy of the committed schema directory at
/// `CARGO_TARGET_TMPDIR/contract-pins/<name>`.
fn schema_copy(name: &str) -> std::path::PathBuf {
    let copy = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("contract-pins")
        .join(name);
    if copy.exists() {
        fs::remove_dir_all(&copy).expect("remove a stale schema copy");
    }
    fs::create_dir_all(&copy).expect("create the schema copy");
    for entry in fs::read_dir(SCHEMA_DIR).expect("read docs/json") {
        let entry = entry.expect("a docs/json entry");
        fs::copy(entry.path(), copy.join(entry.file_name())).expect("copy a docs/json file");
    }
    copy
}

/// The hermetic environment of a `harness-contract-<name>-` tree, for git.
fn scenario(name: &str) -> (ScenarioDirs, Env) {
    let dirs = ScenarioDirs::new("contract", name).unwrap_or_else(|err| panic!("{err}"));
    let env = hermetic_env(&inherited_env(), &dirs);
    (dirs, env)
}

/// D-10b: every committed schema file hashes to its pin.
#[test]
fn schema_files_match_their_pins() {
    let (dirs, env) = scenario("pins");
    assert_eq!(
        pin_mismatches(Path::new(SCHEMA_DIR), &SCHEMA_PINS, &dirs, &env),
        Vec::<String>::new()
    );
}

/// D-10b's control: one field renamed in a scratch copy of check's schema,
/// with no version bump, is exactly one mismatch naming the file and saying
/// a change needs a new version.
#[test]
fn a_schema_edit_without_a_version_bump_fails() {
    let (dirs, env) = scenario("pins_edited");
    let copy = schema_copy("edited");
    let file = copy.join("check.v1.schema.json");
    let before = fs::read_to_string(&file).expect("read the copied schema");
    let after = before.replace("\"problems\"", "\"problem\"");
    assert_ne!(before, after, "the edit changed nothing");
    fs::write(&file, after).expect("write the edited schema");
    let mismatches = pin_mismatches(&copy, &SCHEMA_PINS, &dirs, &env);
    assert_eq!(mismatches.len(), 1, "mismatches: {mismatches:?}");
    assert!(
        mismatches[0].contains("check.v1.schema.json"),
        "{}",
        mismatches[0]
    );
    assert!(
        mismatches[0].contains("needs a new version"),
        "{}",
        mismatches[0]
    );
}

/// D-10c: every `COMMANDS` entry has its schema file and pin, and every
/// schema file and pin belongs to an entry.
#[test]
fn every_command_has_a_pinned_schema() {
    assert_eq!(
        coverage_gaps(COMMANDS, &SCHEMA_PINS, Path::new(SCHEMA_DIR)),
        Vec::<String>::new()
    );
}

/// D-10c's control: a table entry with no schema and a stray schema file
/// are each named; the unmodified inputs give no gap.
#[test]
fn a_command_without_a_pinned_schema_is_named() {
    assert_eq!(
        coverage_gaps(COMMANDS, &SCHEMA_PINS, Path::new(SCHEMA_DIR)),
        Vec::<String>::new(),
        "the unmodified inputs"
    );
    let mut extra = COMMANDS.to_vec();
    extra.push(CommandSpec {
        id: CommandId::Check,
        name: "bogus",
        schema_version: 1,
    });
    let gaps = coverage_gaps(&extra, &SCHEMA_PINS, Path::new(SCHEMA_DIR));
    assert!(
        !gaps.is_empty() && gaps.iter().all(|gap| gap.contains("bogus")),
        "gaps: {gaps:?}"
    );
    let copy = schema_copy("stray");
    fs::write(copy.join("stray.v1.schema.json"), "{}\n").expect("write the stray schema");
    let gaps = coverage_gaps(COMMANDS, &SCHEMA_PINS, &copy);
    assert!(
        !gaps.is_empty() && gaps.iter().all(|gap| gap.contains("stray.v1.schema.json")),
        "gaps: {gaps:?}"
    );
}
