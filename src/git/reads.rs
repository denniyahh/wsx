//! Read queries answered by gix. Each takes revisions as revspec bytes and
//! resolves them through `Repo::resolve`, so callers never hold a gix type.

use std::collections::BTreeSet;

use super::repo::Repo;
use super::{Error, ObjectId};

/// How far two revisions have diverged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AheadBehind {
    /// Commits reachable from the first revision and not from the second
    /// (`git rev-list --count second..first`).
    pub ahead: usize,
    /// Commits reachable from the second revision and not from the first
    /// (`git rev-list --count first..second`).
    pub behind: usize,
}

/// How a path differs between two trees.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeKind {
    /// The path exists only in the second tree.
    Added,
    /// The path exists only in the first tree.
    Deleted,
    /// The path's content or executable bit changed, its entry kind did not.
    Modified,
    /// The path's entry kind changed between blob (plain or executable,
    /// which count as one kind), symlink and submodule commit, as git's `T`.
    TypeChanged,
}

/// One changed file between two trees: what changed and at which path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TreeChange {
    /// How the path changed.
    pub kind: ChangeKind,
    /// The path from the tree root, as raw bytes with `/` separators.
    pub path: Vec<u8>,
}

impl Repo {
    /// The merge-base of `one` and `two`, or `None` when they share no
    /// history. On a criss-cross history with several best bases, this is the
    /// one gix picks first; the tests pin that it equals `git merge-base`'s on
    /// one such history, which shows gix can match git there, not that it
    /// always does.
    pub fn merge_base(&self, one: &[u8], two: &[u8]) -> Result<Option<ObjectId>, Error> {
        let one_id = self.resolve(one)?;
        let two_id = self.resolve(two)?;
        let base = self
            .repo
            .merge_base(one_id, two_id)
            .map_err(|err| Error::Gix {
                what: format!(
                    "merge-base {} {}",
                    String::from_utf8_lossy(one),
                    String::from_utf8_lossy(two)
                ),
                message: err.to_string(),
            })?;
        Ok(base.map(|id| ObjectId::from_gix(&id)))
    }

    /// How many commits `first` has that `second` lacks, and the reverse.
    /// gix has no ahead/behind query, so each side is a walk from one
    /// revision with the other hidden, as `git rev-list --count` counts.
    pub fn ahead_behind(&self, first: &[u8], second: &[u8]) -> Result<AheadBehind, Error> {
        Ok(AheadBehind {
            ahead: self.range_walk(&[first], &[second])?.len(),
            behind: self.range_walk(&[second], &[first])?.len(),
        })
    }

    /// Every commit reachable from one of `tips` and from none of `hidden`
    /// (`git rev-list <tips> --not <hidden>`). The result is a set: no walk
    /// order is promised (gix walks breadth-first topologically, git
    /// reverse-chronologically). No tips is the empty set, without a walk.
    pub fn range_walk(
        &self,
        tips: &[&[u8]],
        hidden: &[&[u8]],
    ) -> Result<BTreeSet<ObjectId>, Error> {
        if tips.is_empty() {
            return Ok(BTreeSet::new());
        }
        let tip_ids = self.resolve_all(tips)?;
        let hidden_ids = self.resolve_all(hidden)?;
        let failed = |message: String| Error::Gix {
            what: format!("walk {} --not {}", render_revs(tips), render_revs(hidden)),
            message,
        };
        let walk = self
            .repo
            .rev_walk(tip_ids)
            .with_hidden(hidden_ids)
            .all()
            .map_err(|err| failed(err.to_string()))?;
        walk.map(|info| {
            info.map(|info| ObjectId::from_gix(&info.id))
                .map_err(|err| failed(err.to_string()))
        })
        .collect()
    }

