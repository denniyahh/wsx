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
use std::fmt::Display;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use gix::bstr::ByteSlice;
use tempfile::TempDir;

use super::ObjectId;

/// An environment for one child, as ordered key/value pairs.
pub type Env = Vec<(OsString, OsString)>;

/// The fixture clock's start: 2026-01-01T00:00:00Z.
pub const T0: i64 = 1_767_225_600;

/// The step between consecutive fixture commits, in seconds.
pub const TICK: i64 = 60;

/// The only PATH a spawned child sees, so the host's own git is never used.
const HERMETIC_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// The scratch global git config, written to `<home>/.gitconfig`. Automatic
/// maintenance is off: git would otherwise start a detached
/// `maintenance run --auto` after commits, which writes into the fixture while
/// later steps walk it.
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
[maintenance]
\tauto = false
[gc]
\tauto = 0
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

/// The hermetic environment of the `Scratch` whose tree holds `path`:
/// `hermetic_env(root, <root>/home)` for the ancestor `root` of `path`
/// that sits directly under `scratch_root()`, which every `Scratch` root
/// does. A test that discovers a repository in its own process hands this to
/// wsx's `Git`, so wsx's children see the fixture's environment, not the
/// host's PATH and global config. `Err` when no such ancestor exists.
pub fn scratch_env_of(path: &Path) -> Result<Env, String> {
    let parent = scratch_root();
    let parent = parent
        .canonicalize()
        .map_err(|err| format!("canonicalize {}: {err}", parent.display()))?;
    path.ancestors()
        .find(|ancestor| ancestor.parent() == Some(parent.as_path()))
        .map(|root| hermetic_env(root, &root.join("home")))
        .ok_or_else(|| {
            format!(
                "{} is not inside a scratch tree under {}",
                path.display(),
                parent.display()
            )
        })
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

/// Runs git in `dir` with exactly `env`, for a test that adds variables to
/// the scratch environment; an error only when git cannot be started.
pub fn git_in_env<I, S>(env: &Env, dir: &Path, args: I) -> Result<Output, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    spawn_git(env, dir, &argv(args))
}

/// `text` as one single-quoted `/bin/sh` word.
fn sh_word(text: &OsStr) -> String {
    format!("'{}'", text.to_string_lossy().replace('\'', r"'\''"))
}

/// Writes `<hooks_dir>/<hook>`, mode 0755: a `/bin/sh` script that `exec`s
/// this unit-test binary to run the single ignored test `child` (its full
/// libtest path), output not captured. git runs it with the hook's own
/// environment and working directory, so the child sees exactly what a
/// program git started as that hook would see. A child that fails makes the
/// hook, and with a pre-commit hook the commit, fail.
pub fn install_hook(hooks_dir: &Path, hook: &str, child: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let exe =
        std::env::current_exe().map_err(|err| format!("locate the test executable: {err}"))?;
    let script = format!(
        "#!/bin/sh\nexec {} --exact {} --include-ignored --nocapture\n",
        sh_word(exe.as_os_str()),
        sh_word(OsStr::new(child))
    );
    let path = hooks_dir.join(hook);
    write(&path, script.as_bytes())?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .map_err(|err| format!("chmod 0755 {}: {err}", path.display()))
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

/// The object id git names for `rev` in `dir` (`git rev-parse --verify`).
/// Tests get ids from git, never from gix's revspec parser, so a read and
/// its oracle cannot agree because they share a parser (D-03).
pub fn object_id(scratch: &Scratch, dir: &Path, rev: &str) -> Result<ObjectId, String> {
    rev_parse(scratch, dir, rev).map(ObjectId)
}

/// `hex` decoded to bytes; an error naming it when it is not even-length hex.
fn hex_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err(format!("{hex:?} has an odd number of hex digits"));
    }
    (0..hex.len())
        .step_by(2)
        .map(|at| {
            hex.get(at..at + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .ok_or_else(|| format!("{hex:?} is not hex"))
        })
        .collect()
}

/// The id a `git hash-object` run in `dir` printed.
fn hashed(scratch: &Scratch, dir: &Path, args: &[&OsStr]) -> Result<String, String> {
    let stdout = git_ok(scratch, dir, args)?;
    String::from_utf8(stdout)
        .map(|id| id.trim_end().to_owned())
        .map_err(|err| format!("git hash-object printed non-UTF-8: {err}"))
}

/// A commit in `dir` whose root tree holds `entries` byte for byte in the
/// order given, each a `100644` file `(name, content)`, so a test can build
/// a tree out of git's order. Each blob is written with `git hash-object
/// -w`; the tree's bytes, `100644 <name>` NUL and the blob's raw id per
/// entry, with `git hash-object --literally -t tree -w`, which neither sorts
/// nor checks them; the commit with `git commit-tree` at `T0`. The bytes go
/// through files under `<scratch root>/raw-tree`. No ref moves. Returns the
/// commit's and the tree's ids.
pub fn raw_tree_commit(
    scratch: &Scratch,
    dir: &Path,
    entries: &[(&str, &[u8])],
) -> Result<(ObjectId, ObjectId), String> {
    let files = scratch.root.join("raw-tree");
    let blob_file = files.join("blob");
    let mut tree = Vec::new();
    for (name, content) in entries {
        write(&blob_file, content)?;
        let blob = hashed(
            scratch,
            dir,
            &["hash-object".as_ref(), "-w".as_ref(), blob_file.as_os_str()],
        )?;
        tree.extend_from_slice(format!("100644 {name}\0").as_bytes());
        tree.extend_from_slice(&hex_bytes(&blob)?);
    }
    let tree_file = files.join("tree");
    write(&tree_file, &tree)?;
    let tree = hashed(
        scratch,
        dir,
        &[
            "hash-object".as_ref(),
            "--literally".as_ref(),
            "-t".as_ref(),
            "tree".as_ref(),
            "-w".as_ref(),
            tree_file.as_os_str(),
        ],
    )?;
    let commit = git_ok_at(
        scratch,
        dir,
        T0,
        ["commit-tree", tree.as_str(), "-m", "tree"],
    )?;
    let commit = String::from_utf8(commit)
        .map(|id| id.trim_end().to_owned())
        .map_err(|err| format!("git commit-tree printed non-UTF-8: {err}"))?;
    Ok((ObjectId(commit), ObjectId(tree)))
}

