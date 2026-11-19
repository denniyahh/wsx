//! IF-02 and SC5 at run time (operator option 1, 2026-11-19): what `wsx check` prints
//! and returns must not depend on the GSD, devflow and agent files SC5 names. One devflow fixture is
//! put in two states that differ only in those files, and `check` and `check --json` must give
//! the same exit code, stdout and stderr in both.
//!
//! - State A: the six tracked files hold well-formed sentinel content, and the git-ignored
//!   `.claude/settings.local.json` and the two HOME files exist.
//! - State B: the six tracked files hold one malformed text, so a parse that fails on it also
//!   differs, and the ignored and HOME files are absent.
//!
//! Why a difference can only come from wsx reading those files:
//! - check reads committed objects and files under `.git`, never the worktree; the worktree is
//!   varied anyway, because that is where a regression would read.
//! - The six tracked files are committed and checked out in both states and `git status
//!   --porcelain` prints nothing in each, so a git command that stats the worktree sees a clean
//!   tree either way.
//! - The ignored file is listed in `info/exclude`, and HOME lies outside the repository.
//! - Nothing fetches, so both staleness times are reasons, not ages, and no clock text appears.
//! - The only text masked is each run's own workspace tip, which each JSON run must hold once.
//! - Content and presence sentinels, not `chmod 000`: the container gate runs as root, where a
//!   file mode does not stop a read.
//!
//! Reach: a read whose result reaches no output is not seen here, which is what
//! `tests/if02_scan.rs` is for. Four reader variants on the test side show the comparison
//! detects a read through each channel: a tracked worktree file, the ignored file, a committed
//! blob and a HOME file.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::env::{ScenarioDirs, command, hermetic_env, inherited_env};
use crate::fixture::{Fixture, ManifestKind, RepoConfig, WORK};

/// The `wsx` binary cargo built for this test run.
const WSX: &str = env!("CARGO_BIN_EXE_wsx");

/// The SC5 files as tracked worktree paths, committed in both states.
const TRACKED: [&str; 6] = [
    ".planning/ROADMAP.md",
    ".planning/STATE.md",
    ".planning/config.json",
    "devflow.toml",
    ".claude/settings.json",
    ".codex/config.toml",
];

/// The git-ignored agent file, present in state A only.
const IGNORED: &str = ".claude/settings.local.json";

/// The agent files under HOME, present in state A only.
const HOME_FILES: [&str; 2] = [".claude/settings.json", ".codex/config.toml"];

/// What each run's own workspace tip is replaced with before comparing.
const TIP_PLACEHOLDER: &str = "<workspace tip>";

/// The preset's workspace branch.
const WORKSPACE_REF: &str = "refs/heads/workspace/dev";

/// The malformed text every tracked file holds in state B.
const MALFORMED: &[u8] = b"if02 sentinel-B: not JSON, TOML or a roadmap\n";

/// The two agent-file states of the fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    A,
    B,
}

/// The content of `path` in `state`: in A a well-formed file of the path's kind carrying
/// "if02 sentinel-A", in B one malformed text for every path.
fn sentinel(path: &str, state: State) -> Vec<u8> {
    if state == State::B {
        return MALFORMED.to_vec();
    }
    let text = if path.ends_with(".json") {
        "{\"note\": \"if02 sentinel-A\"}\n"
    } else if path.ends_with(".toml") {
        "note = \"if02 sentinel-A\"\n"
    } else if path.ends_with("ROADMAP.md") {
        "# Roadmap\n\n### Phase 1: Example\n\nif02 sentinel-A\n"
    } else {
        "# State\n\nif02 sentinel-A\n"
    };
    text.as_bytes().to_vec()
}

/// Writes `path`'s state-A sentinel to `full` in state A, and removes `full` when present in
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
    fs::write(full, sentinel(path, State::A))
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
}

impl Read {
    fn label(&self) -> String {
        match self {
            Read::Worktree(path) => format!("worktree {path}"),
            Read::Object(path) => format!("object {path}"),
            Read::Home(path) => format!("home {path}"),
        }
    }

    /// The file's content as `runtime` holds it, with `tip` the current workspace tip.
    fn content(&self, runtime: &Runtime, tip: &str) -> Vec<u8> {
        match self {
            Read::Worktree(path) => file_or_absent(&runtime.fixture.checkout(WORK).join(path)),
            Read::Object(path) => runtime.git(&["cat-file", "blob", &format!("{tip}:{path}")]),
            Read::Home(path) => file_or_absent(&runtime.home.join(path)),
        }
    }
}

