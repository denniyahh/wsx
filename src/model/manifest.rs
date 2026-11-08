//! The `.wsx.conf` grammar: which paths a workspace branch keeps personal.
//!
//! The parser keeps every line it accepts, in file order, built-ins and
//! duplicates included, with its line number and raw text, so a later check
//! can name the line a problem came from (D-13).

use std::fmt;

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
            String::from_utf8_lossy(&crate::git::c_quote(&self.text))
        )
    }
}

/// Parses a manifest blob: lines split on `\n` and numbered from 1, an empty
/// line or one starting with `#` skipped (D-11), an `only ` or `modified `
/// line kept with one trailing `/` stripped from its path (D-14). Any other
/// line refuses the whole manifest, naming the line.
pub fn parse(bytes: &[u8]) -> Result<Manifest, ParseError> {
    let mut entries = Vec::new();
    for (index, text) in bytes.split(|byte| *byte == b'\n').enumerate() {
        let line = index + 1;
        if text.is_empty() || text.starts_with(b"#") {
            continue;
        }
        let (kind, rest) = if let Some(rest) = text.strip_prefix(b"only ") {
            (EntryKind::Only, rest)
        } else if let Some(rest) = text.strip_prefix(b"modified ") {
            (EntryKind::Modified, rest)
        } else {
            return Err(ParseError {
                line,
                text: text.to_vec(),
                problem: Problem::UnknownKind,
            });
        };
        let path = rest.strip_suffix(b"/").unwrap_or(rest);
        entries.push(Entry {
            kind,
            path: path.to_vec(),
            line,
            text: text.to_vec(),
        });
    }
    Ok(Manifest { entries })
}

/// `path` as a manifest line writes it, so that `parse` reads back exactly
/// `path`. Not yet implemented: returns `path` unchanged.
pub fn encode_path(path: &[u8]) -> Vec<u8> {
    path.to_vec()
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
}
