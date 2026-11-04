//! Peeling object ids to the commits git names for them.

use super::repo::Repo;
use super::{Error, ObjectId};

impl Repo {
    /// The commit `id` names, peeling annotated tags as git's
    /// `<id>^{commit}` does. No ref is read and no revspec is parsed: `id`
    /// is decoded as hex, so a name or a short prefix is refused.
    pub fn peel_commit(&self, id: &ObjectId) -> Result<ObjectId, Error> {
        self.peeled_commit(id)
            .map(|commit| ObjectId::from_gix(&commit.id))
    }

    /// The commit `id` peels to, as gix's commit. A decode failure is
    /// `ObjectId::to_gix`'s error; a missing object, or one that does not
    /// peel to a commit, is `Error::Gix` naming `id`.
    fn peeled_commit(&self, id: &ObjectId) -> Result<gix::Commit<'_>, Error> {
        let gix_id = id.to_gix()?;
        self.repo
            .find_object(gix_id)
            .map_err(|err| err.to_string())
            .and_then(|object| object.peel_to_commit().map_err(|err| err.to_string()))
            .map_err(|message| Error::Gix {
                what: format!("peel {id} to a commit"),
                message,
            })
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Output;

    use super::super::testing::{self, Scratch, T0, TICK, commit_all, git_ok, write};
    use super::super::{Error, ObjectId};
    use super::Repo;

    /// `<root>/repo` with `a.txt` and `sub/b.txt` committed on `main`, the
    /// annotated tag `v1` of that commit, `v1nest` of `v1` and `vtree` of
    /// the commit's tree, each checked by git to be a tag object.
    fn tagged_repo(scratch: &Scratch) -> PathBuf {
        let repo_dir = scratch.root.join("repo");
        std::fs::create_dir_all(&repo_dir)
            .unwrap_or_else(|err| panic!("create {}: {err}", repo_dir.display()));
        let ok =
            |args: &[&str]| git_ok(scratch, &repo_dir, args).unwrap_or_else(|err| panic!("{err}"));
        ok(&["init", "-q", "-b", "main"]);
        write(&repo_dir.join("a.txt"), b"a\n").expect("write a.txt");
        write(&repo_dir.join("sub/b.txt"), b"b\n").expect("write sub/b.txt");
        commit_all(scratch, &repo_dir, "first", T0).expect("commit");
        ok(&["tag", "-a", "v1", "-m", "v1", "HEAD"]);
        ok(&["tag", "-a", "v1nest", "-m", "v1nest", "v1"]);
        ok(&["tag", "-a", "vtree", "-m", "vtree", "HEAD^{tree}"]);
        for tag in ["v1", "v1nest", "vtree"] {
            let kind = ok(&["cat-file", "-t", tag]);
            assert_eq!(
                String::from_utf8_lossy(&kind).trim_end(),
                "tag",
                "{tag} must be an annotated tag object"
            );
        }
        repo_dir
    }

    /// The object id git names for `rev` in `dir`, from
    /// `testing::object_id` (`git rev-parse --verify`), never from gix.
    fn id(scratch: &Scratch, dir: &Path, rev: &str) -> ObjectId {
        testing::object_id(scratch, dir, rev).unwrap_or_else(|err| panic!("{err}"))
    }

    /// git's peel of `hex` to a commit, `git rev-parse --verify
    /// <hex>^{commit}` in `dir`, whatever its exit status.
    fn git_peel(scratch: &Scratch, dir: &Path, hex: &str) -> Output {
        let rev = format!("{hex}^{{commit}}");
        testing::git(scratch, dir, ["rev-parse", "--verify", rev.as_str()])
            .unwrap_or_else(|err| panic!("{err}"))
    }

