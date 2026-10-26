//! Provenance of the vendored oracle (plan 02-04).
//!
//! Proves that the scripts vendored under `tests/oracle` are the files devflow
//! published at the pinned revision, and that the patch the harness applies to
//! its scratch copy still applies and changes only what it says it changes.
//!
//! Isolation: the checked-in files are only read; anything patched is a
//! scratch copy inside the test's own `harness-provenance-` tree under
//! `CARGO_TARGET_TMPDIR`.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;
use std::process::Stdio;

use crate::action::{Action, PushUpdate};
use crate::env::{Env, ScenarioDirs, command, hermetic_env, inherited_env};
use crate::fixture::{Fixture, RepoConfig, WORK};
use crate::oracle::{self, ActionRun, OracleCopy};

/// The devflow commit the oracle was vendored from (`origin/develop`,
/// 2026-10-23, "Merge pull request #228 from denniyahh/feature/dependabot").
const DEVFLOW_COMMIT: &str = "ee6ec792d03f1495fc8c4d90d5894ff08b968d80";

/// The vendored oracle, as committed.
const ORACLE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/oracle");

/// Every vendored file: its path under `tests/oracle`, its path in devflow,
/// its mode there and its blob at `DEVFLOW_COMMIT`, from `git ls-tree -l
/// ee6ec792 -- <path>` in devflow (RESEARCH "Oracle inventory").
const VENDORED_BLOBS: [(&str, &str, &str, &str); 6] = [
    (
        "sync-workspace.sh",
        "scripts/sync-workspace.sh",
        "100755",
        "82ed2d7572e03c7527f3deced2a51562ba891e85",
    ),
    (
        "phase-worktree.sh",
        "scripts/phase-worktree.sh",
        "100755",
        "ae6e47022137768a6ff947151c7169b898781719",
    ),
    (
        "cut-pr-branch.sh",
        "scripts/cut-pr-branch.sh",
        "100755",
        "d77eda55b84aa67215ecf51ddde2b38c982a6989",
    ),
    (
        "check-workspace-divergence.sh",
        "scripts/check-workspace-divergence.sh",
        "100755",
        "0041f8a50fae25a2b88b1fe4ac9393314697909f",
    ),
    (
        "hooks/pre-push",
        "scripts/hooks/pre-push",
        "100755",
        "67ef847ecf98dfaa1b124be5b10179b7a0134dc9",
    ),
    (
        "LICENSE",
        "LICENSE",
        "100644",
        "9fde56fe9dc9a61107fb774925149bd11942de67",
    ),
];

/// `(path, blob)` for every vendored file.
fn vendored_pairs() -> [(&'static str, &'static str); 6] {
    VENDORED_BLOBS.map(|(path, _, _, blob)| (path, blob))
}

/// The blob of each script `oracle.patch` changes, after the patch.
///
/// Produced once, by copying `tests/oracle` to a scratch directory outside any
/// repository, running `git apply -v oracle.patch` there with
/// `GIT_CEILING_DIRECTORIES` set to that directory (all three files reported
/// `Applied patch ... cleanly`), and `git hash-object --no-filters` on each
/// result. Each equals the post-image abbreviation on the patch's own `index`
/// line for that file (`21465e9`, `5553649`, `093b279`).
const PATCHED_BLOBS: [(&str, &str); 3] = [
    (
        "check-workspace-divergence.sh",
        "21465e914f6caed713da72a75824f72ba56756ec",
    ),
    (
        "phase-worktree.sh",
        "55536494523bff4c6faa1af4582231b755a9fc3e",
    ),
    ("hooks/pre-push", "093b279b591b27401613c7c4dbe5edcdcd3d907f"),
];

/// `(path, devflow blob)` for every vendored file the patch leaves alone.
fn unpatched_pairs() -> Vec<(&'static str, &'static str)> {
    vendored_pairs()
        .into_iter()
        .filter(|(path, _)| !PATCHED_BLOBS.iter().any(|(patched, _)| patched == path))
        .collect()
}

