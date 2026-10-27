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

/// `path` with every symlink and `..` resolved.
fn canonical(path: &Path) -> Result<PathBuf, Error> {
    std::fs::canonicalize(path).map_err(|source| Error::Io {
        what: format!("canonicalize {}", path.display()),
        source,
    })
}

#[cfg(test)]
mod tests {
    use gix::sec::trust::DefaultForLevel;

    use super::super::testing::Scratch;

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
}
