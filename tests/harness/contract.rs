//! The `--json` contract (D-08, D-09, D-11) over real fixtures: every
//! outcome of a command prints exactly one JSON object on stdout, with the
//! command's name and schema version from the binary's own `COMMANDS`
//! table, and human text goes to stderr.
//!
//! Isolation: every test builds its fixture in its own
//! `harness-contract-<name>-` tree under `CARGO_TARGET_TMPDIR`, in the
//! hermetic environment, and spawns the built `wsx` there.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Output;

use serde_json::{Map, Value, json};

use crate::commands::{COMMANDS, CommandId, CommandSpec};
use crate::env::{ScenarioDirs, command, hermetic_env, inherited_env};
use crate::fixture::{Fixture, RepoConfig, WORK};

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
    let dirs = ScenarioDirs::new("contract", name).unwrap_or_else(|err| panic!("{err}"));
    let env = hermetic_env(&inherited_env(), &dirs);
    let fixture =
        Fixture::new(&dirs, env, RepoConfig::devflow()).unwrap_or_else(|err| panic!("{err}"));
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