/// Each file in `table` (paths relative to `dir`) whose content does not hash
/// to its recorded blob, as `<path>: <reason>`. A file that cannot be hashed is
/// reported too, so a missing file never reads as a match. `git hash-object`
/// runs in the scenario HOME with `env`, so it reads no host configuration.
fn blob_mismatches(
    dir: &Path,
    table: impl IntoIterator<Item = (&'static str, &'static str)>,
    dirs: &ScenarioDirs,
    env: &Env,
) -> Vec<String> {
    table
        .into_iter()
        .filter_map(
            |(path, recorded)| match hash_blob(&dir.join(path), dirs, env) {
                Ok(actual) if actual == recorded => None,
                Ok(actual) => Some(format!("{path}: blob {actual}, recorded {recorded}")),
                Err(reason) => Some(format!("{path}: {reason}")),
            },
        )
        .collect()
}

/// The blob id git gives `file`'s bytes, with no attribute or line-ending
/// filter applied.
fn hash_blob(file: &Path, dirs: &ScenarioDirs, env: &Env) -> Result<String, String> {
    let output = command("git", &dirs.home, env)
        .args([OsStr::new("hash-object"), OsStr::new("--no-filters")])
        .arg(file)
        .output()
        .map_err(|err| format!("spawn git hash-object: {err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let blob = stdout.trim_end();
    if output.status.code() != Some(0) || blob.len() != 40 {
        return Err(format!(
            "git hash-object exited {:?} with stdout {blob:?}; stderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim_end()
        ));
    }
    Ok(blob.to_owned())
}

/// Copies every vendored file and `oracle.patch` from `tests/oracle` to `to`,
/// keeping each file's permission bits.
fn scratch_copy(to: &Path) {
    for path in VENDORED_BLOBS
        .map(|(path, _, _, _)| path)
        .into_iter()
        .chain(["oracle.patch"])
    {
        let target = to.join(path);
        let parent = target.parent().expect("a vendored path has a parent");
        fs::create_dir_all(parent).expect("create the scratch copy's directories");
        fs::copy(Path::new(ORACLE_DIR).join(path), &target).expect("copy a vendored file");
    }
}

/// The paths a mismatch list names, in order.
fn reported_paths(mismatches: &[String]) -> Vec<&str> {
    mismatches
        .iter()
        .map(|line| {
            line.split_once(": ")
                .map_or(line.as_str(), |(path, _)| path)
        })
        .collect()
}

/// A copy that differs from the vendored set by one flipped byte in one file
/// is reported naming exactly that file.
#[test]
fn flipped_byte_copy_is_reported() {
    let dirs = ScenarioDirs::new("provenance", "flipped_byte").expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let copy = dirs.root.join("copy");
    scratch_copy(&copy);
    let flipped = copy.join("cut-pr-branch.sh");
    let mut bytes = fs::read(&flipped).expect("read the copied script");
    bytes[0] ^= 0x01;
    fs::write(&flipped, &bytes).expect("write the flipped script");

    let mismatches = blob_mismatches(&copy, vendored_pairs(), &dirs, &env);
    assert_eq!(
        reported_paths(&mismatches),
        ["cut-pr-branch.sh"],
        "a flipped byte in cut-pr-branch.sh must be the one file reported against \
         devflow {DEVFLOW_COMMIT}; full report: {mismatches:?}"
    );
}

/// The files under `tests/oracle` hash to the blobs devflow recorded for them
/// at `DEVFLOW_COMMIT`.
///
/// Counterpart: `flipped_byte_copy_is_reported`, which shows the same checker
/// reports a copy one byte away, so an empty list here is not a checker that
/// never reports.
#[test]
fn vendored_files_match_devflow_blobs() {
    let dirs = ScenarioDirs::new("provenance", "vendored_blobs").expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let mismatches = blob_mismatches(Path::new(ORACLE_DIR), vendored_pairs(), &dirs, &env);
    assert!(
        mismatches.is_empty(),
        "tests/oracle no longer matches devflow {DEVFLOW_COMMIT}: {mismatches:?}"
    );
}

/// `git apply` exits non-zero in `dir`, as a second apply of the same patch
/// must; returns its stderr.
fn reapply_fails(dir: &Path, env: &Env) -> String {
    let output = command("git", dir, env)
        .args(["apply", "oracle.patch"])
        .output()
        .expect("spawn git apply");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_ne!(
        output.status.code(),
        Some(0),
        "a second git apply of oracle.patch must fail; stderr: {stderr}"
    );
    stderr
}

/// The harness's own preparation applies the patch cleanly: `OracleCopy::prepare`
/// requires `git apply` to exit 0, the three patched scripts then hash to the
/// recorded post-patch blobs, every other vendored file keeps its devflow blob,
/// and applying the patch again fails because it is already applied.
///
/// Counterpart: `patch_apply_without_ceiling_is_a_silent_no_op`, which shows an
/// exit code of 0 alone would not prove the patch applied.
#[test]
fn oracle_patch_applies_cleanly() {
    let dirs = ScenarioDirs::new("provenance", "patch_applies").expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let copy = OracleCopy::prepare(&dirs, &env).expect("the harness prepares the patched copy");

    let patched = blob_mismatches(&copy.dir, PATCHED_BLOBS, &dirs, &env);
    assert!(
        patched.is_empty(),
        "the patched scripts differ from the recorded post-patch blobs: {patched:?}"
    );
    let unpatched = unpatched_pairs();
    assert_eq!(
        unpatched.iter().map(|(path, _)| *path).collect::<Vec<_>>(),
        ["sync-workspace.sh", "cut-pr-branch.sh", "LICENSE"],
        "the files the patch must leave alone"
    );
    let changed = blob_mismatches(&copy.dir, unpatched, &dirs, &env);
    assert!(
        changed.is_empty(),
        "the patch changed a file it must leave at its devflow blob: {changed:?}"
    );
    let stderr = reapply_fails(&copy.dir, &env);
    assert!(
        stderr.contains("patch does not apply"),
        "the second apply must fail because the patch is already applied; stderr: {stderr}"
    );
}

/// The trap the ceiling exists for (RESEARCH Pitfall 1): inside a git work
/// tree, `git apply` resolves the patch's paths from the top of that tree, so
/// in a subdirectory it prints `Skipped patch '<file>'.` for every file and
/// still exits 0. Here the scenario root is made a repository and only
/// `GIT_CEILING_DIRECTORIES` is removed from the environment: the apply exits
/// 0, every file keeps its devflow blob, and `OracleCopy::prepare` refuses the
/// copy because a patched script lacks `WSX_ORACLE_DIR`. With the ceiling set,
/// git never looks above the scenario root, so the copy is not inside a work
/// tree and the patch applies (`oracle_patch_applies_cleanly`).
#[test]
fn patch_apply_without_ceiling_is_a_silent_no_op() {
    let dirs = ScenarioDirs::new("provenance", "no_ceiling").expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let no_ceiling: Env = env
        .iter()
        .filter(|(key, _)| key != "GIT_CEILING_DIRECTORIES")
        .cloned()
        .collect();
    assert_eq!(
        no_ceiling.len() + 1,
        env.len(),
        "exactly the ceiling is removed from the environment"
    );
    let init = command("git", &dirs.root, &no_ceiling)
        .args(["init", "--quiet"])
        .status()
        .expect("spawn git init");
    assert!(init.success(), "git init at the scenario root: {init}");

    let manual = dirs.root.join("manual");
    scratch_copy(&manual);
    let output = command("git", &manual, &no_ceiling)
        .args(["apply", "-v", "oracle.patch"])
        .output()
        .expect("spawn git apply");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the apply inside a work tree exits 0; stderr: {stderr}"
    );
    for (path, _) in PATCHED_BLOBS {
        let skipped = format!("Skipped patch '{path}'.");
        assert!(
            stderr.contains(&skipped),
            "git apply must report {skipped:?}; stderr: {stderr}"
        );
    }
    let changed = blob_mismatches(&manual, vendored_pairs(), &dirs, &no_ceiling);
    assert!(
        changed.is_empty(),
        "the skipped apply must leave every file at its devflow blob: {changed:?}"
    );

    let refusal = match OracleCopy::prepare(&dirs, &no_ceiling) {
        Ok(copy) => panic!(
            "OracleCopy::prepare must refuse a copy the patch skipped, got {}",
            copy.dir.display()
        ),
        Err(refusal) => refusal,
    };
    assert!(
        refusal.contains("lacks WSX_ORACLE_DIR"),
        "the refusal must name a script without WSX_ORACLE_DIR, got {refusal:?}"
    );
}

/// The arguments and stdin `script` gets in `work` of a devflow fixture: the
/// divergence check against `origin/develop`, a worktree for phase 1, and the
/// pre-push hook as git drives it for a push of the workspace branch.
fn script_inputs(script: &str, fixture: &Fixture) -> (Vec<OsString>, Option<Vec<u8>>) {
    match script {
        "check-workspace-divergence.sh" => (vec!["origin/develop".into()], None),
        "phase-worktree.sh" => (vec!["1".into()], None),
        "hooks/pre-push" => {
            let workspace = format!("refs/heads/{}", fixture.config.workspace);
            let push = Action::PrePush {
                checkout: WORK.to_owned(),
                remote: fixture.config.push_remote.clone(),
                updates: vec![PushUpdate {
                    local_ref: workspace.clone(),
                    remote_ref: workspace,
                }],
            };
            let input = push
                .push_input(fixture)
                .expect("read the push input")
                .expect("a pre-push action has push input");
            (
                vec![fixture.config.push_remote.clone().into(), input.url],
                Some(input.stdin),
            )
        }
        other => panic!("no inputs defined for {other}"),
    }
}

/// Runs patched `script` from a fresh patched copy in `work` of a fresh devflow
/// fixture, with `WSX_ORACLE_DIR` pointing at the copy (`oracle::run_script`,
/// the adapter's own invocation) or absent.
fn run_patched(script: &str, label: &str, with_oracle_dir: bool) -> ActionRun {
    let dirs = ScenarioDirs::new("provenance", label).expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    assert!(
        !env.iter().any(|(key, _)| key == "WSX_ORACLE_DIR"),
        "the scenario environment never carries WSX_ORACLE_DIR itself"
    );
    let fixture = Fixture::new(&dirs, env.clone(), RepoConfig::devflow()).expect("fixture");
    let copy = OracleCopy::prepare(&dirs, &env).expect("patched copy");
    let (args, stdin) = script_inputs(script, &fixture);
    let args: Vec<&OsStr> = args.iter().map(OsString::as_os_str).collect();
    let cwd = fixture.checkout(WORK);
    let run = if with_oracle_dir {
        oracle::run_script(&copy, script, &args, &cwd, &env, stdin.as_deref())
    } else {
        // `run_script` always sets the variable, so this is its command line
        // without it: the same program, script path, arguments, cwd, env and
        // stdin.
        let mut child = command("bash", &cwd, &env);
        child
            .arg(copy.dir.join(script))
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if stdin.is_some() {
            child.stdin(Stdio::piped());
        }
        oracle::spawn_and_wait(child, stdin.as_deref(), script)
    };
    run.unwrap_or_else(|err| panic!("{script}: {err}"))
}

/// A run summary for a failure message.
fn describe(script: &str, run: &ActionRun) -> String {
    format!(
        "{script}: exit {:?}\nstdout: {}\nstderr: {}",
        run.exit,
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    )
}

/// Each patched script stops at its first use of `WSX_ORACLE_DIR` when the
/// variable is absent: a non-zero exit with bash's `${WSX_ORACLE_DIR:?}`
/// error. This also proves each patched hunk is reached on this fixture.
///
/// Counterpart: `patched_scripts_run_with_oracle_dir`, the same runs with the
/// variable set, which exit 0, so the failure here comes from the variable.
#[test]
fn patched_scripts_require_oracle_dir() {
    let mut failures = Vec::new();
    for (index, (script, _)) in PATCHED_BLOBS.into_iter().enumerate() {
        let run = run_patched(script, &format!("no_oracle_dir_{index}"), false);
        let stderr = String::from_utf8_lossy(&run.stderr);
        if run.exit == Some(0) || !stderr.contains("WSX_ORACLE_DIR: parameter null or not set") {
            failures.push(describe(script, &run));
        }
    }
    assert!(
        failures.is_empty(),
        "without WSX_ORACLE_DIR each patched script must fail on the variable:\n{}",
        failures.join("\n\n")
    );
}

/// With `WSX_ORACLE_DIR` set by the adapter, each patched script runs to exit
/// 0 and does the work its patched line leads to: the divergence check reads
/// `FORBIDDEN_PATHS` from the copy and passes, `phase-worktree.sh` finds the
/// copy's `sync-workspace.sh` and syncs (it would print `not found; skipping
/// sync` and still exit 0 if the path were wrong), and the pre-push hook runs
/// the copy's divergence check.
#[test]
fn patched_scripts_run_with_oracle_dir() {
    let declared = "check-workspace-divergence: every difference from origin/develop is declared.";
    let expected_stdout = [
        ("check-workspace-divergence.sh", declared),
        ("phase-worktree.sh", "==> syncing workspace/dev"),
        ("hooks/pre-push", declared),
    ];
    let mut failures = Vec::new();
    for (index, (script, marker)) in expected_stdout.into_iter().enumerate() {
        let run = run_patched(script, &format!("with_oracle_dir_{index}"), true);
        if run.exit != Some(0) || !String::from_utf8_lossy(&run.stdout).contains(marker) {
            failures.push(format!(
                "expected stdout {marker:?}; {}",
                describe(script, &run)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "with WSX_ORACLE_DIR each patched script must run to exit 0:\n{}",
        failures.join("\n\n")
    );
}

/// Each recorded value `readme` lacks: `DEVFLOW_COMMIT`, then every blob in
/// `VENDORED_BLOBS` and `PATCHED_BLOBS`, in that order.
fn readme_missing_records(readme: &str) -> Vec<String> {
    [DEVFLOW_COMMIT]
        .into_iter()
        .chain(vendored_pairs().map(|(_, blob)| blob))
        .chain(PATCHED_BLOBS.map(|(_, blob)| blob))
        .filter(|record| !readme.contains(record))
        .map(str::to_owned)
        .collect()
}

/// The provenance README beside the vendored files.
fn oracle_readme() -> String {
    let path = Path::new(ORACLE_DIR).join("README.md");
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

/// `tests/oracle/README.md` records the pinned devflow commit and every blob
/// the provenance tests check, so the README and the constants cannot drift.
///
/// Counterpart: `readme_check_reports_a_missing_blob`, which shows the check
/// reports a README missing one blob.
#[test]
fn readme_records_the_pinned_commit_and_blobs() {
    let missing = readme_missing_records(&oracle_readme());
    assert!(
        missing.is_empty(),
        "tests/oracle/README.md lacks these recorded values: {missing:?}"
    );
}

/// The README text with the pre-push blob replaced, in memory only, is
/// reported as missing exactly that blob.
#[test]
fn readme_check_reports_a_missing_blob() {
    let pre_push = "67ef847ecf98dfaa1b124be5b10179b7a0134dc9";
    let readme = oracle_readme();
    assert!(
        readme.contains(pre_push),
        "the README must carry the pre-push blob for this control to remove it"
    );
    let edited = readme.replace(pre_push, "<blob removed>");
    assert_eq!(readme_missing_records(&edited), [pre_push]);
}
