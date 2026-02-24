use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct Location {
    pub file: &'static str,
    pub line: u32,
}

impl Location {
    pub fn new(file: &'static str, line: u32) -> Self {
        Self { file, line }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.file.is_empty() { write!(formatter, "{}:{}: ", self.file, self.line) } else { Ok(()) }
    }
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{location}none of [{keys}] found in environment")]
    NotFound { keys: String, location: Location },

    #[error("{location}{key}: expected {expected}, got '{got}': {source}")]
    Parse {
        key: String,
        expected: &'static str,
        got: String,
        source: Box<dyn std::error::Error + Send + Sync>,
        location: Location,
    },

    #[error("failed to load dotenv from {path}: {source}")]
    DotenvLoad { path: PathBuf, source: std::io::Error },

    #[error("{path}:{line}: {message}")]
    DotenvParse { path: PathBuf, line: usize, message: String },
}

impl Error {
    pub fn with_location(self, file: &'static str, line: u32) -> Self {
        let location = Location { file, line };
        match self {
            Self::NotFound { keys, .. } => Self::NotFound { keys, location },
            Self::Parse { key, expected, got, source, .. } => Self::Parse { key, expected, got, source, location },
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_without_location() {
        let err = Error::NotFound {
            keys: "FOO, BAR".into(),
            location: Location::default(),
        };
        assert_eq!(err.to_string(), "none of [FOO, BAR] found in environment");
    }

    #[test]
    fn not_found_with_location() {
        let err = Error::NotFound {
            keys: "PORT".into(),
            location: Location::default(),
        }
        .with_location("src/config.rs", 42);
        let msg = err.to_string();
        assert!(msg.starts_with("src/config.rs:42: "));
        assert!(msg.contains("PORT"));
    }

    #[test]
    fn parse_with_location() {
        let err = Error::Parse {
            key: "PORT".into(),
            expected: "u16",
            got: "banana".into(),
            source: "invalid digit found in string".into(),
            location: Location::default(),
        }
        .with_location("src/main.rs", 10);
        let msg = err.to_string();
        assert!(msg.starts_with("src/main.rs:10: "));
        assert!(msg.contains("PORT"));
        assert!(msg.contains("u16"));
        assert!(msg.contains("banana"));
    }

    #[test]
    fn parse_without_location() {
        let err = Error::Parse {
            key: "PORT".into(),
            expected: "u16",
            got: "banana".into(),
            source: "invalid digit found in string".into(),
            location: Location::default(),
        };
        let msg = err.to_string();
        assert!(msg.starts_with("PORT: "));
    }

    #[test]
    fn dotenv_load_displays_path() {
        let err = Error::DotenvLoad {
            path: PathBuf::from("/tmp/.env"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/.env"));
    }

    #[test]
    fn dotenv_parse_displays_location() {
        let err = Error::DotenvParse {
            path: PathBuf::from("/tmp/.env"),
            line: 3,
            message: "missing = in assignment".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/.env:3"));
        assert!(msg.contains("missing ="));
    }

    #[test]
    fn with_location_passes_through_dotenv_errors() {
        let err = Error::DotenvLoad {
            path: PathBuf::from("/tmp/.env"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        }
        .with_location("src/main.rs", 5);
        assert!(matches!(err, Error::DotenvLoad { .. }));
    }
}
