//! The two forms of a check's report: the human one, diagnostic lines for
//! stderr and the verdict line, and the payload of the JSON object (D-11).
//!
//! Every byte value in prose goes through `git::c_quote`, so a control byte
//! in a path or a branch name never reaches the terminal raw; a path
//! suggested as a manifest line goes through `manifest::encode_path`, so
//! pasting it declares exactly that path. In the JSON payload every byte
//! value goes through `json::bytes` instead (D-09).

use super::{Problem, ProblemKind, Report};
use crate::git::c_quote;
use crate::json::{self, Value};
use crate::model::MANIFEST_NAME;
use crate::model::manifest::encode_path;

/// A check's human output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Human {
    /// For stderr: the not-configured notice when the branch was inferred
    /// (D-04), then the header and one line per problem.
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
    let mut diagnostics = Vec::new();
    if !report.configured {
        diagnostics.push(format!(
            "wsx check: wsx.sourceBranch is not set; checked the one workspace branch found, \
             {branch} (run wsx init to configure this repository)"
        ));
    }
    let count = report.problems.len();
    if count == 0 {
        return Human {
            diagnostics,
            verdict: format!(
                "wsx check: every difference between {branch} and {integration} is declared"
            ),
        };
    }
    diagnostics.push(format!(
        "wsx check: {branch} differs from {integration} in ways {} does not declare:",
        quoted(MANIFEST_NAME)
    ));
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

/// The check's payload for the JSON object, in the contract's key order:
/// `configured`, `workspace`, `integration`, then `problems` in D-05's
/// order, present and empty when there are none (D-11).
pub fn json(report: &Report) -> Vec<(&'static str, Value)> {
    let integration = &report.integration;
    vec![
        ("configured", Value::Bool(report.configured)),
        (
            "workspace",
            Value::Object(vec![
                ("branch", json::bytes(&report.workspace_branch)),
                ("tip", Value::Str(report.workspace_tip.to_string())),
            ]),
        ),
        (
            "integration",
            Value::Object(vec![
                ("branch", json::bytes(&integration.branch)),
                ("ref", json::bytes(&integration.full_ref)),
                ("tip", Value::Str(integration.tip.to_string())),
            ]),
        ),
        (
            "problems",
            Value::Array(report.problems.iter().map(problem_json).collect()),
        ),
    ]
}

/// One problem in the JSON payload: its kind's name, its path and its
/// detail, `null` for a kind that carries none.
fn problem_json(problem: &Problem) -> Value {
    Value::Object(vec![
        ("kind", Value::Str(problem.kind.name().to_owned())),
        ("path", json::bytes(&problem.path)),
        (
            "detail",
            problem.detail.clone().map_or(Value::Null, Value::Str),
        ),
    ])
}
