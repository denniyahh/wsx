//! The oracle adapter: devflow's vendored scripts, run from a patched scratch
//! copy.
//!
//! `tests/oracle` stays byte-identical to devflow. `OracleCopy::prepare`
//! copies it into the scenario root and applies `oracle.patch` there with the
//! scenario environment, whose `GIT_CEILING_DIRECTORIES` stops `git apply` from
//! discovering the wsx checkout above `target/` (where it would skip every
//! file and still exit 0). It then refuses the copy unless every patched
//! script carries `WSX_ORACLE_DIR`, so a silently skipped patch can never run.

use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::action::Action;
use crate::env::{Env, ScenarioDirs, command};
use crate::fixture::Fixture;

/// The vendored oracle, as committed.
const ORACLE_SOURCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/oracle");

/// The scripts `oracle.patch` changes; each must carry `WSX_ORACLE_DIR` after
/// the apply.
const PATCHED_SCRIPTS: [&str; 3] = [
    "check-workspace-divergence.sh",
    "phase-worktree.sh",
    "hooks/pre-push",
];

/// What one run of an implementation did.
#[derive(Clone, Debug)]
pub struct ActionRun {
    /// `None` when the process was killed by a signal.
    pub exit: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// The implementation has no counterpart for this action.
#[derive(Clone, Debug)]
pub struct NotImplemented {
    pub action: Action,
    pub why: &'static str,
}

/// Why an adapter produced no run.
#[derive(Clone, Debug)]
pub enum AdapterError {
    NotImplemented(NotImplemented),
    /// The process could not be started or fed its input.
    Spawn(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdapterError::NotImplemented(NotImplemented { action, why }) => {
                write!(f, "not implemented: {action}: {why}")
            }
            AdapterError::Spawn(reason) => write!(f, "{reason}"),
        }
    }
}

/// A patched scratch copy of the oracle.
pub struct OracleCopy {
    pub dir: PathBuf,
}

impl OracleCopy {
    /// Copies `tests/oracle` to `<root>/oracle`, applies `oracle.patch` there
    /// with `env`, and checks that every patched script mentions
    /// `WSX_ORACLE_DIR`.
    pub fn prepare(dirs: &ScenarioDirs, env: &Env) -> Result<OracleCopy, String> {
        let dir = dirs.root.join("oracle");
        copy_tree(Path::new(ORACLE_SOURCE), &dir)?;
        let output = command("git", &dir, env)
            .args(["apply", "oracle.patch"])
            .output()
            .map_err(|err| format!("spawn git apply in {}: {err}", dir.display()))?;
        if output.status.code() != Some(0) {
            return Err(format!(
                "git apply oracle.patch exited {:?}; stderr: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim_end()
            ));
        }
        for script in PATCHED_SCRIPTS {
            let path = dir.join(script);
            let text = fs::read(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
            if !text.windows(14).any(|window| window == b"WSX_ORACLE_DIR") {
                return Err(format!(
                    "{script} lacks WSX_ORACLE_DIR after git apply: the patch was not applied"
                ));
            }
        }
        Ok(OracleCopy { dir })
    }
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|err| format!("create {}: {err}", to.display()))?;
    let entries = fs::read_dir(from).map_err(|err| format!("read {}: {err}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("read {}: {err}", from.display()))?;
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        let kind = entry
            .file_type()
            .map_err(|err| format!("stat {}: {err}", source.display()))?;
        if kind.is_dir() {
            copy_tree(&source, &target)?;
        } else {
            fs::copy(&source, &target)
                .map_err(|err| format!("copy {}: {err}", source.display()))?;
        }
    }
    Ok(())
}

/// Runs `bash <copy>/<script> <args>` in `cwd` with `env`, plus
/// `WSX_ORACLE_DIR=<copy>` for this child only, feeding `stdin` if given.
pub fn run_script(
    copy: &OracleCopy,
    script: &str,
    args: &[&OsStr],
    cwd: &Path,
    env: &Env,
    stdin: Option<&[u8]>,
) -> Result<ActionRun, AdapterError> {
    let mut child = command("bash", cwd, env);
    child
        .arg(copy.dir.join(script))
        .args(args)
        .env("WSX_ORACLE_DIR", &copy.dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if stdin.is_some() {
        child.stdin(Stdio::piped());
    }
    spawn_and_wait(child, stdin, script)
}

/// Spawns `child`, writes `stdin` to it if given, and collects its run.
pub fn spawn_and_wait(
    mut child: std::process::Command,
    stdin: Option<&[u8]>,
    what: &str,
) -> Result<ActionRun, AdapterError> {
    let fail =
        |step: &str, err: std::io::Error| AdapterError::Spawn(format!("{step} {what}: {err}"));
    let mut process = child.spawn().map_err(|err| fail("spawn", err))?;
    if let Some(input) = stdin {
        let mut pipe = process
            .stdin
            .take()
            .ok_or_else(|| AdapterError::Spawn(format!("no stdin pipe for {what}")))?;
        pipe.write_all(input)
            .map_err(|err| fail("write stdin to", err))?;
    }
    let output = process
        .wait_with_output()
        .map_err(|err| fail("wait for", err))?;
    Ok(ActionRun {
        exit: output.status.code(),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

/// `env` plus `vars`, for one child.
fn with_vars(env: &Env, vars: &[(&str, &str)]) -> Env {
    let mut env = env.clone();
    env.extend(
        vars.iter()
            .map(|(key, value)| ((*key).into(), (*value).into())),
    );
    env
}

/// Whether `text` is a phase number as the oracle scripts accept one: digits
/// and dots only, and not empty.
fn is_phase_number(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
}

/// Runs `action` on the oracle, or says why the oracle has no counterpart.
pub fn run(
    action: &Action,
    fixture: &Fixture,
    copy: &OracleCopy,
    env: &Env,
) -> Result<ActionRun, AdapterError> {
    let not_implemented = |why| {
        Err(AdapterError::NotImplemented(NotImplemented {
            action: action.clone(),
            why,
        }))
    };
    match action {
        Action::Sync { checkout } => {
            let config = &fixture.config;
            run_script(
                copy,
                "sync-workspace.sh",
                &[
                    OsStr::new(&config.base_remote),
                    OsStr::new(&config.integration),
                ],
                &fixture.checkout(checkout),
                env,
                None,
            )
        }
        Action::Check { checkout } => {
            let config = &fixture.config;
            let base_ref = format!("{}/{}", config.base_remote, config.integration);
            run_script(
                copy,
                "check-workspace-divergence.sh",
                &[OsStr::new(&base_ref)],
                &fixture.checkout(checkout),
                env,
                None,
            )
        }
        Action::NewWorktree {
            checkout,
            branch,
            no_sync,
        } => {
            let Some(phase) = branch
                .strip_prefix("feature/phase-")
                .filter(|phase| is_phase_number(phase))
            else {
                return not_implemented(
                    "phase-worktree.sh derives the branch from a phase number \
                     (feature/phase-<N>), so a worktree for any other branch is Rust-only",
                );
            };
            let mut args = vec![OsStr::new(phase)];
            if *no_sync {
                args.push(OsStr::new("--no-sync"));
            }
            run_script(
                copy,
                "phase-worktree.sh",
                &args,
                &fixture.checkout(checkout),
                &with_vars(env, &[("WORKSPACE_BASE", &fixture.config.workspace)]),
                None,
            )
        }
        Action::Cut { .. } | Action::PrePush { .. } => {
            not_implemented("mapped to its script in plan 02-07")
        }
        Action::RemoveWorktree { .. } => {
            not_implemented("the oracle has no command that removes a worktree")
        }
    }
}
