//! Raw-text parsing helpers for the editable Settings form.
//!
//! The form keeps numeric and list inputs as text so a save can reject
//! malformed values instead of silently substituting defaults. These helpers
//! parse one field at a time and record the localized failure key.

use std::collections::BTreeMap;

use super::form::{
    KEY_AI_EMBEDDING_DIMENSIONS, KEY_AI_EXTRA_HOSTS, KEY_AI_RAG_CHUNKS, KEY_BACKUP_EXCLUDE,
    KEY_BACKUP_INTERVAL, KEY_RESEARCH_MAX_DEPTH, KEY_RESEARCH_MAX_RESULTS,
    KEY_SYNC_COMMIT_INTERVAL, KEY_SYNC_SYNC_INTERVAL, KEY_THEME_FONT_SIZE, KEY_TTS_SPEED,
};

/// Intern a dynamic key when it is one of the known static raw keys.
pub(crate) fn static_key(key: &str) -> &'static str {
    const KEYS: [&str; 11] = [
        KEY_THEME_FONT_SIZE,
        KEY_AI_RAG_CHUNKS,
        KEY_AI_EMBEDDING_DIMENSIONS,
        KEY_AI_EXTRA_HOSTS,
        KEY_RESEARCH_MAX_RESULTS,
        KEY_RESEARCH_MAX_DEPTH,
        KEY_SYNC_COMMIT_INTERVAL,
        KEY_SYNC_SYNC_INTERVAL,
        KEY_BACKUP_INTERVAL,
        KEY_BACKUP_EXCLUDE,
        KEY_TTS_SPEED,
    ];
    KEYS.into_iter()
        .find(|candidate| *candidate == key)
        .unwrap_or("")
}

pub(crate) fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

pub(crate) fn parse_u32(
    raw: &str,
    field: &'static str,
    key: &'static str,
    errors: &mut BTreeMap<&'static str, &'static str>,
) -> Option<u32> {
    match raw.trim().parse::<u32>() {
        Ok(value) => Some(value),
        Err(_) => {
            errors.insert(field, key);
            None
        }
    }
}

pub(crate) fn parse_u64(
    raw: &str,
    field: &'static str,
    key: &'static str,
    errors: &mut BTreeMap<&'static str, &'static str>,
) -> Option<u64> {
    match raw.trim().parse::<u64>() {
        Ok(value) => Some(value),
        Err(_) => {
            errors.insert(field, key);
            None
        }
    }
}

pub(crate) fn parse_usize(
    raw: &str,
    field: &'static str,
    key: &'static str,
    errors: &mut BTreeMap<&'static str, &'static str>,
) -> Option<usize> {
    match raw.trim().parse::<usize>() {
        Ok(value) => Some(value),
        Err(_) => {
            errors.insert(field, key);
            None
        }
    }
}

pub(crate) fn parse_f32(
    raw: &str,
    field: &'static str,
    key: &'static str,
    errors: &mut BTreeMap<&'static str, &'static str>,
) -> Option<f32> {
    match raw.trim().parse::<f32>() {
        Ok(value) => Some(value),
        Err(_) => {
            errors.insert(field, key);
            None
        }
    }
}