    /// The files that differ between the trees of `from` and `to`, sorted by
    /// path bytes, then kind, as `git diff-tree -r --no-renames` lists them:
    /// no rename detection (a rename is a deletion plus an addition) and no
    /// directory entries.
    pub fn tree_diff(&self, from: &[u8], to: &[u8]) -> Result<Vec<TreeChange>, Error> {
        use gix::prelude::FindExt;

        let failed = |message: String| Error::Gix {
            what: format!(
                "diff {} {}",
                String::from_utf8_lossy(from),
                String::from_utf8_lossy(to)
            ),
            message,
        };
        let from_tree = self.tree_of(from).map_err(failed)?;
        let to_tree = self.tree_of(to).map_err(failed)?;
        let (mut from_buf, mut to_buf) = (Vec::new(), Vec::new());
        let from_iter = self
            .repo
            .objects
            .find_tree_iter(&from_tree, &mut from_buf)
            .map_err(|err| failed(err.to_string()))?;
        let to_iter = self
            .repo
            .objects
            .find_tree_iter(&to_tree, &mut to_buf)
            .map_err(|err| failed(err.to_string()))?;
        let mut recorder = gix::diff::tree::Recorder::default();
        gix::diff::tree(
            from_iter,
            to_iter,
            gix::diff::tree::State::default(),
            &self.repo.objects,
            &mut recorder,
        )
        .map_err(|err| failed(err.to_string()))?;

        let mut changes: Vec<TreeChange> = recorder
            .records
            .into_iter()
            .filter_map(file_change)
            .collect();
        changes.sort_by(|one, two| one.path.cmp(&two.path).then(one.kind.cmp(&two.kind)));
        Ok(changes)
    }

    /// The tree a revision peels to, or gix's message when it does not.
    fn tree_of(&self, rev: &[u8]) -> Result<gix::ObjectId, String> {
        let id = self.resolve(rev).map_err(|err| err.to_string())?;
        let object = self.repo.find_object(id).map_err(|err| err.to_string())?;
        let tree = object.peel_to_tree().map_err(|err| err.to_string())?;
        Ok(tree.id)
    }

    /// The objects `revs` name, in order.
    fn resolve_all(&self, revs: &[&[u8]]) -> Result<Vec<gix::ObjectId>, Error> {
        revs.iter().map(|rev| self.resolve(rev)).collect()
    }
}

/// The file change a gix tree-diff record stands for, or `None` for a
/// directory entry (git's `-r` lists files only). gix reports a change of
/// entry mode as a modification; it is a type change when the entry kind
/// differs.
fn file_change(record: gix::diff::tree::recorder::Change) -> Option<TreeChange> {
    use gix::diff::tree::recorder::Change;

    let (kind, mode, path) = match record {
        Change::Addition {
            entry_mode, path, ..
        } => (ChangeKind::Added, entry_mode, path),
        Change::Deletion {
            entry_mode, path, ..
        } => (ChangeKind::Deleted, entry_mode, path),
        Change::Modification {
            previous_entry_mode,
            entry_mode,
            path,
            ..
        } => {
            let kind = if entry_class(previous_entry_mode) == entry_class(entry_mode) {
                ChangeKind::Modified
            } else {
                ChangeKind::TypeChanged
            };
            (kind, entry_mode, path)
        }
    };
    (!mode.is_tree()).then(|| TreeChange {
        kind,
        path: path.into(),
    })
}

/// An entry's kind for type-change purposes: a plain and an executable blob
/// are the same kind, as in git.
fn entry_class(mode: gix::object::tree::EntryMode) -> gix::object::tree::EntryKind {
    match mode.kind() {
        gix::object::tree::EntryKind::BlobExecutable => gix::object::tree::EntryKind::Blob,
        kind => kind,
    }
}

