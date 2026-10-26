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
//!
//! Three shapes are built and each is pinned by a test here: devflow's own
//! (planning files and both manifests), a repository carrying no planning,
//! devflow or agent files, and a fork whose push remote is not the remote the
//! integration branch comes from.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Output;

use crate::env::{Env, ScenarioDirs, command, hermetic_env, inherited_env};

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
    /// No GSD, devflow or agent files anywhere: the workspace branch adds only
    /// a personal note and a `.wsx.conf` naming it.
    NoGsd,
}

/// A `.wsx.conf` line kind.
#[derive(Clone, Debug)]
pub enum ManifestKind {
    Only,
    Modified,
}

impl ManifestKind {
    fn keyword(&self) -> &'static [u8] {
        match self {
            ManifestKind::Only => b"only",
            ManifestKind::Modified => b"modified",
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
    /// Whether commits made after the preset is built are signed, with a
    /// throwaway SSH key generated for the scenario. Off in every preset.
    pub signing: bool,
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
            signing: false,
        }
    }

    /// devflow's branches and remote, with the no-GSD layout.
    pub fn no_gsd() -> RepoConfig {
        RepoConfig {
            layout: Layout::NoGsd,
            ..RepoConfig::devflow()
        }
    }

    /// Fetches the integration branch from `base` and pushes the workspace
    /// branch to `push`. When they differ, each is its own bare remote, `work`
    /// is cloned from `base` under that name, and only `base` holds the
    /// integration branch.
    pub fn with_remotes(mut self, base: &str, push: &str) -> RepoConfig {
        self.base_remote = base.to_owned();
        self.push_remote = push.to_owned();
        self
    }

    /// Appends `<kind> <path>` to the preset's manifests.
    pub fn with_manifest_entry(mut self, kind: ManifestKind, path: &[u8]) -> RepoConfig {
        self.manifest.push((kind, path.to_vec()));
        self
    }

    /// Sets `key` to `value` in the work repository's local config.
    pub fn with_local_config(mut self, key: &str, value: &str) -> RepoConfig {
        self.extra_config.push((key.to_owned(), value.to_owned()));
        self
    }

    /// Signs every commit made in the work repository after the preset is
    /// built, with a key generated for the scenario.
    pub fn with_signing(mut self) -> RepoConfig {
        self.signing = true;
        self
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
        fixture.build()?;
        Ok(fixture)
    }

    /// Every preset: a bare base remote seeded with commit 0 on the integration
    /// branch, `work` cloned from it (plus a separate bare push remote when the
    /// config names one), and the workspace branch with commit 1 adding the
    /// layout's files, pushed to the push remote and checked out.
    fn build(&mut self) -> Result<(), String> {
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
        let extra = manifest_lines(&config.manifest);
        match config.layout {
            Layout::Devflow => self.write_devflow_files(&extra)?,
            Layout::NoGsd => self.write_no_gsd_files(&extra)?,
        }
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
        if config.signing {
            self.enable_signing()?;
        }
        Ok(())
    }

    /// The preset's last step when the config declares signing: a throwaway
    /// ed25519 key generated in the scenario root (never a real key, removed
    /// with the TempDir), and local config that signs every later commit in
    /// the work repository and its worktrees with it. The key is named by its
    /// private file path: the hermetic environment has no SSH agent.
    fn enable_signing(&self) -> Result<(), String> {
        let private_key = self.root.join("signkey");
        let keygen = [
            OsStr::new("-q"),
            OsStr::new("-t"),
            OsStr::new("ed25519"),
            OsStr::new("-N"),
            OsStr::new(""),
            OsStr::new("-C"),
            OsStr::new("harness"),
            OsStr::new("-f"),
            private_key.as_os_str(),
        ];
        let output = command("ssh-keygen", &self.root, &self.env)
            .args(keygen)
            .output()
            .map_err(|err| format!("spawn ssh-keygen: {err}"))?;
        if output.status.code() != Some(0) {
            return Err(format!(
                "ssh-keygen exited {:?}; stderr: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim_end()
            ));
        }
        let public_path = self.root.join("signkey.pub");
        let public = fs::read(&public_path)
            .map_err(|err| format!("read {}: {err}", public_path.display()))?;
        let fields: Vec<&[u8]> = public
            .split(|byte| byte.is_ascii_whitespace())
            .filter(|field| !field.is_empty())
            .collect();
        let [kind, base64, ..] = fields[..] else {
            return Err(format!(
                "unexpected public key \"{}\"",
                public.escape_ascii()
            ));
        };
        let mut allowed = format!("{} namespaces=\"git\" ", Ident::harness().email).into_bytes();
        allowed.extend_from_slice(kind);
        allowed.push(b' ');
        allowed.extend_from_slice(base64);
        allowed.push(b'\n');
        let allowed_path = self.root.join("allowed_signers");
        fs::write(&allowed_path, allowed)
            .map_err(|err| format!("write {}: {err}", allowed_path.display()))?;
        let signing_config = [
            ("gpg.format", OsStr::new("ssh")),
            ("user.signingkey", private_key.as_os_str()),
            ("commit.gpgsign", OsStr::new("true")),
            ("gpg.ssh.allowedSignersFile", allowed_path.as_os_str()),
        ];
        for (key, value) in signing_config {
            self.git_ok(
                WORK,
                [
                    OsStr::new("config"),
                    OsStr::new("--local"),
                    OsStr::new(key),
                    value,
                ],
            )?;
        }
        Ok(())
    }

    /// devflow's workspace files: the GSD planning files, `.wsx.conf` naming
    /// them and `.workspace-divergence` naming `.wsx.conf`; `extra` lines are
    /// appended to both manifests.
    fn write_devflow_files(&self, extra: &[u8]) -> Result<(), String> {
        let mut wsx_conf = manifest_lines(&[
            (ManifestKind::Only, b".planning".to_vec()),
            (ManifestKind::Only, b".workspace-divergence".to_vec()),
        ]);
        let mut divergence = manifest_lines(&[(ManifestKind::Only, b".wsx.conf".to_vec())]);
        wsx_conf.extend_from_slice(extra);
        divergence.extend_from_slice(extra);
        self.write(
            WORK,
            b".planning/ROADMAP.md",
            b"# Roadmap\n\n### Phase 1: Example\n",
        )?;
        self.write(WORK, b".planning/STATE.md", b"# State\n")?;
        self.write(WORK, b".workspace-divergence", &divergence)?;
        self.write(WORK, b".wsx.conf", &wsx_conf)
    }

    /// The no-GSD workspace files: one personal note and a `.wsx.conf` naming
    /// its directory, with `extra` lines appended; nothing else.
    fn write_no_gsd_files(&self, extra: &[u8]) -> Result<(), String> {
        let mut wsx_conf = manifest_lines(&[(ManifestKind::Only, b"notes".to_vec())]);
        wsx_conf.extend_from_slice(extra);
        self.write(WORK, b"notes/todo.md", b"todo\n")?;
        self.write(WORK, b".wsx.conf", &wsx_conf)
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
        let env = self.tick();
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
        checked(checkout, &args, output).map(|_| ())
    }

    /// Runs a raw git command that creates one commit (a merge, a
    /// cherry-pick) in `checkout`, dated at the next tick like `commit`, and
    /// returns its stdout. Author and committer come from the command and the
    /// config, not from `commit`'s Ada Author.
    pub fn git_tick<I, S>(&mut self, checkout: &str, args: I) -> Result<Vec<u8>, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let env = self.tick();
        let (args, output) = self.git_with_env(checkout, args, &env)?;
        checked(checkout, &args, output)
    }

    /// Advances the clock: the scenario environment dated at the current tick,
    /// after which the next fixture commit gets the following tick.
    fn tick(&mut self) -> Env {
        let env = dated(&self.env, T0 + TICK * self.commits);
        self.commits += 1;
        env
    }

    /// Deletes `path` (a file, or a directory with everything under it) from
    /// `checkout`'s working tree. Nothing is staged; the next `commit` records
    /// the removal.
    pub fn remove(&self, checkout: &str, path: &[u8]) -> Result<(), String> {
        let full = self.checkout(checkout).join(OsStr::from_bytes(path));
        let metadata = fs::symlink_metadata(&full)
            .map_err(|err| format!("remove {}: {err}", full.display()))?;
        let removed = if metadata.is_dir() {
            fs::remove_dir_all(&full)
        } else {
            fs::remove_file(&full)
        };
        removed.map_err(|err| format!("remove {}: {err}", full.display()))
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

/// Builds `config` in a scratch tree named for a fixture test. The dirs must
/// outlive the fixture: dropping them removes the tree.
fn build_for_test(name: &str, config: RepoConfig) -> (ScenarioDirs, Fixture) {
    let dirs = ScenarioDirs::new("fixture", name).expect("scenario dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let fixture = Fixture::new(&dirs, env, config).expect("fixture preset");
    (dirs, fixture)
}

/// Output the fixture tests expect to be text. Strict: a non-UTF-8 byte fails
/// the test instead of being rendered lossily into a passing comparison.
fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_else(|err| panic!("expected UTF-8 text: {err}"))
}

/// The text of `path` in the tree of `rev`, read in `checkout`.
fn blob(fixture: &Fixture, checkout: &str, rev: &str, path: &str) -> String {
    text(
        fixture
            .git_ok(checkout, ["cat-file", "blob", &format!("{rev}:{path}")])
            .unwrap_or_else(|err| panic!("read {rev}:{path} in {checkout}: {err}")),
    )
}

/// Whether `name` resolves to a ref in `checkout`, from `show-ref`'s exit
/// code: 0 present, 1 absent (measured on git 2.56.0); anything else fails
/// the test.
fn has_ref(fixture: &Fixture, checkout: &str, name: &str) -> bool {
    let output = fixture
        .git(checkout, ["show-ref", "--verify", "--quiet", name])
        .expect("spawn git show-ref");
    match output.status.code() {
        Some(0) => true,
        Some(1) => false,
        other => panic!("git show-ref --verify {name} in {checkout} exited {other:?}"),
    }
}

/// The devflow layout carries the preset's planning files, both manifests with
/// every extra entry appended, the workspace branch checked out and pushed,
/// and the local config the oracle reads.
#[test]
fn devflow_layout_matches_the_preset() {
    let config = RepoConfig::devflow()
        .with_manifest_entry(ManifestKind::Only, b"notes")
        .with_manifest_entry(ManifestKind::Modified, b"README.md");
    let (_dirs, fixture) = build_for_test("devflow_layout", config);
    let ws = "refs/heads/workspace/dev";

    assert_eq!(
        blob(&fixture, WORK, ws, ".wsx.conf"),
        "only .planning\nonly .workspace-divergence\nonly notes\nmodified README.md\n",
        ".wsx.conf on workspace/dev"
    );
    assert_eq!(
        blob(&fixture, WORK, ws, ".workspace-divergence"),
        "only .wsx.conf\nonly notes\nmodified README.md\n",
        ".workspace-divergence on workspace/dev"
    );
    let roadmap = blob(&fixture, WORK, ws, ".planning/ROADMAP.md");
    assert!(
        roadmap.lines().any(|line| line == "### Phase 1: Example"),
        ".planning/ROADMAP.md holds the phase heading: {roadmap:?}"
    );
    assert_eq!(
        text(
            fixture
                .git_ok(WORK, ["symbolic-ref", "HEAD"])
                .expect("HEAD of work")
        ),
        format!("{ws}\n"),
        "work has workspace/dev checked out"
    );
    let local_tip = text(fixture.git_ok(WORK, ["rev-parse", ws]).expect("local tip"));
    let pushed_tip = text(
        fixture
            .git_ok("remotes/origin.git", ["rev-parse", ws])
            .expect("workspace/dev on the push remote"),
    );
    assert_eq!(pushed_tip, local_tip, "workspace/dev on the push remote");
    for (key, value) in [
        ("wsx.sourceBranch", "workspace/dev\n"),
        ("wsx.baseRef", "origin/develop\n"),
    ] {
        assert_eq!(
            text(
                fixture
                    .git_ok(WORK, ["config", "--local", "--get", key])
                    .unwrap_or_else(|err| panic!("local {key}: {err}"))
            ),
            value,
            "local config {key}"
        );
    }
}

/// Names that mark a repository as GSD-, devflow- or agent-shaped.
const NOT_IN_A_NO_GSD_REPO: [&[u8]; 6] = [
    b".planning",
    b".workspace-divergence",
    b"CLAUDE.md",
    b".claude",
    b".codex",
    b"devflow.toml",
];

/// Every file and directory under `dir` whose name is one of
/// `NOT_IN_A_NO_GSD_REPO`, skipping the top-level `.git`.
fn marked_paths_on_disk(dir: &Path, top: bool, found: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
        let name = entry.file_name();
        if top && name == ".git" {
            continue;
        }
        let path = entry.path();
        if NOT_IN_A_NO_GSD_REPO.contains(&name.as_bytes()) {
            found.push(path.clone());
        }
        let file_type = entry
            .file_type()
            .unwrap_or_else(|err| panic!("stat {}: {err}", path.display()));
        if file_type.is_dir() {
            marked_paths_on_disk(&path, false, found);
        }
    }
}

