//! IF-02 source scan: wsx must not read GSD, devflow or agent files.
//!
//! Every `*.rs` file under `src/` is tokenised with proc-macro2, and values
//! are tested for `ROADMAP.md`, `STATE.md`, `.planning/config.json`,
//! `devflow.toml`, or `.claude` or `.codex` as a whole path component (D-14).
//! What the scan matches:
//! - each string, byte-string, C-string, raw-string or char literal by its
//!   decoded value, so escapes (`\x2e`, `\u{2e}`, a line continuation) are seen;
//! - `concat!` by its joined value, nested, `std::` or `core::` qualified and
//!   with any delimiter, each argument still checked alone;
//! - chained `.join` calls whose arguments are each one literal or one
//!   `concat!`, joined as a path, so `Path::new(".planning").join("config.json")`
//!   is a finding.
//!
//! `.planning` alone is manifest path data to wsx (D-14), not a finding, and
//! so is `.planning` beside `config.json` in a tuple, an array or unrelated
//! functions.
//!
//! What it leaves: `format!`, consts and statics, `stringify!`, `env!`,
//! `include_str!`, a renamed `concat!`, arrays or tuples collected into a
//! path, `push`, a join argument that is not one literal or one `concat!`,
//! separators or dot components spelled apart (`.planning//config.json`,
//! `.planning/./config.json`), and anything built at run time. For `wsx check`,
//! `tests/harness/if02_runtime.rs` covers those reads by comparing its output
//! across agent-file states.
//!
//! Tokens, not text: comments never become tokens and doc comments become
//! `#[doc]` attributes, which are skipped whole, so a comment that names one
//! of those files is not a read while a literal in the same file is. A regex
//! cannot draw that line across raw strings and nested block comments. A file
//! the lexer refuses is a finding, never skipped; the lexer refuses every
//! malformed escape, so the decoder panics on an escape it does not know
//! instead of skipping the literal. There is no allowlist: the agent-hook
//! exception of the 2026-11-15 amendment is not built until its scope is
//! decided.
//!
//! The real `src/` is the positive case. The controls build their trees,
//! the planted copy of `src/` included, under CARGO_TARGET_TMPDIR, because
//! the container mounts the checkout read-only and nothing may be planted in
//! the checkout's own `src/`.

use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};

const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");

/// The fewest `*.rs` files the real `src/` held when the scan was written.
const MIN_SOURCE_FILES: usize = 20;

/// What one scan saw: how many files it tokenised and what it found.
struct Scan {
    files: usize,
    findings: Vec<String>,
}

/// One literal: its text as written, which findings quote, and its value.
struct Piece {
    source: String,
    value: Vec<u8>,
}

/// A value built from several literals: its source as findings quote it,
/// the value, and the pieces it was built from.
struct Joined {
    source: String,
    value: Vec<u8>,
    covers: Range<usize>,
    kind: JoinKind,
}

/// How a joined value was built, which decides its finding's text.
#[derive(Clone, Copy)]
enum JoinKind {
    /// A `concat!` value, reported like a literal.
    Concat,
    /// A chain of `.join` calls, reported as a joined path.
    Chain,
}

/// Every literal of one file, and every value joined from them.
#[derive(Default)]
struct Literals {
    pieces: Vec<Piece>,
    joined: Vec<Joined>,
}

/// Names whose appearance anywhere in a literal is a finding.
const NAMED: [&str; 4] = [
    "ROADMAP.md",
    "STATE.md",
    ".planning/config.json",
    "devflow.toml",
];

/// Directory names that are a finding only as a whole path component.
const COMPONENTS: [&str; 2] = [".claude", ".codex"];

/// Scans every `*.rs` file under `root` for literals naming a GSD, devflow
/// or agent file. Files are read in sorted order; one that is not UTF-8 or
/// that the lexer refuses is a finding.
fn scan(root: &Path) -> Scan {
    let mut paths = Vec::new();
    collect_rs_files(root, &mut paths);
    paths.sort();
    let mut findings = Vec::new();
    for path in &paths {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        let bytes = fs::read(path).expect("read a source file");
        let tokens = String::from_utf8(bytes)
            .map_err(|error| error.to_string())
            .and_then(|text| TokenStream::from_str(&text).map_err(|error| error.to_string()));
        match tokens {
            Ok(tokens) => {
                let mut literals = Literals::default();
                collect_literals(tokens, &mut literals);
                findings.extend(file_findings(&relative, &literals));
            }
            Err(error) => findings.push(format!("cannot tokenize {relative}: {error}")),
        }
    }
    Scan {
        files: paths.len(),
        findings,
    }
}

