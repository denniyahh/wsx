//! Read queries answered by gix. Each takes revisions as revspec bytes and
//! resolves them through `Repo::resolve`, so callers never hold a gix type.

use super::repo::Repo;
use super::{Error, ObjectId};

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
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::spawn::Git;
    use super::super::testing::{Scratch, criss_cross};
    use super::Repo;

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
}
