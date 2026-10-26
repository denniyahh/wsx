//! Hermeticity of the scenario environment (plan 02-09).
//!
//! Proves that git configuration, hooks and identity planted in a host
//! environment never reach a scenario: its fixture and its action see only
//! the harness's own configuration.
//!
//! Isolation: the planted host is itself a scratch tree inside the test's own
//! `harness-hermeticity-` tree under `CARGO_TARGET_TMPDIR`; the test process's
//! own environment is never changed.

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::env::{Env, ScenarioDirs, command, hermetic_env, inherited_env};
use crate::fixture::{Fixture, RepoConfig, WORK};
use crate::outcome::compare;
use crate::scenario::{self, Entry, Side};

/// Every key `hermetic_env` sets, in order, whatever it inherits.
const HARNESS_KEYS: [&str; 17] = [
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
];

/// The PATH every child sees.
const HERMETIC_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// The oldest git the oracle scripts and config-defined hooks run on.
const MIN_GIT: (u32, u32) = (2, 54);

/// The hooks planted under the host's `core.hooksPath`: each appends
/// `hookspath:<name>` to the marker when git runs it.
const PLANTED_HOOKS: [&str; 7] = [
    "reference-transaction",
    "pre-commit",
    "commit-msg",
    "post-commit",
    "post-checkout",
    "pre-merge-commit",
    "pre-push",
];

/// The scenarios the planted-host proof runs: the sync tracer, the PR cut and
/// the pre-push guard (whose two controls run too).
const PROVEN_SCENARIOS: [(&str, &str); 3] = [
    ("sync", "self_test"),
    ("cut", "self_test"),
    ("guard", "self_test"),
];

/// A host environment planted inside its own scratch tree: config files at
/// every documented git config source, hooks that record that they ran, a
/// decoy repository, and the inherited environment pointing at all of it.
struct PlantedHost {
    /// The planted global config, also reachable as `$HOME/.gitconfig`.
    global_config: PathBuf,
    /// Created only if a planted hook runs.
    marker: PathBuf,
    /// The decoy repository's work tree.
    decoy: PathBuf,
    /// The decoy's refs and config as planted.
    decoy_before: DecoyState,
    /// The inherited environment of the planted host.
    env: Env,
    /// The plant's own scratch tree; dropping it removes everything planted.
    dirs: ScenarioDirs,
}

/// What a retargeted git command could change in the decoy: its refs
/// (`git show-ref --head`) and its `.git/config` bytes.
#[derive(Debug, PartialEq)]
struct DecoyState {
    refs: Vec<u8>,
    config: Vec<u8>,
}

/// The planted git config: hooks through `core.hooksPath` and a config hook
/// (`hook.planted.command`), a template directory (`init.templateDir`) with
/// its own hook, a host identity, `core.quotePath`
/// off (which would change how a scenario's path names come out), and signing
/// on with a key that does not exist, so a commit made under it fails.
fn planted_config(root: &Path, marker: &Path) -> String {
    let root = root.display();
    let marker = marker.display();
    format!(
        "[core]
\thooksPath = {root}/hooks
\tquotePath = false
[hook \"planted\"]
\tcommand = echo config-hook >> '{marker}'
\tevent = pre-commit
\tevent = pre-push
[init]
\ttemplateDir = {root}/tmpl
[user]
\tname = Host Person
\temail = host@example.invalid
\tsigningkey = /nonexistent
[commit]
\tgpgsign = true
"
    )
}

fn write(path: &Path, content: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|err| panic!("create {}: {err}", parent.display()));
    }
    fs::write(path, content).unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
}

/// Writes a hook script that appends `line` to `marker`.
fn write_hook(path: &Path, line: &str, marker: &Path) {
    let script = format!("#!/bin/sh\necho '{line}' >> '{}'\n", marker.display());
    write(path, script.as_bytes());
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .unwrap_or_else(|err| panic!("chmod {}: {err}", path.display()));
}

/// Runs git in `cwd` with the plant's own harness environment (not the
/// planted one) and returns its stdout.
fn plant_git(dirs: &ScenarioDirs, cwd: &Path, args: &[&str]) -> Vec<u8> {
    let output = command("git", cwd, &hermetic_env(&[], dirs))
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("spawn git {args:?}: {err}"));
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        cwd.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn decoy_state(dirs: &ScenarioDirs, decoy: &Path) -> DecoyState {
    let config = decoy.join(".git/config");
    DecoyState {
        refs: plant_git(dirs, decoy, &["show-ref", "--head"]),
        config: fs::read(&config).unwrap_or_else(|err| panic!("read {}: {err}", config.display())),
    }
}