/// One file's findings: each literal whose value is forbidden, then each
/// joined value that is forbidden while none of the literals it was built
/// from was reported, inner values first.
fn file_findings(relative: &str, literals: &Literals) -> Vec<String> {
    let mut reported = vec![false; literals.pieces.len()];
    let mut findings = Vec::new();
    for (index, piece) in literals.pieces.iter().enumerate() {
        if let Some(name) = forbidden_name(&piece.value) {
            findings.push(literal_finding(relative, &piece.source, name));
            reported[index] = true;
        }
    }
    for joined in &literals.joined {
        if reported[joined.covers.clone()].contains(&true) {
            continue;
        }
        if let Some(name) = forbidden_name(&joined.value) {
            let finding = match joined.kind {
                JoinKind::Concat => literal_finding(relative, &joined.source, name),
                JoinKind::Chain => format!(
                    "{relative}: path joined from {} names {name}, a GSD, devflow or agent file wsx must not read (IF-02)",
                    joined.source
                ),
            };
            findings.push(finding);
            reported[joined.covers.clone()].fill(true);
        }
    }
    findings
}

/// The finding for a literal, or a `concat!` value, written as `source`.
fn literal_finding(relative: &str, source: &str, name: &str) -> String {
    format!(
        "{relative}: literal {source} names {name}, a GSD, devflow or agent file wsx must not read (IF-02)"
    )
}

/// Every file under `dir` whose name ends in `.rs`, recursively.
fn collect_rs_files(dir: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read a source directory") {
        let entry = entry.expect("read a directory entry");
        let kind = entry.file_type().expect("read an entry's type");
        if kind.is_dir() {
            collect_rs_files(&entry.path(), paths);
        } else if entry.file_name().to_string_lossy().ends_with(".rs") {
            paths.push(entry.path());
        }
    }
}

/// Every literal in `stream`, groups included, except inside doc attributes;
/// each `concat!` also adds its joined value, after its own arguments.
fn collect_literals(stream: TokenStream, literals: &mut Literals) {
    let trees: Vec<TokenTree> = stream.into_iter().collect();
    let mut at = 0;
    while at < trees.len() {
        if let Some(after) = doc_attribute_end(&trees, at) {
            at = after;
            continue;
        }
        if let Some((after, group)) = concat_end(&trees, at) {
            let (source, value) = (format!("concat!{group}"), concat_bytes(&group));
            push_joined(literals, [&group], source, value, JoinKind::Concat);
            at = after;
            continue;
        }
        if let Some((after, value, sources)) = join_chain_end(&trees, at) {
            let groups = trees[at..after].iter().filter_map(|tree| match tree {
                TokenTree::Group(group) => Some(group),
                _ => None,
            });
            push_joined(literals, groups, sources.join(", "), value, JoinKind::Chain);
            at = after;
            continue;
        }
        match &trees[at] {
            TokenTree::Group(group) => collect_literals(group.stream(), literals),
            TokenTree::Literal(literal) => {
                let source = literal.to_string();
                if let Some(value) = literal_value(&source) {
                    literals.pieces.push(Piece { source, value });
                }
            }
            TokenTree::Ident(_) | TokenTree::Punct(_) => {}
        }
        at += 1;
    }
}

/// Collects the literals of `groups`, then pushes the value of `kind`
/// joined from them, covering those literals.
fn push_joined<'a>(
    literals: &mut Literals,
    groups: impl IntoIterator<Item = &'a Group>,
    source: String,
    value: Vec<u8>,
    kind: JoinKind,
) {
    let start = literals.pieces.len();
    for group in groups {
        collect_literals(group.stream(), literals);
    }
    literals.joined.push(Joined {
        source,
        value,
        covers: start..literals.pieces.len(),
        kind,
    });
}

/// The value of a call's argument `group` when it is parenthesised and holds
/// exactly one literal with a value, or exactly one `concat!`, optionally
/// `std::` or `core::` qualified; anything else is `None`.
fn argument_value(group: &Group) -> Option<Vec<u8>> {
    if group.delimiter() != Delimiter::Parenthesis {
        return None;
    }
    let trees: Vec<TokenTree> = group.stream().into_iter().collect();
    if let [TokenTree::Literal(literal)] = trees.as_slice() {
        return literal_value(&literal.to_string());
    }
    let start = match trees.as_slice() {
        [
            TokenTree::Ident(root),
            TokenTree::Punct(first),
            TokenTree::Punct(second),
            ..,
        ] if (root == "std" || root == "core")
            && first.as_char() == ':'
            && second.as_char() == ':' =>
        {
            3
        }
        _ => 0,
    };
    let (end, inner) = concat_end(&trees, start)?;
    (end == trees.len()).then(|| concat_bytes(&inner))
}

