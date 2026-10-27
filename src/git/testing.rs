//! Test support for the git layer: scratch repositories on disk and a
//! hermetic environment for every git child a test runs.
//!
//! Unit tests of a binary crate get no `CARGO_TARGET_TMPDIR`, so the scratch
//! root is derived from the test executable instead: `<target>/tmp`, on disk,
//! never the RAM-backed `/tmp`. Each `Scratch` is a `TempDir` named
//! `git-unit-<name>-` so a leftover is attributable, and it is removed on drop
//! even when the test panics.
//!
//! The scratch dirs sit inside the wsx checkout that holds `target/`, so every
//! child gets `GIT_CEILING_DIRECTORIES` at the scratch root's parent: git
//! applies a ceiling only to proper ancestors of its working directory, and
//! the parent is not a repository, so discovery never climbs into wsx itself.
//! `command` clears the environment first, so nothing from the host
//! (`GIT_DIR`, `GIT_CONFIG_*`, the user's PATH and hooks) reaches git.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

/// An environment for one child, as ordered key/value pairs.
pub type Env = Vec<(OsString, OsString)>;

/// The fixture clock's start: 2026-01-01T00:00:00Z.
pub const T0: i64 = 1_767_225_600;

/// The step between consecutive fixture commits, in seconds.
pub const TICK: i64 = 60;

/// The only PATH a spawned child sees, so the host's own git is never used.
const HERMETIC_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// The scratch global git config, written to `<home>/.gitconfig`.
const GLOBAL_CONFIG: &str = "\
[user]
\tname = Unit
\temail = unit@example.invalid
[init]
\tdefaultBranch = main
[commit]
\tgpgsign = false
[tag]
\tgpgsign = false
";

/// `<target>/tmp`, derived from the test executable at
/// `<target>/<profile>/deps/<crate>-<hash>`, created if missing. Panics when
/// the executable is not in a `deps` directory, so a different build layout
/// fails loudly instead of writing somewhere unexpected.
pub fn scratch_root() -> PathBuf {
    let exe = std::env::current_exe().expect("locate the test executable");
    let deps = exe
        .parent()
        .unwrap_or_else(|| panic!("{} has no parent", exe.display()));
    assert_eq!(
        deps.file_name(),
        Some(OsStr::new("deps")),
        "test executable {} is not in a deps directory",
        exe.display()
    );
    let root = deps.join("../../tmp");
    fs::create_dir_all(&root).unwrap_or_else(|err| panic!("create {}: {err}", root.display()));
    root
}

/// One test's scratch space: a canonical root, a HOME with the scratch global
/// config, and the hermetic environment every git child runs with.
pub struct Scratch {
    /// Removes the whole scratch tree on drop.
    _temp: TempDir,
    /// The canonical scratch root.
    pub root: PathBuf,
    /// HOME for every child, holding `.gitconfig`.
    pub home: PathBuf,
    /// The environment every git child runs with.
    pub env: Env,
}

impl Scratch {
    /// A fresh scratch tree under `scratch_root()`, prefixed `git-unit-<name>-`.
    pub fn new(name: &str) -> Result<Scratch, String> {
        let parent = scratch_root();
        let prefix = format!("git-unit-{name}-");
        let temp = TempDir::with_prefix_in(&prefix, &parent).map_err(|err| {
            format!(
                "create a TempDir {prefix}* under {}: {err}",
                parent.display()
            )
        })?;
        let root = temp
            .path()
            .canonicalize()
            .map_err(|err| format!("canonicalize {}: {err}", temp.path().display()))?;
        let home = root.join("home");
        let env = hermetic_env(&root, &home);
        let scratch = Scratch {
            _temp: temp,
            root,
            home,
            env,
        };
        write(&scratch.home.join(".gitconfig"), GLOBAL_CONFIG.as_bytes())?;
        Ok(scratch)
    }
}