/// Plants a host in a `harness-hermeticity-<name>-` tree: the planted config
/// at `host-home/.gitconfig`, at `host-xdg/git/config` (beside a `git/ignore`
/// holding `src/`) and at `host-system.gitconfig`; the hooks under `hooks/`
/// and the template's `post-commit` under `tmpl/hooks/`; and a decoy
/// repository with one commit under `decoy/`. Its inherited environment points
/// HOME, `GIT_CONFIG_GLOBAL`, `XDG_CONFIG_HOME` and `GIT_CONFIG_SYSTEM` at
/// those files, `GIT_CONFIG_PARAMETERS` at the hooks, `GIT_DIR` and
/// `GIT_WORK_TREE` at the decoy, and `SSH_AUTH_SOCK` and `LLVM_PROFILE_FILE`
/// at paths in the tree.
fn plant_host(name: &str) -> PlantedHost {
    let dirs = ScenarioDirs::new("hermeticity", name).expect("the plant's scratch tree");
    let root = dirs.root.clone();
    let marker = root.join("marker");
    let config = planted_config(&root, &marker);

    let home = root.join("host-home");
    let global_config = home.join(".gitconfig");
    let xdg = root.join("host-xdg");
    let system_config = root.join("host-system.gitconfig");
    for path in [&global_config, &xdg.join("git/config"), &system_config] {
        write(path, config.as_bytes());
    }
    write(&xdg.join("git/ignore"), b"src/\n");
    for hook in PLANTED_HOOKS {
        write_hook(
            &root.join("hooks").join(hook),
            &format!("hookspath:{hook}"),
            &marker,
        );
    }
    write_hook(
        &root.join("tmpl/hooks/post-commit"),
        "template:post-commit",
        &marker,
    );

    let decoy = root.join("decoy");
    plant_git(&dirs, &root, &["init", "--quiet", "-b", "main", "decoy"]);
    write(&decoy.join("decoy.txt"), b"decoy\n");
    plant_git(&dirs, &decoy, &["add", "decoy.txt"]);
    plant_git(&dirs, &decoy, &["commit", "--quiet", "-m", "decoy"]);
    let decoy_before = decoy_state(&dirs, &decoy);

    let path = |path: &Path| path.as_os_str().to_os_string();
    let env: Env = vec![
        ("HOME".into(), path(&home)),
        ("GIT_CONFIG_GLOBAL".into(), path(&global_config)),
        ("XDG_CONFIG_HOME".into(), path(&xdg)),
        ("GIT_CONFIG_SYSTEM".into(), path(&system_config)),
        (
            "GIT_CONFIG_PARAMETERS".into(),
            format!("'core.hookspath'='{}/hooks'", root.display()).into(),
        ),
        ("GIT_DIR".into(), path(&decoy.join(".git"))),
        ("GIT_WORK_TREE".into(), path(&decoy)),
        ("SSH_AUTH_SOCK".into(), path(&root.join("agent.sock"))),
        (
            "LLVM_PROFILE_FILE".into(),
            path(&root.join("llvm-%p.profraw")),
        ),
    ];
    PlantedHost {
        global_config,
        marker,
        decoy,
        decoy_before,
        env,
        dirs,
    }
}

/// The registered scenario `module::name`.
fn registered(module: &str, name: &str) -> Entry {
    crate::all_scenarios()
        .into_iter()
        .find(|entry| entry.module == module && entry.name == name)
        .unwrap_or_else(|| panic!("{module}::{name} is registered"))
}

/// The marker's contents, or `None` when no planted hook ran.
fn marker_contents(planted: &PlantedHost) -> Option<String> {
    match fs::read_to_string(&planted.marker) {
        Ok(contents) => Some(contents),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => panic!("read {}: {err}", planted.marker.display()),
    }
}

