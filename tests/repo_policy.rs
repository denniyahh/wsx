//! Repository policy tests: properties of tracked configuration that must
//! hold for every contributor, checked through the tool that consumes each
//! file (rustc for the toolchain pin, git for `.gitconfig`). The action-pin
//! check has no consuming tool offline, so its checker is first shown to
//! reject the refs it exists to catch.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");

fn text(bytes: Vec<u8>, what: &str) -> String {
    String::from_utf8(bytes).unwrap_or_else(|err| panic!("{what} is not UTF-8: {err}"))
}

fn repo_path(relative: &str) -> PathBuf {
    Path::new(REPO_ROOT).join(relative)
}

// ---------------------------------------------------------------- DEV-02

/// The `channel` value of the `[toolchain]` table in rust-toolchain.toml.
fn pinned_channel() -> String {
    let path = repo_path("rust-toolchain.toml");
    let contents = fs::read_to_string(&path).expect("read rust-toolchain.toml");
    let mut in_toolchain = false;
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_toolchain = line == "[toolchain]";
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if in_toolchain && key.trim() == "channel" {
            let value = value.split_once(" #").map_or(value, |(v, _)| v).trim();
            return value.trim_matches('"').to_owned();
        }
    }
    panic!("no [toolchain] channel in {}", path.display());
}

/// A rustup-proxied command run at the repository root, with the inherited
/// `RUSTUP_TOOLCHAIN` removed: the outer cargo run sets it, and it outranks
/// rust-toolchain.toml, so leaving it would measure the outer run instead.
fn at_repo_root(program: &str, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .current_dir(REPO_ROOT)
        .env_remove("RUSTUP_TOOLCHAIN")
        .output()
        .unwrap_or_else(|err| panic!("spawn {program}: {err}"))
}

#[test]
fn rustc_at_repo_root_is_the_pinned_channel() {
    let channel = pinned_channel();
    let output = at_repo_root("rustc", &["--version"]);
    let stdout = text(output.stdout, "rustc stdout");
    assert_eq!(output.status.code(), Some(0), "rustc --version exit code");

    // `rustc 1.99.1 (58cf8f9c3 2026-10-13)`: the second field is the release.
    // A floating channel such as `stable` never equals it, so this also fails
    // if the pin stops being an exact version.
    let release = stdout.split_whitespace().nth(1).unwrap_or_default();
    assert_eq!(
        release, channel,
        "rustc at the repository root must be the release rust-toolchain.toml pins; \
         rustc said {stdout:?}"
    );
}

#[test]
fn rust_toolchain_toml_is_what_selects_the_compiler() {
    // An equal version could also come from the default toolchain happening
    // to match; rustup names the source of the active toolchain.
    let output = at_repo_root("rustup", &["show", "active-toolchain"]);
    let stdout = text(output.stdout, "rustup stdout");
    assert_eq!(
        output.status.code(),
        Some(0),
        "rustup show active-toolchain exit code"
    );

    let source = format!(
        "(overridden by '{}')",
        repo_path("rust-toolchain.toml").display()
    );
    assert!(
        stdout.contains(&source),
        "the active toolchain must come from the repository's rust-toolchain.toml, \
         got {stdout:?}"
    );
}

// ---------------------------------------------------------------- DEV-05

/// Every key and value the tracked .gitconfig must set, and nothing else.
const GIT_POLICY: [(&str, &str); 10] = [
    ("gpg.format", "ssh"),
    ("commit.gpgsign", "true"),
    ("tag.gpgsign", "true"),
    ("gitflow.branch.master", "main"),
    ("gitflow.branch.develop", "develop"),
    ("gitflow.prefix.feature", "feature/"),
    ("gitflow.prefix.release", "release/"),
    ("gitflow.prefix.hotfix", "hotfix/"),
    ("gitflow.prefix.support", "support/"),
    ("gitflow.prefix.versiontag", "v"),
];

/// `git config -f <file> --list`, as git itself parses the file. The child
/// starts from an empty environment, so no inherited `GIT_*` variable
/// (`GIT_DIR`, `GIT_CONFIG_PARAMETERS`, ...) and no global or system config
/// can change what it reports.
fn git_config_entries(file: &Path) -> Vec<(String, String)> {
    let home = Path::new(TMP_ROOT).join("git-policy-home");
    fs::create_dir_all(&home).expect("create scratch HOME for git");
    let output = Command::new("git")
        .args(["config", "--no-includes", "-f"])
        .arg(file)
        .arg("--list")
        .env_clear()
        .env("PATH", std::env::var_os("PATH").expect("PATH is set"))
        .env("HOME", &home)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("LC_ALL", "C")
        .output()
        .expect("spawn git config");
    let stdout = text(output.stdout, "git config stdout");
    assert_eq!(
        output.status.code(),
        Some(0),
        "git config -f {} --list exit code; stderr: {:?}",
        file.display(),
        text(output.stderr, "git config stderr")
    );
    stdout
        .lines()
        .map(|line| {
            let (key, value) = line.split_once('=').unwrap_or((line, ""));
            (key.to_owned(), value.to_owned())
        })
        .collect()
}

/// Every way `entries` differs from GIT_POLICY, one line each.
fn git_policy_violations(entries: &[(String, String)]) -> Vec<String> {
    let mut violations = Vec::new();
    for (key, value) in entries {
        if key.starts_with("user.") {
            violations.push(format!("personal key {key} must stay in local config"));
        } else if !GIT_POLICY.contains(&(key.as_str(), value.as_str())) {
            violations.push(format!("unexpected {key}={value}"));
        }
    }
    for (key, value) in GIT_POLICY {
        let count = entries
            .iter()
            .filter(|(k, v)| k == key && v == value)
            .count();
        if count != 1 {
            violations.push(format!("{key}={value} set {count} times, expected once"));
        }
    }
    violations
}