/// The directories searched for the real git, in PATH order.
const SYSTEM_BIN_DIRS: [&str; 3] = ["/usr/local/bin", "/usr/bin", "/bin"];

/// The first executable `git` in the system bin directories.
pub fn real_git() -> PathBuf {
    SYSTEM_BIN_DIRS
        .iter()
        .map(|dir| Path::new(dir).join("git"))
        .find(|path| {
            fs::metadata(path)
                .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        })
        .unwrap_or_else(|| panic!("no executable git in {SYSTEM_BIN_DIRS:?}"))
}

/// `path` as a single-quoted `/bin/sh` word.
pub fn sh_quote(path: &Path) -> String {
    let text = path
        .to_str()
        .unwrap_or_else(|| panic!("{} is not UTF-8", path.display()));
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// Writes `<root>/fakebin/git`, a `/bin/sh` script that appends `"$*"` to
/// `<root>/git.log` and then runs `body`, and returns the scratch's
/// hermetic environment with `<root>/fakebin` first on PATH. The child
/// resolves a bare `git` through its own PATH, so the helper runs the fake.
pub fn fake_git(scratch: &Scratch, body: &str) -> Env {
    let fakebin = scratch.root.join("fakebin");
    let script = fakebin.join("git");
    let log = sh_quote(&scratch.root.join("git.log"));
    let content = format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> {log}\n{body}\n");
    write(&script, content.as_bytes()).expect("write the fake git");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755))
        .expect("make the fake git executable");
    let path = OsString::from(format!(
        "{}:{}",
        fakebin.display(),
        SYSTEM_BIN_DIRS.join(":")
    ));
    let mut env = scratch.env.clone();
    let entry = env
        .iter_mut()
        .find(|(key, _)| key == "PATH")
        .expect("the hermetic env sets PATH");
    entry.1 = path;
    env
}

/// The lines of `<root>/git.log`, one per fake git call.
pub fn fake_git_log(scratch: &Scratch) -> Vec<String> {
    let log = scratch.root.join("git.log");
    fs::read_to_string(&log)
        .unwrap_or_else(|err| panic!("read {}: {err}", log.display()))
        .lines()
        .map(str::to_owned)
        .collect()
}

