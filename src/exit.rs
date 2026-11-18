//! Exit classes: what a wsx run's exit code means (D-06), one class per
//! kind of failure, so an orchestrator can branch on the code and read the
//! error's kind for the detail. `docs/exit-codes.md` is the table.

use crate::model;

/// What a run's exit code says, in the order of the codes 0 to 6.
// `cfg_attr(not(test), ...)`: the tests construct every class, so on the test
// target the expectation would be unfulfilled.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "classify gives each class in the next commit")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitClass {
    /// The command ran and found nothing to report.
    Success,
    /// The command ran and found problems, which it printed.
    Problems,
    /// The arguments are not ones wsx accepts.
    Usage,
    /// The repository is not configured for wsx, and none could be inferred.
    NotConfigured,
    /// The repository's wsx config or manifest is invalid.
    InvalidConfig,
    /// wsx refuses the repository or its environment on purpose.
    Refused,
    /// git, gix or the operating system failed.
    GitFailed,
}

impl ExitClass {
    /// The process exit code of this class.
    pub fn code(self) -> u8 {
        match self {
            ExitClass::Success => 0,
            ExitClass::Problems => 1,
            ExitClass::Usage => 2,
            ExitClass::NotConfigured => 3,
            ExitClass::InvalidConfig => 4,
            ExitClass::Refused => 5,
            ExitClass::GitFailed => 6,
        }
    }

    /// The class's name as the documentation and the JSON output write it.
    // `cfg_attr(not(test), ...)`: the binary reads the name first in the JSON
    // output; the tests read it, which would leave the expectation unfulfilled.
    #[cfg_attr(not(test), expect(dead_code, reason = "read by --json, 05-02"))]
    pub fn name(self) -> &'static str {
        match self {
            ExitClass::Success => "success",
            ExitClass::Problems => "problems",
            ExitClass::Usage => "usage",
            ExitClass::NotConfigured => "not_configured",
            ExitClass::InvalidConfig => "invalid_config",
            ExitClass::Refused => "refused",
            ExitClass::GitFailed => "git_failed",
        }
    }
}

/// An error's class and its kind, the error's variant name in snake_case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Classified {
    /// The exit class.
    pub class: ExitClass,
    /// The error's kind (D-08).
    pub kind: &'static str,
}

