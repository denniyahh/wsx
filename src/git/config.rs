//! Reads of the repository's own config: wsx's keys and the remotes it
//! defines. Both come from one `git config` child per call, filtered to this
//! repository's own file (keys) or its local and worktree scopes (remotes),
//! so what wsx reads is what git reads, as fresh as the call.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

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
    /// This repository's `wsx.*` values and the remotes it defines, from one
    /// `git config -z --show-scope --show-origin --get-regexp '^(wsx|remote)\.'`
    /// child run through `git` in the worktree root (the git dir of a bare
    /// repository).
    ///
    /// A `wsx.<var>` value is kept only from a record of scope `local` whose
    /// origin is the common dir's `config`, the repository's own file: a
    /// global, system, `include.path`, `config.worktree` or command-line
    /// value never configures the repository or names its workspace branch
    /// (D-01). Every kept value stays, raw and in file order, so a repeated
    /// key is still seen as repeated (D-18). A remote is defined by any
    /// `remote.<name>.<var>` record of scope `local` or `worktree`, from any
    /// origin, includes too: the set `git remote add` refuses with exit 3
    /// (D-36, A2).
    ///
    /// git's exit 1 with nothing on stdout or stderr means no key matched and
    /// is an empty read; any other failure is `Error::GitFailed`, and output that does
    /// not parse is `Error::GitBadOutput`, never fewer keys (CONF-07). The
    /// read is as fresh as the call: unlike gix's config snapshot, frozen when
    /// `Repo::discover` opened the repository, a value set later is seen.
    ///
    /// The child runs without `GIT_CONFIG` even in wsx's own worktree:
    /// `git config` alone honours it and then reads only that file, as scope
    /// `command`, so a configured repository would read as not configured
    /// while `git push`, `git remote add` and every other command read the
    /// repository's config (04-17, the operator).
    pub fn read_config(&self, git: &Git) -> Result<ConfigRead, Error> {
        let cwd = self.worktree().unwrap_or(self.git_dir());
        let output = git.run_without_var(cwd, READ_ARGS, "GIT_CONFIG")?;
        let nothing_matched =
            output.status.code() == Some(1) && output.stderr.is_empty() && output.stdout.is_empty();
        if nothing_matched {
            return Ok(ConfigRead::default());
        }
        if !output.status.success() {
            return Err(Error::GitFailed {
                argv: READ_ARGS.iter().map(OsString::from).collect(),
                status: output.status.code(),
                stderr: output.stderr,
            });
        }
        let stdout = output.stdout;
        let own = self.common_dir().join("config");
        // No file of its own (git works without one) means no record is the
        // repository's own, so every `wsx` key is unset.
        let own_config = match std::fs::canonicalize(&own) {
            Ok(path) => Some(path),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(Error::Io {
                    what: format!("canonicalize {}", own.display()),
                    source,
                });
            }
        };
        parse_config_z(&stdout, cwd, own_config.as_deref()).map_err(|problem| Error::GitBadOutput {
            argv: READ_ARGS.iter().map(OsString::from).collect(),
            problem,
        })
    }
}

/// The argv of the one config read: every `wsx.*` and `remote.*` record
/// from every source, NUL-separated, each with its scope and origin.
const READ_ARGS: [&str; 6] = [
    "config",
    "-z",
    "--show-scope",
    "--show-origin",
    "--get-regexp",
    r"^(wsx|remote)\.",
];