/// `fake_git` whose body execs the real git, so the helper runs git as it
/// would and `fake_git_log` lists every call it made.
pub fn counting_git(scratch: &Scratch) -> Env {
    fake_git(scratch, &format!("exec {} \"$@\"", sh_quote(&real_git())))
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

/// The edge names `all_bytes_repo` adds to its `p<byte>q` names: edge space
/// and tab, inner spaces, a double quote, a backslash, raw UTF-8, raw
/// non-UTF-8, the `#`, `*` and `-` that start a word, newline, CR and DEL.
const ALL_BYTES_EDGE_NAMES: [&[u8]; 16] = [
    b" lead",
    b"trail ",
    b"\tleadtab",
    b"trailtab\t",
    b"my dir",
    b"\"q\"",
    b"back\\slash",
    b"caf\xc3\xa9",
    b"raw\xe9",
    b"#hash",
    b"*star",
    b"nl\nnl",
    b"cr\r",
    b"\x7f",
    b"-dash",
    b"a  b",
];

/// `<root>/names`: one empty file staged, not committed, for `p<byte>q` with
/// every byte 1 to 255 except `/`, and for each of `ALL_BYTES_EDGE_NAMES`,
/// 270 names in all. Fails unless `git ls-files -z` lists exactly those
/// names, so a name the filesystem or git changed cannot pass unnoticed.
pub fn all_bytes_repo(scratch: &Scratch) -> Result<PathBuf, String> {
    let repo = scratch.root.join("names");
    fs::create_dir_all(&repo).map_err(|err| format!("create {}: {err}", repo.display()))?;
    git_ok(scratch, &repo, ["init", "-q", "-b", "main"])?;
    let mut names: Vec<Vec<u8>> = (1..=u8::MAX)
        .filter(|byte| *byte != b'/')
        .map(|byte| vec![b'p', byte, b'q'])
        .collect();
    names.extend(ALL_BYTES_EDGE_NAMES.iter().map(|name| name.to_vec()));
    for name in &names {
        write(&repo.join(OsStr::from_bytes(name)), b"")?;
    }
    git_ok(scratch, &repo, ["add", "--all"])?;
    let mut listed = records(&git_ok(scratch, &repo, ["ls-files", "-z"])?, 0);
    listed.sort();
    names.sort();
    if listed != names {
        let unlisted: Vec<_> = names
            .iter()
            .filter(|name| !listed.contains(name))
            .map(|name| name.as_bstr())
            .collect();
        return Err(format!(
            "all-bytes repo: git ls-files -z lists {} names, not the {} written; unlisted: {unlisted:?}",
            listed.len(),
            names.len()
        ));
    }
    Ok(repo)
}

/// `output` split into records ending in `separator`: one per `git ls-files`
/// line with `b'\n'`, one per raw name with `0` for `-z`. The empty record
/// after the last separator is dropped.
pub fn records(output: &[u8], separator: u8) -> Vec<Vec<u8>> {
    let mut fields: Vec<Vec<u8>> = output
        .split(|byte| *byte == separator)
        .map(<[u8]>::to_vec)
        .collect();
    if fields.last().is_some_and(Vec::is_empty) {
        fields.pop();
    }
    fields
}

/// A repository with a ref named like `git describe` output, whose hex part
/// names another commit than the ref holds.
pub struct DescribeShaped {
    /// The repository, `<scratch root>/repo`, with `main` checked out.
    pub repo: PathBuf,
    /// The full ref name, `refs/remotes/origin/rel-1-g` followed by the
    /// first seven hex characters of `other`.
    pub name: String,
    /// The commit `name` and `main` hold (ONE), as hex.
    pub one: String,
    /// The commit branch `other` holds (Y, a child of ONE), as hex: the
    /// object `name`'s hex part prefixes.
    pub other: String,
}

/// Builds the describe-shaped fixture in `<scratch root>/repo` and returns
/// it, with `main` checked out.
///
/// `main` holds ONE (`a.txt`); branch `other` holds Y, a child of ONE that
/// adds `b.txt`. The ref `refs/remotes/origin/rel-1-g<first 7 hex of Y>`
/// holds ONE. git reads that name as the ref, so it names ONE. Research
/// measured on gix 0.90.0 that gix's revspec parser tries the hex prefix
/// and describe output before refs, and so reads the same name as Y.
///
/// Fails unless git reads `name` as ONE, the seven hex characters name Y
/// and only Y, and ONE differs from Y.
pub fn describe_shaped_ref(scratch: &Scratch) -> Result<DescribeShaped, String> {
    let repo = scratch.root.join("repo");
    fs::create_dir_all(&repo).map_err(|err| format!("create {}: {err}", repo.display()))?;
    git_ok(scratch, &repo, ["init", "-q", "-b", "main"])?;

    write(&repo.join("a.txt"), b"a\n")?;
    commit_all(scratch, &repo, "one", T0)?;
    let one = rev_parse(scratch, &repo, "HEAD")?;

    git_ok(scratch, &repo, ["switch", "-q", "-c", "other"])?;
    write(&repo.join("b.txt"), b"b\n")?;
    commit_all(scratch, &repo, "y", T0 + TICK)?;
    let other = rev_parse(scratch, &repo, "HEAD")?;
    git_ok(scratch, &repo, ["switch", "-q", "main"])?;

    let prefix = other
        .get(..7)
        .ok_or_else(|| format!("describe-shaped: Y's id {other:?} is shorter than 7"))?;
    let name = format!("refs/remotes/origin/rel-1-g{prefix}");
    git_ok(scratch, &repo, ["update-ref", name.as_str(), one.as_str()])?;

    let named = rev_parse(scratch, &repo, &name)?;
    if named != one {
        return Err(format!(
            "describe-shaped: git reads {name} as {named}, not as ONE {one}"
        ));
    }
    let prefixed = rev_parse(scratch, &repo, prefix)?;
    if prefixed != other {
        return Err(format!(
            "describe-shaped: git reads {prefix} as {prefixed}, not as Y {other}"
        ));
    }
    if one == other {
        return Err(format!("describe-shaped: ONE and Y are both {one}"));
    }
    Ok(DescribeShaped {
        repo,
        name,
        one,
        other,
    })
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

/// The variable a hook test hands its child: the canonical path of `wt2`,
/// the sibling the child aims at.
pub const SIBLING_VAR: &str = "WSX_TEST_SIBLING";

/// What a child prints for an environment variable that is not set.
pub const UNSET: &str = "<unset>";

/// `name` from the process environment, or `UNSET`, for a child to print.
pub fn env_or_unset(name: &str) -> String {
    std::env::var_os(name).map_or_else(
        || UNSET.to_owned(),
        |value| value.to_string_lossy().into_owned(),
    )
}

/// The error for a git run that did not exit 0, quoting both streams (a
/// hook's output reaches git's stderr).
pub fn failed_run(what: &str, output: &Output) -> String {
    format!(
        "{what} exited {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Builds `linked_worktrees`, installs a pre-commit hook in the common hooks
/// directory that re-executes `child`, changes `b.txt` in `wt1` and runs the
/// partial commit `git commit -q -m x b.txt` there, with the scratch
/// environment plus `SIBLING_VAR=<wt2>`. git exports a temporary
/// `next-index-<pid>.lock` as the hook's `GIT_INDEX_FILE` for a partial
/// commit. An error, quoting the hook's output, unless the commit exited 0:
/// a child that fails fails the hook and so the commit.
pub fn linked_hook_commit(scratch: &Scratch, child: &str) -> Result<(Worktrees, Output), String> {
    let worktrees = linked_worktrees(scratch)?;
    install_hook(&worktrees.main.join(".git/hooks"), "pre-commit", child)?;
    write(&worktrees.wt1.join("b.txt"), b"b in wt1\n")?;
    let mut env = scratch.env.clone();
    env.push((SIBLING_VAR.into(), worktrees.wt2.clone().into()));
    let output = git_in_env(&env, &worktrees.wt1, ["commit", "-q", "-m", "x", "b.txt"])?;
    if output.status.code() != Some(0) {
        return Err(failed_run("the partial commit in wt1", &output));
    }
    Ok((worktrees, output))
}

/// `path` with its parent directory canonicalised and its file name kept:
/// how an index path is compared, because a lock file such as
/// `next-index-<pid>.lock` is gone once its commit ends, so the file itself
/// cannot be canonicalised afterwards. A relative `path` is taken against
/// `base`.
pub fn canonical_parent(base: &Path, path: &Path) -> Result<PathBuf, String> {
    let path = base.join(path);
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(format!("{} has no parent or no file name", path.display()));
    };
    let parent = parent
        .canonicalize()
        .map_err(|err| format!("canonicalize {}: {err}", parent.display()))?;
    Ok(parent.join(name))
}

/// The prefix of every fact line a re-executed child prints.
///
/// A child half reports what it saw as `WSXCHILD <key>=<value>`, and its
/// parent asserts that line. The parent never trusts the exit status alone:
/// re-executing the test binary with a child name that matches no test exits
/// 0 after `running 0 tests` (`reexec_of_an_unknown_child_has_no_marker`).
pub const MARKER: &str = "WSXCHILD ";

/// Prints one `WSXCHILD <key>=<value>` line to stdout, for the parent.
pub fn child_print(key: &str, value: impl Display) {
    println!("{MARKER}{key}={value}");
}

/// The process `reexec` and `reexec_within` run: this unit-test binary
/// running only `child`, ignored tests included, output not captured, with
/// an empty environment plus `env`, `cwd` as its working directory and
/// every stream piped.
fn spawn_child(child: &str, env: &Env, cwd: &Path) -> Result<std::process::Child, String> {
    let exe =
        std::env::current_exe().map_err(|err| format!("locate the test executable: {err}"))?;
    Command::new(&exe)
        .args(["--exact", child, "--include-ignored", "--nocapture"])
        .env_clear()
        .envs(env.iter().map(|(key, value)| (key, value)))
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("spawn {} --exact {child}: {err}", exe.display()))
}

/// Re-executes this unit-test binary to run the single test `child` (its full
/// libtest path, for example `git::spawn::tests::null_stdin_child`), ignored
/// tests included, with output not captured.
///
/// The child gets an empty environment plus `env`, `cwd` as its working
/// directory, and a piped stdin that receives `stdin` and is then closed, so
/// a subject that reads the process environment or stdin runs against values
/// the parent chose, without `set_var`. stdout and stderr are captured. An
/// error only when the child cannot be started, fed or waited on.
pub fn reexec(child: &str, env: &Env, cwd: &Path, stdin: &[u8]) -> Result<Output, String> {
    let mut process = spawn_child(child, env, cwd)?;
    let mut pipe = process
        .stdin
        .take()
        .ok_or_else(|| format!("{child}: the child has no stdin pipe"))?;
    // Fed from another thread, so a child that writes a lot before it reads
    // (or never reads) cannot deadlock against this one. Dropping the pipe
    // closes the child's stdin.
    std::thread::scope(|scope| {
        let feeder = scope.spawn(move || pipe.write_all(stdin));
        let output = process
            .wait_with_output()
            .map_err(|err| format!("wait for {child}: {err}"))?;
        feeder
            .join()
            .map_err(|_| format!("{child}: the stdin feeder panicked"))?
            .map_err(|err| format!("write {} stdin bytes to {child}: {err}", stdin.len()))?;
        Ok(output)
    })
}

/// How often `reexec_within` checks whether its child has exited.
const WAIT_POLL: std::time::Duration = std::time::Duration::from_millis(20);

/// Reads all of `stream` on its own thread, so a child that fills one pipe
/// never blocks on it while the parent waits.
fn drain(mut stream: impl std::io::Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        // A read error ends the capture early; the parent reports what it got.
        let _ = stream.read_to_end(&mut bytes);
        bytes
    })
}

