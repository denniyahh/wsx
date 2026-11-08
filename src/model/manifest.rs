//! The `.wsx.conf` grammar: which paths a workspace branch keeps personal.
//!
//! The parser keeps every line it accepts, in file order, built-ins and
//! duplicates included, with its line number and raw text, so a later check
//! can name the line a problem came from (D-13).

use std::fmt;

use crate::git::{c_quote, c_unquote};

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
        }
    }
}

/// A refused manifest: the first line that failed and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The line's number, from 1.
    pub line: usize,
    /// The line's raw bytes, without its newline.
    pub text: Vec<u8>,
    /// What is wrong with it.
    pub problem: Problem,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}: {}: {}",
            self.line,
            self.problem,
            String::from_utf8_lossy(&c_quote(&self.text))
        )
    }
}

/// Parses a manifest blob: lines split on `\n` and numbered from 1, each
/// read by `read_line`. The first line it refuses refuses the whole
/// manifest, naming that line.
pub fn parse(bytes: &[u8]) -> Result<Manifest, ParseError> {
    let mut entries = Vec::new();
    for (index, text) in bytes.split(|byte| *byte == b'\n').enumerate() {
        let line = index + 1;
        let read = read_line(text).map_err(|problem| ParseError {
            line,
            text: text.to_vec(),
            problem,
        })?;
        if let Some((kind, path)) = read {
            entries.push(Entry {
                kind,
                path,
                line,
                text: text.to_vec(),
            });
        }
    }
    Ok(Manifest { entries })
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
    use std::sync::atomic::{AtomicU32, Ordering};

    use gix::bstr::ByteSlice;
    use proptest::prelude::*;
    use proptest::test_runner::Config;

    use super::{Entry, EntryKind, Manifest, ParseError, Problem, encode_path, parse};
    use crate::git::c_quote;
    use crate::git::testing::{Scratch, all_bytes_repo, git_ok, records};

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
            "only {:?}",
            value.as_bstr()
        );
    }

    /// The path `parse` gives the one line `only <value>`.
    fn parsed_path(value: &[u8]) -> Vec<u8> {
        let manifest = parse(&only_line(value))
            .unwrap_or_else(|err| panic!("only {:?}: {err:?}", value.as_bstr()));
        assert_eq!(manifest.entries.len(), 1, "only {:?}", value.as_bstr());
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
                    "only {:?}",
                    line.as_bstr()
                ),
                Err(ParseError {
                    problem: Problem::EdgeWhitespace,
                    ..
                }) => refused.push(name.clone()),
                Err(err) => panic!("only {:?}: {err:?}", line.as_bstr()),
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

    /// The line and problem `parse` refuses `bytes` with; panics on `Ok`.
    fn refusal(bytes: &[u8]) -> (usize, Problem) {
        match parse(bytes) {
            Ok(manifest) => panic!("{:?} parsed: {manifest:?}", bytes.as_bstr()),
            Err(err) => (err.line, err.problem),
        }
    }

    /// The paths of the entries `parse` gives `bytes`, in order; panics on
    /// `Err`.
    fn paths(bytes: &[u8]) -> Vec<Vec<u8>> {
        parse(bytes)
            .unwrap_or_else(|err| panic!("{:?}: {err:?}", bytes.as_bstr()))
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
}
