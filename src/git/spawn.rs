//! The one constructor of `git` child processes.
//!
//! `Git` owns an explicit environment snapshot instead of reading the process
//! environment per call, so what a child sees is decided in one place and can
//! be tested without mutating the process. Every child starts from an empty
//! environment plus that snapshot, runs in an explicit working directory and
//! reads stdin from `/dev/null`, so git can never stop to wait for input.
//! Arguments pass through `Command::args`, never through a shell.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use super::repo::Repo;
use super::{Error, render_argv};

/// The oldest git wsx runs, as (major, minor). CONTEXT "Running the git
/// binary": git 2.54 or newer is checked once, on the first `git` spawn, and
/// an older git is refused naming the version found and the one required.
const MIN_GIT_VERSION: (u32, u32) = (2, 54);

/// The first line of `output`, trimmed, lossily decoded.
fn first_line(output: &[u8]) -> String {
    String::from_utf8_lossy(output)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_owned()
}

/// One dot-separated version part as a number: ASCII digits only.
fn version_part(part: Option<&str>) -> Option<u32> {
    let part = part?;
    if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

/// The (major, minor) of `git --version` output: the first line's token after
/// `git version `, its first two dot-separated parts. Anything after them
/// (`.windows.1`, ` (Apple Git-155)`) is ignored. `None` when the output does
/// not have that shape.
fn parse_git_version(output: &[u8]) -> Option<(u32, u32)> {
    let line = first_line(output);
    let token = line
        .strip_prefix("git version ")?
        .split_whitespace()
        .next()?;
    let mut parts = token.split('.');
    Some((version_part(parts.next())?, version_part(parts.next())?))
}

/// What a refusal names as the version found: the token after
/// `git version `, or the whole first line when git printed something else.
fn reported_version(output: &[u8]) -> String {
    let line = first_line(output);
    match line
        .strip_prefix("git version ")
        .and_then(|rest| rest.split_whitespace().next())
    {
        Some(token) => token.to_owned(),
        None => line,
    }
}

/// Runs `command` (whose arguments are `argv`) in `worktree` and returns its
/// stdout. A child that cannot be started is `Error::Io`; one that does not
/// exit 0, a signal-killed one included, is `Error::GitFailed` carrying its
/// argv, exit code (`None` for a signal) and stderr.
fn finish(mut command: Command, argv: Vec<OsString>, worktree: &Path) -> Result<Vec<u8>, Error> {
    let output = command.output().map_err(|source| Error::Io {
        what: format!("spawn git {} in {}", render_argv(&argv), worktree.display()),
        source,
    })?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(Error::GitFailed {
            argv,
            status: output.status.code(),
            stderr: output.stderr,
        })
    }
}

/// A `git` child for `args` in `worktree`: an empty environment plus `env`,
/// `worktree` as its working directory, stdin from `/dev/null`.
fn child<'a>(
    worktree: &Path,
    args: &[OsString],
    env: impl Iterator<Item = &'a (OsString, OsString)>,
) -> Command {
    let mut command = Command::new("git");
    command
        .env_clear()
        .envs(env.map(|(key, value)| (key, value)))
        .current_dir(worktree)
        .stdin(Stdio::null())
        .args(args);
    command
}

/// Runs the `git` binary with a fixed environment snapshot.
///
/// A child aimed at wsx's own worktree gets the whole snapshot, so inside a
/// hook it sees the hook's `GIT_DIR` and `GIT_INDEX_FILE` exactly as git set
/// them. A child aimed at any other worktree loses every variable
/// `git rev-parse --local-env-vars` lists, so a hook's repository and index
/// never redirect it. The helper adds nothing to what the caller asked for:
/// no hook-skipping flag and no signing override.
///
/// The git version check and the `--local-env-vars` list are cached per
/// helper, not per process: each helper resolves `git` through its own
/// snapshot's PATH, so two helpers may run two different binaries, and what
/// one of them learned says nothing about the other. Production builds one
/// helper per process (`Git::from_process`), so there each runs once per
/// process, as CONTEXT decides.
pub struct Git {
    /// The complete environment every child gets, as ordered key/value pairs.
    env: Vec<(OsString, OsString)>,
    /// wsx's own worktree, canonical; `None` counts every target as another
    /// worktree.
    own_worktree: Option<PathBuf>,
    /// Set once `git --version` has passed `MIN_GIT_VERSION`.
    version_checked: OnceLock<()>,
    /// The names `git rev-parse --local-env-vars` printed, fetched on first
    /// need.
    local_env_vars: OnceLock<Vec<OsString>>,
}

impl Git {
    /// A helper whose children see exactly `env`.
    pub fn new(env: Vec<(OsString, OsString)>) -> Git {
        Git {
            env,
            own_worktree: None,
            version_checked: OnceLock::new(),
            local_env_vars: OnceLock::new(),
        }
    }