/// One run of `wsx`, with each run's own workspace tip masked.
struct Observation {
    command: String,
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

    /// Runs the built `wsx` with `args` in `work`; with `read`, appends that file's
    /// content, escaped, to stdout.
    fn observe(&self, args: &[&str], read: Option<&Read>) -> Observation {
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
        }
    }
}

/// One difference: the stream named, then its state-A and state-B text, escaped.
fn difference(command: &str, stream: &str, a: &str, b: &str) -> String {
    format!(
        "{command}: {stream} differs between state A and state B\n  A: {}\n  B: {}",
        a.as_bytes().escape_ascii(),
        b.as_bytes().escape_ascii()
    )
}

/// Every way `a` and `b`, one command in the two states, differ.
fn differences(a: &Observation, b: &Observation) -> Vec<String> {
    let mut found = Vec::new();
    if a.exit != b.exit {
        let (exit_a, exit_b) = (format!("{:?}", a.exit), format!("{:?}", b.exit));
        found.push(difference(&a.command, "exit code", &exit_a, &exit_b));
    }
    found
}

/// `check` and `check --json` observed in state A and then state B, paired by command.
fn run_states(name: &str, read: Option<Read>) -> Vec<(Observation, Observation)> {
    let commands: [&[&str]; 2] = [&["check"], &["check", "--json"]];
    let mut runtime = runtime(name);
    runtime.enter(State::A);
    let in_a: Vec<Observation> = commands
        .iter()
        .map(|args| runtime.observe(args, read.as_ref()))
        .collect();
    runtime.enter(State::B);
    let in_b: Vec<Observation> = commands
        .iter()
        .map(|args| runtime.observe(args, read.as_ref()))
        .collect();
    in_a.into_iter().zip(in_b).collect()
}

/// Every pair's differences, one per line.
fn all_differences(pairs: &[(Observation, Observation)]) -> String {
    pairs
        .iter()
        .flat_map(|(a, b)| differences(a, b))
        .collect::<Vec<_>>()
        .join("\n")
}

/// With `read` added, the states differ and the difference carries both `expected`
/// texts.
fn assert_read_differs(name: &str, read: Read, what: &str, expected: [&str; 2]) {
    let found = all_differences(&run_states(name, Some(read)));
    assert!(
        !found.is_empty(),
        "a read of {what} made no difference between the states"
    );
    for text in expected {
        assert!(found.contains(text), "{text} is missing from: {found}");
    }
}

/// check and check --json give the same exit code, stdout and stderr whatever
/// the SC5 files hold, and state A passes.
#[test]
fn check_output_does_not_depend_on_agent_files() {
    let pairs = run_states("check_output_does_not_depend_on_agent_files", None);
    let found = all_differences(&pairs);
    assert!(found.is_empty(), "{found}");
    let (human, _) = &pairs[0];
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
    for (a, b) in &pairs[1..] {
        assert_eq!(
            (a.masked, b.masked),
            (1, 1),
            "{}: workspace tips masked",
            a.command
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
        ["sentinel-A", "sentinel-B"],
    );
}

/// A read of the git-ignored agent file makes the states differ.
#[test]
fn ignored_file_read_makes_the_states_differ() {
    assert_read_differs(
        "ignored_file_read_makes_the_states_differ",
        Read::Worktree(IGNORED),
        "the worktree's .claude/settings.local.json",
        ["sentinel-A", "absent"],
    );
}

/// A read of a committed blob at the workspace tip makes the states differ.
#[test]
fn object_read_makes_the_states_differ() {
    assert_read_differs(
        "object_read_makes_the_states_differ",
        Read::Object(".planning/STATE.md"),
        "the workspace tip's .planning/STATE.md",
        ["sentinel-A", "sentinel-B"],
    );
}

/// A read of a HOME agent file makes the states differ.
#[test]
fn home_read_makes_the_states_differ() {
    assert_read_differs(
        "home_read_makes_the_states_differ",
        Read::Home(".codex/config.toml"),
        "HOME's .codex/config.toml",
        ["sentinel-A", "absent"],
    );
}
