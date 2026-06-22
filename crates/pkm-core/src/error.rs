use thiserror::Error;

/// Unified error type for the Stratum PKM system.
#[derive(Error, Debug)]
pub enum PkmError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Markdown parse error: {0}")]
    MarkdownParse(String),

    #[error("Frontmatter error: {0}")]
    Frontmatter(String),

    #[error("Note not found: {0}")]
    NoteNotFound(String),

    #[error("Block not found: {0}")]
    BlockNotFound(String),

    #[error("Page not found: {0}")]
    PageNotFound(String),

    #[error("Cycle detected: {0}")]
    CycleDetected(String),

    #[error("Index error: {0}")]
    Index(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("AI provider error: {0}")]
    Ai(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Unsupported: {0}")]
    Unsupported(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience alias.
pub type PkmResult<T> = Result<T, PkmError>;

impl From<serde_yaml::Error> for PkmError {
    fn from(e: serde_yaml::Error) -> Self {
        PkmError::Frontmatter(e.to_string())
    }
}

impl From<serde_json::Error> for PkmError {
    fn from(e: serde_json::Error) -> Self {
        PkmError::Serialization(e.to_string())
    }
}

impl From<toml::de::Error> for PkmError {
    fn from(e: toml::de::Error) -> Self {
        PkmError::Config(e.to_string())
    }
}

impl From<toml::ser::Error> for PkmError {
    fn from(e: toml::ser::Error) -> Self {
        PkmError::Config(e.to_string())
    }
}

impl From<regex::Error> for PkmError {
    fn from(e: regex::Error) -> Self {
        PkmError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PkmError::NoteNotFound("test.md".to_string());
        assert_eq!(err.to_string(), "Note not found: test.md");

        let err = PkmError::Index("corrupt index".to_string());
        assert_eq!(err.to_string(), "Index error: corrupt index");
    }

    #[test]
    fn test_error_from_std_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let pkm_err: PkmError = io_err.into();
        assert!(pkm_err.to_string().contains("file missing"));
    }

    #[test]
    fn test_error_from_serde_yaml() {
        // Create a serde_yaml error by trying to parse invalid YAML
        let result: Result<serde_yaml::Value, _> = serde_yaml::from_str(": bad yaml");
        let yaml_err = result.unwrap_err();
        let pkm_err: PkmError = yaml_err.into();
        assert!(matches!(pkm_err, PkmError::Frontmatter(_)));
    }

    #[test]
    fn test_pkm_result_usage() {
        fn works() -> PkmResult<i32> {
            Ok(42)
        }
        fn fails() -> PkmResult<i32> {
            Err(PkmError::Internal("boom".to_string()))
        }
        assert_eq!(works().unwrap(), 42);
        assert!(fails().is_err());
    }

    #[test]
    fn test_all_variants_display() {
        let variants: Vec<PkmError> = vec![
            PkmError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io")),
            PkmError::Config("config".into()),
            PkmError::MarkdownParse("md".into()),
            PkmError::Frontmatter("fm".into()),
            PkmError::NoteNotFound("nn".into()),
            PkmError::Index("idx".into()),
            PkmError::Search("search".into()),
            PkmError::Git("git".into()),
            PkmError::Sync("sync".into()),
            PkmError::Plugin("plug".into()),
            PkmError::Ai("ai".into()),
            PkmError::Serialization("ser".into()),
            PkmError::Validation("val".into()),
            PkmError::NotFound("nf".into()),
            PkmError::AlreadyExists("ae".into()),
            PkmError::Unsupported("uns".into()),
            PkmError::Internal("int".into()),
        ];
        for v in &variants {
            let s = v.to_string();
            assert!(!s.is_empty(), "Display impl produced empty for {:?}", v);
        }
    }
}
