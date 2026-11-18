//! The divergence check (DIV-01): how the workspace branch differs from the
//! integration branch, judged against the manifest.
//!
//! The check reads the repository model first (`model::resolve`, which
//! refuses replace refs before any object is read), then the tree diff
//! between the two tips that model recorded. It never reads HEAD, the index
//! or the working tree (D-02), and it writes nothing. `evaluate` is pure:
//! every judgement is made over the model and the diff, so a ref that moves
//! during a run cannot mix two tips.

pub mod report;

use std::collections::BTreeSet;

use crate::git::{ChangeKind, Git, ObjectId, Repo, TreeChange};
use crate::model::manifest::{EntryKind, covers};
use crate::model::{self, Model, RepoModel};

/// What one check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Whether local config names the workspace branch (D-15); otherwise it
    /// was inferred.
    pub configured: bool,
    /// The workspace branch's short name.
    pub workspace_branch: Vec<u8>,
    /// The workspace tip the check compared.
    pub workspace_tip: ObjectId,
    /// The integration branch and the tip the check compared.
    pub integration: model::Integration,
    /// Every problem found, sorted by path bytes (D-05).
    pub problems: Vec<Problem>,
}

/// One difference the manifest does not declare, or one declaration that
/// no longer matches the branches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// What is wrong.
    pub kind: ProblemKind,
    /// The path it concerns, raw bytes from the tree root.
    pub path: Vec<u8>,
    /// More about the problem, for the kinds that carry it.
    pub detail: Option<String>,
}

/// The kinds of problem the check reports (D-11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProblemKind {
    /// A path only the workspace branch has, which no `only` entry covers.
    UndeclaredAdded,
}

impl ProblemKind {
    /// The kind's name as the JSON output writes it (D-11).
    #[cfg_attr(not(test), expect(dead_code, reason = "read by --json, 05-02"))]
    pub fn name(self) -> &'static str {
        match self {
            ProblemKind::UndeclaredAdded => "undeclared_added",
        }
    }
}

/// Runs the check in `repo`: resolves the model, reads the tree diff from
/// the integration tip to the workspace tip and evaluates it. The model is
/// resolved before any other read, so its replace-ref guard stands for the
/// reads by id after it (D-22).
pub fn run(repo: &Repo, git: &Git) -> Result<Report, model::Error> {
    // The recorded set stays empty until phase 8 records workspace branches.
    let resolved = model::resolve(repo, git, &BTreeSet::new())?;
    let configured = matches!(resolved, RepoModel::Configured(_));
    let model = resolved.require()?;
    // From the integration tip, so `Added` is a workspace-only path (D-01).
    let changes = repo.tree_diff(&model.integration.tip, &model.workspace_tip)?;
    let problems = evaluate(&model, &changes);
    Ok(Report {
        configured,
        workspace_branch: model.workspace_branch.clone(),
        workspace_tip: model.workspace_tip.clone(),
        integration: model.integration.clone(),
        problems,
    })
}

/// The problems in `changes`, the diff from the integration tip to the
/// workspace tip, judged against `model`'s built-ins and manifest; every
/// problem of the run, sorted by path bytes.
pub fn evaluate(model: &Model, changes: &[TreeChange]) -> Vec<Problem> {
    let builtins = model.builtins();
    let mut problems = Vec::new();
    for change in changes {
        if builtins.iter().any(|builtin| covers(builtin, &change.path)) {
            continue;
        }
        match change.kind {
            ChangeKind::Added => {
                if !declared_only(model, &change.path) {
                    problems.push(Problem {
                        kind: ProblemKind::UndeclaredAdded,
                        path: change.path.clone(),
                        detail: None,
                    });
                }
            }
            // 05-04 judges changes to shared files.
            ChangeKind::Modified | ChangeKind::Deleted | ChangeKind::TypeChanged => {}
        }
    }
    problems.sort_by(|a, b| a.path.cmp(&b.path));
    problems
}

/// Whether an `only` entry of `model`'s manifest covers `path`.
fn declared_only(model: &Model, path: &[u8]) -> bool {
    model
        .manifest
        .entries
        .iter()
        .any(|entry| entry.kind == EntryKind::Only && covers(&entry.path, path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Integration;
    use crate::model::manifest::{Entry, Manifest};

    /// A model with `worktree_dir` and one `only` entry per path in `only`.
    fn model_with(worktree_dir: &[u8], only: &[&[u8]]) -> Model {
        Model {
            workspace_branch: b"workspace/dev".to_vec(),
            workspace_tip: ObjectId::for_test("1111111111111111111111111111111111111111"),
            integration: Integration {
                branch: b"develop".to_vec(),
                full_ref: b"refs/remotes/origin/develop".to_vec(),
                tip: ObjectId::for_test("2222222222222222222222222222222222222222"),
            },
            base_remote: Some(b"origin".to_vec()),
            push_remote: Some(b"origin".to_vec()),
            worktree_dir: worktree_dir.to_vec(),
            manifest: Manifest {
                entries: only
                    .iter()
                    .enumerate()
                    .map(|(index, path)| Entry {
                        kind: EntryKind::Only,
                        path: path.to_vec(),
                        line: index + 1,
                        text: [&b"only "[..], path].concat(),
                    })
                    .collect(),
            },
        }
    }

    /// An `Added` change at `path`.
    fn added(path: &[u8]) -> TreeChange {
        TreeChange {
            kind: ChangeKind::Added,
            path: path.to_vec(),
        }
    }

    /// The kinds and paths of `problems`.
    fn found(problems: &[Problem]) -> Vec<(&'static str, Vec<u8>)> {
        problems
            .iter()
            .map(|problem| (problem.kind.name(), problem.path.clone()))
            .collect()
    }

    /// A path under the worktree directory is a built-in personal path, so
    /// adding it is no problem; under another worktree directory the same
    /// path is undeclared (control).
    #[test]
    fn added_path_under_a_builtin_is_not_a_problem() {
        let changes = [added(b".worktrees/x")];
        assert_eq!(
            found(&evaluate(&model_with(b".worktrees", &[]), &changes)),
            []
        );
        assert_eq!(
            found(&evaluate(&model_with(b".other", &[]), &changes)),
            [("undeclared_added", b".worktrees/x".to_vec())]
        );
    }

    /// An added path is declared by an `only` entry that covers it, and
    /// `covers` is literal: `notesX` does not cover `notes/a`.
    #[test]
    fn added_path_is_declared_only_by_a_covering_only_entry() {
        let changes = [added(b"notes/a")];
        assert_eq!(
            found(&evaluate(&model_with(b".worktrees", &[b"notes"]), &changes)),
            []
        );
        assert_eq!(
            found(&evaluate(
                &model_with(b".worktrees", &[b"notesX"]),
                &changes
            )),
            [("undeclared_added", b"notes/a".to_vec())]
        );
    }

    /// Problems come out in path-byte order, whatever order the changes
    /// arrive in: `A` < `a` < `b` < `\xFF`.
    #[test]
    fn problems_are_sorted_by_path_bytes() {
        let changes = [added(b"b"), added(b"a"), added(b"\xFF"), added(b"A")];
        let paths: Vec<Vec<u8>> = evaluate(&model_with(b".worktrees", &[]), &changes)
            .into_iter()
            .map(|problem| problem.path)
            .collect();
        assert_eq!(
            paths,
            [
                b"A".to_vec(),
                b"a".to_vec(),
                b"b".to_vec(),
                b"\xFF".to_vec()
            ]
        );
    }
}
