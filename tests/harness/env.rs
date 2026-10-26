//! The hermetic environment every process the harness spawns runs in.
//!
//! Each scenario gets its own scratch root: a `tempfile::TempDir` under
//! `CARGO_TARGET_TMPDIR` (on disk, never the RAM-backed `/tmp`), named
//! `harness-<module>-<name>-` so a leftover is attributable. Inside it live the
//! scenario's HOME, XDG dirs, global git config and an empty system config.
//!
//! `hermetic_env` is a pure function of the inherited environment and those
//! dirs, so a test can feed it a planted host environment without mutating the
//! process. `command` is the only way the harness builds a child: it clears the
//! environment first, so nothing from the host (`GIT_DIR`, `GIT_CONFIG_*`,
//! `SSH_AUTH_SOCK`, the user's PATH) reaches git, bash or wsx.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::TempDir;

/// Where every scenario TempDir is created: on disk, inside `target/`.
const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");

/// The only PATH a spawned process sees. Built from scratch so the host's
/// linuxbrew bash and git, first on the user's PATH, are never picked up.
const HERMETIC_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// The harness's global git config, written to `<home>/.gitconfig`: the
/// default location, so it still applies after the oracle's pre-push hook
/// unsets `GIT_CONFIG_GLOBAL`.
const GLOBAL_CONFIG: &str = "\
[user]
\tname = Harness
\temail = harness@example.invalid
[init]
\tdefaultBranch = develop
[commit]
\tgpgsign = false
[tag]
\tgpgsign = false
";

/// An environment for one child, as ordered key/value pairs.
pub type Env = Vec<(OsString, OsString)>;

/// One scenario's scratch tree. Dropping it removes the whole tree.
pub struct ScenarioDirs {
    /// The canonical TempDir path; fixtures, remotes and the oracle copy live here.
    pub root: PathBuf,
    pub home: PathBuf,
    pub xdg_config: PathBuf,
    pub xdg_data: PathBuf,
    pub xdg_cache: PathBuf,
    pub xdg_state: PathBuf,
    /// An empty file `GIT_CONFIG_SYSTEM` points at.
    pub system_config: PathBuf,
    /// Held only so the tree lives as long as the dirs and is removed on drop.
    _temp: TempDir,
}

impl ScenarioDirs {
    /// Creates `harness-<module>-<name>-*` under `CARGO_TARGET_TMPDIR`, with an
    /// empty HOME holding the harness global config, empty XDG dirs and an
    /// empty system config file.
    pub fn new(module: &str, name: &str) -> Result<ScenarioDirs, String> {
        let prefix = format!("harness-{module}-{name}-");
        let temp = TempDir::with_prefix_in(&prefix, TMP_ROOT)
            .map_err(|err| format!("create a TempDir {prefix}* under {TMP_ROOT}: {err}"))?;
        let root = temp
            .path()
            .canonicalize()
            .map_err(|err| format!("canonicalize {}: {err}", temp.path().display()))?;
        let dirs = ScenarioDirs {
            home: root.join("home"),
            xdg_config: root.join("xdg/config"),
            xdg_data: root.join("xdg/data"),
            xdg_cache: root.join("xdg/cache"),
            xdg_state: root.join("xdg/state"),
            system_config: root.join("system.gitconfig"),
            root,
            _temp: temp,
        };
        for dir in [
            &dirs.home,
            &dirs.xdg_config,
            &dirs.xdg_data,
            &dirs.xdg_cache,
            &dirs.xdg_state,
        ] {
            fs::create_dir_all(dir).map_err(|err| format!("create {}: {err}", dir.display()))?;
        }
        write_file(&dirs.home.join(".gitconfig"), GLOBAL_CONFIG.as_bytes())?;
        write_file(&dirs.system_config, b"")?;
        Ok(dirs)
    }
}

fn write_file(path: &Path, content: &[u8]) -> Result<(), String> {
    fs::write(path, content).map_err(|err| format!("write {}: {err}", path.display()))
}

/// The environment of the test process itself. Read-only: nothing here mutates
/// the process environment.
pub fn inherited_env() -> Env {
    std::env::vars_os().collect()
}

/// The complete environment of a spawned child: the fixed harness keys, then
/// every inherited pair whose key starts with `LLVM_` (so coverage still sees
/// the child). Nothing else from `inherited` is used.
pub fn hermetic_env(inherited: &[(OsString, OsString)], dirs: &ScenarioDirs) -> Env {
    let pair = |key: &str, value: &OsStr| (OsString::from(key), value.to_os_string());
    let mut env = vec![
        pair("HOME", dirs.home.as_os_str()),
        pair(
            "GIT_CONFIG_GLOBAL",
            dirs.home.join(".gitconfig").as_os_str(),
        ),
        pair("GIT_CONFIG_SYSTEM", dirs.system_config.as_os_str()),
        pair("GIT_CONFIG_NOSYSTEM", OsStr::new("1")),
        pair("XDG_CONFIG_HOME", dirs.xdg_config.as_os_str()),
        pair("XDG_DATA_HOME", dirs.xdg_data.as_os_str()),
        pair("XDG_CACHE_HOME", dirs.xdg_cache.as_os_str()),
        pair("XDG_STATE_HOME", dirs.xdg_state.as_os_str()),
        pair("GIT_CEILING_DIRECTORIES", dirs.root.as_os_str()),
        pair("GIT_ALLOW_PROTOCOL", OsStr::new("file")),
        pair("GIT_TERMINAL_PROMPT", OsStr::new("0")),
        pair("GIT_EDITOR", OsStr::new("false")),
        pair("GIT_SEQUENCE_EDITOR", OsStr::new("false")),
        pair("GIT_MERGE_AUTOEDIT", OsStr::new("no")),
        pair("LC_ALL", OsStr::new("C")),
        pair("TZ", OsStr::new("UTC")),
        pair("PATH", OsStr::new(HERMETIC_PATH)),
    ];
    env.extend(
        inherited
            .iter()
            .filter(|(key, _)| key.as_bytes().starts_with(b"LLVM_"))
            .cloned(),
    );
    env
}