/// The no-GSD layout carries no planning, devflow or agent file on any ref of
/// the work repository or a remote, nor in the checkout, and still has the
/// `.wsx.conf` its workspace branch is built around.
#[test]
fn no_gsd_layout_has_no_gsd_devflow_or_agent_files() {
    let (_dirs, fixture) = build_for_test("no_gsd_layout", RepoConfig::no_gsd());

    let mut found = Vec::new();
    let mut refs_seen = Vec::new();
    for repo in [WORK, "remotes/origin.git"] {
        let refs = fixture
            .git_ok(repo, ["for-each-ref", "--format=%(refname)"])
            .expect("list refs");
        for name in String::from_utf8(refs).expect("ref names").lines() {
            let paths = fixture
                .git_ok(
                    repo,
                    ["ls-tree", "-r", "-z", "--name-only", "--full-tree", name],
                )
                .unwrap_or_else(|err| panic!("list the tree of {name} in {repo}: {err}"));
            for path in paths
                .split(|byte| *byte == 0)
                .filter(|path| !path.is_empty())
            {
                if path
                    .split(|byte| *byte == b'/')
                    .any(|part| NOT_IN_A_NO_GSD_REPO.contains(&part))
                {
                    found.push(format!("{repo} {name}: {}", String::from_utf8_lossy(path)));
                }
            }
            refs_seen.push(format!("{repo} {name}"));
        }
    }
    assert!(
        refs_seen.contains(&"work refs/heads/workspace/dev".to_owned())
            && refs_seen.contains(&"remotes/origin.git refs/heads/workspace/dev".to_owned()),
        "the scan covers workspace/dev in work and on the remote: {refs_seen:?}"
    );
    assert!(found.is_empty(), "no-GSD refs carry {found:?}");

    let mut on_disk = Vec::new();
    marked_paths_on_disk(&fixture.checkout(WORK), true, &mut on_disk);
    assert!(
        on_disk.is_empty(),
        "the no-GSD checkout carries {on_disk:?}"
    );

    let ws = "refs/heads/workspace/dev";
    assert_eq!(
        text(
            fixture
                .git_ok(
                    WORK,
                    ["ls-tree", "-r", "-z", "--name-only", "--full-tree", ws]
                )
                .expect("workspace/dev tree")
        ),
        ".wsx.conf\0README.md\0notes/todo.md\0src/a.txt\0",
        "workspace/dev holds exactly these files"
    );
    assert_eq!(
        blob(&fixture, WORK, ws, ".wsx.conf"),
        "only notes\n",
        ".wsx.conf on workspace/dev"
    );
}