#[test]
fn tracked_gitconfig_sets_exactly_the_shared_policy() {
    let entries = git_config_entries(&repo_path(".gitconfig"));
    let violations = git_policy_violations(&entries);
    assert!(
        violations.is_empty(),
        "tracked .gitconfig violates the repository policy: {violations:#?}\nentries: {entries:#?}"
    );
}

#[test]
fn gitconfig_policy_check_rejects_unsigned_and_personal_config() {
    let dir = Path::new(TMP_ROOT).join("git-policy-fixture");
    fs::create_dir_all(&dir).expect("create fixture dir");
    let file = dir.join("gitconfig");
    fs::write(
        &file,
        "[gpg]\n\tformat = ssh\n[commit]\n\tgpgsign = false\n\
         [user]\n\temail = someone@example.invalid\n\
         [gitflow \"branch\"]\n\tmaster = master\n",
    )
    .expect("write fixture gitconfig");

    let violations = git_policy_violations(&git_config_entries(&file));
    for expected in [
        "personal key user.email must stay in local config",
        "unexpected commit.gpgsign=false",
        "commit.gpgsign=true set 0 times, expected once",
        "tag.gpgsign=true set 0 times, expected once",
        "unexpected gitflow.branch.master=master",
    ] {
        assert!(
            violations.iter().any(|v| v == expected),
            "the policy check must report {expected:?}, got {violations:#?}"
        );
    }
}

// ---------------------------------------------------------------- SUP-04

/// Every `uses:` value in `workflow`, including flow-mapping forms such as
/// `- {uses: x@y}`. Comments are dropped first so a `# uses:` note is ignored.
fn uses_refs(workflow: &str) -> Vec<String> {
    let mut refs = Vec::new();
    for line in workflow.lines() {
        let code = if line.trim_start().starts_with('#') {
            ""
        } else {
            line.split_once(" #").map_or(line, |(code, _)| code)
        };
        let mut rest = code;
        while let Some(at) = rest.find("uses:") {
            let preceded_by_key_char = rest[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            let after = &rest[at + "uses:".len()..];
            if !preceded_by_key_char {
                let value: String = after
                    .trim_start()
                    .chars()
                    .take_while(|c| !c.is_whitespace() && *c != ',' && *c != '}')
                    .collect();
                refs.push(value.trim_matches(|c| c == '"' || c == '\'').to_owned());
            }
            rest = after;
        }
    }
    refs
}

/// A remote action or reusable workflow must be pinned to a full 40-character
/// lowercase commit SHA; a local `./` path is not fetched and is exempt.
fn is_sha_pinned(reference: &str) -> bool {
    if reference.starts_with("./") {
        return true;
    }
    let Some((_, git_ref)) = reference.rsplit_once('@') else {
        return false;
    };
    git_ref.len() == 40 && git_ref.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

fn unpinned_refs(workflow: &str) -> Vec<String> {
    uses_refs(workflow)
        .into_iter()
        .filter(|r| !is_sha_pinned(r))
        .collect()
}

fn workflow_files() -> Vec<PathBuf> {
    let dir = repo_path(".github/workflows");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
        .map(|entry| entry.expect("read workflow dir entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
        })
        .collect();
    files.sort();
    files
}

#[test]
fn every_workflow_action_is_pinned_to_a_full_commit_sha() {
    let files = workflow_files();
    assert!(
        !files.is_empty(),
        "no workflow files found; nothing was checked"
    );

    let mut checked = 0;
    let mut unpinned = Vec::new();
    for file in &files {
        let workflow = fs::read_to_string(file).expect("read workflow");
        checked += uses_refs(&workflow).len();
        for reference in unpinned_refs(&workflow) {
            unpinned.push(format!("{}: {reference}", file.display()));
        }
    }
    assert!(
        checked > 0,
        "no `uses:` found in {files:?}; nothing was checked"
    );
    assert!(
        unpinned.is_empty(),
        "actions not pinned to a full commit SHA: {unpinned:#?}"
    );
}

#[test]
fn sha_pin_check_rejects_tags_branches_and_short_shas() {
    let sha = "3d3c42e5aac5ba805825da76410c181273ba90b1";
    let fixture = format!(
        "jobs:\n  a:\n    steps:\n\
         \x20     - uses: actions/checkout@v7.0.1\n\
         \x20     - uses: actions/cache@main # tracks a branch\n\
         \x20     - uses: \"actions/setup@3d3c42e\"\n\
         \x20     - uses: actions/upper@{upper}\n\
         \x20     - uses: actions/nopin\n\
         \x20     - {{uses: actions/flow@v1, with: {{}}}}\n\
         \x20     - uses: actions/checkout@{sha} # v7.0.1\n\
         \x20     - uses: ./local-action\n\
         \x20     # - uses: actions/commented@v1\n\
         \x20 b:\n    uses: org/repo/.github/workflows/w.yml@v2\n",
        upper = sha.to_uppercase()
    );

    let unpinned = unpinned_refs(&fixture);
    let expected = [
        "actions/checkout@v7.0.1".to_owned(),
        "actions/cache@main".to_owned(),
        "actions/setup@3d3c42e".to_owned(),
        format!("actions/upper@{}", sha.to_uppercase()),
        "actions/nopin".to_owned(),
        "actions/flow@v1".to_owned(),
        "org/repo/.github/workflows/w.yml@v2".to_owned(),
    ];
    assert_eq!(
        unpinned, expected,
        "the pin check must reject exactly the unpinned refs"
    );
    assert_eq!(
        uses_refs(&fixture).len(),
        expected.len() + 2,
        "pinned and local refs are read too"
    );
}
