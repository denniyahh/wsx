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

use super::{Error, render_argv};

/// Runs the `git` binary with a fixed environment snapshot.
pub struct Git {
    /// The complete environment every child gets, as ordered key/value pairs.
    env: Vec<(OsString, OsString)>,
}

impl Git {
    /// A helper whose children see exactly `env`.
    pub fn new(env: Vec<(OsString, OsString)>) -> Git {
        Git { env }
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
