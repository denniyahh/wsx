//! The `.wsx.conf` grammar: which paths a workspace branch keeps personal.
//!
//! The parser keeps every line it accepts, in file order, built-ins and
//! duplicates included, with its line number and raw text, so a later check
//! can name the line a problem came from (D-13).

use std::fmt;

use crate::git::{c_quote, c_unquote};

/// The longest manifest `parse` reads: 64 KiB. By the operator's ruling of
/// 2026-11-05 under D-12, "only ambiguity is invalid" contrasts ambiguity
/// with staleness and does not forbid a size limit, and a manifest over
/// 64 KiB is invalid: the overlap check is quadratic in the entry count and
/// the blob can arrive through a merge (T-04-10).
const MAX_MANIFEST_BYTES: usize = 65_536;

/// What a manifest line declares about its path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// `only <path>`: the path is personal and never reaches a shared branch.
    Only,
    /// `modified <path>`: a shared path the workspace branch changes.
    Modified,
}

/// One accepted manifest line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// What the line declares.
    pub kind: EntryKind,
    /// The path, raw bytes, one trailing `/` stripped (D-14).
    pub path: Vec<u8>,
    /// The line's number, from 1, counting every line.
    pub line: usize,
    /// The line's raw bytes, without its newline.
    pub text: Vec<u8>,
}

/// A parsed manifest: every accepted line, in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// The entries, duplicates and built-ins kept.
    pub entries: Vec<Entry>,
}

/// Why a manifest line was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The line is not a comment, not empty and not an `only` or `modified`
    /// line (D-11).
    UnknownKind,
    /// A path written bare starts or ends with space, tab, CR, VT or FF,
    /// one trailing `/` set aside; such a path must be C-quoted (D-19).
    EdgeWhitespace,
    /// A path starting with a double quote is not exactly one C-quoted
    /// string (D-19).
    BadQuoting {
        /// Why the quoted string was refused.
        reason: String,
    },
    /// The line ends in a carriage return, as every line of a CRLF file
    /// does (D-19). Checked before anything else, comments included.
    LineEndsInCr,
    /// The path is empty: the keyword alone, the keyword and one space, a
    /// lone `/`, or `""` (D-14).
    EmptyPath,
    /// The path starts with `/` (D-14).
    AbsolutePath,
    /// The path has an empty component, as `a//b` does (D-14).
    EmptyComponent,
    /// The path has a `.` or `..` component (D-14).
    DotComponent,
    /// The path holds a NUL byte, which no git tree path can (D-14's reason).
    Nul,
    /// The path is both `only` and `modified`: this line and an earlier line
    /// of the other kind cover one another, in either direction (D-12).
    Overlap {
        /// The earlier line's number.
        other_line: usize,
        /// The earlier line's raw bytes, without its newline.
        other_text: Vec<u8>,
    },
    /// The manifest is longer than `MAX_MANIFEST_BYTES`; reported at the
    /// line holding the first byte past the limit, with no line text.
    TooLarge {
        /// The manifest's length in bytes.
        size: usize,
    },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::UnknownKind => {
                f.write_str(r#"expected a comment or an "only <path>" or "modified <path>" line"#)
            }
            Problem::EdgeWhitespace => {
                f.write_str("a path with leading or trailing whitespace must be C-quoted")
            }
            Problem::BadQuoting { reason } => {
                write!(f, "the C-quoted path is malformed: {reason}")
            }
            Problem::LineEndsInCr => f.write_str(
                "the line ends in a carriage return (CRLF line endings are not allowed)",
            ),
            Problem::EmptyPath => f.write_str("the path is empty"),
            Problem::AbsolutePath => f.write_str("the path starts with /"),
            Problem::EmptyComponent => f.write_str("the path has an empty component (//)"),
            Problem::DotComponent => f.write_str("the path has a . or .. component"),
            Problem::Nul => f.write_str("the path contains a NUL byte"),
            Problem::Overlap {
                other_line,
                other_text,
            } => write!(
                f,
                "this line overlaps line {other_line}: {}; a path cannot be both only and modified",
                String::from_utf8_lossy(&c_quote(other_text))
            ),
            Problem::TooLarge { size } => write!(
                f,
                "the manifest is {size} bytes, over the limit of {MAX_MANIFEST_BYTES} bytes (64 KiB)"
            ),
        }
    }
}

/// A refused manifest: the first line that failed and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The line's number, from 1.
    pub line: usize,
    /// The line's raw bytes, without its newline; empty for
    /// `Problem::TooLarge`, whose line can be arbitrarily long.
    pub text: Vec<u8>,
    /// What is wrong with it.
    pub problem: Problem,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.problem)?;
        if self.text.is_empty() {
            return Ok(());
        }
        write!(f, ": {}", String::from_utf8_lossy(&c_quote(&self.text)))
    }
}

