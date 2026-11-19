//! IF-02 and SC5 at run time (operator option 1, 2026-11-19): what `wsx check` prints
//! and returns must not depend on the GSD, devflow and agent files SC5 names. One devflow fixture is
//! put in three states that differ only in those files, and `check` and `check --json` must give
//! the same exit code, stdout and stderr in all three.
//!
//! - States A and C: the six tracked files are valid files with real GSD, devflow, Claude and
//!   Codex keys at non-default values that differ between A and C, and the git-ignored
//!   `.claude/settings.local.json` and the two HOME files hold each state's content.
//! - State B: the six tracked files hold one malformed text, and the ignored and HOME files are
//!   absent.
//!
//! So a reader of one of those settings sees A's value against C's, and a lenient reader that
//! falls back to its defaults on a failed parse sees A's value against its fallback in B, or,
//! when its fallback is A's value, C's value against A's.
//!
//! Why a difference can only come from wsx reading those files:
//! - check reads committed objects and files under `.git`, never the worktree; the worktree is
//!   varied anyway, because that is where a regression would read.
//! - The six tracked files are committed and checked out in every state and `git status
//!   --porcelain` prints nothing in each, so a git command that stats the worktree sees a clean
//!   tree each time.
//! - The ignored file is listed in `info/exclude`, and HOME lies outside the repository.
//! - Nothing fetches, so both staleness times are reasons, not ages, and no clock text appears.
//! - The only text masked is each run's own workspace tip, which each JSON run must hold once.
//! - Content and presence sentinels, not `chmod 000`: the container gate runs as root, where a
//!   file mode does not stop a read.
//!
//! Reach: a read whose result reaches no output is not seen here, which is what
//! `tests/if02_scan.rs` is for. Six reader variants on the test side show the comparison
//! detects a read through each channel: four raw reads (a tracked worktree file, the ignored
//! file, a committed blob and a HOME file) and two lenient readers of `.planning/config.json`
//! through a variable-built path, one falling back to "none" and one to state A's value.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::env::{ScenarioDirs, command, hermetic_env, inherited_env};
use crate::fixture::{Fixture, ManifestKind, RepoConfig, WORK};

/// The `wsx` binary cargo built for this test run.
const WSX: &str = env!("CARGO_BIN_EXE_wsx");

/// The SC5 files as tracked worktree paths, committed in every state.
const TRACKED: [&str; 6] = [
    ".planning/ROADMAP.md",
    ".planning/STATE.md",
    ".planning/config.json",
    "devflow.toml",
    ".claude/settings.json",
    ".codex/config.toml",
];

/// The git-ignored agent file, present in states A and C.
const IGNORED: &str = ".claude/settings.local.json";

/// The agent files under HOME, present in states A and C.
const HOME_FILES: [&str; 2] = [".claude/settings.json", ".codex/config.toml"];

/// What each run's own workspace tip is replaced with before comparing.
const TIP_PLACEHOLDER: &str = "<workspace tip>";

/// The preset's workspace branch.
const WORKSPACE_REF: &str = "refs/heads/workspace/dev";

/// The malformed text every tracked file holds in state B.
const MALFORMED: &[u8] = b"if02 sentinel-B: not JSON, TOML or a roadmap\n";

/// The three agent-file states of the fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    A,
    B,
    C,
}

/// `a` in state A and `c` in state C; state B holds no valid value.
fn pick<'value>(state: State, a: &'value str, c: &'value str) -> &'value str {
    match state {
        State::A => a,
        State::C => c,
        State::B => panic!("state B holds no valid value"),
    }
}

/// The content of `path` in `state`: in A and C a valid file of the path's kind with real
/// keys at values that differ between the two, carrying "if02 sentinel-A" or "if02
/// sentinel-C"; in B one malformed text for every path.
fn sentinel(path: &str, state: State) -> Vec<u8> {
    if state == State::B {
        return MALFORMED.to_vec();
    }
    match path {
        ".planning/config.json" => gsd_config(state),
        ".planning/STATE.md" => gsd_state(state),
        ".planning/ROADMAP.md" => gsd_roadmap(state),
        "devflow.toml" => devflow_config(state),
        ".claude/settings.json" | ".claude/settings.local.json" => claude_settings(state),
        ".codex/config.toml" => codex_config(state),
        other => panic!("no sentinel for {other}"),
    }
}