/// The `ConfigRead` of `git config -z --show-scope --show-origin` output
/// read in `cwd`, `own_config` being the repository's own file, canonical,
/// or `None` when it has none.
/// Each record is `scope NUL origin NUL key [LF value] NUL`; a key with no LF
/// is one empty value. Records `read_config` does not keep are skipped. A
/// last record that is not a full triple, or output not ending in NUL, is
/// `Err` naming the problem.
fn parse_config_z(
    stdout: &[u8],
    cwd: &Path,
    own_config: Option<&Path>,
) -> Result<ConfigRead, String> {
    let Some(body) = stdout.strip_suffix(b"\0") else {
        return Err("the output does not end in NUL, so its last record is incomplete".to_owned());
    };
    let fields: Vec<&[u8]> = body.split(|byte| *byte == 0).collect();
    let (records, partial) = fields.as_chunks::<3>();
    if !partial.is_empty() {
        let last = if partial.len() == 1 {
            "scope"
        } else {
            "origin"
        };
        return Err(format!(
            "record {} ends after its {last}",
            records.len() + 1
        ));
    }
    let mut read = ConfigRead::default();
    for (index, &[scope, origin, entry]) in records.iter().enumerate() {
        let number = index + 1;
        if !GIT_SCOPES.contains(&scope) {
            return Err(format!(
                "record {number} has scope {}, which git does not print",
                scope.escape_ascii()
            ));
        }
        let (key, value) = match entry.iter().position(|byte| *byte == b'\n') {
            Some(at) => (&entry[..at], &entry[at + 1..]),
            None => (entry, &b""[..]),
        };
        if let Some(var) = key.strip_prefix(b"wsx.") {
            if !var.contains(&b'.') && scope == b"local" {
                let own = is_own_file(origin, cwd, own_config).ok_or_else(|| {
                    format!(
                        "record {number} has origin {}, not a config file that can be resolved",
                        origin.escape_ascii()
                    )
                })?;
                if own {
                    read.values.push((key.to_vec(), value.to_vec()));
                }
            }
        } else if let Some(name) = remote_name(key)
            && (scope == b"local" || scope == b"worktree")
        {
            read.remotes.insert(name.to_vec());
        }
    }
    Ok(read)
}

/// The remote a `remote.<name>.<var>` key names: everything between
/// `remote.` and the last dot, so a name may hold dots. A two-part key such
/// as `remote.pushdefault` names none.
fn remote_name(key: &[u8]) -> Option<&[u8]> {
    let rest = key.strip_prefix(b"remote.")?;
    let dot = rest.iter().rposition(|byte| *byte == b'.')?;
    Some(&rest[..dot]).filter(|name| !name.is_empty())
}

/// The scopes `git config --show-scope` prints.
const GIT_SCOPES: [&[u8]; 7] = [
    b"system",
    b"global",
    b"local",
    b"worktree",
    b"command",
    b"submodule",
    b"unknown",
];