/// Parses a manifest blob. A manifest longer than `MAX_MANIFEST_BYTES` is
/// refused first, before any line is read. Otherwise lines are split on
/// `\n` and numbered from 1, each read by `read_line` and then checked
/// against the earlier entries by `overlap`. The first line refused refuses
/// the whole manifest, naming that line. Nothing is filtered or deduped
/// (D-13).
pub fn parse(bytes: &[u8]) -> Result<Manifest, ParseError> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(ParseError {
            line: line_holding(bytes, MAX_MANIFEST_BYTES),
            text: Vec::new(),
            problem: Problem::TooLarge { size: bytes.len() },
        });
    }
    let mut entries = Vec::new();
    for (index, text) in bytes.split(|byte| *byte == b'\n').enumerate() {
        let line = index + 1;
        let refuse = |problem| ParseError {
            line,
            text: text.to_vec(),
            problem,
        };
        let Some((kind, path)) = read_line(text).map_err(refuse)? else {
            continue;
        };
        if let Some(other) = overlap(&entries, &kind, &path) {
            return Err(refuse(Problem::Overlap {
                other_line: other.line,
                other_text: other.text.clone(),
            }));
        }
        entries.push(Entry {
            kind,
            path,
            line,
            text: text.to_vec(),
        });
    }
    Ok(Manifest { entries })
}

/// The 1-based number of the line holding `bytes[index]`, counting lines as
/// `parse` does: a newline belongs to the line it ends.
fn line_holding(bytes: &[u8], index: usize) -> usize {
    1 + bytes[..index].iter().filter(|byte| **byte == b'\n').count()
}

/// The first earlier entry of the other kind that `path` overlaps: either
/// path covers the other (D-12). Entries of the same kind never overlap, so
/// duplicates and nesting pass. Quadratic over a manifest, which
/// `MAX_MANIFEST_BYTES` bounds (T-04-10).
fn overlap<'a>(entries: &'a [Entry], kind: &EntryKind, path: &[u8]) -> Option<&'a Entry> {
    entries.iter().find(|entry| {
        entry.kind != *kind && (covers(&entry.path, path) || covers(path, &entry.path))
    })
}

/// The entry one manifest line declares, or `None` for a comment or an empty
/// line. In order: a line ending in CR is refused, a comment included
/// (D-19); an empty line or one starting with `#` is skipped (D-11); the
/// keyword is split off by `keyword`; the value is read by `path_value`; and
/// the path is checked by `normalize_path` (D-14).
fn read_line(text: &[u8]) -> Result<Option<(EntryKind, Vec<u8>)>, Problem> {
    if text.last() == Some(&b'\r') {
        return Err(Problem::LineEndsInCr);
    }
    if text.is_empty() || text.starts_with(b"#") {
        return Ok(None);
    }
    let (kind, value) = keyword(text)?;
    let path = normalize_path(&path_value(value)?)?;
    Ok(Some((kind, path)))
}

/// A line's kind and its value, everything after the keyword and one space
/// (D-19). A line that is exactly `only` or `modified` has an empty path;
/// any other line not starting with a keyword and one space is unknown, a
/// keyword followed by a tab or another byte included (D-11).
fn keyword(text: &[u8]) -> Result<(EntryKind, &[u8]), Problem> {
    for (word, kind) in [
        (&b"only"[..], EntryKind::Only),
        (b"modified", EntryKind::Modified),
    ] {
        let Some(rest) = text.strip_prefix(word) else {
            continue;
        };
        if rest.is_empty() {
            return Err(Problem::EmptyPath);
        }
        if let Some(value) = rest.strip_prefix(b" ") {
            return Ok((kind, value));
        }
    }
    Err(Problem::UnknownKind)
}

/// The path a manifest line's value names, before `normalize_path` (D-19).
/// A value starting with a double quote must be exactly one C-quoted string,
/// read by `c_unquote`; any other value is its bytes as written, refused
/// when it starts or ends with edge whitespace once one trailing `/` is set
/// aside. The `/` stays on: `normalize_path` strips it, once.
fn path_value(value: &[u8]) -> Result<Vec<u8>, Problem> {
    if value.starts_with(b"\"") {
        return c_unquote(value).map_err(|reason| Problem::BadQuoting { reason });
    }
    if has_edge_whitespace(value.strip_suffix(b"/").unwrap_or(value)) {
        return Err(Problem::EdgeWhitespace);
    }
    Ok(value.to_vec())
}

/// A manifest path as an entry holds it: one trailing `/` stripped, then
/// refused if it could never name a git tree path (D-14): a NUL byte, an
/// empty path, a leading `/`, an empty component (`a//b`, or `a//` once one
/// `/` is stripped), or a `.` or `..` component. Raw bytes, no other change.
pub fn normalize_path(path: &[u8]) -> Result<Vec<u8>, Problem> {
    if path.contains(&0) {
        return Err(Problem::Nul);
    }
    let stripped = path.strip_suffix(b"/").unwrap_or(path);
    if stripped.is_empty() {
        return Err(Problem::EmptyPath);
    }
    if stripped.starts_with(b"/") {
        return Err(Problem::AbsolutePath);
    }
    for component in stripped.split(|byte| *byte == b'/') {
        match component {
            b"" => return Err(Problem::EmptyComponent),
            b"." | b".." => return Err(Problem::DotComponent),
            _ => {}
        }
    }
    Ok(stripped.to_vec())
}

