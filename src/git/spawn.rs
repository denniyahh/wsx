//! The one constructor of `git` child processes.
//!
//! `Git` owns an explicit environment snapshot instead of reading the process
//! environment per call, so what a child sees is decided in one place and can
//! be tested without mutating the process. Every child starts from an empty
//! environment plus that snapshot, runs in an explicit working directory and
//! reads stdin from `/dev/null`, so git can never stop to wait for input.
//! Arguments pass through `Command::args`, never through a shell.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use super::{Error, render_argv};

/// The oldest git wsx runs, as (major, minor). CONTEXT "Running the git
/// binary": git 2.54 or newer is checked once, on the first `git` spawn, and
/// an older git is refused naming the version found and the one required.
const MIN_GIT_VERSION: (u32, u32) = (2, 54);

/// The (major, minor) of `git --version` output: the first line's token after
/// `git version `, its first two dot-separated parts. Anything after them
/// (`.windows.1`, ` (Apple Git-155)`) is ignored. `None` when the output does
/// not have that shape.
fn parse_git_version(_output: &[u8]) -> Option<(u32, u32)> {
    None
}

/// Runs the `git` binary with a fixed environment snapshot.
pub struct Git {
    /// The complete environment every child gets, as ordered key/value pairs.
    env: Vec<(OsString, OsString)>,
    /// Set once `git --version` has passed `MIN_GIT_VERSION`.
    version_checked: OnceLock<()>,
}

impl Git {
    /// A helper whose children see exactly `env`.
    pub fn new(env: Vec<(OsString, OsString)>) -> Git {
        Git {
            env,
            version_checked: OnceLock::new(),
        }
    }

    /// A child for `args` in `worktree` with the whole snapshot: an empty
    /// environment plus every snapshot pair, `worktree` as its working
    /// directory, stdin from `/dev/null`.
    fn base_command(&self, worktree: &Path, args: &[OsString]) -> Command {
        let mut command = Command::new("git");
        command
            .env_clear()
            .envs(self.env.iter().map(|(key, value)| (key, value)))
            .current_dir(worktree)
            .stdin(Stdio::null())
            .args(args);
        command
    }

    /// The child for `args` in `worktree`: an empty environment plus the
    /// snapshot, `worktree` as its working directory, stdin from `/dev/null`.
    fn command(&self, worktree: &Path, args: &[OsString]) -> Command {
        let mut command = Command::new("git");
        command
            .env_clear()
            .envs(self.env.iter().map(|(key, value)| (key, value)))
            .current_dir(worktree)
            .stdin(Stdio::null())
            .args(args);
        command
    }

    /// Runs `git <args>` in `worktree` and returns its stdout. A child that
    /// cannot be started is `Error::Io`; one that does not exit 0 is
    /// `Error::GitFailed` carrying its argv, exit code and stderr.
    pub fn run<I, S>(&self, worktree: &Path, args: I) -> Result<Vec<u8>, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let argv: Vec<OsString> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_os_string())
            .collect();
        let output = self
            .command(worktree, &argv)
            .output()
            .map_err(|source| Error::Io {
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
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    use super::super::Error;
    use super::super::testing::{self, Env, Scratch};
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
}
