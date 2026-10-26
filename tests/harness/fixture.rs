//! Fixture repositories, built step by step inside a scenario's scratch root.
//!
//! Every git call runs through `env::command` with the scenario environment,
//! so fixtures see only the harness global config. Fixture commits run on a
//! deterministic clock: commit k is authored and committed at `T0 + TICK * k`,
//! which keeps author dates known constants and keeps history walks from
//! reordering commits with equal timestamps. The action under test runs later,
//! at `ACTION_TIME`.
//!
//! Checkout names are paths relative to the scenario root (`work`,
//! `work/.worktrees/<x>`). File paths and commit messages are bytes end to end.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Output;

use crate::env::{Env, ScenarioDirs, command};

/// 2026-01-01T00:00:00Z: the date of fixture commit 0.
pub const T0: i64 = 1_767_225_600;
/// Seconds between consecutive fixture commits.
pub const TICK: i64 = 60;
/// The instant the action under test runs at, one day after `T0`.
pub const ACTION_TIME: i64 = T0 + 86_400;

/// The main checkout every preset clones.
pub const WORK: &str = "work";

/// A git identity.
#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub email: String,
}

impl Ident {
    /// The author of every fixture commit.
    pub fn ada() -> Ident {
        Ident {
            name: "Ada Author".to_owned(),
            email: "ada@example.invalid".to_owned(),
        }
    }

    /// The identity in the harness global config: the committer of fixture
    /// commits and the author and committer of anything the action creates.
    pub fn harness() -> Ident {
        Ident {
            name: "Harness".to_owned(),
            email: "harness@example.invalid".to_owned(),
        }
    }
}

/// How the fixture's workspace branch is laid out.
#[derive(Clone, Debug)]
pub enum Layout {
    /// devflow's shape: GSD planning files and both manifests on the workspace branch.
    Devflow,
}

/// A `.wsx.conf` line kind.
#[derive(Clone, Debug)]
pub enum ManifestKind {
    Only,
}

impl ManifestKind {
    fn keyword(&self) -> &'static [u8] {
        match self {
            ManifestKind::Only => b"only",
        }
    }
}

/// A scenario's repository configuration, stated explicitly.
#[derive(Clone, Debug)]
pub struct RepoConfig {
    pub layout: Layout,
    /// The integration branch, e.g. `develop`.
    pub integration: String,
    /// The workspace branch, e.g. `workspace/dev`.
    pub workspace: String,
    /// The remote the integration branch is fetched from.
    pub base_remote: String,
    /// The remote the workspace branch is pushed to.
    pub push_remote: String,
    /// Manifest entries beyond the preset's own, appended to `.wsx.conf`.
    pub manifest: Vec<(ManifestKind, Vec<u8>)>,
    /// Extra local config pairs for the work repository.
    pub extra_config: Vec<(String, String)>,
}

impl RepoConfig {
    /// devflow's own setup: `develop`, `workspace/dev`, one remote `origin`.
    pub fn devflow() -> RepoConfig {
        RepoConfig {
            layout: Layout::Devflow,
            integration: "develop".to_owned(),
            workspace: "workspace/dev".to_owned(),
            base_remote: "origin".to_owned(),
            push_remote: "origin".to_owned(),
            manifest: Vec::new(),
            extra_config: Vec::new(),
        }
    }
}

/// Manifest entries as `<kind> <path>` lines.
fn manifest_lines(entries: &[(ManifestKind, Vec<u8>)]) -> Vec<u8> {
    let mut lines = Vec::new();
    for (kind, path) in entries {
        lines.extend_from_slice(kind.keyword());
        lines.push(b' ');
        lines.extend_from_slice(path);
        lines.push(b'\n');
    }
    lines
}

/// A built fixture: the scenario root, the environment and the config.
pub struct Fixture {
    pub root: PathBuf,
    pub env: Env,
    pub config: RepoConfig,
    /// Fixture commits made so far; the next commit is dated at this tick.
    commits: i64,
    /// Scratch clones made so far, for unique names.
    scratch: u32,
}

/// `env` plus author and committer dates fixed at `epoch` UTC.
pub fn dated(env: &Env, epoch: i64) -> Env {
    let date = OsString::from(format!("@{epoch} +0000"));
    let mut env = env.clone();
    env.push(("GIT_AUTHOR_DATE".into(), date.clone()));
    env.push(("GIT_COMMITTER_DATE".into(), date));
    env
}