/// `reexec`'s child, given at most `limit` to exit. Past it the child is
/// killed and reaped, and the answer is `Err` saying it did not finish
/// within `limit`, so a subject that hangs fails its test instead of
/// hanging the suite. The streams are fed and drained on threads that are
/// not joined after a kill, so a grandchild still holding a pipe cannot
/// block the parent either.
pub fn reexec_within(
    child: &str,
    env: &Env,
    cwd: &Path,
    stdin: &[u8],
    limit: std::time::Duration,
) -> Result<Output, String> {
    let mut process = spawn_child(child, env, cwd)?;
    let (Some(mut pipe), Some(stdout), Some(stderr)) = (
        process.stdin.take(),
        process.stdout.take(),
        process.stderr.take(),
    ) else {
        return Err(format!("{child}: the child is missing a piped stream"));
    };
    let fed = stdin.to_vec();
    let feeder = std::thread::spawn(move || pipe.write_all(&fed));
    let (stdout, stderr) = (drain(stdout), drain(stderr));
    let started = std::time::Instant::now();
    let status = loop {
        match process.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= limit => {
                let killed = process.kill().and_then(|()| process.wait());
                return Err(format!(
                    "{child} did not finish within {limit:?}; killed it ({killed:?})"
                ));
            }
            Ok(None) => std::thread::sleep(WAIT_POLL),
            Err(err) => return Err(format!("wait for {child}: {err}")),
        }
    };
    feeder
        .join()
        .map_err(|_| format!("{child}: the stdin feeder panicked"))?
        .map_err(|err| format!("write {} stdin bytes to {child}: {err}", stdin.len()))?;
    let joined = |reader: std::thread::JoinHandle<Vec<u8>>, name: &str| {
        reader
            .join()
            .map_err(|_| format!("{child}: the {name} reader panicked"))
    };
    Ok(Output {
        status,
        stdout: joined(stdout, "stdout")?,
        stderr: joined(stderr, "stderr")?,
    })
}