/// The only way the harness builds a child process: an empty environment plus
/// `env`, the given working directory, and stdin from `/dev/null`. Because
/// `PATH` is set on the child, a bare program name resolves through the
/// hermetic PATH, not the test process's.
pub fn command(program: impl AsRef<OsStr>, cwd: &Path, env: &Env) -> Command {
    let mut command = Command::new(program);
    command
        .env_clear()
        .envs(env.iter().map(|(key, value)| (key, value)))
        .current_dir(cwd)
        .stdin(Stdio::null());
    command
}

/// A live scenario TempDir sits directly under `CARGO_TARGET_TMPDIR`, carries
/// the `harness-<module>-<name>-` prefix that makes a leftover attributable,
/// and is gone once dropped.
#[test]
fn scenario_dirs_carry_the_harness_prefix() {
    let dirs = ScenarioDirs::new("sync", "self_test").expect("scenario dirs");
    let root = dirs.root.clone();
    let tmp_root = Path::new(TMP_ROOT)
        .canonicalize()
        .expect("canonical CARGO_TARGET_TMPDIR");
    assert_eq!(
        root.parent(),
        Some(tmp_root.as_path()),
        "parent of {}",
        root.display()
    );
    let name = root.file_name().expect("the root has a file name");
    assert!(
        name.as_bytes().starts_with(b"harness-sync-self_test-"),
        "scratch root name {name:?} lacks the harness prefix"
    );
    assert!(
        root.is_dir(),
        "{} exists while the dirs live",
        root.display()
    );
    drop(dirs);
    let gone = fs::symlink_metadata(&root);
    assert!(
        matches!(&gone, Err(err) if err.kind() == std::io::ErrorKind::NotFound),
        "{} must be removed on drop, got {gone:?}",
        root.display()
    );
}

/// A child sees exactly the harness keys, in order, then the inherited
/// `LLVM_*` pairs. A planted host environment (git repository variables, the
/// host HOME and PATH, an SSH agent, a global config path) must not reach it.
#[test]
fn hermetic_env_passes_only_the_harness_keys_and_llvm() {
    let dirs = ScenarioDirs::new("env", "key_table").expect("scenario dirs");
    let planted: Env = [
        ("GIT_DIR", "/host/.git"),
        ("GIT_CONFIG_GLOBAL", "/host/.gitconfig"),
        ("HOME", "/host/home"),
        ("PATH", "/host/bin"),
        ("SSH_AUTH_SOCK", "/host/agent"),
        ("LLVM_PROFILE_FILE", "/cov/%p.profraw"),
        ("XLLVM_NOT_A_PREFIX", "dropped"),
    ]
    .into_iter()
    .map(|(key, value)| (OsString::from(key), OsString::from(value)))
    .collect();
    let env = hermetic_env(&planted, &dirs);
    let keys: Vec<&str> = env
        .iter()
        .map(|(key, _)| key.to_str().expect("UTF-8 key"))
        .collect();
    assert_eq!(
        keys,
        [
            "HOME",
            "GIT_CONFIG_GLOBAL",
            "GIT_CONFIG_SYSTEM",
            "GIT_CONFIG_NOSYSTEM",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "XDG_STATE_HOME",
            "GIT_CEILING_DIRECTORIES",
            "GIT_ALLOW_PROTOCOL",
            "GIT_TERMINAL_PROMPT",
            "GIT_EDITOR",
            "GIT_SEQUENCE_EDITOR",
            "GIT_MERGE_AUTOEDIT",
            "LC_ALL",
            "TZ",
            "PATH",
            "LLVM_PROFILE_FILE",
        ]
    );
    let value = |key: &str| {
        env.iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.clone())
            .expect("key present")
    };
    assert_eq!(value("HOME"), dirs.home.as_os_str());
    assert_eq!(
        value("GIT_CONFIG_GLOBAL"),
        dirs.home.join(".gitconfig").as_os_str()
    );
    assert_eq!(value("GIT_CEILING_DIRECTORIES"), dirs.root.as_os_str());
    assert_eq!(value("PATH"), HERMETIC_PATH);
    assert_eq!(value("LLVM_PROFILE_FILE"), "/cov/%p.profraw");
}