/// Under a planted host environment, three real scenarios reproduce their
/// explicit expected outcomes on the oracle side, no planted hook runs, and
/// the decoy repository `GIT_DIR` points at is left as it was. The built env
/// keeps only `LLVM_PROFILE_FILE` from the plant.
#[test]
fn planted_host_config_never_reaches_scenarios() {
    let planted = plant_host("planted_proof");

    let probe = ScenarioDirs::new("hermeticity", "planted_env").expect("scratch dirs");
    let built = hermetic_env(&planted.env, &probe);
    for (key, value) in &planted.env {
        let carried = built.iter().any(|(_, built)| built == value);
        if key == "LLVM_PROFILE_FILE" {
            assert!(
                built.contains(&(key.clone(), value.clone())),
                "LLVM_PROFILE_FILE must pass through: {built:?}"
            );
        } else {
            assert!(!carried, "the planted {key:?} reached the env: {built:?}");
        }
    }
    assert!(
        !built.iter().any(|(key, _)| key == "SSH_AUTH_SOCK"),
        "SSH_AUTH_SOCK reached the env: {built:?}"
    );

    let make_env = |dirs: &ScenarioDirs| hermetic_env(&planted.env, dirs);
    for (module, name) in PROVEN_SCENARIOS {
        scenario::run_with_env(Side::Oracle, &registered(module, name), &make_env);
        println!("{module}::{name} reproduced its expected outcome under the plant");
    }

    assert_eq!(
        marker_contents(&planted),
        None,
        "a planted hook ran: {}",
        planted.marker.display()
    );
    assert_eq!(
        decoy_state(&planted.dirs, &planted.decoy),
        planted.decoy_before,
        "the decoy repository changed"
    );
}

/// Control for the planted-host proof: the same plant, with only
/// `GIT_CONFIG_GLOBAL` wired to the planted file, fires the planted hooks and
/// does not reproduce `sync::self_test`'s expected outcome. It rules out a
/// proof that passes only because the plant was inert (a hook that cannot
/// run, a config git does not read).
#[test]
fn wired_to_planted_config_fires_hook() {
    let planted = plant_host("wired_control");
    let entry = registered("sync", "self_test");
    let scenario = (entry.build)();
    let case = &scenario.subject;
    let wired = |dirs: &ScenarioDirs| {
        let mut env = hermetic_env(&planted.env, dirs);
        let mut replaced = 0;
        for (key, value) in &mut env {
            if key == "GIT_CONFIG_GLOBAL" {
                *value = OsString::from(planted.global_config.as_os_str());
                replaced += 1;
            }
        }
        assert_eq!(replaced, 1, "the env holds one GIT_CONFIG_GLOBAL: {env:?}");
        env
    };

    let result = scenario::run_case(Side::Oracle, &entry, case, &wired);

    let marker = marker_contents(&planted).expect("the wired run fired no planted hook");
    println!("marker after the wired run:\n{marker}");
    assert!(
        marker.lines().any(|line| line == "hookspath:pre-commit"),
        "the planted pre-commit hook did not run: {marker:?}"
    );
    match result {
        Err(err) => println!("the wired run failed, as the planted signing key is missing: {err}"),
        Ok(run) => {
            let expected = case
                .expected
                .as_ref()
                .expect("sync::self_test states an outcome");
            let mismatches = compare(expected, &run.before, &run.after, run.run.exit);
            assert!(
                !mismatches.is_empty(),
                "the wired run reproduced the expected outcome, so the plant changed nothing"
            );
        }
    }
}

/// The keys of `env`, in order.
fn keys(env: &Env) -> Vec<&str> {
    env.iter()
        .map(|(key, _)| key.to_str().expect("UTF-8 key"))
        .collect()
}

/// The value `env` holds for `key`; panics unless it holds exactly one.
fn value<'a>(env: &'a Env, key: &str) -> &'a OsString {
    let values: Vec<&OsString> = env
        .iter()
        .filter(|(name, _)| name == key)
        .map(|(_, value)| value)
        .collect();
    assert_eq!(values.len(), 1, "{key} appears once in {env:?}");
    values[0]
}

/// Only keys starting with `LLVM_` pass through: `LLVM`, `LLVMX_A` and
/// `XLLVM_B` sit next to the prefix and are dropped. A planted HOME, PATH or
/// `GIT_CONFIG_GLOBAL`, before or after the `LLVM_` keys, never overrides the
/// harness value, and no key appears twice.
#[test]
fn env_passes_through_only_llvm_keys() {
    let dirs = ScenarioDirs::new("hermeticity", "llvm_keys").expect("scratch dirs");
    let inherited: Env = [
        ("HOME", "/host/home"),
        ("LLVM_PROFILE_FILE", "/cov/%p.profraw"),
        ("LLVM", "no-underscore"),
        ("PATH", "/host/bin"),
        ("LLVMX_A", "longer-prefix"),
        ("LLVM_COV", "/host/llvm-cov"),
        ("XLLVM_B", "prefixed"),
        ("GIT_CONFIG_GLOBAL", "/host/.gitconfig"),
        ("GIT_DIR", "/host/.git"),
        ("SSH_AUTH_SOCK", "/host/agent"),
    ]
    .into_iter()
    .map(|(key, value)| (OsString::from(key), OsString::from(value)))
    .collect();
    let built = hermetic_env(&inherited, &dirs);

    let mut got = keys(&built);
    got.sort_unstable();
    let mut expected: Vec<&str> = HARNESS_KEYS.to_vec();
    expected.extend(["LLVM_PROFILE_FILE", "LLVM_COV"]);
    expected.sort_unstable();
    assert_eq!(got, expected, "built env: {built:?}");

    assert_eq!(value(&built, "HOME"), dirs.home.as_os_str());
    assert_eq!(value(&built, "PATH"), HERMETIC_PATH);
    assert_eq!(
        value(&built, "GIT_CONFIG_GLOBAL"),
        dirs.home.join(".gitconfig").as_os_str()
    );
    assert_eq!(value(&built, "LLVM_PROFILE_FILE"), "/cov/%p.profraw");
    assert_eq!(value(&built, "LLVM_COV"), "/host/llvm-cov");
}