/// The value after `WSXCHILD <key>=` on its first line in the child's stdout,
/// else in its stderr (a hook's stdout reaches git's stderr). An error quoting
/// both streams when neither has the line.
pub fn marker(output: &Output, key: &str) -> Result<String, String> {
    let prefix = format!("{MARKER}{key}=");
    let find = |stream: &[u8]| {
        String::from_utf8_lossy(stream).lines().find_map(|line| {
            line.find(&prefix)
                .map(|start| line[start + prefix.len()..].to_owned())
        })
    };
    find(&output.stdout)
        .or_else(|| find(&output.stderr))
        .ok_or_else(|| {
            format!(
                "no `{prefix}` line from the child (exit {:?})\n--- stdout ---\n{}\n--- stderr ---\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        })
}

/// Set in the environment of a test `rerun_scrubbed` re-executes, so the
/// re-executed test reports that it ran.
const RERUN_VAR: &str = "WSX_TEST_RERUN";

/// The names `git rev-parse --local-env-vars` prints, the variables that aim
/// git at one repository (`GIT_DIR`, `GIT_INDEX_FILE`, ...). An error when
/// git fails or the list lacks `GIT_DIR`, so an empty list cannot turn the
/// scrub into a no-op.
fn local_env_vars() -> Result<Vec<OsString>, String> {
    let env: Env = vec![
        ("PATH".into(), HERMETIC_PATH.into()),
        ("LC_ALL".into(), "C".into()),
    ];
    let args = argv(["rev-parse", "--local-env-vars"]);
    let root = Path::new("/");
    let stdout = checked(root, &args, spawn_git(&env, root, &args)?)?;
    let names: Vec<OsString> = stdout
        .split(|byte| *byte == b'\n')
        .filter(|name| !name.is_empty())
        .map(|name| OsStr::from_bytes(name).to_os_string())
        .collect();
    if names.iter().any(|name| name == "GIT_DIR") {
        Ok(names)
    } else {
        Err(format!(
            "git rev-parse --local-env-vars did not list GIT_DIR: {names:?}"
        ))
    }
}

/// Keeps the repository-local git variables of the test runner away from a
/// test that discovers a repository in its own process: a suite run from a
/// git hook, or a shell that exports `GIT_DIR`, would otherwise aim
/// `Repo::discover` at that repository instead of the test's fixture.
///
/// Call it first in such a test, as `if testing::rerun_scrubbed() { return;
/// }`. When the process environment holds none of the variables
/// `git rev-parse --local-env-vars` lists, it returns `false` and the test
/// runs here. Otherwise it re-executes the calling test with the same
/// environment minus those variables, relays the child's output, panics
/// with it unless the child exited 0 and reported that it ran the test, and
/// returns `true`. The test is named by its thread: libtest runs each test
/// on a thread named after its full path.
pub fn rerun_scrubbed() -> bool {
    let local = local_env_vars().unwrap_or_else(|err| panic!("{err}"));
    let test = std::thread::current()
        .name()
        .filter(|name| name.contains("::"))
        .map(str::to_owned)
        .unwrap_or_else(|| {
            panic!("rerun_scrubbed must run on libtest's thread, named after the test's full path")
        });
    if !std::env::vars_os().any(|(key, _)| local.contains(&key)) {
        if std::env::var_os(RERUN_VAR).is_some() {
            child_print("rerun", &test);
        }
        return false;
    }
    let mut env: Env = std::env::vars_os()
        .filter(|(key, _)| !local.contains(key))
        .collect();
    env.push((RERUN_VAR.into(), "1".into()));
    let cwd = std::env::current_dir().expect("read the working directory");
    let output = reexec(&test, &env, &cwd, b"").unwrap_or_else(|err| panic!("{err}"));
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    let what = format!("{test}, re-executed without the repository-local git variables");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        failed_run(&what, &output)
    );
    assert_eq!(
        marker(&output, "rerun").as_deref(),
        Ok(test.as_str()),
        "{}",
        failed_run(&what, &output)
    );
    true
}

/// The user-namespace runner for a non-root test: inner uid 0 is the test
/// user, inner uid 1 the first subordinate uid. Absolute, because a user
/// PATH can resolve a different `unshare` first.
const UNSHARE: &str = "/usr/bin/unshare";

/// The arguments before the inner command of every `UNSHARE` run.
const UNSHARE_ARGS: [&str; 3] = ["--map-root-user", "--map-users=auto", "--map-groups=auto"];

/// The owner a non-root test gives a fixture: inner uid and gid 1, which
/// `--map-users=auto` maps to the first subordinate id.
const INNER_FOREIGN: &str = "1:1";

/// The owner a root test gives a fixture: `nobody`.
const ROOT_FOREIGN: &str = "65534:65534";

/// A fixture really owned by another uid until it is restored.
///
/// `new` makes the tree at `path` world-readable, then hands it to another
/// uid: as root with `chown -R 65534:65534`, otherwise with `chown -R 1:1`
/// inside `/usr/bin/unshare --map-root-user --map-users=auto
/// --map-groups=auto`. When neither works it panics naming what is missing:
/// a foreign-owner test fails, it never skips. `restore` gives the tree back
/// to its original owner, and `Drop` does so when a test did not, so the
/// scratch directory stays deletable. Declare the guard after the `Scratch`
/// that holds `path`, so the guard drops first.
pub struct ForeignOwned {
    /// The root of the foreign-owned tree.
    path: PathBuf,
    /// The original owner, which is the test's own uid.
    uid: u32,
    /// The original group.
    gid: u32,
    /// Whether ownership was given back.
    restored: bool,
}

impl ForeignOwned {
    /// Makes the tree at `path`, owned by the test user, owned by another
    /// uid; panics with the failing command and what is missing when that
    /// cannot be done, or when the owner did not change.
    pub fn new(path: &Path) -> ForeignOwned {
        use std::os::unix::fs::MetadataExt;

        let (uid, gid) = owner(path).unwrap_or_else(|err| panic!("{err}"));
        run_tool(&["chmod", "-R", "go+rX"], path).unwrap_or_else(|err| panic!("{err}"));
        // Built before the chown: a partial chown must be given back too.
        let guard = ForeignOwned {
            path: path.to_path_buf(),
            uid,
            gid,
            restored: false,
        };
        let changed = if uid == 0 {
            run_tool(&["chown", "-R", ROOT_FOREIGN], path)
        } else {
            run_tool(&unshare_chown(INNER_FOREIGN), path)
        };
        if let Err(err) = changed {
            panic!(
                "cannot make {} foreign-owned: {err}\n{}",
                path.display(),
                missing_mechanism(uid)
            );
        }
        let now = fs::metadata(path)
            .unwrap_or_else(|err| panic!("stat {}: {err}", path.display()))
            .uid();
        assert_ne!(
            now,
            uid,
            "{} is still owned by uid {uid} after the chown\n{}",
            path.display(),
            missing_mechanism(uid)
        );
        guard
    }

    /// Gives the tree back to its original owner recursively and checks the
    /// owner is the test's uid again.
    pub fn restore(&mut self) -> Result<(), String> {
        if self.uid == 0 {
            run_tool(
                &["chown", "-R", &format!("{}:{}", self.uid, self.gid)],
                &self.path,
            )?;
        } else {
            // Inner root is the test user and its primary group.
            run_tool(&unshare_chown("0:0"), &self.path)?;
        }
        let (now, _) = owner(&self.path)?;
        if now != self.uid {
            return Err(format!(
                "{} is owned by uid {now} after the restore, expected {}",
                self.path.display(),
                self.uid
            ));
        }
        self.restored = true;
        Ok(())
    }
}

impl Drop for ForeignOwned {
    fn drop(&mut self) {
        if !self.restored
            && let Err(err) = self.restore()
        {
            eprintln!(
                "warning: could not give {} back to uid {}: {err}",
                self.path.display(),
                self.uid
            );
        }
    }
}

/// The uid and gid owning `path`.
fn owner(path: &Path) -> Result<(u32, u32), String> {
    use std::os::unix::fs::MetadataExt;

    fs::metadata(path)
        .map(|meta| (meta.uid(), meta.gid()))
        .map_err(|err| format!("stat {}: {err}", path.display()))
}

/// The argv of `chown -R <ids>` run as inner root of a user namespace.
fn unshare_chown(ids: &str) -> Vec<&str> {
    let mut args = vec![UNSHARE];
    args.extend(UNSHARE_ARGS);
    args.extend(["chown", "-R", ids]);
    args
}

/// Runs `argv` followed by `path` with only the hermetic PATH and `LC_ALL=C`,
/// in `/`; an error naming the command, its exit status and stderr unless it
/// exits 0.
fn run_tool(argv: &[&str], path: &Path) -> Result<(), String> {
    let env: Env = vec![
        ("PATH".into(), HERMETIC_PATH.into()),
        ("LC_ALL".into(), "C".into()),
    ];
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| "run_tool: empty argv".to_owned())?;
    let rendered = format!("{} {}", argv.join(" "), path.display());
    let output = command(program, Path::new("/"), &env)
        .args(args)
        .arg(path)
        .output()
        .map_err(|err| format!("spawn `{rendered}`: {err}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{rendered}` exited {}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim_end()
        ))
    }
}

