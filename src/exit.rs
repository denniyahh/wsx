//! Exit classes: what a wsx run's exit code means (D-06), one class per
//! kind of failure, so an orchestrator can branch on the code and read the
//! error's kind for the detail. `docs/exit-codes.md` is the table.

use crate::model;

/// What a run's exit code says, in the order of the codes 0 to 6.
#[expect(dead_code, reason = "classify gives each class in the next commit")]
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
    #[expect(dead_code, reason = "read by the exit tests and by --json, 05-02")]
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