/// When `trees[start]` begins a chain, a call whose argument has an
/// `argument_value` followed by one or more `.join` calls of the same kind,
/// the index past the last call, the path joined from the arguments' values
/// and their source texts. A single call is not a chain.
fn join_chain_end(trees: &[TokenTree], start: usize) -> Option<(usize, Vec<u8>, Vec<String>)> {
    let call = |at: usize| match (trees.get(at)?, trees.get(at + 1)?) {
        (TokenTree::Ident(_), TokenTree::Group(group)) => {
            Some((argument_value(group)?, group.stream().to_string()))
        }
        _ => None,
    };
    let (mut value, source) = call(start)?;
    let mut sources = vec![source];
    let mut end = start + 2;
    while let (Some(TokenTree::Punct(dot)), Some(TokenTree::Ident(join))) =
        (trees.get(end), trees.get(end + 1))
        && dot.as_char() == '.'
        && join == "join"
    {
        let Some((next, source)) = call(end + 1) else {
            break;
        };
        while value.last() == Some(&b'/') {
            value.pop();
        }
        value.push(b'/');
        value.extend(next);
        sources.push(source);
        end += 3;
    }
    (sources.len() > 1).then_some((end, value, sources))
}

/// When `trees[start]` is `concat`, then `!`, then a group of any delimiter,
/// the index just past the group and the group. A `std::` or `core::` path
/// before it needs nothing extra: the ident is still `concat`.
fn concat_end(trees: &[TokenTree], start: usize) -> Option<(usize, Group)> {
    let (TokenTree::Ident(ident), TokenTree::Punct(bang), TokenTree::Group(group)) = (
        trees.get(start)?,
        trees.get(start + 1)?,
        trees.get(start + 2)?,
    ) else {
        return None;
    };
    (ident == "concat" && bang.as_char() == '!').then(|| (start + 3, group.clone()))
}

/// The value `concat!` gives for `group`: each top-level literal's value,
/// or its text as written when it is a number, and each nested `concat!`'s
/// value, in order.
fn concat_bytes(group: &Group) -> Vec<u8> {
    let trees: Vec<TokenTree> = group.stream().into_iter().collect();
    let mut value = Vec::new();
    let mut at = 0;
    while at < trees.len() {
        if let Some((after, inner)) = concat_end(&trees, at) {
            value.extend(concat_bytes(&inner));
            at = after;
            continue;
        }
        if let TokenTree::Literal(literal) = &trees[at] {
            let source = literal.to_string();
            value.extend(literal_value(&source).unwrap_or_else(|| source.into_bytes()));
        }
        at += 1;
    }
    value
}

/// The value of the literal written as `source`: a string, byte string, C
/// string, char or byte char unescaped, a raw string verbatim. A number is
/// `None`. A suffix after the closing quote is ignored.
fn literal_value(source: &str) -> Option<Vec<u8>> {
    let rest = source
        .strip_prefix('b')
        .or_else(|| source.strip_prefix('c'))
        .unwrap_or(source);
    if let Some(raw) = rest.strip_prefix('r')
        && raw.starts_with(['#', '"'])
    {
        let (open, close) = (raw.find('"')?, raw.rfind('"')?);
        return Some(raw.as_bytes()[open + 1..close].to_vec());
    }
    let quote = rest
        .chars()
        .next()
        .filter(|first| matches!(first, '"' | '\''))?;
    let (open, close) = (rest.find(quote)?, rest.rfind(quote)?);
    Some(unescape(&rest[open + 1..close], source))
}