/// The complete environment of a git child in scratch `root` with HOME
/// `home`: the fixed keys, then every inherited pair whose key starts with
/// `LLVM_` (so coverage still sees the child). Nothing else is inherited.
pub fn hermetic_env(root: &Path, home: &Path) -> Env {
    let ceiling = root
        .parent()
        .unwrap_or_else(|| panic!("scratch root {} has no parent", root.display()));
    let pair = |key: &str, value: &OsStr| (OsString::from(key), value.to_os_string());
    let mut env = vec![
        pair("HOME", home.as_os_str()),
        pair("GIT_CONFIG_GLOBAL", home.join(".gitconfig").as_os_str()),
        pair("GIT_CONFIG_NOSYSTEM", OsStr::new("1")),
        pair("XDG_CONFIG_HOME", home.join(".config").as_os_str()),
        pair("XDG_DATA_HOME", home.join(".local/share").as_os_str()),
        pair("XDG_CACHE_HOME", home.join(".cache").as_os_str()),
        pair("XDG_STATE_HOME", home.join(".local/state").as_os_str()),
        pair("GIT_CEILING_DIRECTORIES", ceiling.as_os_str()),
        pair("GIT_ALLOW_PROTOCOL", OsStr::new("file")),
        pair("GIT_TERMINAL_PROMPT", OsStr::new("0")),
        pair("GIT_EDITOR", OsStr::new("false")),
        pair("LC_ALL", OsStr::new("C")),
        pair("TZ", OsStr::new("UTC")),
        pair("PATH", OsStr::new(HERMETIC_PATH)),
    ];
    env.extend(std::env::vars_os().filter(|(key, _)| key.as_bytes().starts_with(b"LLVM_")));
    env
}

/// `env` plus author and committer dates fixed at `epoch` UTC.
pub fn dated(env: &Env, epoch: i64) -> Env {
    let date = OsString::from(format!("@{epoch} +0000"));
    let mut env = env.clone();
    env.push(("GIT_AUTHOR_DATE".into(), date.clone()));
    env.push(("GIT_COMMITTER_DATE".into(), date));
    env
}

/// The only way test support builds a child: an empty environment plus
/// `env`, `cwd` as working directory, stdin from `/dev/null`. Because `PATH`
/// is set on the child, a bare program name resolves through the hermetic
/// PATH, not the test process's.
pub fn command(program: impl AsRef<OsStr>, cwd: &Path, env: &Env) -> Command {
    let mut command = Command::new(program);
    command
        .env_clear()
        .envs(env.iter().map(|(key, value)| (key, value)))
        .current_dir(cwd)
        .stdin(Stdio::null());
    command
}

/// `args` as one line for a message.
fn render(args: &[OsString]) -> String {
    args.iter()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Collects an argv into owned strings.
fn argv<I, S>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    args.into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect()
}

/// Runs git in `dir` with `env`; an error only when it cannot be started.
fn spawn_git(env: &Env, dir: &Path, args: &[OsString]) -> Result<Output, String> {
    command("git", dir, env)
        .args(args)
        .output()
        .map_err(|err| format!("spawn git {} in {}: {err}", render(args), dir.display()))
}

/// The stdout of a git run, or an error naming its argv, directory, exit code
/// and stderr when it did not exit 0.
fn checked(dir: &Path, args: &[OsString], output: Output) -> Result<Vec<u8>, String> {
    if output.status.code() == Some(0) {
        Ok(output.stdout)
    } else {
        Err(format!(
            "git {} in {} exited {:?}; stderr: {}",
            render(args),
            dir.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim_end()
        ))
    }
}

/// Runs git in `dir` with the scratch environment.
pub fn git<I, S>(scratch: &Scratch, dir: &Path, args: I) -> Result<Output, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    spawn_git(&scratch.env, dir, &argv(args))
}

/// Runs git in `dir` and returns its stdout, or an error carrying the
/// command, exit code and stderr when it does not exit 0.
pub fn git_ok<I, S>(scratch: &Scratch, dir: &Path, args: I) -> Result<Vec<u8>, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args = argv(args);
    let output = git(scratch, dir, &args)?;
    checked(dir, &args, output)
}

/// `git_ok` with author and committer dates fixed at `epoch`.
fn git_ok_at<I, S>(scratch: &Scratch, dir: &Path, epoch: i64, args: I) -> Result<Vec<u8>, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args = argv(args);
    let output = spawn_git(&dated(&scratch.env, epoch), dir, &args)?;
    checked(dir, &args, output)
}

