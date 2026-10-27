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
    /// The index this repository uses: inside a hook, the hook's
    /// `GIT_INDEX_FILE`. Its parent directory is canonical; the file itself
    /// may not exist (a fresh repository, or a lock file a commit removes).
    index_path: PathBuf,
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

    /// Wraps an open repository, canonicalising its worktree, git dir, common
    /// dir and the index's directory. gix reports the common dir as `<git-dir>/../..`, and on
    /// some hosts `/home` is a symlink, so only canonical paths compare.
    fn from_gix(repo: gix::Repository) -> Result<Repo, Error> {
        let worktree = repo.workdir().map(canonical).transpose()?;
        let git_dir = canonical(repo.git_dir())?;
        let common_dir = canonical(repo.common_dir())?;
        let index_path = canonical_parent(&repo.index_path())?;
        Ok(Repo {
            repo,
            worktree,
            git_dir,
            common_dir,
            index_path,
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

    /// The index file this repository reads and writes, which a hook's
    /// `GIT_INDEX_FILE` overrides; the parent directory is canonical.
    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    /// The object a revspec (`main`, `feat~3`, a hex id) names. A ref gix
    /// cannot decode because of its storage backend is the reftable refusal,
    /// never a generic read error.
    pub(super) fn resolve(&self, rev: &[u8]) -> Result<gix::ObjectId, Error> {
        match self.repo.rev_parse_single(rev) {
            Ok(id) => Ok(id.detach()),
            Err(err) if err.is_unsupported() => Err(self.unsupported_backend()),
            Err(err) => Err(Error::Gix {
                what: format!("resolve {}", String::from_utf8_lossy(rev)),
                message: err.to_string(),
            }),
        }
    }

    /// The backstop behind `refuse_ref_storage`: the refusal for a ref whose
    /// storage backend gix reports as unsupported, naming the configured
    /// `extensions.refStorage`, or `unsupported` when none is set.
    fn unsupported_backend(&self) -> Error {
        Error::Reftable {
            path: self
                .worktree
                .clone()
                .unwrap_or_else(|| self.git_dir.clone()),
            format: ref_storage(&self.repo).map_or_else(
                || "unsupported".to_owned(),
                |value| String::from_utf8_lossy(&value).into_owned(),
            ),
        }
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

/// `path` with its parent directory canonicalised and its file name kept,
/// for a file that may not exist: gix reports the index absolute (a relative
/// `GIT_INDEX_FILE` is anchored at the current directory when opening), and
/// inside a hook it is a lock file the commit removes once it ends.
fn canonical_parent(path: &Path) -> Result<PathBuf, Error> {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => Ok(canonical(parent)?.join(name)),
        _ => Err(Error::Io {
            what: format!("split {} into directory and file name", path.display()),
            source: std::io::Error::from(std::io::ErrorKind::InvalidInput),
        }),
    }
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
    use std::process::Output;

    use gix::sec::trust::DefaultForLevel;

    use super::super::Error;
    use super::super::spawn::Git;
    use super::super::testing::{self, Scratch, T0, TICK, commit_all, git, git_ok, write};
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

    /// The full libtest path of a child half in this module.
    fn child_path(name: &str) -> String {
        format!("git::repo::tests::{name}")
    }

    /// The value of `key` from a hook child's output.
    fn fact(output: &Output, key: &str) -> String {
        testing::marker(output, key).unwrap_or_else(|err| panic!("{err}"))
    }

    /// Prints what `Repo::discover` resolves from the working directory, and
    /// the raw `GIT_DIR` and `GIT_INDEX_FILE` the child was started with.
    fn print_resolution() {
        let cwd = std::env::current_dir().expect("read the working directory");
        let repo = Repo::discover(&cwd).expect("discover the hook's repository");
        let worktree = repo
            .worktree()
            .expect("the hooked repository has a worktree");
        testing::child_print("worktree", worktree.display());
        testing::child_print("git_dir", repo.git_dir().display());
        testing::child_print("common_dir", repo.common_dir().display());
        testing::child_print("index", repo.index_path().display());
        testing::child_print("env_GIT_DIR", testing::env_or_unset("GIT_DIR"));
        testing::child_print(
            "env_GIT_INDEX_FILE",
            testing::env_or_unset("GIT_INDEX_FILE"),
        );
    }

    /// Inside the pre-commit hook of a partial commit in linked worktree
    /// wt1, `Repo::discover` resolves wt1, its own git dir, the shared
    /// common dir, and the temporary `next-index-*` index git exported as
    /// the hook's `GIT_INDEX_FILE`, not the worktree's `index`.
    #[test]
    fn hook_resolves_linked_worktree() {
        let scratch = Scratch::new("hook-linked").expect("scratch");
        let (worktrees, output) =
            testing::linked_hook_commit(&scratch, &child_path("linked_hook_resolution_child"))
                .unwrap_or_else(|err| panic!("{err}"));
        let git_dir = worktrees.main.join(".git/worktrees/wt1");

        assert_eq!(
            fact(&output, "worktree"),
            worktrees.wt1.display().to_string()
        );
        assert_eq!(fact(&output, "git_dir"), git_dir.display().to_string());
        assert_eq!(
            fact(&output, "common_dir"),
            worktrees.main.join(".git").display().to_string()
        );
        let hook_index = PathBuf::from(fact(&output, "env_GIT_INDEX_FILE"));
        let hook_index = testing::canonical_parent(&worktrees.wt1, &hook_index)
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(hook_index.parent(), Some(git_dir.as_path()));
        let name = hook_index.file_name().expect("the index has a file name");
        assert!(
            name.to_string_lossy().starts_with("next-index-"),
            "a partial commit's hook index is a next-index-* file, got {}",
            hook_index.display()
        );
        assert_eq!(
            fact(&output, "index"),
            hook_index.display().to_string(),
            "wsx did not resolve the hook's index"
        );
    }

    #[test]
    #[ignore = "child half of hook_resolves_linked_worktree; runs only when re-executed"]
    fn linked_hook_resolution_child() {
        print_resolution();
    }

    /// Inside the main worktree's pre-commit hook for a plain commit of
    /// staged changes, git leaves `GIT_DIR` unset and exports the relative
    /// `.git/index`; the child prints both raw values, so the test fails if
    /// git exports something else. `Repo::discover` still resolves the main
    /// worktree, `<main>/.git` and `<main>/.git/index`.
    #[test]
    fn hook_resolves_main_worktree_with_relative_index() {
        let scratch = Scratch::new("hook-main").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).unwrap_or_else(|err| panic!("{err}"));
        testing::install_hook(
            &worktrees.main.join(".git/hooks"),
            "pre-commit",
            &child_path("main_hook_resolution_child"),
        )
        .unwrap_or_else(|err| panic!("{err}"));
        write(&worktrees.main.join("a.txt"), b"a in main\n").expect("write a.txt");
        git_ok(&scratch, &worktrees.main, ["add", "a.txt"]).expect("git add a.txt");
        let output =
            testing::git_in_env(&scratch.env, &worktrees.main, ["commit", "-q", "-m", "y"])
                .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run("the commit in main", &output)
        );
        let git_dir = worktrees.main.join(".git");

        assert_eq!(fact(&output, "env_GIT_DIR"), testing::UNSET);
        assert_eq!(fact(&output, "env_GIT_INDEX_FILE"), ".git/index");
        assert_eq!(
            fact(&output, "worktree"),
            worktrees.main.display().to_string()
        );
        assert_eq!(fact(&output, "git_dir"), git_dir.display().to_string());
        assert_eq!(fact(&output, "common_dir"), git_dir.display().to_string());
        assert_eq!(
            fact(&output, "index"),
            git_dir.join("index").display().to_string()
        );
    }

    #[test]
    #[ignore = "child half of hook_resolves_main_worktree_with_relative_index; runs only when re-executed"]
    fn main_hook_resolution_child() {
        print_resolution();
    }
}