/// The bytes `body`, the inside of the literal `source`, stands for. The
/// lexer has already refused every malformed escape, so an escape this does
/// not know is a decoder defect: it panics naming `source`.
fn unescape(body: &str, source: &str) -> Vec<u8> {
    let mut value = Vec::new();
    let mut chars = body.chars().peekable();
    while let Some(next) = chars.next() {
        if next != '\\' {
            push_char(&mut value, next);
            continue;
        }
        match chars.next() {
            Some('n') => value.push(b'\n'),
            Some('r') => value.push(b'\r'),
            Some('t') => value.push(b'\t'),
            Some('0') => value.push(0),
            Some(quoted @ ('\\' | '\'' | '"')) => push_char(&mut value, quoted),
            Some('x') => {
                let digits: String = chars.by_ref().take(2).collect();
                let byte = u8::from_str_radix(&digits, 16)
                    .unwrap_or_else(|error| panic!("{source}: \\x{digits}: {error}"));
                value.push(byte);
            }
            Some('u') => push_char(&mut value, unicode_escape(&mut chars, source)),
            Some('\n' | '\r') => {
                while chars
                    .next_if(|next| matches!(next, ' ' | '\t' | '\r' | '\n'))
                    .is_some()
                {}
            }
            other => panic!("{source}: cannot decode the escape \\{other:?}"),
        }
    }
    value
}

/// Appends `next`'s UTF-8 bytes to `value`.
fn push_char(value: &mut Vec<u8>, next: char) {
    let mut buffer = [0; 4];
    value.extend_from_slice(next.encode_utf8(&mut buffer).as_bytes());
}

/// The char of a `\u{...}` escape whose `u` was just read from `chars`:
/// hex digits of either case, underscores ignored.
fn unicode_escape(chars: &mut impl Iterator<Item = char>, source: &str) -> char {
    let braced: String = chars.take_while(|next| *next != '}').collect();
    let digits: String = braced
        .strip_prefix('{')
        .unwrap_or_else(|| panic!("{source}: \\u without a brace"))
        .chars()
        .filter(|digit| *digit != '_')
        .collect();
    u32::from_str_radix(&digits, 16)
        .ok()
        .and_then(char::from_u32)
        .unwrap_or_else(|| panic!("{source}: \\u{{{digits}}} is not a char"))
}

/// When `trees[start]` opens a doc attribute (`#` or `# !`, then a bracket
/// group whose first token is `doc`), the index just past it.
fn doc_attribute_end(trees: &[TokenTree], start: usize) -> Option<usize> {
    let TokenTree::Punct(hash) = trees.get(start)? else {
        return None;
    };
    if hash.as_char() != '#' {
        return None;
    }
    let mut at = start + 1;
    if let Some(TokenTree::Punct(bang)) = trees.get(at)
        && bang.as_char() == '!'
    {
        at += 1;
    }
    let TokenTree::Group(group) = trees.get(at)? else {
        return None;
    };
    if group.delimiter() != Delimiter::Bracket {
        return None;
    }
    match group.stream().into_iter().next()? {
        TokenTree::Ident(ident) if ident == "doc" => Some(at + 1),
        _ => None,
    }
}

/// Whether `needle` appears anywhere in `haystack`.
fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The forbidden name the decoded `value` holds, if any.
fn forbidden_name(value: &[u8]) -> Option<&'static str> {
    NAMED
        .into_iter()
        .find(|name| contains_bytes(value, name.as_bytes()))
        .or_else(|| {
            COMPONENTS
                .into_iter()
                .find(|name| holds_component(value, name))
        })
}

