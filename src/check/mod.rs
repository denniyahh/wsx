//! The divergence check (DIV-01): how the workspace branch differs from the
//! integration branch, judged against the manifest.
//!
//! The check reads the repository model first (`model::resolve`, which
//! refuses replace refs before any object is read), then the tree diff
//! between the two tips that model recorded. It never reads HEAD, the index
//! or the working tree (D-02), and it writes nothing. `evaluate` is pure:
//! every judgement is made over the model and the diff, so a ref that moves
//! during a run cannot mix two tips.

pub mod hooks;
pub mod report;

pub use hooks::HookStatus;

use std::collections::BTreeSet;

use crate::git::{ChangeKind, Git, ObjectId, Repo, TreeChange, c_quote};
use crate::model::manifest::{Entry, EntryKind, covers, encode_path};
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
    /// What the check made of `core.hooksPath` (D-03).
    pub hooks: HookStatus,
    /// Every problem found, in D-05's order.
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

/// The kinds of problem the check reports, in D-11's order, which is the
/// order D-05 sorts ties in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProblemKind {
    /// A path only the workspace branch has, which no `only` entry covers.
    UndeclaredAdded,
    /// A shared path the workspace branch changes, which no `modified`
    /// entry names.
    UndeclaredModified,
    /// A shared path the workspace branch deletes, which no `modified`
    /// entry names.
    UndeclaredDeleted,
    /// An `only` entry whose path the integration branch carries.
    StaleOnly,
    /// A `modified` entry that covers no difference between the branches.
    StaleModified,
    /// An entry a built-in personal path already covers.
    RedundantBuiltin,
    /// A file under the hooks directory not committed executable, or a
    /// symlink there that does not resolve to one (DIV-03).
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "check::run reports it from 05-06 task 2")
    )]
    HookNotExecutable,
}

impl ProblemKind {
    /// The kind's name as the JSON output writes it (D-11).
    pub fn name(self) -> &'static str {
        match self {
            ProblemKind::UndeclaredAdded => "undeclared_added",
            ProblemKind::UndeclaredModified => "undeclared_modified",
            ProblemKind::UndeclaredDeleted => "undeclared_deleted",
            ProblemKind::StaleOnly => "stale_only",
            ProblemKind::StaleModified => "stale_modified",
            ProblemKind::RedundantBuiltin => "redundant_builtin",
            ProblemKind::HookNotExecutable => "hook_not_executable",
        }
    }

    /// The kind's group in D-05's order: undeclared paths, stale
    /// declarations, then hook modes.
    fn group(self) -> u8 {
        match self {
            ProblemKind::UndeclaredAdded
            | ProblemKind::UndeclaredModified
            | ProblemKind::UndeclaredDeleted => 0,
            ProblemKind::StaleOnly | ProblemKind::StaleModified | ProblemKind::RedundantBuiltin => {
                1
            }
            ProblemKind::HookNotExecutable => 2,
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
    let carried = carried_only_paths(repo, git, &model)?;
    let problems = evaluate(&model, &changes, &carried);
    Ok(Report {
        configured,
        workspace_branch: model.workspace_branch.clone(),
        workspace_tip: model.workspace_tip.clone(),
        integration: model.integration.clone(),
        hooks: HookStatus::NotSet,
        problems,
    })
}

/// The `only` paths no built-in covers that the integration tip's tree
/// holds, as any entry type: one lookup child for every distinct path, and
/// none when there is no such path. Each path is a NUL-framed request, never
/// a pathspec (T-05-13).
fn carried_only_paths(
    repo: &Repo,
    git: &Git,
    model: &Model,
) -> Result<BTreeSet<Vec<u8>>, model::Error> {
    let builtins = model.builtins();
    let paths: BTreeSet<&[u8]> = model
        .manifest
        .entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Only)
        .map(|entry| entry.path.as_slice())
        .filter(|path| !builtins.iter().any(|builtin| covers(builtin, path)))
        .collect();
    if paths.is_empty() {
        return Ok(BTreeSet::new());
    }
    let paths: Vec<&[u8]> = paths.into_iter().collect();
    let found = repo.lookup_paths(git, &model.integration.tip, &paths)?;
    Ok(paths
        .into_iter()
        .zip(found)
        .filter_map(|(path, entry)| entry.map(|_| path.to_vec()))
        .collect())
}