    /// The production helper: a snapshot of the process environment, taken
    /// once, here, with `repo`'s worktree (when it has one) as wsx's own
    /// worktree. Inside a hook the snapshot carries the hook's `GIT_DIR` and
    /// `GIT_INDEX_FILE`, which children aimed at that worktree keep and
    /// children aimed at any other worktree lose.
    pub fn from_process(repo: &Repo) -> Git {
        let git = Git::new(std::env::vars_os().collect());
        match repo.worktree() {
            Some(worktree) => git.with_own_worktree(worktree.to_path_buf()),
            None => git,
        }
    }

    /// This helper with `worktree` as wsx's own worktree, whose children keep
    /// the whole snapshot. `worktree` is expected canonical (`Repo::worktree()`
    /// is) and must be a worktree root: own-versus-other is exact canonical
    /// equality, so a subdirectory of the own worktree counts as another
    /// worktree and loses the hook's environment (the safe direction). Without
    /// an own worktree every child counts as aimed at another worktree.
    pub fn with_own_worktree(self, worktree: PathBuf) -> Git {
        Git {
            own_worktree: Some(worktree),
            ..self
        }
    }

    /// A child for `args` in `worktree` with the whole snapshot: an empty
    /// environment plus every snapshot pair, `worktree` as its working
    /// directory, stdin from `/dev/null`.
    fn base_command(&self, worktree: &Path, args: &[OsString]) -> Command {
        child(worktree, args, self.env.iter())
    }

    /// The names `git rev-parse --local-env-vars` prints, queried through
    /// `base_command` in `worktree` on first need and cached. A query that
    /// cannot be started is `Error::Io`; a non-zero exit or a signal is
    /// `Error::GitFailed` naming this query's argv, and an exit 0 that prints
    /// no name (nothing, or only whitespace) is `Error::GitEmptyOutput`, so no
    /// child aimed at another worktree is built without the list. Git 2.54
    /// always prints the list, so an empty one means a broken or substituted
    /// git, and removing nothing would let a hook's variables through.
    fn removal_list(&self, worktree: &Path) -> Result<&[OsString], Error> {
        if let Some(names) = self.local_env_vars.get() {
            return Ok(names);
        }
        let argv: Vec<OsString> = ["rev-parse", "--local-env-vars"]
            .into_iter()
            .map(OsString::from)
            .collect();
        let stdout = finish(self.base_command(worktree, &argv), argv.clone(), worktree)?;
        let names: Vec<OsString> = stdout
            .split(u8::is_ascii_whitespace)
            .filter(|name| !name.is_empty())
            .map(|name| OsStr::from_bytes(name).to_os_string())
            .collect();
        if names.is_empty() {
            return Err(Error::GitEmptyOutput {
                argv,
                expected: "variable names",
            });
        }
        Ok(self.local_env_vars.get_or_init(|| names))
    }

    /// The child for `args` in `worktree`: program `git`, `args` exactly as
    /// given, `worktree` as passed for its working directory, stdin from
    /// `/dev/null`, and an empty environment plus the snapshot. `worktree`
    /// must be a worktree root: it is canonicalised (`Error::Io` on failure)
    /// and compared for exact equality with the own worktree, so a
    /// subdirectory of the own worktree counts as another worktree. A target
    /// that is not the own worktree gets the snapshot without every name on
    /// git's `--local-env-vars` list; never a prefix test, because wsx nests
    /// sibling worktrees inside its own worktree directory.
    fn command(&self, worktree: &Path, args: &[OsString]) -> Result<Command, Error> {
        let target = std::fs::canonicalize(worktree).map_err(|source| Error::Io {
            what: format!("canonicalize {}", worktree.display()),
            source,
        })?;
        if self.own_worktree.as_deref() == Some(target.as_path()) {
            return Ok(self.base_command(worktree, args));
        }
        let removed = self.removal_list(worktree)?;
        let kept = self.env.iter().filter(|(key, _)| !removed.contains(key));
        Ok(child(worktree, args, kept))
    }

    /// Runs `git --version` unless this helper has already passed the check,
    /// and refuses a git older than `MIN_GIT_VERSION` or one whose version
    /// cannot be read, naming what git printed. Racing first callers may each
    /// run the check; none proceeds before its own check has passed.
    fn ensure_version(&self, worktree: &Path) -> Result<(), Error> {
        if self.version_checked.get().is_some() {
            return Ok(());
        }
        let argv = vec![OsString::from("--version")];
        let stdout = finish(self.base_command(worktree, &argv), argv, worktree)?;
        match parse_git_version(&stdout) {
            Some(version) if version >= MIN_GIT_VERSION => {
                self.version_checked.get_or_init(|| ());
                Ok(())
            }
            _ => Err(Error::GitTooOld {
                found: reported_version(&stdout),
                required: MIN_GIT_VERSION,
            }),
        }
    }

