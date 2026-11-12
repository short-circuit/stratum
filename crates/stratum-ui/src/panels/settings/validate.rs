//! Client-side settings validation.
//!
//! The shipped React page relies on HTML `min`/`max` hints and trusts the
//! backend to reject bad values. The native surface validates the complete DTO
//! before saving so a malformed colour, endpoint or interval never reaches the
//! config file. [`validate`] is pure: it returns a field → catalog-key map and
//! never mutates the input.

use std::collections::BTreeMap;

use stratum_app::settings::SettingsDto;

use crate::theme::is_hex_color;

/// Validate every field the native Settings form can edit.
///
/// Keys identify the offending input; values name the localized explanation.
/// An empty map means the DTO is safe to save.
pub fn validate(settings: &SettingsDto) -> BTreeMap<&'static str, &'static str> {
    let mut errors = BTreeMap::new();

    if settings.vault_path.trim().is_empty() {
        errors.insert("vault_path", "settings.validation.required");
    }
    if !is_hex_color(&settings.theme.primary_color) {
        errors.insert("theme_primary", "settings.validation.hex_color");
    }
    if !is_hex_color(&settings.theme.secondary_color) {
        errors.insert("theme_secondary", "settings.validation.hex_color");
    }
    if !(12..=28).contains(&settings.theme.font_size) {
        errors.insert("theme_font_size", "settings.validation.range");
    }

    if let Some(endpoint) = settings.ai.endpoint.as_deref() {
        if !endpoint.trim().is_empty() && !is_http_url(endpoint) {
            errors.insert("ai_endpoint", "settings.validation.url");
        }
    }
    if !(1..=20).contains(&settings.ai.rag_chunk_count) {
        errors.insert("ai_rag_chunk_count", "settings.validation.range");
    }
    if settings.ai.embedding_dimensions > 4096 {
        errors.insert("ai_embedding_dimensions", "settings.validation.range");
    }
    if settings
        .ai
        .extra_local_hosts
        .iter()
        .any(|host| !host.trim().is_empty() && !is_http_url(host))
    {
        errors.insert("ai_extra_local_hosts", "settings.validation.url");
    }

    if !is_http_url(&settings.research.searxng_endpoint) {
        errors.insert("research_endpoint", "settings.validation.url");
    }
    if !(1..=10).contains(&settings.research.max_results) {
        errors.insert("research_max_results", "settings.validation.range");
    }
    if !(1..=5).contains(&settings.research.max_depth) {
        errors.insert("research_max_depth", "settings.validation.range");
    }

    if settings.sync.branch.trim().is_empty() {
        errors.insert("sync_branch", "settings.validation.required");
    }
    if settings.sync.auto_commit_interval_secs < 30 {
        errors.insert(
            "sync_commit_interval",
            "settings.validation.commit_interval",
        );
    }
    if settings.sync.auto_sync_interval_secs < 60 {
        errors.insert("sync_sync_interval", "settings.validation.sync_interval");
    }
    if !settings.sync.commit_template.contains("{datetime}") {
        errors.insert(
            "sync_commit_template",
            "settings.validation.commit_template",
        );
    }

    let backup = &settings.sync.backup;
    if backup.interval_minutes < 1 {
        errors.insert("backup_interval", "settings.validation.positive");
    }
    let uses_webdav = matches!(backup.mode.as_str(), "webdav" | "both");
    if uses_webdav && backup.enabled {
        match backup.webdav_url.as_deref() {
            Some(url) if is_http_url(url) => {}
            _ => {
                errors.insert("backup_webdav_url", "settings.validation.url");
            }
        }
    } else if let Some(url) = backup.webdav_url.as_deref() {
        if !url.trim().is_empty() && !is_http_url(url) {
            errors.insert("backup_webdav_url", "settings.validation.url");
        }
    }

    if !settings.stt.endpoint.trim().is_empty() && !is_http_url(&settings.stt.endpoint) {
        errors.insert("stt_endpoint", "settings.validation.url");
    }

    if !settings.tts.endpoint.trim().is_empty() && !is_http_url(&settings.tts.endpoint) {
        errors.insert("tts_endpoint", "settings.validation.url");
    }
    if settings.tts.voice.trim().is_empty() {
        errors.insert("tts_voice", "settings.validation.required");
    }
    if settings.tts.format.trim().is_empty() {
        errors.insert("tts_format", "settings.validation.required");
    }
    if !(0.25..=4.0).contains(&settings.tts.speed) {
        errors.insert("tts_speed", "settings.validation.range");
    }

    if settings.tray.quick_capture_shortcut.trim().is_empty() {
        errors.insert("tray_shortcut", "settings.validation.required");
    }
    if settings.tray.capture_target == "inbox" && settings.tray.inbox_path.trim().is_empty() {
        errors.insert("tray_inbox_path", "settings.validation.required");
    }

    if settings.voice.target.trim().is_empty() {
        errors.insert("voice_target", "settings.validation.required");
    }

    errors
}

/// Whether `value` is an absolute `http://` or `https://` URL with a host.
pub fn is_http_url(value: &str) -> bool {
    let trimmed = value.trim();
    let Some(rest) = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
    else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or("");
    !host.is_empty() && !host.starts_with(':')
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn accepts_http_and_https_urls() {
        assert!(is_http_url("http://localhost:11434"));
        assert!(is_http_url("https://searx.be"));
        assert!(!is_http_url("ftp://example.com"));
        assert!(!is_http_url("localhost:8888"));
        assert!(!is_http_url("http://"));
    }
}