/// `revs` as one space-separated line for a message.
fn render_revs(revs: &[&[u8]]) -> String {
    revs.iter()
        .map(|rev| String::from_utf8_lossy(rev))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::super::spawn::Git;
    use super::super::testing::{Scratch, T0, TICK, commit_all, criss_cross, git_ok, write};
    use super::super::{Error, ObjectId};
    use super::{AheadBehind, ChangeKind, Repo, TreeChange};

    /// The first line of a git command's stdout, without its newline.
    fn line(stdout: Vec<u8>) -> String {
        String::from_utf8(stdout)
            .expect("git printed UTF-8")
            .trim_end()
            .to_owned()
    }

    /// Runs `git <args>` in `dir` through wsx's own spawn helper.
    fn git_line(scratch: &Scratch, dir: &Path, args: &[&str]) -> String {
        let git = Git::new(scratch.env.clone());
        let stdout = git
            .run(dir, args)
            .unwrap_or_else(|err| panic!("git {args:?} in {}: {err}", dir.display()));
        line(stdout)
    }

    /// The whole stdout of `git <args>` in `dir`, run through wsx's spawn
    /// helper.
    fn git_stdout(scratch: &Scratch, dir: &Path, args: &[&str]) -> Vec<u8> {
        Git::new(scratch.env.clone())
            .run(dir, args)
            .unwrap_or_else(|err| panic!("git {args:?} in {}: {err}", dir.display()))
    }

    /// git's ahead/behind for `first` against `second`: the left and right
    /// counts of `git rev-list --left-right --count first...second`.
    fn git_ahead_behind(scratch: &Scratch, dir: &Path, first: &str, second: &str) -> AheadBehind {
        let range = format!("{first}...{second}");
        let printed = git_line(
            scratch,
            dir,
            &["rev-list", "--left-right", "--count", range.as_str()],
        );
        let counts: Vec<usize> = printed
            .split('\t')
            .map(|count| {
                count
                    .parse()
                    .unwrap_or_else(|err| panic!("count {count:?} in {printed:?}: {err}"))
            })
            .collect();
        assert_eq!(
            counts.len(),
            2,
            "rev-list --left-right --count {range} printed {printed:?}"
        );
        AheadBehind {
            ahead: counts[0],
            behind: counts[1],
        }
    }

    /// The ids a git command prints, one hex id per line.
    fn git_ids(scratch: &Scratch, dir: &Path, args: &[&str]) -> BTreeSet<String> {
        String::from_utf8(git_stdout(scratch, dir, args))
            .expect("git printed UTF-8")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// git's file changes from `from` to `to`, parsed from
    /// `git diff-tree -r --no-renames --name-status -z` and sorted by path,
    /// then kind. A status letter other than A, D, M or T fails the test.
    fn git_tree_diff(scratch: &Scratch, dir: &Path, from: &str, to: &str) -> Vec<TreeChange> {
        let raw = git_stdout(
            scratch,
            dir,
            &[
                "diff-tree",
                "-r",
                "--no-renames",
                "--name-status",
                "-z",
                from,
                to,
            ],
        );
        let mut fields: Vec<&[u8]> = raw.split(|byte| *byte == 0).collect();
        if fields.last() == Some(&&b""[..]) {
            fields.pop();
        }
        assert_eq!(
            fields.len() % 2,
            0,
            "diff-tree {from} {to} printed an odd field count: {:?}",
            String::from_utf8_lossy(&raw)
        );
        let mut changes: Vec<TreeChange> = fields
            .chunks(2)
            .map(|pair| {
                let kind = match pair[0] {
                    b"A" => ChangeKind::Added,
                    b"D" => ChangeKind::Deleted,
                    b"M" => ChangeKind::Modified,
                    b"T" => ChangeKind::TypeChanged,
                    other => panic!(
                        "diff-tree {from} {to}: status {:?} in {:?}",
                        String::from_utf8_lossy(other),
                        String::from_utf8_lossy(&raw)
                    ),
                };
                TreeChange {
                    kind,
                    path: pair[1].to_vec(),
                }
            })
            .collect();
        changes.sort_by(|one, two| one.path.cmp(&two.path).then(one.kind.cmp(&two.kind)));
        changes
    }

    /// A `TreeChange` for the test's expected lists.
    fn change(kind: ChangeKind, path: &str) -> TreeChange {
        TreeChange {
            kind,
            path: path.as_bytes().to_vec(),
        }
    }

    /// wsx's ids as the hex strings git prints.
    fn hex(ids: &BTreeSet<ObjectId>) -> BTreeSet<String> {
        ids.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn merge_base_matches_git() {
        let scratch = Scratch::new("merge-base").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let canonical = repo_dir.canonicalize().expect("canonicalize fixture");
        let dot_git = canonical.join(".git");

        let repo = Repo::discover(&repo_dir.join("f")).expect("discover from f");
        assert_eq!(
            repo.worktree(),
            Some(canonical.as_path()),
            "worktree resolved from {}/f",
            repo_dir.display()
        );
        assert_eq!(
            repo.git_dir(),
            dot_git,
            "git dir against {}",
            dot_git.display()
        );
        assert_eq!(
            repo.common_dir(),
            dot_git,
            "common dir against {}",
            dot_git.display()
        );

        let from_git = git_line(&scratch, &repo_dir, &["merge-base", "main", "feat"]);
        let from_gix = repo
            .merge_base(b"main", b"feat")
            .expect("gix merge-base main feat")
            .map(|id| id.to_string());
        assert_eq!(
            from_gix,
            Some(from_git.clone()),
            "gix merge-base main feat against git's {from_git}"
        );

        let all = git_line(
            &scratch,
            &repo_dir,
            &["merge-base", "--all", "main", "feat"],
        );
        let candidates: Vec<&str> = all.lines().collect();
        assert_eq!(
            candidates.len(),
            2,
            "git merge-base --all main feat printed {candidates:?}"
        );
        assert!(
            candidates.contains(&from_git.as_str()),
            "{from_git} is not among git merge-base --all's {candidates:?}"
        );
    }

    #[test]
    fn merge_base_moves_with_the_pair() {
        let scratch = Scratch::new("merge-base-moves").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let f1 = git_line(&scratch, &repo_dir, &["rev-parse", "feat~3"]);
        let from_git = git_line(&scratch, &repo_dir, &["merge-base", "main", "feat~3"]);
        let moved = repo
            .merge_base(b"main", b"feat~3")
            .expect("gix merge-base main feat~3")
            .map(|id| id.to_string());
        assert_eq!(
            moved,
            Some(from_git.clone()),
            "gix merge-base main feat~3 against git's {from_git}"
        );
        assert_eq!(from_git, f1, "git merge-base main feat~3 against F1 {f1}");

        // git's single base for main/feat is also F1 (the newer of the two
        // criss-cross bases), so the answer must be moved by a pair below the
        // cross: main~2 is C1, an ancestor of feat, so the base becomes C1.
        let c1 = git_line(&scratch, &repo_dir, &["rev-parse", "main~2"]);
        let from_git_c1 = git_line(&scratch, &repo_dir, &["merge-base", "main~2", "feat"]);
        let other = repo
            .merge_base(b"main~2", b"feat")
            .expect("gix merge-base main~2 feat")
            .map(|id| id.to_string());
        assert_eq!(
            other,
            Some(from_git_c1.clone()),
            "gix merge-base main~2 feat against git's {from_git_c1}"
        );
        assert_eq!(
            from_git_c1, c1,
            "git merge-base main~2 feat against C1 {c1}"
        );
        assert_ne!(
            moved, other,
            "merge-base main feat~3 {moved:?} against main~2 feat {other:?}"
        );
    }

    #[test]
    fn ahead_behind_matches_git() {
        let scratch = Scratch::new("ahead-behind").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        for (first, second) in [("main", "feat"), ("feat", "main"), ("main~1", "feat~2")] {
            let from_git = git_ahead_behind(&scratch, &repo_dir, first, second);
            let from_gix = repo
                .ahead_behind(first.as_bytes(), second.as_bytes())
                .unwrap_or_else(|err| panic!("gix ahead/behind {first} {second}: {err}"));
            assert_eq!(
                from_gix, from_git,
                "ahead/behind {first} {second}: gix {from_gix:?} against git {from_git:?}"
            );
        }

        let main_feat = repo
            .ahead_behind(b"main", b"feat")
            .expect("gix ahead/behind main feat");
        assert_eq!(
            main_feat,
            AheadBehind {
                ahead: 2,
                behind: 3
            },
            "ahead/behind main feat: main-only M1, C2; feat-only F2, F3, F4"
        );
    }

    #[test]
    fn ahead_behind_changes_with_the_pair() {
        let scratch = Scratch::new("ahead-behind-moves").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let from_git = git_ahead_behind(&scratch, &repo_dir, "main", "feat~3");
        let moved = repo
            .ahead_behind(b"main", b"feat~3")
            .expect("gix ahead/behind main feat~3");
        assert_eq!(
            moved, from_git,
            "ahead/behind main feat~3: gix {moved:?} against git {from_git:?}"
        );
        assert_eq!(
            moved,
            AheadBehind {
                ahead: 3,
                behind: 0
            },
            "ahead/behind main feat~3: F1 is an ancestor of main"
        );

        let main_feat = repo
            .ahead_behind(b"main", b"feat")
            .expect("gix ahead/behind main feat");
        assert_ne!(
            moved, main_feat,
            "ahead/behind main feat~3 {moved:?} against main feat {main_feat:?}"
        );
    }

    #[test]
    fn range_walk_matches_git() {
        let scratch = Scratch::new("range-walk").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let walked = hex(&repo
            .range_walk(&[b"feat"], &[b"main"])
            .expect("gix walk feat --not main"));
        let from_git = git_ids(&scratch, &repo_dir, &["rev-list", "feat", "--not", "main"]);
        assert_eq!(
            walked, from_git,
            "walk feat --not main: gix against git rev-list"
        );

        let subjects: BTreeSet<String> = walked
            .iter()
            .map(|id| git_line(&scratch, &repo_dir, &["log", "-1", "--format=%s", id]))
            .collect();
        let expected: BTreeSet<String> = ["f2 merge main", "f3 skewed", "f4 feat"]
            .map(str::to_owned)
            .into();
        assert_eq!(
            subjects, expected,
            "subjects of walk feat --not main {walked:?}"
        );
    }

    #[test]
    fn range_walk_changes_with_the_hidden_tip() {
        let scratch = Scratch::new("range-walk-moves").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let moved = hex(&repo
            .range_walk(&[b"feat"], &[b"feat~3"])
            .expect("gix walk feat --not feat~3"));
        let from_git = git_ids(
            &scratch,
            &repo_dir,
            &["rev-list", "feat", "--not", "feat~3"],
        );
        assert_eq!(
            moved, from_git,
            "walk feat --not feat~3: gix against git rev-list"
        );
        let c1 = git_line(&scratch, &repo_dir, &["rev-parse", "main~2"]);
        let mut expected = git_ids(&scratch, &repo_dir, &["rev-list", "feat", "--not", "main"]);
        expected.insert(c1);
        assert_eq!(moved, expected, "walk feat --not feat~3 is C1, F2, F3, F4");

        let walked = hex(&repo
            .range_walk(&[b"feat"], &[b"main"])
            .expect("gix walk feat --not main"));
        assert_ne!(
            moved, walked,
            "walk feat --not feat~3 against feat --not main"
        );
    }

    #[test]
    fn tree_diff_matches_git() {
        let scratch = Scratch::new("tree-diff").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let from_gix = repo
            .tree_diff(b"main", b"feat")
            .expect("gix diff main feat");
        let from_git = git_tree_diff(&scratch, &repo_dir, "main", "feat");
        assert_eq!(
            from_gix, from_git,
            "diff main feat: gix against git diff-tree --no-renames"
        );

        // The rename f/x -> f/z is a deletion plus an addition, and the file
        // p replaced by a symlink is a type change.
        let expected = vec![
            change(ChangeKind::Modified, "b.txt"),
            change(ChangeKind::Deleted, "f/x"),
            change(ChangeKind::Added, "f/z"),
            change(ChangeKind::Deleted, "new/y"),
            change(ChangeKind::TypeChanged, "p"),
            change(ChangeKind::Added, "q"),
        ];
        assert_eq!(
            from_gix, expected,
            "diff main feat against the fixture's changes"
        );
    }

    #[test]
    fn tree_diff_changes_with_the_pair() {
        let scratch = Scratch::new("tree-diff-moves").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let moved = repo
            .tree_diff(b"main", b"feat~3")
            .expect("gix diff main feat~3");
        let from_git = git_tree_diff(&scratch, &repo_dir, "main", "feat~3");
        assert_eq!(
            moved, from_git,
            "diff main feat~3: gix against git diff-tree --no-renames"
        );
        assert!(!moved.is_empty(), "diff main feat~3 is empty");

        let main_feat = repo
            .tree_diff(b"main", b"feat")
            .expect("gix diff main feat");
        assert_ne!(
            moved, main_feat,
            "diff main feat~3 {moved:?} against main feat {main_feat:?}"
        );
    }

    #[test]
    fn tree_diff_of_a_revision_with_itself_is_empty() {
        let scratch = Scratch::new("tree-diff-self").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let from_gix = repo
            .tree_diff(b"feat", b"feat")
            .expect("gix diff feat feat");
        assert_eq!(
            from_gix,
            Vec::<TreeChange>::new(),
            "diff feat feat from gix"
        );
        let raw = git_stdout(
            &scratch,
            &repo_dir,
            &[
                "diff-tree",
                "-r",
                "--no-renames",
                "--name-status",
                "-z",
                "feat",
                "feat",
            ],
        );
        assert_eq!(
            String::from_utf8_lossy(&raw),
            "",
            "git diff-tree feat feat printed something"
        );
    }

    #[test]
    fn a_commit_with_itself_is_its_own_merge_base_and_level() {
        let scratch = Scratch::new("self-pair").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let f4 = git_line(&scratch, &repo_dir, &["rev-parse", "feat"]);
        let from_git = git_line(&scratch, &repo_dir, &["merge-base", "feat", "feat"]);
        let from_gix = repo
            .merge_base(b"feat", b"feat")
            .expect("gix merge-base feat feat")
            .map(|id| id.to_string());
        assert_eq!(
            from_gix,
            Some(from_git.clone()),
            "merge-base feat feat: gix against git's {from_git}"
        );
        assert_eq!(from_git, f4, "git merge-base feat feat against F4 {f4}");

        let level_git = git_ahead_behind(&scratch, &repo_dir, "feat", "feat");
        let level = repo
            .ahead_behind(b"feat", b"feat")
            .expect("gix ahead/behind feat feat");
        assert_eq!(
            level, level_git,
            "ahead/behind feat feat: gix {level:?} against git {level_git:?}"
        );
        assert_eq!(
            level,
            AheadBehind {
                ahead: 0,
                behind: 0
            },
            "ahead/behind feat feat"
        );
    }

    #[test]
    fn parent_and_child_share_the_parent_as_merge_base() {
        let scratch = Scratch::new("parent-child").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let parent = git_line(&scratch, &repo_dir, &["rev-parse", "main~1"]);
        let from_git = git_line(&scratch, &repo_dir, &["merge-base", "main~1", "main"]);
        let from_gix = repo
            .merge_base(b"main~1", b"main")
            .expect("gix merge-base main~1 main")
            .map(|id| id.to_string());
        assert_eq!(
            from_gix,
            Some(from_git.clone()),
            "merge-base main~1 main: gix against git's {from_git}"
        );
        assert_eq!(
            from_git, parent,
            "git merge-base main~1 main against main~1 {parent}"
        );

        let step_git = git_ahead_behind(&scratch, &repo_dir, "main", "main~1");
        let step = repo
            .ahead_behind(b"main", b"main~1")
            .expect("gix ahead/behind main main~1");
        assert_eq!(
            step, step_git,
            "ahead/behind main main~1: gix {step:?} against git {step_git:?}"
        );
        assert_eq!(
            step,
            AheadBehind {
                ahead: 1,
                behind: 0
            },
            "ahead/behind main main~1"
        );
    }

    #[test]
    fn unrelated_histories_have_no_merge_base() {
        let scratch = Scratch::new("unrelated").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        git_ok(&scratch, &repo_dir, ["switch", "-q", "--orphan", "lone"])
            .expect("switch to orphan branch lone");
        write(&repo_dir.join("lone.txt"), b"lone\n").expect("write lone.txt");
        commit_all(&scratch, &repo_dir, "l1 lone", T0 + 7 * TICK).expect("commit on lone");
        git_ok(&scratch, &repo_dir, ["switch", "-q", "main"]).expect("switch back to main");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let from_gix = repo
            .merge_base(b"main", b"lone")
            .expect("gix merge-base main lone");
        assert_eq!(from_gix, None, "merge-base main lone from gix");

        let from_git = Git::new(scratch.env.clone()).run(&repo_dir, ["merge-base", "main", "lone"]);
        match from_git {
            Err(Error::GitFailed {
                status: Some(1),
                ref stderr,
                ..
            }) => assert_eq!(
                String::from_utf8_lossy(stderr),
                "",
                "git merge-base main lone stderr"
            ),
            other => panic!("git merge-base main lone: expected exit 1, got {other:?}"),
        }
    }

    #[test]
    fn range_walk_with_every_tip_hidden_is_empty() {
        let scratch = Scratch::new("walk-hidden").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let walked = repo
            .range_walk(&[b"feat"], &[b"feat"])
            .expect("gix walk feat --not feat");
        assert_eq!(walked, BTreeSet::new(), "walk feat --not feat from gix");
        let from_git = git_stdout(&scratch, &repo_dir, &["rev-list", "feat", "--not", "feat"]);
        assert_eq!(
            String::from_utf8_lossy(&from_git),
            "",
            "git rev-list feat --not feat printed something"
        );
    }

    #[test]
    fn range_walk_without_tips_is_empty() {
        let scratch = Scratch::new("walk-no-tips").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let walked = repo
            .range_walk(&[], &[b"main"])
            .expect("gix walk --not main");
        assert_eq!(walked, BTreeSet::new(), "walk --not main from gix");
        let from_git = git_stdout(&scratch, &repo_dir, &["rev-list", "--not", "main"]);
        assert_eq!(
            String::from_utf8_lossy(&from_git),
            "",
            "git rev-list --not main printed something"
        );
    }

    #[test]
    fn an_executable_bit_change_is_a_modification() {
        let scratch = Scratch::new("exec-bit").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let a_txt = repo_dir.join("a.txt");
        std::fs::set_permissions(&a_txt, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap_or_else(|err| panic!("chmod 755 {}: {err}", a_txt.display()));
        commit_all(&scratch, &repo_dir, "c3 exec", T0 + 7 * TICK).expect("commit the mode change");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let from_gix = repo
            .tree_diff(b"main~1", b"main")
            .expect("gix diff main~1 main");
        let from_git = git_tree_diff(&scratch, &repo_dir, "main~1", "main");
        assert_eq!(
            from_gix, from_git,
            "diff main~1 main: gix against git diff-tree --no-renames"
        );
        assert_eq!(
            from_gix,
            vec![change(ChangeKind::Modified, "a.txt")],
            "diff main~1 main after chmod +x a.txt"
        );
    }

    #[test]
    fn repeated_reads_return_equal_values() {
        let scratch = Scratch::new("repeated").expect("create scratch");
        let repo_dir = criss_cross(&scratch).expect("build criss-cross fixture");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let base = || {
            repo.merge_base(b"main", b"feat")
                .expect("gix merge-base main feat")
        };
        let (first, second) = (base(), base());
        assert_eq!(
            first, second,
            "merge-base main feat twice: {first:?} then {second:?}"
        );

        let level = || {
            repo.ahead_behind(b"main", b"feat")
                .expect("gix ahead/behind main feat")
        };
        let (first, second) = (level(), level());
        assert_eq!(
            first, second,
            "ahead/behind main feat twice: {first:?} then {second:?}"
        );

        let walk = || {
            repo.range_walk(&[b"feat"], &[b"main"])
                .expect("gix walk feat --not main")
        };
        let (first, second) = (walk(), walk());
        assert_eq!(
            first, second,
            "walk feat --not main twice: {first:?} then {second:?}"
        );

        let diff = || {
            repo.tree_diff(b"main", b"feat")
                .expect("gix diff main feat")
        };
        let (first, second) = (diff(), diff());
        assert_eq!(
            first, second,
            "diff main feat twice: {first:?} then {second:?}"
        );
    }
}
