//! Actionable diagnostics that retain source cell context.

use std::fmt;

/// A user-facing failure, optionally associated with a source cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Description of the problem and how to fix it.
    pub message: String,
    /// Physical source line, including the header as line 1.
    pub row: Option<usize>,
    /// Source column label.
    pub column: Option<String>,
    /// Rejected value, when relevant.
    pub value: Option<String>,
}

/// The result type used throughout the conversion library.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Construct a configuration, I/O, or table-level diagnostic.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            row: None,
            column: None,
            value: None,
        }
    }

    /// Construct a diagnostic pointing to a particular input cell.
    pub fn cell(row: usize, column: &str, value: &str, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            row: Some(row),
            column: Some(column.into()),
            value: Some(value.into()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(row) = self.row {
            write!(f, "row {row}: ")?;
        }
        if let Some(column) = &self.column {
            write!(f, "column {column:?}: ")?;
        }
        if let Some(value) = &self.value {
            write!(f, "value {value:?}: ")?;
        }
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::new(format!("I/O error: {error}"))
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::new(format!("Invalid configuration JSON: {error}"))
    }
}
