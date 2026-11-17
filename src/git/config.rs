//! Reads of the repository's own config: wsx's keys and the remotes it
//! defines. Each read takes the config gix froze when `Repo::discover` opened
//! the repository, so a value set later is seen only by a new `Repo`.

use std::collections::BTreeSet;
use std::path::Path;

use gix::bstr::ByteSlice;

use super::Error;
use super::repo::Repo;
use super::spawn::Git;

/// What one `git config` child says about this repository: the `wsx.*`
/// values its own config file holds, and the remotes it defines.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigRead {
    /// Each kept `wsx.<var>` record as (lowercased key, raw value), in the
    /// order git printed them.
    values: Vec<(Vec<u8>, Vec<u8>)>,
    /// The names of the remotes this repository defines.
    remotes: BTreeSet<Vec<u8>>,
}

impl ConfigRead {
    /// Every value of `key` (compared lowercased, as git prints keys), in
    /// file order, as raw bytes; empty when the key is unset.
    pub fn values(&self, key: &str) -> Vec<Vec<u8>> {
        let key = key.to_ascii_lowercase();
        self.values
            .iter()
            .filter(|(found, _)| found.as_slice() == key.as_bytes())
            .map(|(_, value)| value.clone())
            .collect()
    }

    /// Whether this repository defines the remote `name`.
    pub fn defines_remote(&self, name: &[u8]) -> bool {
        self.remotes.contains(name)
    }
}

impl Repo {
    /// This repository's `wsx.*` values and the remotes it defines.
    pub fn read_config(&self, _git: &Git) -> Result<ConfigRead, Error> {
        Ok(ConfigRead::default())
    }
    /// Every value of `key` in this repository's own config file (gix's
    /// `Source::Local` at include level 0, the common dir's `config`), in
    /// file order, as raw bytes. Nothing else configures the repository
    /// (D-01): a global, system, `config.worktree` or `GIT_CONFIG_COUNT`
    /// value is never returned, and neither is one from an `include.path`
    /// file, which gix reads as `Source::Local` at level 1 and which a
    /// tracked include would read live from whatever branch is checked out.
    ///
    /// Measured with git 2.56.0 and gix-config 0.62.2, the result equals
    /// `git config --local --get-all`: no values where git exits 1 (an
    /// included, global or `config.worktree` value, which `--includes` or
    /// plain `git config --get` does return, and a `[wsx "sub"]` one),
    /// every value of a repeated key in order, across sections too, one
    /// empty value for `key =` and for a key line with no `=`, quotes and
    /// comments normalised as git does, and non-UTF-8 bytes unchanged. A
    /// linked worktree reads the common dir's file. `string_filter`, the
    /// call `ref_storage` makes, is not used: it returns only the last value
    /// of a repeated key and reads a key with no `=` as absent, which hides
    /// both cases D-18 refuses.
    pub fn local_config_values(&self, key: &str) -> Vec<Vec<u8>> {
        self.repo
            .config_snapshot()
            .plumbing()
            .strings_filter(key, |meta| {
                meta.source == gix::config::Source::Local && meta.level == 0
            })
            .unwrap_or_default()
            .into_iter()
            .map(Vec::from)
            .collect()
    }

    /// Whether this repository defines the remote `name`: a `[remote "name"]`
    /// section with at least one key, read from the repository's own config
    /// (gix's `Source::Local`) or its `config.worktree` (`Source::Worktree`).
    /// This is the test `git remote add` makes: it refuses a name, exit 3,
    /// only when the repository already defines it (D-03, D-04, A2).
    ///
    /// Unlike `local_config_values`, the include level is not filtered: a
    /// section from an `include.path` file counts, because `git remote add`
    /// refuses its name too.
    ///
    /// Measured with git 2.56.0 and gix 0.90.0 against `git remote add`:
    /// a section only in global config (`git remote` lists it, `git remote
    /// add` succeeds) and a local section with no keys (gix's
    /// `remote_names()` lists it, `git remote add` succeeds) are not remotes;
    /// a local, included or `config.worktree` section with a key is
    /// (`git remote add` exits 3), with or without a `url`. A remote's url is
    /// never checked, so one whose url is dead still counts.
    pub fn remote_configured(&self, name: &[u8]) -> bool {
        let snapshot = self.repo.config_snapshot();
        let Some(mut sections) = snapshot.plumbing().sections_by_name("remote") else {
            return false;
        };
        let in_this_repo = |meta: &gix::config::file::Metadata| {
            matches!(
                meta.source,
                gix::config::Source::Local | gix::config::Source::Worktree
            )
        };
        sections.any(|section| {
            in_this_repo(section.meta())
                && section.value_names().next().is_some()
                && section.header().subsection_name() == Some(name.as_bstr())
        })
    }
}