/// What a foreign-owner fixture needs that this host lacks, as a sentence.
/// Root needs only the `CHOWN` capability; a user needs `/usr/bin/unshare`,
/// `newuidmap` and `newgidmap`, and subordinate ids in `/etc/subuid` and
/// `/etc/subgid`.
fn missing_mechanism(uid: u32) -> String {
    if uid == 0 {
        return "running as root: the chown needs the CHOWN capability".to_owned();
    }
    let mut missing = Vec::new();
    for tool in [UNSHARE, "/usr/bin/newuidmap", "/usr/bin/newgidmap"] {
        if !Path::new(tool).exists() {
            missing.push(tool.to_owned());
        }
    }
    let name = Command::new("/usr/bin/id")
        .arg("-un")
        .env_clear()
        .stdin(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned());
    for file in ["/etc/subuid", "/etc/subgid"] {
        let listed = fs::read_to_string(file).is_ok_and(|content| {
            content.lines().any(|line| {
                let owner = line.split(':').next().unwrap_or("");
                owner == uid.to_string() || name.as_deref() == Some(owner)
            })
        });
        if !listed {
            missing.push(format!("an {file} entry for uid {uid}"));
        }
    }
    if missing.is_empty() {
        "every prerequisite is present (unshare, newuidmap, newgidmap, subordinate ids); \
         user namespaces may be restricted on this host"
            .to_owned()
    } else {
        format!("missing: {}", missing.join(", "))
    }
}

#[test]
fn foreign_guard_changes_and_restores_the_owner() {
    use std::os::unix::fs::MetadataExt;

    let scratch = Scratch::new("foreign-guard").expect("scratch");
    let repo = scratch.root.join("repo");
    let repo_arg = repo.to_str().expect("UTF-8 scratch path");
    git_ok(&scratch, &scratch.root, ["init", "-q", repo_arg]).expect("git init");
    let head = repo.join(".git/HEAD");
    let uid_of = |path: &Path| {
        fs::metadata(path)
            .unwrap_or_else(|err| panic!("stat {}: {err}", path.display()))
            .uid()
    };
    let uid = uid_of(&repo);

    let mut guard = ForeignOwned::new(&repo);
    assert_ne!(uid_of(&repo), uid, "the repository is still the test's");
    assert_ne!(uid_of(&head), uid, ".git/HEAD is still the test's");

    guard.restore().expect("restore the owner");
    assert_eq!(uid_of(&repo), uid, "the repository was not given back");
    assert_eq!(uid_of(&head), uid, ".git/HEAD was not given back");
}

#[test]
fn reexec_of_an_unknown_child_has_no_marker() {
    let scratch = Scratch::new("reexec-unknown").expect("scratch");
    let output = reexec(
        "git::testing::no_such_child",
        &scratch.env,
        &scratch.root,
        b"",
    )
    .expect("re-execute the test binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a child name matching no test exits 0; stdout: {stdout}"
    );
    assert!(
        stdout.contains("running 0 tests"),
        "the unknown child ran a test; stdout: {stdout}"
    );
    assert!(
        marker(&output, "anything").is_err(),
        "a zero-test run produced a marker; stdout: {stdout}"
    );
}

/// How long `sleeping_child` sleeps: far past the limit its test gives it.
const SLEEP_FOR: std::time::Duration = std::time::Duration::from_secs(30);

/// The limit `reexec_within_kills_a_child_that_outlives_its_limit` gives
/// `sleeping_child`, and the limit its control finishes within.
const SHORT_LIMIT: std::time::Duration = std::time::Duration::from_secs(1);
const ROOMY_LIMIT: std::time::Duration = std::time::Duration::from_secs(10);

#[test]
#[ignore = "child half of reexec_within_kills_a_child_that_outlives_its_limit; runs only when re-executed"]
fn sleeping_child() {
    std::thread::sleep(SLEEP_FOR);
}