/// The class and kind of `err`.
pub fn classify(_err: &model::Error) -> Classified {
    Classified {
        class: ExitClass::GitFailed,
        kind: "git_failed",
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::time::Duration;

    use super::*;
    use crate::git;
    use crate::model::manifest::{ParseError, Problem};

    /// The exit classes in code order.
    const CLASSES: [ExitClass; 7] = [
        ExitClass::Success,
        ExitClass::Problems,
        ExitClass::Usage,
        ExitClass::NotConfigured,
        ExitClass::InvalidConfig,
        ExitClass::Refused,
        ExitClass::GitFailed,
    ];

    /// An argv for a git failure.
    fn argv() -> Vec<OsString> {
        vec![OsString::from("status")]
    }

    /// An operating-system error.
    fn io_error() -> std::io::Error {
        std::io::Error::other("denied")
    }

    /// The three refusals of the replace refs, each with its kind.
    fn replacement_refusals() -> Vec<(git::Error, &'static str)> {
        vec![
            (
                git::Error::ReplaceRefsPresent {
                    path: PathBuf::from("refs/replace"),
                    packed: false,
                },
                "replace_refs_present",
            ),
            (
                git::Error::ReplaceBaseSet {
                    name: "GIT_REPLACE_REF_BASE",
                    value: b"refs/other/".to_vec(),
                },
                "replace_base_set",
            ),
            (
                git::Error::ReplaceRefsUnreadable {
                    path: PathBuf::from("refs/replace"),
                    source: io_error(),
                },
                "replace_refs_unreadable",
            ),
        ]
    }

    /// One value of every `git::Error` variant with its class and kind.
    fn git_cases() -> Vec<(git::Error, ExitClass, &'static str)> {
        let path = || PathBuf::from("/r");
        let mut cases = vec![
            (
                git::Error::NotARepo {
                    start: path(),
                    reason: "none".to_owned(),
                },
                ExitClass::NotConfigured,
                "not_a_repo",
            ),
            (
                git::Error::ForeignOwner { path: path() },
                ExitClass::Refused,
                "foreign_owner",
            ),
            (
                git::Error::Reftable {
                    path: path(),
                    format: "reftable".to_owned(),
                },
                ExitClass::Refused,
                "reftable",
            ),
            (
                git::Error::GitTooOld {
                    found: "2.0.0".to_owned(),
                    required: (2, 45),
                },
                ExitClass::GitFailed,
                "git_too_old",
            ),
            (
                git::Error::GitFailed {
                    argv: argv(),
                    status: Some(128),
                    stderr: b"fatal\n".to_vec(),
                },
                ExitClass::GitFailed,
                "git_failed",
            ),
            (
                git::Error::GitTimedOut {
                    argv: argv(),
                    after: Duration::from_secs(1),
                },
                ExitClass::GitFailed,
                "git_timed_out",
            ),
            (
                git::Error::OpenTimedOut {
                    start: path(),
                    after: Duration::from_secs(1),
                },
                ExitClass::GitFailed,
                "open_timed_out",
            ),
            (
                git::Error::GitEmptyOutput {
                    argv: argv(),
                    expected: "a line",
                },
                ExitClass::GitFailed,
                "git_empty_output",
            ),
            (
                git::Error::GitBadOutput {
                    argv: argv(),
                    problem: "garbled".to_owned(),
                },
                ExitClass::GitFailed,
                "git_bad_output",
            ),
            (
                git::Error::RelativeRepoEnv {
                    name: "GIT_DIR",
                    value: PathBuf::from(".git"),
                    cwd: path(),
                    worktree: path(),
                },
                ExitClass::Refused,
                "relative_repo_env",
            ),
            (
                git::Error::ObjectStoreEnv {
                    name: "GIT_OBJECT_DIRECTORY",
                    value: path(),
                },
                ExitClass::Refused,
                "object_store_env",
            ),
            (
                git::Error::RefsNamespace {
                    namespace: b"ns".to_vec(),
                },
                ExitClass::Refused,
                "refs_namespace",
            ),
            (
                git::Error::ImplicitWorkTree {
                    git_dir: path(),
                    reason: "config.worktree".to_owned(),
                },
                ExitClass::Refused,
                "implicit_work_tree",
            ),
            (
                git::Error::StaleWorktree {
                    worktree: path(),
                    git_dir: path(),
                },
                ExitClass::Refused,
                "stale_worktree",
            ),
            (
                git::Error::Io {
                    what: "read".to_owned(),
                    source: io_error(),
                },
                ExitClass::GitFailed,
                "io",
            ),
            (
                git::Error::Gix {
                    what: "open".to_owned(),
                    message: "failed".to_owned(),
                },
                ExitClass::GitFailed,
                "gix",
            ),
            (
                git::Error::ObjectMissing {
                    request: b"abc".to_vec(),
                },
                ExitClass::GitFailed,
                "object_missing",
            ),
        ];
        cases.extend(
            replacement_refusals()
                .into_iter()
                .map(|(err, kind)| (err, ExitClass::Refused, kind)),
        );
        cases
    }

    /// A `git::Error` of the git-failed class.
    fn git_failed() -> git::Error {
        git::Error::GitFailed {
            argv: argv(),
            status: Some(1),
            stderr: Vec::new(),
        }
    }

    /// One value of every `model::Error` variant with its class and kind.
    fn model_cases() -> Vec<(model::Error, ExitClass, &'static str)> {
        let branch = || b"workspace/dev".to_vec();
        vec![
            (
                model::Error::Git(git_failed()),
                ExitClass::GitFailed,
                "git_failed",
            ),
            (
                model::Error::KeyRepeated {
                    key: model::SOURCE_BRANCH,
                    values: vec![b"a".to_vec(), b"b".to_vec()],
                },
                ExitClass::InvalidConfig,
                "key_repeated",
            ),
            (
                model::Error::KeyEmpty {
                    key: model::SOURCE_BRANCH,
                },
                ExitClass::InvalidConfig,
                "key_empty",
            ),
            (
                model::Error::NotABranchName {
                    key: model::SOURCE_BRANCH,
                    value: b"a..b".to_vec(),
                },
                ExitClass::InvalidConfig,
                "not_a_branch_name",
            ),
            (
                model::Error::NoSuchRemote {
                    key: model::BASE_REMOTE,
                    remote: b"up".to_vec(),
                },
                ExitClass::InvalidConfig,
                "no_such_remote",
            ),
            (
                model::Error::BaseRefMissing {
                    value: b"develop".to_vec(),
                    tried: b"refs/remotes/origin/develop".to_vec(),
                },
                ExitClass::InvalidConfig,
                "base_ref_missing",
            ),
            (
                model::Error::NoIntegrationBranch {
                    remote: Some(b"origin".to_vec()),
                    tried: vec![b"refs/remotes/origin/develop".to_vec()],
                },
                ExitClass::InvalidConfig,
                "no_integration_branch",
            ),
            (
                model::Error::IntegrationRefSymbolic {
                    full_ref: b"refs/remotes/origin/develop".to_vec(),
                    target: b"refs/remotes/origin/main".to_vec(),
                },
                ExitClass::InvalidConfig,
                "integration_ref_symbolic",
            ),
            (
                model::Error::WorkspaceBranchMissing { branch: branch() },
                ExitClass::InvalidConfig,
                "workspace_branch_missing",
            ),
            (
                model::Error::WorkspaceCandidates {
                    candidates: Vec::new(),
                },
                ExitClass::NotConfigured,
                "workspace_candidates",
            ),
            (
                model::Error::ManifestMissing { branch: branch() },
                ExitClass::InvalidConfig,
                "manifest_missing",
            ),
            (
                model::Error::ManifestNotABlob {
                    branch: branch(),
                    kind: "tree",
                },
                ExitClass::InvalidConfig,
                "manifest_not_a_blob",
            ),
            (
                model::Error::ManifestUnreadable {
                    branch: branch(),
                    source: git::Error::ObjectMissing {
                        request: b"abc".to_vec(),
                    },
                },
                ExitClass::GitFailed,
                "manifest_unreadable",
            ),
            (
                model::Error::ManifestInvalid {
                    branch: branch(),
                    source: ParseError {
                        line: 1,
                        text: b"bogus".to_vec(),
                        problem: Problem::UnknownKind,
                    },
                },
                ExitClass::InvalidConfig,
                "manifest_invalid",
            ),
            (
                model::Error::WorktreeDirInvalid {
                    value: b"/abs".to_vec(),
                    problem: Problem::AbsolutePath,
                },
                ExitClass::InvalidConfig,
                "worktree_dir_invalid",
            ),
            (
                model::Error::NotConfigured {
                    reason: Box::new(model::Error::WorkspaceCandidates {
                        candidates: Vec::new(),
                    }),
                },
                ExitClass::NotConfigured,
                "not_configured",
            ),
        ]
    }

    /// The class and kind of `err` as a git-layer error inside the model.
    fn of_model_git(err: git::Error) -> (ExitClass, &'static str) {
        let classified = classify(&model::Error::Git(err));
        (classified.class, classified.kind)
    }

    /// The codes are D-06's table, 0 to 6 in declaration order, and each
    /// class has its documented name.
    #[test]
    fn class_codes_are_the_d06_table() {
        let codes: Vec<u8> = CLASSES.iter().map(|class| class.code()).collect();
        assert_eq!(codes, [0, 1, 2, 3, 4, 5, 6]);
        let names: Vec<&str> = CLASSES.iter().map(|class| class.name()).collect();
        assert_eq!(
            names,
            [
                "success",
                "problems",
                "usage",
                "not_configured",
                "invalid_config",
                "refused",
                "git_failed"
            ]
        );
    }

    /// Every `git::Error` variant, inside `model::Error::Git`, has the class
    /// and kind of the contract table.
    #[test]
    fn every_git_error_has_its_class() {
        let cases = git_cases();
        assert_eq!(cases.len(), 20, "one case per git::Error variant");
        for (err, class, kind) in cases {
            let shown = format!("{err:?}");
            assert_eq!(
                of_model_git(err),
                (class, kind),
                "class and kind of {shown}"
            );
        }
    }

    /// Every `model::Error` variant has the class and kind of the contract
    /// table; a missing manifest that leaves the repository unconfigured is
    /// `not_configured`, class 3.
    #[test]
    fn every_model_error_has_its_class() {
        let cases = model_cases();
        assert_eq!(cases.len(), 16, "one case per model::Error variant");
        for (err, class, kind) in cases {
            assert_eq!(
                classify(&err),
                Classified { class, kind },
                "class and kind of {err:?}"
            );
        }
        let unconfigured = model::Error::NotConfigured {
            reason: Box::new(model::Error::ManifestMissing {
                branch: b"workspace/dev".to_vec(),
            }),
        };
        assert_eq!(
            classify(&unconfigured),
            Classified {
                class: ExitClass::NotConfigured,
                kind: "not_configured"
            }
        );
    }

    /// A refusal of the replace refs is class 5 with its own kind however the
    /// model wraps it; a git failure in the same wrapping stays not
    /// configured (control).
    #[test]
    fn replacement_refusals_are_refused_wherever_wrapped() {
        let refused = |kind| Classified {
            class: ExitClass::Refused,
            kind,
        };
        let wrappings: [fn(git::Error) -> model::Error; 3] = [
            model::Error::Git,
            |err| model::Error::NotConfigured {
                reason: Box::new(model::Error::Git(err)),
            },
            |err| model::Error::NotConfigured {
                reason: Box::new(model::Error::ManifestUnreadable {
                    branch: b"workspace/dev".to_vec(),
                    source: err,
                }),
            },
        ];
        for (index, (_, kind)) in replacement_refusals().into_iter().enumerate() {
            let fresh = || replacement_refusals().swap_remove(index).0;
            assert_eq!(of_model_git(fresh()), (ExitClass::Refused, kind));
            for wrap in wrappings {
                let err = wrap(fresh());
                assert_eq!(classify(&err), refused(kind), "classify {err:?}");
            }
        }
        let control = model::Error::NotConfigured {
            reason: Box::new(model::Error::Git(git_failed())),
        };
        assert_eq!(
            classify(&control),
            Classified {
                class: ExitClass::NotConfigured,
                kind: "not_configured"
            }
        );
    }

    /// The documentation of the exit codes.
    const EXIT_CODES_DOC: &str = include_str!("../docs/exit-codes.md");

    /// What `doc` lacks: a `| <code> | <name> |` row per class, and a row
    /// opening with each kind in backticks, for every kind the variant tests
    /// produce and for `usage`.
    fn missing_from(doc: &str) -> Vec<String> {
        let mut missing: Vec<String> = CLASSES
            .iter()
            .map(|class| format!("| {} | {} |", class.code(), class.name()))
            .filter(|row| !doc.contains(row.as_str()))
            .collect();
        let mut kinds: Vec<&str> = git_cases().into_iter().map(|case| case.2).collect();
        kinds.extend(model_cases().into_iter().map(|case| case.2));
        kinds.push("usage");
        kinds.sort_unstable();
        kinds.dedup();
        missing.extend(
            kinds
                .into_iter()
                .map(|kind| format!("| `{kind}` |"))
                .filter(|row| !doc.contains(row.as_str())),
        );
        missing
    }

    /// `docs/exit-codes.md` lists every class with its code and every kind;
    /// the same check over the doc without the refused row names it
    /// (control).
    #[test]
    fn exit_codes_doc_lists_every_class_and_kind() {
        assert_eq!(missing_from(EXIT_CODES_DOC), Vec::<String>::new());
        let without_refused: String = EXIT_CODES_DOC
            .lines()
            .filter(|line| !line.starts_with("| 5 | refused |"))
            .map(|line| format!("{line}\n"))
            .collect();
        assert_eq!(missing_from(&without_refused), ["| 5 | refused |"]);
    }
}
