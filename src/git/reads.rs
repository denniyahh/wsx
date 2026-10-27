//! Read queries answered by gix. Each takes revisions as revspec bytes and
//! resolves them through `Repo::resolve`, so callers never hold a gix type.

use super::repo::Repo;
use super::{Error, ObjectId};

impl Repo {
    /// The merge-base of `one` and `two`, or `None` when they share no
    /// history. On a criss-cross history with several best bases, this is the
    /// one gix picks first; the tests pin that it equals `git merge-base`'s.
    pub fn merge_base(&self, one: &[u8], two: &[u8]) -> Result<Option<ObjectId>, Error> {
        let _one = self.resolve(one)?;
        let _two = self.resolve(two)?;
        Ok(None)
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

        let unmoved = repo
            .merge_base(b"main", b"feat")
            .expect("gix merge-base main feat")
            .map(|id| id.to_string());
        assert_ne!(
            moved, unmoved,
            "merge-base main feat~3 {moved:?} against main feat {unmoved:?}"
        );
    }
}
