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

    /// The objects `revs` name, in order.
    fn resolve_all(&self, revs: &[&[u8]]) -> Result<Vec<gix::ObjectId>, Error> {
        revs.iter().map(|rev| self.resolve(rev)).collect()
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

    use super::super::ObjectId;
    use super::super::spawn::Git;
    use super::super::testing::{Scratch, criss_cross};
    use super::{AheadBehind, Repo};

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
}