/// Whether `origin` is `file:<path>` and `<path>`, resolved against `cwd`
/// when relative, canonicalises to `own_config` (never, when that is `None`);
/// `None` when `origin` is not
/// `file:` or its path cannot be canonicalised, so the caller cannot tell
/// whose key it is.
fn is_own_file(origin: &[u8], cwd: &Path, own_config: Option<&Path>) -> Option<bool> {
    let path = origin.strip_prefix(b"file:")?;
    let found = cwd.join(OsStr::from_bytes(path)).canonicalize().ok()?;
    Some(own_config == Some(found.as_path()))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::process::Output;
    use std::time::{Duration, Instant};

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
    /// `defines_remote` about.
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

    /// `Repo::read_config` on a fresh `Repo::discover(repo)`, with a helper
    /// whose children see the scratch environment; fails the test on `Err`.
    fn read_of(scratch: &Scratch, repo: &Path) -> ConfigRead {
        let found =
            Repo::discover(repo).unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()));
        found
            .read_config(&Git::for_repo(scratch.env.clone(), &found))
            .unwrap_or_else(|err| panic!("read_config {}: {err}", repo.display()))
    }

    /// The values of `key` that `read_of(scratch, repo)` holds.
    fn local_values(scratch: &Scratch, repo: &Path, key: &str) -> Vec<Vec<u8>> {
        read_of(scratch, repo).values(key)
    }

    /// The scope and origin lines `git config --show-scope --show-origin
    /// --get-all KEY` prints with includes followed: the control that git
    /// itself sees the value the read leaves out.
    fn git_sees(scratch: &Scratch, repo: &Path) -> String {
        let all = git(
            scratch,
            repo,
            &["config", "--show-scope", "--show-origin", "--get-all", KEY],
        );
        assert_eq!(
            all.status.code(),
            Some(0),
            "{}",
            testing::failed_run("git config --show-scope --get-all", &all)
        );
        String::from_utf8_lossy(&all.stdout).into_owned()
    }

    /// The scratch environment plus `START_VAR=<repo>`.
    fn child_env(scratch: &Scratch, repo: &Path) -> Env {
        let mut env = scratch.env.clone();
        env.push((START_VAR.into(), repo.into()));
        env
    }

    /// Runs `count_child` with `env` and returns its `count` marker,
    /// failing with the child's output when it printed `error`.
    fn child_count(scratch: &Scratch, env: &Env) -> String {
        let child = testing::reexec(&child_path("count_child"), env, &scratch.root, b"")
            .unwrap_or_else(|err| panic!("{err}"));
        testing::marker(&child, "count").unwrap_or_else(|err| panic!("{err}"))
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
            git_sees(&scratch, &repo),
            "local\tfile:.git/extra.conf\tworkspace/dev\n",
            "git does not list the included value, so this test shows nothing"
        );
        assert_eq!(local_values(&scratch, &repo, KEY), Vec::<Vec<u8>>::new());
    }

    /// A value in global config does not configure the repository: plain
    /// `git config --get` returns it, `--local --get` does not, and a child
    /// with the scratch environment does not read it.
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
        assert_eq!(child_count(&scratch, &child_env(&scratch, &repo)), "0");
    }

    /// The control for the child: the same child returns a value set in the
    /// repository's own config.
    #[test]
    fn local_key_is_read_by_the_same_child() {
        let scratch = Scratch::new("config-local-child").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        git_ok(&scratch, &repo, ["config", KEY, "workspace/dev"])
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(child_count(&scratch, &child_env(&scratch, &repo)), "1");
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
            git_sees(&scratch, &repo),
            "worktree\tfile:.git/config.worktree\tworkspace/dev\n",
            "git does not list the config.worktree value, so this test shows nothing"
        );
        assert_eq!(local_values(&scratch, &repo, KEY), Vec::<Vec<u8>>::new());
    }

    /// A value given through `GIT_CONFIG_COUNT` is not the repository's own
    /// config: git returns it under that environment, and a child with the
    /// same environment does not read it.
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
        assert_eq!(child_count(&scratch, &env), "0");
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
        assert_eq!(local_values(&scratch, &trees.wt1, KEY), git_values);
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
        assert_eq!(local_values(&scratch, &repo, KEY), git_values);
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
        assert_eq!(local_values(&scratch, &repo, KEY), git_values);
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
        assert_eq!(local_values(&scratch, &repo, KEY), git_values);
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
        assert_eq!(local_values(&scratch, &repo, KEY), Vec::<Vec<u8>>::new());
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
            assert_eq!(local_values(&scratch, &repo, KEY), git_values, "{name}");
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
        assert_eq!(local_values(&scratch, &repo, KEY), git_values);
    }

    /// `[wsx "sub"] sourceBranch` is the key `wsx.sub.sourceBranch`, not
    /// `wsx.sourceBranch`: git reads the subsection key under its own name,
    /// and the read does not take it for `wsx.sourceBranch`.
    #[test]
    fn subsection_key_is_not_read() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-subsection").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        append_config(&repo, b"[wsx \"sub\"]\n\tsourceBranch = x\n");

        assert_git_finds_nothing(&scratch, &repo, &["config", "--local", "--get", KEY]);
        let sub = git(
            &scratch,
            &repo,
            &["config", "--local", "--get", "wsx.sub.sourceBranch"],
        );
        let git_values = printed_values("git config --local --get wsx.sub.sourceBranch", &sub);
        assert_eq!(
            git_values,
            [b"x".to_vec()],
            "git does not read the subsection key, so this test shows nothing"
        );
        assert_eq!(local_values(&scratch, &repo, KEY), Vec::<Vec<u8>>::new());
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

    /// Whether `read_of(scratch, repo)` defines the remote `name`.
    fn configured(scratch: &Scratch, repo: &Path, name: &[u8]) -> bool {
        read_of(scratch, repo).defines_remote(name)
    }

    /// The scope lines `git config --show-scope --get-regexp` prints for
    /// the keys of the remote `name`, with includes followed: the control
    /// that git itself sees a section the read must count or drop.
    fn git_remote_keys(scratch: &Scratch, repo: &Path, name: &str) -> String {
        let pattern = format!("^remote\\.{name}\\.");
        let keys = git(
            scratch,
            repo,
            &["config", "--show-scope", "--get-regexp", &pattern],
        );
        String::from_utf8_lossy(&keys.stdout).into_owned()
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

    /// A local `[remote "empty"]` with no keys is not a remote: git prints no
    /// key for it and `git remote add` treats the name as free. Once that
    /// `git remote add` gives the section keys, the read defines it.
    #[test]
    fn keyless_local_remote_section_is_not_a_remote() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-remote-keyless").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        append_config(&repo, b"[remote \"empty\"]\n");

        assert_eq!(git_remote_keys(&scratch, &repo, "empty"), "");
        assert!(
            !configured(&scratch, &repo, b"empty"),
            "read_config counts a local remote section with no keys"
        );
        // Last: it writes the repository's config.
        assert_git_exits(&scratch, &repo, &["remote", "add", "empty", "/y"], 0);
        assert!(
            configured(&scratch, &repo, b"empty"),
            "control: git remote add gave the section a key but read_config defines no empty"
        );
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

        assert!(configured(&scratch, &repo, b"origin"));
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

        assert!(configured(&scratch, &repo, b"bare"));
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
        assert_eq!(
            git_remote_keys(&scratch, &repo, "inc"),
            "local\tremote.inc.url /x\n"
        );
        assert!(configured(&scratch, &repo, b"inc"));
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
        assert_eq!(
            git_remote_keys(&scratch, &repo, "wt"),
            "worktree\tremote.wt.url /x\n"
        );
        assert!(configured(&scratch, &repo, b"wt"));
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

        assert!(!configured(&scratch, &repo, b"nope"));
        assert!(configured(&scratch, &repo, b"origin"), "the control remote");
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
        parse_config_z(stdout, repo, Some(&own))
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

    /// A scope git never prints is refused, never read as a record of
    /// another scope and dropped.
    #[test]
    fn parse_config_z_refuses_a_scope_git_does_not_print() {
        let scratch = Scratch::new("config-parse-scope").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        let control = record(b"local", b"file:.git/config", b"wsx.sourcebranch\ndev");
        assert_eq!(
            parsed_ok(&repo, &control).values(KEY),
            [b"dev".to_vec()],
            "control: the same record at scope local is read"
        );

        let stdout = record(b"LOCAL", b"file:.git/config", b"wsx.sourcebranch\ndev");
        match parsed(&repo, &stdout) {
            Err(problem) => assert!(
                problem.contains("record 1") && problem.contains("scope"),
                "the problem does not name the record's scope: {problem}"
            ),
            other => panic!("expected Err for scope LOCAL, got {other:?}"),
        }
    }

    /// A `wsx` key at scope `local` whose origin is not a file that can be
    /// resolved is refused, never dropped as a key from another file.
    #[test]
    fn parse_config_z_refuses_a_local_key_whose_origin_cannot_be_resolved() {
        let scratch = Scratch::new("config-parse-origin").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        let missing = file_origin(&scratch.root.join("definitely/missing/config"));
        for origin in [missing.as_slice(), b"blob:deadbeef"] {
            let stdout = record(b"local", origin, b"wsx.sourcebranch\ndev");
            match parsed(&repo, &stdout) {
                Err(problem) => assert!(
                    problem.contains("record 1") && problem.contains("origin"),
                    "the problem does not name the record's origin: {problem}"
                ),
                other => panic!(
                    "expected Err for origin {}, got {other:?}",
                    origin.escape_ascii()
                ),
            }
        }
    }

    /// The `--get-regexp` the read runs, for the git-side assertions.
    const REGEXP_ARGS: [&str; 3] = ["config", "--get-regexp", "^(wsx|remote)\\."];

    /// git's exit 1 for "no key matched" is an empty read, not a failure; a
    /// remote set afterwards on the same `Repo` is seen by the next read.
    #[test]
    fn config_read_with_no_keys_is_empty() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-read-empty").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        assert_git_exits(&scratch, &repo, &REGEXP_ARGS, 1);
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
        let listed = git(&scratch, &repo, &REGEXP_ARGS);
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

    /// Only git's exit 1 with nothing on stderr means no key matched; an exit
    /// 1 that prints an error fails the read with git's own failure.
    #[test]
    fn config_read_exit_1_with_an_error_is_a_failure() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-read-exit-1-error").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        let found = Repo::discover(&repo)
            .unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()));
        let body = format!(
            "if [ \"$1\" = config ]; then echo 'error: cannot read config' >&2; exit 1; fi\nexec {} \"$@\"",
            testing::sh_quote(&testing::real_git())
        );
        let helper = Git::for_repo(testing::fake_git(&scratch, &body), &found);

        match found.read_config(&helper) {
            Err(Error::GitFailed {
                status: Some(1),
                stderr,
                ..
            }) => assert_eq!(
                stderr.as_bstr(),
                "error: cannot read config\n".as_bytes().as_bstr()
            ),
            Err(other) => panic!("expected GitFailed with exit 1, got {other:?}"),
            Ok(_) => panic!("expected GitFailed with exit 1, got a read"),
        }
    }

    /// An exit 1 with nothing on stderr is an empty read only when git also
    /// printed nothing: records followed by exit 1 fail the read, never read
    /// as no keys.
    #[test]
    fn config_read_exit_1_after_records_is_a_failure() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-read-exit-1-records").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        append_config(&repo, b"[wsx]\n\tsourceBranch = dev\n");
        let found = Repo::discover(&repo)
            .unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()));
        let real = Git::for_repo(scratch.env.clone(), &found);
        let control = found
            .read_config(&real)
            .unwrap_or_else(|err| panic!("control read_config: {err}"));
        assert_eq!(
            control.values(KEY),
            [b"dev".to_vec()],
            "control: git's own read"
        );

        let real_git = testing::sh_quote(&testing::real_git());
        let body = format!(
            "if [ \"$1\" = config ]; then {real_git} \"$@\"; exit 1; fi\nexec {real_git} \"$@\""
        );
        let helper = Git::for_repo(testing::fake_git(&scratch, &body), &found);

        match found.read_config(&helper) {
            Err(Error::GitFailed {
                status: Some(1),
                stderr,
                ..
            }) => assert!(stderr.is_empty(), "stderr: {:?}", stderr.as_bstr()),
            Err(other) => panic!("expected GitFailed with exit 1, got {other:?}"),
            Ok(read) => panic!("expected GitFailed with exit 1, got a read: {read:?}"),
        }
    }

    /// A fifo where git reads a ref while it evaluates an `includeIf "onbranch:"`
    /// blocks git itself; the read is stopped at the helper's deadline and
    /// fails, never hangs. Control: the same fifo without the include reads.
    #[test]
    fn config_read_stops_on_a_fifo_branch_under_an_onbranch_include() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-read-fifo-onbranch").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        append_config(&repo, b"[wsx]\n\tsourceBranch = dev\n");
        let found = Repo::discover(&repo)
            .unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()));
        let deadline = Duration::from_secs(1);
        let helper = Git::for_repo(scratch.env.clone(), &found).with_deadline(deadline);
        let branch = repo.join(".git/refs/heads/main");
        std::fs::remove_file(&branch)
            .unwrap_or_else(|err| panic!("remove {}: {err}", branch.display()));
        let made = testing::command("mkfifo", &repo, &scratch.env)
            .arg(&branch)
            .output()
            .unwrap_or_else(|err| panic!("mkfifo {}: {err}", branch.display()));
        assert_eq!(made.status.code(), Some(0), "mkfifo: {made:?}");
        let control = found
            .read_config(&helper)
            .unwrap_or_else(|err| panic!("control read_config: {err}"));
        assert_eq!(
            control.values(KEY),
            [b"dev".to_vec()],
            "control: no include, the fifo is not read"
        );

        write(&repo.join(".git/include.conf"), b"").unwrap_or_else(|err| panic!("{err}"));
        append_config(
            &repo,
            b"[includeIf \"onbranch:main\"]\n\tpath = include.conf\n",
        );
        let hung = testing::command("timeout", &repo, &scratch.env)
            .args(["3", "git", "config", "--get-regexp", r"^(wsx|remote)\."])
            .output()
            .unwrap_or_else(|err| panic!("timeout git config: {err}"));
        assert_eq!(
            hung.status.code(),
            Some(124),
            "git itself does not block here: {hung:?}"
        );

        let started = Instant::now();
        match found.read_config(&helper) {
            Err(Error::GitTimedOut { after, .. }) => assert_eq!(after, deadline),
            Err(other) => panic!("expected GitTimedOut, got {other:?}"),
            Ok(read) => panic!("expected GitTimedOut, got a read: {read:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "the read was stopped after {:?}",
            started.elapsed()
        );
    }

    /// A repository with no `.git/config` of its own, which git still works in,
    /// reads as holding no keys of its own when git prints a global record,
    /// never fails the read. Control: the same global record with the file
    /// present reads the same way.
    #[test]
    fn config_read_without_an_own_config_file_keeps_no_keys() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-read-no-own-file").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        let global = scratch.home.join(".gitconfig");
        let mut config = std::fs::read(&global).expect("read the scratch global config");
        config.extend_from_slice(
            b"[wsx]\n\tsourceBranch = workspace/dev\n[remote]\n\tpushDefault = g\n",
        );
        write(&global, &config).expect("append the global keys");
        let found = Repo::discover(&repo)
            .unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()));
        let helper = Git::for_repo(scratch.env.clone(), &found);
        let control = found
            .read_config(&helper)
            .unwrap_or_else(|err| panic!("control read_config: {err}"));
        assert_eq!(
            control.values(KEY),
            Vec::<Vec<u8>>::new(),
            "control: the global key is not read"
        );

        let own = repo.join(".git/config");
        std::fs::remove_file(&own).unwrap_or_else(|err| panic!("remove {}: {err}", own.display()));
        let status = git(&scratch, &repo, &["status", "--porcelain"]);
        assert_eq!(
            status.status.code(),
            Some(0),
            "git works without its config: {status:?}"
        );
        let listed = git(&scratch, &repo, &REGEXP_ARGS);
        assert_eq!(
            listed.status.code(),
            Some(0),
            "git prints the global records: {listed:?}"
        );

        let read = found
            .read_config(&helper)
            .unwrap_or_else(|err| panic!("read_config without .git/config: {err}"));
        assert_eq!(read.values(KEY), Vec::<Vec<u8>>::new());
        assert!(!read.defines_remote(b"g"));
    }

    /// A `.git/config` that exists but cannot be resolved (a symlink to itself)
    /// fails the read naming it, never reads as a repository with no config
    /// of its own. git refuses such a config itself, so a fake git prints the
    /// one record a real git would have to print for the read to get there.
    #[test]
    fn config_read_with_an_unresolvable_own_config_file_is_an_error() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-read-looped-own-file").expect("scratch");
        let repo = one_commit_repo(&scratch, "repo");
        let found = Repo::discover(&repo)
            .unwrap_or_else(|err| panic!("discover {}: {err}", repo.display()));
        let own = repo.join(".git/config");
        std::fs::remove_file(&own).unwrap_or_else(|err| panic!("remove {}: {err}", own.display()));
        std::os::unix::fs::symlink(&own, &own)
            .unwrap_or_else(|err| panic!("symlink {}: {err}", own.display()));
        let body = "if [ \"$1\" = --version ]; then echo 'git version 2.54.0'; exit 0; fi\nprintf 'global\\0file:/g\\0remote.g.url\\nu\\0'";
        let helper = Git::for_repo(testing::fake_git(&scratch, body), &found);

        match found.read_config(&helper) {
            Err(Error::Io { what, .. }) => assert!(
                what.contains(".git/config"),
                "the error does not name the config file: {what}"
            ),
            Err(other) => panic!("expected Io naming .git/config, got {other:?}"),
            Ok(read) => panic!("expected Io naming .git/config, got a read: {read:?}"),
        }
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

    /// A repository whose path holds a space and a byte that is not UTF-8
    /// reads its own key: the child's cwd and the own file are handled as
    /// bytes. Once git unsets the key, both answers are empty.
    #[test]
    fn config_read_from_a_path_with_a_space_and_a_non_utf8_byte() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("config-odd-path").expect("scratch");
        let repo = scratch.root.join(OsStr::from_bytes(b"work dir\xe9"));
        std::fs::create_dir_all(&repo)
            .unwrap_or_else(|err| panic!("create {}: {err}", repo.display()));
        for args in [
            ["init", "-q", "-b", "main"].as_slice(),
            &["config", "--local", KEY, "dev"],
        ] {
            git_ok(&scratch, &repo, args).unwrap_or_else(|err| panic!("{err}"));
        }

        let get = git(&scratch, &repo, &["config", "--local", "--get-all", KEY]);
        let git_values = printed_values("git config --local --get-all", &get);
        assert_eq!(git_values, [b"dev".to_vec()]);
        assert_eq!(local_values(&scratch, &repo, KEY), git_values);

        git_ok(&scratch, &repo, ["config", "--local", "--unset", KEY])
            .unwrap_or_else(|err| panic!("{err}"));
        assert_git_finds_nothing(&scratch, &repo, &["config", "--local", "--get-all", KEY]);
        assert_eq!(local_values(&scratch, &repo, KEY), Vec::<Vec<u8>>::new());
    }

    /// Prints `configured`, what `read_config` through `Git::from_process`
    /// answers for the name in `REMOTE_VAR` on `Repo::discover` of the
    /// directory in `START_VAR`; or `error` when discovery or the read fails.
    #[test]
    #[ignore = "child half of the remote definition tests; runs only when re-executed"]
    fn remote_child() {
        let start = std::env::var_os(START_VAR).expect("the parent set the start directory");
        let name = std::env::var_os(REMOTE_VAR).expect("the parent set the remote name");
        match Repo::discover(Path::new(&start))
            .and_then(|found| found.read_config(&Git::from_process(&found)))
        {
            Ok(read) => {
                testing::child_print("configured", read.defines_remote(name.as_encoded_bytes()));
            }
            Err(err) => testing::child_print("error", err.to_string().replace('\n', "\\n")),
        }
    }

    /// Prints `count`, the number of values of `KEY` that `read_config`
    /// through `Git::from_process` returns on `Repo::discover` of the
    /// directory in `START_VAR`; or `error` when discovery or the read fails.
    #[test]
    #[ignore = "child half of the config scope tests; runs only when re-executed"]
    fn count_child() {
        let start = std::env::var_os(START_VAR).expect("the parent set the start directory");
        match Repo::discover(Path::new(&start))
            .and_then(|found| found.read_config(&Git::from_process(&found)))
        {
            Ok(read) => testing::child_print("count", read.values(KEY).len()),
            Err(err) => testing::child_print("error", err.to_string().replace('\n', "\\n")),
        }
    }
}
