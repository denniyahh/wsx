//! Repository resolution: the single place wsx asks gix to open a repository.

use std::path::{Path, PathBuf};

use gix::sec::trust::DefaultForLevel;

use super::Error;

/// A repository wsx resolved, with its locations canonicalised once.
pub struct Repo {
    /// The open gix repository; read only through this module's queries.
    pub(super) repo: gix::Repository,
    /// The canonical worktree root, or `None` for a bare repository.
    worktree: Option<PathBuf>,
    /// The canonical git dir (`<worktree>/.git`, or the linked worktree's own).
    git_dir: PathBuf,
    /// The canonical common dir, shared by every worktree of the repository.
    common_dir: PathBuf,
}

impl Repo {
    /// Finds the repository containing `start` the way git would, honouring
    /// `GIT_DIR`, `GIT_INDEX_FILE` and `GIT_CEILING_DIRECTORIES` from the
    /// process environment.
    pub fn discover(start: &Path) -> Result<Repo, Error> {
        let repo = Repo::open_gix(start)?;
        refuse_ref_storage(&repo)?;
        Repo::from_gix(repo)
    }

    /// The only gix open or discover call in wsx. Trust is mapped without a
    /// bail, so a repository of reduced trust still opens here; the refusal is
    /// wsx's own check after discovery.
    #[expect(
        clippy::disallowed_methods,
        reason = "the single sanctioned way wsx opens a repository; see clippy.toml"
    )]
    fn open_gix(start: &Path) -> Result<gix::Repository, Error> {
        let trust_map = gix::sec::trust::Mapping {
            full: gix::open::Options::default_for_level(gix::sec::Trust::Full),
            reduced: gix::open::Options::default_for_level(gix::sec::Trust::Reduced),
        };
        let repo = gix::ThreadSafeRepository::discover_with_environment_overrides_opts(
            start,
            gix::discover::upwards::Options {
                match_ceiling_dir_or_error: false,
                ..Default::default()
            },
            trust_map,
        )
        .map_err(|err| Error::NotARepo {
            start: start.to_path_buf(),
            reason: err.to_string(),
        })?;
        Ok(repo.to_thread_local())
    }

    /// Wraps an open repository, canonicalising its worktree, git dir and
    /// common dir. gix reports the common dir as `<git-dir>/../..`, and on
    /// some hosts `/home` is a symlink, so only canonical paths compare.
    fn from_gix(repo: gix::Repository) -> Result<Repo, Error> {
        let worktree = repo.workdir().map(canonical).transpose()?;
        let git_dir = canonical(repo.git_dir())?;
        let common_dir = canonical(repo.common_dir())?;
        Ok(Repo {
            repo,
            worktree,
            git_dir,
            common_dir,
        })
    }

    /// The canonical worktree root, or `None` for a bare repository.
    pub fn worktree(&self) -> Option<&Path> {
        self.worktree.as_deref()
    }

    /// The canonical git dir.
    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    /// The canonical common dir.
    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }

    /// The object a revspec (`main`, `feat~3`, a hex id) names.
    pub(super) fn resolve(&self, rev: &[u8]) -> Result<gix::ObjectId, Error> {
        self.repo
            .rev_parse_single(rev)
            .map(|id| id.detach())
            .map_err(|err| Error::Gix {
                what: format!("resolve {}", String::from_utf8_lossy(rev)),
                message: err.to_string(),
            })
    }
}

/// The configured `extensions.refStorage`, or `None` when it is not set.
fn ref_storage(repo: &gix::Repository) -> Option<gix::bstr::BString> {
    repo.config_snapshot().string("extensions.refStorage")
}

/// Refuses a repository whose refs are not in the files backend, read from
/// `extensions.refStorage` before any ref is touched: gix 0.90 opens a
/// reftable repository and fails only when it decodes a ref. Only the exact
/// bytes `files` pass, as in git, which rejects any other spelling outright.
fn refuse_ref_storage(repo: &gix::Repository) -> Result<(), Error> {
    match ref_storage(repo) {
        None => Ok(()),
        Some(value) if value == "files" => Ok(()),
        Some(value) => Err(Error::Reftable {
            path: refusal_path(repo)?,
            format: String::from_utf8_lossy(&value).into_owned(),
        }),
    }
}

/// The path a refusal names: the canonical worktree, or the canonical git
/// dir of a bare repository.
fn refusal_path(repo: &gix::Repository) -> Result<PathBuf, Error> {
    canonical(repo.workdir().unwrap_or_else(|| repo.git_dir()))
}

