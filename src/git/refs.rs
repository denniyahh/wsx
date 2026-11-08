//! Exact ref reads, the root-tree entry of a commit, and peeling object ids
//! to the commits git names for them; callers never hold a gix type.
//!
//! A ref is read only by its exact full name (`find_ref`), the way
//! `git show-ref --verify` reads it, and a ref's commit is the id that read
//! returned, peeled (`ref_commit`). An id is peeled by decoding its hex
//! (`peel_commit`), and `root_entry` takes the commit id the caller already
//! holds. So no read here resolves a name through git's DWIM ladder or passes
//! a name or an id to gix's revspec parser, which reads a name shaped like
//! `describe` output as the object its hex prefixes (`clippy.toml` bans that
//! parser; `banned_gix_revspec_parsers_read_a_describe_shaped_ref_as_another_object`
//! in `repo.rs` pins the ban).

use gix::bstr::ByteSlice;

use super::repo::Repo;
use super::{Error, ObjectId};

/// What a ref holds, read without following it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefTarget {
    /// The ref holds this object id.
    Object(ObjectId),
    /// The ref is symbolic: it names this other full ref.
    Symbolic(Vec<u8>),
}

/// What a commit's root tree holds under one name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RootEntry {
    /// The root tree has no entry of that name.
    Absent,
    /// The entry is a blob with these bytes.
    Blob(Vec<u8>),
}

/// The most refs `Repo::ref_commit` reads for one name, the first included,
/// so four symbolic hops resolve and a fifth is refused. Measured on git
/// 2.56.0: `git rev-parse --verify` resolves a ref four symbolic hops from a
/// commit and refuses one five hops away.
const SYMREF_MAX_DEPTH: usize = 5;

/// Whether `name` is a valid branch name, as `git check-ref-format --branch`
/// decides: `refs/heads/<name>` is a valid full ref name, and `name` is
/// neither `HEAD` nor starts with `-`. A caller reads the branch only as that
/// one full ref.
///
/// gix's full-name check alone agrees with git on every name tried except
/// `HEAD` and a leading `-`, which gix accepts and git refuses, so those two
/// checks are added here. Measured on git 2.56.0 by
/// `branch_names_agree_with_git`: git accepts `develop`, `workspace/dev`,
/// `a/b`, `refs/heads/x` and a name holding the byte E9, and refuses
/// `HEAD`, `-x`, `a..b`, `a b`, `a.lock`, `.a`, `a/` and the empty name.
pub fn is_branch_name(name: &[u8]) -> bool {
    if name == b"HEAD" || name.starts_with(b"-") {
        return false;
    }
    let full = [b"refs/heads/".as_slice(), name].concat();
    gix::refs::FullName::try_from(full.as_bstr()).is_ok()
}