fn render(args: &[OsString]) -> String {
    args.iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

impl Fixture {
    /// Builds the preset `config.layout` names under `dirs.root`.
    pub fn new(dirs: &ScenarioDirs, env: Env, config: RepoConfig) -> Result<Fixture, String> {
        let mut fixture = Fixture {
            root: dirs.root.clone(),
            env,
            config,
            commits: 0,
            scratch: 0,
        };
        match fixture.config.layout {
            Layout::Devflow => fixture.build_devflow()?,
        }
        Ok(fixture)
    }

    /// devflow's preset: a bare base remote seeded with commit 0 on the
    /// integration branch, `work` cloned from it, and the workspace branch with
    /// commit 1 adding the GSD and manifest files, pushed and checked out.
    fn build_devflow(&mut self) -> Result<(), String> {
        let config = self.config.clone();
        let base = self.bare_remote(&config.base_remote)?;
        let seed = self.scratch_name();
        self.git_ok(".", ["init", "--quiet", "-b", &config.integration, &seed])?;
        self.write(&seed, b"README.md", b"# Fixture\n")?;
        self.write(&seed, b"src/a.txt", b"a\n")?;
        self.commit(&seed, b"chore: initial commit\n")?;
        let base_arg = base.as_os_str();
        self.git_ok(
            &seed,
            [
                OsStr::new("push"),
                OsStr::new("--quiet"),
                base_arg,
                OsStr::new(&config.integration),
            ],
        )?;
        self.remove_scratch(&seed)?;

        self.git_ok(
            ".",
            [
                OsStr::new("clone"),
                OsStr::new("--quiet"),
                OsStr::new("-o"),
                OsStr::new(&config.base_remote),
                base_arg,
                OsStr::new(WORK),
            ],
        )?;
        if config.push_remote != config.base_remote {
            let push = self.bare_remote(&config.push_remote)?;
            self.git_ok(
                WORK,
                [
                    OsStr::new("remote"),
                    OsStr::new("add"),
                    OsStr::new(&config.push_remote),
                    push.as_os_str(),
                ],
            )?;
        }
        let base_ref = format!("{}/{}", config.base_remote, config.integration);
        let mut local = vec![
            ("wsx.sourceBranch".to_owned(), config.workspace.clone()),
            ("wsx.baseRef".to_owned(), base_ref),
            ("commit.gpgsign".to_owned(), "false".to_owned()),
        ];
        local.extend(config.extra_config.iter().cloned());
        for (key, value) in &local {
            self.git_ok(WORK, ["config", "--local", key, value])?;
        }

        self.git_ok(WORK, ["checkout", "--quiet", "-b", &config.workspace])?;
        let mut wsx_conf = manifest_lines(&[
            (ManifestKind::Only, b".planning".to_vec()),
            (ManifestKind::Only, b".workspace-divergence".to_vec()),
        ]);
        let mut divergence = manifest_lines(&[(ManifestKind::Only, b".wsx.conf".to_vec())]);
        let extra = manifest_lines(&config.manifest);
        wsx_conf.extend_from_slice(&extra);
        divergence.extend_from_slice(&extra);
        self.write(
            WORK,
            b".planning/ROADMAP.md",
            b"# Roadmap\n\n### Phase 1: Example\n",
        )?;
        self.write(WORK, b".planning/STATE.md", b"# State\n")?;
        self.write(WORK, b".workspace-divergence", &divergence)?;
        self.write(WORK, b".wsx.conf", &wsx_conf)?;
        self.commit(WORK, b"chore(ws): add workspace files\n")?;
        self.git_ok(
            WORK,
            [
                "push",
                "--quiet",
                "-u",
                &config.push_remote,
                &config.workspace,
            ],
        )?;
        Ok(())
    }

    /// Creates the bare repository `remotes/<name>.git` and returns its path.
    fn bare_remote(&self, name: &str) -> Result<PathBuf, String> {
        let path = self.root.join("remotes").join(format!("{name}.git"));
        self.git_ok(
            ".",
            [
                OsStr::new("init"),
                OsStr::new("--quiet"),
                OsStr::new("--bare"),
                path.as_os_str(),
            ],
        )?;
        Ok(path)
    }

    fn scratch_name(&mut self) -> String {
        self.scratch += 1;
        format!("scratch-{}", self.scratch)
    }

    fn remove_scratch(&self, name: &str) -> Result<(), String> {
        let path = self.checkout(name);
        fs::remove_dir_all(&path).map_err(|err| format!("remove {}: {err}", path.display()))
    }

    /// The absolute path of a checkout named relative to the scenario root.
    pub fn checkout(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// Runs git in `checkout` with the scenario environment.
    pub fn git<I, S>(&self, checkout: &str, args: I) -> Result<Output, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.git_with_env(checkout, args, &self.env)
            .map(|(_, output)| output)
    }

    fn git_with_env<I, S>(
        &self,
        checkout: &str,
        args: I,
        env: &Env,
    ) -> Result<(Vec<OsString>, Output), String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<OsString> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_os_string())
            .collect();
        let output = command("git", &self.checkout(checkout), env)
            .args(&args)
            .output()
            .map_err(|err| format!("spawn git {} in {checkout}: {err}", render(&args)))?;
        Ok((args, output))
    }

    /// Runs git in `checkout` and returns its stdout, or an error carrying the
    /// command, exit code and stderr when it does not exit 0.
    pub fn git_ok<I, S>(&self, checkout: &str, args: I) -> Result<Vec<u8>, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<OsString> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_os_string())
            .collect();
        let output = self.git(checkout, &args)?;
        checked(checkout, &args, output)
    }

    /// Writes `content` to `path` in `checkout`, creating parent directories.
    pub fn write(&self, checkout: &str, path: &[u8], content: &[u8]) -> Result<(), String> {
        let full = self.checkout(checkout).join(OsStr::from_bytes(path));
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create {}: {err}", parent.display()))?;
        }
        fs::write(&full, content).map_err(|err| format!("write {}: {err}", full.display()))
    }

    /// Stages everything in `checkout` and commits it at the next tick, authored
    /// by Ada Author and committed by the config identity. The message goes
    /// through a file with `--cleanup=verbatim`, so its bytes are stored exactly
    /// (include the trailing newline).
    pub fn commit(&mut self, checkout: &str, message: &[u8]) -> Result<(), String> {
        let message_file = self.root.join("commit-message");
        fs::write(&message_file, message)
            .map_err(|err| format!("write {}: {err}", message_file.display()))?;
        self.git_ok(checkout, ["add", "--all"])?;
        let author = Ident::ada();
        let env = dated(&self.env, T0 + TICK * self.commits);
        let (args, output) = self.git_with_env(
            checkout,
            [
                OsStr::new("commit"),
                OsStr::new("--quiet"),
                OsStr::new("--cleanup=verbatim"),
                OsStr::new("--author"),
                OsStr::new(&format!("{} <{}>", author.name, author.email)),
                OsStr::new("--file"),
                message_file.as_os_str(),
            ],
            &env,
        )?;
        checked(checkout, &args, output)?;
        self.commits += 1;
        Ok(())
    }

    /// Advances `branch` on `remote`: clones it into a scratch checkout, runs
    /// `steps` there (they receive the scratch checkout's name), pushes the
    /// branch back and removes the scratch checkout.
    pub fn advance_remote(
        &mut self,
        remote: &str,
        branch: &str,
        steps: impl FnOnce(&mut Fixture, &str) -> Result<(), String>,
    ) -> Result<(), String> {
        let remote_path = self.root.join("remotes").join(format!("{remote}.git"));
        let scratch = self.scratch_name();
        self.git_ok(
            ".",
            [
                OsStr::new("clone"),
                OsStr::new("--quiet"),
                OsStr::new("-b"),
                OsStr::new(branch),
                remote_path.as_os_str(),
                OsStr::new(&scratch),
            ],
        )?;
        steps(self, &scratch)?;
        self.git_ok(&scratch, ["push", "--quiet", "origin", branch])?;
        self.remove_scratch(&scratch)
    }
}

fn checked(checkout: &str, args: &[OsString], output: Output) -> Result<Vec<u8>, String> {
    if output.status.code() == Some(0) {
        Ok(output.stdout)
    } else {
        Err(format!(
            "git {} in {checkout} exited {:?}; stderr: {}",
            render(args),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim_end()
        ))
    }
}