/// The fork layout fetches the integration branch from the base remote only:
/// the push remote holds the workspace branch and never the integration branch.
#[test]
fn fork_layout_has_no_integration_branch_on_the_push_remote() {
    let config = RepoConfig::devflow()
        .with_remotes("upstream", "origin")
        .with_local_config("fixture.layout", "fork");
    let (_dirs, fixture) = build_for_test("fork_layout", config);

    assert_eq!(
        text(fixture.git_ok(WORK, ["remote"]).expect("list remotes")),
        "origin\nupstream\n",
        "work has exactly the two remotes"
    );
    assert!(has_ref(&fixture, WORK, "refs/remotes/upstream/develop"));
    assert!(!has_ref(&fixture, WORK, "refs/remotes/origin/develop"));
    assert!(has_ref(&fixture, WORK, "refs/remotes/origin/workspace/dev"));
    assert!(!has_ref(
        &fixture,
        WORK,
        "refs/remotes/upstream/workspace/dev"
    ));
    assert!(!has_ref(
        &fixture,
        "remotes/origin.git",
        "refs/heads/develop"
    ));
    assert!(has_ref(
        &fixture,
        "remotes/origin.git",
        "refs/heads/workspace/dev"
    ));
    assert!(has_ref(
        &fixture,
        "remotes/upstream.git",
        "refs/heads/develop"
    ));
    assert!(!has_ref(
        &fixture,
        "remotes/upstream.git",
        "refs/heads/workspace/dev"
    ));
    for (key, value) in [
        ("wsx.baseRef", "upstream/develop\n"),
        ("fixture.layout", "fork\n"),
    ] {
        assert_eq!(
            text(
                fixture
                    .git_ok(WORK, ["config", "--local", "--get", key])
                    .unwrap_or_else(|err| panic!("local {key}: {err}"))
            ),
            value,
            "local config {key}"
        );
    }
}