/// The problems in `changes`, the diff from the integration tip to the
/// workspace tip, judged against `model`'s built-ins and manifest; `carried`
/// holds the `only` paths the integration tip's tree has. Every problem of
/// the run, in D-05's order.
pub fn evaluate(
    model: &Model,
    changes: &[TreeChange],
    carried: &BTreeSet<Vec<u8>>,
) -> Vec<Problem> {
    let builtins = model.builtins();
    let mut problems = Vec::new();
    for change in changes {
        if builtins.iter().any(|builtin| covers(builtin, &change.path)) {
            continue;
        }
        if let Some(kind) = undeclared(model, change) {
            problems.push(Problem {
                kind,
                path: change.path.clone(),
                detail: None,
            });
        }
    }
    problems.extend(
        distinct_entries(&model.manifest.entries)
            .into_iter()
            .filter_map(|entry| judge_entry(entry, &builtins, changes, carried)),
    );
    problems
        .sort_by(|a, b| (a.kind.group(), &a.path, a.kind).cmp(&(b.kind.group(), &b.path, b.kind)));
    problems
}

/// The kind of problem `change` is when the manifest does not declare it:
/// an added path needs an `only` entry covering it, a modified, retyped or
/// deleted one a `modified` entry equal to it.
fn undeclared(model: &Model, change: &TreeChange) -> Option<ProblemKind> {
    match change.kind {
        ChangeKind::Added => {
            (!declared_only(model, &change.path)).then_some(ProblemKind::UndeclaredAdded)
        }
        ChangeKind::Modified | ChangeKind::TypeChanged => {
            (!declared_modified(model, &change.path)).then_some(ProblemKind::UndeclaredModified)
        }
        ChangeKind::Deleted => {
            (!declared_modified(model, &change.path)).then_some(ProblemKind::UndeclaredDeleted)
        }
    }
}

/// `entries` with each exact duplicate (same kind, same path) dropped, the
/// first kept.
fn distinct_entries(entries: &[Entry]) -> Vec<&Entry> {
    let mut seen = BTreeSet::new();
    entries
        .iter()
        .filter(|entry| seen.insert((entry.kind == EntryKind::Only, entry.path.as_slice())))
        .collect()
}

/// The problem with `entry`, if any: redundant when a built-in covers it,
/// and then nothing else; otherwise a stale `only` entry is one whose path
/// is `carried`, a stale `modified` entry one that covers no change.
fn judge_entry(
    entry: &Entry,
    builtins: &[&[u8]],
    changes: &[TreeChange],
    carried: &BTreeSet<Vec<u8>>,
) -> Option<Problem> {
    if let Some(builtin) = builtins.iter().find(|builtin| covers(builtin, &entry.path)) {
        return Some(Problem {
            kind: ProblemKind::RedundantBuiltin,
            path: entry.path.clone(),
            detail: Some(redundant_detail(entry, builtin)),
        });
    }
    let stale = match entry.kind {
        EntryKind::Only => carried
            .contains(&entry.path)
            .then_some(ProblemKind::StaleOnly),
        EntryKind::Modified => (!changes
            .iter()
            .any(|change| covers(&entry.path, &change.path)))
        .then_some(ProblemKind::StaleModified),
    };
    stale.map(|kind| Problem {
        kind,
        path: entry.path.clone(),
        detail: None,
    })
}

/// The detail of a redundant `entry` that `builtin` covers.
fn redundant_detail(entry: &Entry, builtin: &[u8]) -> String {
    let word = match entry.kind {
        EntryKind::Only => "only",
        EntryKind::Modified => "modified",
    };
    format!(
        "{word} {} is already personal as the built-in {}",
        String::from_utf8_lossy(&encode_path(&entry.path)),
        String::from_utf8_lossy(&c_quote(builtin))
    )
}

