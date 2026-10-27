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

// Nothing in `main` calls this module until phase 5's `wsx check`, which
// removes this attribute. `cfg_attr(not(test), ...)`: the tests use every
// item, so on the test target the expectation would be unfulfilled. The
// reason is short so rustfmt keeps the attribute on one line.
#![cfg_attr(not(test), expect(dead_code, reason = "no caller until phase 5"))]

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
    /// The repository at `path` is owned by another user and no global or
    /// system `safe.directory` trusts it; git refuses it too.
    ForeignOwner { path: PathBuf },
    /// The repository at `path` stores its refs in the `format` backend
    /// (`extensions.refStorage`), which this version of wsx cannot read.
    Reftable { path: PathBuf, format: String },
    /// The `git` binary reports version `found`, older than `required`
    /// (major, minor).
    GitTooOld { found: String, required: (u32, u32) },
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
            // The fix line is git's own: a global `safe.directory` is the one
            // both git and gix honour (gix ignores `-c safe.directory`).
            Error::ForeignOwner { path } => write!(
                f,
                "refusing {path}: it is owned by another user, and git refuses such a repository too\n\
                 To trust it, run:\n\
                 \tgit config --global --add safe.directory {path}",
                path = path.display()
            ),
            Error::Reftable { path, format } => write!(
                f,
                "refusing {}: its refs use the '{format}' storage format (extensions.refStorage); \
                 this version of wsx reads only the files backend and cannot read {format} repositories",
                path.display()
            ),
            Error::GitTooOld {
                found,
                required: (major, minor),
            } => write!(
                f,
                "git {found} is too old: wsx needs git {major}.{minor} or newer"
            ),
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::io;
    use std::path::PathBuf;

    use super::Error;

    fn argv(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn error_messages_name_their_cause() {
        let not_a_repo = Error::NotARepo {
            start: PathBuf::from("/s"),
            reason: "r".to_owned(),
        };
        assert_eq!(not_a_repo.to_string(), "not a git repository: /s (r)");

        let foreign = Error::ForeignOwner {
            path: PathBuf::from("/r"),
        };
        assert_eq!(
            foreign.to_string(),
            "refusing /r: it is owned by another user, and git refuses such a repository too\n\
             To trust it, run:\n\
             \tgit config --global --add safe.directory /r"
        );

        let reftable = Error::Reftable {
            path: PathBuf::from("/r"),
            format: "reftable".to_owned(),
        };
        assert_eq!(
            reftable.to_string(),
            "refusing /r: its refs use the 'reftable' storage format (extensions.refStorage); \
             this version of wsx reads only the files backend and cannot read reftable repositories"
        );

        let too_old = Error::GitTooOld {
            found: "2.53.0".to_owned(),
            required: (2, 54),
        };
        assert_eq!(
            too_old.to_string(),
            "git 2.53.0 is too old: wsx needs git 2.54 or newer"
        );

        let failed = Error::GitFailed {
            argv: argv(&["merge-base", "a", "b"]),
            status: Some(128),
            stderr: b"fatal: x\n".to_vec(),
        };
        assert_eq!(
            failed.to_string(),
            "git merge-base a b exited 128: fatal: x"
        );

        let failed_quietly = Error::GitFailed {
            argv: argv(&["merge-base", "a", "b"]),
            status: Some(1),
            stderr: Vec::new(),
        };
        assert_eq!(failed_quietly.to_string(), "git merge-base a b exited 1");

        let killed = Error::GitFailed {
            argv: argv(&["merge-base", "a", "b"]),
            status: None,
            stderr: b"y\n".to_vec(),
        };
        assert_eq!(
            killed.to_string(),
            "git merge-base a b was killed by a signal: y"
        );

        let io = Error::Io {
            what: "canonicalize /m".to_owned(),
            source: io::Error::new(io::ErrorKind::NotFound, "gone"),
        };
        assert_eq!(io.to_string(), "canonicalize /m: gone");

        let gix = Error::Gix {
            what: "resolve x".to_owned(),
            message: "m".to_owned(),
        };
        assert_eq!(gix.to_string(), "resolve x: m");
    }
}
