//! The two staleness times of D-12, as amended: when the integration ref
//! last moved, and when any worktree of the repository last fetched.
//!
//! `Repo::ref_moved_at` is the time of the ref's newest reflog entry: when
//! this repository last moved that ref (a fetch that changed it, or any other
//! update), not when it last asked the remote. A fetch that finds nothing new
//! adds no entry, and a fresh clone has no reflog for its remote-tracking
//! refs at all. `Repo::fetched_at` is the newest `FETCH_HEAD` modification
//! time: git keeps one per worktree, so it is when any worktree last fetched
//! anything, from any remote, not that the integration ref is current
//! (05-RESEARCH Pitfall 1).
//!
//! Every caller reads these after `model::resolve` has named the integration
//! ref (05-01's read order); the ref is passed by its full name only, after
//! `--end-of-options`.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::repo::Repo;
use super::{Error, Git, NO_LAZY_FETCH};

/// The argv of the reflog read, before the ref's full name: the newest
/// entry's selector with its unix time, never a signature check on stdout.
const REFLOG_ARGS: [&str; 8] = [
    NO_LAZY_FETCH,
    "reflog",
    "show",
    "--no-show-signature",
    "-n1",
    "--date=unix",
    "--format=%gd",
    "--end-of-options",
];

impl Repo {
    /// Stub.
    pub fn ref_moved_at(&self, _git: &Git, _full_ref: &[u8]) -> Result<Option<i64>, Error> {
        Ok(None)
    }

    /// Stub.
    pub fn fetched_at(&self) -> Result<Option<i64>, Error> {
        Ok(None)
    }

    /// Stub.
    fn fetch_head_paths(&self) -> Result<Vec<PathBuf>, Error> {
        Ok(Vec::new())
    }
}

