//! Reads of the repository's own config: wsx's keys and the remotes it
//! defines. Each read takes the config gix froze when `Repo::discover` opened
//! the repository, so a value set later is seen only by a new `Repo`.

use gix::bstr::ByteSlice;

use super::repo::Repo;

impl Repo {
    /// Every value of `key` in this repository's own config file (gix's
    /// `Source::Local`, the common dir's `config`), in file order, as raw
    /// bytes. A global, system or `-c` value is never returned, so a value
    /// set for the machine cannot configure this repository (D-01).
    pub fn local_config_values(&self, key: &str) -> Vec<Vec<u8>> {
        self.repo
            .config_snapshot()
            .plumbing()
            .strings_filter(key, |meta| meta.source == gix::config::Source::Local)
            .unwrap_or_default()
            .into_iter()
            .map(Vec::from)
            .collect()
    }

    /// Whether gix lists the remote `name` among this repository's remotes.
    pub fn remote_configured(&self, name: &[u8]) -> bool {
        self.repo
            .remote_names()
            .iter()
            .any(|remote| remote.as_bytes() == name)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Output;

    use super::super::testing::{self, Env, Scratch, T0, commit_all, git_ok, write};
    use super::Repo;

    /// The key every test here reads.
    const KEY: &str = "wsx.sourceBranch";

    /// The variable a parent hands `count_child`: the directory it discovers
    /// from.
    const START_VAR: &str = "WSX_TEST_START";

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
