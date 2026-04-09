use thiserror::Error;

/// Type de résultat standard de LogBog.
pub type Result<T> = std::result::Result<T, Error>;

/// Erreurs LogBog.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Configuration file not found: {0}")]
    ConfigNotFound(String),

    #[error("Pack not found: {0}")]
    PackNotFound(String),

    #[error("Pack already installed: {0}")]
    PackAlreadyInstalled(String),

    #[error("Pack validation failed: {0}")]
    PackInvalid(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Collector error: {0}")]
    Collector(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Service not running")]
    NotRunning,

    #[error("Service already running")]
    AlreadyRunning,

    #[error("{0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::PackNotFound("nginx".into());
        assert_eq!(err.to_string(), "Pack not found: nginx");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }
}