    /// The commit git peels `hex` to in `dir`, as hex; fails the test,
    /// printing git's stderr, when git refuses.
    fn git_peeled(scratch: &Scratch, dir: &Path, hex: &str) -> String {
        let output = git_peel(scratch, dir, hex);
        assert_eq!(
            output.status.code(),
            Some(0),
            "git rev-parse --verify {hex}^{{commit}} refused: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned()
    }

    /// git refuses to peel `id` to a commit in `dir`, and so does
    /// `peel_commit`, as `Error::Gix` rendering `peel <hex> to a commit: `
    /// followed by gix's message.
    fn assert_refused_as_git_does(scratch: &Scratch, dir: &Path, repo: &Repo, id: &ObjectId) {
        let hex = id.to_string();
        let git = git_peel(scratch, dir, &hex);
        assert_ne!(
            git.status.code(),
            Some(0),
            "git rev-parse --verify {hex}^{{commit}} accepted it, printing {:?}",
            String::from_utf8_lossy(&git.stdout).trim_end()
        );
        let prefix = format!("peel {hex} to a commit: ");
        match repo.peel_commit(id) {
            Err(err @ Error::Gix { .. }) => {
                let rendered = err.to_string();
                let message = rendered.strip_prefix(prefix.as_str());
                assert!(
                    message.is_some_and(|message| !message.is_empty()),
                    "peel_commit({hex}) rendered {rendered:?}, not {prefix:?} and gix's message"
                );
            }
            other => panic!(
                "peel_commit({hex}): expected Error::Gix, got {other:?}; git refused with {}",
                String::from_utf8_lossy(&git.stderr).trim_end()
            ),
        }
    }

    /// `<root>/<name>`, made with `git init --object-format=<format>`, with
    /// `a.txt` committed on `main` at `T0` and the annotated tag `v1` of
    /// that commit; canonical.
    fn repo_with_format(scratch: &Scratch, name: &str, format: &str) -> PathBuf {
        let repo_dir = scratch.root.join(name);
        let repo_arg = repo_dir.to_str().expect("UTF-8 scratch path");
        let format_arg = format!("--object-format={format}");
        git_ok(
            scratch,
            &scratch.root,
            ["init", "-q", "-b", "main", format_arg.as_str(), repo_arg],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        write(&repo_dir.join("a.txt"), b"a\n").expect("write a.txt");
        commit_all(scratch, &repo_dir, "first", T0).expect("commit");
        git_ok(scratch, &repo_dir, ["tag", "-a", "v1", "-m", "v1", "HEAD"])
            .unwrap_or_else(|err| panic!("{err}"));
        repo_dir
            .canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize {}: {err}", repo_dir.display()))
    }

    /// The rendering of `peel_commit(id)`'s error; fails the test when it
    /// succeeds.
    fn peel_refusal(repo: &Repo, id: &ObjectId) -> String {
        match repo.peel_commit(id) {
            Err(err) => err.to_string(),
            Ok(peeled) => panic!("peel_commit({id:?}) accepted it, peeling to {peeled}"),
        }
    }

    #[test]
    fn peel_commit_of_a_commit_is_that_commit() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-commit").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let head = id(&scratch, &repo_dir, "HEAD");
        let from_git = git_peeled(&scratch, &repo_dir, &head.to_string());
        assert_eq!(
            from_git,
            head.to_string(),
            "git peels commit {head} to itself"
        );
        let peeled = repo
            .peel_commit(&head)
            .unwrap_or_else(|err| panic!("peel_commit({head}): {err}"));
        assert_eq!(
            peeled.to_string(),
            from_git,
            "peel_commit({head}) against git's {from_git}"
        );
    }

    #[test]
    fn peel_commit_peels_an_annotated_tag_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-tag").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let tag = id(&scratch, &repo_dir, "v1");
        let from_git = git_peeled(&scratch, &repo_dir, &tag.to_string());
        let peeled = repo
            .peel_commit(&tag)
            .unwrap_or_else(|err| panic!("peel_commit({tag}): {err}"));
        assert_eq!(
            peeled.to_string(),
            from_git,
            "peel_commit of tag v1 {tag} against git's {from_git}"
        );
        assert_ne!(peeled, tag, "peel_commit returned the tag v1's own id");
    }

