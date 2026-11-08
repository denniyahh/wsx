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

mod config;
mod quote;
mod reads;
mod refs;
mod repo;
mod spawn;
#[cfg(test)]
pub(crate) mod testing;

pub use quote::shell_word;
pub use quote::{c_quote, c_unquote};
pub use refs::{RefTarget, RootEntry, is_branch_name};
pub use repo::Repo;

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

    /// The gix form of this id in a repository whose objects are hashed
    /// with `kind`: hex decoding only, never revspec parsing, so no ref is
    /// read and no prefix or describe output is expanded. Anything but a
    /// full-length hex id is refused, naming the id. An id of another hash
    /// kind is refused here too, as git refuses it, because gix-odb would
    /// `debug_assert` on it (and, in a release build, read it as missing).
    fn to_gix(&self, kind: gix::hash::Kind) -> Result<gix::ObjectId, Error> {
        let failed = |message: String| Error::Gix {
            what: format!("read object id {self}"),
            message,
        };
        let id =
            gix::ObjectId::from_hex(self.0.as_bytes()).map_err(|err| failed(err.to_string()))?;
        if id.kind() == kind {
            Ok(id)
        } else {
            Err(failed(format!("a {} id in a {kind} repository", id.kind())))
        }
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
    /// A `git` child exited 0 but printed nothing usable where wsx needs
    /// output; `expected` names what it should have printed.
    GitEmptyOutput {
        argv: Vec<OsString>,
        expected: &'static str,
    },
    /// The repository-path variable `name` holds the relative path `value`
    /// while wsx runs in `cwd`, below the worktree root `worktree`. gix
    /// resolves such a value against `cwd`, and git against the worktree
    /// root (or takes `cwd` as the worktree of a relative `GIT_DIR`), so
    /// wsx's reads and its git children would use different files.
    RelativeRepoEnv {
        name: &'static str,
        value: PathBuf,
        cwd: PathBuf,
        worktree: PathBuf,
    },
    /// The object-store variable `name` (`GIT_OBJECT_DIRECTORY`,
    /// `GIT_ALTERNATE_OBJECT_DIRECTORIES` or `GIT_COMMON_DIR`) is set to
    /// `value`. gix ignores it and git honours it, so wsx's reads and its git
    /// children would use different object stores.
    ObjectStoreEnv { name: &'static str, value: PathBuf },
    /// gix applies the ref namespace `namespace` (`refs/namespaces/<ns>/`),
    /// set by `GIT_NAMESPACE` or `gitoxide.core.refsNamespace`, which git's
    /// own reads ignore, so wsx would read every ref, and every replace ref,
    /// from inside the namespace while git reads them outside it.
    RefsNamespace { namespace: Vec<u8> },
    /// `GIT_DIR` is `git_dir` without `GIT_WORK_TREE`, and `reason` names
    /// what keeps wsx from deciding the worktree as git does (a setting in
    /// `config.worktree`, or a `core.bare` or `GIT_IMPLICIT_WORK_TREE` that
    /// is not a boolean), so wsx and git could see different worktrees.
    ImplicitWorkTree { git_dir: PathBuf, reason: String },
    /// The linked worktree `worktree`, registered in `git_dir`, no longer
    /// exists: `git worktree list` shows it as prunable.
    StaleWorktree { worktree: PathBuf, git_dir: PathBuf },
    /// An operating-system call failed; `what` names the call and its subject.
    Io {
        what: String,
        source: std::io::Error,
    },
    /// A gix operation failed; `what` names the operation, `message` is gix's.
    Gix { what: String, message: String },
    /// The root tree `tree` of `commit` lists its entries out of git's tree
    /// order, so git's lookup by name can miss an entry a scan finds; git
    /// fsck reports such a tree as `treeNotSorted`.
    TreeNotSorted { commit: ObjectId, tree: ObjectId },
    /// The replace ref `name` is symbolic, naming the ref `target`, or holds
    /// `target`, the hex of an object another replace ref replaces. git
    /// follows both shapes and gix does not, so a read could return other
    /// bytes than git's.
    ReplacementRef { name: Vec<u8>, target: Vec<u8> },
    /// The replace refs `first` and `second` name one object in different
    /// hex case: git stops with `duplicate replace ref` while both exist, and
    /// gix applies one of them.
    ReplacementRefDuplicate { first: Vec<u8>, second: Vec<u8> },
    /// `gitoxide.objects.replaceRefBase` is `value`, so gix takes replace
    /// refs from under `value` while git, which does not read the key, takes
    /// them from under `git_base`.
    ReplaceBaseDiffers { value: Vec<u8>, git_base: Vec<u8> },
}

impl Error {
    /// Whether this is one of the refusals of a replacement git and gix
    /// follow differently: `ReplacementRef`, `ReplacementRefDuplicate` or
    /// `ReplaceBaseDiffers`. Every caller that treats those apart from other
    /// failures reads this one list.
    pub fn is_replacement_refusal(&self) -> bool {
        false
    }
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
            Error::GitEmptyOutput { argv, expected } => write!(
                f,
                "git {} exited 0 but printed no {expected}",
                render_argv(argv)
            ),
            Error::RelativeRepoEnv {
                name,
                value,
                cwd,
                worktree,
            } => write!(
                f,
                "refusing {name}={value}: a relative path is ambiguous in {cwd}, below the worktree root {worktree}; \
                 make it absolute or run wsx from {worktree}",
                value = value.display(),
                cwd = cwd.display(),
                worktree = worktree.display()
            ),
            Error::ObjectStoreEnv { name, value } => write!(
                f,
                "refusing {name}={value}: this version of wsx reads objects through gix, which ignores {name}, \
                 so it would not read the objects git reads; run wsx without {name}",
                value = value.display()
            ),
            // The command line holds no byte field, so it is safe to paste.
            Error::RefsNamespace { namespace } => write!(
                f,
                "refusing the ref namespace {namespace}: wsx reads refs through gix, which reads every ref \
                 inside the namespace that GIT_NAMESPACE or gitoxide.core.refsNamespace sets, replace refs \
                 included, while git's own reads of this repository, such as git show-ref, read them \
                 outside it, so wsx would not read the refs git reads.\n\
                 Run wsx without GIT_NAMESPACE, and remove gitoxide.core.refsNamespace from the config \
                 file that sets it; this shows where it is set:\n\
                 \tgit config --show-origin --get-all gitoxide.core.refsNamespace",
                namespace = String::from_utf8_lossy(&c_quote(namespace))
            ),
            Error::ImplicitWorkTree { git_dir, reason } => write!(
                f,
                "refusing GIT_DIR={} without GIT_WORK_TREE: {reason}; \
                 set GIT_WORK_TREE to the worktree git should use",
                git_dir.display()
            ),
            Error::StaleWorktree { worktree, git_dir } => write!(
                f,
                "worktree {} is missing, but {} still registers it (git lists it as prunable)\n\
                 If it is gone for good, run:\n\
                 \tgit worktree prune",
                worktree.display(),
                git_dir.display()
            ),
            Error::Io { what, source } => write!(f, "{what}: {source}"),
            Error::Gix { what, message } => write!(f, "{what}: {message}"),
            // Both ids are hex, so the command line is safe to paste.
            Error::TreeNotSorted { commit, tree } => write!(
                f,
                "the root tree {tree} of commit {commit} lists its entries out of git's order, \
                 so git cannot find some of them by name and wsx could read an entry git does not; \
                 git fsck reports it as treeNotSorted.\n\
                 Write the tree again with git, which sorts it, and commit that tree:\n\
                 \tgit ls-tree -z {commit} | git mktree -z"
            ),
            Error::ReplacementRef { name, target } => write!(
                f,
                "refusing {}: it points at {}, and git and wsx's reader follow a replacement \
                 differently when it is symbolic or is itself replaced, so wsx could read other \
                 bytes than git.\n\
                 To remove it, run:\n\
                 \tgit update-ref --no-deref -d {}",
                shown(name),
                shown(target),
                ref_word(name)
            ),
            Error::ReplacementRefDuplicate { first, second } => write!(
                f,
                "refusing {} and {}: they name one object in different hex case, so git stops \
                 with \"duplicate replace ref\" while both exist, and wsx's reader would take one \
                 of them.\n\
                 To remove one, run one of:\n\
                 \tgit update-ref --no-deref -d {}\n\
                 \tgit update-ref --no-deref -d {}",
                shown(first),
                shown(second),
                ref_word(first),
                ref_word(second)
            ),
            // The command line holds no byte field, so it is safe to paste.
            Error::ReplaceBaseDiffers { value, git_base } => write!(
                f,
                "refusing gitoxide.objects.replaceRefBase = {value}: wsx's reader takes replace \
                 refs from under {value}, while git takes them from under {git_base}, so wsx \
                 could read other bytes than git.\n\
                 Unset the key in the config file that sets it, for example:\n\
                 \tgit config --local --unset gitoxide.objects.replaceRefBase",
                value = shown(value),
                git_base = shown(git_base)
            ),
        }
    }
}