/// Whether a `modified` entry of `model`'s manifest names exactly `path`.
fn declared_modified(model: &Model, path: &[u8]) -> bool {
    model
        .manifest
        .entries
        .iter()
        .any(|entry| entry.kind == EntryKind::Modified && entry.path == path)
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
        let entries: Vec<(EntryKind, &[u8])> =
            only.iter().map(|path| (EntryKind::Only, *path)).collect();
        model_of(worktree_dir, &entries)
    }

    /// A model with `worktree_dir` and one manifest entry per item of
    /// `entries`, in order.
    fn model_of(worktree_dir: &[u8], entries: &[(EntryKind, &[u8])]) -> Model {
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
                entries: entries
                    .iter()
                    .enumerate()
                    .map(|(index, (kind, path))| Entry {
                        kind: kind.clone(),
                        path: path.to_vec(),
                        line: index + 1,
                        text: [kind_word(kind), b" ", path].concat(),
                    })
                    .collect(),
            },
        }
    }

    /// The keyword a manifest line of `kind` starts with.
    fn kind_word(kind: &EntryKind) -> &'static [u8] {
        match kind {
            EntryKind::Only => b"only",
            EntryKind::Modified => b"modified",
        }
    }

    /// An `Added` change at `path`.
    fn added(path: &[u8]) -> TreeChange {
        change(ChangeKind::Added, path)
    }

    /// A change of `kind` at `path`.
    fn change(kind: ChangeKind, path: &[u8]) -> TreeChange {
        TreeChange {
            kind,
            path: path.to_vec(),
        }
    }

    /// No carried `only` path.
    fn none() -> BTreeSet<Vec<u8>> {
        BTreeSet::new()
    }

    /// The carried set holding `paths`.
    fn carrying(paths: &[&[u8]]) -> BTreeSet<Vec<u8>> {
        paths.iter().map(|path| path.to_vec()).collect()
    }

    /// The kinds and paths `evaluate` finds in `changes` against a model
    /// with the worktree directory `.worktrees` and `entries`.
    fn judge(
        entries: &[(EntryKind, &[u8])],
        changes: &[TreeChange],
        carried: &BTreeSet<Vec<u8>>,
    ) -> Vec<(&'static str, Vec<u8>)> {
        found(&evaluate(
            &model_of(b".worktrees", entries),
            changes,
            carried,
        ))
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
            found(&evaluate(
                &model_with(b".worktrees", &[]),
                &changes,
                &none()
            )),
            []
        );
        assert_eq!(
            found(&evaluate(&model_with(b".other", &[]), &changes, &none())),
            [("undeclared_added", b".worktrees/x".to_vec())]
        );
    }

    /// An added path is declared by an `only` entry that covers it, and
    /// `covers` is literal: `notesX` does not cover `notes/a`.
    #[test]
    fn added_path_is_declared_only_by_a_covering_only_entry() {
        let changes = [added(b"notes/a")];
        assert_eq!(
            found(&evaluate(
                &model_with(b".worktrees", &[b"notes"]),
                &changes,
                &none()
            )),
            []
        );
        assert_eq!(
            found(&evaluate(
                &model_with(b".worktrees", &[b"notesX"]),
                &changes,
                &none()
            )),
            [("undeclared_added", b"notes/a".to_vec())]
        );
    }

    /// Problems come out in path-byte order, whatever order the changes
    /// arrive in: `A` < `a` < `b` < `\xFF`.
    #[test]
    fn problems_are_sorted_by_path_bytes() {
        let changes = [added(b"b"), added(b"a"), added(b"\xFF"), added(b"A")];
        let paths: Vec<Vec<u8>> = evaluate(&model_with(b".worktrees", &[]), &changes, &none())
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

    /// A modified shared file is declared only by a `modified` entry equal
    /// to its path: not by a prefix of it (`READ`, itself then stale) and
    /// not by an `only` entry.
    #[test]
    fn modified_shared_file_must_be_declared_exactly() {
        let changes = [change(ChangeKind::Modified, b"README.md")];
        assert_eq!(
            judge(&[], &changes, &none()),
            [("undeclared_modified", b"README.md".to_vec())]
        );
        assert_eq!(
            judge(&[(EntryKind::Modified, b"README.md")], &changes, &none()),
            []
        );
        assert_eq!(
            judge(&[(EntryKind::Modified, b"READ")], &changes, &none()),
            [
                ("undeclared_modified", b"README.md".to_vec()),
                ("stale_modified", b"READ".to_vec())
            ]
        );
        assert_eq!(
            judge(&[(EntryKind::Only, b"README.md")], &changes, &none()),
            [("undeclared_modified", b"README.md".to_vec())]
        );
    }

    /// A type change (a file become a symlink, say) is a modification.
    #[test]
    fn type_change_is_an_undeclared_modification() {
        let changes = [change(ChangeKind::TypeChanged, b"x")];
        assert_eq!(
            judge(&[], &changes, &none()),
            [("undeclared_modified", b"x".to_vec())]
        );
        assert_eq!(judge(&[(EntryKind::Modified, b"x")], &changes, &none()), []);
    }

    /// A deleted shared file is declared by a `modified` entry equal to its
    /// path.
    #[test]
    fn deleted_shared_file_must_be_declared_modified() {
        let changes = [change(ChangeKind::Deleted, b"src/a.txt")];
        assert_eq!(
            judge(&[], &changes, &none()),
            [("undeclared_deleted", b"src/a.txt".to_vec())]
        );
        assert_eq!(
            judge(&[(EntryKind::Modified, b"src/a.txt")], &changes, &none()),
            []
        );
    }

    /// A change under a built-in personal path is never a problem; under
    /// another worktree directory the same deletion is undeclared (control).
    #[test]
    fn change_under_a_builtin_is_never_a_problem() {
        let changes = [
            change(ChangeKind::Modified, b".wsx.conf"),
            change(ChangeKind::Deleted, b".worktrees/x"),
        ];
        assert_eq!(judge(&[], &changes, &none()), []);
        assert_eq!(
            found(&evaluate(&model_of(b".other", &[]), &changes, &none())),
            [("undeclared_deleted", b".worktrees/x".to_vec())]
        );
    }

    /// An `only` entry whose path the integration tip carries is stale.
    #[test]
    fn only_entry_the_integration_branch_carries_is_stale() {
        let entries = [(EntryKind::Only, &b"shared"[..])];
        assert_eq!(
            judge(&entries, &[], &carrying(&[b"shared"])),
            [("stale_only", b"shared".to_vec())]
        );
        assert_eq!(judge(&entries, &[], &none()), []);
    }

    /// A `modified` entry that covers no change is stale.
    #[test]
    fn modified_entry_that_no_longer_differs_is_stale() {
        let entries = [(EntryKind::Modified, &b"README.md"[..])];
        assert_eq!(
            judge(&entries, &[], &none()),
            [("stale_modified", b"README.md".to_vec())]
        );
        assert_eq!(
            judge(
                &entries,
                &[change(ChangeKind::Modified, b"README.md")],
                &none()
            ),
            []
        );
    }

    /// `modified docs` declares `docs` alone, as the oracle's `grep -Fxq`,
    /// but covers a change below it, as the oracle's pathspec: the change
    /// is undeclared and the entry is not stale.
    #[test]
    fn modified_entry_declares_its_own_path_and_covers_below_it() {
        assert_eq!(
            judge(
                &[(EntryKind::Modified, b"docs")],
                &[change(ChangeKind::Modified, b"docs/a")],
                &none()
            ),
            [("undeclared_modified", b"docs/a".to_vec())]
        );
    }

    /// An entry a built-in covers is redundant and gets no stale check,
    /// although the `only` path is carried and the `modified` path is
    /// unchanged.
    #[test]
    fn declaration_under_a_builtin_is_redundant_and_nothing_else() {
        let model = model_of(
            b".worktrees",
            &[
                (EntryKind::Only, b".worktrees/x"),
                (EntryKind::Modified, b".wsx.conf"),
            ],
        );
        assert_eq!(
            evaluate(&model, &[], &carrying(&[b".worktrees/x"])),
            [
                Problem {
                    kind: ProblemKind::RedundantBuiltin,
                    path: b".worktrees/x".to_vec(),
                    detail: Some(
                        "only .worktrees/x is already personal as the built-in .worktrees"
                            .to_owned()
                    ),
                },
                Problem {
                    kind: ProblemKind::RedundantBuiltin,
                    path: b".wsx.conf".to_vec(),
                    detail: Some(
                        "modified .wsx.conf is already personal as the built-in .wsx.conf"
                            .to_owned()
                    ),
                },
            ]
        );
    }

    /// An exact duplicate declaration is judged once; a different path is
    /// still judged on its own (control).
    #[test]
    fn exact_duplicate_declarations_report_once() {
        let twice = [
            (EntryKind::Modified, &b"README.md"[..]),
            (EntryKind::Modified, &b"README.md"[..]),
        ];
        assert_eq!(
            judge(&twice, &[], &none()),
            [("stale_modified", b"README.md".to_vec())]
        );
        let distinct = [
            (EntryKind::Modified, &b"README.md"[..]),
            (EntryKind::Modified, &b"README"[..]),
        ];
        assert_eq!(
            judge(&distinct, &[], &none()),
            [
                ("stale_modified", b"README".to_vec()),
                ("stale_modified", b"README.md".to_vec())
            ]
        );
    }

    /// Undeclared changes come before stale declarations, each group in
    /// path-byte order, whatever order the changes and entries arrive in.
    #[test]
    fn problems_are_grouped_then_sorted() {
        let entries = [
            (EntryKind::Only, &b"a"[..]),
            (EntryKind::Only, &b".worktrees/z"[..]),
        ];
        let changes = [added(b"b"), change(ChangeKind::Modified, b"a")];
        assert_eq!(
            judge(&entries, &changes, &carrying(&[b"a"])),
            [
                ("undeclared_modified", b"a".to_vec()),
                ("undeclared_added", b"b".to_vec()),
                ("redundant_builtin", b".worktrees/z".to_vec()),
                ("stale_only", b"a".to_vec())
            ]
        );
    }
}
