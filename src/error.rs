use std::fmt;

/// Unified error type for FerrumDB.
#[derive(Debug)]
pub enum FerrumError {
    Io(std::io::Error),
    Serialize(serde_json::Error),
    InvalidCommand(String),
    MissingArgument(&'static str),
}

impl fmt::Display for FerrumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Serialize(e) => write!(f, "Serialization error: {e}"),
            Self::InvalidCommand(cmd) => write!(f, "Unknown command: {cmd}"),
            Self::MissingArgument(arg) => write!(f, "Missing argument: {arg}"),
        }
    }
}

impl std::error::Error for FerrumError {}