/// Stub.
fn parse_reflog_selector(_stdout: &[u8]) -> Result<Option<i64>, String> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, UNIX_EPOCH};

    use gix::bstr::ByteSlice;

    use super::super::testing::{self, Scratch, T0, TICK, commit_all, git_ok, write};
    use super::super::{Git, Repo};
    use super::parse_reflog_selector;

    /// The integration ref every test reads.
    const REF: &str = "refs/remotes/origin/develop";

    /// `<root>/origin` on `develop` with one commit at `T0`, and its clone
    /// `<root>/work`; both canonical.
    fn cloned(scratch: &Scratch) -> (PathBuf, PathBuf) {
        let origin = scratch.root.join("origin");
        fs::create_dir_all(&origin).unwrap_or_else(|err| panic!("create origin: {err}"));
        git_ok(scratch, &origin, ["init", "-q", "-b", "develop"])
            .unwrap_or_else(|err| panic!("{err}"));
        write(&origin.join("a.txt"), b"a\n").unwrap_or_else(|err| panic!("{err}"));
        commit_all(scratch, &origin, "base", T0).unwrap_or_else(|err| panic!("{err}"));
        let work = scratch.root.join("work");
        git_ok(
            scratch,
            &scratch.root,
            [
                OsStr::new("clone"),
                OsStr::new("-q"),
                origin.as_os_str(),
                work.as_os_str(),
            ],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let canonical = |path: &Path| {
            path.canonicalize()
                .unwrap_or_else(|err| panic!("canonicalize {}: {err}", path.display()))
        };
        (canonical(&origin), canonical(&work))
    }

    /// Commits `name` in `origin` at `epoch`, so the next fetch moves the ref.
    fn advance(scratch: &Scratch, origin: &Path, name: &str, epoch: i64) {
        write(&origin.join(name), name.as_bytes()).unwrap_or_else(|err| panic!("{err}"));
        commit_all(scratch, origin, name, epoch).unwrap_or_else(|err| panic!("{err}"));
    }

    /// `git fetch -q origin` in `dir` with the committer date at `epoch`,
    /// the time git writes into a reflog entry.
    fn fetch_at(scratch: &Scratch, dir: &Path, epoch: i64) {
        let output = testing::git_in_env(
            &testing::dated(&scratch.env, epoch),
            dir,
            ["fetch", "-q", "origin"],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            testing::failed_run("git fetch", &output)
        );
    }

    /// The exit code of `git reflog exists REF` in `dir`.
    fn reflog_exists(scratch: &Scratch, dir: &Path) -> Option<i32> {
        testing::git(scratch, dir, ["reflog", "exists", REF])
            .unwrap_or_else(|err| panic!("{err}"))
            .status
            .code()
    }

    /// Adds the linked worktree `<root>/linked` on a new branch to `work`;
    /// returns it canonical.
    fn add_linked(scratch: &Scratch, work: &Path) -> PathBuf {
        let linked = scratch.root.join("linked");
        git_ok(
            scratch,
            work,
            [
                OsStr::new("worktree"),
                OsStr::new("add"),
                OsStr::new("-q"),
                OsStr::new("-b"),
                OsStr::new("linked"),
                linked.as_os_str(),
            ],
        )
        .unwrap_or_else(|err| panic!("{err}"));
        linked
            .canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize linked: {err}"))
    }

    /// The `FETCH_HEAD` path `git rev-parse --git-path FETCH_HEAD` prints in
    /// `dir`, with its parent canonicalised.
    fn git_path_fetch_head(scratch: &Scratch, dir: &Path) -> PathBuf {
        let stdout = git_ok(scratch, dir, ["rev-parse", "--git-path", "FETCH_HEAD"])
            .unwrap_or_else(|err| panic!("{err}"));
        let printed = stdout
            .strip_suffix(b"\n")
            .unwrap_or_else(|| panic!("git rev-parse printed no newline: {stdout:?}"));
        let path = printed
            .to_path()
            .unwrap_or_else(|err| panic!("a path git printed: {err}"));
        testing::canonical_parent(dir, path).unwrap_or_else(|err| panic!("{err}"))
    }

    /// `path`'s modification time in whole seconds since the epoch.
    fn mtime_secs(path: &Path) -> i64 {
        let modified = fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or_else(|err| panic!("stat {}: {err}", path.display()));
        let secs = modified
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|err| panic!("{} predates the epoch: {err}", path.display()))
            .as_secs();
        i64::try_from(secs).unwrap_or_else(|err| panic!("{secs} seconds: {err}"))
    }

    /// `Repo::discover(dir)` and a helper whose children see the scratch
    /// environment.
    fn open(scratch: &Scratch, dir: &Path) -> (Repo, Git) {
        let found =
            Repo::discover(dir).unwrap_or_else(|err| panic!("discover {}: {err}", dir.display()));
        let git = Git::for_repo(scratch.env.clone(), &found);
        (found, git)
    }

    /// `ref_moved_at(REF)` in `dir`, failing the test on `Err`.
    fn moved_at(scratch: &Scratch, dir: &Path) -> Option<i64> {
        let (found, git) = open(scratch, dir);
        found
            .ref_moved_at(&git, REF.as_bytes())
            .unwrap_or_else(|err| panic!("ref_moved_at: {err}"))
    }

    /// `fetched_at()` from a repository opened in `dir`, failing on `Err`.
    fn fetched_from(scratch: &Scratch, dir: &Path) -> Option<i64> {
        open(scratch, dir)
            .0
            .fetched_at()
            .unwrap_or_else(|err| panic!("fetched_at: {err}"))
    }

    #[test]
    fn ref_moved_at_is_none_after_a_clone() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("staleness-clone").expect("scratch");
        let (_, work) = cloned(&scratch);
        assert_eq!(reflog_exists(&scratch, &work), Some(1), "git has no reflog");
        assert_eq!(moved_at(&scratch, &work), None);
    }

    #[test]
    fn ref_moved_at_is_the_newest_entry() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("staleness-newest").expect("scratch");
        let (origin, work) = cloned(&scratch);
        advance(&scratch, &origin, "b.txt", T0 + TICK);
        fetch_at(&scratch, &work, T0);
        advance(&scratch, &origin, "c.txt", T0 + 2 * TICK);
        fetch_at(&scratch, &work, T0 + TICK);
        assert_eq!(reflog_exists(&scratch, &work), Some(0), "git has a reflog");
        assert_eq!(moved_at(&scratch, &work), Some(T0 + TICK));
    }

    /// Signing config for `origin` with a throwaway ed25519 key, and the
    /// `allowed_signers` file that lets `work` verify it, as
    /// `spawn::tests::signing_repo` sets them up.
    fn sign_commits(scratch: &Scratch, origin: &Path, work: &Path) {
        let key = scratch.root.join("signkey");
        let output = testing::command("ssh-keygen", &scratch.root, &scratch.env)
            .args([
                OsStr::new("-q"),
                OsStr::new("-t"),
                OsStr::new("ed25519"),
                OsStr::new("-N"),
                OsStr::new(""),
                OsStr::new("-C"),
                OsStr::new("unit"),
                OsStr::new("-f"),
                key.as_os_str(),
            ])
            .output()
            .unwrap_or_else(|err| panic!("spawn ssh-keygen: {err}"));
        assert!(
            output.status.success(),
            "{}",
            testing::failed_run("ssh-keygen", &output)
        );
        let public = fs::read(scratch.root.join("signkey.pub"))
            .unwrap_or_else(|err| panic!("read signkey.pub: {err}"));
        let fields: Vec<&[u8]> = public
            .split(u8::is_ascii_whitespace)
            .filter(|field| !field.is_empty())
            .collect();
        let [kind, base64, ..] = fields[..] else {
            panic!("unexpected public key {:?}", public.as_bstr());
        };
        let mut allowed = b"unit@example.invalid namespaces=\"git\" ".to_vec();
        allowed.extend_from_slice(kind);
        allowed.push(b' ');
        allowed.extend_from_slice(base64);
        allowed.push(b'\n');
        let allowed_path = scratch.root.join("allowed_signers");
        write(&allowed_path, &allowed).unwrap_or_else(|err| panic!("{err}"));
        let config = |dir: &Path, name: &str, value: &OsStr| {
            git_ok(
                scratch,
                dir,
                [
                    OsStr::new("config"),
                    OsStr::new("--local"),
                    OsStr::new(name),
                    value,
                ],
            )
            .unwrap_or_else(|err| panic!("{err}"));
        };
        config(origin, "gpg.format", OsStr::new("ssh"));
        config(origin, "user.signingkey", key.as_os_str());
        config(origin, "commit.gpgsign", OsStr::new("true"));
        config(work, "gpg.ssh.allowedSignersFile", allowed_path.as_os_str());
        config(work, "log.showSignature", OsStr::new("true"));
    }

    /// The unix time in a `<ref>@{<time>}` selector line, for the oracle.
    fn selector_time(line: &str) -> i64 {
        let (_, rest) = line
            .rsplit_once("@{")
            .unwrap_or_else(|| panic!("no @{{ in {line:?}"));
        rest.strip_suffix('}')
            .and_then(|digits| digits.parse().ok())
            .unwrap_or_else(|| panic!("no time in {line:?}"))
    }

    #[test]
    fn ref_moved_at_ignores_log_show_signature() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("staleness-signature").expect("scratch");
        let (origin, work) = cloned(&scratch);
        sign_commits(&scratch, &origin, &work);
        advance(&scratch, &origin, "b.txt", T0 + TICK);
        fetch_at(&scratch, &work, T0 + TICK);

        let shown = |extra: &[&str]| {
            let mut args = vec!["reflog", "show"];
            args.extend_from_slice(extra);
            args.extend([
                "-n1",
                "--date=unix",
                "--format=%gd",
                "--end-of-options",
                REF,
                "--",
            ]);
            let stdout = git_ok(&scratch, &work, &args).unwrap_or_else(|err| panic!("{err}"));
            String::from_utf8(stdout).unwrap_or_else(|err| panic!("non-UTF-8: {err}"))
        };
        let with_signature = shown(&[]);
        let lines: Vec<&str> = with_signature.lines().collect();
        assert!(
            lines.len() > 1 && !lines[0].contains("@{"),
            "log.showSignature puts a line before the selector: {with_signature:?}"
        );
        let plain = shown(&["--no-show-signature"]);
        assert_eq!(plain.lines().count(), 1, "one selector line: {plain:?}");
        let expected = selector_time(plain.trim_end());
        assert_eq!(moved_at(&scratch, &work), Some(expected));
    }

    #[test]
    fn fetched_at_is_none_before_any_fetch() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("staleness-no-fetch").expect("scratch");
        let (_, work) = cloned(&scratch);
        let linked = add_linked(&scratch, &work);
        assert!(
            !work.join(".git/FETCH_HEAD").exists(),
            "a clone has no FETCH_HEAD"
        );
        assert_eq!(fetched_from(&scratch, &work), None);
        assert_eq!(fetched_from(&scratch, &linked), None);
    }

    #[test]
    fn fetched_at_is_the_fetch_head_mtime() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("staleness-fetch-head").expect("scratch");
        let (_, work) = cloned(&scratch);
        fetch_at(&scratch, &work, T0);
        let fetch_head = git_path_fetch_head(&scratch, &work);
        assert_eq!(fetched_from(&scratch, &work), Some(mtime_secs(&fetch_head)));
    }

    #[test]
    fn fetch_head_paths_include_each_worktrees_git_path() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("staleness-git-path").expect("scratch");
        let (_, work) = cloned(&scratch);
        let linked = add_linked(&scratch, &work);
        let main_path = git_path_fetch_head(&scratch, &work);
        let linked_path = git_path_fetch_head(&scratch, &linked);
        assert_ne!(
            main_path, linked_path,
            "git keeps one FETCH_HEAD per worktree"
        );
        for dir in [&work, &linked] {
            let paths = open(&scratch, dir)
                .0
                .fetch_head_paths()
                .unwrap_or_else(|err| panic!("fetch_head_paths: {err}"));
            for expected in [&main_path, &linked_path] {
                assert!(
                    paths.contains(expected),
                    "{} among the paths read from {}: {paths:?}",
                    expected.display(),
                    dir.display()
                );
            }
        }
    }

    #[test]
    fn fetched_at_reads_a_fetch_in_the_main_worktree_from_a_linked_one() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("staleness-linked").expect("scratch");
        let (_, work) = cloned(&scratch);
        let linked = add_linked(&scratch, &work);
        fetch_at(&scratch, &work, T0);
        let own = git_path_fetch_head(&scratch, &linked);
        assert!(
            !own.exists(),
            "the linked worktree's own {} is absent",
            own.display()
        );
        let main = git_path_fetch_head(&scratch, &work);
        assert_eq!(fetched_from(&scratch, &linked), Some(mtime_secs(&main)));
    }

    #[test]
    fn fetched_at_is_the_newest_across_worktrees() {
        if testing::rerun_scrubbed() {
            return;
        }
        let scratch = Scratch::new("staleness-newest-worktree").expect("scratch");
        let (_, work) = cloned(&scratch);
        let linked = add_linked(&scratch, &work);
        fetch_at(&scratch, &work, T0);
        fetch_at(&scratch, &linked, T0);
        let set = |path: &Path, epoch: i64| {
            let secs = u64::try_from(epoch).unwrap_or_else(|err| panic!("{epoch}: {err}"));
            fs::File::options()
                .write(true)
                .open(path)
                .and_then(|file| file.set_modified(UNIX_EPOCH + Duration::from_secs(secs)))
                .unwrap_or_else(|err| panic!("set the mtime of {}: {err}", path.display()));
        };
        set(&git_path_fetch_head(&scratch, &work), T0);
        set(&git_path_fetch_head(&scratch, &linked), T0 + TICK);
        assert_eq!(fetched_from(&scratch, &work), Some(T0 + TICK));
    }

    #[test]
    fn parse_reflog_selector_reads_the_time_after_the_last_at_brace() {
        assert_eq!(
            parse_reflog_selector(b"origin/develop@{1795024796}\n"),
            Ok(Some(1_795_024_796))
        );
        assert_eq!(
            parse_reflog_selector(b"we@{ird}@{1795024796}\n"),
            Ok(Some(1_795_024_796))
        );
        assert_eq!(parse_reflog_selector(b""), Ok(None));
    }

    #[test]
    fn parse_reflog_selector_refuses_other_output() {
        assert_eq!(
            parse_reflog_selector(b"origin/develop\n"),
            Err("no @{ in the selector".to_owned())
        );
        assert_eq!(
            parse_reflog_selector(b"origin/develop@{yesterday}\n"),
            Err("the selector's time is not a unix time".to_owned())
        );
        assert_eq!(
            parse_reflog_selector(b"Good signature\norigin/develop@{1}\n"),
            Err("more than one line".to_owned())
        );
        assert_eq!(
            parse_reflog_selector(b"origin/develop@{1}"),
            Err("the output does not end in LF".to_owned())
        );
    }
}