/// Writes `content` to `path`, creating parent directories.
pub fn write(path: &Path, content: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    fs::write(path, content).map_err(|err| format!("write {}: {err}", path.display()))
}

/// Stages everything in `dir` and commits it with `message`, authored and
/// committed at `epoch`.
pub fn commit_all(scratch: &Scratch, dir: &Path, message: &str, epoch: i64) -> Result<(), String> {
    git_ok(scratch, dir, ["add", "--all"])?;
    git_ok_at(scratch, dir, epoch, ["commit", "--quiet", "-m", message]).map(|_| ())
}

/// The id `rev` names in `dir`, as hex.
fn rev_parse(scratch: &Scratch, dir: &Path, rev: &str) -> Result<String, String> {
    let stdout = git_ok(scratch, dir, ["rev-parse", "--verify", rev])?;
    String::from_utf8(stdout)
        .map(|id| id.trim_end().to_owned())
        .map_err(|err| format!("git rev-parse {rev} printed non-UTF-8: {err}"))
}

/// Builds the criss-cross fixture in `<scratch root>/repo` and returns that
/// path, with `main` checked out.
///
/// `main`: C0 - C1 - M1 (merges F1) - C2. `feat`: C0 - F1 - F2 (merges C1) -
/// F3 - F4. M1 and F2 cross, so `main` and `feat` have two best merge-bases,
/// C1 and F1. F3 is dated before its parent F2 (clock skew); it renames `f/x`
/// to `f/z` and turns the file `p` into a symlink. C2 deletes `q` and adds
/// `new/y`. Fails unless `git merge-base --all main feat` prints exactly C1
/// and F1, and F3's committer time is below F2's.
pub fn criss_cross(scratch: &Scratch) -> Result<PathBuf, String> {
    let repo = scratch.root.join("repo");
    fs::create_dir_all(&repo).map_err(|err| format!("create {}: {err}", repo.display()))?;
    git_ok(scratch, &repo, ["init", "-q", "-b", "main"])?;

    write(&repo.join("a.txt"), b"a\n")?;
    write(&repo.join("f/x"), b"x\n")?;
    write(&repo.join("p"), b"p\n")?;
    write(&repo.join("q"), b"q\n")?;
    commit_all(scratch, &repo, "c0 base", T0)?;
    git_ok(scratch, &repo, ["branch", "feat"])?;

    write(&repo.join("a.txt"), b"a1\n")?;
    commit_all(scratch, &repo, "c1 main", T0 + TICK)?;
    let c1 = rev_parse(scratch, &repo, "HEAD")?;

    git_ok(scratch, &repo, ["switch", "-q", "feat"])?;
    write(&repo.join("b.txt"), b"b\n")?;
    commit_all(scratch, &repo, "f1 feat", T0 + 2 * TICK)?;
    let f1 = rev_parse(scratch, &repo, "HEAD")?;

    git_ok(scratch, &repo, ["switch", "-q", "main"])?;
    git_ok_at(
        scratch,
        &repo,
        T0 + 3 * TICK,
        [
            "merge",
            "--no-ff",
            "--no-edit",
            "-m",
            "m1 merge feat",
            "feat",
        ],
    )?;

    git_ok(scratch, &repo, ["switch", "-q", "feat"])?;
    git_ok_at(
        scratch,
        &repo,
        T0 + 4 * TICK,
        [
            "merge",
            "--no-ff",
            "--no-edit",
            "-m",
            "f2 merge main",
            c1.as_str(),
        ],
    )?;

    git_ok(scratch, &repo, ["switch", "-q", "main"])?;
    git_ok(scratch, &repo, ["rm", "-q", "q"])?;
    write(&repo.join("new/y"), b"y\n")?;
    commit_all(scratch, &repo, "c2 main", T0 + 5 * TICK)?;

    git_ok(scratch, &repo, ["switch", "-q", "feat"])?;
    git_ok(scratch, &repo, ["mv", "f/x", "f/z"])?;
    let p = repo.join("p");
    fs::remove_file(&p).map_err(|err| format!("remove {}: {err}", p.display()))?;
    std::os::unix::fs::symlink("a.txt", &p)
        .map_err(|err| format!("symlink {} -> a.txt: {err}", p.display()))?;
    commit_all(scratch, &repo, "f3 skewed", T0 + 30)?;

    write(&repo.join("b.txt"), b"b2\n")?;
    commit_all(scratch, &repo, "f4 feat", T0 + 6 * TICK)?;
    git_ok(scratch, &repo, ["switch", "-q", "main"])?;

    let all = git_ok(scratch, &repo, ["merge-base", "--all", "main", "feat"])?;
    let mut bases: Vec<String> = String::from_utf8_lossy(&all)
        .lines()
        .map(str::to_owned)
        .collect();
    bases.sort();
    let mut expected = vec![c1, f1];
    expected.sort();
    if bases != expected {
        return Err(format!(
            "criss-cross: git merge-base --all main feat printed {bases:?}, expected C1 and F1 {expected:?}"
        ));
    }

    let f3_time = git_ok(scratch, &repo, ["log", "-1", "--format=%ct", "feat~1"])?;
    let f2_time = git_ok(scratch, &repo, ["log", "-1", "--format=%ct", "feat~2"])?;
    let parse = |stdout: &[u8]| -> Result<i64, String> {
        String::from_utf8_lossy(stdout)
            .trim()
            .parse::<i64>()
            .map_err(|err| format!("parse committer time {stdout:?}: {err}"))
    };
    let (f3_time, f2_time) = (parse(&f3_time)?, parse(&f2_time)?);
    if f3_time >= f2_time {
        return Err(format!(
            "criss-cross: F3 committer time {f3_time} is not below F2's {f2_time}"
        ));
    }
    Ok(repo)
}