/// From an empty inherited environment the builder still sets every harness
/// key, in order, and no `LLVM_` key.
#[test]
fn env_from_an_empty_inheritance_is_complete() {
    let dirs = ScenarioDirs::new("hermeticity", "empty_inheritance").expect("scratch dirs");
    let built = hermetic_env(&[], &dirs);
    assert_eq!(keys(&built), HARNESS_KEYS, "built env: {built:?}");
}

/// The oracle pre-push hook's own `unset` statement, read from the vendored
/// file, with its continuation lines joined into one line.
fn pre_push_unset_statement() -> String {
    const START: &str = "unset $(git rev-parse --local-env-vars)";
    const END: &str = "GIT_CONFIG_GLOBAL GIT_CONFIG_SYSTEM";
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/oracle/hooks/pre-push");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    let start = text
        .find(START)
        .unwrap_or_else(|| panic!("{} no longer contains {START:?}", path.display()));
    let rest = &text[start..];
    let end = rest
        .find(END)
        .unwrap_or_else(|| panic!("{}: {START:?} no longer ends with {END:?}", path.display()));
    let statement = rest[..end + END.len()].replace("\\\n", " ");
    assert!(
        !statement.contains('\n'),
        "the statement spans lines that are not continuations: {statement:?}"
    );
    statement
}

/// What git reads its config from, seen from one shell.
struct ConfigProbe {
    /// `GIT_CONFIG_NOSYSTEM` as the shell sees it, or `unset`.
    nosystem: String,
    /// `git config --list --show-origin`.
    listing: String,
}