/// The records of `git config -z --show-scope --show-origin` output.
fn parse_config_z(_stdout: &[u8], _cwd: &Path, _own_config: &Path) -> Result<ConfigRead, String> {
    Ok(ConfigRead::default())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::process::Output;

    use gix::bstr::ByteSlice;

    use super::super::Error;
    use super::super::spawn::Git;
    use super::super::testing::{self, Env, Scratch, T0, commit_all, git_ok, write};
    use super::{ConfigRead, Repo, parse_config_z};

    /// The key every test here reads.
    const KEY: &str = "wsx.sourceBranch";

    /// The variable a parent hands `count_child` and `remote_child`: the
    /// directory it discovers from.
    const START_VAR: &str = "WSX_TEST_START";

    /// The variable a parent hands `remote_child`: the remote name it asks
    /// `remote_configured` about.
    const REMOTE_VAR: &str = "WSX_TEST_REMOTE";

    /// The full libtest path of a child half in this module.
    fn child_path(name: &str) -> String {
        format!("git::config::tests::{name}")
    }

    /// A one-commit repository at `<root>/<name>` on `main`, canonical.
    fn one_commit_repo(scratch: &Scratch, name: &str) -> PathBuf {
        let dir = scratch.root.join(name);
        std::fs::create_dir_all(&dir).unwrap_or_else(|err| panic!("create {name}: {err}"));
        git_ok(scratch, &dir, ["init", "-q", "-b", "main"]).unwrap_or_else(|err| panic!("{err}"));
        write(&dir.join("a.txt"), b"a\n").unwrap_or_else(|err| panic!("{err}"));
        commit_all(scratch, &dir, name, T0).unwrap_or_else(|err| panic!("{err}"));
        dir.canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize {name}: {err}"))
    }

    /// Appends `text` to `<repo>/.git/config`.
    fn append_config(repo: &Path, text: &[u8]) {
        let path = repo.join(".git/config");
        let mut config =
            std::fs::read(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        config.extend_from_slice(text);
        write(&path, &config).unwrap_or_else(|err| panic!("{err}"));
    }

    /// git in `repo` with the scratch environment, panicking only when git
    /// cannot be started.
    fn git(scratch: &Scratch, repo: &Path, args: &[&str]) -> Output {
        testing::git(scratch, repo, args).unwrap_or_else(|err| panic!("{err}"))
    }

    /// The values a `git config --get` or `--get-all` run printed, one per
    /// line, after checking it exited 0.
    fn printed_values(what: &str, output: &Output) -> Vec<Vec<u8>> {
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run(what, output)
        );
        let stdout = output
            .stdout
            .strip_suffix(b"\n")
            .unwrap_or_else(|| panic!("{what} printed no newline: {:?}", output.stdout));
        stdout
            .split(|byte| *byte == b'\n')
            .map(<[u8]>::to_vec)
            .collect()
    }

    /// Asserts `git config <args>` exits 1, git's answer for a key it does
    /// not find.
    fn assert_git_finds_nothing(scratch: &Scratch, repo: &Path, args: &[&str]) {
        let output = git(scratch, repo, args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            testing::failed_run(&format!("git {}", args.join(" ")), &output)
        );
    }

    /// The values of `key` that `Repo::discover(repo)` reads through
    /// `local_config_values`.
    fn local_values(repo: &Path, key: &str) -> Vec<Vec<u8>> {
        Repo::discover(repo)
            .unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()))
            .local_config_values(key)
    }

    /// The values of `KEY` gix loaded from `source`, at any include level:
    /// the control that gix did read the value `local_config_values` leaves
    /// out.
    fn values_from(repo: &Path, source: gix::config::Source) -> Vec<Vec<u8>> {
        let found =
            Repo::discover(repo).unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()));
        found
            .repo
            .config_snapshot()
            .plumbing()
            .strings_filter(KEY, |meta| meta.source == source)
            .unwrap_or_default()
            .into_iter()
            .map(Vec::from)
            .collect()
    }

    /// The scratch environment plus `START_VAR=<repo>`.
    fn child_env(scratch: &Scratch, repo: &Path) -> Env {
        let mut env = scratch.env.clone();
        env.push((START_VAR.into(), repo.into()));
        env
    }

    /// Runs `count_child` with `env` and returns its `count` and `all`
    /// markers, failing with the child's output when it printed `error`.
    fn child_counts(scratch: &Scratch, env: &Env) -> (String, String) {
        let child = testing::reexec(&child_path("count_child"), env, &scratch.root, b"")
            .unwrap_or_else(|err| panic!("{err}"));
        let fact = |key: &str| testing::marker(&child, key).unwrap_or_else(|err| panic!("{err}"));
        (fact("count"), fact("all"))
    }

    /// A value in an `include.path` file is not this repository's own config:
    /// `git config --local` sees it only with `--includes`, and a tracked
    /// include would be read live from whatever branch is checked out.
    #[test]
    fn included_key_is_not_read() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-include").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        write(
            &repo.join(".git/extra.conf"),
            b"[wsx]\n\tsourceBranch = workspace/dev\n",
        )
        .unwrap_or_else(|err| panic!("{err}"));
        append_config(&repo, b"[include]\n\tpath = extra.conf\n");

        let included = git(
            &scratch,
            &repo,
            &["config", "--local", "--includes", "--get", KEY],
        );
        assert_eq!(
            printed_values("git config --local --includes --get", &included),
            [b"workspace/dev".to_vec()]
        );
        assert_git_finds_nothing(&scratch, &repo, &["config", "--local", "--get", KEY]);
        assert_eq!(
            values_from(&repo, gix::config::Source::Local),
            [b"workspace/dev".to_vec()],
            "gix did not load the included file, so this test shows nothing"
        );
        assert_eq!(local_values(&repo, KEY), Vec::<Vec<u8>>::new());
    }

    /// A value in global config does not configure the repository: plain
    /// `git config --get` returns it, `--local --get` does not, and gix in a
    /// child with the scratch environment loads it but does not return it.
    #[test]
    fn global_key_is_not_read() {
        let scratch = Scratch::new("config-global").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        let global = scratch.home.join(".gitconfig");
        let mut config = std::fs::read(&global).expect("read the scratch global config");
        config.extend_from_slice(b"[wsx]\n\tsourceBranch = workspace/dev\n");
        write(&global, &config).expect("append wsx.sourceBranch");

        let plain = git(&scratch, &repo, &["config", "--get", KEY]);
        assert_eq!(
            printed_values("git config --get", &plain),
            [b"workspace/dev".to_vec()]
        );
        assert_git_finds_nothing(&scratch, &repo, &["config", "--local", "--get", KEY]);
        assert_eq!(
            child_counts(&scratch, &child_env(&scratch, &repo)),
            ("0".to_owned(), "1".to_owned())
        );
    }

    /// The control for the child: the same child returns a value set in the
    /// repository's own config.
    #[test]
    fn local_key_is_read_by_the_same_child() {
        let scratch = Scratch::new("config-local-child").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        git_ok(&scratch, &repo, ["config", KEY, "workspace/dev"])
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            child_counts(&scratch, &child_env(&scratch, &repo)),
            ("1".to_owned(), "1".to_owned())
        );
    }

    /// A value in the worktree's `config.worktree` is not the repository's
    /// own config file: plain `git config --get` returns it, `--local --get`
    /// does not.
    #[test]
    fn worktree_config_key_is_not_read() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-worktree").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        for args in [
            ["config", "core.repositoryformatversion", "1"].as_slice(),
            &["config", "extensions.worktreeConfig", "true"],
            &["config", "--worktree", KEY, "workspace/dev"],
        ] {
            git_ok(&scratch, &repo, args).unwrap_or_else(|err| panic!("{err}"));
        }

        let plain = git(&scratch, &repo, &["config", "--get", KEY]);
        assert_eq!(
            printed_values("git config --get", &plain),
            [b"workspace/dev".to_vec()]
        );
        assert_git_finds_nothing(&scratch, &repo, &["config", "--local", "--get", KEY]);
        assert_eq!(
            values_from(&repo, gix::config::Source::Worktree),
            [b"workspace/dev".to_vec()],
            "gix did not load config.worktree, so this test shows nothing"
        );
        assert_eq!(local_values(&repo, KEY), Vec::<Vec<u8>>::new());
    }

    /// A value given through `GIT_CONFIG_COUNT` is not the repository's own
    /// config: git returns it under that environment, and gix in a child with
    /// the same environment loads it but does not return it.
    #[test]
    fn command_line_config_key_is_not_read() {
        let scratch = Scratch::new("config-env").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        let mut env = child_env(&scratch, &repo);
        env.push(("GIT_CONFIG_COUNT".into(), "1".into()));
        env.push(("GIT_CONFIG_KEY_0".into(), KEY.into()));
        env.push(("GIT_CONFIG_VALUE_0".into(), "workspace/dev".into()));

        let plain = testing::git_in_env(&env, &repo, ["config", "--get", KEY])
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            printed_values("git config --get under GIT_CONFIG_COUNT", &plain),
            [b"workspace/dev".to_vec()]
        );
        assert_eq!(
            child_counts(&scratch, &env),
            ("0".to_owned(), "1".to_owned())
        );
    }

    /// A linked worktree reads the common dir's `config`, the file every
    /// worktree of the repository shares, as `git config --local` does there.
    #[test]
    fn linked_worktree_reads_the_common_config() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-linked").expect("scratch");
        let trees = testing::linked_worktrees(&scratch).unwrap_or_else(|err| panic!("{err}"));
        git_ok(&scratch, &trees.main, ["config", KEY, "workspace/dev"])
            .unwrap_or_else(|err| panic!("{err}"));

        let from_wt1 = git(&scratch, &trees.wt1, &["config", "--local", "--get", KEY]);
        let git_values = printed_values("git -C wt1 config --local --get", &from_wt1);
        assert_eq!(git_values, [b"workspace/dev".to_vec()]);
        assert_eq!(local_values(&trees.wt1, KEY), git_values);
    }

    /// Every value of a repeated key, in file order, as `--get-all` prints
    /// them, so the model can tell a repeated key from a single one.
    #[test]
    fn repeated_key_returns_every_value() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-repeated").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        for value in ["workspace/a", "workspace/b"] {
            git_ok(&scratch, &repo, ["config", "--local", "--add", KEY, value])
                .unwrap_or_else(|err| panic!("{err}"));
        }

        let all = git(&scratch, &repo, &["config", "--local", "--get-all", KEY]);
        let git_values = printed_values("git config --local --get-all", &all);
        assert_eq!(
            git_values,
            [b"workspace/a".to_vec(), b"workspace/b".to_vec()]
        );
        assert_eq!(local_values(&repo, KEY), git_values);
    }

    /// The same for a key set once in each of two `[wsx]` sections.
    #[test]
    fn key_repeated_across_sections_returns_both() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-sections").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        append_config(
            &repo,
            b"[wsx]\n\tsourceBranch = workspace/a\n[wsx]\n\tsourceBranch = workspace/b\n",
        );

        let all = git(&scratch, &repo, &["config", "--local", "--get-all", KEY]);
        let git_values = printed_values("git config --local --get-all", &all);
        assert_eq!(
            git_values,
            [b"workspace/a".to_vec(), b"workspace/b".to_vec()]
        );
        assert_eq!(local_values(&repo, KEY), git_values);
    }

    /// A key line with no `=` is one empty value, as git prints it, not an
    /// absent key.
    #[test]
    fn key_without_a_value_reads_as_one_empty_value() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-no-value").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        append_config(&repo, b"[wsx]\n\tsourceBranch\n");

        let get = git(&scratch, &repo, &["config", "--local", "--get", KEY]);
        assert_eq!(
            get.stdout,
            b"\n",
            "{}",
            testing::failed_run("git config", &get)
        );
        let git_values = printed_values("git config --local --get", &get);
        assert_eq!(git_values, [Vec::<u8>::new()]);
        assert_eq!(local_values(&repo, KEY), git_values);
    }

    /// An unset key reads as no values, as git's exit 1 says.
    #[test]
    fn absent_key_reads_as_no_values() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-absent").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        assert_git_finds_nothing(&scratch, &repo, &["config", "--local", "--get", KEY]);
        assert_eq!(local_values(&repo, KEY), Vec::<Vec<u8>>::new());
    }

    /// Quotes and comments are read as git reads them: a quoted `#` is part
    /// of the value, an unquoted one starts a comment.
    #[test]
    fn value_is_normalised_as_git_does() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-normalised").expect("scratch");
        for (name, line, expected) in [
            (
                "quoted",
                b"\tsourceBranch = \"a b # c\"\n".as_slice(),
                b"a b # c".as_slice(),
            ),
            ("comment", b"\tsourceBranch = v # comment\n", b"v"),
        ] {
            let repo = one_commit_repo(&scratch, name);
            let mut text = b"[wsx]\n".to_vec();
            text.extend_from_slice(line);
            append_config(&repo, &text);

            let get = git(&scratch, &repo, &["config", "--local", "--get", KEY]);
            let git_values = printed_values("git config --local --get", &get);
            assert_eq!(git_values, [expected.to_vec()], "{name}");
            assert_eq!(local_values(&repo, KEY), git_values, "{name}");
        }
    }

    /// A value that is not UTF-8 comes back as git prints it, byte for byte.
    #[test]
    fn non_utf8_value_is_returned_raw() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-non-utf8").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        append_config(&repo, b"[wsx]\n\tsourceBranch = caf\xe9\n");

        let get = git(&scratch, &repo, &["config", "--local", "--get", KEY]);
        let git_values = printed_values("git config --local --get", &get);
        assert_eq!(git_values, [b"caf\xe9".to_vec()]);
        assert_eq!(local_values(&repo, KEY), git_values);
    }

    /// `[wsx "sub"] sourceBranch` is the key `wsx.sub.sourceBranch`, not
    /// `wsx.sourceBranch`; the subsection key itself is read, as git reads it.
    #[test]
    fn subsection_key_is_not_read() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-subsection").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        append_config(&repo, b"[wsx \"sub\"]\n\tsourceBranch = x\n");

        assert_git_finds_nothing(&scratch, &repo, &["config", "--local", "--get", KEY]);
        assert_eq!(local_values(&repo, KEY), Vec::<Vec<u8>>::new());
        let sub = git(
            &scratch,
            &repo,
            &["config", "--local", "--get", "wsx.sub.sourceBranch"],
        );
        let git_values = printed_values("git config --local --get wsx.sub.sourceBranch", &sub);
        assert_eq!(git_values, [b"x".to_vec()]);
        assert_eq!(local_values(&repo, "wsx.sub.sourceBranch"), git_values);
    }

    /// Asserts `git <args>` in `repo` exits `code`.
    fn assert_git_exits(scratch: &Scratch, repo: &Path, args: &[&str], code: i32) {
        let output = git(scratch, repo, args);
        assert_eq!(
            output.status.code(),
            Some(code),
            "{}",
            testing::failed_run(&format!("git {}", args.join(" ")), &output)
        );
    }

    /// The names `git remote` lists in `repo`.
    fn git_remotes(scratch: &Scratch, repo: &Path) -> Vec<Vec<u8>> {
        let output = git(scratch, repo, &["remote"]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run("git remote", &output)
        );
        output
            .stdout
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(<[u8]>::to_vec)
            .collect()
    }

    /// `remote_configured(name)` on `Repo::discover(repo)`.
    fn configured(repo: &Path, name: &[u8]) -> bool {
        Repo::discover(repo)
            .unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()))
            .remote_configured(name)
    }

    /// Whether gix's own `remote_names()` lists `name`: the control that a
    /// section gix counts as a remote is one `remote_configured` refuses.
    fn gix_lists_remote(repo: &Path, name: &[u8]) -> bool {
        let found =
            Repo::discover(repo).unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()));
        found
            .repo
            .remote_names()
            .iter()
            .any(|remote| remote.as_bytes() == name)
    }

    /// Runs `remote_child` on `repo` for the remote `name` with the scratch
    /// environment and returns its `configured` marker.
    fn child_configured(scratch: &Scratch, repo: &Path, name: &str) -> String {
        let mut env = child_env(scratch, repo);
        env.push((REMOTE_VAR.into(), name.into()));
        let child = testing::reexec(&child_path("remote_child"), &env, &scratch.root, b"")
            .unwrap_or_else(|err| panic!("{err}"));
        testing::marker(&child, "configured").unwrap_or_else(|err| panic!("{err}"))
    }

    /// A remote section in global config is not a remote of this
    /// repository: `git remote` lists it, but `git remote add` treats the
    /// name as free, and a child with the scratch environment does not count
    /// it. The child then counts the section `git remote add` wrote, the
    /// control that it can answer `true` for the same name.
    #[test]
    fn global_remote_section_is_not_a_remote() {
        let scratch = Scratch::new("config-remote-global").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        let global = scratch.home.join(".gitconfig");
        let mut config = std::fs::read(&global).expect("read the scratch global config");
        config.extend_from_slice(b"[remote \"x\"]\n\tprune = true\n");
        write(&global, &config).expect("append remote.x.prune");

        assert!(
            git_remotes(&scratch, &repo).contains(&b"x".to_vec()),
            "git remote does not list the global section, so this test shows nothing"
        );
        assert_eq!(child_configured(&scratch, &repo, "x"), "false");
        // Last: it writes the repository's config.
        assert_git_exits(&scratch, &repo, &["remote", "add", "x", "/y"], 0);
        assert_eq!(child_configured(&scratch, &repo, "x"), "true");
    }

    /// A local `[remote "empty"]` with no keys is not a remote: gix lists it,
    /// `git remote add` treats the name as free.
    #[test]
    fn keyless_local_remote_section_is_not_a_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-remote-keyless").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        append_config(&repo, b"[remote \"empty\"]\n");

        assert!(
            gix_lists_remote(&repo, b"empty"),
            "gix does not list the keyless section, so this test shows nothing"
        );
        assert!(
            !configured(&repo, b"empty"),
            "remote_configured counts a local remote section with no keys"
        );
        // Last: it writes the repository's config.
        assert_git_exits(&scratch, &repo, &["remote", "add", "empty", "/y"], 0);
    }

    /// A remote `git remote add` wrote is a remote, and a second
    /// `git remote add` of the name refuses with exit 3.
    #[test]
    fn local_remote_is_a_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-remote-local").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        git_ok(&scratch, &repo, ["remote", "add", "origin", "/x"])
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(configured(&repo, b"origin"));
        assert_git_exits(&scratch, &repo, &["remote", "add", "origin", "/y"], 3);
    }

    /// A local remote section with a key but no `url` is a remote, and
    /// `git remote` lists it.
    #[test]
    fn local_remote_without_a_url_is_a_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-remote-no-url").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        append_config(&repo, b"[remote \"bare\"]\n\tprune = true\n");

        assert!(configured(&repo, b"bare"));
        assert!(
            git_remotes(&scratch, &repo).contains(&b"bare".to_vec()),
            "git remote does not list the section"
        );
    }

    /// A remote section from an `include.path` file is a remote:
    /// `git remote add` refuses the name with exit 3, though
    /// `git config --local` without `--includes` does not see its url.
    #[test]
    fn included_remote_is_a_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-remote-include").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        write(
            &repo.join(".git/extra.conf"),
            b"[remote \"inc\"]\n\turl = /x\n",
        )
        .unwrap_or_else(|err| panic!("{err}"));
        append_config(&repo, b"[include]\n\tpath = extra.conf\n");

        assert_git_finds_nothing(
            &scratch,
            &repo,
            &["config", "--local", "--get", "remote.inc.url"],
        );
        assert!(configured(&repo, b"inc"));
        assert_git_exits(&scratch, &repo, &["remote", "add", "inc", "/y"], 3);
    }

    /// A remote section in `config.worktree` is a remote: `git remote add`
    /// refuses the name with exit 3.
    #[test]
    fn worktree_config_remote_is_a_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-remote-worktree").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        for args in [
            ["config", "core.repositoryformatversion", "1"].as_slice(),
            &["config", "extensions.worktreeConfig", "true"],
            &["config", "--worktree", "remote.wt.url", "/x"],
        ] {
            git_ok(&scratch, &repo, args).unwrap_or_else(|err| panic!("{err}"));
        }

        assert_git_finds_nothing(
            &scratch,
            &repo,
            &["config", "--local", "--get", "remote.wt.url"],
        );
        assert!(configured(&repo, b"wt"));
        assert_git_exits(&scratch, &repo, &["remote", "add", "wt", "/y"], 3);
    }

    /// A name no section defines is not a remote, in a repository that has
    /// another remote, which is.
    #[test]
    fn undefined_remote_is_not_a_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-remote-undefined").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        git_ok(&scratch, &repo, ["remote", "add", "origin", "/x"])
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(!configured(&repo, b"nope"));
        assert!(configured(&repo, b"origin"), "the control remote");
    }

    /// One `-z` record: scope, origin and entry, each ended by NUL.
    fn record(scope: &[u8], origin: &[u8], entry: &[u8]) -> Vec<u8> {
        [scope, b"\0", origin, b"\0", entry, b"\0"].concat()
    }

    /// `file:<path>`, the origin git prints for a config file.
    fn file_origin(path: &Path) -> Vec<u8> {
        [b"file:".as_slice(), path.as_os_str().as_bytes()].concat()
    }

    /// `parse_config_z` of `stdout` as read from `repo`, whose own config
    /// is `<repo>/.git/config`.
    fn parsed(repo: &Path, stdout: &[u8]) -> Result<ConfigRead, String> {
        let own = repo
            .join(".git/config")
            .canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize {}/.git/config: {err}", repo.display()));
        parse_config_z(stdout, repo, &own)
    }

    /// `parsed`, failing the test on `Err`.
    fn parsed_ok(repo: &Path, stdout: &[u8]) -> ConfigRead {
        parsed(repo, stdout).unwrap_or_else(|err| panic!("parse_config_z: {err}"))
    }

    /// A `wsx` key counts only from the repository's own file at scope
    /// `local`; a remote counts from any `local` or `worktree` record,
    /// included files too, and never from global config.
    #[test]
    fn parse_config_z_keeps_only_the_own_local_file() {
        let scratch = Scratch::new("config-parse-own").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        let inc = repo.join("inc.conf");
        write(&inc, b"").unwrap_or_else(|err| panic!("{err}"));
        let inc = file_origin(&inc);
        let global = file_origin(&scratch.home.join(".gitconfig"));
        let own: &[u8] = b"file:.git/config";
        let worktree: &[u8] = b"file:.git/config.worktree";
        let stdout = [
            record(b"local", own, b"wsx.sourcebranch\ndev"),
            record(b"local", &inc, b"wsx.sourcebranch\ninc"),
            record(b"worktree", worktree, b"wsx.sourcebranch\nwt"),
            record(b"global", &global, b"wsx.sourcebranch\ng"),
            record(b"command", b"command line:", b"wsx.sourcebranch\nc"),
            record(b"local", own, b"remote.origin.url\nu"),
            record(b"worktree", worktree, b"remote.wt.url\nu"),
            record(b"local", &inc, b"remote.inc.url\nu"),
            record(b"global", &global, b"remote.g.url\nu"),
        ]
        .concat();

        let read = parsed_ok(&repo, &stdout);
        assert_eq!(read.values(KEY), [b"dev".to_vec()]);
        assert_eq!(
            [b"origin".as_slice(), b"wt", b"inc", b"g"].map(|name| read.defines_remote(name)),
            [true, true, true, false]
        );
    }

    /// A key printed with no LF (`key` with no `=`) and one printed as
    /// `key LF` (`key =`) are each one empty value.
    #[test]
    fn parse_config_z_bare_key_is_one_empty_value() {
        let scratch = Scratch::new("config-parse-bare").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        for entry in [b"wsx.sourcebranch".as_slice(), b"wsx.sourcebranch\n"] {
            let stdout = record(b"local", b"file:.git/config", entry);
            assert_eq!(
                parsed_ok(&repo, &stdout).values(KEY),
                [Vec::<u8>::new()],
                "{}",
                entry.escape_ascii()
            );
        }
    }

    /// Output that stops inside a record is refused, never read as fewer
    /// keys.
    #[test]
    fn parse_config_z_refuses_a_truncated_record() {
        let scratch = Scratch::new("config-parse-truncated").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        let mut stdout = record(b"local", b"file:.git/config", b"wsx.sourcebranch\ndev");
        stdout.extend_from_slice(b"local\0file:.git/config\0");

        match parsed(&repo, &stdout) {
            Err(problem) => assert!(
                problem.contains("record 2") && problem.contains("ends after its origin"),
                "the problem does not name the incomplete record: {problem}"
            ),
            other => panic!("expected Err for a truncated record, got {other:?}"),
        }
    }

    /// A remote name is everything between `remote.` and the last dot, so it
    /// may hold dots; a two-part `remote.*` key names no remote.
    #[test]
    fn parse_config_z_names_a_remote_up_to_the_last_dot() {
        let scratch = Scratch::new("config-parse-dots").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        let stdout = [
            record(b"local", b"file:.git/config", b"remote.a.b.url\nu"),
            record(b"local", b"file:.git/config", b"remote.pushdefault\nx"),
        ]
        .concat();

        let read = parsed_ok(&repo, &stdout);
        assert!(read.defines_remote(b"a.b"));
        assert!(!read.defines_remote(b"a"));
        assert!(!read.defines_remote(b"pushdefault"));
        assert!(!read.defines_remote(b""));
    }

    /// The `--get-regexp` the read runs, for the git-side assertions.
    const READ_ARGS: [&str; 3] = ["config", "--get-regexp", "^(wsx|remote)\\."];

    /// git's exit 1 for "no key matched" is an empty read, not a failure; a
    /// remote set afterwards on the same `Repo` is seen by the next read.
    #[test]
    fn config_read_with_no_keys_is_empty() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-read-empty").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        assert_git_exits(&scratch, &repo, &READ_ARGS, 1);
        let found = Repo::discover(&repo)
            .unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()));
        let helper = Git::for_repo(scratch.env.clone(), &found);

        let read = found
            .read_config(&helper)
            .unwrap_or_else(|err| panic!("read_config: {err}"));
        assert_eq!(read.values(KEY), Vec::<Vec<u8>>::new());
        assert!(!read.defines_remote(b"origin"));

        let elsewhere = scratch.root.join("elsewhere");
        git_ok(
            &scratch,
            &repo,
            [
                OsStr::new("config"),
                OsStr::new("remote.origin.url"),
                elsewhere.as_os_str(),
            ],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let listed = git(&scratch, &repo, &READ_ARGS);
        assert_eq!(
            listed.status.code(),
            Some(0),
            "{}",
            testing::failed_run("git config --get-regexp", &listed)
        );
        assert!(
            listed.stdout.starts_with(b"remote.origin.url "),
            "git config --get-regexp printed {:?}",
            listed.stdout.as_bstr()
        );
        let read = found
            .read_config(&helper)
            .unwrap_or_else(|err| panic!("read_config: {err}"));
        assert!(
            read.defines_remote(b"origin"),
            "control: git lists remote.origin.url but read_config defines no origin"
        );
    }

    /// A config file git cannot parse fails the read with git's own failure,
    /// never an empty answer.
    #[test]
    fn config_read_of_a_malformed_file_is_git_failed() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-read-malformed").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        git_ok(&scratch, &repo, ["config", KEY, "dev"]).unwrap_or_else(|err| panic!("{err}"));
        let found = Repo::discover(&repo)
            .unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()));
        append_config(&repo, b"[bad\n");
        assert_git_exits(&scratch, &repo, &["config", "--get-all", KEY], 128);

        match found.read_config(&Git::for_repo(scratch.env.clone(), &found)) {
            Err(Error::GitFailed {
                status: Some(128), ..
            }) => {}
            other => panic!("expected Err(GitFailed) with status 128, got {other:?}"),
        }
    }

    /// Prints `configured`, what `remote_configured` answers for the name in
    /// `REMOTE_VAR` on `Repo::discover` of the directory in `START_VAR`; or
    /// `error` when discovery fails.
    #[test]
    #[ignore = "child half of the remote definition tests; runs only when re-executed"]
    fn remote_child() {
        let start = std::env::var_os(START_VAR).expect("the parent set the start directory");
        let name = std::env::var_os(REMOTE_VAR).expect("the parent set the remote name");
        match Repo::discover(Path::new(&start)) {
            Ok(found) => {
                testing::child_print(
                    "configured",
                    found.remote_configured(name.as_encoded_bytes()),
                );
            }
            Err(err) => testing::child_print("error", err.to_string().replace('\n', "\\n")),
        }
    }

    /// Prints what `Repo::discover` of the directory in `START_VAR` reads
    /// for `KEY`: `count`, the number of values `local_config_values`
    /// returns, and `all`, the number gix loaded from any source; or
    /// `error` when discovery fails.
    #[test]
    #[ignore = "child half of the config scope tests; runs only when re-executed"]
    fn count_child() {
        let start = std::env::var_os(START_VAR).expect("the parent set the start directory");
        match Repo::discover(Path::new(&start)) {
            Ok(found) => {
                testing::child_print("count", found.local_config_values(KEY).len());
                let all = found
                    .repo
                    .config_snapshot()
                    .plumbing()
                    .strings(KEY)
                    .map_or(0, |values| values.len());
                testing::child_print("all", all);
            }
            Err(err) => testing::child_print("error", err.to_string().replace('\n', "\\n")),
        }
    }
}