/// Every fixture commit, whether made by `commit` (a removal included) or by
/// `git_tick`, gets its own author and committer date one tick after the
/// previous fixture commit, in the order the steps ran.
#[test]
fn fixture_commits_have_distinct_dates() {
    let (_dirs, mut fixture) = build_for_test("distinct_dates", RepoConfig::devflow());
    let steps = |fixture: &mut Fixture| -> Result<(), String> {
        fixture.git_ok(WORK, ["checkout", "--quiet", "-b", "feature"])?;
        fixture.write(WORK, b"notes/feature.md", b"feature\n")?;
        fixture.commit(WORK, b"feat: feature work\n")?;
        fixture.git_ok(WORK, ["checkout", "--quiet", "workspace/dev"])?;
        fixture.write(WORK, b"notes/ws.md", b"ws\n")?;
        fixture.commit(WORK, b"chore(ws): add a workspace note\n")?;
        fixture.git_tick(
            WORK,
            [
                "merge",
                "--quiet",
                "--no-ff",
                "-m",
                "merge feature",
                "feature",
            ],
        )?;
        fixture.remove(WORK, b"notes/ws.md")?;
        fixture.commit(WORK, b"chore(ws): drop the workspace note\n")
    };
    steps(&mut fixture).expect("fixture steps");

    let log = fixture
        .git_ok(WORK, ["log", "--all", "--format=%at %ct %s"])
        .expect("log every ref");
    let mut dates = BTreeMap::new();
    for line in String::from_utf8(log).expect("log text").lines() {
        let mut fields = line.splitn(3, ' ');
        let mut epoch = || -> i64 {
            fields
                .next()
                .and_then(|field| field.parse().ok())
                .unwrap_or_else(|| panic!("log line {line:?}"))
        };
        let (author, committer) = (epoch(), epoch());
        let subject = fields.next().unwrap_or_else(|| panic!("log line {line:?}"));
        assert_eq!(
            dates.insert(subject.to_owned(), (author, committer)),
            None,
            "subject {subject:?} appears once"
        );
    }
    let expected: BTreeMap<String, (i64, i64)> = [
        "chore: initial commit",
        "chore(ws): add workspace files",
        "feat: feature work",
        "chore(ws): add a workspace note",
        "merge feature",
        "chore(ws): drop the workspace note",
    ]
    .into_iter()
    .zip(0..)
    .map(|(subject, tick)| (subject.to_owned(), (T0 + TICK * tick, T0 + TICK * tick)))
    .collect();
    assert_eq!(dates, expected, "author and committer date of each commit");

    fixture
        .git_ok(
            WORK,
            ["rev-parse", "--verify", "--quiet", "workspace/dev~1^2"],
        )
        .expect("the git_tick commit is a merge with a second parent");
    assert!(
        fixture
            .git_ok(WORK, ["cat-file", "-e", "workspace/dev:notes/ws.md"])
            .is_err(),
        "the removal is committed"
    );
}
