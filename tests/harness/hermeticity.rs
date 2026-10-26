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

use crate::env::{Env, ScenarioDirs, command, hermetic_env};
use crate::outcome::compare;
use crate::scenario::{self, Entry, Side};

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