/// `path` with every symlink and `..` resolved.
fn canonical(path: &Path) -> Result<PathBuf, Error> {
    std::fs::canonicalize(path).map_err(|source| Error::Io {
        what: format!("canonicalize {}", path.display()),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use gix::sec::trust::DefaultForLevel;

    use super::super::Error;
    use super::super::spawn::Git;
    use super::super::testing::{Scratch, T0, TICK, commit_all, git, git_ok, write};
    use super::Repo;

    /// The trimmed stdout of `git <args>` run in `dir`.
    fn git_line(scratch: &Scratch, dir: &Path, args: &[&str]) -> String {
        let stdout = git_ok(scratch, dir, args).unwrap_or_else(|err| panic!("{err}"));
        String::from_utf8(stdout)
            .expect("git printed UTF-8")
            .trim_end()
            .to_owned()
    }

    /// Builds `<root>/files` and `<root>/table` with the same steps and
    /// dates (`a.txt` at `T0`, `sub/b.txt` at `T0 + TICK`), checks both
    /// have the same `HEAD`, then migrates `table` to reftable with git
    /// itself and checks git reports the reftable format there.
    fn twins(scratch: &Scratch) -> (PathBuf, PathBuf) {
        let build = |name: &str| {
            let dir = scratch.root.join(name);
            std::fs::create_dir_all(&dir).expect("create twin dir");
            git_ok(scratch, &dir, ["init", "-q", "-b", "main"]).expect("git init");
            write(&dir.join("a.txt"), b"a\n").expect("write a.txt");
            commit_all(scratch, &dir, "a", T0).expect("commit a");
            write(&dir.join("sub/b.txt"), b"b\n").expect("write sub/b.txt");
            commit_all(scratch, &dir, "b", T0 + TICK).expect("commit b");
            dir
        };
        let files = build("files");
        let table = build("table");
        assert_eq!(
            git_line(scratch, &files, &["rev-parse", "HEAD"]),
            git_line(scratch, &table, &["rev-parse", "HEAD"]),
            "the twins must hold the same history"
        );
        git_ok(
            scratch,
            &table,
            ["refs", "migrate", "--ref-format=reftable"],
        )
        .expect("git refs migrate --ref-format=reftable");
        assert_eq!(
            git_line(scratch, &table, &["rev-parse", "--show-ref-format"]),
            "reftable",
            "the migrated twin must use reftable"
        );
        assert_eq!(
            git_line(scratch, &files, &["rev-parse", "--show-ref-format"]),
            "files",
            "the control twin must use the files backend"
        );
        (files, table)
    }

    /// A one-commit repository at `<root>/<name>` with
    /// `core.repositoryformatversion = 1` and `extensions.refStorage` set to
    /// `value` by `git config`.
    fn ref_storage_repo(scratch: &Scratch, name: &str, value: &str) -> PathBuf {
        let dir = scratch.root.join(name);
        std::fs::create_dir_all(&dir).expect("create repo dir");
        git_ok(scratch, &dir, ["init", "-q", "-b", "main"]).expect("git init");
        write(&dir.join("a.txt"), b"a\n").expect("write a.txt");
        commit_all(scratch, &dir, "a", T0).expect("commit a");
        git_ok(
            scratch,
            &dir,
            ["config", "core.repositoryformatversion", "1"],
        )
        .expect("set core.repositoryformatversion");
        git_ok(scratch, &dir, ["config", "extensions.refStorage", value])
            .expect("set extensions.refStorage");
        dir
    }

    /// The `format` and rendered message of a `Reftable` refusal, panicking
    /// on an open or any other error.
    fn refused_format(result: Result<Repo, Error>, what: &str) -> (String, String) {
        let err = match result {
            Ok(repo) => panic!(
                "{what}: expected a Reftable refusal, but it opened at {}",
                repo.git_dir().display()
            ),
            Err(err) => err,
        };
        let message = err.to_string();
        match err {
            Error::Reftable { format, .. } => (format, message),
            _ => panic!("{what}: expected a Reftable refusal, got {message}"),
        }
    }

    /// A repository git migrated to reftable is refused from a subdirectory,
    /// naming reftable and saying wsx cannot read it.
    #[test]
    fn reftable_repo_is_refused() {
        let scratch = Scratch::new("reftable").expect("scratch");
        let (_files, table) = twins(&scratch);
        let (format, message) = refused_format(Repo::discover(&table.join("sub")), "reftable twin");
        assert_eq!(format, "reftable");
        assert!(message.contains("reftable"), "message: {message}");
        assert!(message.contains("cannot read"), "message: {message}");
    }

    /// Control: the same history in the files backend opens from the same
    /// subdirectory.
    #[test]
    fn files_backend_twin_opens() {
        let scratch = Scratch::new("files-twin").expect("scratch");
        let (files, _table) = twins(&scratch);
        let repo = Repo::discover(&files.join("sub")).expect("the files twin opens");
        assert_eq!(repo.worktree(), Some(files.as_path()));
    }

    /// A reftable repository with no commit is refused the same way: the
    /// check reads configuration, not refs.
    #[test]
    fn empty_reftable_repo_is_refused() {
        let scratch = Scratch::new("empty-reftable").expect("scratch");
        let empty = scratch.root.join("empty");
        let empty_arg = empty.to_str().expect("UTF-8 scratch path");
        git_ok(
            &scratch,
            &scratch.root,
            ["init", "-q", "--ref-format=reftable", empty_arg],
        )
        .expect("git init --ref-format=reftable");
        let (format, _message) = refused_format(Repo::discover(&empty), "empty reftable");
        assert_eq!(format, "reftable");
    }

    /// `files` set explicitly, as git writes it, opens.
    #[test]
    fn explicit_files_ref_storage_opens() {
        let scratch = Scratch::new("explicit-files").expect("scratch");
        let dir = ref_storage_repo(&scratch, "repo", "files");
        git_ok(&scratch, &dir, ["rev-parse", "HEAD"]).expect("git still reads the repository");
        let repo = Repo::discover(&dir).expect("an explicit files backend opens");
        assert_eq!(repo.worktree(), Some(dir.as_path()));
    }

    /// `Files` is not `files`: wsx refuses it, as git does.
    #[test]
    fn miscased_ref_storage_is_refused_as_git_refuses_it() {
        let scratch = Scratch::new("miscased").expect("scratch");
        let dir = ref_storage_repo(&scratch, "repo", "Files");
        let (format, _message) = refused_format(Repo::discover(&dir), "miscased Files");
        assert_eq!(format, "Files");

        let output = git(&scratch, &dir, ["rev-parse", "--git-dir"]).expect("spawn git");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(128), "git stderr: {stderr}");
        assert!(
            stderr.contains("invalid value for 'extensions.refstorage'"),
            "git stderr: {stderr}"
        );
    }

    /// The trust mapping `open_gix` passes: each level's default options.
    fn trust_map() -> gix::sec::trust::Mapping<gix::open::Options> {
        gix::sec::trust::Mapping {
            full: gix::open::Options::default_for_level(gix::sec::Trust::Full),
            reduced: gix::open::Options::default_for_level(gix::sec::Trust::Reduced),
        }
    }

    /// Default upwards-discovery options.
    fn upwards() -> gix::discover::upwards::Options<'static> {
        gix::discover::upwards::Options::default()
    }

    /// The repository at `dir`, opened past `refuse_ref_storage`, as only a
    /// test can.
    fn open_unchecked(dir: &Path) -> Repo {
        let repo = Repo::open_gix(dir).expect("gix opens the repository");
        Repo::from_gix(repo).expect("wrap the open repository")
    }

    /// With the configuration check bypassed, a reftable repository opens,
    /// and resolving `HEAD` is the reftable refusal, not a generic read
    /// error.
    #[test]
    fn reftable_ref_decode_maps_to_the_refusal() {
        let scratch = Scratch::new("reftable-decode").expect("scratch");
        let (_files, table) = twins(&scratch);
        let repo = open_unchecked(&table);
        match repo.resolve(b"HEAD") {
            Err(Error::Reftable { format, .. }) => assert_eq!(format, "reftable"),
            Err(other) => panic!("resolve HEAD: expected a Reftable refusal, got {other}"),
            Ok(id) => panic!("resolve HEAD on reftable: expected a refusal, got {id}"),
        }
    }

    /// Control: the files twin resolves `HEAD` through the same path, to
    /// the id git prints.
    #[test]
    fn files_repo_resolves_head_through_the_same_path() {
        let scratch = Scratch::new("files-decode").expect("scratch");
        let (files, _table) = twins(&scratch);
        let repo = open_unchecked(&files);
        let id = repo
            .resolve(b"HEAD")
            .unwrap_or_else(|err| panic!("resolve HEAD on the files twin: {err}"));
        assert_eq!(
            id.to_string(),
            git_line(&scratch, &files, &["rev-parse", "HEAD"])
        );
    }

    /// A ref that does not exist is an ordinary read error, never the
    /// reftable refusal.
    #[test]
    fn unknown_revision_is_not_reported_as_reftable() {
        let scratch = Scratch::new("unknown-rev").expect("scratch");
        let (files, _table) = twins(&scratch);
        let repo = open_unchecked(&files);
        match repo.resolve(b"refs/heads/no-such-branch") {
            Err(Error::Gix { .. }) => {}
            Err(other) => panic!("missing branch: expected Error::Gix, got {other}"),
            Ok(id) => panic!("missing branch resolved to {id}"),
        }
    }

    /// Calls every gix open/discover entry point `clippy.toml` bans, each
    /// under its own expectation, so the ban list cannot go stale silently:
    /// a renamed entry point stops compiling, and a misspelled ban path
    /// leaves that call's expectation unfulfilled, which `-D warnings`
    /// turns into an error. The start path does not exist, so every
    /// discover variant fails before climbing towards the wsx checkout.
    #[test]
    fn banned_gix_entry_points_refuse_a_missing_path() {
        let scratch = Scratch::new("ban-pin").expect("scratch");
        let missing = scratch.root.join("does-not-exist");
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let open = gix::open(&missing).is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let open_opts = gix::open_opts(&missing, gix::open::Options::default()).is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let discover = gix::discover(&missing).is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let discover_opts =
            gix::discover_opts(&missing, upwards(), gix::open::Options::default()).is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let discover_env = gix::discover_with_environment_overrides(&missing).is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let open_env = gix::open_with_environment_overrides(&missing).is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let ts_open = gix::ThreadSafeRepository::open(&missing).is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let ts_open_opts =
            gix::ThreadSafeRepository::open_opts(&missing, gix::open::Options::default()).is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let ts_open_env =
            gix::ThreadSafeRepository::open_with_environment_overrides(&missing, trust_map())
                .is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let ts_discover = gix::ThreadSafeRepository::discover(&missing).is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let ts_discover_opts =
            gix::ThreadSafeRepository::discover_opts(&missing, upwards(), trust_map()).is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let ts_discover_env =
            gix::ThreadSafeRepository::discover_with_environment_overrides(&missing).is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let ts_discover_env_opts =
            gix::ThreadSafeRepository::discover_with_environment_overrides_opts(
                &missing,
                upwards(),
                trust_map(),
            )
            .is_err();
        #[expect(
            clippy::disallowed_methods,
            reason = "pins clippy.toml's gix ban: a renamed entry point stops compiling and a misspelled ban path leaves this expectation unfulfilled"
        )]
        let options_open = gix::open::Options::default().open(&missing).is_err();
        for (entry_point, refused) in [
            ("gix::open", open),
            ("gix::open_opts", open_opts),
            ("gix::discover", discover),
            ("gix::discover_opts", discover_opts),
            ("gix::discover_with_environment_overrides", discover_env),
            ("gix::open_with_environment_overrides", open_env),
            ("gix::ThreadSafeRepository::open", ts_open),
            ("gix::ThreadSafeRepository::open_opts", ts_open_opts),
            (
                "gix::ThreadSafeRepository::open_with_environment_overrides",
                ts_open_env,
            ),
            ("gix::ThreadSafeRepository::discover", ts_discover),
            ("gix::ThreadSafeRepository::discover_opts", ts_discover_opts),
            (
                "gix::ThreadSafeRepository::discover_with_environment_overrides",
                ts_discover_env,
            ),
            (
                "gix::ThreadSafeRepository::discover_with_environment_overrides_opts",
                ts_discover_env_opts,
            ),
            ("gix::open::Options::open", options_open),
        ] {
            assert!(
                refused,
                "{entry_point} opened the missing path {}",
                missing.display()
            );
        }
    }

    /// A repository made with `--object-format=sha256` opens through the
    /// single entry point, and its ids come back at SHA-256 length, equal to
    /// git's. Read as SHA-1, the id would be 40 characters, or the open
    /// would fail.
    #[test]
    fn sha256_repo_opens() {
        let scratch = Scratch::new("sha256").expect("scratch");
        let repo_dir = scratch.root.join("repo");
        let repo_arg = repo_dir.to_str().expect("UTF-8 scratch path");
        git_ok(
            &scratch,
            &scratch.root,
            ["init", "-q", "--object-format=sha256", repo_arg],
        )
        .expect("git init --object-format=sha256");
        write(&repo_dir.join("a.txt"), b"a\n").expect("write a.txt");
        commit_all(&scratch, &repo_dir, "first", T0).expect("commit");

        let repo = Repo::discover(&repo_dir).expect("discover a SHA-256 repository");
        let base = repo
            .merge_base(b"HEAD", b"HEAD")
            .expect("merge-base HEAD HEAD")
            .expect("HEAD is its own merge-base")
            .to_string();
        let head = Git::new(scratch.env.clone())
            .run(&repo_dir, ["rev-parse", "HEAD"])
            .expect("git rev-parse HEAD");
        let head = String::from_utf8(head).expect("git printed UTF-8");

        assert_eq!(base.len(), 64, "merge-base id {base} is not SHA-256 length");
        assert_eq!(base, head.trim_end(), "gix and git disagree on HEAD");
    }
}