    #[test]
    fn peel_commit_peels_a_nested_tag_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-nested-tag").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let outer = id(&scratch, &repo_dir, "v1nest");
        let inner = id(&scratch, &repo_dir, "v1");
        let from_git = git_peeled(&scratch, &repo_dir, &outer.to_string());
        let peeled = repo
            .peel_commit(&outer)
            .unwrap_or_else(|err| panic!("peel_commit({outer}): {err}"));
        assert_eq!(
            peeled.to_string(),
            from_git,
            "peel_commit of tag v1nest {outer} against git's {from_git}"
        );
        assert_ne!(
            peeled, outer,
            "peel_commit returned the tag v1nest's own id"
        );
        assert_ne!(
            peeled, inner,
            "peel_commit stopped at the inner tag v1 {inner}"
        );
    }

    #[test]
    fn peel_commit_refuses_a_tree_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-tree").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let tree = id(&scratch, &repo_dir, "HEAD^{tree}");
        assert_refused_as_git_does(&scratch, &repo_dir, &repo, &tree);
    }

    #[test]
    fn peel_commit_refuses_a_blob_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-blob").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let blob = id(&scratch, &repo_dir, "HEAD:a.txt");
        assert_refused_as_git_does(&scratch, &repo_dir, &repo, &blob);
    }

    #[test]
    fn peel_commit_refuses_a_tag_of_a_tree_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-tree-tag").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let tag = id(&scratch, &repo_dir, "vtree");
        assert_refused_as_git_does(&scratch, &repo_dir, &repo, &tag);
    }

    #[test]
    fn peel_commit_refuses_a_missing_object_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-missing").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let other_dir = scratch.root.join("other");
        std::fs::create_dir_all(&other_dir)
            .unwrap_or_else(|err| panic!("create {}: {err}", other_dir.display()));
        git_ok(&scratch, &other_dir, ["init", "-q", "-b", "main"]).expect("git init other");
        write(&other_dir.join("x.txt"), b"x\n").expect("write x.txt");
        commit_all(&scratch, &other_dir, "other", T0 + TICK).expect("commit in other");
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let foreign = id(&scratch, &other_dir, "HEAD");
        let hex = foreign.to_string();
        let present = testing::git(&scratch, &repo_dir, ["cat-file", "-e", hex.as_str()])
            .unwrap_or_else(|err| panic!("{err}"));
        assert_ne!(
            present.status.code(),
            Some(0),
            "{hex} from <root>/other exists in <root>/repo"
        );
        assert_refused_as_git_does(&scratch, &repo_dir, &repo, &foreign);
    }

    #[test]
    fn peel_commit_refuses_a_sha1_id_in_a_sha256_repository() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-sha1-in-sha256").expect("create scratch");
        let sha1_dir = repo_with_format(&scratch, "sha1", "sha1");
        let sha256_dir = repo_with_format(&scratch, "sha256", "sha256");
        let repo = Repo::discover(&sha256_dir).expect("discover the SHA-256 fixture");

        let sha1 = id(&scratch, &sha1_dir, "HEAD");
        let git = git_peel(&scratch, &sha256_dir, &sha1.to_string());
        assert_ne!(
            git.status.code(),
            Some(0),
            "git peeled the SHA-1 id {sha1} in a SHA-256 repository to {:?}",
            String::from_utf8_lossy(&git.stdout).trim_end()
        );
        assert_eq!(
            peel_refusal(&repo, &sha1),
            format!("read object id {sha1}: a sha1 id in a sha256 repository")
        );
    }

    #[test]
    fn peel_commit_refuses_a_sha256_id_in_a_sha1_repository() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-sha256-in-sha1").expect("create scratch");
        let sha1_dir = repo_with_format(&scratch, "sha1", "sha1");
        let sha256_dir = repo_with_format(&scratch, "sha256", "sha256");
        let repo = Repo::discover(&sha1_dir).expect("discover the SHA-1 fixture");

        let sha256 = id(&scratch, &sha256_dir, "HEAD");
        let git = git_peel(&scratch, &sha1_dir, &sha256.to_string());
        assert_ne!(
            git.status.code(),
            Some(0),
            "git peeled the SHA-256 id {sha256} in a SHA-1 repository to {:?}",
            String::from_utf8_lossy(&git.stdout).trim_end()
        );
        assert_eq!(
            peel_refusal(&repo, &sha256),
            format!("read object id {sha256}: a sha256 id in a sha1 repository")
        );
    }

    #[test]
    fn peel_commit_peels_a_tag_in_a_sha256_repository() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-sha256-tag").expect("create scratch");
        let sha256_dir = repo_with_format(&scratch, "sha256", "sha256");
        let repo = Repo::discover(&sha256_dir).expect("discover the SHA-256 fixture");

        let tag = id(&scratch, &sha256_dir, "v1");
        let from_git = git_peeled(&scratch, &sha256_dir, &tag.to_string());
        let peeled = repo
            .peel_commit(&tag)
            .unwrap_or_else(|err| panic!("peel_commit({tag}): {err}"));
        assert_eq!(
            peeled.to_string(),
            from_git,
            "peel_commit of tag v1 {tag} against git's {from_git}"
        );
        assert_eq!(
            from_git.len(),
            64,
            "git's peel {from_git} is not SHA-256 length"
        );
        assert_ne!(peeled, tag, "peel_commit returned the tag v1's own id");
    }
}