/// Whether `name` appears in `value` with the start or end of the value, or
/// a `/`, on both sides.
fn holds_component(value: &[u8], name: &str) -> bool {
    let name = name.as_bytes();
    let bounds = |byte: Option<&u8>| byte.is_none_or(|byte| *byte == b'/');
    value.windows(name.len()).enumerate().any(|(at, window)| {
        let before = at.checked_sub(1).and_then(|index| value.get(index));
        window == name && bounds(before) && bounds(value.get(at + name.len()))
    })
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

/// A copy of the real `src/` under CARGO_TARGET_TMPDIR/if02-scan/`name`, plus
/// `planted.rs` holding `contents`.
fn planted_copy(name: &str, contents: &str) -> PathBuf {
    let src = scratch_src(name);
    copy_tree(&Path::new(REPO_ROOT).join("src"), &src);
    fs::write(src.join("planted.rs"), contents).expect("write planted.rs");
    src
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
    let src = planted_copy(
        "planted",
        "pub fn planted() -> std::io::Result<Vec<u8>> {\n    std::fs::read(\".planning/config.json\")\n}\n",
    );
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

#[test]
fn planted_escaped_read_trips_the_scan() {
    let src = planted_copy(
        "planted-escaped",
        r#"pub fn planted() -> std::io::Result<Vec<u8>> {
    std::fs::read("STATE\x2emd")
}
"#,
    );
    let findings = scan(&src).findings;
    assert_eq!(findings.len(), 1, "{findings:?}");
    for part in [
        "planted.rs",
        r#""STATE\x2emd""#,
        "names STATE.md,",
        "(IF-02)",
    ] {
        assert!(findings[0].contains(part), "{part:?} in {:?}", findings[0]);
    }
}

#[test]
fn escaped_spellings_trip_the_scan() {
    let cases = [
        (r#""STATE\x2emd""#, "STATE.md"),
        (r#""STATE\x2Emd""#, "STATE.md"),
        (r#""ROADMAP\u{2e}md""#, "ROADMAP.md"),
        (r#""ROADMAP\u{2E}md""#, "ROADMAP.md"),
        (r#""ROADMAP\u{2_E}md""#, "ROADMAP.md"),
        (r#"b"devflow\x2etoml""#, "devflow.toml"),
        (r#"".planning\x2fconfig.json""#, ".planning/config.json"),
        (r#"".planning\x2Fconfig.json""#, ".planning/config.json"),
        (r#"".cl\x61ude/settings.json""#, ".claude"),
        (r#""\x2ecodex/config.toml""#, ".codex"),
        (r#"c"STATE\x2emd""#, "STATE.md"),
    ];
    for (index, (literal, name)) in cases.into_iter().enumerate() {
        let src = one_file_tree(&format!("escaped-{index}"), &using(literal));
        let findings = scan(&src).findings;
        assert_eq!(findings.len(), 1, "{literal}: {findings:?}");
        assert!(
            findings[0].contains(literal) && findings[0].contains(&format!("names {name},")),
            "{literal}: {:?}",
            findings[0]
        );
    }
    let continued = "\"STATE\\\n        .md\"";
    let src = one_file_tree("escaped-continuation", &using(continued));
    let findings = scan(&src).findings;
    assert_eq!(findings.len(), 1, "{continued}: {findings:?}");
    for part in ["\"STATE\\", "names STATE.md,"] {
        assert!(findings[0].contains(part), "{part:?} in {:?}", findings[0]);
    }
}

#[test]
fn crlf_line_continuation_trips_the_scan() {
    let continued = "\"STATE\\\r\n        .md\"";
    let src = one_file_tree("escaped-crlf-continuation", &using(continued));
    let findings = scan(&src).findings;
    assert_eq!(findings.len(), 1, "{continued:?}: {findings:?}");
    for part in ["\"STATE\\", "names STATE.md,"] {
        assert!(findings[0].contains(part), "{part:?} in {:?}", findings[0]);
    }
}

#[test]
fn escaped_lookalikes_do_not_trip_the_scan() {
    let literals = [
        r#""STATE\x2ers""#,
        r#""ROADMAP\x5fmd""#,
        r#""x\x2eclaude""#,
        r#""\x2eclaudex""#,
        r#""only .planning\n""#,
    ];
    for (index, literal) in literals.into_iter().enumerate() {
        let src = one_file_tree(&format!("escaped-lookalike-{index}"), &using(literal));
        let findings = scan(&src).findings;
        assert!(findings.is_empty(), "{literal}: {findings:?}");
    }
}

#[test]
fn malformed_escapes_fail_closed_at_the_lexer() {
    let literals = [
        r#""STATE\x2gmd""#,
        r#""STATE\x80md""#,
        r#"b"STATE\u{2e}md""#,
        r#""\u{110000}""#,
    ];
    for (index, literal) in literals.into_iter().enumerate() {
        let src = one_file_tree(&format!("malformed-{index}"), &using(literal));
        let findings = scan(&src).findings;
        assert_eq!(findings.len(), 1, "{literal}: {findings:?}");
        assert!(
            findings[0].starts_with("cannot tokenize"),
            "{literal}: {:?}",
            findings[0]
        );
    }
}

#[test]
fn concat_spellings_trip_the_scan() {
    let cases: [(&str, &str, &[&str]); 7] = [
        (
            r#"concat!("STATE", ".md")"#,
            "STATE.md",
            &[r#""STATE""#, r#"".md""#],
        ),
        (
            r#"concat!(".planning/", "config.json")"#,
            ".planning/config.json",
            &[r#"".planning/""#, r#""config.json""#],
        ),
        (
            r#"concat!("devflow", '.', "toml")"#,
            "devflow.toml",
            &[r#""devflow""#, "'.'", r#""toml""#],
        ),
        (
            r#"concat!(concat!("ROAD", "MAP"), ".md")"#,
            "ROADMAP.md",
            &[r#""ROAD""#, r#""MAP""#, r#"".md""#],
        ),
        (
            r#"std::concat!(".cla", "ude/settings.json")"#,
            ".claude",
            &[r#"".cla""#, r#""ude/settings.json""#],
        ),
        (
            r#"core::concat!{".co", "dex/config.toml"}"#,
            ".codex",
            &[r#"".co""#, r#""dex/config.toml""#],
        ),
        (
            r#"concat!["STATE", "\x2emd"]"#,
            "STATE.md",
            &[r#""STATE""#, r#""\x2emd""#],
        ),
    ];
    for (index, (expression, name, arguments)) in cases.into_iter().enumerate() {
        let src = one_file_tree(&format!("concat-{index}"), &using(expression));
        let findings = scan(&src).findings;
        assert_eq!(findings.len(), 1, "{expression}: {findings:?}");
        let names = format!("names {name},");
        for part in ["concat!", names.as_str()].iter().chain(arguments) {
            assert!(findings[0].contains(part), "{part:?} in {:?}", findings[0]);
        }
    }
}

#[test]
fn concat_lookalikes_do_not_trip_the_scan() {
    let expressions = [
        r#"concat!("STATE", ".rs")"#,
        r#"concat!(".planning", "/other.json")"#,
        r#"concat!("devflow", ".tom")"#,
        r#"concat!("ROAD", "MAP")"#,
        r#"concat!(".plan", "ning")"#,
    ];
    for (index, expression) in expressions.into_iter().enumerate() {
        let src = one_file_tree(&format!("concat-lookalike-{index}"), &using(expression));
        let findings = scan(&src).findings;
        assert!(findings.is_empty(), "{expression}: {findings:?}");
    }
}

#[test]
fn planted_join_read_trips_the_scan() {
    let src = planted_copy(
        "planted-join",
        r#"pub fn planted() -> std::io::Result<Vec<u8>> {
    std::fs::read(std::path::Path::new(".planning").join("config.json"))
}
"#,
    );
    let findings = scan(&src).findings;
    assert_eq!(findings.len(), 1, "{findings:?}");
    let parts = [
        "planted.rs",
        r#"".planning""#,
        r#""config.json""#,
        "path joined from",
        "names .planning/config.json,",
        "(IF-02)",
    ];
    for part in parts {
        assert!(findings[0].contains(part), "{part:?} in {:?}", findings[0]);
    }
}

#[test]
fn join_spellings_trip_the_scan() {
    let expressions = [
        r#"std::path::Path::new(".planning").join("config.json")"#,
        r#"std::path::PathBuf::from(".planning/").join("config.json")"#,
        r#"root.join(".planning").join("config.json")"#,
        r#"std::path::Path::new(".").join(".planning").join("config.json")"#,
        r#"std::path::Path::new(concat!(".plan", "ning")).join("config.json")"#,
        r#"std::path::Path::new(".planning").join(concat!("config", ".json"))"#,
    ];
    for (index, expression) in expressions.into_iter().enumerate() {
        let src = one_file_tree(&format!("join-{index}"), &using(expression));
        let findings = scan(&src).findings;
        assert_eq!(findings.len(), 1, "{expression}: {findings:?}");
        for part in ["path joined from", "names .planning/config.json,"] {
            assert!(findings[0].contains(part), "{part:?} in {:?}", findings[0]);
        }
    }
}

#[test]
fn path_data_does_not_trip_the_scan() {
    let mut files: Vec<String> = [
        r#"".planning""#,
        r#"b".planning/x""#,
        r#"[".wsx.conf", ".worktrees", ".planning"]"#,
        r#"(b".planning", b"config.json")"#,
        r#"[".planning", "config.json"]"#,
        r#"std::path::Path::new(".planning").join("other.json")"#,
        r#"std::path::Path::new("config.json").join(".planning")"#,
        r#"std::path::Path::new(".planning").join("x").join("config.json")"#,
        r#""config.json""#,
    ]
    .into_iter()
    .map(using)
    .collect();
    files.push(
        "pub fn planning() -> &'static str {\n    \".planning\"\n}\n\n\
         pub fn config() -> &'static str {\n    \"config.json\"\n}\n"
            .to_owned(),
    );
    files.push(
        "pub fn bound() {\n    let _ = std::path::Path::new(\".planning\");\n    \
         let _ = std::path::Path::new(\"config.json\");\n}\n"
            .to_owned(),
    );
    for (index, contents) in files.iter().enumerate() {
        let src = one_file_tree(&format!("path-data-{index}"), contents);
        let findings = scan(&src).findings;
        assert!(findings.is_empty(), "{contents}: {findings:?}");
    }
}