/// `value` as pretty JSON with a trailing newline.
fn pretty_json(value: &serde_json::Value) -> Vec<u8> {
    let mut text = serde_json::to_vec_pretty(value).expect("serialize a sentinel");
    text.push(b'\n');
    text
}

/// GSD's `.planning/config.json`: the git branching settings.
fn gsd_config(state: State) -> Vec<u8> {
    pretty_json(&serde_json::json!({
        "git": {
            "branching_strategy": pick(state, "phase", "milestone"),
            "phase_branch_template":
                pick(state, "if02-sentinel-A/phase-{phase}", "if02-sentinel-C/phase-{phase}"),
            "milestone_branch_template":
                pick(state, "if02-sentinel-A/{milestone}", "if02-sentinel-C/{milestone}"),
        }
    }))
}

/// Claude Code's settings: model, one permission rule and one environment variable.
fn claude_settings(state: State) -> Vec<u8> {
    pretty_json(&serde_json::json!({
        "model": pick(state, "opus", "haiku"),
        "permissions": {
            "allow": [pick(state, "Bash(echo if02 sentinel-A)", "Bash(echo if02 sentinel-C)")]
        },
        "env": { "IF02_STATE": pick(state, "A", "C") },
    }))
}

/// devflow's `devflow.toml`: the base branch.
fn devflow_config(state: State) -> Vec<u8> {
    let base = pick(state, "workspace/if02-a", "workspace/if02-c");
    format!("# if02 sentinel-{state:?}\nbase_branch = \"{base}\"\n").into_bytes()
}

/// Codex's `config.toml`: the reasoning effort.
fn codex_config(state: State) -> Vec<u8> {
    let effort = pick(state, "high", "low");
    format!("# if02 sentinel-{state:?}\nmodel_reasoning_effort = \"{effort}\"\n").into_bytes()
}

/// GSD's `.planning/STATE.md`: the frontmatter keys, then a body.
fn gsd_state(state: State) -> Vec<u8> {
    let milestone = pick(state, "v0.1.0", "v0.2.0");
    let phase = pick(state, "02", "03");
    let status = pick(state, "executing", "verifying");
    format!(
        "---\ngsd_state_version: \"1.0\"\nmilestone: {milestone}\ncurrent_phase: {phase}\n\
         status: {status}\n---\n\n# State\n\nif02 sentinel-{state:?}\n"
    )
    .into_bytes()
}

/// GSD's `.planning/ROADMAP.md`: the preset's phase heading; state A has a second phase.
fn gsd_roadmap(state: State) -> Vec<u8> {
    let second = pick(state, "\n### Phase 2: Second\n", "");
    format!("# Roadmap\n\n### Phase 1: Example\n\n**Goal:** if02 sentinel-{state:?}\n{second}")
        .into_bytes()
}

/// Writes `path`'s sentinel to `full` in states A and C, and removes `full` when present in
/// state B.
fn set_presence(full: &Path, path: &str, state: State) {
    if state == State::B {
        match fs::remove_file(full) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => panic!("remove {}: {err}", full.display()),
        }
        return;
    }
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|err| panic!("create {}: {err}", parent.display()));
    }
    fs::write(full, sentinel(path, state))
        .unwrap_or_else(|err| panic!("write {}: {err}", full.display()));
}

/// `full`'s bytes, or "absent" when it does not exist.
fn file_or_absent(full: &Path) -> Vec<u8> {
    match fs::read(full) {
        Ok(content) => content,
        Err(err) if err.kind() == ErrorKind::NotFound => b"absent".to_vec(),
        Err(err) => panic!("read {}: {err}", full.display()),
    }
}