/// A main worktree and two linked worktrees of one repository, all canonical.
pub struct Worktrees {
    /// The main worktree, `<scratch root>/main`, on branch `main`.
    pub main: PathBuf,
    /// A linked worktree beside the main one, `<scratch root>/wt1`, on `wt1`.
    pub wt1: PathBuf,
    /// A linked worktree nested inside the main one,
    /// `<main>/.worktrees/wt2`, on `wt2`: the layout wsx itself creates.
    pub wt2: PathBuf,
}

/// Builds `Worktrees` in the scratch: `main` with one commit (`a.txt`,
/// `b.txt` at `T0`) and `.worktrees/` in its `info/exclude`, then the linked
/// worktrees `wt1` beside it and `wt2` nested inside it.
pub fn linked_worktrees(scratch: &Scratch) -> Result<Worktrees, String> {
    let main = scratch.root.join("main");
    fs::create_dir_all(&main).map_err(|err| format!("create {}: {err}", main.display()))?;
    git_ok(scratch, &main, ["init", "-q", "-b", "main"])?;
    write(&main.join("a.txt"), b"a\n")?;
    write(&main.join("b.txt"), b"b\n")?;
    commit_all(scratch, &main, "base", T0)?;

    let exclude = main.join(".git/info/exclude");
    let mut patterns = match fs::read(&exclude) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(err) => return Err(format!("read {}: {err}", exclude.display())),
    };
    if !patterns.is_empty() && !patterns.ends_with(b"\n") {
        patterns.push(b'\n');
    }
    patterns.extend_from_slice(b".worktrees/\n");
    write(&exclude, &patterns)?;

    let wt1 = scratch.root.join("wt1");
    let wt2 = main.join(".worktrees/wt2");
    git_ok(
        scratch,
        &main,
        [
            OsStr::new("worktree"),
            "add".as_ref(),
            "-q".as_ref(),
            "-b".as_ref(),
            "wt1".as_ref(),
            wt1.as_os_str(),
        ],
    )?;
    git_ok(
        scratch,
        &main,
        [
            OsStr::new("worktree"),
            "add".as_ref(),
            "-q".as_ref(),
            "-b".as_ref(),
            "wt2".as_ref(),
            wt2.as_os_str(),
        ],
    )?;

    let canonical = |path: &Path| {
        path.canonicalize()
            .map_err(|err| format!("canonicalize {}: {err}", path.display()))
    };
    Ok(Worktrees {
        main: canonical(&main)?,
        wt1: canonical(&wt1)?,
        wt2: canonical(&wt2)?,
    })
}
