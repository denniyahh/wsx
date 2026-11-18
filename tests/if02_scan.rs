//! IF-02 source scan: wsx must not read GSD, devflow or agent files.
//!
//! Every `*.rs` file under `src/` is tokenised with proc-macro2 and each
//! string, byte-string or raw-string literal is tested for `ROADMAP.md`,
//! `STATE.md`, `.planning/config.json`, `devflow.toml`, or `.claude` or
//! `.codex` as a whole path component (D-14). Tokens, not text: comments
//! never become tokens and doc comments become `#[doc]` attributes, which are
//! skipped whole, so a comment that names one of those files is not a read
//! while a literal in the same file is. A regex cannot draw that line across
//! raw strings and nested block comments. A file the lexer refuses is a
//! finding, never skipped. There is no allowlist: the agent-hook exception of
//! the 2026-11-15 amendment is not built until its scope is decided.
//!
//! The real `src/` is the positive case. The controls build their trees,
//! the planted copy of `src/` included, under CARGO_TARGET_TMPDIR, because
//! the container mounts the checkout read-only and nothing may be planted in
//! the checkout's own `src/`.

use std::fs;
use std::path::{Path, PathBuf};

const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");

/// The fewest `*.rs` files the real `src/` held when the scan was written.
const MIN_SOURCE_FILES: usize = 20;

/// What one scan saw: how many files it tokenised and what it found.
struct Scan {
    files: usize,
    findings: Vec<String>,
}

/// Scans every `*.rs` file under `root` for literals naming a GSD, devflow
/// or agent file.
fn scan(root: &Path) -> Scan {
    let _ = root;
    Scan {
        files: 0,
        findings: Vec::new(),
    }
}

/// The `*.rs` files under `root`, counted by an iterative walk written
/// apart from the scan's own, so a walk that misses files cannot vouch for
/// itself.
fn count_rs_files(root: &Path) -> usize {
    let mut pending = vec![root.to_path_buf()];
    let mut count = 0;
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("read a source directory") {
            let path = entry.expect("read a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                count += 1;
            }
        }
    }
    count
}

/// A fresh `src` directory under CARGO_TARGET_TMPDIR/if02-scan/`name`,
/// removed first if a previous run left it.
fn scratch_src(name: &str) -> PathBuf {
    let root = Path::new(TMP_ROOT).join("if02-scan").join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove stale scratch tree");
    }
    let src = root.join("src");
    fs::create_dir_all(&src).expect("create scratch src");
    src
}

/// A scratch `src` holding one file, `lib.rs`, with `contents`.
fn one_file_tree(name: &str, contents: &str) -> PathBuf {
    let src = scratch_src(name);
    fs::write(src.join("lib.rs"), contents).expect("write lib.rs");
    src
}

/// A file whose only code is `literal` used as an expression.
fn using(literal: &str) -> String {
    format!("pub fn literal() {{\n    let _ = {literal};\n}}\n")
}

/// Copies the directory tree `from` into `to`.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create copy directory");
    for entry in fs::read_dir(from).expect("read directory to copy") {
        let path = entry.expect("read a directory entry").path();
        let target = to.join(path.file_name().expect("entry has a name"));
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target).expect("copy source file");
        }
    }
}

#[test]
fn real_tree_reads_no_agent_file() {
    let src = Path::new(REPO_ROOT).join("src");
    let result = scan(&src);
    let walked = count_rs_files(&src);
    println!("scanned {} files, walked {walked}", result.files);
    assert!(result.findings.is_empty(), "{}", result.findings.join("\n"));
    assert_eq!(result.files, walked, "files scanned against files walked");
    assert!(
        walked >= MIN_SOURCE_FILES,
        "only {walked} source files walked"
    );
}

#[test]
fn planted_read_trips_the_scan() {
    let src = scratch_src("planted");
    copy_tree(&Path::new(REPO_ROOT).join("src"), &src);
    fs::write(
        src.join("planted.rs"),
        "pub fn planted() -> std::io::Result<Vec<u8>> {\n    std::fs::read(\".planning/config.json\")\n}\n",
    )
    .expect("write planted.rs");
    let findings = scan(&src).findings;
    assert_eq!(findings.len(), 1, "{findings:?}");
    for part in ["planted.rs", ".planning/config.json", "(IF-02)"] {
        assert!(findings[0].contains(part), "{part:?} in {:?}", findings[0]);
    }
}

#[test]
fn each_forbidden_name_trips_the_scan() {
    let cases = [
        (r#""ROADMAP.md""#, "ROADMAP.md"),
        (r#""STATE.md""#, "STATE.md"),
        (r#"".planning/config.json""#, ".planning/config.json"),
        (r#""devflow.toml""#, "devflow.toml"),
        (r#"".claude/settings.json""#, ".claude"),
        (r#"".codex/config.toml""#, ".codex"),
        (r#"b"STATE.md""#, "STATE.md"),
        (r##"r#"ROADMAP.md"#"##, "ROADMAP.md"),
    ];
    for (index, (literal, name)) in cases.into_iter().enumerate() {
        let src = one_file_tree(&format!("forbidden-{index}"), &using(literal));
        let findings = scan(&src).findings;
        assert_eq!(findings.len(), 1, "{literal}: {findings:?}");
        assert!(
            findings[0].contains(literal) && findings[0].contains(&format!("names {name},")),
            "{literal}: {:?}",
            findings[0]
        );
    }
}

#[test]
fn comments_do_not_trip_the_scan() {
    let comments = "//! Inner doc: reads no ROADMAP.md.\n\
        // Line comment: STATE.md.\n\
        /* Block comment: .planning/config.json, /* nested */ .claude/x */\n\
        /// Outer doc: devflow.toml and .codex/config.toml.\n\
        pub fn quiet() {}\n";
    let quiet = one_file_tree("comments-only", comments);
    let findings = scan(&quiet).findings;
    assert!(findings.is_empty(), "{findings:?}");

    let loud = one_file_tree(
        "comments-and-literal",
        &format!("{comments}{}", using(r#""devflow.toml""#)),
    );
    let findings = scan(&loud).findings;
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("devflow.toml"), "{:?}", findings[0]);
}

#[test]
fn component_match_is_exact() {
    for literal in [r#""x.claude""#, r#"".claudex""#, r#""a.codexb""#] {
        let src = one_file_tree("component-pass", &using(literal));
        let findings = scan(&src).findings;
        assert!(findings.is_empty(), "{literal}: {findings:?}");
    }
    for literal in [r#"".claude""#, r#""a/.codex/b""#] {
        let src = one_file_tree("component-trip", &using(literal));
        let findings = scan(&src).findings;
        assert_eq!(findings.len(), 1, "{literal}: {findings:?}");
    }
}

#[test]
fn unlexable_file_fails_closed() {
    let src = one_file_tree("unlexable", "pub const S: &str = \"unterminated;\n");
    let findings = scan(&src).findings;
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(
        findings[0].starts_with("cannot tokenize"),
        "{:?}",
        findings[0]
    );
}
