use std::fmt;

/// Unified error type for FerrumDB.
#[derive(Debug)]
pub enum FerrumError {
    Io(std::io::Error),
    Bincode(bincode::Error),
    Corruption(String),
    InvalidCommand(String),
    InvalidConfig(String),
    MissingArgument(&'static str),
}

impl fmt::Display for FerrumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Bincode(e) => write!(f, "Bincode error: {e}"),
            Self::Corruption(e) => write!(f, "Data corruption: {e}"),
            Self::InvalidCommand(cmd) => write!(f, "Unknown command: {cmd}"),
            Self::InvalidConfig(msg) => write!(f, "Invalid config: {msg}"),
            Self::MissingArgument(arg) => write!(f, "Missing argument: {arg}"),
        }
    }
}

impl std::error::Error for FerrumError {}
