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
    /// process environment. A relative repository-path variable, or a
    /// `GIT_DIR` that leaves git to take the current directory as the
    /// worktree, is refused below the worktree root (see
    /// `refuse_relative_repo_env` and `refuse_implicit_worktree`).
    pub fn discover(start: &Path) -> Result<Repo, Error> {
        let repo = Repo::open_gix(start)?;
        refuse_at_ceiling(start, &repo)?;
        refuse_untrusted(&repo)?;
        refuse_ref_storage(&repo)?;
        refuse_relative_repo_env(&repo)?;
        refuse_implicit_worktree(&repo)?;
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

    /// Every other worktree of this repository, the main one included, each
    /// with its own git dir and index, sorted by canonical worktree path and
    /// then git dir. Siblings are opened only through gix's main-repo and
    /// worktree-list calls, which switch the repository environment overrides
    /// off: inside a hook, opening a sibling by path would pair it with the
    /// hook's `GIT_DIR` and `GIT_INDEX_FILE`.
    ///
    /// A gix failure to list or open one is `Error::Gix`; a linked worktree
    /// whose directory is gone (`git worktree list` shows it as prunable)
    /// fails the whole list closed as `Error::StaleWorktree` naming it. A
    /// sibling git would refuse as owned by another user is
    /// `Error::ForeignOwner` naming it (see `refuse_untrusted_sibling`).
    pub fn siblings(&self) -> Result<Vec<Repo>, Error> {
        let mut opened = vec![self.repo.main_repo().map_err(|err| self.list_failed(err))?];
        for proxy in self.repo.worktrees().map_err(|err| self.list_failed(err))? {
            opened.push(
                proxy
                    .into_repo_with_possibly_inaccessible_worktree()
                    .map_err(|err| self.list_failed(err))?,
            );
        }
        let mut siblings = Vec::new();
        for repo in opened {
            refuse_missing_worktree(&repo)?;
            let sibling = Repo::from_gix(repo)?;
            let is_self = match &self.worktree {
                Some(own) => sibling.worktree.as_ref() == Some(own),
                None => sibling.git_dir == self.git_dir,
            };
            if !is_self {
                refuse_untrusted_sibling(&sibling.repo)?;
                siblings.push(sibling);
            }
        }
        siblings.sort_by(|a, b| {
            a.worktree
                .cmp(&b.worktree)
                .then_with(|| a.git_dir.cmp(&b.git_dir))
        });
        Ok(siblings)
    }

    /// The error for a gix failure to list or open this repository's
    /// worktrees.
    fn list_failed(&self, err: impl std::fmt::Display) -> Error {
        Error::Gix {
            what: format!("list worktrees of {}", self.git_dir.display()),
            message: err.to_string(),
        }
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

    /// The commit a revspec names, peeling annotated tags as git's
    /// commit-taking commands do (`rev-list`, `merge-base`). Resolution
    /// errors are `resolve`'s; an object that does not peel to a commit is
    /// `Error::Gix` naming the revision.
    pub(super) fn resolve_commit(&self, rev: &[u8]) -> Result<gix::ObjectId, Error> {
        let id = self.resolve(rev)?;
        self.repo
            .find_object(id)
            .map_err(|err| err.to_string())
            .and_then(|object| object.peel_to_commit().map_err(|err| err.to_string()))
            .map(|commit| commit.id)
            .map_err(|message| Error::Gix {
                what: format!("peel {} to a commit", String::from_utf8_lossy(rev)),
                message,
            })
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

/// The `extensions.refStorage` of the repository's own config (gix's
/// `Source::Local`, the common dir's `config`), or `None` when it is not set
/// there. git reads repository extensions from that file alone: measured with
/// git 2.56.0, a `reftable` value in global config, in `config.worktree` or
/// through `GIT_CONFIG_COUNT` leaves a files repository readable.
fn ref_storage(repo: &gix::Repository) -> Option<gix::bstr::BString> {
    repo.config_snapshot()
        .plumbing()
        .string_filter("extensions.refStorage", |meta| {
            meta.source == gix::config::Source::Local
        })
}

/// Refuses, as not a repository, a repository git would not have found
/// because `GIT_CEILING_DIRECTORIES` stops its discovery first. git never
/// searches a ceiling directory it climbs to; gix does, so it finds `<c>/.git`
/// from below a ceiling `c`. Measured with git 2.56.0, start under `r1`:
///
/// | start | ceiling | git | gix |
/// |---|---|---|---|
/// | `r1/inner` | `r1` | not found | finds `r1/.git` |
/// | `r1/inner/deeper` | `r1` | not found | finds `r1/.git` |
/// | `r1/inner` | `r1/inner` | finds `r1/.git` | finds `r1/.git` (`match_ceiling_dir_or_error = false`) |
/// | `r1/inner/deeper` | `r1/inner` | not found | not found |
///
/// So the refusal applies when a ceiling entry is the directory discovery
/// stopped in while the start is not that directory: discovery climbed into
/// the ceiling. That directory is the one holding `.git`, or the git dir
/// itself when the start is inside it (see `discovery_dir`), never the
/// worktree: `GIT_WORK_TREE` or `core.worktree` can move the worktree
/// elsewhere, and git still compares the ceiling with the directory holding
/// `.git`. The same divergence hides a bare repository from inside it (start
/// `r.git/objects`, ceiling `r.git`: git finds nothing, gix finds `r.git`),
/// while from inside a non-bare `r1/.git` with ceiling `r1`, git recognises
/// the git dir below the ceiling and opens it.
/// A start equal to the ceiling is searched by both. Nothing is checked when
/// `GIT_DIR` is set (git does no discovery then). Entries are canonicalised;
/// empty, relative and non-canonicalisable entries are skipped, as git
/// ignores them. git's rule that an empty entry turns off symlink resolution
/// for later entries is not modelled. The variable is only read here; gix
/// applied it already.
fn refuse_at_ceiling(start: &Path, repo: &gix::Repository) -> Result<(), Error> {
    if std::env::var_os("GIT_DIR").is_some() {
        return Ok(());
    }
    let Some(ceilings) = std::env::var_os("GIT_CEILING_DIRECTORIES") else {
        return Ok(());
    };
    let start_dir = canonical(start)?;
    let git_dir = canonical(repo.git_dir())?;
    let Some(found) = discovery_dir(&start_dir, &git_dir) else {
        return Err(Error::NotARepo {
            start: start.to_path_buf(),
            reason: format!(
                "no directory between it and the root leads to the git dir {} gix found, \
                 so wsx cannot apply GIT_CEILING_DIRECTORIES as git does",
                git_dir.display()
            ),
        });
    };
    if start_dir == found {
        return Ok(());
    }
    let hidden = std::env::split_paths(&ceilings)
        .filter(|entry| entry.is_absolute())
        .filter_map(|entry| std::fs::canonicalize(entry).ok())
        .any(|entry| entry == found);
    if hidden {
        Err(Error::NotARepo {
            start: start.to_path_buf(),
            reason: format!(
                "GIT_CEILING_DIRECTORIES stops discovery before it reaches {}, as git's does",
                found.display()
            ),
        })
    } else {
        Ok(())
    }
}

/// The directory git's upward discovery from `start_dir` stopped in to find
/// the canonical `git_dir`: `git_dir` itself when the start is inside it,
/// else the nearest ancestor of the start (the start included) whose `.git`
/// is that git dir, either a directory or a `gitdir:` file pointing at it.
/// `None` when no ancestor leads there. Both paths are canonical.
fn discovery_dir(start_dir: &Path, git_dir: &Path) -> Option<PathBuf> {
    if start_dir.starts_with(git_dir) {
        return Some(git_dir.to_path_buf());
    }
    let leads_to_git_dir = |dot_git: &Path| {
        let target = if dot_git.is_file() {
            gix::discover::path::from_gitdir_file(dot_git).ok()?
        } else {
            dot_git.to_path_buf()
        };
        Some(std::fs::canonicalize(target).ok()? == git_dir)
    };
    start_dir
        .ancestors()
        .find(|dir| leads_to_git_dir(&dir.join(".git")) == Some(true))
        .map(Path::to_path_buf)
}

/// Refuses a repository git would refuse as owned by another user, before
/// any of its configuration is acted on: unless gix's `git_dir_trust()` is
/// `Trust::Full`, this is `Error::ForeignOwner`, naming the canonical
/// worktree (the git dir of a bare repository) with the global-config fix
/// git prints. gix grants full trust to a repository owned by the current
/// user, or one a `safe.directory` entry in global or system config lists.
/// The typed trust is checked, not gix's bail error, which is an untyped
/// message.
///
/// Two divergences from git are accepted. `safe.directory` given through
/// `-c`, `GIT_CONFIG_PARAMETERS` or `GIT_CONFIG_COUNT` is ignored by gix, so
/// wsx refuses a repository git would open there (the stricter direction).
/// gix honours `SUDO_UID` as the owner for any effective uid, where git does
/// so only as root: a non-root process with `SUDO_UID` set trusts that
/// uid's repositories.
fn refuse_untrusted(repo: &gix::Repository) -> Result<(), Error> {
    if repo.git_dir_trust() == gix::sec::Trust::Full {
        Ok(())
    } else {
        Err(Error::ForeignOwner {
            path: refusal_path(repo)?,
        })
    }
}

/// Refuses a sibling whose worktree directory no longer exists, which gix
/// opens anyway, as `Error::StaleWorktree` naming the worktree and its git
/// dir. Any other failure to read the directory is left to
/// canonicalisation.
fn refuse_missing_worktree(repo: &gix::Repository) -> Result<(), Error> {
    let Some(workdir) = repo.workdir() else {
        return Ok(());
    };
    match std::fs::symlink_metadata(workdir) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(Error::StaleWorktree {
            worktree: workdir.to_path_buf(),
            git_dir: canonical(repo.git_dir())?,
        }),
        _ => Ok(()),
    }
}

/// Refuses a sibling worktree git would refuse as owned by another user.
/// gix opens every sibling with the discovering repository's trust already
/// set to full, so it re-checks only the sibling's worktree: one owned by
/// another user and not listed in `safe.directory` comes back with reduced
/// trust and its config dropped, refused here as `discover` refuses. A bare
/// sibling (the main repository of linked worktrees) has no worktree, so
/// gix checks nothing for it; its git dir must then be owned by the current
/// user, as gix's `Trust::from_path_ownership` decides.
///
/// One divergence from git is accepted: a foreign bare sibling listed in
/// `safe.directory` is still refused, because gix applies `safe.directory`
/// to a sibling only through its worktree. The stricter direction.
fn refuse_untrusted_sibling(repo: &gix::Repository) -> Result<(), Error> {
    refuse_untrusted(repo)?;
    if repo.workdir().is_some() {
        return Ok(());
    }
    let ownership =
        gix::sec::Trust::from_path_ownership(repo.git_dir()).map_err(|source| Error::Io {
            what: format!("read the owner of {}", repo.git_dir().display()),
            source,
        })?;
    if ownership == gix::sec::Trust::Full {
        Ok(())
    } else {
        Err(Error::ForeignOwner {
            path: refusal_path(repo)?,
        })
    }
}

/// The repository-path variables whose relative values gix and git resolve
/// differently below the worktree root.
const REPO_PATH_VARS: [&str; 4] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
];

/// Refuses a relative repository-path variable when wsx runs below the
/// worktree root. gix resolves such a value against the current directory,
/// and git does not: measured with git 2.56.0 from `main/sub`, a relative
/// `GIT_INDEX_FILE=alt.idx` is `main/alt.idx` to git and `main/sub/alt.idx`
/// to gix, and with a relative `GIT_DIR=../.git` git takes `main/sub` as
/// the worktree, where gix takes `main`; a spawn-helper child, which runs at
/// the worktree root, then finds no repository at `../.git`. At the
/// worktree root the two agree, and git runs hooks there, so a hook's
/// relative `GIT_INDEX_FILE` still applies. Absolute values, and a
/// repository without a worktree, are not checked. The current directory
/// is the one gix resolved the values against.
fn refuse_relative_repo_env(repo: &gix::Repository) -> Result<(), Error> {
    let Some(workdir) = repo.workdir() else {
        return Ok(());
    };
    let relative = REPO_PATH_VARS.into_iter().find_map(|name| {
        let value = PathBuf::from(std::env::var_os(name)?);
        (!value.as_os_str().is_empty() && value.is_relative()).then_some((name, value))
    });
    let Some((name, value)) = relative else {
        return Ok(());
    };
    let worktree = canonical(workdir)?;
    let cwd = canonical(repo.current_dir())?;
    if cwd == worktree {
        Ok(())
    } else {
        Err(Error::RelativeRepoEnv {
            name,
            value,
            cwd,
            worktree,
        })
    }
}

/// Refuses `GIT_DIR` given without `GIT_WORK_TREE` when wsx runs anywhere
/// but the worktree root, unless the repository's own config sets
/// `core.worktree`. git then takes the current directory as the worktree,
/// and gix the repository's: measured with git 2.56.0 from `main/sub` with
/// an absolute `GIT_DIR=<main>/.git`, `git rev-parse --show-toplevel` prints
/// `main/sub`, while gix's worktree is `main`. A relative `GIT_DIR` is
/// refused first by `refuse_relative_repo_env`. git runs hooks at the
/// worktree root, so a hook's `GIT_DIR` still applies. `core.worktree` counts
/// only from the repository's own config (`Source::Local`), the file git
/// reads it from; set anywhere else, the check still runs, the stricter
/// direction. A repository without a worktree is not checked.
fn refuse_implicit_worktree(repo: &gix::Repository) -> Result<(), Error> {
    let Some(workdir) = repo.workdir() else {
        return Ok(());
    };
    let Some(value) = std::env::var_os("GIT_DIR") else {
        return Ok(());
    };
    let core_worktree = repo
        .config_snapshot()
        .plumbing()
        .string_filter("core.worktree", |meta| {
            meta.source == gix::config::Source::Local
        });
    if std::env::var_os("GIT_WORK_TREE").is_some() || core_worktree.is_some() {
        return Ok(());
    }
    let worktree = canonical(workdir)?;
    let cwd = canonical(repo.current_dir())?;
    if cwd == worktree {
        Ok(())
    } else {
        Err(Error::ImplicitWorkTree {
            git_dir: PathBuf::from(value),
            cwd,
            worktree,
        })
    }
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
    use super::super::testing::{
        self, Env, ForeignOwned, Scratch, T0, TICK, commit_all, git, git_ok, write,
    };
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
        if testing::rerun_scrubbed() {
            return;
        }
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
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("files-twin").expect("scratch");
        let (files, _table) = twins(&scratch);
        let repo = Repo::discover(&files.join("sub")).expect("the files twin opens");
        assert_eq!(repo.worktree(), Some(files.as_path()));
    }

    /// A reftable repository with no commit is refused the same way: the
    /// check reads configuration, not refs.
    #[test]
    fn empty_reftable_repo_is_refused() {
        if testing::rerun_scrubbed() {
            return;
        }
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
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("explicit-files").expect("scratch");
        let dir = ref_storage_repo(&scratch, "repo", "files");
        git_ok(&scratch, &dir, ["rev-parse", "HEAD"]).expect("git still reads the repository");
        let repo = Repo::discover(&dir).expect("an explicit files backend opens");
        assert_eq!(repo.worktree(), Some(dir.as_path()));
    }

    /// `Files` is not `files`: wsx refuses it, as git does.
    #[test]
    fn miscased_ref_storage_is_refused_as_git_refuses_it() {
        if testing::rerun_scrubbed() {
            return;
        }
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
        if testing::rerun_scrubbed() {
            return;
        }
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
        if testing::rerun_scrubbed() {
            return;
        }
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

    /// With the configuration check bypassed, `tree_diff` on a reftable
    /// repository keeps the reftable refusal from resolving its revisions,
    /// not a generic diff error.
    #[test]
    fn tree_diff_keeps_the_reftable_refusal() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("reftable-diff").expect("scratch");
        let (_files, table) = twins(&scratch);
        let repo = open_unchecked(&table);
        match repo.tree_diff(b"HEAD", b"HEAD") {
            Err(Error::Reftable { format, .. }) => assert_eq!(format, "reftable"),
            Err(other) => panic!("diff HEAD HEAD: expected a Reftable refusal, got {other}"),
            Ok(changes) => {
                panic!("diff HEAD HEAD on reftable: expected a refusal, got {changes:?}")
            }
        }
    }

    /// Control: the files twin diffs `HEAD` with itself through the same
    /// path, to no changes.
    #[test]
    fn files_repo_diffs_head_through_the_same_path() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("files-diff").expect("scratch");
        let (files, _table) = twins(&scratch);
        let repo = open_unchecked(&files);
        let changes = repo
            .tree_diff(b"HEAD", b"HEAD")
            .unwrap_or_else(|err| panic!("diff HEAD HEAD on the files twin: {err}"));
        assert!(changes.is_empty(), "diff HEAD HEAD: {changes:?}");
    }

    /// A ref that does not exist is an ordinary read error, never the
    /// reftable refusal.
    #[test]
    fn unknown_revision_is_not_reported_as_reftable() {
        if testing::rerun_scrubbed() {
            return;
        }
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
        if testing::rerun_scrubbed() {
            return;
        }
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
        if testing::rerun_scrubbed() {
            return;
        }
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

    /// Prints every sibling `Repo::siblings` opens from the hook's
    /// repository, then what `Repo::discover` resolves for the sibling path
    /// in `SIBLING_VAR`.
    fn print_siblings() {
        let cwd = std::env::current_dir().expect("read the working directory");
        let repo = Repo::discover(&cwd).expect("discover the hook's repository");
        let siblings = repo.siblings().expect("open the sibling worktrees");
        testing::child_print("sibling_count", siblings.len());
        for (i, sibling) in siblings.iter().enumerate() {
            let worktree = sibling
                .worktree()
                .map_or_else(|| "<none>".to_owned(), |path| path.display().to_string());
            testing::child_print(&format!("sibling_{i}_worktree"), worktree);
            testing::child_print(&format!("sibling_{i}_git_dir"), sibling.git_dir().display());
            testing::child_print(
                &format!("sibling_{i}_index"),
                sibling.index_path().display(),
            );
        }
        let wt2 = std::env::var_os(testing::SIBLING_VAR).expect("the parent set the sibling path");
        match Repo::discover(Path::new(&wt2)) {
            Ok(found) => testing::child_print("discover_wt2_git_dir", found.git_dir().display()),
            Err(err) => testing::child_print("discover_wt2_error", err),
        }
    }

    /// Inside wt1's pre-commit hook, `Repo::siblings()` returns exactly the
    /// main worktree, then wt2 (sorted by canonical worktree path: wt2 is
    /// nested inside main), each with its own git dir and index, never the
    /// hook's.
    #[test]
    fn siblings_in_a_hook_use_their_own_git_dirs() {
        let scratch = Scratch::new("hook-siblings").expect("scratch");
        let (worktrees, output) =
            testing::linked_hook_commit(&scratch, &child_path("linked_hook_siblings_child"))
                .unwrap_or_else(|err| panic!("{err}"));
        let main_git_dir = worktrees.main.join(".git");
        let wt2_git_dir = main_git_dir.join("worktrees/wt2");
        let expected = [
            (&worktrees.main, main_git_dir.clone()),
            (&worktrees.wt2, wt2_git_dir),
        ];

        assert_eq!(fact(&output, "sibling_count"), expected.len().to_string());
        for (i, (worktree, git_dir)) in expected.iter().enumerate() {
            assert_eq!(
                fact(&output, &format!("sibling_{i}_worktree")),
                worktree.display().to_string()
            );
            assert_eq!(
                fact(&output, &format!("sibling_{i}_git_dir")),
                git_dir.display().to_string()
            );
            assert_eq!(
                fact(&output, &format!("sibling_{i}_index")),
                git_dir.join("index").display().to_string()
            );
        }
    }

    /// The control for `siblings_in_a_hook_use_their_own_git_dirs`: in the
    /// same hook, discovering wt2 by its path follows the hook's `GIT_DIR`
    /// and resolves wt1's git dir, the leak `siblings()` avoids.
    #[test]
    fn discovering_a_sibling_inside_a_hook_follows_the_hook_env() {
        let scratch = Scratch::new("hook-discover-sibling").expect("scratch");
        let (worktrees, output) =
            testing::linked_hook_commit(&scratch, &child_path("linked_hook_siblings_child"))
                .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            fact(&output, "discover_wt2_git_dir"),
            worktrees
                .main
                .join(".git/worktrees/wt1")
                .display()
                .to_string()
        );
    }

    #[test]
    #[ignore = "child half of the sibling tests; runs only when re-executed"]
    fn linked_hook_siblings_child() {
        print_siblings();
    }

    /// Siblings come sorted by canonical worktree path, not in gix's order
    /// (the main repository first, then linked worktrees by git dir): a
    /// linked worktree at `<root>/aa` sorts before `<root>/main`, and opened
    /// from wt2 the siblings are aa, main, wt1. The child is re-executed
    /// with the scratch environment, so a `GIT_DIR` in the test process's
    /// own environment cannot redirect its discovery.
    #[test]
    fn siblings_are_sorted_by_worktree_path() {
        let scratch = Scratch::new("siblings-order").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let aa = scratch.root.join("aa");
        let aa_arg = aa.to_str().expect("UTF-8 scratch path");
        git_ok(
            &scratch,
            &worktrees.main,
            ["worktree", "add", "-q", "-b", "aa", aa_arg],
        )
        .expect("git worktree add aa");
        let mut env = scratch.env.clone();
        env.push((testing::SIBLING_VAR.into(), worktrees.wt2.clone().into()));
        let output = testing::reexec(
            &child_path("linked_hook_siblings_child"),
            &env,
            &worktrees.wt2,
            b"",
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run("the siblings child", &output)
        );

        let expected = [&aa, &worktrees.main, &worktrees.wt1];
        assert_eq!(fact(&output, "sibling_count"), expected.len().to_string());
        for (i, worktree) in expected.iter().enumerate() {
            assert_eq!(
                fact(&output, &format!("sibling_{i}_worktree")),
                worktree.display().to_string()
            );
        }
    }

    /// The variable a foreign-owner test hands its child: the repository
    /// the child discovers.
    const REPO_VAR: &str = "WSX_TEST_REPO";

    /// The line of git's dubious-ownership refusal that carries its fix.
    const GIT_FIX: &str = "git config --global --add safe.directory";

    /// A one-commit repository at `<root>/repo`, canonical, then really
    /// owned by another uid. Bind the guard after the scratch, so it drops
    /// first and the scratch stays deletable.
    fn foreign_repo(scratch: &Scratch) -> (PathBuf, ForeignOwned) {
        let dir = scratch.root.join("repo");
        std::fs::create_dir_all(&dir).expect("create repo dir");
        git_ok(scratch, &dir, ["init", "-q", "-b", "main"]).expect("git init");
        write(&dir.join("a.txt"), b"a\n").expect("write a.txt");
        commit_all(scratch, &dir, "a", T0).expect("commit a");
        let dir = dir.canonicalize().expect("canonicalize the repository");
        let guard = ForeignOwned::new(&dir);
        (dir, guard)
    }

    /// Re-executes `foreign_owner_child` from the scratch root with `env`
    /// plus `REPO_VAR=<repo>`, and checks it exited 0.
    fn discover_in_child(scratch: &Scratch, env: &Env, repo: &Path) -> Output {
        let mut env = env.clone();
        env.push((REPO_VAR.into(), repo.into()));
        let output = testing::reexec(&child_path("foreign_owner_child"), &env, &scratch.root, b"")
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run("the foreign-owner child", &output)
        );
        output
    }

    /// The trimmed first line of `text` containing `needle`.
    fn line_with(text: &str, needle: &str) -> String {
        text.lines()
            .find(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("no line containing `{needle}` in:\n{text}"))
            .trim()
            .to_owned()
    }

    /// The child's `result` fact, failing with its whole output otherwise.
    fn child_result(output: &Output) -> String {
        testing::marker(output, "result")
            .unwrap_or_else(|err| panic!("{err}\n{}", testing::failed_run("the child", output)))
    }

    /// A repository really owned by another uid is refused by both: git
    /// exits 128 on dubious ownership, and wsx's `Repo::discover` returns
    /// `ForeignOwner` whose fix line is the one git prints.
    #[test]
    fn foreign_owner_refused() {
        let scratch = Scratch::new("foreign-owner").expect("scratch");
        let (repo, mut guard) = foreign_repo(&scratch);

        let output = git(&scratch, &repo, ["rev-parse", "--absolute-git-dir"]).expect("spawn git");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(128), "git stderr: {stderr}");
        assert!(
            stderr.contains("detected dubious ownership"),
            "git stderr: {stderr}"
        );

        let child = discover_in_child(&scratch, &scratch.env, &repo);
        assert_eq!(
            child_result(&child),
            "foreign_owner",
            "{}",
            testing::failed_run("the foreign-owner child", &child)
        );
        assert_eq!(fact(&child, "fix"), line_with(&stderr, GIT_FIX));

        guard.restore().expect("give the repository back");
    }

    /// Control: the same repository listed as `safe.directory` in the
    /// child's global config opens for git and for wsx.
    #[test]
    fn safe_directory_in_global_config_opens_a_foreign_repo() {
        let scratch = Scratch::new("foreign-safe-global").expect("scratch");
        let (repo, mut guard) = foreign_repo(&scratch);
        let global = scratch.home.join(".gitconfig");
        let mut config = std::fs::read(&global).expect("read the scratch global config");
        config.extend_from_slice(format!("[safe]\n\tdirectory = {}\n", repo.display()).as_bytes());
        write(&global, &config).expect("append safe.directory");

        let output = git(&scratch, &repo, ["rev-parse", "--absolute-git-dir"]).expect("spawn git");
        assert_eq!(
            output.status.code(),
            Some(0),
            "git stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let child = discover_in_child(&scratch, &scratch.env, &repo);
        assert_eq!(
            child_result(&child),
            "opened",
            "{}",
            testing::failed_run("the foreign-owner child", &child)
        );
        assert_eq!(fact(&child, "worktree"), repo.display().to_string());

        guard.restore().expect("give the repository back");
    }

    /// Recorded divergence: `safe.directory` given only through
    /// `GIT_CONFIG_COUNT` makes git open the repository, while wsx still
    /// refuses it, because gix reads `safe.directory` only from global and
    /// system config. The stricter direction.
    #[test]
    fn command_line_safe_directory_does_not_trust_a_foreign_repo() {
        let scratch = Scratch::new("foreign-safe-env").expect("scratch");
        let (repo, mut guard) = foreign_repo(&scratch);
        let mut env = scratch.env.clone();
        env.push(("GIT_CONFIG_COUNT".into(), "1".into()));
        env.push(("GIT_CONFIG_KEY_0".into(), "safe.directory".into()));
        env.push(("GIT_CONFIG_VALUE_0".into(), repo.clone().into()));

        let output = testing::git_in_env(&env, &repo, ["rev-parse", "--absolute-git-dir"])
            .expect("spawn git");
        assert_eq!(
            output.status.code(),
            Some(0),
            "git stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let child = discover_in_child(&scratch, &env, &repo);
        assert_eq!(
            child_result(&child),
            "foreign_owner",
            "{}",
            testing::failed_run("the foreign-owner child", &child)
        );

        guard.restore().expect("give the repository back");
    }

    #[test]
    #[ignore = "child half of the foreign-owner tests; runs only when re-executed"]
    fn foreign_owner_child() {
        let repo = std::env::var_os(REPO_VAR).expect("the parent set the repository path");
        match Repo::discover(Path::new(&repo)) {
            Ok(found) => {
                testing::child_print("result", "opened");
                let worktree = found
                    .worktree()
                    .map_or_else(|| "<none>".to_owned(), |path| path.display().to_string());
                testing::child_print("worktree", worktree);
            }
            Err(err @ Error::ForeignOwner { .. }) => {
                testing::child_print("result", "foreign_owner");
                testing::child_print("fix", line_with(&err.to_string(), "safe.directory"));
            }
            Err(err) => {
                testing::child_print("result", "other");
                testing::child_print("error", err.to_string().replace('\n', "\\n"));
            }
        }
    }

    /// Appends `path` to `safe.directory` in the scratch global config.
    fn trust_globally(scratch: &Scratch, path: &Path) {
        let global = scratch.home.join(".gitconfig");
        let mut config = std::fs::read(&global).expect("read the scratch global config");
        config.extend_from_slice(format!("[safe]\n\tdirectory = {}\n", path.display()).as_bytes());
        write(&global, &config).expect("append safe.directory");
    }

    /// The exit code of `git rev-parse --absolute-git-dir` in `dir`, and
    /// its stderr.
    fn git_opens(scratch: &Scratch, dir: &Path) -> (Option<i32>, String) {
        let output = git(scratch, dir, ["rev-parse", "--absolute-git-dir"]).expect("spawn git");
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    /// Checks git refuses `dir` as owned by another user.
    fn assert_git_refuses_ownership(scratch: &Scratch, dir: &Path) {
        let (code, stderr) = git_opens(scratch, dir);
        assert_eq!(code, Some(128), "git in {}: {stderr}", dir.display());
        assert!(
            stderr.contains("detected dubious ownership"),
            "git in {}: {stderr}",
            dir.display()
        );
    }

    /// Checks git opens `dir`.
    fn assert_git_opens(scratch: &Scratch, dir: &Path) {
        let (code, stderr) = git_opens(scratch, dir);
        assert_eq!(code, Some(0), "git in {}: {stderr}", dir.display());
    }

    /// Re-executes `sibling_trust_child` in `start` with the scratch
    /// environment, and checks it exited 0.
    fn siblings_in_child(scratch: &Scratch, start: &Path) -> Output {
        let output = testing::reexec(&child_path("sibling_trust_child"), &scratch.env, start, b"")
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run("the sibling-trust child", &output)
        );
        output
    }

    /// A linked sibling worktree really owned by another uid is refused by
    /// `siblings()`, naming that worktree, as git refuses it. gix itself
    /// opens such a sibling with reduced trust and its config dropped.
    #[test]
    fn a_foreign_owned_sibling_worktree_is_refused() {
        let scratch = Scratch::new("foreign-sibling").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let mut guard = ForeignOwned::new(&worktrees.wt1);
        assert_git_refuses_ownership(&scratch, &worktrees.wt1);

        let child = siblings_in_child(&scratch, &worktrees.main);
        assert_eq!(
            child_result(&child),
            "foreign_owner",
            "{}",
            testing::failed_run("the sibling-trust child", &child)
        );
        assert_eq!(fact(&child, "path"), worktrees.wt1.display().to_string());

        guard.restore().expect("give the worktree back");
    }

    /// Control: the same foreign sibling listed as `safe.directory` in
    /// global config opens for git, and `siblings()` returns both linked
    /// worktrees, sorted by path (wt2 is nested inside main).
    #[test]
    fn a_foreign_sibling_worktree_in_safe_directory_opens() {
        let scratch = Scratch::new("foreign-sibling-safe").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let mut guard = ForeignOwned::new(&worktrees.wt1);
        trust_globally(&scratch, &worktrees.wt1);
        assert_git_opens(&scratch, &worktrees.wt1);

        let child = siblings_in_child(&scratch, &worktrees.main);
        assert_eq!(
            child_result(&child),
            "opened",
            "{}",
            testing::failed_run("the sibling-trust child", &child)
        );
        let common = worktrees.main.join(".git/worktrees");
        let expected = [common.join("wt2"), common.join("wt1")];
        assert_eq!(fact(&child, "sibling_count"), expected.len().to_string());
        for (i, git_dir) in expected.iter().enumerate() {
            assert_eq!(
                fact(&child, &format!("sibling_{i}_git_dir")),
                git_dir.display().to_string()
            );
        }

        guard.restore().expect("give the worktree back");
    }

    /// A linked worktree whose directory was deleted without
    /// `git worktree prune` (git lists it as prunable) makes `siblings()`
    /// fail closed with an error naming that worktree and the prune that
    /// clears it, not a bare canonicalisation failure.
    #[test]
    fn a_stale_sibling_worktree_is_named_with_its_fix() {
        let scratch = Scratch::new("stale-sibling").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).unwrap_or_else(|err| panic!("{err}"));
        std::fs::remove_dir_all(&worktrees.wt1).expect("delete wt1's directory");
        let listed = git_line(
            &scratch,
            &worktrees.main,
            &["worktree", "list", "--porcelain"],
        );
        let wt1_entry = listed
            .split("\n\n")
            .find(|entry| entry.contains(&format!("worktree {}", worktrees.wt1.display())))
            .unwrap_or_else(|| panic!("git worktree list has no wt1 entry:\n{listed}"));
        assert!(
            wt1_entry.contains("prunable"),
            "git lists wt1 as prunable:\n{listed}"
        );

        let child = siblings_in_child(&scratch, &worktrees.main);
        let error = fact(&child, "error");
        assert!(
            error.contains(&worktrees.wt1.display().to_string()),
            "error: {error}"
        );
        assert!(error.contains("git worktree prune"), "error: {error}");
    }

    /// Control: with every linked worktree present, `siblings()` from main
    /// opens both.
    #[test]
    fn siblings_open_every_present_linked_worktree() {
        let scratch = Scratch::new("present-siblings").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let child = siblings_in_child(&scratch, &worktrees.main);
        assert_eq!(
            child_result(&child),
            "opened",
            "{}",
            testing::failed_run("the sibling-trust child", &child)
        );
        assert_eq!(fact(&child, "sibling_count"), "2");
    }

    /// A bare repository `<root>/r.git`, cloned from a one-commit
    /// repository, with the linked worktree `<root>/wt` on branch `wt`;
    /// both canonical.
    fn bare_with_worktree(scratch: &Scratch) -> (PathBuf, PathBuf) {
        let src = scratch.root.join("src");
        std::fs::create_dir_all(&src).expect("create src");
        git_ok(scratch, &src, ["init", "-q", "-b", "main"]).expect("git init src");
        write(&src.join("a.txt"), b"a\n").expect("write a.txt");
        commit_all(scratch, &src, "a", T0).expect("commit a");
        let bare = scratch.root.join("r.git");
        let wt = scratch.root.join("wt");
        let (src_arg, bare_arg, wt_arg) = (
            src.to_str().expect("UTF-8 scratch path"),
            bare.to_str().expect("UTF-8 scratch path"),
            wt.to_str().expect("UTF-8 scratch path"),
        );
        git_ok(
            scratch,
            &scratch.root,
            ["clone", "-q", "--bare", src_arg, bare_arg],
        )
        .expect("git clone --bare");
        git_ok(
            scratch,
            &bare,
            ["worktree", "add", "-q", "-b", "wt", wt_arg],
        )
        .expect("git worktree add wt");
        (
            bare.canonicalize().expect("canonicalize r.git"),
            wt.canonicalize().expect("canonicalize wt"),
        )
    }

    /// A bare main repository really owned by another uid is refused as a
    /// sibling of a linked worktree the user trusts through
    /// `safe.directory`: git opens the worktree and refuses the bare
    /// repository. gix opens that sibling with the worktree's full trust
    /// and checks nothing, as it has no worktree.
    #[test]
    fn a_foreign_owned_bare_main_repository_is_refused_as_a_sibling() {
        let scratch = Scratch::new("foreign-bare-main").expect("scratch");
        let (bare, wt) = bare_with_worktree(&scratch);
        let mut guard = ForeignOwned::new(&bare);
        trust_globally(&scratch, &wt);
        assert_git_opens(&scratch, &wt);
        assert_git_refuses_ownership(&scratch, &bare);

        let child = siblings_in_child(&scratch, &wt);
        assert_eq!(
            child_result(&child),
            "foreign_owner",
            "{}",
            testing::failed_run("the sibling-trust child", &child)
        );
        assert_eq!(fact(&child, "path"), bare.display().to_string());

        guard.restore().expect("give the bare repository back");
    }

    /// Control: the same layout, owned by the test user, opens the bare
    /// main repository as the one sibling.
    #[test]
    fn an_owned_bare_main_repository_opens_as_a_sibling() {
        let scratch = Scratch::new("owned-bare-main").expect("scratch");
        let (bare, wt) = bare_with_worktree(&scratch);
        assert_git_opens(&scratch, &bare);

        let child = siblings_in_child(&scratch, &wt);
        assert_eq!(
            child_result(&child),
            "opened",
            "{}",
            testing::failed_run("the sibling-trust child", &child)
        );
        assert_eq!(fact(&child, "sibling_count"), "1");
        assert_eq!(
            fact(&child, "sibling_0_git_dir"),
            bare.display().to_string()
        );
    }

    /// Recorded divergence: a foreign bare main repository listed in global
    /// `safe.directory` opens for git, but `siblings()` still refuses it.
    /// gix checks `safe.directory` for a sibling only against its worktree,
    /// and a bare repository has none, so wsx checks ownership alone. The
    /// stricter direction.
    #[test]
    fn safe_directory_does_not_trust_a_foreign_bare_main_sibling() {
        let scratch = Scratch::new("foreign-bare-main-safe").expect("scratch");
        let (bare, wt) = bare_with_worktree(&scratch);
        let mut guard = ForeignOwned::new(&bare);
        trust_globally(&scratch, &wt);
        trust_globally(&scratch, &bare);
        assert_git_opens(&scratch, &bare);

        let child = siblings_in_child(&scratch, &wt);
        assert_eq!(
            child_result(&child),
            "foreign_owner",
            "{}",
            testing::failed_run("the sibling-trust child", &child)
        );

        guard.restore().expect("give the bare repository back");
    }

    /// Prints what `siblings()` returns from the repository discovered in
    /// the working directory: `result=opened` with `sibling_count` and each
    /// `sibling_<i>_git_dir`, `result=foreign_owner` with `path`, or
    /// `result=other` with `error`.
    #[test]
    #[ignore = "child half of the sibling-trust tests; runs only when re-executed"]
    fn sibling_trust_child() {
        let cwd = std::env::current_dir().expect("read the working directory");
        let repo = Repo::discover(&cwd).expect("discover the starting worktree");
        match repo.siblings() {
            Ok(siblings) => {
                testing::child_print("result", "opened");
                testing::child_print("sibling_count", siblings.len());
                for (i, sibling) in siblings.iter().enumerate() {
                    testing::child_print(
                        &format!("sibling_{i}_git_dir"),
                        sibling.git_dir().display(),
                    );
                }
            }
            Err(Error::ForeignOwner { path }) => {
                testing::child_print("result", "foreign_owner");
                testing::child_print("path", path.display());
            }
            Err(err) => {
                testing::child_print("result", "other");
                testing::child_print("error", err.to_string().replace('\n', "\\n"));
            }
        }
    }

    /// The variable a ceiling test hands its child: the directory the child
    /// discovers from.
    const START_VAR: &str = "WSX_TEST_START";

    /// `<root>/r1`, canonical, made with `git init -q` and one commit holding
    /// `inner/deeper/z.txt`, so `r1/inner` and `r1/inner/deeper` exist.
    fn ceiling_repo(scratch: &Scratch) -> PathBuf {
        let r1 = scratch.root.join("r1");
        std::fs::create_dir_all(&r1).expect("create r1");
        git_ok(scratch, &r1, ["init", "-q"]).expect("git init r1");
        write(&r1.join("inner/deeper/z.txt"), b"z").expect("write z.txt");
        commit_all(scratch, &r1, "z", T0).expect("commit z");
        r1.canonicalize().expect("canonicalize r1")
    }

    /// What git and wsx each found from one start under one ceiling.
    struct CeilingRun {
        /// `git rev-parse --absolute-git-dir` run in the start directory.
        git: Output,
        /// `ceiling_child` re-executed with the same environment.
        child: Output,
    }

    impl CeilingRun {
        /// Both outputs, for a failure message.
        fn both(&self) -> String {
            format!(
                "{}\n{}",
                testing::failed_run("git rev-parse --absolute-git-dir", &self.git),
                testing::failed_run("the ceiling child", &self.child)
            )
        }
    }

    /// Runs git and `ceiling_child` from `start` with the scratch
    /// environment, its `GIT_CEILING_DIRECTORIES` replaced by `ceiling`.
    fn ceiling_run(scratch: &Scratch, ceiling: &Path, start: &Path) -> CeilingRun {
        ceiling_run_with(scratch, &[], ceiling, start)
    }

    /// `ceiling_run` with the `extra` variables added to both runs.
    fn ceiling_run_with(
        scratch: &Scratch,
        extra: &[(&str, &Path)],
        ceiling: &Path,
        start: &Path,
    ) -> CeilingRun {
        let mut env: Env = scratch
            .env
            .iter()
            .filter(|(key, _)| key != "GIT_CEILING_DIRECTORIES")
            .cloned()
            .collect();
        env.push(("GIT_CEILING_DIRECTORIES".into(), ceiling.into()));
        env.extend(
            extra
                .iter()
                .map(|(name, value)| ((*name).into(), (*value).into())),
        );
        let git = testing::git_in_env(&env, start, ["rev-parse", "--absolute-git-dir"])
            .unwrap_or_else(|err| panic!("{err}"));
        env.push((START_VAR.into(), start.into()));
        let child = testing::reexec(&child_path("ceiling_child"), &env, &scratch.root, b"")
            .unwrap_or_else(|err| panic!("{err}"));
        CeilingRun { git, child }
    }

    /// The child's `result`, failing with both outputs when it printed none.
    fn ceiling_result(run: &CeilingRun) -> String {
        testing::marker(&run.child, "result").unwrap_or_else(|err| panic!("{err}\n{}", run.both()))
    }

    /// git found no repository (exit 128), then wsx returned `NotARepo`.
    fn assert_both_hidden(run: &CeilingRun) {
        assert_eq!(run.git.status.code(), Some(128), "{}", run.both());
        assert_eq!(ceiling_result(run), "not_a_repo", "{}", run.both());
    }

    /// git found `<r1>/.git` (exit 0), then wsx opened worktree `r1`.
    fn assert_both_open(run: &CeilingRun, r1: &Path) {
        assert_eq!(run.git.status.code(), Some(0), "{}", run.both());
        assert_eq!(
            String::from_utf8_lossy(&run.git.stdout).trim_end(),
            r1.join(".git").display().to_string(),
            "{}",
            run.both()
        );
        assert_eq!(ceiling_result(run), "opened", "{}", run.both());
        assert_eq!(
            testing::marker(&run.child, "worktree").unwrap_or_else(|err| panic!("{err}")),
            r1.display().to_string(),
            "{}",
            run.both()
        );
    }

    /// With the ceiling at the repository's own top level, discovery from
    /// `r1/inner` finds nothing in git, and none in wsx, although gix alone
    /// searches the ceiling directory and finds `r1/.git`.
    #[test]
    fn ceiling_at_the_repository_root_hides_it_like_git() {
        let scratch = Scratch::new("ceiling-root").expect("scratch");
        let r1 = ceiling_repo(&scratch);
        assert_both_hidden(&ceiling_run(&scratch, &r1, &r1.join("inner")));
    }

    /// The same from two levels below the ceiling, `r1/inner/deeper`.
    #[test]
    fn ceiling_at_the_repository_root_hides_it_two_levels_down() {
        let scratch = Scratch::new("ceiling-root-deeper").expect("scratch");
        let r1 = ceiling_repo(&scratch);
        assert_both_hidden(&ceiling_run(&scratch, &r1, &r1.join("inner/deeper")));
    }

    /// Control: with the ceiling at `r1`'s parent, which is not a
    /// repository, git and wsx both find `r1` from `r1/inner`.
    #[test]
    fn ceiling_above_the_repository_lets_wsx_and_git_find_it() {
        let scratch = Scratch::new("ceiling-above").expect("scratch");
        let r1 = ceiling_repo(&scratch);
        let parent = r1.parent().expect("r1 has a parent");
        assert_both_open(&ceiling_run(&scratch, parent, &r1.join("inner")), &r1);
    }

    /// git applies a ceiling only to directories above the start, so a
    /// ceiling equal to the start `r1/inner` does not stop git or wsx from
    /// finding `r1`.
    #[test]
    fn ceiling_equal_to_the_start_directory_does_not_stop_discovery() {
        let scratch = Scratch::new("ceiling-start").expect("scratch");
        let r1 = ceiling_repo(&scratch);
        let inner = r1.join("inner");
        assert_both_open(&ceiling_run(&scratch, &inner, &inner), &r1);
    }

    /// Starting in the ceiling directory that holds the repository opens it
    /// for git and for wsx: the ceiling hides a repository only when
    /// discovery had to climb into it.
    #[test]
    fn starting_in_a_ceiling_that_holds_the_repository_opens_it() {
        let scratch = Scratch::new("ceiling-in-root").expect("scratch");
        let r1 = ceiling_repo(&scratch);
        assert_both_open(&ceiling_run(&scratch, &r1, &r1), &r1);
    }

    /// `<root>/<name>`, created and canonical: a worktree directory outside
    /// `r1`.
    fn outside_dir(scratch: &Scratch, name: &str) -> PathBuf {
        let dir = scratch.root.join(name);
        std::fs::create_dir_all(&dir).expect("create a directory outside r1");
        dir.canonicalize()
            .expect("canonicalize the outside directory")
    }

    /// git compares the ceiling with the directory holding `.git`, not with
    /// the worktree: with `GIT_WORK_TREE` elsewhere and no `GIT_DIR`, a
    /// ceiling at `r1` still hides `r1/.git` from `r1/inner`, for git and
    /// for wsx, although gix reports the worktree as the other directory.
    #[test]
    fn ceiling_hides_the_repository_when_git_work_tree_points_elsewhere() {
        let scratch = Scratch::new("ceiling-work-tree").expect("scratch");
        let r1 = ceiling_repo(&scratch);
        let elsewhere = outside_dir(&scratch, "elsewhere");
        let run = ceiling_run_with(
            &scratch,
            &[("GIT_WORK_TREE", &elsewhere)],
            &r1,
            &r1.join("inner"),
        );
        assert_both_hidden(&run);
    }

    /// The same for `core.worktree` in the repository's config: a ceiling at
    /// `r1` hides `r1/.git` from `r1/inner`, for git and for wsx.
    #[test]
    fn ceiling_hides_the_repository_when_core_worktree_points_elsewhere() {
        let scratch = Scratch::new("ceiling-core-worktree").expect("scratch");
        let r1 = ceiling_repo(&scratch);
        let wt2 = outside_dir(&scratch, "wt2");
        let wt2_arg = wt2.to_str().expect("UTF-8 scratch path");
        git_ok(&scratch, &r1, ["config", "core.worktree", wt2_arg]).expect("set core.worktree");
        assert_both_hidden(&ceiling_run(&scratch, &r1, &r1.join("inner")));
    }

    /// Control for the two tests above: with the same `GIT_WORK_TREE` and
    /// the ceiling at `r1`'s parent, git and wsx both find `r1/.git`, and
    /// wsx opens the other directory as the worktree.
    #[test]
    fn ceiling_above_the_repository_opens_it_with_git_work_tree_elsewhere() {
        let scratch = Scratch::new("ceiling-work-tree-above").expect("scratch");
        let r1 = ceiling_repo(&scratch);
        let elsewhere = outside_dir(&scratch, "elsewhere");
        let parent = r1.parent().expect("r1 has a parent");
        let run = ceiling_run_with(
            &scratch,
            &[("GIT_WORK_TREE", &elsewhere)],
            parent,
            &r1.join("inner"),
        );
        assert_eq!(run.git.status.code(), Some(0), "{}", run.both());
        assert_eq!(ceiling_result(&run), "opened", "{}", run.both());
        assert_eq!(
            testing::marker(&run.child, "worktree").unwrap_or_else(|err| panic!("{err}")),
            elsewhere.display().to_string(),
            "{}",
            run.both()
        );
    }

    /// A linked worktree's `.git` is a `gitdir:` file: with the ceiling at
    /// `wt1`, discovery from `wt1/sub` finds nothing in git, and none in wsx.
    #[test]
    fn ceiling_at_a_linked_worktree_hides_it_like_git() {
        let scratch = Scratch::new("ceiling-linked").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let sub = worktrees.wt1.join("sub");
        std::fs::create_dir_all(&sub).expect("create wt1/sub");
        assert_both_hidden(&ceiling_run(&scratch, &worktrees.wt1, &sub));
    }

    /// Control: with the ceiling at `wt1`'s parent, git and wsx both find
    /// wt1 from `wt1/sub` through its `gitdir:` file.
    #[test]
    fn ceiling_above_a_linked_worktree_lets_wsx_and_git_find_it() {
        let scratch = Scratch::new("ceiling-above-linked").expect("scratch");
        let worktrees = testing::linked_worktrees(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let sub = worktrees.wt1.join("sub");
        std::fs::create_dir_all(&sub).expect("create wt1/sub");
        let run = ceiling_run(&scratch, &scratch.root, &sub);
        assert_eq!(run.git.status.code(), Some(0), "{}", run.both());
        assert_eq!(
            String::from_utf8_lossy(&run.git.stdout).trim_end(),
            worktrees
                .main
                .join(".git/worktrees/wt1")
                .display()
                .to_string(),
            "{}",
            run.both()
        );
        assert_eq!(ceiling_result(&run), "opened", "{}", run.both());
        assert_eq!(
            testing::marker(&run.child, "worktree").unwrap_or_else(|err| panic!("{err}")),
            worktrees.wt1.display().to_string(),
            "{}",
            run.both()
        );
    }

    /// Starting inside `r1/.git`, git recognises the git dir itself while
    /// climbing, below the ceiling `r1`, and opens it; so does wsx.
    #[test]
    fn starting_inside_the_git_dir_below_a_ceiling_opens_it() {
        let scratch = Scratch::new("ceiling-inside-git-dir").expect("scratch");
        let r1 = ceiling_repo(&scratch);
        assert_both_open(&ceiling_run(&scratch, &r1, &r1.join(".git/objects")), &r1);
    }

    /// `<root>/r.git`, canonical, made with `git init -q --bare`, so
    /// `r.git/objects` exists.
    fn bare_ceiling_repo(scratch: &Scratch) -> PathBuf {
        let bare = scratch.root.join("r.git");
        let bare_arg = bare.to_str().expect("UTF-8 scratch path");
        git_ok(scratch, &scratch.root, ["init", "-q", "--bare", bare_arg])
            .expect("git init --bare r.git");
        bare.canonicalize().expect("canonicalize r.git")
    }

    /// git found the bare `bare` (exit 0, printing it), then wsx opened it
    /// with no worktree.
    fn assert_both_open_bare(run: &CeilingRun, bare: &Path) {
        assert_eq!(run.git.status.code(), Some(0), "{}", run.both());
        assert_eq!(
            String::from_utf8_lossy(&run.git.stdout).trim_end(),
            bare.display().to_string(),
            "{}",
            run.both()
        );
        assert_eq!(ceiling_result(run), "opened", "{}", run.both());
        assert_eq!(
            testing::marker(&run.child, "worktree").unwrap_or_else(|err| panic!("{err}")),
            "<none>",
            "{}",
            run.both()
        );
    }

    /// With the ceiling at a bare repository, discovery from inside it
    /// (`r.git/objects`) finds nothing in git, and none in wsx, although gix
    /// alone searches the ceiling directory and finds `r.git`.
    #[test]
    fn ceiling_at_a_bare_repository_hides_it_like_git() {
        let scratch = Scratch::new("ceiling-bare").expect("scratch");
        let bare = bare_ceiling_repo(&scratch);
        assert_both_hidden(&ceiling_run(&scratch, &bare, &bare.join("objects")));
    }

    /// Control: with the ceiling at the bare repository's parent, git and
    /// wsx both find `r.git` from `r.git/objects`.
    #[test]
    fn ceiling_above_a_bare_repository_lets_wsx_and_git_find_it() {
        let scratch = Scratch::new("ceiling-above-bare").expect("scratch");
        let bare = bare_ceiling_repo(&scratch);
        let parent = bare.parent().expect("r.git has a parent");
        assert_both_open_bare(&ceiling_run(&scratch, parent, &bare.join("objects")), &bare);
    }

    /// Starting in the bare repository that is the ceiling opens it for git
    /// and for wsx.
    #[test]
    fn starting_in_a_bare_repository_that_is_the_ceiling_opens_it() {
        let scratch = Scratch::new("ceiling-in-bare").expect("scratch");
        let bare = bare_ceiling_repo(&scratch);
        assert_both_open_bare(&ceiling_run(&scratch, &bare, &bare), &bare);
    }

    /// Prints what `Repo::discover` makes of the directory in `START_VAR`:
    /// `result=opened` with `worktree`, `result=not_a_repo`,
    /// `result=reftable` with `format`, or `result=other` with `error`.
    fn print_discovery() {
        let start = std::env::var_os(START_VAR).expect("the parent set the start directory");
        match Repo::discover(Path::new(&start)) {
            Ok(found) => {
                testing::child_print("result", "opened");
                let worktree = found
                    .worktree()
                    .map_or_else(|| "<none>".to_owned(), |path| path.display().to_string());
                testing::child_print("worktree", worktree);
            }
            Err(Error::NotARepo { .. }) => testing::child_print("result", "not_a_repo"),
            Err(Error::Reftable { format, .. }) => {
                testing::child_print("result", "reftable");
                testing::child_print("format", format);
            }
            Err(err) => {
                testing::child_print("result", "other");
                testing::child_print("error", err.to_string().replace('\n', "\\n"));
            }
        }
    }

    #[test]
    #[ignore = "child half of the ceiling tests; runs only when re-executed"]
    fn ceiling_child() {
        print_discovery();
    }

    /// A one-commit files repository at `<root>/repo`, canonical, and the
    /// scratch environment plus `START_VAR=<repo>`.
    fn files_repo_and_env(scratch: &Scratch) -> (PathBuf, Env) {
        let dir = scratch.root.join("repo");
        std::fs::create_dir_all(&dir).expect("create repo dir");
        git_ok(scratch, &dir, ["init", "-q", "-b", "main"]).expect("git init");
        write(&dir.join("a.txt"), b"a\n").expect("write a.txt");
        commit_all(scratch, &dir, "a", T0).expect("commit a");
        let dir = dir.canonicalize().expect("canonicalize the repository");
        let mut env = scratch.env.clone();
        env.push((START_VAR.into(), dir.clone().into()));
        (dir, env)
    }

    /// With `env`, git reads `repo` as a files repository (`rev-parse
    /// --show-ref-format` exits 0 printing `files`), and `Repo::discover`
    /// in a re-executed `ref_storage_child` opens it; both outputs are
    /// printed on failure.
    fn assert_both_read_files(scratch: &Scratch, env: &Env, repo: &Path) {
        let git = testing::git_in_env(env, repo, ["rev-parse", "--show-ref-format"])
            .unwrap_or_else(|err| panic!("{err}"));
        let child = testing::reexec(&child_path("ref_storage_child"), env, &scratch.root, b"")
            .unwrap_or_else(|err| panic!("{err}"));
        let both = format!(
            "{}\n{}",
            testing::failed_run("git rev-parse --show-ref-format", &git),
            testing::failed_run("the ref-storage child", &child)
        );
        assert_eq!(git.status.code(), Some(0), "{both}");
        assert_eq!(
            String::from_utf8_lossy(&git.stdout).trim_end(),
            "files",
            "{both}"
        );
        let result = testing::marker(&child, "result").unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(result, "opened", "{both}");
        assert_eq!(
            testing::marker(&child, "worktree").unwrap_or_else(|err| panic!("{err}")),
            repo.display().to_string(),
            "{both}"
        );
    }

    /// git reads `extensions.refStorage` from the repository's own config
    /// only: set to `reftable` in global config, it leaves a files
    /// repository readable to git, and to wsx.
    #[test]
    fn ref_storage_in_global_config_does_not_refuse_a_files_repo() {
        let scratch = Scratch::new("ref-storage-global").expect("scratch");
        let (repo, env) = files_repo_and_env(&scratch);
        let global = scratch.home.join(".gitconfig");
        let mut config = std::fs::read(&global).expect("read the scratch global config");
        config.extend_from_slice(b"[extensions]\n\trefStorage = reftable\n");
        write(&global, &config).expect("append extensions.refStorage");
        assert_both_read_files(&scratch, &env, &repo);
    }

    /// The same for `extensions.refStorage = reftable` in the worktree's
    /// `config.worktree`, which git 2.56.0 reads for other keys
    /// (`extensions.worktreeConfig = true`) but not for the ref format.
    #[test]
    fn ref_storage_in_worktree_config_does_not_refuse_a_files_repo() {
        let scratch = Scratch::new("ref-storage-worktree").expect("scratch");
        let (repo, env) = files_repo_and_env(&scratch);
        for args in [
            ["config", "core.repositoryformatversion", "1"].as_slice(),
            &["config", "extensions.worktreeConfig", "true"],
            &["config", "--worktree", "extensions.refStorage", "reftable"],
        ] {
            git_ok(&scratch, &repo, args).unwrap_or_else(|err| panic!("{err}"));
        }
        assert_both_read_files(&scratch, &env, &repo);
    }

    /// The same for `extensions.refStorage = reftable` given through
    /// `GIT_CONFIG_COUNT`.
    #[test]
    fn ref_storage_in_environment_config_does_not_refuse_a_files_repo() {
        let scratch = Scratch::new("ref-storage-env").expect("scratch");
        let (repo, mut env) = files_repo_and_env(&scratch);
        env.push(("GIT_CONFIG_COUNT".into(), "1".into()));
        env.push(("GIT_CONFIG_KEY_0".into(), "extensions.refStorage".into()));
        env.push(("GIT_CONFIG_VALUE_0".into(), "reftable".into()));
        assert_both_read_files(&scratch, &env, &repo);
    }

    #[test]
    #[ignore = "child half of the ref-storage source tests; runs only when re-executed"]
    fn ref_storage_child() {
        print_discovery();
    }

    /// `<root>/main`, canonical, committing `a.txt` and `b.txt`, with an
    /// empty `sub/` and two extra indexes: `main/alt.idx` holding only
    /// `b.txt` and `main/sub/alt.idx` holding only `a.txt`.
    fn two_index_repo(scratch: &Scratch) -> PathBuf {
        let main = scratch.root.join("main");
        std::fs::create_dir_all(main.join("sub")).expect("create main/sub");
        git_ok(scratch, &main, ["init", "-q", "-b", "main"]).expect("git init");
        write(&main.join("a.txt"), b"a\n").expect("write a.txt");
        write(&main.join("b.txt"), b"b\n").expect("write b.txt");
        commit_all(scratch, &main, "ab", T0).expect("commit a and b");
        let main = main.canonicalize().expect("canonicalize main");
        for (index, file) in [("alt.idx", "b.txt"), ("sub/alt.idx", "a.txt")] {
            let mut env = scratch.env.clone();
            env.push(("GIT_INDEX_FILE".into(), main.join(index).into()));
            let output = testing::git_in_env(&env, &main, ["add", file])
                .unwrap_or_else(|err| panic!("{err}"));
            assert_eq!(
                output.status.code(),
                Some(0),
                "{}",
                testing::failed_run("git add into an extra index", &output)
            );
        }
        main
    }

    /// The scratch environment plus `name=value`.
    fn env_with(scratch: &Scratch, name: &str, value: impl Into<std::ffi::OsString>) -> Env {
        let mut env = scratch.env.clone();
        env.push((name.into(), value.into()));
        env
    }

    /// The trimmed stdout of `git <args>` in `dir` with `env`, which must
    /// exit 0.
    fn git_line_in(env: &Env, dir: &Path, args: &[&str]) -> String {
        let output = testing::git_in_env(env, dir, args).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run("git", &output)
        );
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned()
    }

    /// Re-executes `relative_env_child` in `cwd` with `env`, and checks it
    /// exited 0.
    fn relative_env_run(env: &Env, cwd: &Path) -> Output {
        let output = testing::reexec(&child_path("relative_env_child"), env, cwd, b"")
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run("the relative-env child", &output)
        );
        output
    }

    /// Below the worktree root, git resolves a relative `GIT_INDEX_FILE`
    /// against the worktree root (`main/alt.idx`, so `b.txt`), and gix
    /// against the current directory (`main/sub/alt.idx`): wsx refuses it,
    /// naming the variable, rather than read one index and write another.
    #[test]
    fn a_relative_index_below_the_worktree_root_is_refused() {
        let scratch = Scratch::new("relative-index-sub").expect("scratch");
        let main = two_index_repo(&scratch);
        let sub = main.join("sub");
        let env = env_with(&scratch, "GIT_INDEX_FILE", "alt.idx");
        assert_eq!(
            git_line_in(&env, &sub, &["ls-files", "--full-name", ":/"]),
            "b.txt",
            "git reads a relative GIT_INDEX_FILE from the worktree root"
        );
        let absolute = env_with(&scratch, "GIT_INDEX_FILE", sub.join("alt.idx"));
        assert_eq!(
            git_line_in(&absolute, &sub, &["ls-files", "--full-name", ":/"]),
            "a.txt",
            "the index beside the current directory holds a.txt"
        );

        let child = relative_env_run(&env, &sub);
        assert_eq!(
            child_result(&child),
            "refused",
            "{}",
            testing::failed_run("the relative-env child", &child)
        );
        let error = fact(&child, "error");
        assert!(error.contains("GIT_INDEX_FILE=alt.idx"), "error: {error}");
    }

    /// Below the worktree root, git takes the current directory as the
    /// worktree of a relative `GIT_DIR` given without `GIT_WORK_TREE`, and
    /// gix the repository's own worktree: wsx refuses it, naming the
    /// variable.
    #[test]
    fn a_relative_git_dir_below_the_worktree_root_is_refused() {
        let scratch = Scratch::new("relative-git-dir-sub").expect("scratch");
        let main = two_index_repo(&scratch);
        let sub = main.join("sub");
        let env = env_with(&scratch, "GIT_DIR", "../.git");
        assert_eq!(
            git_line_in(&env, &sub, &["rev-parse", "--show-toplevel"]),
            sub.display().to_string(),
            "git takes the current directory as the worktree"
        );

        let child = relative_env_run(&env, &sub);
        assert_eq!(
            child_result(&child),
            "refused",
            "{}",
            testing::failed_run("the relative-env child", &child)
        );
        let error = fact(&child, "error");
        assert!(error.contains("GIT_DIR=../.git"), "error: {error}");
    }

    /// An absolute `GIT_DIR` given without `GIT_WORK_TREE` or
    /// `core.worktree` makes git take the current directory `main/sub` as
    /// the worktree, where gix takes `main`: wsx refuses it below the root,
    /// naming the variable.
    #[test]
    fn an_absolute_git_dir_below_the_worktree_root_is_refused() {
        let scratch = Scratch::new("absolute-git-dir-sub").expect("scratch");
        let main = two_index_repo(&scratch);
        let sub = main.join("sub");
        let git_dir = main.join(".git");
        let env = env_with(&scratch, "GIT_DIR", git_dir.clone());
        assert_eq!(
            git_line_in(&env, &sub, &["rev-parse", "--show-toplevel"]),
            sub.display().to_string(),
            "git takes the current directory as the worktree"
        );

        let child = relative_env_run(&env, &sub);
        assert_eq!(
            child_result(&child),
            "refused",
            "{}",
            testing::failed_run("the relative-env child", &child)
        );
        let error = fact(&child, "error");
        assert!(
            error.contains(&format!("GIT_DIR={}", git_dir.display())),
            "error: {error}"
        );
    }

    /// Control: the same absolute `GIT_DIR` at the worktree root, where git
    /// takes `main` as the worktree too, opens, and wsx's git child lists
    /// the repository's files.
    #[test]
    fn an_absolute_git_dir_at_the_worktree_root_opens_like_git() {
        let scratch = Scratch::new("absolute-git-dir-root").expect("scratch");
        let main = two_index_repo(&scratch);
        let env = env_with(&scratch, "GIT_DIR", main.join(".git"));
        assert_eq!(
            git_line_in(&env, &main, &["rev-parse", "--show-toplevel"]),
            main.display().to_string()
        );

        let child = relative_env_run(&env, &main);
        assert_eq!(
            child_result(&child),
            "opened",
            "{}",
            testing::failed_run("the relative-env child", &child)
        );
        assert_eq!(fact(&child, "git_ls_files"), "a.txt,b.txt");
    }

    /// Control: below the root, an absolute `GIT_DIR` with `GIT_WORK_TREE`
    /// naming `main` is the worktree git uses too, and opens.
    #[test]
    fn an_absolute_git_dir_with_git_work_tree_below_the_root_opens() {
        let scratch = Scratch::new("absolute-git-dir-work-tree").expect("scratch");
        let main = two_index_repo(&scratch);
        let sub = main.join("sub");
        let mut env = env_with(&scratch, "GIT_DIR", main.join(".git"));
        env.push(("GIT_WORK_TREE".into(), main.clone().into()));
        assert_eq!(
            git_line_in(&env, &sub, &["rev-parse", "--show-toplevel"]),
            main.display().to_string()
        );

        let child = relative_env_run(&env, &sub);
        assert_eq!(
            child_result(&child),
            "opened",
            "{}",
            testing::failed_run("the relative-env child", &child)
        );
    }

    /// Control: below the root, an absolute `GIT_DIR` whose repository sets
    /// `core.worktree` to `main` is the worktree git uses too, and opens.
    #[test]
    fn an_absolute_git_dir_with_core_worktree_below_the_root_opens() {
        let scratch = Scratch::new("absolute-git-dir-core-worktree").expect("scratch");
        let main = two_index_repo(&scratch);
        let main_arg = main.to_str().expect("UTF-8 scratch path");
        git_ok(&scratch, &main, ["config", "core.worktree", main_arg]).expect("set core.worktree");
        let sub = main.join("sub");
        let env = env_with(&scratch, "GIT_DIR", main.join(".git"));
        assert_eq!(
            git_line_in(&env, &sub, &["rev-parse", "--show-toplevel"]),
            main.display().to_string()
        );

        let child = relative_env_run(&env, &sub);
        assert_eq!(
            child_result(&child),
            "opened",
            "{}",
            testing::failed_run("the relative-env child", &child)
        );
    }

    /// Control: at the worktree root, as in a hook, a relative
    /// `GIT_INDEX_FILE` opens, and wsx's index and its git child both use
    /// `main/alt.idx`, as git does.
    #[test]
    fn a_relative_index_at_the_worktree_root_opens_like_git() {
        let scratch = Scratch::new("relative-index-root").expect("scratch");
        let main = two_index_repo(&scratch);
        let env = env_with(&scratch, "GIT_INDEX_FILE", "alt.idx");
        let from_git = git_line_in(&env, &main, &["ls-files", "--full-name", ":/"]);
        assert_eq!(from_git, "b.txt");

        let child = relative_env_run(&env, &main);
        assert_eq!(
            child_result(&child),
            "opened",
            "{}",
            testing::failed_run("the relative-env child", &child)
        );
        assert_eq!(
            fact(&child, "index"),
            main.join("alt.idx").display().to_string()
        );
        assert_eq!(fact(&child, "git_ls_files"), from_git);
    }

    /// Control: below the worktree root, an absolute `GIT_INDEX_FILE` opens,
    /// and wsx's index and its git child both use that file.
    #[test]
    fn an_absolute_index_below_the_worktree_root_opens() {
        let scratch = Scratch::new("absolute-index-sub").expect("scratch");
        let main = two_index_repo(&scratch);
        let sub = main.join("sub");
        let index = sub.join("alt.idx");
        let env = env_with(&scratch, "GIT_INDEX_FILE", index.clone());

        let child = relative_env_run(&env, &sub);
        assert_eq!(
            child_result(&child),
            "opened",
            "{}",
            testing::failed_run("the relative-env child", &child)
        );
        assert_eq!(fact(&child, "index"), index.display().to_string());
        assert_eq!(fact(&child, "git_ls_files"), "a.txt");
    }

    /// Prints what `Repo::discover` makes of the working directory:
    /// `result=opened` with `index` and the `git ls-files --full-name :/`
    /// a `Git::from_process` child prints in the worktree (`git_ls_files`,
    /// comma-separated, or `git_error`), or `result=refused` with `error`.
    #[test]
    #[ignore = "child half of the relative-env tests; runs only when re-executed"]
    fn relative_env_child() {
        let cwd = std::env::current_dir().expect("read the working directory");
        match Repo::discover(&cwd) {
            Ok(found) => {
                testing::child_print("result", "opened");
                testing::child_print("index", found.index_path().display());
                let worktree = found.worktree().expect("the repository has a worktree");
                match Git::from_process(&found).run(worktree, ["ls-files", "--full-name", ":/"]) {
                    Ok(stdout) => testing::child_print(
                        "git_ls_files",
                        String::from_utf8_lossy(&stdout)
                            .lines()
                            .collect::<Vec<_>>()
                            .join(","),
                    ),
                    Err(err) => {
                        testing::child_print("git_error", err.to_string().replace('\n', "\\n"));
                    }
                }
            }
            Err(err) => {
                testing::child_print("result", "refused");
                testing::child_print("error", err.to_string().replace('\n', "\\n"));
            }
        }
    }
}
