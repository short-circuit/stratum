//! Vault-local speaker registry (`<vault>/.pkm/speakers.toml`).
//!
//! Maps voice identities (names) to enrollment clips and cached speaker
//! embeddings. Embeddings are stored in the vault so enrollment survives
//! endpoint restarts; re-embedding happens lazily when a clip is present.

use pkm_core::{PkmError, PkmResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single enrolled (or manually named) voice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeakerEntry {
    /// Display name, e.g. "Alice".
    pub name: String,
    /// Vault-relative path of the enrollment clip (optional for manual-only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<String>,
    /// Cached speaker embedding (192-dim for ECAPA-TDNN).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// ISO timestamp of enrollment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrolled_at: Option<String>,
}

/// The registry: an ordered list of known voices.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SpeakerRegistry {
    #[serde(default)]
    pub speakers: Vec<SpeakerEntry>,
}

impl SpeakerRegistry {
    /// Load from disk; a missing file yields an empty registry.
    pub fn load(path: &Path) -> PkmResult<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .map_err(|e| PkmError::Io(std::io::Error::other(format!("{path:?}: {e}"))))?;
        toml::from_str(&raw).map_err(PkmError::from)
    }

    /// Persist to disk (creating parent dirs).
    pub fn save(&self, path: &Path) -> PkmResult<()> {
        let raw = toml::to_string_pretty(self).map_err(PkmError::from)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(PkmError::from)?;
        }
        std::fs::write(path, raw).map_err(PkmError::from)?;
        Ok(())
    }

    /// Names of all enrolled voices.
    pub fn names(&self) -> Vec<String> {
        self.speakers.iter().map(|s| s.name.clone()).collect()
    }

    /// Entries that have an embedding and can be matched against.
    pub fn enrollable(&self) -> Vec<(String, Vec<f32>)> {
        self.speakers
            .iter()
            .filter_map(|s| s.embedding.as_ref().map(|e| (s.name.clone(), e.clone())))
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<&SpeakerEntry> {
        self.speakers.iter().find(|s| s.name.eq_ignore_ascii_case(name))
    }

    /// Insert or replace an entry by name (case-insensitive).
    pub fn upsert(&mut self, entry: SpeakerEntry) {
        if let Some(existing) = self
            .speakers
            .iter_mut()
            .find(|s| s.name.eq_ignore_ascii_case(&entry.name))
        {
            *existing = entry;
        } else {
            self.speakers.push(entry);
        }
    }

    /// Remove an entry by name; returns whether anything was removed.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.speakers.len();
        self.speakers
            .retain(|s| !s.name.eq_ignore_ascii_case(name));
        self.speakers.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn entry(name: &str) -> SpeakerEntry {
        SpeakerEntry {
            name: name.to_string(),
            clip: Some(format!("assets/speakers/{name}.flac")),
            embedding: Some(vec![0.1, 0.2, 0.3]),
            enrolled_at: Some("2026-08-04T19:00:00Z".to_string()),
        }
    }

    #[test]
    fn test_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("speakers.toml");
        let mut reg = SpeakerRegistry::default();
        reg.upsert(entry("Alice"));
        reg.upsert(entry("Bob"));
        reg.save(&path).unwrap();

        let loaded = SpeakerRegistry::load(&path).unwrap();
        assert_eq!(loaded.names(), vec!["Alice", "Bob"]);
        assert_eq!(loaded.get("alice").unwrap().embedding, Some(vec![0.1, 0.2, 0.3]));
    }

    #[test]
    fn test_missing_file_is_empty() {
        let dir = tempdir().unwrap();
        let reg = SpeakerRegistry::load(&dir.path().join("nope.toml")).unwrap();
        assert!(reg.speakers.is_empty());
        assert!(reg.enrollable().is_empty());
    }

    #[test]
    fn test_upsert_and_remove() {
        let mut reg = SpeakerRegistry::default();
        reg.upsert(entry("Alice"));
        reg.upsert(SpeakerEntry {
            name: "ALICE".to_string(),
            clip: None,
            embedding: None,
            enrolled_at: None,
        });
        assert_eq!(reg.speakers.len(), 1, "case-insensitive upsert");
        assert_eq!(reg.get("alice").unwrap().clip, None);
        assert!(reg.remove("ALICE"));
        assert!(!reg.remove("nobody"));
        assert!(reg.speakers.is_empty());
    }
}