    /// Runs `git <args>` in `worktree` and returns its stdout. `worktree` must
    /// be a worktree root: a child aimed anywhere but wsx's own worktree
    /// (exact canonical equality, so a subdirectory of it counts as another
    /// worktree) loses git's repo-local variables. The first run on a helper
    /// checks the git version first, and a refused check spawns nothing else.
    /// A child that cannot be started is `Error::Io`; one that does not exit 0
    /// is `Error::GitFailed` carrying its argv, exit code and stderr.
    pub fn run<I, S>(&self, worktree: &Path, args: I) -> Result<Vec<u8>, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let argv: Vec<OsString> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_os_string())
            .collect();
        self.ensure_version(worktree)?;
        let command = self.command(worktree, &argv)?;
        finish(command, argv, worktree)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output, Stdio};

    use super::super::Error;
    use super::super::repo::Repo;
    use super::super::testing::{self, Env, Scratch, Worktrees};
    use super::{Git, MIN_GIT_VERSION, parse_git_version};

    /// The directories searched for the real git, in PATH order.
    const SYSTEM_BIN_DIRS: [&str; 3] = ["/usr/local/bin", "/usr/bin", "/bin"];

    /// The first executable `git` in the system bin directories.
    fn real_git() -> PathBuf {
        SYSTEM_BIN_DIRS
            .iter()
            .map(|dir| Path::new(dir).join("git"))
            .find(|path| {
                fs::metadata(path)
                    .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false)
            })
            .unwrap_or_else(|| panic!("no executable git in {SYSTEM_BIN_DIRS:?}"))
    }

    /// `path` as a single-quoted `/bin/sh` word.
    fn sh_quote(path: &Path) -> String {
        let text = path
            .to_str()
            .unwrap_or_else(|| panic!("{} is not UTF-8", path.display()));
        format!("'{}'", text.replace('\'', r"'\''"))
    }

    /// Writes `<root>/fakebin/git`, a `/bin/sh` script that appends `"$*"` to
    /// `<root>/git.log` and then runs `body`, and returns the scratch's
    /// hermetic environment with `<root>/fakebin` first on PATH. The child
    /// resolves a bare `git` through its own PATH, so the helper runs the fake.
    fn fake_git(scratch: &Scratch, body: &str) -> Env {
        let fakebin = scratch.root.join("fakebin");
        let script = fakebin.join("git");
        let log = sh_quote(&scratch.root.join("git.log"));
        let content = format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> {log}\n{body}\n");
        testing::write(&script, content.as_bytes()).expect("write the fake git");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755))
            .expect("make the fake git executable");
        let path = OsString::from(format!(
            "{}:{}",
            fakebin.display(),
            SYSTEM_BIN_DIRS.join(":")
        ));
        let mut env = scratch.env.clone();
        let entry = env
            .iter_mut()
            .find(|(key, _)| key == "PATH")
            .expect("the hermetic env sets PATH");
        entry.1 = path;
        env
    }

    /// A fake body that answers `--version` with `answer` and execs the real
    /// git for every other call.
    fn version_answer(answer: &str) -> String {
        format!(
            "if [ \"$*\" = \"--version\" ]; then echo '{answer}'; exit 0; fi\nexec {} \"$@\"",
            sh_quote(&real_git())
        )
    }

    /// A fake body that SIGKILLs itself when its arguments are exactly
    /// `argv` and execs the real git otherwise.
    fn kill_on(argv: &str) -> String {
        format!(
            "if [ \"$*\" = \"{argv}\" ]; then kill -KILL $$; fi\nexec {} \"$@\"",
            sh_quote(&real_git())
        )
    }

    /// The lines of `<root>/git.log`, one per fake git call.
    fn log_lines(scratch: &Scratch) -> Vec<String> {
        let log = scratch.root.join("git.log");
        fs::read_to_string(&log)
            .unwrap_or_else(|err| panic!("read {}: {err}", log.display()))
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// `git init -q <root>/repo` in the scratch, returning that path.
    fn init_repo(scratch: &Scratch) -> PathBuf {
        let repo = scratch.root.join("repo");
        testing::git_ok(
            scratch,
            &scratch.root,
            [OsString::from("init"), "-q".into(), repo.clone().into()],
        )
        .expect("git init");
        repo
    }

    #[test]
    fn git_version_output_parses_leniently() {
        assert_eq!(parse_git_version(b"git version 2.56.0\n"), Some((2, 56)));
        assert_eq!(
            parse_git_version(b"git version 2.54.0.windows.1\n"),
            Some((2, 54))
        );
        assert_eq!(
            parse_git_version(b"git version 2.55.1 (Apple Git-155)\n"),
            Some((2, 55))
        );
        assert_eq!(parse_git_version(b"git version 2\n"), None);
        assert_eq!(parse_git_version(b"nonsense\n"), None);
    }

    #[test]
    fn old_git_refused() {
        let scratch = Scratch::new("old-git").expect("scratch");
        let repo = init_repo(&scratch);
        let env = fake_git(&scratch, &version_answer("git version 2.53.0"));
        let result = Git::new(env).run(&repo, ["rev-parse", "--absolute-git-dir"]);
        match result {
            Err(Error::GitTooOld { found, required }) => {
                assert_eq!(found, "2.53.0");
                assert_eq!(required, (2, 54));
            }
            other => panic!("git 2.53.0 must be refused as GitTooOld, got {other:?}"),
        }
        assert_eq!(log_lines(&scratch), ["--version"]);
    }

    #[test]
    fn git_2_54_is_accepted() {
        let scratch = Scratch::new("git-2-54").expect("scratch");
        let repo = init_repo(&scratch);
        let env = fake_git(&scratch, &version_answer("git version 2.54.0"));
        let stdout = Git::new(env)
            .run(&repo, ["rev-parse", "--absolute-git-dir"])
            .expect("git 2.54.0 is accepted");
        assert_eq!(
            String::from_utf8_lossy(&stdout).trim_end(),
            repo.join(".git").to_string_lossy()
        );
        let lines = log_lines(&scratch);
        assert_eq!(lines.first().map(String::as_str), Some("--version"));
        assert_eq!(
            lines.last().map(String::as_str),
            Some("rev-parse --absolute-git-dir")
        );
        let without_list: Vec<&str> = lines
            .iter()
            .map(String::as_str)
            .filter(|line| *line != "rev-parse --local-env-vars")
            .collect();
        assert_eq!(without_list, ["--version", "rev-parse --absolute-git-dir"]);
    }

    #[test]
    fn unparseable_git_version_is_refused() {
        let scratch = Scratch::new("git-nonsense").expect("scratch");
        let repo = init_repo(&scratch);
        let env = fake_git(&scratch, &version_answer("nonsense"));
        let result = Git::new(env).run(&repo, ["rev-parse", "--absolute-git-dir"]);
        match result {
            Err(Error::GitTooOld { found, required }) => {
                assert_eq!(found, "nonsense");
                assert_eq!(required, MIN_GIT_VERSION);
            }
            other => panic!("an unreadable version must be refused as GitTooOld, got {other:?}"),
        }
        assert!(
            !log_lines(&scratch)
                .iter()
                .any(|line| line == "rev-parse --absolute-git-dir"),
            "the requested command ran after a refused version check"
        );
    }

    #[test]
    fn version_is_checked_once_per_helper() {
        let scratch = Scratch::new("git-version-once").expect("scratch");
        let repo = init_repo(&scratch);
        let env = fake_git(&scratch, &format!("exec {} \"$@\"", sh_quote(&real_git())));
        let version_lines = || {
            log_lines(&scratch)
                .iter()
                .filter(|line| *line == "--version")
                .count()
        };

        let git = Git::new(env.clone());
        git.run(&repo, ["rev-parse", "--absolute-git-dir"])
            .expect("first run");
        git.run(&repo, ["rev-parse", "--absolute-git-dir"])
            .expect("second run");
        assert_eq!(version_lines(), 1, "one helper checks the version once");

        Git::new(env)
            .run(&repo, ["rev-parse", "--absolute-git-dir"])
            .expect("run on a second helper");
        assert_eq!(version_lines(), 2, "a second helper checks again");
    }

    #[test]
    fn failing_child_reports_argv_status_and_stderr() {
        let scratch = Scratch::new("git-failing-child").expect("scratch");
        let repo = init_repo(&scratch);
        let result =
            Git::new(scratch.env.clone()).run(&repo, ["rev-parse", "--verify", "refs/heads/nope"]);
        match result {
            Err(Error::GitFailed {
                argv,
                status,
                stderr,
            }) => {
                assert_eq!(argv, ["rev-parse", "--verify", "refs/heads/nope"]);
                assert_eq!(status, Some(128));
                assert!(
                    stderr.starts_with(b"fatal: "),
                    "stderr: {}",
                    String::from_utf8_lossy(&stderr)
                );
            }
            other => panic!("a non-zero exit must be GitFailed, got {other:?}"),
        }
    }

    #[test]
    fn signal_killed_child_is_a_failure() {
        let scratch = Scratch::new("git-killed-child").expect("scratch");
        let repo = init_repo(&scratch);
        let env = fake_git(&scratch, &kill_on("rev-parse --absolute-git-dir"));
        let result = Git::new(env).run(&repo, ["rev-parse", "--absolute-git-dir"]);
        match result {
            Err(Error::GitFailed { argv, status, .. }) => {
                assert_eq!(argv, ["rev-parse", "--absolute-git-dir"]);
                assert_eq!(status, None, "a signal-killed child has no exit code");
            }
            other => panic!("a signal-killed child must be GitFailed, got {other:?}"),
        }
    }

    /// The planted variables of a hook running in wt1, at the fixture's real
    /// paths.
    fn hook_vars(worktrees: &Worktrees) -> Env {
        let git_dir = worktrees.main.join(".git/worktrees/wt1");
        vec![
            ("GIT_DIR".into(), git_dir.clone().into()),
            ("GIT_INDEX_FILE".into(), git_dir.join("index").into()),
            ("GIT_WORK_TREE".into(), worktrees.wt1.clone().into()),
            ("GIT_PREFIX".into(), OsString::new()),
            ("GIT_CONFIG_PARAMETERS".into(), "'core.abbrev'='12'".into()),
        ]
    }

    /// The scratch environment plus a wt1 hook's variables.
    fn hook_snapshot(scratch: &Scratch, worktrees: &Worktrees) -> Env {
        let mut env = scratch.env.clone();
        env.extend(hook_vars(worktrees));
        env
    }

    /// git's repo-local variable names, queried independently of the helper,
    /// with every planted hook name asserted on it so a vacuous list cannot
    /// pass a removal test.
    fn local_env_var_list(scratch: &Scratch, worktrees: &Worktrees) -> BTreeSet<OsString> {
        let stdout = testing::git_ok(scratch, &scratch.root, ["rev-parse", "--local-env-vars"])
            .expect("git rev-parse --local-env-vars");
        let names: BTreeSet<OsString> = String::from_utf8(stdout)
            .expect("the list is ASCII")
            .split_ascii_whitespace()
            .map(OsString::from)
            .collect();
        for (name, _) in hook_vars(worktrees) {
            assert!(names.contains(&name), "{name:?} is not on {names:?}");
        }
        names
    }

    /// `env` as a key-to-value map: environment order carries no meaning.
    fn env_map(env: &Env) -> BTreeMap<OsString, OsString> {
        env.iter().cloned().collect()
    }

    /// The environment a built command sets, as a map. Panics on a removal
    /// entry: after `env_clear` the helper only ever adds pairs.
    fn command_env(command: &std::process::Command) -> BTreeMap<OsString, OsString> {
        command
            .get_envs()
            .map(|(key, value)| {
                let value = value.unwrap_or_else(|| panic!("{key:?} is marked for removal"));
                (key.to_os_string(), value.to_os_string())
            })
            .collect()
    }

    /// `snapshot` without the names on `removed`.
    fn without(snapshot: &Env, removed: &BTreeSet<OsString>) -> BTreeMap<OsString, OsString> {
        env_map(snapshot)
            .into_iter()
            .filter(|(key, _)| !removed.contains(key))
            .collect()
    }

    #[test]
    fn sibling_target_removes_local_env_vars() {
        let scratch = Scratch::new("env-sibling").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).expect("linked worktrees");
        let snapshot = hook_snapshot(&scratch, &worktrees);
        let removed = local_env_var_list(&scratch, &worktrees);
        let git = Git::new(snapshot.clone()).with_own_worktree(worktrees.wt1.clone());

        let command = git
            .command(&worktrees.wt2, &["status".into()])
            .expect("build the child");
        let env = command_env(&command);
        for name in &removed {
            assert!(
                !env.contains_key(name),
                "{name:?} reached a sibling's child"
            );
        }
        assert_eq!(env, without(&snapshot, &removed));
    }

    #[test]
    fn own_target_through_a_symlink_keeps_the_env() {
        let scratch = Scratch::new("env-own-symlink").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).expect("linked worktrees");
        let link = scratch.root.join("wt1-link");
        std::os::unix::fs::symlink(&worktrees.wt1, &link).expect("symlink wt1-link -> wt1");
        let snapshot = hook_snapshot(&scratch, &worktrees);
        let git = Git::new(snapshot.clone()).with_own_worktree(worktrees.wt1.clone());

        let command = git
            .command(&link, &["status".into()])
            .expect("build the child");
        assert_eq!(command.get_current_dir(), Some(link.as_path()));
        let env = command_env(&command);
        assert!(env.contains_key(&OsString::from("GIT_DIR")));
        assert_eq!(env, env_map(&snapshot));
    }

    #[test]
    fn nested_sibling_counts_as_another_worktree() {
        let scratch = Scratch::new("env-nested").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).expect("linked worktrees");
        assert!(
            worktrees.wt2.starts_with(&worktrees.main),
            "wt2 {} must be nested in main {}",
            worktrees.wt2.display(),
            worktrees.main.display()
        );
        let snapshot = hook_snapshot(&scratch, &worktrees);
        let removed = local_env_var_list(&scratch, &worktrees);
        let git = Git::new(snapshot.clone()).with_own_worktree(worktrees.main.clone());

        let command = git
            .command(&worktrees.wt2, &["status".into()])
            .expect("build the child");
        let env = command_env(&command);
        for name in &removed {
            assert!(
                !env.contains_key(name),
                "{name:?} reached a nested sibling's child"
            );
        }
        assert_eq!(env, without(&snapshot, &removed));
    }

    #[test]
    fn child_gets_exact_argv_cwd_and_no_bypass() {
        let scratch = Scratch::new("env-exact").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).expect("linked worktrees");
        let snapshot = hook_snapshot(&scratch, &worktrees);
        let removed = local_env_var_list(&scratch, &worktrees);
        let git = Git::new(snapshot.clone()).with_own_worktree(worktrees.wt1.clone());
        let args: Vec<OsString> = ["commit", "-q", "-m", "message"]
            .into_iter()
            .map(OsString::from)
            .collect();

        for (target, expected_env) in [
            (&worktrees.wt2, without(&snapshot, &removed)),
            (&worktrees.wt1, env_map(&snapshot)),
        ] {
            let command = git.command(target, &args).expect("build the child");
            assert_eq!(command.get_program(), "git");
            assert_eq!(command.get_args().collect::<Vec<_>>(), args);
            assert_eq!(command.get_current_dir(), Some(target.as_path()));
            assert_eq!(
                command_env(&command),
                expected_env,
                "env for {}",
                target.display()
            );
        }
    }

    #[test]
    fn killed_list_query_is_a_failure_naming_its_argv() {
        let scratch = Scratch::new("env-list-killed").expect("scratch");
        let repo = init_repo(&scratch);
        let env = fake_git(&scratch, &kill_on("rev-parse --local-env-vars"));
        let result = Git::new(env).run(&repo, ["rev-parse", "--absolute-git-dir"]);
        match result {
            Err(Error::GitFailed { argv, status, .. }) => {
                assert_eq!(argv, ["rev-parse", "--local-env-vars"]);
                assert_eq!(status, None, "a signal-killed child has no exit code");
            }
            other => panic!("a killed list query must be GitFailed naming it, got {other:?}"),
        }
        assert!(
            !log_lines(&scratch)
                .iter()
                .any(|line| line == "rev-parse --absolute-git-dir"),
            "the requested command ran without the removal list"
        );
    }

    /// The empty blob's id: what `git hash-object --stdin` prints for a
    /// stdin that holds nothing, `/dev/null` included.
    const EMPTY_BLOB: &str = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";

    /// The bytes a parent feeds its re-executed child's stdin.
    const STDIN_MARKER: &[u8] = b"wsx-stdin-marker\n";

    /// The value of the child's `key` marker after asserting it exited 0;
    /// either failure prints both of the child's streams.
    fn child_fact(output: &Output, key: &str) -> String {
        assert_eq!(
            output.status.code(),
            Some(0),
            "the child failed\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        testing::marker(output, key).unwrap_or_else(|err| panic!("{err}"))
    }

    #[test]
    fn children_get_null_stdin() {
        let scratch = Scratch::new("null-stdin").expect("scratch");
        let repo = init_repo(&scratch);
        let output = testing::reexec(
            "git::spawn::tests::null_stdin_child",
            &scratch.env,
            &repo,
            STDIN_MARKER,
        )
        .expect("re-execute the child");
        assert_eq!(child_fact(&output, "stdin_blob"), EMPTY_BLOB);
    }

    #[test]
    #[ignore = "child half of children_get_null_stdin; runs only when re-executed"]
    fn null_stdin_child() {
        let cwd = std::env::current_dir().expect("read the working directory");
        let repo = Repo::discover(&cwd).expect("discover the child's repository");
        let worktree = repo.worktree().expect("the repository has a worktree");
        let stdout = Git::from_process(&repo)
            .run(worktree, ["hash-object", "--stdin"])
            .expect("git hash-object --stdin through the helper");
        testing::child_print("stdin_blob", String::from_utf8_lossy(&stdout).trim());
    }

    #[test]
    fn piped_stdin_reaches_an_unmanaged_child() {
        let scratch = Scratch::new("piped-stdin").expect("scratch");
        let repo = init_repo(&scratch);
        let file = scratch.root.join("stdin-marker");
        testing::write(&file, STDIN_MARKER).expect("write the marker file");
        let expected = testing::git_ok(
            &scratch,
            &scratch.root,
            [OsString::from("hash-object"), file.into()],
        )
        .expect("git hash-object <marker file>");
        let expected = String::from_utf8_lossy(&expected).trim().to_owned();
        assert_ne!(expected, EMPTY_BLOB);

        let output = testing::reexec(
            "git::spawn::tests::piped_stdin_child",
            &scratch.env,
            &repo,
            STDIN_MARKER,
        )
        .expect("re-execute the child");
        assert_eq!(child_fact(&output, "stdin_blob"), expected);
    }

    #[test]
    #[ignore = "child half of piped_stdin_reaches_an_unmanaged_child; runs only when re-executed"]
    fn piped_stdin_child() {
        let cwd = std::env::current_dir().expect("read the working directory");
        // `output()` defaults stdin to null; inherit it so the child's own
        // piped stdin reaches git.
        let output = Command::new("git")
            .args(["hash-object", "--stdin"])
            .current_dir(&cwd)
            .stdin(Stdio::inherit())
            .output()
            .expect("spawn git hash-object --stdin");
        assert!(
            output.status.success(),
            "git hash-object --stdin exited {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        testing::child_print("stdin_blob", String::from_utf8_lossy(&output.stdout).trim());
    }

    /// The hooks `signing_repo` installs; each appends its own name to the
    /// hook log.
    const LOGGED_HOOKS: [&str; 3] = ["pre-commit", "commit-msg", "pre-merge-commit"];

    /// A repository whose own local config signs every commit, with logging
    /// hooks, built in `<root>/repo`; returns it and the hook log path
    /// `<root>/hooks.log`.
    ///
    /// `main` holds `a.txt` at `T0` and `side` adds `side.txt` at `T0 + TICK`,
    /// both committed unsigned before any hook exists; `main` is checked out.
    /// Then a throwaway ed25519 key (`<root>/signkey`, no passphrase), an
    /// `allowed_signers` file naming it for the scratch identity, and local
    /// config `gpg.format=ssh`, `user.signingkey`, `commit.gpgsign=true` and
    /// `gpg.ssh.allowedSignersFile` (without that file `%G?` cannot print
    /// `G`). Last, executable `/bin/sh` hooks for every `LOGGED_HOOKS` name.
    fn signing_repo(scratch: &Scratch) -> Result<(PathBuf, PathBuf), String> {
        let root = &scratch.root;
        let repo = root.join("repo");
        fs::create_dir_all(&repo).map_err(|err| format!("create {}: {err}", repo.display()))?;
        testing::git_ok(scratch, &repo, ["init", "-q", "-b", "main"])?;
        testing::write(&repo.join("a.txt"), b"a\n")?;
        testing::commit_all(scratch, &repo, "base", testing::T0)?;
        testing::git_ok(scratch, &repo, ["branch", "side"])?;
        testing::git_ok(scratch, &repo, ["switch", "-q", "side"])?;
        testing::write(&repo.join("side.txt"), b"side\n")?;
        testing::commit_all(scratch, &repo, "side", testing::T0 + testing::TICK)?;
        testing::git_ok(scratch, &repo, ["switch", "-q", "main"])?;

        let key = root.join("signkey");
        let keygen = [
            OsStr::new("-q"),
            OsStr::new("-t"),
            OsStr::new("ed25519"),
            OsStr::new("-N"),
            OsStr::new(""),
            OsStr::new("-C"),
            OsStr::new("unit"),
            OsStr::new("-f"),
            key.as_os_str(),
        ];
        let output = testing::command("ssh-keygen", root, &scratch.env)
            .args(keygen)
            .output()
            .map_err(|err| format!("spawn ssh-keygen: {err}"))?;
        if !output.status.success() {
            return Err(format!(
                "ssh-keygen exited {:?}; stderr: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim_end()
            ));
        }
        let public_path = root.join("signkey.pub");
        let public = fs::read(&public_path)
            .map_err(|err| format!("read {}: {err}", public_path.display()))?;
        let fields: Vec<&[u8]> = public
            .split(u8::is_ascii_whitespace)
            .filter(|field| !field.is_empty())
            .collect();
        let [kind, base64, ..] = fields[..] else {
            return Err(format!(
                "unexpected public key \"{}\"",
                public.escape_ascii()
            ));
        };
        let mut allowed = b"unit@example.invalid namespaces=\"git\" ".to_vec();
        allowed.extend_from_slice(kind);
        allowed.push(b' ');
        allowed.extend_from_slice(base64);
        allowed.push(b'\n');
        let allowed_path = root.join("allowed_signers");
        testing::write(&allowed_path, &allowed)?;
        let signing_config = [
            ("gpg.format", OsStr::new("ssh")),
            ("user.signingkey", key.as_os_str()),
            ("commit.gpgsign", OsStr::new("true")),
            ("gpg.ssh.allowedSignersFile", allowed_path.as_os_str()),
        ];
        for (name, value) in signing_config {
            testing::git_ok(
                scratch,
                &repo,
                [
                    OsStr::new("config"),
                    OsStr::new("--local"),
                    OsStr::new(name),
                    value,
                ],
            )?;
        }

        let log = root.join("hooks.log");
        for name in LOGGED_HOOKS {
            let hook = repo.join(".git/hooks").join(name);
            let script = format!("#!/bin/sh\nprintf '%s\\n' {name} >> {}\n", sh_quote(&log));
            testing::write(&hook, script.as_bytes())?;
            fs::set_permissions(&hook, fs::Permissions::from_mode(0o755))
                .map_err(|err| format!("make {} executable: {err}", hook.display()))?;
        }
        Ok((repo, log))
    }

    /// `git log --format=%G? -2` in `repo`: the signature state of HEAD and
    /// the commit before it, one letter each.
    fn last_two_signatures(scratch: &Scratch, repo: &Path) -> Vec<String> {
        let stdout = testing::git_ok(scratch, repo, ["log", "--format=%G?", "-2"])
            .expect("git log --format=%G? -2");
        String::from_utf8_lossy(&stdout)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// How often each hook name appears in the hook log; empty when no hook
    /// ever wrote it.
    fn hook_runs(log: &Path) -> BTreeMap<String, usize> {
        let content = match fs::read_to_string(log) {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(err) => panic!("read {}: {err}", log.display()),
        };
        let mut runs = BTreeMap::new();
        for line in content.lines() {
            *runs.entry(line.to_owned()).or_insert(0) += 1;
        }
        runs
    }

    /// Asserts HEAD in `repo` is a merge commit (two parents).
    fn assert_head_is_a_merge(scratch: &Scratch, repo: &Path) {
        let stdout = testing::git_ok(scratch, repo, ["rev-list", "--parents", "-1", "HEAD"])
            .expect("git rev-list --parents -1 HEAD");
        let ids = String::from_utf8_lossy(&stdout).split_whitespace().count();
        assert_eq!(ids, 3, "HEAD is not a two-parent merge");
    }

    #[test]
    fn commit_and_merge_run_hooks_and_sign() {
        let scratch = Scratch::new("hooks-sign").expect("scratch");
        let (repo, log) = signing_repo(&scratch).expect("signing fixture");
        let git = Git::new(scratch.env.clone()).with_own_worktree(repo.clone());
        testing::write(&repo.join("c.txt"), b"c\n").expect("write c.txt");
        testing::git_ok(&scratch, &repo, ["add", "c.txt"]).expect("stage c.txt");

        git.run(&repo, ["commit", "-m", "c"])
            .expect("commit through the helper");
        git.run(&repo, ["merge", "--no-ff", "--no-edit", "side"])
            .expect("merge through the helper");

        assert_head_is_a_merge(&scratch, &repo);
        assert_eq!(last_two_signatures(&scratch, &repo), ["G", "G"]);
        let expected: BTreeMap<String, usize> = [
            ("commit-msg".to_owned(), 2),
            ("pre-commit".to_owned(), 1),
            ("pre-merge-commit".to_owned(), 1),
        ]
        .into();
        assert_eq!(hook_runs(&log), expected);
    }

    #[test]
    fn explicit_bypass_skips_hooks_and_signing() {
        let scratch = Scratch::new("hooks-bypass").expect("scratch");
        let (repo, log) = signing_repo(&scratch).expect("signing fixture");
        let git = Git::new(scratch.env.clone()).with_own_worktree(repo.clone());
        testing::write(&repo.join("c.txt"), b"c\n").expect("write c.txt");
        testing::git_ok(&scratch, &repo, ["add", "c.txt"]).expect("stage c.txt");

        git.run(
            &repo,
            [
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--no-verify",
                "-m",
                "c",
            ],
        )
        .expect("bypassing commit through the helper");
        git.run(
            &repo,
            [
                "-c",
                "commit.gpgsign=false",
                "merge",
                "--no-verify",
                "--no-ff",
                "--no-edit",
                "side",
            ],
        )
        .expect("bypassing merge through the helper");

        assert_head_is_a_merge(&scratch, &repo);
        assert_eq!(last_two_signatures(&scratch, &repo), ["N", "N"]);
        assert_eq!(hook_runs(&log), BTreeMap::new());
    }

    /// A fake body that answers `rev-parse --local-env-vars` with the
    /// `printf` format `answer` and execs the real git for every other call.
    fn list_answer(answer: &str) -> String {
        format!(
            "if [ \"$*\" = \"rev-parse --local-env-vars\" ]; then printf '{answer}'; exit 0; fi\nexec {} \"$@\"",
            sh_quote(&real_git())
        )
    }

    /// Runs `rev-parse --absolute-git-dir` aimed at wt2 through a helper whose
    /// own worktree is wt1 and whose git answers the list query with `answer`.
    fn sibling_run_with_list(
        name: &str,
        answer: &str,
    ) -> (Scratch, Worktrees, Result<Vec<u8>, Error>) {
        let scratch = Scratch::new(name).expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).expect("linked worktrees");
        let env = fake_git(&scratch, &list_answer(answer));
        let result = Git::new(env)
            .with_own_worktree(worktrees.wt1.clone())
            .run(&worktrees.wt2, ["rev-parse", "--absolute-git-dir"]);
        (scratch, worktrees, result)
    }

    /// Asserts `result` refuses the empty list under the list query's argv,
    /// that the query did run through the fake, and that the requested
    /// command never did.
    fn assert_list_refused(scratch: &Scratch, result: Result<Vec<u8>, Error>) {
        match result {
            Err(Error::GitEmptyOutput { argv, expected }) => {
                assert_eq!(argv, ["rev-parse", "--local-env-vars"]);
                assert_eq!(expected, "variable names");
            }
            other => panic!("an empty removal list must fail closed, got {other:?}"),
        }
        let lines = log_lines(scratch);
        assert!(
            lines
                .iter()
                .any(|line| line == "rev-parse --local-env-vars"),
            "the list query never reached the fake git: {lines:?}"
        );
        assert!(
            !lines
                .iter()
                .any(|line| line == "rev-parse --absolute-git-dir"),
            "the requested command ran with an empty removal list: {lines:?}"
        );
    }

    #[test]
    fn empty_list_query_fails_closed_before_the_command() {
        let (scratch, _worktrees, result) = sibling_run_with_list("env-list-empty", "");
        assert_list_refused(&scratch, result);
    }

    #[test]
    fn whitespace_only_list_query_fails_closed_before_the_command() {
        let (scratch, _worktrees, result) = sibling_run_with_list("env-list-blank", " \\n\\t\\n");
        assert_list_refused(&scratch, result);
    }

    #[test]
    fn one_name_list_query_lets_the_command_run() {
        let (scratch, worktrees, result) = sibling_run_with_list("env-list-one", "GIT_DIR\\n");
        let stdout = result.expect("a one-name list lets the sibling's command run");
        assert_eq!(
            String::from_utf8_lossy(&stdout).trim_end(),
            worktrees.main.join(".git/worktrees/wt2").to_string_lossy()
        );
        assert_eq!(
            log_lines(&scratch).last().map(String::as_str),
            Some("rev-parse --absolute-git-dir")
        );
    }
}
