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
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::UnknownKind => {
                f.write_str(r#"expected a comment or an "only <path>" or "modified <path>" line"#)
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
            String::from_utf8_lossy(&self.text)
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

/// Whether the manifest path `entry` covers the tree path `path`: `path` is
/// `entry` itself or lies below it. Literal bytes, no glob.
pub fn covers(entry: &[u8], path: &[u8]) -> bool {
    path.strip_prefix(entry)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(b"/"))
}

#[cfg(test)]
mod tests {
    use super::{Entry, EntryKind, Manifest, ParseError, Problem, parse};

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