/// Runs, in `work` under `env`, a bash script of `prefix` (empty, or the
/// hook's unset statement), then a line printing `GIT_CONFIG_NOSYSTEM`, then
/// `git config --list --show-origin`.
fn probe_config(work: &Path, env: &Env, prefix: &str) -> ConfigProbe {
    let script = format!(
        "set -euo pipefail\n{prefix}\nprintf '%s\\n' \"${{GIT_CONFIG_NOSYSTEM-unset}}\"\n\
         git config --list --show-origin\n"
    );
    let output = command("bash", work, env)
        .args(["-c", &script])
        .output()
        .expect("spawn bash");
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(
        output.status.success(),
        "the probe failed: {stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let (nosystem, listing) = stdout.split_once('\n').expect("a GIT_CONFIG_NOSYSTEM line");
    ConfigProbe {
        nosystem: nosystem.to_owned(),
        listing: listing.to_owned(),
    }
}

/// A devflow fixture, its `work` checkout and the harness env it was built in.
fn devflow_fixture(dirs: &ScenarioDirs) -> (Fixture, Env) {
    let env = hermetic_env(&inherited_env(), dirs);
    let fixture =
        Fixture::new(dirs, env.clone(), RepoConfig::devflow()).expect("the devflow preset");
    (fixture, env)
}

/// Under the harness env the oracle pre-push hook's unset of the config
/// location variables changes nothing: git lists the same config, from the
/// same files, before and after it, every file lies under the scenario root,
/// and `GIT_CONFIG_NOSYSTEM=1` survives, so the host's system config stays
/// unread.
#[test]
fn pre_push_unset_leaves_config_sources_unchanged() {
    let statement = pre_push_unset_statement();
    let dirs = ScenarioDirs::new("hermeticity", "pre_push_unset").expect("scratch dirs");
    let (fixture, env) = devflow_fixture(&dirs);
    let work = fixture.checkout(WORK);

    let without = probe_config(&work, &env, "");
    let with = probe_config(&work, &env, &statement);

    assert_eq!(
        with.listing, without.listing,
        "the unset changed the config"
    );
    let harness_identity = format!(
        "file:{}\tuser.name=Harness",
        dirs.home.join(".gitconfig").display()
    );
    assert!(
        with.listing.lines().any(|line| line == harness_identity),
        "the harness global config is not read: {}",
        with.listing
    );
    for line in with.listing.lines() {
        let (origin, _) = line.split_once('\t').expect("an origin and an entry");
        let file = origin
            .strip_prefix("file:")
            .unwrap_or_else(|| panic!("an origin that is not a file: {line:?}"));
        let file = work.join(file);
        assert!(
            file.starts_with(&dirs.root),
            "{} lies outside the scenario root {}",
            file.display(),
            dirs.root.display()
        );
    }
    assert_eq!(with.nosystem, "1", "GIT_CONFIG_NOSYSTEM after the unset");
}

/// Control for the unset test: with `GIT_CONFIG_GLOBAL` pointed at a file
/// other than `$HOME/.gitconfig`, the same unset switches git back to
/// `$HOME/.gitconfig`, so the planted key disappears. It proves the probe can
/// see a source change, so the identical listings in
/// `pre_push_unset_leaves_config_sources_unchanged` are a real result.
#[test]
fn pre_push_unset_switches_source_when_global_is_elsewhere() {
    let statement = pre_push_unset_statement();
    let dirs = ScenarioDirs::new("hermeticity", "pre_push_switch").expect("scratch dirs");
    let (fixture, mut env) = devflow_fixture(&dirs);
    let work = fixture.checkout(WORK);
    let elsewhere = dirs.root.join("elsewhere.gitconfig");
    write(&elsewhere, b"[wsx]\n\tplanted = elsewhere\n");
    for (key, value) in &mut env {
        if key == "GIT_CONFIG_GLOBAL" {
            *value = OsString::from(elsewhere.as_os_str());
        }
    }
    assert_eq!(value(&env, "GIT_CONFIG_GLOBAL"), elsewhere.as_os_str());

    let without = probe_config(&work, &env, "");
    let with = probe_config(&work, &env, &statement);

    let planted = |listing: &str| {
        listing
            .lines()
            .any(|line| line.ends_with("\twsx.planted=elsewhere"))
    };
    assert!(
        planted(&without.listing),
        "without the unset: {}",
        without.listing
    );
    assert!(!planted(&with.listing), "after the unset: {}", with.listing);
    assert!(
        with.listing.contains("\tuser.name=Harness"),
        "after the unset git reads $HOME/.gitconfig: {}",
        with.listing
    );
}

/// Whether `text`, the output of `git --version`, names version `major.minor`
/// or later. Anything else, including an unparsable version, is `false`.
fn git_version_at_least(text: &str, (major, minor): (u32, u32)) -> bool {
    let Some(version) = text.trim().strip_prefix("git version ") else {
        return false;
    };
    let mut parts = version.split('.').map(str::parse::<u32>);
    match (parts.next(), parts.next()) {
        (Some(Ok(got_major)), Some(Ok(got_minor))) => (got_major, got_minor) >= (major, minor),
        _ => false,
    }
}

/// The git the hermetic PATH resolves is at least 2.54: the oracle scripts
/// and the config-defined hooks the planted-host proof relies on need it.
#[test]
fn hermetic_git_is_at_least_2_54() {
    let dirs = ScenarioDirs::new("hermeticity", "git_version").expect("scratch dirs");
    let env = hermetic_env(&inherited_env(), &dirs);
    let output = command("git", &dirs.root, &env)
        .arg("--version")
        .output()
        .expect("spawn git through the hermetic PATH");
    assert!(output.status.success(), "git --version: {output:?}");
    let version = String::from_utf8_lossy(&output.stdout);
    println!("hermetic git: {}", version.trim());
    assert!(
        git_version_at_least(&version, MIN_GIT),
        "the hermetic git is older than {MIN_GIT:?}: {version:?}"
    );
}

/// Control for the version check: it rejects 2.53 and accepts 2.54, compares
/// numerically (2.100 is newer than 2.54), and rejects text that is not a git
/// version.
#[test]
fn version_check_rejects_an_older_git() {
    assert!(!git_version_at_least("git version 2.53.0\n", MIN_GIT));
    assert!(git_version_at_least("git version 2.54.0\n", MIN_GIT));
    assert!(git_version_at_least("git version 2.100.1\n", MIN_GIT));
    assert!(git_version_at_least("git version 3.0.0\n", MIN_GIT));
    assert!(!git_version_at_least("1.99.0\n", MIN_GIT));
    assert!(!git_version_at_least("git version two.54\n", MIN_GIT));
}