/// A child that outlives its limit is killed and reported, well before it
/// would have finished. Control: a child that exits at once (an unknown name,
/// which runs no test) finishes within the roomier limit.
#[test]
fn reexec_within_kills_a_child_that_outlives_its_limit() {
    let scratch = Scratch::new("reexec-within").expect("scratch");
    let started = std::time::Instant::now();
    match reexec_within(
        "git::testing::sleeping_child",
        &scratch.env,
        &scratch.root,
        b"",
        SHORT_LIMIT,
    ) {
        Err(err) => assert!(err.contains("did not finish within"), "{err}"),
        Ok(output) => panic!("the sleeping child finished: {output:?}"),
    }
    assert!(
        started.elapsed() < ROOMY_LIMIT,
        "the sleeping child was waited for {:?}, not killed",
        started.elapsed()
    );

    let output = reexec_within(
        "git::testing::no_such_child",
        &scratch.env,
        &scratch.root,
        b"",
        ROOMY_LIMIT,
    )
    .unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

/// A one-commit repository at `<root>/<name>`, canonical.
fn one_commit_repo(scratch: &Scratch, name: &str) -> PathBuf {
    let dir = scratch.root.join(name);
    fs::create_dir_all(&dir).unwrap_or_else(|err| panic!("create {name}: {err}"));
    git_ok(scratch, &dir, ["init", "-q"]).unwrap_or_else(|err| panic!("{err}"));
    write(&dir.join("a.txt"), b"a\n").unwrap_or_else(|err| panic!("{err}"));
    commit_all(scratch, &dir, name, T0).unwrap_or_else(|err| panic!("{err}"));
    dir.canonicalize()
        .unwrap_or_else(|err| panic!("canonicalize {name}: {err}"))
}

/// The libtest filters for the modules whose tests discover a repository in
/// the test process itself.
const DISCOVERING_SUITES: [&str; 8] = [
    "git::config::tests::",
    "git::objects::tests::",
    "git::reads::tests::",
    "git::refs::tests::",
    "git::repo::tests::",
    "git::staleness::tests::",
    "git::trees::tests::",
    "model::tests::",
];

/// The `test result:` line and every failed test name in a libtest run's
/// stdout, for a failure message.
fn libtest_report(stdout: &str) -> String {
    stdout
        .lines()
        .filter(|line| line.starts_with("test result:") || line.ends_with("... FAILED"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The passed and failed counts of a libtest run's `test result:` line.
fn libtest_counts(stdout: &str) -> Option<(usize, usize)> {
    let result = stdout
        .lines()
        .find(|line| line.starts_with("test result:"))?;
    let count = |label: &str| {
        result
            .split(';')
            .find_map(|part| part.trim().strip_suffix(label))
            .and_then(|number| number.trim().rsplit(' ').next()?.parse().ok())
    };
    Some((count(" passed")?, count(" failed")?))
}

/// Every test that discovers a repository in its own process passes when
/// the runner's environment carries another repository's `GIT_DIR` and
/// `GIT_INDEX_FILE`, as a suite run from inside a git hook does: those
/// suites are re-executed with the runner's environment plus both
/// variables aimed at a decoy repository. Each suite runs on its own and
/// must pass at least one test, so a filter that matches nothing fails
/// rather than hiding behind the others' counts.
#[test]
fn discovering_tests_pass_with_a_runner_git_dir() {
    let scratch = Scratch::new("runner-git-dir").expect("scratch");
    let git_dir = one_commit_repo(&scratch, "decoy").join(".git");

    let exe = std::env::current_exe().expect("locate the test executable");
    for suite in DISCOVERING_SUITES {
        // Not `command`: the point is the runner's own environment,
        // inherited, plus the two variables a hook would export.
        let output = Command::new(&exe)
            .arg(suite)
            .env("GIT_DIR", &git_dir)
            .env("GIT_INDEX_FILE", git_dir.join("index"))
            .stdin(Stdio::null())
            .output()
            .unwrap_or_else(|err| panic!("re-execute the suite {suite}: {err}"));
        let stdout = String::from_utf8_lossy(&output.stdout);
        let counts = libtest_counts(&stdout);
        assert!(
            output.status.code() == Some(0) && matches!(counts, Some((passed, 0)) if passed > 0),
            "with GIT_DIR={} the suite {suite} exited {:?}, counts (passed, failed) \
             {counts:?}\n{}",
            git_dir.display(),
            output.status.code(),
            libtest_report(&stdout)
        );
    }
}

/// The modules under `src/git` and `src/model` whose tests call
/// `rerun_scrubbed()` are exactly the ones `DISCOVERING_SUITES` lists, so a
/// new discovering module gets its hook-environment run and a removed one
/// leaves no stale entry. `src/<dir>/mod.rs` maps to `<dir>::tests::` and
/// `src/<dir>/<stem>.rs` to `<dir>::<stem>::tests::`. This module is left
/// out: its callers are children re-executed on purpose.
#[test]
fn discovering_suites_are_the_modules_that_rerun_scrubbed() {
    let mut callers = std::collections::BTreeSet::new();
    for module in ["git", "model"] {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join(module);
        let entries =
            fs::read_dir(&dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
        for entry in entries {
            let path = entry
                .unwrap_or_else(|err| panic!("read an entry of {}: {err}", dir.display()))
                .path();
            if path.extension() != Some(OsStr::new("rs")) {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(OsStr::to_str)
                .unwrap_or_else(|| panic!("{} has no UTF-8 stem", path.display()));
            if module == "git" && stem == "testing" {
                continue;
            }
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
            if source.contains("rerun_scrubbed() {") {
                callers.insert(if stem == "mod" {
                    format!("{module}::tests::")
                } else {
                    format!("{module}::{stem}::tests::")
                });
            }
        }
    }
    let listed: std::collections::BTreeSet<String> = DISCOVERING_SUITES
        .iter()
        .map(|suite| (*suite).to_owned())
        .collect();
    assert!(
        !callers.is_empty(),
        "no module under src/git or src/model calls rerun_scrubbed()"
    );
    assert_eq!(
        callers, listed,
        "modules calling rerun_scrubbed() against DISCOVERING_SUITES"
    );
}

/// The variable naming the directory a discovery child discovers from.
const DISCOVER_VAR: &str = "WSX_TEST_DISCOVER";

/// The full libtest path of a child half in this module.
fn testing_child(name: &str) -> String {
    format!("git::testing::{name}")
}

/// A fixture and a decoy repository, and the scratch environment plus
/// `DISCOVER_VAR=<fixture>` and the decoy's `GIT_DIR` and `GIT_INDEX_FILE`,
/// as a git hook in the decoy would export them.
fn decoy_env(scratch: &Scratch) -> (PathBuf, PathBuf, Env) {
    let fixture = one_commit_repo(scratch, "fixture");
    let decoy_git_dir = one_commit_repo(scratch, "decoy").join(".git");
    let mut env = scratch.env.clone();
    env.push((DISCOVER_VAR.into(), fixture.clone().into()));
    env.push(("GIT_DIR".into(), decoy_git_dir.clone().into()));
    env.push(("GIT_INDEX_FILE".into(), decoy_git_dir.join("index").into()));
    (fixture, decoy_git_dir, env)
}

/// Re-executes the discovery child `child` with `env` from the decoy's
/// worktree root, where git runs a hook (and where `Repo::discover` accepts
/// a `GIT_DIR` given without `GIT_WORK_TREE`), checks it exited 0, and
/// returns its output.
fn run_discovery_child(decoy_git_dir: &Path, child: &str, env: &Env) -> Output {
    let cwd = decoy_git_dir
        .parent()
        .unwrap_or_else(|| panic!("{} has no parent", decoy_git_dir.display()));
    let output = reexec(&testing_child(child), env, cwd, b"").unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        failed_run(child, &output)
    );
    output
}

/// The `git_dir` fact of a discovery child, failing with its output.
fn discovered_git_dir(output: &Output) -> String {
    marker(output, "git_dir").unwrap_or_else(|err| panic!("{err}"))
}

/// Prints the git dir `Repo::discover` finds from `DISCOVER_VAR`.
fn print_discovered_git_dir() {
    let start = std::env::var_os(DISCOVER_VAR).expect("the parent set the start directory");
    let repo = super::repo::Repo::discover(Path::new(&start))
        .unwrap_or_else(|err| panic!("discover {}: {err}", Path::new(&start).display()));
    child_print("git_dir", repo.git_dir().display());
}

/// With a decoy's `GIT_DIR` in the runner's environment, a test that calls
/// `rerun_scrubbed` first is re-executed without it and discovers its own
/// fixture.
#[test]
fn rerun_scrubbed_discovers_the_fixture_under_a_runner_git_dir() {
    let scratch = Scratch::new("rerun-scrubbed").expect("scratch");
    let (fixture, decoy, env) = decoy_env(&scratch);
    let child = "scrubbed_discovery_child";
    let output = run_discovery_child(&decoy, child, &env);
    assert_eq!(
        marker(&output, "rerun").as_deref(),
        Ok(testing_child(child).as_str()),
        "{}",
        failed_run(child, &output)
    );
    assert_eq!(
        discovered_git_dir(&output),
        fixture.join(".git").display().to_string(),
        "{}",
        failed_run(child, &output)
    );
}

/// Control: the same environment makes a test that does not call
/// `rerun_scrubbed` discover the decoy, so the decoy is a real hazard.
#[test]
fn a_runner_git_dir_redirects_unscrubbed_discovery() {
    let scratch = Scratch::new("rerun-unscrubbed").expect("scratch");
    let (_fixture, decoy, env) = decoy_env(&scratch);
    let child = "unscrubbed_discovery_child";
    let output = run_discovery_child(&decoy, child, &env);
    assert_eq!(
        discovered_git_dir(&output),
        decoy.display().to_string(),
        "{}",
        failed_run(child, &output)
    );
}

#[test]
#[ignore = "child half of rerun_scrubbed_discovers_the_fixture_under_a_runner_git_dir; runs only when re-executed"]
fn scrubbed_discovery_child() {
    if rerun_scrubbed() {
        return;
    }
    print_discovered_git_dir();
}

#[test]
#[ignore = "child half of a_runner_git_dir_redirects_unscrubbed_discovery; runs only when re-executed"]
fn unscrubbed_discovery_child() {
    print_discovered_git_dir();
}

/// Whether an empty commit in `repo`, with `config` given through `-c`,
/// starts git's background `maintenance run --auto`, read from the commit's
/// trace2 event log.
fn commit_starts_auto_maintenance(scratch: &Scratch, repo: &Path, config: &[&str]) -> bool {
    let trace = scratch.root.join("commit.trace2.json");
    let mut env = scratch.env.clone();
    env.push(("GIT_TRACE2_EVENT".into(), trace.clone().into_os_string()));
    let mut args: Vec<&str> = config.iter().flat_map(|pair| ["-c", *pair]).collect();
    args.extend(["commit", "--allow-empty", "-q", "-m", "traced"]);
    let output = command("git", repo, &env)
        .args(&args)
        .output()
        .unwrap_or_else(|err| panic!("spawn git {args:?}: {err}"));
    assert_eq!(
        output.status.code(),
        Some(0),
        "git {args:?}; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events =
        fs::read_to_string(&trace).unwrap_or_else(|err| panic!("read {}: {err}", trace.display()));
    events.contains(r#""maintenance","run","--auto""#)
}

#[test]
fn fixture_commits_start_no_background_maintenance() {
    // A detached maintenance run writes objects/maintenance.lock into the
    // fixture while later steps (the ForeignOwned chmod -R walk) read it.
    let scratch = Scratch::new("no-auto-maintenance").expect("scratch");
    let repo = one_commit_repo(&scratch, "repo");
    assert!(
        !commit_starts_auto_maintenance(&scratch, &repo, &[]),
        "a commit under the scratch config must not start auto maintenance"
    );
}

#[test]
fn auto_maintenance_start_is_visible_in_the_trace() {
    // The counterpart: with maintenance.auto back on, the same trace shows
    // the child, so its absence above is not a blind check.
    let scratch = Scratch::new("auto-maintenance-on").expect("scratch");
    let repo = one_commit_repo(&scratch, "repo");
    assert!(
        // Not detached, so no maintenance run outlives the scratch tree.
        commit_starts_auto_maintenance(
            &scratch,
            &repo,
            &["maintenance.auto=true", "maintenance.autoDetach=false"]
        ),
        "a commit with maintenance.auto=true must show the maintenance child in trace2"
    );
}

#[test]
fn scratch_env_of_a_fixture_path_is_its_scratch_env() {
    let scratch = Scratch::new("scratch-env-of").expect("scratch");
    assert_eq!(
        scratch_env_of(&scratch.root.join("home")),
        Ok(scratch.env.clone())
    );
    assert!(
        scratch_env_of(Path::new("/")).is_err(),
        "/ is not inside a scratch tree"
    );
}
