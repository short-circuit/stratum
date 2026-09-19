//! Input validation & payload caps (contract §11).
//!
//! The server enforces:
//! - `MCP_BODY_MAX` (2 MiB) on inbound request bodies so a single oversized
//!   write cannot exhaust memory;
//! - per-field length caps (path ≤ 4096, query ≤ some bound, content capped by
//!   body limit);
//! - structural validation of tool arguments (each `kb_*` tool has a binding
//!   JSON Schema; invalid input is rejected before dispatch as `InvalidArgs`).
//!
//! This module provides the shared constants and small validators the server
//! binds to its tool layer. All validators return [`crate::error::SecurityError`]
//! so failures map onto the contract `InvalidArgs` error cleanly.

use crate::error::SecurityError;

/// Maximum length of a user-supplied query string for search/autocomplete.
pub const MAX_QUERY_LEN: usize = 512;
/// Maximum length of a single block's content (kept far below body cap so a
/// huge single block cannot be passed through).
pub const MAX_BLOCK_CONTENT_LEN: usize = 64 * 1024;
/// Maximum number of blocks a single `kb_write_page` may carry (guards against
/// pathological payloads while remaining far above real notes).
pub const MAX_BLOCKS_PER_PAGE: usize = 20_000;

/// Validate a user-supplied query string.
///
/// Rejects empty queries and queries exceeding [`MAX_QUERY_LEN`].
pub fn validate_query(query: &str) -> Result<(), SecurityError> {
    if query.is_empty() {
        return Err(SecurityError::InvalidInput(
            "query must not be empty".into(),
        ));
    }
    if query.len() > MAX_QUERY_LEN {
        return Err(SecurityError::InputTooLarge(format!(
            "query exceeds {} chars",
            MAX_QUERY_LEN
        )));
    }
    Ok(())
}

/// Validate a single block content length.
pub fn validate_block_content(content: &str) -> Result<(), SecurityError> {
    if content.len() > MAX_BLOCK_CONTENT_LEN {
        return Err(SecurityError::InputTooLarge(format!(
            "block content exceeds {} bytes",
            MAX_BLOCK_CONTENT_LEN
        )));
    }
    Ok(())
}

/// Validate a page's block count against the per-page cap.
pub fn validate_block_count(count: usize) -> Result<(), SecurityError> {
    if count > MAX_BLOCKS_PER_PAGE {
        return Err(SecurityError::InputTooLarge(format!(
            "page exceeds {} blocks",
            MAX_BLOCKS_PER_PAGE
        )));
    }
    Ok(())
}

/// Validate an inbound request body against `MCP_BODY_MAX`.
pub fn validate_body_len(len: usize) -> Result<(), SecurityError> {
    if len > crate::MCP_BODY_MAX {
        return Err(SecurityError::InputTooLarge(format!(
            "request body exceeds {} bytes",
            crate::MCP_BODY_MAX
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_length_validation() {
        assert!(validate_query("hello").is_ok());
        assert!(validate_query("").is_err());
        assert!(validate_query(&"x".repeat(MAX_QUERY_LEN)).is_ok());
        assert!(validate_query(&"x".repeat(MAX_QUERY_LEN + 1)).is_err());
    }

    #[test]
    fn block_content_validation() {
        assert!(validate_block_content("short").is_ok());
        assert!(validate_block_content(&"x".repeat(MAX_BLOCK_CONTENT_LEN)).is_ok());
        assert!(validate_block_content(&"x".repeat(MAX_BLOCK_CONTENT_LEN + 1)).is_err());
    }

    #[test]
    fn block_count_validation() {
        assert!(validate_block_count(100).is_ok());
        assert!(validate_block_count(MAX_BLOCKS_PER_PAGE).is_ok());
        assert!(validate_block_count(MAX_BLOCKS_PER_PAGE + 1).is_err());
    }

    #[test]
    fn body_len_validation() {
        assert!(validate_body_len(10).is_ok());
        assert!(validate_body_len(crate::MCP_BODY_MAX).is_ok());
        assert!(validate_body_len(crate::MCP_BODY_MAX + 1).is_err());
    }
}
