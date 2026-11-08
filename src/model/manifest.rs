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
            Problem::UnknownKind => f.write_str(""),
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
        write!(f, "line {}: {}", self.line, self.problem)
    }
}

/// Parses a manifest blob.
pub fn parse(_bytes: &[u8]) -> Result<Manifest, ParseError> {
    Ok(Manifest {
        entries: Vec::new(),
    })
}

/// Whether the manifest path `entry` covers the tree path `path`.
pub fn covers(_entry: &[u8], _path: &[u8]) -> bool {
    false
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