/// `path` as a manifest line writes it, so that `parse` reads back exactly
/// `path`, with or without one trailing `/`: git's C quoting (`c_quote`),
/// plus double quotes around a path git leaves bare whose edge, one trailing
/// `/` set aside, is whitespace. Git prints a leading or trailing space bare,
/// and D-19 refuses that bare. The `/` is set aside because `parse` sets it
/// aside before its own edge check, so `a /` must be written quoted too.
pub fn encode_path(path: &[u8]) -> Vec<u8> {
    let quoted = c_quote(path);
    if quoted == path && has_edge_whitespace(path.strip_suffix(b"/").unwrap_or(path)) {
        return [&b"\""[..], path, b"\""].concat();
    }
    quoted
}

/// Whether `path` starts or ends with a byte `is_edge_whitespace` accepts.
fn has_edge_whitespace(path: &[u8]) -> bool {
    path.first().copied().is_some_and(is_edge_whitespace)
        || path.last().copied().is_some_and(is_edge_whitespace)
}

/// The whitespace D-19 refuses bare on a path's edge: space, tab, CR, VT and
/// FF (assumption A6). Not `u8::is_ascii_whitespace`, which leaves out VT
/// and takes in LF.
fn is_edge_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | 0x0b | 0x0c)
}

/// Whether the manifest path `entry` covers the tree path `path`: `path` is
/// `entry` itself or lies below it. Literal bytes, no glob.
pub fn covers(entry: &[u8], path: &[u8]) -> bool {
    path.strip_prefix(entry)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(b"/"))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::sync::atomic::{AtomicU32, Ordering};

    use proptest::prelude::*;
    use proptest::test_runner::Config;

    use super::{Entry, EntryKind, Manifest, ParseError, Problem, covers, encode_path, parse};
    use crate::git::c_quote;
    use crate::git::testing::{Scratch, all_bytes_repo, git_ok, records, write};

    /// The case count every property here runs, at least.
    const PROPERTY_CASES: u32 = 2048;

    /// The config every `proptest!` block here takes: `PROPERTY_CASES`
    /// cases, and no failure file, since the container mounts the checkout
    /// read-only. A failure prints its minimal input; it becomes a named
    /// regression test, not a persisted seed. `PROPTEST_CASES` overrides the
    /// count, so `property_runs_its_configured_cases` checks what ran.
    fn property_config() -> Config {
        Config {
            cases: PROPERTY_CASES,
            failure_persistence: None,
            ..Config::default()
        }
    }

    /// One git tree name: 1 to 11 bytes from 1 to 255 except `/`, never `.`
    /// or `..`. Weighted toward printable bytes and spaces, because a bare
    /// edge space needs every other byte of the path to be printable.
    fn component() -> impl Strategy<Value = Vec<u8>> {
        proptest::collection::vec(
            prop_oneof![2 => 1u8..=255u8, 2 => 0x21u8..=0x7eu8, 1 => Just(b' ')],
            1..12,
        )
        .prop_filter("a git tree name", |name| {
            !name.contains(&b'/') && name != b"." && name != b".."
        })
    }

    /// A git path: 1 to 3 components joined with `/`.
    fn git_path() -> impl Strategy<Value = Vec<u8>> {
        proptest::collection::vec(component(), 1..4).prop_map(|parts| parts.join(&b'/'))
    }

    /// The manifest `only <value>` followed by a newline.
    fn only_line(value: &[u8]) -> Vec<u8> {
        [&b"only "[..], value, b"\n"].concat()
    }

    /// `parse` of the one line `only <value>`, which must be refused at line
    /// 1 for `problem`.
    fn assert_refused(value: &[u8], problem: Problem) {
        let text = [&b"only "[..], value].concat();
        assert_eq!(
            parse(&only_line(value)),
            Err(ParseError {
                line: 1,
                text,
                problem,
            }),
            "only {}",
            shown(value)
        );
    }

    /// The path `parse` gives the one line `only <value>`.
    fn parsed_path(value: &[u8]) -> Vec<u8> {
        let manifest =
            parse(&only_line(value)).unwrap_or_else(|err| panic!("only {}: {err:?}", shown(value)));
        assert_eq!(manifest.entries.len(), 1, "only {}", shown(value));
        manifest.entries[0].path.clone()
    }

    proptest! {
        #![proptest_config(property_config())]

        #[test]
        fn any_git_path_round_trips(path in git_path(), trailing_slash in any::<bool>()) {
            let written = if trailing_slash {
                [&path[..], b"/"].concat()
            } else {
                path.clone()
            };
            let manifest = parse(&only_line(&encode_path(&written))).expect("an encoded path parses");
            prop_assert_eq!(manifest.entries.len(), 1);
            prop_assert_eq!(&manifest.entries[0].path, &path);
        }
    }

    #[test]
    fn property_runs_its_configured_cases() {
        let ran = AtomicU32::new(0);
        proptest!(property_config(), |(_byte in any::<u8>())| {
            ran.fetch_add(1, Ordering::Relaxed);
        });
        let ran = ran.load(Ordering::Relaxed);
        assert!(
            ran >= PROPERTY_CASES,
            "a property under property_config() ran {ran} cases, below {PROPERTY_CASES}; \
             PROPTEST_CASES lowers the count, and is {:?} here",
            std::env::var_os("PROPTEST_CASES")
        );
    }

    #[test]
    fn git_quoted_output_parses_to_git_raw_names() {
        let scratch = Scratch::new("manifest-all-bytes").expect("create scratch");
        let repo = all_bytes_repo(&scratch).unwrap_or_else(|err| panic!("{err}"));
        let ls =
            |args: &[&str]| git_ok(&scratch, &repo, args).unwrap_or_else(|err| panic!("{err}"));
        let raw = records(&ls(&["ls-files", "-z"]), 0);
        let lines = records(&ls(&["ls-files"]), b'\n');
        assert_eq!(
            raw.len(),
            lines.len(),
            "git ls-files and -z list different counts"
        );
        let mut refused = Vec::new();
        for (name, line) in raw.iter().zip(&lines) {
            match parse(&only_line(line)) {
                Ok(manifest) => assert_eq!(
                    manifest
                        .entries
                        .iter()
                        .map(|entry| &entry.path)
                        .collect::<Vec<_>>(),
                    vec![name],
                    "only {}",
                    shown(line)
                ),
                Err(ParseError {
                    problem: Problem::EdgeWhitespace,
                    ..
                }) => refused.push(name.clone()),
                Err(err) => panic!("only {}: {err:?}", shown(line)),
            }
        }
        refused.sort();
        assert_eq!(refused, vec![b" lead".to_vec(), b"trail ".to_vec()]);
    }

    #[test]
    fn encoded_edge_spaces_are_quoted() {
        assert_eq!(encode_path(b" a "), b"\" a \"".to_vec());
        assert_eq!(encode_path(b"plain"), b"plain".to_vec());
    }

    #[test]
    fn encoded_space_before_a_trailing_slash_round_trips() {
        assert_eq!(encode_path(b"a /"), b"\"a /\"".to_vec());
        assert_eq!(
            parse(&only_line(b"\"a /\"")),
            Ok(Manifest {
                entries: vec![Entry {
                    kind: EntryKind::Only,
                    path: b"a ".to_vec(),
                    line: 1,
                    text: b"only \"a /\"".to_vec(),
                }],
            })
        );
        // Control: git leaves `a /` bare, and bare it is edge whitespace, so
        // an encoder that checked the byte before the `/` was removed would
        // hand back a line `parse` refuses.
        assert_eq!(c_quote(b"a /"), b"a /".to_vec());
        assert_refused(b"a /", Problem::EdgeWhitespace);
    }

    #[test]
    fn space_after_the_keyword_starts_the_path() {
        assert_eq!(parsed_path(b"my dir"), b"my dir".to_vec());
    }

    #[test]
    fn trailing_slash_is_stripped_from_a_quoted_path() {
        assert_eq!(parsed_path(b"\"a/\""), b"a".to_vec());
    }

    #[test]
    fn text_after_a_quoted_path_is_bad_quoting() {
        assert_refused(
            b"\"a\"b",
            Problem::BadQuoting {
                reason: "text after the closing quote".to_owned(),
            },
        );
    }

    #[test]
    fn leading_tab_is_edge_whitespace() {
        assert_refused(b"\ta", Problem::EdgeWhitespace);
    }

    #[test]
    fn trailing_tab_is_edge_whitespace() {
        assert_refused(b"a\t", Problem::EdgeWhitespace);
    }

    #[test]
    fn leading_vertical_tab_is_edge_whitespace() {
        assert_refused(b"\x0ba", Problem::EdgeWhitespace);
    }

    #[test]
    fn trailing_vertical_tab_is_edge_whitespace() {
        assert_refused(b"a\x0b", Problem::EdgeWhitespace);
    }

    #[test]
    fn leading_form_feed_is_edge_whitespace() {
        assert_refused(b"\x0ca", Problem::EdgeWhitespace);
    }

    #[test]
    fn trailing_form_feed_is_edge_whitespace() {
        assert_refused(b"a\x0c", Problem::EdgeWhitespace);
    }

    #[test]
    fn leading_carriage_return_is_edge_whitespace() {
        assert_refused(b"\ra", Problem::EdgeWhitespace);
    }

    #[test]
    fn carriage_return_before_the_trailing_slash_is_edge_whitespace() {
        assert_refused(b"a\r/", Problem::EdgeWhitespace);
    }

    #[test]
    fn control_byte_on_the_edge_parses_bare() {
        assert_eq!(parsed_path(b"\x01a\x01"), b"\x01a\x01".to_vec());
    }

    #[test]
    fn only_line_keeps_its_number_and_text() {
        assert_eq!(
            parse(b"# c\n\nonly a/\nmodified b\n"),
            Ok(Manifest {
                entries: vec![
                    Entry {
                        kind: EntryKind::Only,
                        path: b"a".to_vec(),
                        line: 3,
                        text: b"only a/".to_vec(),
                    },
                    Entry {
                        kind: EntryKind::Modified,
                        path: b"b".to_vec(),
                        line: 4,
                        text: b"modified b".to_vec(),
                    },
                ],
            })
        );
    }

    #[test]
    fn escape_bytes_in_a_line_render_quoted() {
        let err = ParseError {
            line: 1,
            text: b"sync \x1b[2J".to_vec(),
            problem: Problem::UnknownKind,
        };
        let rendered = err.to_string();
        assert!(
            !rendered.contains('\x1b'),
            "a raw ESC byte reached the message: {rendered:?}"
        );
        assert_eq!(
            rendered,
            r#"line 1: expected a comment or an "only <path>" or "modified <path>" line: "sync \033[2J""#
        );
    }

    #[test]
    fn unknown_kind_names_the_line() {
        assert_eq!(
            parse(b"only a\nsync b\n"),
            Err(ParseError {
                line: 2,
                text: b"sync b".to_vec(),
                problem: Problem::UnknownKind,
            })
        );
    }

    /// `bytes` as git's C-quoting prints them, for a failure message: every
    /// byte readable, and no gix type in the model.
    fn shown(bytes: &[u8]) -> String {
        String::from_utf8_lossy(&c_quote(bytes)).into_owned()
    }

    /// The line and problem `parse` refuses `bytes` with; panics on `Ok`.
    fn refusal(bytes: &[u8]) -> (usize, Problem) {
        match parse(bytes) {
            Ok(manifest) => panic!("{} parsed: {manifest:?}", shown(bytes)),
            Err(err) => (err.line, err.problem),
        }
    }

    /// The paths of the entries `parse` gives `bytes`, in order; panics on
    /// `Err`.
    fn paths(bytes: &[u8]) -> Vec<Vec<u8>> {
        parse(bytes)
            .unwrap_or_else(|err| panic!("{}: {err:?}", shown(bytes)))
            .entries
            .into_iter()
            .map(|entry| entry.path)
            .collect()
    }

    #[test]
    fn leading_space_is_an_unknown_line() {
        assert_eq!(refusal(b" only a\n"), (1, Problem::UnknownKind));
    }

    #[test]
    fn tab_after_the_keyword_is_an_unknown_line() {
        assert_eq!(refusal(b"only\ta\n"), (1, Problem::UnknownKind));
    }

    #[test]
    fn whitespace_only_line_is_an_unknown_line() {
        assert_eq!(refusal(b"  \n"), (1, Problem::UnknownKind));
    }

    #[test]
    fn keyword_with_a_suffix_is_an_unknown_line() {
        assert_eq!(refusal(b"onlyx a\n"), (1, Problem::UnknownKind));
    }

    #[test]
    fn line_ending_in_cr_is_refused() {
        assert_eq!(refusal(b"only a\r\n"), (1, Problem::LineEndsInCr));
    }

    #[test]
    fn comment_ending_in_cr_is_refused() {
        assert_eq!(refusal(b"# c\r\n"), (1, Problem::LineEndsInCr));
    }

    #[test]
    fn crlf_file_fails_on_its_first_line() {
        assert_eq!(
            refusal(b"only a\r\nmodified b\r\n"),
            (1, Problem::LineEndsInCr)
        );
    }

    #[test]
    fn keyword_alone_is_an_empty_path() {
        assert_eq!(refusal(b"only\n"), (1, Problem::EmptyPath));
    }

    #[test]
    fn keyword_and_space_is_an_empty_path() {
        assert_eq!(refusal(b"only \n"), (1, Problem::EmptyPath));
    }

    #[test]
    fn slash_alone_is_an_empty_path() {
        assert_eq!(refusal(b"only /\n"), (1, Problem::EmptyPath));
    }

    #[test]
    fn empty_quoted_path_is_an_empty_path() {
        assert_eq!(refusal(b"only \"\"\n"), (1, Problem::EmptyPath));
    }

    #[test]
    fn two_spaces_after_the_keyword_are_edge_whitespace() {
        assert_eq!(refusal(b"only  a\n"), (1, Problem::EdgeWhitespace));
    }

    #[test]
    fn trailing_space_is_edge_whitespace() {
        assert_eq!(refusal(b"only a \n"), (1, Problem::EdgeWhitespace));
    }

    #[test]
    fn space_before_the_trailing_slash_is_edge_whitespace() {
        assert_eq!(refusal(b"only a /\n"), (1, Problem::EdgeWhitespace));
    }

    #[test]
    fn leading_slash_is_refused() {
        assert_eq!(refusal(b"only /a\n"), (1, Problem::AbsolutePath));
    }

    #[test]
    fn double_slash_is_refused() {
        assert_eq!(refusal(b"only a//b\n"), (1, Problem::EmptyComponent));
    }

    #[test]
    fn trailing_double_slash_is_refused() {
        assert_eq!(refusal(b"only a//\n"), (1, Problem::EmptyComponent));
    }

    #[test]
    fn lone_dot_is_refused() {
        assert_eq!(refusal(b"only .\n"), (1, Problem::DotComponent));
    }

    #[test]
    fn dot_component_is_refused() {
        assert_eq!(refusal(b"only a/./b\n"), (1, Problem::DotComponent));
    }

    #[test]
    fn dot_dot_component_is_refused() {
        assert_eq!(refusal(b"only a/../b\n"), (1, Problem::DotComponent));
    }

    #[test]
    fn quoted_nul_is_refused() {
        assert_eq!(refusal(b"only \"a\\000b\"\n"), (1, Problem::Nul));
    }

    #[test]
    fn raw_nul_is_refused() {
        assert_eq!(refusal(b"only a\x00b\n"), (1, Problem::Nul));
    }

    #[test]
    fn modified_line_follows_the_same_rules() {
        assert_eq!(refusal(b"modified a/../b\n"), (1, Problem::DotComponent));
    }

    #[test]
    fn inner_spaces_stay_in_the_path() {
        assert_eq!(paths(b"only my dir\n"), vec![b"my dir".to_vec()]);
    }

    #[test]
    fn one_trailing_slash_is_stripped() {
        assert_eq!(paths(b"only a/\n"), vec![b"a".to_vec()]);
    }

    #[test]
    fn star_is_an_ordinary_byte() {
        assert_eq!(paths(b"only a*b\n"), vec![b"a*b".to_vec()]);
    }

    #[test]
    fn hash_after_the_keyword_is_part_of_the_path() {
        assert_eq!(paths(b"only #x\n"), vec![b"#x".to_vec()]);
    }

    #[test]
    fn trailing_text_is_part_of_the_path() {
        assert_eq!(paths(b"only a # note\n"), vec![b"a # note".to_vec()]);
    }

    #[test]
    fn quote_inside_a_bare_path_is_literal() {
        assert_eq!(paths(b"only a\"b\n"), vec![b"a\"b".to_vec()]);
    }

    #[test]
    fn quoted_edge_spaces_parse() {
        assert_eq!(paths(b"only \" a \"\n"), vec![b" a ".to_vec()]);
    }

    #[test]
    fn raw_utf8_parses_unquoted() {
        assert_eq!(
            paths("only café\n".as_bytes()),
            vec![b"caf\xc3\xa9".to_vec()]
        );
    }

    #[test]
    fn raw_non_utf8_parses_unquoted() {
        assert_eq!(paths(b"only raw\xe9\n"), vec![b"raw\xe9".to_vec()]);
    }

    #[test]
    fn control_byte_parses_unquoted() {
        assert_eq!(paths(b"only a\x01b\n"), vec![b"a\x01b".to_vec()]);
    }

    #[test]
    fn quoted_octal_gives_the_utf8_bytes() {
        assert_eq!(
            paths(b"only \"caf\\303\\251\"\n"),
            vec![b"caf\xc3\xa9".to_vec()]
        );
    }

    #[test]
    fn no_trailing_newline_parses() {
        assert_eq!(
            paths(b"only a\nmodified b"),
            vec![b"a".to_vec(), b"b".to_vec()]
        );
    }

    #[test]
    fn empty_manifest_is_valid() {
        assert_eq!(parse(b""), Ok(Manifest { entries: vec![] }));
    }

    #[test]
    fn comments_only_manifest_is_valid() {
        assert_eq!(parse(b"# a\n#\n# b\n"), Ok(Manifest { entries: vec![] }));
    }

    #[test]
    fn blank_lines_are_ignored() {
        assert_eq!(
            paths(b"\n\nonly a\n\n\nmodified b\n\n"),
            vec![b"a".to_vec(), b"b".to_vec()]
        );
    }

    #[test]
    fn single_line_manifest_has_one_entry() {
        assert_eq!(
            parse(b"modified a\n"),
            Ok(Manifest {
                entries: vec![Entry {
                    kind: EntryKind::Modified,
                    path: b"a".to_vec(),
                    line: 1,
                    text: b"modified a".to_vec(),
                }],
            })
        );
    }

    #[test]
    fn line_numbers_count_comments_and_blanks() {
        let lines: Vec<usize> = parse(b"# c\n\nonly a\n# d\n\n\nmodified b\nonly c\n")
            .expect("parses")
            .entries
            .iter()
            .map(|entry| entry.line)
            .collect();
        assert_eq!(lines, vec![3, 7, 8]);
    }

    /// `parse` of `bytes` must be `Problem::Overlap` at `line` (`text`)
    /// naming `other_line` (`other_text`), and render exactly `rendered`.
    fn assert_overlap(
        bytes: &[u8],
        (line, text): (usize, &[u8]),
        (other_line, other_text): (usize, &[u8]),
        rendered: &str,
    ) {
        let expected = ParseError {
            line,
            text: text.to_vec(),
            problem: Problem::Overlap {
                other_line,
                other_text: other_text.to_vec(),
            },
        };
        assert_eq!(parse(bytes), Err(expected.clone()));
        assert_eq!(expected.to_string(), rendered);
    }

    #[test]
    fn only_with_modified_of_the_same_path_overlaps() {
        assert_overlap(
            b"only a\nmodified a\n",
            (2, b"modified a"),
            (1, b"only a"),
            "line 2: this line overlaps line 1: only a; a path cannot be both only and modified: modified a",
        );
    }

    #[test]
    fn only_with_modified_under_it_overlaps() {
        assert_overlap(
            b"only a\nmodified a/b\n",
            (2, b"modified a/b"),
            (1, b"only a"),
            "line 2: this line overlaps line 1: only a; a path cannot be both only and modified: modified a/b",
        );
    }

    #[test]
    fn modified_under_a_later_only_overlaps() {
        assert_overlap(
            b"modified a/b\nonly a\n",
            (2, b"only a"),
            (1, b"modified a/b"),
            "line 2: this line overlaps line 1: modified a/b; a path cannot be both only and modified: only a",
        );
    }

    #[test]
    fn only_with_a_trailing_slash_overlaps_modified() {
        assert_overlap(
            b"only a/\nmodified a\n",
            (2, b"modified a"),
            (1, b"only a/"),
            "line 2: this line overlaps line 1: only a/; a path cannot be both only and modified: modified a",
        );
    }

    #[test]
    fn modified_with_only_under_it_overlaps() {
        assert_overlap(
            b"modified a\nonly a/b\n",
            (2, b"only a/b"),
            (1, b"modified a"),
            "line 2: this line overlaps line 1: modified a; a path cannot be both only and modified: only a/b",
        );
    }

    #[test]
    fn only_under_a_later_modified_overlaps() {
        assert_overlap(
            b"# c\nonly a/b\n\nmodified a\n",
            (4, b"modified a"),
            (2, b"only a/b"),
            "line 4: this line overlaps line 2: only a/b; a path cannot be both only and modified: modified a",
        );
    }

    #[test]
    fn exact_duplicates_of_one_kind_parse() {
        assert_eq!(
            paths(b"only a\nonly a\nmodified b\nmodified b\n"),
            vec![b"a".to_vec(), b"a".to_vec(), b"b".to_vec(), b"b".to_vec()]
        );
    }

    #[test]
    fn nested_lines_of_one_kind_parse() {
        assert_eq!(
            paths(b"only a\nonly a/b\nmodified c/d\nmodified c\n"),
            vec![
                b"a".to_vec(),
                b"a/b".to_vec(),
                b"c/d".to_vec(),
                b"c".to_vec()
            ]
        );
    }

    #[test]
    fn name_extension_across_kinds_parses() {
        assert_eq!(
            paths(b"only a\nmodified ab\n"),
            vec![b"a".to_vec(), b"ab".to_vec()]
        );
    }

    /// 4,096 `only` lines of exactly 16 bytes, `only p`, nine digits and a
    /// newline, every path distinct: 65,536 bytes. With `over`, the last
    /// path has ten digits, so the manifest is 65,537 bytes and its 65,537th
    /// byte is line 4,096's newline.
    fn sized_manifest(over: bool) -> Vec<u8> {
        const LINES: usize = 4096;
        let mut bytes = Vec::new();
        for index in 0..LINES {
            let line = if over && index == LINES - 1 {
                format!("only p{index:010}\n")
            } else {
                format!("only p{index:09}\n")
            };
            bytes.extend_from_slice(line.as_bytes());
        }
        bytes
    }

    #[test]
    fn manifest_of_exactly_64_kib_parses() {
        let bytes = sized_manifest(false);
        assert_eq!(bytes.len(), 65_536);
        let manifest = parse(&bytes).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(manifest.entries.len(), 4096);
    }

    #[test]
    fn manifest_over_64_kib_is_refused() {
        let bytes = sized_manifest(true);
        assert_eq!(bytes.len(), 65_537);
        let err = parse(&bytes).expect_err("a manifest over 64 KiB parsed");
        assert_eq!(
            err,
            ParseError {
                line: 4096,
                text: vec![],
                problem: Problem::TooLarge { size: 65_537 },
            }
        );
        assert_eq!(
            err.to_string(),
            "line 4096: the manifest is 65537 bytes, over the limit of 65536 bytes (64 KiB)"
        );
    }

    #[test]
    fn size_limit_is_checked_before_any_line() {
        let mut bytes = sized_manifest(true);
        bytes.splice(0..16, b"sync a000000000\n".iter().copied());
        assert_eq!(bytes.len(), 65_537);
        assert_eq!(refusal(&bytes), (4096, Problem::TooLarge { size: 65_537 }));
    }

    #[test]
    fn declaration_covered_by_a_built_in_parses() {
        assert_eq!(
            parse(b"only .wsx.conf\nonly .worktrees/x\n"),
            Ok(Manifest {
                entries: vec![
                    Entry {
                        kind: EntryKind::Only,
                        path: b".wsx.conf".to_vec(),
                        line: 1,
                        text: b"only .wsx.conf".to_vec(),
                    },
                    Entry {
                        kind: EntryKind::Only,
                        path: b".worktrees/x".to_vec(),
                        line: 2,
                        text: b"only .worktrees/x".to_vec(),
                    },
                ],
            })
        );
    }

    /// The shape of `.workspace-divergence` on the workspace branch, copied
    /// inline: that file exists only there, not on the PR branches this
    /// code reaches.
    const REAL_MANIFEST: &[u8] = b"\
# How workspace/denniyahh differs from develop. Read by
# scripts/workspace/cut-pr-branch.sh, which never replays an `only` path.
# Ported from devflow (Workflow B). The divergence check hook is not ported yet.
#
# `only <path>`: exists only on this branch (a directory covers everything in it).
# `modified <path>`: a develop file this branch deliberately changes.
# .planning/, graphify-out/, agent-tool paths and this file are built in
# and need no line.

