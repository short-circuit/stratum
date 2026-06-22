use crate::error::PkmResult;

/// Common types used across the project.

/// A vault-relative file path.
pub type VaultPath = String;

/// File change event kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileEvent {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// Search mode for vault search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchMode {
    FullText,
    Semantic,
    Graph,
    Regex,
}

/// Search result from index queries.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: VaultPath,
    pub title: String,
    pub snippet: String,
    pub score: f64,
    pub matched_terms: Vec<String>,
}

/// Sync status for the vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    Idle,
    Syncing,
    Conflicts,
    Error,
    UpToDate,
}

/// Progress callback for long-running operations.
pub type ProgressCallback = Box<dyn Fn(String, f32) + Send + 'static>;

/// Initialize logging/tracing for the application.
pub fn init_logging() -> PkmResult<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_mode_variants() {
        let modes = [
            SearchMode::FullText,
            SearchMode::Semantic,
            SearchMode::Graph,
            SearchMode::Regex,
        ];
        assert_eq!(modes.len(), 4);
    }

    #[test]
    fn test_file_event_variants() {
        let events = [
            FileEvent::Created,
            FileEvent::Modified,
            FileEvent::Deleted,
            FileEvent::Renamed,
        ];
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn test_sync_status_transitions() {
        let statuses = [
            SyncStatus::Idle,
            SyncStatus::Syncing,
            SyncStatus::Conflicts,
            SyncStatus::Error,
            SyncStatus::UpToDate,
        ];
        assert_eq!(statuses.len(), 5);
    }

    #[test]
    fn test_search_result_creation() {
        let result = SearchResult {
            path: "notes/test.md".to_string(),
            title: "Test Note".to_string(),
            snippet: "This is a test snippet with <b>matched</b> terms.".to_string(),
            score: 0.95,
            matched_terms: vec!["test".to_string(), "note".to_string()],
        };
        assert_eq!(result.path, "notes/test.md");
        assert_eq!(result.score, 0.95);
        assert_eq!(result.matched_terms.len(), 2);
    }

    #[test]
    fn test_init_logging_does_not_panic() {
        // just ensure it doesn't crash on first call
        let _ = init_logging();
    }
}
