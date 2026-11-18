//! The human form of a check's report: the diagnostic lines for stderr and
//! the verdict line.
//!
//! Every byte value in prose goes through `git::c_quote`, so a control byte
//! in a path or a branch name never reaches the terminal raw; a path
//! suggested as a manifest line goes through `manifest::encode_path`, so
//! pasting it declares exactly that path.

use super::{Problem, ProblemKind, Report};
use crate::git::c_quote;
use crate::model::MANIFEST_NAME;
use crate::model::manifest::encode_path;

/// A check's human output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Human {
    /// The header and one line per problem, for stderr; empty without
    /// problems.
    pub diagnostics: Vec<String>,
    /// The one-line verdict.
    pub verdict: String,
}

/// `bytes` in git's C-style quoting, for prose.
fn quoted(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&c_quote(bytes)).into_owned()
}

/// `path` as a manifest line writes it.
fn encoded(path: &[u8]) -> String {
    String::from_utf8_lossy(&encode_path(path)).into_owned()
}

/// The line that reports `problem`, without its indent.
fn problem_line(problem: &Problem) -> String {
    match problem.kind {
        ProblemKind::UndeclaredAdded => format!(
            "undeclared workspace-only path: {} (declare: only {})",
            quoted(&problem.path),
            encoded(&problem.path)
        ),
    }
}

/// The human output of `report`.
pub fn human(report: &Report) -> Human {
    let branch = quoted(&report.workspace_branch);
    let integration = quoted(&report.integration.full_ref);
    let count = report.problems.len();
    if count == 0 {
        return Human {
            diagnostics: Vec::new(),
            verdict: format!(
                "wsx check: every difference between {branch} and {integration} is declared"
            ),
        };
    }
    let mut diagnostics = vec![format!(
        "wsx check: {branch} differs from {integration} in ways {} does not declare:",
        quoted(MANIFEST_NAME)
    )];
    diagnostics.extend(
        report
            .problems
            .iter()
            .map(|problem| format!("  {}", problem_line(problem))),
    );
    let noun = if count == 1 { "problem" } else { "problems" };
    Human {
        diagnostics,
        verdict: format!("wsx check: {count} {noun} between {branch} and {integration}"),
    }
}