/// A read of one SC5 file a reader variant appends to its observation.
enum Read {
    /// A file in the work checkout.
    Worktree(&'static str),
    /// A blob of the current workspace tip.
    Object(&'static str),
    /// A file under HOME.
    Home(&'static str),
    /// The work checkout's `dir`/`file`, through a path built from those fields, parsed as
    /// JSON for `git.branching_strategy`, with `default` on any failure.
    Lenient {
        dir: &'static str,
        file: &'static str,
        default: &'static str,
    },
}

impl Read {
    fn label(&self) -> String {
        match self {
            Read::Worktree(path) => format!("worktree {path}"),
            Read::Object(path) => format!("object {path}"),
            Read::Home(path) => format!("home {path}"),
            Read::Lenient { dir, file, default } => {
                format!("lenient {dir}/{file} default {default}")
            }
        }
    }

    /// The file's content as `runtime` holds it, with `tip` the current workspace tip.
    fn content(&self, runtime: &Runtime, tip: &str) -> Vec<u8> {
        match self {
            Read::Worktree(path) => file_or_absent(&runtime.fixture.checkout(WORK).join(path)),
            Read::Object(path) => runtime.git(&["cat-file", "blob", &format!("{tip}:{path}")]),
            Read::Home(path) => file_or_absent(&runtime.home.join(path)),
            Read::Lenient { dir, file, default } => {
                let path = runtime.fixture.checkout(WORK).join(dir).join(file);
                lenient_branching_strategy(&path, default).into_bytes()
            }
        }
    }
}

/// "git.branching_strategy={value}" for the JSON file at `path`, with `default` when the read,
/// the parse or the key fails: the unwrap_or_default shape of a lenient settings reader.
fn lenient_branching_strategy(path: &Path, default: &str) -> String {
    let value = fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|config| {
            config
                .pointer("/git/branching_strategy")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| default.to_owned());
    format!("git.branching_strategy={value}")
}

/// One run of `wsx`, with each run's own workspace tip masked.
struct Observation {
    command: String,
    state: State,
    exit: Option<i32>,
    stdout: String,
    stderr: String,
    /// How many times the tip appeared in stdout and stderr.
    masked: usize,
}

/// A fixture and the scenario tree holding it, removed when this is dropped
/// (fields drop in order, so the fixture goes first).
struct Runtime {
    fixture: Fixture,
    preset_tip: String,
    home: PathBuf,
    _dirs: ScenarioDirs,
}

/// The devflow preset declaring every path the states add, in its own
/// `harness-if02_runtime-<name>-` tree, with the ignored file excluded.
fn runtime(name: &str) -> Runtime {
    let dirs = ScenarioDirs::new("if02_runtime", name).unwrap_or_else(|err| panic!("{err}"));
    let env = hermetic_env(&inherited_env(), &dirs);
    let config = RepoConfig::devflow()
        .with_manifest_entry(ManifestKind::Only, b"devflow.toml")
        .with_manifest_entry(ManifestKind::Only, b".claude")
        .with_manifest_entry(ManifestKind::Only, b".codex");
    let fixture = Fixture::new(&dirs, env, config).unwrap_or_else(|err| panic!("{err}"));
    let mut runtime = Runtime {
        fixture,
        preset_tip: String::new(),
        home: dirs.home.clone(),
        _dirs: dirs,
    };
    runtime.exclude_ignored();
    runtime.preset_tip = runtime.tip();
    runtime
}

impl Runtime {
    /// Runs git in `work` and returns its stdout; a failing git fails the test.
    fn git(&self, args: &[&str]) -> Vec<u8> {
        self.fixture
            .git_ok(WORK, args)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    /// The current workspace tip, as hex.
    fn tip(&self) -> String {
        let out = self.git(&["rev-parse", "--verify", WORKSPACE_REF]);
        String::from_utf8(out)
            .expect("rev-parse prints hex")
            .trim_end()
            .to_owned()
    }

    /// Lists the ignored file in `work`'s `info/exclude`, and checks git ignores it.
    fn exclude_ignored(&self) {
        let out = self.git(&["rev-parse", "--git-path", "info/exclude"]);
        let relative = String::from_utf8(out).expect("rev-parse prints a path");
        let exclude = self.fixture.checkout(WORK).join(relative.trim_end());
        if let Some(parent) = exclude.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|err| panic!("create {}: {err}", parent.display()));
        }
        let mut content = match fs::read(&exclude) {
            Ok(content) => content,
            Err(err) if err.kind() == ErrorKind::NotFound => Vec::new(),
            Err(err) => panic!("read {}: {err}", exclude.display()),
        };
        if !content.is_empty() && !content.ends_with(b"\n") {
            content.push(b'\n');
        }
        content.extend_from_slice(format!("/{IGNORED}\n").as_bytes());
        fs::write(&exclude, content)
            .unwrap_or_else(|err| panic!("write {}: {err}", exclude.display()));
        self.git(&["check-ignore", "-q", IGNORED]);
    }

    /// Puts the fixture in `state`: the tracked files committed on the preset tip and
    /// checked out, the ignored and HOME files present or absent, and a clean status.
    fn enter(&mut self, state: State) {
        let preset_tip = self.preset_tip.clone();
        self.git(&["reset", "--quiet", "--hard", &preset_tip]);
        for path in TRACKED {
            self.fixture
                .write(WORK, path.as_bytes(), &sentinel(path, state))
                .unwrap_or_else(|err| panic!("{err}"));
        }
        self.fixture
            .commit(WORK, b"chore(ws): add agent files\n")
            .unwrap_or_else(|err| panic!("{err}"));
        set_presence(&self.fixture.checkout(WORK).join(IGNORED), IGNORED, state);
        for path in HOME_FILES {
            set_presence(&self.home.join(path), path, state);
        }
        let status = self.git(&["status", "--porcelain"]);
        assert!(
            status.is_empty(),
            "git status in state {state:?}: {}",
            status.escape_ascii()
        );
    }

    /// Runs the built `wsx` with `args` in `work`, which is in `state`; with `read`,
    /// appends that file's content, escaped, to stdout.
    fn observe(&self, args: &[&str], read: Option<&Read>, state: State) -> Observation {
        let tip = self.tip();
        let output = command(WSX, &self.fixture.checkout(WORK), &self.fixture.env)
            .args(args)
            .output()
            .expect("spawn the wsx binary");
        let command = format!("wsx {}", args.join(" "));
        let utf8 = |bytes: Vec<u8>, stream: &str| {
            String::from_utf8(bytes)
                .unwrap_or_else(|err| panic!("{command}: {stream} is not UTF-8: {err}"))
        };
        let stdout = utf8(output.stdout, "stdout");
        let stderr = utf8(output.stderr, "stderr");
        let masked = stdout.matches(&tip).count() + stderr.matches(&tip).count();
        let mut stdout = stdout.replace(&tip, TIP_PLACEHOLDER);
        if let Some(read) = read {
            let content = read.content(self, &tip);
            stdout.push_str(&format!(
                "\nread {}: {}",
                read.label(),
                content.escape_ascii()
            ));
        }
        Observation {
            exit: output.status.code(),
            stdout,
            stderr: stderr.replace(&tip, TIP_PLACEHOLDER),
            masked,
            command,
            state,
        }
    }
}

/// One difference: the stream named, then its text in `a`'s and `b`'s state, escaped.
fn difference(
    a: &Observation,
    b: &Observation,
    stream: &str,
    a_text: &str,
    b_text: &str,
) -> String {
    format!(
        "{}: {stream} differs between state {:?} and state {:?}\n  {:?}: {}\n  {:?}: {}",
        a.command,
        a.state,
        b.state,
        a.state,
        a_text.as_bytes().escape_ascii(),
        b.state,
        b_text.as_bytes().escape_ascii()
    )
}

/// Every way `a` and `b`, one command in two states, differ.
fn differences(a: &Observation, b: &Observation) -> Vec<String> {
    let mut found = Vec::new();
    if a.exit != b.exit {
        let (exit_a, exit_b) = (format!("{:?}", a.exit), format!("{:?}", b.exit));
        found.push(difference(a, b, "exit code", &exit_a, &exit_b));
    }
    if a.stdout != b.stdout {
        found.push(difference(a, b, "stdout", &a.stdout, &b.stdout));
    }
    if a.stderr != b.stderr {
        found.push(difference(a, b, "stderr", &a.stderr, &b.stderr));
    }
    found
}

/// `check` and `check --json` observed in states A, B and C in that order: one list per
/// command, in state order.
fn run_states(name: &str, read: Option<Read>) -> Vec<Vec<Observation>> {
    let commands: [&[&str]; 2] = [&["check"], &["check", "--json"]];
    let mut runtime = runtime(name);
    let mut by_command: Vec<Vec<Observation>> = commands.iter().map(|_| Vec::new()).collect();
    for state in [State::A, State::B, State::C] {
        runtime.enter(state);
        for (args, observed) in commands.iter().zip(&mut by_command) {
            observed.push(runtime.observe(args, read.as_ref(), state));
        }
    }
    by_command
}

/// Each command's state-A observation against each later one, one difference per line.
fn all_differences(by_command: &[Vec<Observation>]) -> String {
    by_command
        .iter()
        .flat_map(|observed| {
            let (in_a, later) = observed
                .split_first()
                .expect("each command is observed in state A");
            later.iter().flat_map(move |other| differences(in_a, other))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// With `read` added, the states differ and the difference carries every `expected`
/// text.
fn assert_read_differs(name: &str, read: Read, what: &str, expected: &[&str]) {
    let found = all_differences(&run_states(name, Some(read)));
    assert!(
        !found.is_empty(),
        "a read of {what} made no difference between the states"
    );
    for text in expected {
        assert!(found.contains(*text), "{text} is missing from: {found}");
    }
}

/// check and check --json give the same exit code, stdout and stderr in all three states,
/// and state A passes.
#[test]
fn check_output_does_not_depend_on_agent_files() {
    let by_command = run_states("check_output_does_not_depend_on_agent_files", None);
    let found = all_differences(&by_command);
    assert!(found.is_empty(), "{found}");
    let human = &by_command[0][0];
    assert_eq!(
        human.exit,
        Some(0),
        "{}: stderr: {}",
        human.command,
        human.stderr
    );
    assert!(
        human.stdout.contains("every difference between"),
        "{}: stdout: {}",
        human.command,
        human.stdout
    );
    assert_eq!(by_command[1].len(), 3, "check --json runs, one per state");
    for json in &by_command[1] {
        assert_eq!(
            json.masked, 1,
            "{} in state {:?}: workspace tips masked",
            json.command, json.state
        );
    }
}

/// A read of a tracked worktree file makes the states differ.
#[test]
fn tracked_file_read_makes_the_states_differ() {
    assert_read_differs(
        "tracked_file_read_makes_the_states_differ",
        Read::Worktree(".planning/config.json"),
        "the worktree's .planning/config.json",
        &["sentinel-A", "sentinel-B"],
    );
}

/// A read of the git-ignored agent file makes the states differ.
#[test]
fn ignored_file_read_makes_the_states_differ() {
    assert_read_differs(
        "ignored_file_read_makes_the_states_differ",
        Read::Worktree(IGNORED),
        "the worktree's .claude/settings.local.json",
        &["sentinel-A", "absent"],
    );
}

/// A read of a committed blob at the workspace tip makes the states differ.
#[test]
fn object_read_makes_the_states_differ() {
    assert_read_differs(
        "object_read_makes_the_states_differ",
        Read::Object(".planning/STATE.md"),
        "the workspace tip's .planning/STATE.md",
        &["sentinel-A", "sentinel-B"],
    );
}

/// A read of a HOME agent file makes the states differ.
#[test]
fn home_read_makes_the_states_differ() {
    assert_read_differs(
        "home_read_makes_the_states_differ",
        Read::Home(".codex/config.toml"),
        "HOME's .codex/config.toml",
        &["sentinel-A", "absent"],
    );
}

/// A lenient read of .planning/config.json through a variable-built path, falling back to
/// "none", makes the states differ.
#[test]
fn lenient_config_read_makes_the_states_differ() {
    assert_read_differs(
        "lenient_config_read_makes_the_states_differ",
        Read::Lenient {
            dir: ".planning",
            file: "config.json",
            default: "none",
        },
        "the worktree's .planning/config.json, parsed leniently through a variable-built path",
        &[
            "git.branching_strategy=phase",
            "git.branching_strategy=none",
            "git.branching_strategy=milestone",
        ],
    );
}

/// The same lenient read falling back to state A's value differs only against state C.
#[test]
fn lenient_read_with_state_a_default_differs_in_state_c() {
    let read = Read::Lenient {
        dir: ".planning",
        file: "config.json",
        default: "phase",
    };
    let label = read.label();
    let found = all_differences(&run_states(
        "lenient_read_with_state_a_default_differs_in_state_c",
        Some(read),
    ));
    assert!(
        !found.is_empty(),
        "a read {label} made no difference between the states"
    );
    assert!(found.contains("between state A and state C"), "{found}");
    assert!(
        found.contains("git.branching_strategy=milestone"),
        "{found}"
    );
    assert!(!found.contains("between state A and state B"), "{found}");
}
