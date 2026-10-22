//! Database errors.

use std::error::Error as StdError;
use std::fmt;

/// Result type for flux-db operations.
pub type Result<T> = std::result::Result<T, Error>;

/// A database error.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Query rejected before it reaches the database
    /// (empty insert, unfiltered write, bad identifier, too many params...).
    InvalidQuery(String),
    /// Value type mismatch during conversion (e.g. [`FromValue`](crate::value::FromValue)).
    Conversion { expected: &'static str, received: &'static str },
    /// Row decode failed (missing column, NULL into non-Option, unknown DB type...).
    Decode { column: String, message: String },
    /// `fetch_one` / `find` returned no rows.
    NotFound,
    /// Anything from the driver, passed through untouched.
    Driver(Box<dyn StdError + Send + Sync>),
}

impl Error {
    /// Creates an [`InvalidQuery`](Error::InvalidQuery) error.
    pub fn invalid_query(message: impl Into<String>) -> Self {
        Self::InvalidQuery(message.into())
    }

    /// Creates a [`Conversion`](Error::Conversion) error.
    pub fn conversion(expected: &'static str, received: &'static str) -> Self {
        Self::Conversion { expected, received }
    }

    /// Creates a [`Decode`](Error::Decode) error for a named column.
    pub fn decode(column: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Decode {
            column: column.into(),
            message: message.into(),
        }
    }

    /// Wraps a driver / pool error.
    pub fn driver(err: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::Driver(err.into())
    }

    /// Returns `true` if this is [`NotFound`](Error::NotFound).
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuery(message) => write!(f, "invalid query: {message}"),
            Self::Conversion { expected, received } => {
                write!(f, "type conversion failed: expected `{expected}`, received `{received}`")
            }
            Self::Decode { column, message } => {
                write!(f, "decode error in column `{column}`: {message}")
            }
            Self::NotFound => write!(f, "record not found"),
            Self::Driver(err) => write!(f, "driver error: {err}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Driver(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl From<Box<dyn StdError + Send + Sync>> for Error {
    fn from(err: Box<dyn StdError + Send + Sync>) -> Self {
        Self::Driver(err)
    }
}