# Workflow B tooling (sync, phase worktrees, clean PR branches)
only scripts/workspace

# Ignores .worktrees/ and graphify/research caches
modified .gitignore
";

    #[test]
    fn parses_the_shape_of_a_real_manifest() {
        assert_eq!(
            parse(REAL_MANIFEST),
            Ok(Manifest {
                entries: vec![
                    Entry {
                        kind: EntryKind::Only,
                        path: b"scripts/workspace".to_vec(),
                        line: 11,
                        text: b"only scripts/workspace".to_vec(),
                    },
                    Entry {
                        kind: EntryKind::Modified,
                        path: b".gitignore".to_vec(),
                        line: 14,
                        text: b"modified .gitignore".to_vec(),
                    },
                ],
            })
        );
    }

    #[test]
    fn path_covers_itself() {
        assert!(covers(b".planning", b".planning"));
    }

    #[test]
    fn path_covers_what_is_under_it() {
        assert!(covers(b".planning", b".planning/x"));
        assert!(covers(b".planning", b".planning/x/y"));
    }

    #[test]
    fn shorter_name_is_not_covered() {
        assert!(!covers(b".planning", b".plannin"));
        assert!(!covers(b".planning", b".planningX"));
    }

    proptest! {
        #![proptest_config(property_config())]

        #[test]
        fn name_extension_is_not_covered(
            path in git_path(),
            byte in any::<u8>().prop_filter("not a slash", |byte| *byte != b'/'),
            below in git_path(),
        ) {
            let extended = [&path[..], &[byte]].concat();
            prop_assert!(!covers(&path, &extended));
            let under = [&path[..], b"/", &below[..]].concat();
            prop_assert!(covers(&path, &under));
        }
    }

    #[test]
    fn covers_matches_a_literal_pathspec() {
        let scratch = Scratch::new("manifest-pathspec").expect("create scratch");
        let repo = scratch.root.join("repo");
        let tracked_names: [&[u8]; 5] =
            [b".planning/x", b".planningX", b".plan", b"a*b/c", b"axb/c"];
        for name in tracked_names {
            write(&repo.join(OsStr::from_bytes(name)), b"").unwrap_or_else(|err| panic!("{err}"));
        }
        let git =
            |args: &[&OsStr]| git_ok(&scratch, &repo, args).unwrap_or_else(|err| panic!("{err}"));
        git(&[
            OsStr::new("init"),
            OsStr::new("-q"),
            OsStr::new("-b"),
            OsStr::new("main"),
        ]);
        git(&[OsStr::new("add"), OsStr::new("--all")]);
        let tracked = records(&git(&[OsStr::new("ls-files"), OsStr::new("-z")]), 0);
        let mut sorted_names: Vec<Vec<u8>> =
            tracked_names.iter().map(|name| name.to_vec()).collect();
        sorted_names.sort();
        assert_eq!(tracked, sorted_names, "the repository tracks other paths");
        let expected: [(&[u8], Vec<&[u8]>); 3] = [
            (b".planning", vec![b".planning/x"]),
            (b".plan", vec![b".plan"]),
            (b"a*b", vec![b"a*b/c"]),
        ];
        for (entry, selects) in expected {
            let pathspec = [&b":(literal)"[..], entry].concat();
            let from_git = records(
                &git(&[
                    OsStr::new("ls-files"),
                    OsStr::new("-z"),
                    OsStr::new("--"),
                    OsStr::from_bytes(&pathspec),
                ]),
                0,
            );
            let from_covers: Vec<Vec<u8>> = tracked
                .iter()
                .filter(|path| covers(entry, path))
                .cloned()
                .collect();
            assert_eq!(from_covers, from_git, "entry {}", shown(entry));
            let selects: Vec<Vec<u8>> = selects.iter().map(|path| path.to_vec()).collect();
            assert_eq!(from_git, selects, "git for entry {}", shown(entry));
        }
    }

    #[test]
    fn byte_equality_separates_unicode_normal_forms() {
        let nfc: &[u8] = b"caf\xc3\xa9";
        let nfd: &[u8] = b"cafe\xcc\x81";
        assert!(!covers(nfc, nfd));
        assert!(!covers(nfd, nfc));
        assert!(covers(nfc, nfc));
        assert!(covers(nfd, nfd));
    }
}