/// `bytes` for a message naming a ref.
fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

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
        let gix_id = id.to_gix(self.repo.object_hash())?;
        self.repo
            .find_object(gix_id)
            .map_err(|err| err.to_string())
            .and_then(|object| object.peel_to_commit().map_err(|err| err.to_string()))
            .map_err(|message| Error::Gix {
                what: format!("peel {id} to a commit"),
                message,
            })
    }

    /// The ref named exactly `full`, read without following it, or `None`
    /// when git has no ref of that name.
    ///
    /// For a full name gix also tries the loose files `.git/heads/<full>`,
    /// `.git/tags/<full>`, `.git/remotes/<full>` and
    /// `.git/remotes/<full>/HEAD` when `<full>` itself is absent, and git
    /// reads none of them (measured on gix 0.90.0 and git 2.56.0). Such a ref
    /// comes back under another name, so a ref whose name differs from `full`
    /// reads as `None`.
    ///
    /// The comparison cannot see a ref namespace: under one, gix reads
    /// `refs/namespaces/<ns>/<full>` and reports the name `<full>`.
    /// `Repo::discover` refuses any namespace gix would apply, so no `Repo`
    /// it returns reaches this read with one. A `Repo` from `Repo::siblings`
    /// is not built by `discover`, a limit recorded where that refusal lives.
    pub fn find_ref(&self, full: &[u8]) -> Result<Option<RefTarget>, Error> {
        let failed = |message: String| Error::Gix {
            what: format!("read {}", lossy(full)),
            message,
        };
        let name =
            gix::refs::FullName::try_from(full.as_bstr()).map_err(|err| failed(err.to_string()))?;
        let found = self
            .repo
            .try_find_reference(name.as_ref())
            .map_err(|err| failed(err.to_string()))?;
        let Some(found) = found else {
            return Ok(None);
        };
        let found_name: &[u8] = found.name().as_bstr();
        if found_name != full {
            return Ok(None);
        }
        Ok(Some(match found.target() {
            gix::refs::TargetRef::Object(id) => RefTarget::Object(ObjectId::from_gix(id)),
            gix::refs::TargetRef::Symbolic(target) => {
                RefTarget::Symbolic(target.as_bstr().to_vec())
            }
        }))
    }

    /// The commit the ref named exactly `full` holds, following symbolic
    /// refs as git does, to at most `SYMREF_MAX_DEPTH` refs in all.
    ///
    /// Each ref on the way is read once, through `find_ref`, and the commit
    /// is the id that read returned, peeled with `peel_commit`; the name is
    /// never resolved again. Resolving it again would let a ref changed
    /// between the two reads hand back a commit the checked read never saw,
    /// and resolving a name through gix's revspec parser reads a name shaped
    /// like `describe` output as another object. The read-once property has
    /// no runtime test: placing a change between two reads needs a seam
    /// here that exists for no other reason, so that race is accepted
    /// (T-04-37), and the source check on this function's shape is a lint,
    /// not evidence for it.
    ///
    /// A missing ref is `Error::Gix` naming the ref read last; a chain
    /// deeper than git follows, a cycle included, is `Error::Gix` naming
    /// `full`.
    pub fn ref_commit(&self, full: &[u8]) -> Result<ObjectId, Error> {
        let mut name = full.to_vec();
        for _ in 0..SYMREF_MAX_DEPTH {
            match self.find_ref(&name)? {
                None => {
                    return Err(Error::Gix {
                        what: format!("read {}", lossy(&name)),
                        message: "no such ref".to_owned(),
                    });
                }
                Some(RefTarget::Object(id)) => return self.peel_commit(&id),
                Some(RefTarget::Symbolic(target)) => name = target,
            }
        }
        Err(Error::Gix {
            what: format!("read {}", lossy(full)),
            message: format!(
                "its symbolic refs go deeper than the {SYMREF_MAX_DEPTH} refs git follows"
            ),
        })
    }

    /// The entry `name` in the root tree of `commit`, read from the object
    /// database, never from a working tree. `commit` is peeled with the
    /// peel `peel_commit` uses, so no revspec is parsed.
    pub fn root_entry(&self, commit: &ObjectId, name: &[u8]) -> Result<RootEntry, Error> {
        let failed = |message: String| Error::Gix {
            what: format!("read {} in the root tree of {commit}", lossy(name)),
            message,
        };
        let tree = self
            .peeled_commit(commit)?
            .tree()
            .map_err(|err| failed(err.to_string()))?;
        let Some(entry) = tree.find_entry(name) else {
            return Ok(RootEntry::Absent);
        };
        let blob = self
            .repo
            .find_blob(entry.oid().to_owned())
            .map_err(|err| failed(err.to_string()))?;
        Ok(RootEntry::Blob(blob.data.clone()))
    }

    /// The full names of the refs exactly one level under `prefix`, in byte
    /// order. Not implemented yet.
    pub fn refs_directly_under(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
        let _ = prefix;
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::process::Output;

    use gix::bstr::{BStr, BString};

    use super::super::testing::{self, Scratch, T0, TICK, commit_all, git_ok, write};
    use super::super::{Error, ObjectId};
    use super::{RefTarget, Repo, is_branch_name};

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

    // The inputs below are not ids git printed, so they are built directly.
    // git resolves each of them; `peel_commit` refuses each at the hex
    // decode, so a name has no path to a commit through the id boundary.

    #[test]
    fn peel_commit_refuses_a_branch_name() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-branch-name").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let head = id(&scratch, &repo_dir, "HEAD");
        assert_eq!(
            git_peeled(&scratch, &repo_dir, "main"),
            head.to_string(),
            "git peels the branch name main to HEAD"
        );
        assert_eq!(
            peel_refusal(&repo, &ObjectId("main".to_owned())),
            "read object id main: A hash sized 4 hexadecimal characters is invalid"
        );
    }

    #[test]
    fn peel_commit_refuses_an_abbreviated_id() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-abbreviated").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        let head = id(&scratch, &repo_dir, "HEAD").to_string();
        let short = head.get(..7).expect("HEAD's id is longer than 7");
        assert_eq!(
            git_peeled(&scratch, &repo_dir, short),
            head,
            "git peels the prefix {short} to HEAD"
        );
        assert_eq!(
            peel_refusal(&repo, &ObjectId(short.to_owned())),
            format!("read object id {short}: A hash sized 7 hexadecimal characters is invalid")
        );
    }

    #[test]
    fn peel_commit_refuses_an_empty_id() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-empty").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let repo = Repo::discover(&repo_dir).expect("discover fixture");

        assert_eq!(
            peel_refusal(&repo, &ObjectId(String::new())),
            "read object id : A hash sized 0 hexadecimal characters is invalid"
        );
    }

    #[test]
    fn peel_commit_refuses_a_describe_shaped_ref_name() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("peel-describe-shaped").expect("create scratch");
        let fixture = testing::describe_shaped_ref(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let repo = Repo::discover(&fixture.repo).expect("discover fixture");

        assert_eq!(
            git_peeled(&scratch, &fixture.repo, &fixture.name),
            fixture.one,
            "git peels {} to ONE",
            fixture.name
        );
        assert_eq!(
            peel_refusal(&repo, &ObjectId(fixture.name.clone())),
            format!(
                "read object id {}: A hash sized {} hexadecimal characters is invalid",
                fixture.name,
                fixture.name.len()
            )
        );
    }

    /// `<root>/work`, a `git clone` of `<root>/source`, which holds `a.txt`
    /// committed on `main` at `T0`. The clone has `refs/remotes/origin/main`
    /// and `refs/remotes/origin/HEAD` symbolic to it. Canonical.
    fn cloned_repo(scratch: &Scratch) -> PathBuf {
        let source = scratch.root.join("source");
        std::fs::create_dir_all(&source)
            .unwrap_or_else(|err| panic!("create {}: {err}", source.display()));
        git_ok(scratch, &source, ["init", "-q", "-b", "main"])
            .unwrap_or_else(|err| panic!("{err}"));
        write(&source.join("a.txt"), b"a\n").expect("write a.txt");
        commit_all(scratch, &source, "first", T0).expect("commit");
        git_ok(scratch, &scratch.root, ["clone", "-q", "source", "work"])
            .unwrap_or_else(|err| panic!("{err}"));
        let work = scratch.root.join("work");
        work.canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize {}: {err}", work.display()))
    }

    /// The id `git show-ref --verify <full>` prints in `dir`, or `None` when
    /// git finds no ref of exactly that name. Not `git rev-parse`, whose DWIM
    /// rules would find a ref under another name.
    fn show_ref(scratch: &Scratch, dir: &Path, full: &str) -> Option<String> {
        let output = testing::git(scratch, dir, ["show-ref", "--verify", full])
            .unwrap_or_else(|err| panic!("{err}"));
        if output.status.code() != Some(0) {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let id = stdout
            .split_whitespace()
            .next()
            .unwrap_or_else(|| panic!("git show-ref --verify {full} exited 0 and printed no id"));
        Some(id.to_owned())
    }

    /// `find_ref(full)`'s answer; fails the test on an error.
    fn read(repo: &Repo, full: &str) -> Option<RefTarget> {
        repo.find_ref(full.as_bytes())
            .unwrap_or_else(|err| panic!("find_ref({full}): {err}"))
    }

    /// `ref_commit(full)`'s answer as hex; fails the test on an error.
    fn commit_of(repo: &Repo, full: &str) -> String {
        repo.ref_commit(full.as_bytes())
            .unwrap_or_else(|err| panic!("ref_commit({full}): {err}"))
            .to_string()
    }

    #[test]
    fn find_ref_tells_direct_from_symbolic() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("find-ref-kinds").expect("create scratch");
        let work = cloned_repo(&scratch);
        let head_target = git_ok(
            &scratch,
            &work,
            ["symbolic-ref", "refs/remotes/origin/HEAD"],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let head_target = String::from_utf8_lossy(&head_target).trim_end().to_owned();
        assert_eq!(
            head_target, "refs/remotes/origin/main",
            "git symbolic-ref refs/remotes/origin/HEAD in the clone"
        );
        let main = show_ref(&scratch, &work, "refs/remotes/origin/main")
            .expect("git show-ref finds refs/remotes/origin/main");
        assert_eq!(
            show_ref(&scratch, &work, "refs/remotes/origin/ghost"),
            None,
            "git show-ref finds refs/remotes/origin/ghost"
        );
        let repo = Repo::discover(&work).expect("discover the clone");

        assert_eq!(
            read(&repo, "refs/remotes/origin/HEAD"),
            Some(RefTarget::Symbolic(head_target.into_bytes())),
            "find_ref(refs/remotes/origin/HEAD) against git symbolic-ref"
        );
        assert_eq!(
            read(&repo, "refs/remotes/origin/main"),
            Some(RefTarget::Object(ObjectId(main.clone()))),
            "find_ref(refs/remotes/origin/main) against git show-ref's {main}"
        );
        assert_eq!(
            read(&repo, "refs/remotes/origin/ghost"),
            None,
            "find_ref(refs/remotes/origin/ghost), which git does not find"
        );
    }

    #[test]
    fn find_ref_refuses_a_ref_read_under_another_name() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("find-ref-exact").expect("create scratch");
        let work = cloned_repo(&scratch);
        let ok =
            |args: &[&str]| git_ok(&scratch, &work, args).unwrap_or_else(|err| panic!("{err}"));
        ok(&["switch", "-q", "-c", "other"]);
        write(&work.join("b.txt"), b"b\n").expect("write b.txt");
        commit_all(&scratch, &work, "other", T0 + TICK).expect("commit");
        ok(&["switch", "-q", "main"]);
        let main = show_ref(&scratch, &work, "refs/remotes/origin/main")
            .expect("git show-ref finds refs/remotes/origin/main");
        let other = show_ref(&scratch, &work, "refs/heads/other")
            .expect("git show-ref finds refs/heads/other");
        assert_ne!(main, other, "the fixture's two commits");

        let git_dir = work.join(".git");
        let ghost = "refs/remotes/origin/ghost";
        write(
            &git_dir.join("heads").join(ghost),
            format!("{main}\n").as_bytes(),
        )
        .expect("write .git/heads/refs/remotes/origin/ghost");
        assert_eq!(
            show_ref(&scratch, &work, ghost),
            None,
            "git show-ref reads {ghost} from .git/heads/"
        );
        let repo = Repo::discover(&work).expect("discover the clone");
        let raw = repo
            .repo
            .try_find_reference(ghost)
            .unwrap_or_else(|err| panic!("try_find_reference({ghost}): {err}"))
            .map(|found| found.name().as_bstr().to_string());
        assert_eq!(
            raw.as_deref(),
            Some("heads/refs/remotes/origin/ghost"),
            "gix's raw lookup of {ghost} reads the file under .git/heads/"
        );
        assert_eq!(read(&repo, ghost), None, "find_ref({ghost})");
        let commit = repo.ref_commit(ghost.as_bytes());
        assert!(commit.is_err(), "ref_commit({ghost}) gave {commit:?}");

        let exact = "refs/remotes/origin/main";
        write(
            &git_dir.join("heads").join(exact),
            format!("{other}\n").as_bytes(),
        )
        .expect("write .git/heads/refs/remotes/origin/main");
        for packing in ["loose", "packed"] {
            if packing == "packed" {
                ok(&["pack-refs", "--all"]);
                assert!(
                    !git_dir.join(exact).exists(),
                    "git pack-refs --all left {exact} loose"
                );
            }
            assert_eq!(
                show_ref(&scratch, &work, exact).as_deref(),
                Some(main.as_str()),
                "git show-ref --verify {exact}, {packing}"
            );
            let repo = Repo::discover(&work).expect("discover the clone");
            assert_eq!(
                read(&repo, exact),
                Some(RefTarget::Object(ObjectId(main.clone()))),
                "find_ref({exact}), {packing}, beside .git/heads/{exact} holding {other}"
            );
            assert_eq!(
                commit_of(&repo, exact),
                main,
                "ref_commit({exact}), {packing}"
            );
        }
    }

    /// `is_branch_name` agrees with `git check-ref-format --branch` on each
    /// name in the table, which git accepts some of and refuses others of,
    /// so a check that always answers one way fails.
    #[test]
    fn branch_names_agree_with_git() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("branch-names").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let names: [&[u8]; 13] = [
            b"develop",
            b"workspace/dev",
            b"a/b",
            b"HEAD",
            b"-x",
            b"a..b",
            b"a b",
            b"a.lock",
            b".a",
            b"a/",
            b"",
            b"\xE9",
            b"refs/heads/x",
        ];
        let (mut accepted, mut refused) = (0, 0);
        for name in names {
            let output = testing::git(
                &scratch,
                &repo_dir,
                [
                    OsStr::new("check-ref-format"),
                    OsStr::new("--branch"),
                    OsStr::from_bytes(name),
                ],
            )
            .unwrap_or_else(|err| panic!("{err}"));
            let shown = BStr::new(name);
            let git_accepts = match output.status.code() {
                Some(0) => true,
                Some(128) => false,
                other => panic!(
                    "git check-ref-format --branch {shown:?} exited {other:?}: {}",
                    String::from_utf8_lossy(&output.stderr).trim_end()
                ),
            };
            assert_eq!(
                is_branch_name(name),
                git_accepts,
                "is_branch_name({shown:?}) against git check-ref-format --branch"
            );
            if git_accepts {
                accepted += 1;
            } else {
                refused += 1;
            }
        }
        assert!(
            accepted > 0 && refused > 0,
            "git accepted {accepted} and refused {refused} of the table"
        );
    }

    /// A full name reads only that ref: `refs/remotes/origin/ghost` is
    /// absent for `find_ref` and `ref_commit`, as for `git show-ref --verify`,
    /// while `refs/heads/refs/remotes/origin/ghost` (a longer name ending in
    /// it) exists and the loose file `.git/heads/refs/remotes/origin/ghost`,
    /// a path gix tries for that full name, holds a commit id. The longer
    /// name, read by its own full name, is the control that the ref is there.
    #[test]
    fn full_name_read_is_exact() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("full-name-exact").expect("create scratch");
        let work = cloned_repo(&scratch);
        let ghost = "refs/remotes/origin/ghost";
        let longer = "refs/heads/refs/remotes/origin/ghost";
        git_ok(&scratch, &work, ["update-ref", longer, "HEAD"])
            .unwrap_or_else(|err| panic!("{err}"));
        let head = show_ref(&scratch, &work, "refs/heads/main")
            .expect("git show-ref finds refs/heads/main");
        write(
            &work.join(".git/heads").join(ghost),
            format!("{head}\n").as_bytes(),
        )
        .expect("write .git/heads/refs/remotes/origin/ghost");
        assert_eq!(
            show_ref(&scratch, &work, longer).as_deref(),
            Some(head.as_str()),
            "git show-ref --verify {longer}"
        );
        assert_eq!(
            show_ref(&scratch, &work, ghost),
            None,
            "git show-ref --verify {ghost}"
        );
        let repo = Repo::discover(&work).expect("discover the clone");
        let raw = repo
            .repo
            .try_find_reference(ghost)
            .unwrap_or_else(|err| panic!("try_find_reference({ghost}): {err}"))
            .map(|found| found.name().as_bstr().to_string());
        assert!(
            raw.as_deref().is_some_and(|name| name != ghost),
            "gix's raw lookup of {ghost} gave {raw:?}, not a ref under another name, \
             so this test shows nothing about find_ref's name check"
        );

        assert_eq!(
            read(&repo, longer),
            Some(RefTarget::Object(ObjectId(head.clone()))),
            "find_ref({longer})"
        );
        assert_eq!(read(&repo, ghost), None, "find_ref({ghost})");
        let commit = repo.ref_commit(ghost.as_bytes());
        assert!(commit.is_err(), "ref_commit({ghost}) gave {commit:?}");
    }

    /// A tag never satisfies a branch's full name: neither `refs/tags/develop`
    /// for `refs/heads/develop` nor `refs/tags/refs/heads/ghost2` for
    /// `refs/heads/ghost2`, agreeing with `git show-ref --verify`. Each tag,
    /// read by its own full name, is the control that it exists.
    #[test]
    fn tag_does_not_satisfy_a_branch_name() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("tag-not-branch").expect("create scratch");
        let work = cloned_repo(&scratch);
        let ok =
            |args: &[&str]| git_ok(&scratch, &work, args).unwrap_or_else(|err| panic!("{err}"));
        ok(&["tag", "develop"]);
        ok(&["update-ref", "refs/tags/refs/heads/ghost2", "HEAD"]);
        let repo = Repo::discover(&work).expect("discover the clone");

        for (tag, branch) in [
            ("refs/tags/develop", "refs/heads/develop"),
            ("refs/tags/refs/heads/ghost2", "refs/heads/ghost2"),
        ] {
            let tagged = show_ref(&scratch, &work, tag)
                .unwrap_or_else(|| panic!("git show-ref --verify {tag} finds no tag"));
            assert_eq!(
                read(&repo, tag),
                Some(RefTarget::Object(ObjectId(tagged))),
                "find_ref({tag})"
            );
            assert_eq!(
                show_ref(&scratch, &work, branch),
                None,
                "git show-ref --verify {branch}"
            );
            assert_eq!(read(&repo, branch), None, "find_ref({branch})");
            let commit = repo.ref_commit(branch.as_bytes());
            assert!(commit.is_err(), "ref_commit({branch}) gave {commit:?}");
        }
    }

    /// `ref_commit` gives the commit `git show-ref --verify` prints for a
    /// local branch, a tracking ref and the symbolic `<remote>/HEAD`. The
    /// branches hold different commits, so a read of the wrong one shows.
    #[test]
    fn ref_commit_matches_show_ref() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("ref-commit-show-ref").expect("create scratch");
        let work = cloned_repo(&scratch);
        let ok =
            |args: &[&str]| git_ok(&scratch, &work, args).unwrap_or_else(|err| panic!("{err}"));
        ok(&["switch", "-q", "-c", "other"]);
        write(&work.join("b.txt"), b"b\n").expect("write b.txt");
        commit_all(&scratch, &work, "other", T0 + TICK).expect("commit");
        ok(&["switch", "-q", "main"]);
        let repo = Repo::discover(&work).expect("discover the clone");

        let mut seen = Vec::new();
        for full in [
            "refs/heads/main",
            "refs/heads/other",
            "refs/remotes/origin/main",
            "refs/remotes/origin/HEAD",
        ] {
            let from_git = show_ref(&scratch, &work, full)
                .unwrap_or_else(|| panic!("git show-ref --verify {full} finds no ref"));
            assert_eq!(commit_of(&repo, full), from_git, "ref_commit({full})");
            seen.push(from_git);
        }
        assert_ne!(seen[0], seen[1], "main and other hold the same commit");
    }

    #[test]
    fn ref_commit_reads_the_commit_a_describe_shaped_ref_holds() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("ref-commit-describe-shaped").expect("create scratch");
        let fixture = testing::describe_shaped_ref(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let prefix = fixture
            .other
            .get(..7)
            .expect("a hex id of at least 7 characters");
        let alias = format!("refs/heads/alias-1-g{prefix}");
        git_ok(
            &scratch,
            &fixture.repo,
            ["symbolic-ref", alias.as_str(), fixture.name.as_str()],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            show_ref(&scratch, &fixture.repo, &fixture.name).as_deref(),
            Some(fixture.one.as_str()),
            "git show-ref --verify {} prints ONE",
            fixture.name
        );
        assert_eq!(
            id(&scratch, &fixture.repo, &alias).to_string(),
            fixture.one,
            "git rev-parse --verify {alias} prints ONE"
        );
        let repo = Repo::discover(&fixture.repo).expect("discover the fixture");

        #[expect(
            clippy::disallowed_methods,
            reason = "the control that this fixture reaches gix's describe-output parsing, which reads the name as the object its hex prefixes"
        )]
        let parsed = repo
            .repo
            .rev_parse_single(BStr::new(fixture.name.as_bytes()))
            .map(|found| found.to_string());
        match parsed {
            Ok(parsed) => assert_eq!(
                parsed, fixture.other,
                "gix's rev_parse_single({}) against Y",
                fixture.name
            ),
            Err(err) => panic!("gix's rev_parse_single({}): {err}", fixture.name),
        }

        assert_eq!(
            commit_of(&repo, &fixture.name),
            fixture.one,
            "ref_commit({}) against ONE, which the ref holds",
            fixture.name
        );
        assert_eq!(
            commit_of(&repo, &alias),
            fixture.one,
            "ref_commit({alias}), symbolic to {}, against ONE",
            fixture.name
        );
    }

    #[test]
    fn ref_commit_follows_symbolic_refs_as_far_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("ref-commit-depth").expect("create scratch");
        let repo_dir = tagged_repo(&scratch);
        let ok =
            |args: &[&str]| git_ok(&scratch, &repo_dir, args).unwrap_or_else(|err| panic!("{err}"));
        ok(&["branch", "workspace/dev"]);
        for (name, target) in [
            ("refs/heads/s5", "refs/heads/workspace/dev"),
            ("refs/heads/s4", "refs/heads/s5"),
            ("refs/heads/s3", "refs/heads/s4"),
            ("refs/heads/s2", "refs/heads/s3"),
            ("refs/heads/s1", "refs/heads/s2"),
        ] {
            ok(&["symbolic-ref", name, target]);
        }
        let tip = show_ref(&scratch, &repo_dir, "refs/heads/workspace/dev")
            .expect("git show-ref finds refs/heads/workspace/dev");
        assert_eq!(
            id(&scratch, &repo_dir, "refs/heads/s2").to_string(),
            tip,
            "git rev-parse --verify refs/heads/s2, four symbolic hops from the tip"
        );
        let deeper = testing::git(
            &scratch,
            &repo_dir,
            ["rev-parse", "--verify", "refs/heads/s1"],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert_ne!(
            deeper.status.code(),
            Some(0),
            "git rev-parse --verify refs/heads/s1, five symbolic hops, printed {:?}",
            String::from_utf8_lossy(&deeper.stdout).trim_end()
        );
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");

        assert_eq!(
            commit_of(&repo, "refs/heads/s2"),
            tip,
            "ref_commit(refs/heads/s2) against git"
        );
        let commit = repo.ref_commit(b"refs/heads/s1");
        assert!(
            commit.is_err(),
            "ref_commit(refs/heads/s1), where git refuses, gave {commit:?}"
        );
    }

    /// The prefixes workspace-branch candidates are listed under (D-21),
    /// each with its trailing slash.
    const CANDIDATE_PREFIXES: [&str; 2] = ["refs/heads/workspace/", "refs/heads/personal/"];

    /// The branches `listing_repo` adds: one level under each prefix, one
    /// nested two levels down, and names that only extend a prefix's text.
    const LISTING_BRANCHES: [&str; 8] = [
        "workspace/a",
        "workspace/nested/deep",
        "workspace-x",
        "workspaceY",
        "personal/workspace",
        "personal/workspace-backup",
        "personal-z",
        "notworkspace/q",
    ];

    /// `tagged_repo` with each of `LISTING_BRANCHES` made at `HEAD`.
    fn listing_repo(scratch: &Scratch) -> PathBuf {
        let repo_dir = tagged_repo(scratch);
        for branch in LISTING_BRANCHES {
            git_ok(scratch, &repo_dir, ["branch", branch]).unwrap_or_else(|err| panic!("{err}"));
        }
        repo_dir
    }

    /// `names` as a set of `BString`, so a failed comparison prints names.
    fn name_set<I: IntoIterator<Item = Vec<u8>>>(names: I) -> BTreeSet<BString> {
        names.into_iter().map(BString::from).collect()
    }

    /// `refs_directly_under(prefix)`'s answer; fails the test on an error.
    fn under(repo: &Repo, prefix: &str) -> Vec<Vec<u8>> {
        repo.refs_directly_under(prefix.as_bytes())
            .unwrap_or_else(|err| panic!("refs_directly_under({prefix}): {err}"))
    }

    /// The union of `refs_directly_under` over `CANDIDATE_PREFIXES`.
    fn listed(repo: &Repo) -> BTreeSet<BString> {
        name_set(
            CANDIDATE_PREFIXES
                .into_iter()
                .flat_map(|prefix| under(repo, prefix)),
        )
    }

    /// The full names `git for-each-ref 'refs/heads/workspace/*'
    /// 'refs/heads/personal/*'` lists in `dir`, as raw bytes. for-each-ref
    /// has no `-z`, so each name is formatted with a NUL after it; a ref name
    /// holds neither NUL nor LF, so every line must be one name and its NUL.
    fn git_listed(scratch: &Scratch, dir: &Path) -> BTreeSet<BString> {
        let mut args = vec![
            "for-each-ref".to_owned(),
            "--format=%(refname)%00".to_owned(),
        ];
        args.extend(CANDIDATE_PREFIXES.map(|prefix| format!("{prefix}*")));
        let stdout = git_ok(scratch, dir, &args).unwrap_or_else(|err| panic!("{err}"));
        name_set(testing::records(&stdout, b'\n').into_iter().map(
            |record| match record.split_last() {
                Some((0, name)) => name.to_vec(),
                _ => panic!(
                    "git for-each-ref printed a line not ending in NUL: {:?}",
                    BStr::new(&record)
                ),
            },
        ))
    }

    /// A3: the listing is git's one-level glob. Of the fixture's branches
    /// only `workspace/a`, `personal/workspace` and
    /// `personal/workspace-backup` are directly under a prefix; the nested
    /// `workspace/nested/deep` and the names that only extend a prefix's
    /// text are not, for git as for `refs_directly_under`.
    #[test]
    fn refs_directly_under_matches_for_each_ref() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-for-each-ref").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        let from_git = git_listed(&scratch, &repo_dir);
        assert_eq!(
            from_git,
            name_set([
                b"refs/heads/personal/workspace".to_vec(),
                b"refs/heads/personal/workspace-backup".to_vec(),
                b"refs/heads/workspace/a".to_vec(),
            ]),
            "git for-each-ref's one-level globs over the fixture"
        );
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");

        assert_eq!(
            listed(&repo),
            from_git,
            "refs_directly_under against git for-each-ref"
        );
    }

    /// The control for the differential: a new branch under a prefix changes
    /// git's answer and `refs_directly_under`'s alike, so the two do not
    /// agree only because the answer is constant.
    #[test]
    fn listing_changes_with_a_new_branch() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-new-branch").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        let git_before = git_listed(&scratch, &repo_dir);
        let before = listed(&Repo::discover(&repo_dir).expect("discover the fixture"));

        git_ok(&scratch, &repo_dir, ["branch", "workspace/b"])
            .unwrap_or_else(|err| panic!("{err}"));
        let git_after = git_listed(&scratch, &repo_dir);
        let after = listed(&Repo::discover(&repo_dir).expect("discover the fixture"));

        assert_ne!(git_after, git_before, "git for-each-ref before and after");
        assert_ne!(after, before, "refs_directly_under before and after");
        assert_eq!(after, git_after, "refs_directly_under against git, after");
        assert!(
            after.contains(&BString::from("refs/heads/workspace/b")),
            "refs_directly_under after the branch was made: {after:?}"
        );
    }

    /// Packed and loose refs are listed together: after `git pack-refs
    /// --all`, which leaves `workspace/a` only in `packed-refs`, and one new
    /// loose branch, the listing still equals git's and holds both.
    #[test]
    fn listing_survives_packing() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-packed").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        let ok =
            |args: &[&str]| git_ok(&scratch, &repo_dir, args).unwrap_or_else(|err| panic!("{err}"));
        ok(&["pack-refs", "--all"]);
        let git_dir = repo_dir.join(".git");
        assert!(
            !git_dir.join("refs/heads/workspace/a").exists(),
            "git pack-refs --all left refs/heads/workspace/a loose"
        );
        ok(&["branch", "workspace/loose"]);
        assert!(
            git_dir.join("refs/heads/workspace/loose").is_file(),
            "git branch did not write refs/heads/workspace/loose loose"
        );
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");

        let names = listed(&repo);
        assert_eq!(
            names,
            git_listed(&scratch, &repo_dir),
            "refs_directly_under against git for-each-ref, packed and loose"
        );
        for full in ["refs/heads/workspace/a", "refs/heads/workspace/loose"] {
            assert!(
                names.contains(&BString::from(full)),
                "{full} is missing from {names:?}"
            );
        }
    }

    /// The names come back in byte order, whatever order they were made in:
    /// `C` sorts before `a`, as uppercase ASCII does.
    #[test]
    fn listing_is_sorted() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-sorted").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        for branch in ["workspace/b", "workspace/C"] {
            git_ok(&scratch, &repo_dir, ["branch", branch]).unwrap_or_else(|err| panic!("{err}"));
        }
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");

        let names = under(&repo, "refs/heads/workspace/");
        assert_eq!(
            names.iter().cloned().map(BString::from).collect::<Vec<_>>(),
            vec![
                BString::from("refs/heads/workspace/C"),
                BString::from("refs/heads/workspace/a"),
                BString::from("refs/heads/workspace/b"),
            ],
            "refs_directly_under(refs/heads/workspace/)"
        );
        assert!(
            names.windows(2).all(|pair| pair[0] < pair[1]),
            "not in byte order: {names:?}"
        );
    }

    /// A branch whose name holds the byte E9, not UTF-8, is listed with
    /// that byte, as git lists it.
    #[test]
    fn non_utf8_branch_is_listed_by_its_bytes() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-non-utf8").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        let branch: &[u8] = b"workspace/raw\xE9";
        git_ok(
            &scratch,
            &repo_dir,
            [OsStr::new("branch"), OsStr::from_bytes(branch)],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let full = BString::from([b"refs/heads/".as_slice(), branch].concat());
        let from_git = git_listed(&scratch, &repo_dir);
        assert!(
            from_git.contains(&full),
            "git for-each-ref does not list {full:?}: {from_git:?}"
        );
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");

        assert_eq!(
            listed(&repo),
            from_git,
            "refs_directly_under against git for-each-ref"
        );
    }

    /// A loose ref file holding garbage under the prefix fails the listing
    /// of that prefix, naming it, where git warns and lists the rest: a
    /// shorter list could leave one candidate where there are two. The
    /// other prefix, which holds no such file, still lists.
    #[test]
    fn corrupt_ref_under_the_prefix_is_an_error() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-corrupt").expect("create scratch");
        let repo_dir = listing_repo(&scratch);
        let before = git_listed(&scratch, &repo_dir);
        write(
            &repo_dir.join(".git/refs/heads/workspace/bad"),
            b"garbage\n",
        )
        .expect("write .git/refs/heads/workspace/bad");
        assert_eq!(
            git_listed(&scratch, &repo_dir),
            before,
            "git for-each-ref skips the corrupt ref and lists the rest"
        );
        let repo = Repo::discover(&repo_dir).expect("discover the fixture");

        let prefix = "refs/heads/workspace/";
        match repo.refs_directly_under(prefix.as_bytes()) {
            Err(err @ Error::Gix { .. }) => {
                let rendered = err.to_string();
                let message =
                    rendered.strip_prefix("list the refs directly under refs/heads/workspace/: ");
                assert!(
                    message.is_some_and(|message| !message.is_empty()),
                    "refs_directly_under({prefix}) rendered {rendered:?}, not the prefix and gix's message"
                );
            }
            other => panic!("refs_directly_under({prefix}): expected Error::Gix, got {other:?}"),
        }
        assert_eq!(
            name_set(under(&repo, "refs/heads/personal/")),
            name_set([
                b"refs/heads/personal/workspace".to_vec(),
                b"refs/heads/personal/workspace-backup".to_vec(),
            ]),
            "refs_directly_under(refs/heads/personal/) beside the corrupt workspace ref"
        );
    }

    /// From a linked worktree the listing is the main worktree's, and git's
    /// from that worktree: branches are shared, not per worktree.
    #[test]
    fn linked_worktree_lists_the_same_branches() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("refs-under-worktree").expect("create scratch");
        let trees = testing::linked_worktrees(&scratch).unwrap_or_else(|err| panic!("{err}"));
        for branch in ["workspace/a", "personal/b", "workspace/n/deep"] {
            git_ok(&scratch, &trees.main, ["branch", branch]).unwrap_or_else(|err| panic!("{err}"));
        }
        let main = listed(&Repo::discover(&trees.main).expect("discover the main worktree"));
        assert_eq!(
            main,
            name_set([
                b"refs/heads/personal/b".to_vec(),
                b"refs/heads/workspace/a".to_vec(),
            ]),
            "refs_directly_under from the main worktree"
        );

        let linked = listed(&Repo::discover(&trees.wt1).expect("discover the linked worktree"));
        assert_eq!(
            linked, main,
            "from the linked worktree against the main one"
        );
        assert_eq!(
            linked,
            git_listed(&scratch, &trees.wt1),
            "from the linked worktree against git for-each-ref there"
        );
    }
}
