//! wsx's git layer: repository resolution and reads through gix, history
//! changes through the `git` binary.
//!
//! wsx opens a repository only through `Repo::discover`, the single place gix
//! is asked to find one. It honours `GIT_DIR` and `GIT_INDEX_FILE` the way git
//! does, so wsx resolves the same repository inside a hook as git would, and
//! canonicalises the worktree, git dir and common dir once, there.
//!
//! Every history change goes through `Git`, which runs the real `git` binary,
//! so the user's hooks, signing, merge drivers and credentials apply exactly
//! as they would to a git command the user typed.
//!
//! gix types never leave this module: callers get wsx's own types (`ObjectId`,
//! raw-byte paths), and a gix error crosses the boundary only as its message.
//!
//! Every refusal names its cause: an `Error` renders a message a user can act
//! on without reading wsx's source.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "no caller until phase 5's wsx check; phase 5 removes this attribute"
    )
)]

mod reads;
mod repo;
mod spawn;
#[cfg(test)]
mod testing;

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

/// A git object id as lowercase hex, independent of the hash kind.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(String);

impl ObjectId {
    /// The wsx form of a gix object id: its lowercase hex.
    fn from_gix(id: &gix::oid) -> ObjectId {
        ObjectId(id.to_hex().to_string())
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why the git layer could not answer.
#[derive(Debug)]
pub enum Error {
    /// No repository was found from `start`; `reason` is gix's explanation.
    NotARepo { start: PathBuf, reason: String },
    /// A `git` child exited unsuccessfully. `argv` excludes the program name;
    /// `status` is `None` when a signal killed the child.
    GitFailed {
        argv: Vec<OsString>,
        status: Option<i32>,
        stderr: Vec<u8>,
    },
    /// An operating-system call failed; `what` names the call and its subject.
    Io {
        what: String,
        source: std::io::Error,
    },
    /// A gix operation failed; `what` names the operation, `message` is gix's.
    Gix { what: String, message: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotARepo { start, reason } => {
                write!(f, "not a git repository: {} ({reason})", start.display())
            }
            Error::GitFailed {
                argv,
                status,
                stderr,
            } => {
                let command = render_argv(argv);
                match status {
                    Some(code) => write!(f, "git {command} exited {code}")?,
                    None => write!(f, "git {command} was killed by a signal")?,
                }
                let stderr = String::from_utf8_lossy(stderr);
                let stderr = stderr.trim_end();
                if stderr.is_empty() {
                    Ok(())
                } else {
                    write!(f, ": {stderr}")
                }
            }
            Error::Io { what, source } => write!(f, "{what}: {source}"),
            Error::Gix { what, message } => write!(f, "{what}: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// `argv` as one line for a message: each argument lossily, single spaces.
fn render_argv(argv: &[OsString]) -> String {
    argv.iter()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}
