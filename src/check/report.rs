//! The two forms of a check's report: the human one, diagnostic lines for
//! stderr and the verdict line, and the payload of the JSON object (D-11).
//!
//! Every byte value in prose goes through `git::c_quote`, so a control byte
//! in a path or a branch name never reaches the terminal raw; a path
//! suggested as a manifest line goes through `manifest::encode_path`, so
//! pasting it declares exactly that path. In the JSON payload every byte
//! value goes through `json::bytes` instead (D-09).

use super::{HookStatus, Problem, ProblemKind, Report};
use crate::git::c_quote;
use crate::json::{self, Value};
use crate::model::MANIFEST_NAME;
use crate::model::manifest::encode_path;

/// A check's human output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Human {
    /// For stderr: the not-configured notice when the branch was inferred
    /// (D-04), the notice of a `core.hooksPath` not checked (D-03), then the
    /// header and one line per problem.
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

/// The line that reports `problem`, without its indent; `integration` is
/// the integration ref, quoted.
fn problem_line(problem: &Problem, integration: &str) -> String {
    let path = quoted(&problem.path);
    let entry = encoded(&problem.path);
    match problem.kind {
        ProblemKind::UndeclaredAdded => {
            format!("undeclared workspace-only path: {path} (declare: only {entry})")
        }
        ProblemKind::UndeclaredModified => format!(
            "undeclared change to a shared file: {path} (move it to the integration branch, \
             or declare: modified {entry})"
        ),
        ProblemKind::UndeclaredDeleted => format!(
            "undeclared deletion of a shared file: {path} (restore it, or declare: modified \
             {entry})"
        ),
        ProblemKind::StaleOnly => {
            format!("stale declaration: only {entry}, but {integration} carries it")
        }
        ProblemKind::StaleModified => {
            format!("stale declaration: modified {entry}, but it matches {integration}")
        }
        ProblemKind::RedundantBuiltin => format!(
            "redundant declaration: {}",
            problem.detail.as_deref().unwrap_or_default()
        ),
        ProblemKind::HookNotExecutable => format!(
            "hook not committed executable: {path} ({})",
            problem.detail.as_deref().unwrap_or_default()
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
    if let HookStatus::NotChecked { value, reason } = &report.hooks {
        diagnostics.push(format!(
            "wsx check: core.hooksPath {} is not checked: {reason}",
            quoted(value)
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
            .map(|problem| format!("  {}", problem_line(problem, &integration))),
    );
    let noun = if count == 1 { "problem" } else { "problems" };
    Human {
        diagnostics,
        verdict: format!("wsx check: {count} {noun} between {branch} and {integration}"),
    }
}

/// The check's payload for the JSON object, in the contract's key order:
/// `configured`, `workspace`, `integration`, `hooks`, then `problems` in
/// D-05's order, present and empty when there are none (D-11).
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
        ("hooks", hooks_json(&report.hooks)),
        (
            "problems",
            Value::Array(report.problems.iter().map(problem_json).collect()),
        ),
    ]
}

/// The hooks object of the JSON payload: the value or directory, whether it
/// was checked and, when not, why.
fn hooks_json(hooks: &HookStatus) -> Value {
    let (path, checked, reason) = match hooks {
        HookStatus::NotSet => (Value::Null, false, Value::Str(NOT_SET_REASON.to_owned())),
        HookStatus::NotChecked { value, reason } => {
            (json::bytes(value), false, Value::Str(reason.clone()))
        }
        HookStatus::Checked { dir } => (json::bytes(dir), true, Value::Null),
    };
    Value::Object(vec![
        ("path", path),
        ("checked", Value::Bool(checked)),
        ("reason", reason),
    ])
}

/// Why the hooks object of an unset `core.hooksPath` says it was not checked.
const NOT_SET_REASON: &str = "core.hooksPath is not set";

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::staleness::{Source, Staleness, TimeRead, time_of};
    use crate::git::ObjectId;
    use crate::model::Integration;

    /// The integration ref of the fixtures.
    const ORIGIN_DEVELOP: &[u8] = b"refs/remotes/origin/develop";

    /// The clock reading of the staleness tests.
    const NOW: i64 = 1_800_000_000;

    /// A configured report against `full_ref` with no `core.hooksPath`.
    fn report_of(full_ref: &[u8], staleness: Staleness, problems: Vec<Problem>) -> Report {
        Report {
            configured: true,
            workspace_branch: b"workspace/dev".to_vec(),
            workspace_tip: ObjectId::for_test("1111111111111111111111111111111111111111"),
            integration: Integration {
                branch: b"develop".to_vec(),
                full_ref: full_ref.to_vec(),
                tip: ObjectId::for_test("2222222222222222222222222222222222222222"),
            },
            staleness,
            hooks: HookStatus::NotSet,
            problems,
        }
    }

    /// Staleness that applies, with `ref_moved_at` and `fetched_at`.
    fn applying(ref_moved_at: TimeRead, fetched_at: TimeRead) -> Staleness {
        Staleness {
            applies: true,
            ref_moved_at,
            fetched_at,
            now: NOW,
        }
    }

    /// The one diagnostic `human` prints for a report with no problem
    /// against `full_ref`: the staleness line.
    fn staleness_line(full_ref: &[u8], staleness: Staleness) -> String {
        let mut diagnostics = human(&report_of(full_ref, staleness, Vec::new())).diagnostics;
        assert_eq!(
            diagnostics.len(),
            1,
            "the staleness line alone: {diagnostics:?}"
        );
        diagnostics.remove(0)
    }

    /// The staleness object of the JSON payload of a report with no problem.
    fn staleness_json(staleness: Staleness) -> Value {
        json(&report_of(ORIGIN_DEVELOP, staleness, Vec::new()))
            .into_iter()
            .find(|(key, _)| *key == "staleness")
            .map(|(_, value)| value)
            .expect("a staleness key")
    }

    /// The line `human` prints for a report holding `problem` alone,
    /// against `refs/remotes/origin/develop`: the last diagnostic, after the
    /// header.
    fn line_of(kind: ProblemKind, path: &[u8], detail: Option<&str>) -> String {
        let report = report_of(
            ORIGIN_DEVELOP,
            applying(TimeRead::At(NOW), TimeRead::At(NOW)),
            vec![Problem {
                kind,
                path: path.to_vec(),
                detail: detail.map(str::to_owned),
            }],
        );
        let mut diagnostics = human(&report).diagnostics;
        let line = diagnostics.pop().expect("a problem line");
        assert!(
            diagnostics
                .last()
                .is_some_and(|header| header.starts_with("wsx check: workspace/dev differs from")),
            "the header before the one problem line: {diagnostics:?}"
        );
        line
    }

    /// Both times read: each with its age and its source.
    #[test]
    fn staleness_line_names_both_times() {
        assert_eq!(
            staleness_line(
                ORIGIN_DEVELOP,
                applying(TimeRead::At(NOW - 90), TimeRead::At(NOW - 7200))
            ),
            "wsx check: integration ref refs/remotes/origin/develop: last moved 1m 30s ago \
             (reflog); last fetch 2h 0m ago (FETCH_HEAD)"
        );
    }

    /// Neither time there: each reason in its place.
    #[test]
    fn staleness_line_says_what_is_missing() {
        let staleness = applying(
            time_of(Source::Reflog(ORIGIN_DEVELOP), Ok::<_, &str>(None)),
            time_of(Source::FetchHead, Ok::<_, &str>(None)),
        );
        assert_eq!(
            staleness_line(ORIGIN_DEVELOP, staleness),
            "wsx check: integration ref refs/remotes/origin/develop: no reflog for \
             refs/remotes/origin/develop; no FETCH_HEAD"
        );
    }

    /// A local integration branch: staleness does not apply.
    #[test]
    fn local_branch_line_says_staleness_does_not_apply() {
        let local = b"refs/heads/develop";
        assert_eq!(
            staleness_line(local, Staleness::unread(local, NOW)),
            "wsx check: integration ref refs/heads/develop is a local branch; staleness does \
             not apply"
        );
    }

    /// A time 10 s after now: age -10 in JSON, as it is, and 0s in the line.
    #[test]
    fn staleness_json_reports_a_future_time_as_a_negative_age() {
        let future = || applying(TimeRead::At(NOW + 10), TimeRead::At(NOW + 10));
        assert_eq!(
            staleness_json(future()),
            Value::Object(vec![
                ("applies", Value::Bool(true)),
                ("reason", Value::Null),
                ("ref_moved_at", Value::Int(NOW + 10)),
                ("ref_moved_age_seconds", Value::Int(-10)),
                ("ref_moved_reason", Value::Null),
                ("fetched_at", Value::Int(NOW + 10)),
                ("fetched_age_seconds", Value::Int(-10)),
                ("fetched_reason", Value::Null),
            ])
        );
        assert_eq!(
            staleness_line(ORIGIN_DEVELOP, future()),
            "wsx check: integration ref refs/remotes/origin/develop: last moved 0s ago \
             (reflog); last fetch 0s ago (FETCH_HEAD)"
        );
    }

    /// No problem and both times a year old: the verdict is still that every
    /// difference is declared.
    #[test]
    fn staleness_never_changes_the_verdict() {
        let year = 365 * 86_400;
        let old = applying(TimeRead::At(NOW - year), TimeRead::At(NOW - year));
        assert_eq!(
            human(&report_of(ORIGIN_DEVELOP, old, Vec::new())).verdict,
            "wsx check: every difference between workspace/dev and refs/remotes/origin/develop \
             is declared"
        );
    }

    /// Each kind's line is the check contract's text (05-01).
    #[test]
    fn every_kind_renders_its_contract_line() {
        assert_eq!(
            line_of(ProblemKind::UndeclaredAdded, b"notes/a", None),
            "  undeclared workspace-only path: notes/a (declare: only notes/a)"
        );
        assert_eq!(
            line_of(ProblemKind::UndeclaredModified, b"README.md", None),
            "  undeclared change to a shared file: README.md (move it to the integration \
             branch, or declare: modified README.md)"
        );
        assert_eq!(
            line_of(ProblemKind::UndeclaredDeleted, b"src/a.txt", None),
            "  undeclared deletion of a shared file: src/a.txt (restore it, or declare: \
             modified src/a.txt)"
        );
        assert_eq!(
            line_of(ProblemKind::StaleOnly, b"shared", None),
            "  stale declaration: only shared, but refs/remotes/origin/develop carries it"
        );
        assert_eq!(
            line_of(ProblemKind::StaleModified, b"README.md", None),
            "  stale declaration: modified README.md, but it matches \
             refs/remotes/origin/develop"
        );
        assert_eq!(
            line_of(
                ProblemKind::RedundantBuiltin,
                b".worktrees/x",
                Some("only .worktrees/x is already personal as the built-in .worktrees")
            ),
            "  redundant declaration: only .worktrees/x is already personal as the built-in \
             .worktrees"
        );
    }
}
