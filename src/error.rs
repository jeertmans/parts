//! Error and Result structure used all across this crate.

/// Enumeration of all possible error types.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    /// Error from the command line parsing (see [clap::Error]).
    Cli(#[from] clap::Error),
    /// Error from reading and writing to IO (see [std::io::Error]).
    #[error(transparent)]
    IO(#[from] std::io::Error),
    /// Error from deserializing TOML (see [toml::de::Error]).
    #[error(transparent)]
    TomlDecode(#[from] toml::de::Error),
    /// Error accessing a key in a TOML document.
    #[error("TOML file {path:?} does not contain keys {keys:?}")]
    KeysNotFound { keys: String, path: String },
    /// Error parsing a TOML value in a table.
    #[error("TOML file {path:?} does not contain (nested) tables as expected")]
    ValueIsNotTable { path: String },
    /// Error parsing a TOML value in a table.
    #[error("no TOML config file was found, use verbose output (`-v`) for more details")]
    NoConfigFileFound,
    /// Specified config file value is invalid.
    #[error("user-defined TOML config file value {value:?} does not exist")]
    ConfigFileDoesNotExist { value: String },
    #[error("unknown part name: {part:?}")]
    UnknownPart { part: String },
}

/// Result type alias with error type defined above (see [Error]).
pub type Result<T> = std::result::Result<T, Error>;
