//! A config or theme problem, with the line a person can open.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub file: String,
    pub line: usize,
    pub message: String,
}

impl Problem {
    pub fn new(file: impl Into<String>, line: usize, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.message)
    }
}

/// Turns a byte offset in `source` into a 1-based line number.
pub fn line_at(source: &str, offset: usize) -> usize {
    source
        .as_bytes()
        .iter()
        .take(offset.min(source.len()))
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

/// The line a KDL node starts on.
pub fn node_line(source: &str, node: &kdl::KdlNode) -> usize {
    line_at(source, node.span().offset())
}