/// `bytes` for prose in a message: git's C-style quoting, then lossy UTF-8,
/// so a control byte in a ref name never reaches the terminal raw.
fn shown(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&c_quote(bytes)).into_owned()
}

/// The ref `name` as an argument in a command line a message prints for the
/// user to paste: `shell_word`'s form, or `<ref>` when no shell quoting can
/// carry it. `c_quote` would leave a command substitution in the name live.
fn ref_word(name: &[u8]) -> String {
    shell_word(name).unwrap_or_else(|| "<ref>".to_owned())
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

    use super::{Error, ObjectId};

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

        let relative = Error::RelativeRepoEnv {
            name: "GIT_INDEX_FILE",
            value: PathBuf::from("i"),
            cwd: PathBuf::from("/w/s"),
            worktree: PathBuf::from("/w"),
        };
        assert_eq!(
            relative.to_string(),
            "refusing GIT_INDEX_FILE=i: a relative path is ambiguous in /w/s, below the worktree root /w; \
             make it absolute or run wsx from /w"
        );

        let object_store = Error::ObjectStoreEnv {
            name: "GIT_OBJECT_DIRECTORY",
            value: PathBuf::from("/q"),
        };
        assert_eq!(
            object_store.to_string(),
            "refusing GIT_OBJECT_DIRECTORY=/q: this version of wsx reads objects through gix, \
             which ignores GIT_OBJECT_DIRECTORY, so it would not read the objects git reads; \
             run wsx without GIT_OBJECT_DIRECTORY"
        );

        let implicit = Error::ImplicitWorkTree {
            git_dir: PathBuf::from("/w/.git"),
            reason: "r".to_owned(),
        };
        assert_eq!(
            implicit.to_string(),
            "refusing GIT_DIR=/w/.git without GIT_WORK_TREE: r; \
             set GIT_WORK_TREE to the worktree git should use"
        );

        let stale = Error::StaleWorktree {
            worktree: PathBuf::from("/w"),
            git_dir: PathBuf::from("/m/.git/worktrees/w"),
        };
        assert_eq!(
            stale.to_string(),
            "worktree /w is missing, but /m/.git/worktrees/w still registers it (git lists it as prunable)\n\
             If it is gone for good, run:\n\
             \tgit worktree prune"
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

    #[test]
    fn refs_namespace_message_names_it_and_the_fix() {
        let namespace = Error::RefsNamespace {
            namespace: b"refs/namespaces/foo/".to_vec(),
        };
        assert_eq!(
            namespace.to_string(),
            "refusing the ref namespace refs/namespaces/foo/: wsx reads refs through gix, which reads \
             every ref inside the namespace that GIT_NAMESPACE or gitoxide.core.refsNamespace sets, \
             replace refs included, while git's own reads of this repository, such as git show-ref, \
             read them outside it, so wsx would not read the refs git reads.\n\
             Run wsx without GIT_NAMESPACE, and remove gitoxide.core.refsNamespace from the config \
             file that sets it; this shows where it is set:\n\
             \tgit config --show-origin --get-all gitoxide.core.refsNamespace"
        );
    }

    #[test]
    fn tree_not_sorted_message_names_the_tree_and_the_fix() {
        let unsorted = Error::TreeNotSorted {
            commit: ObjectId("c0ffee".to_owned()),
            tree: ObjectId("7ee".to_owned()),
        };
        assert_eq!(
            unsorted.to_string(),
            "the root tree 7ee of commit c0ffee lists its entries out of git's order, \
             so git cannot find some of them by name and wsx could read an entry git does not; \
             git fsck reports it as treeNotSorted.\n\
             Write the tree again with git, which sorts it, and commit that tree:\n\
             \tgit ls-tree -z c0ffee | git mktree -z"
        );
    }

    /// A replaced object's name and its replacement, as 40-hex ids.
    const A_HEX: &str = "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1";
    const B_HEX: &str = "b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2";
    const A_UPPER_HEX: &str = "A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1A1";

    fn replace_ref(hex: &str) -> Vec<u8> {
        format!("refs/replace/{hex}").into_bytes()
    }

    #[test]
    fn replacement_ref_message_names_the_ref_and_the_fix() {
        let chained = Error::ReplacementRef {
            name: replace_ref(A_HEX),
            target: B_HEX.as_bytes().to_vec(),
        };
        assert_eq!(
            chained.to_string(),
            format!(
                "refusing refs/replace/{A_HEX}: it points at {B_HEX}, and git and wsx's reader \
                 follow a replacement differently when it is symbolic or is itself replaced, so \
                 wsx could read other bytes than git.\n\
                 To remove it, run:\n\
                 \tgit update-ref --no-deref -d refs/replace/{A_HEX}"
            )
        );
    }

    #[test]
    fn duplicate_replacement_refs_message_names_both_and_the_fix() {
        let duplicate = Error::ReplacementRefDuplicate {
            first: replace_ref(A_UPPER_HEX),
            second: replace_ref(A_HEX),
        };
        assert_eq!(
            duplicate.to_string(),
            format!(
                "refusing refs/replace/{A_UPPER_HEX} and refs/replace/{A_HEX}: they name one \
                 object in different hex case, so git stops with \"duplicate replace ref\" while \
                 both exist, and wsx's reader would take one of them.\n\
                 To remove one, run one of:\n\
                 \tgit update-ref --no-deref -d refs/replace/{A_UPPER_HEX}\n\
                 \tgit update-ref --no-deref -d refs/replace/{A_HEX}"
            )
        );
    }

    #[test]
    fn replace_base_differs_message_names_the_key_and_the_fix() {
        let differs = Error::ReplaceBaseDiffers {
            value: b"refs/other/".to_vec(),
            git_base: b"refs/replace/".to_vec(),
        };
        assert_eq!(
            differs.to_string(),
            "refusing gitoxide.objects.replaceRefBase = refs/other/: wsx's reader takes replace \
             refs from under refs/other/, while git takes them from under refs/replace/, so wsx \
             could read other bytes than git.\n\
             Unset the key in the config file that sets it, for example:\n\
             \tgit config --local --unset gitoxide.objects.replaceRefBase"
        );
    }

    /// The lines of `rendered` that are command lines: a tab, then `git`.
    fn command_lines(rendered: &str) -> Vec<&str> {
        rendered
            .lines()
            .filter(|line| line.starts_with("\tgit "))
            .collect()
    }

    #[test]
    fn replacement_fix_commands_quote_a_ref_name_for_the_shell() {
        let shell_syntax = b"refs/replace/$(touch${IFS}m)".to_vec();
        let symbolic = Error::ReplacementRef {
            name: shell_syntax.clone(),
            target: b"refs/tags/x".to_vec(),
        };
        assert_eq!(
            command_lines(&symbolic.to_string()),
            vec!["\tgit update-ref --no-deref -d 'refs/replace/$(touch${IFS}m)'"],
            "ReplacementRef's command line"
        );

        let duplicate = Error::ReplacementRefDuplicate {
            first: shell_syntax,
            second: replace_ref(A_HEX),
        };
        assert_eq!(
            command_lines(&duplicate.to_string()),
            vec![
                "\tgit update-ref --no-deref -d 'refs/replace/$(touch${IFS}m)'".to_owned(),
                format!("\tgit update-ref --no-deref -d refs/replace/{A_HEX}"),
            ],
            "ReplacementRefDuplicate's command lines"
        );

        let latin1 = Error::ReplacementRef {
            name: b"refs/replace/\xe9".to_vec(),
            target: B_HEX.as_bytes().to_vec(),
        };
        let rendered = latin1.to_string();
        assert_eq!(
            command_lines(&rendered),
            vec!["\tgit update-ref --no-deref -d <ref>"],
            "ReplacementRef's command line for a name holding byte 0xe9"
        );
        assert!(
            !rendered.as_bytes().contains(&0xe9),
            "{rendered:?} holds the raw byte 0xe9"
        );
        assert!(
            !rendered.contains('\u{fffd}'),
            "{rendered:?} replaced a byte instead of quoting it"
        );
    }

    #[test]
    fn replacement_refusals_are_recognised() {
        let refusals = [
            Error::ReplacementRef {
                name: replace_ref(A_HEX),
                target: B_HEX.as_bytes().to_vec(),
            },
            Error::ReplacementRefDuplicate {
                first: replace_ref(A_UPPER_HEX),
                second: replace_ref(A_HEX),
            },
            Error::ReplaceBaseDiffers {
                value: b"refs/other/".to_vec(),
                git_base: b"refs/replace/".to_vec(),
            },
        ];
        for refusal in &refusals {
            assert!(refusal.is_replacement_refusal(), "{refusal:?}");
        }
        let gix = Error::Gix {
            what: "read x".to_owned(),
            message: "m".to_owned(),
        };
        assert!(!gix.is_replacement_refusal(), "{gix:?}");
    }

    #[test]
    fn empty_output_message_names_the_query() {
        let empty = Error::GitEmptyOutput {
            argv: argv(&["rev-parse", "--local-env-vars"]),
            expected: "variable names",
        };
        assert_eq!(
            empty.to_string(),
            "git rev-parse --local-env-vars exited 0 but printed no variable names"
        );
    }
}
