//! HTTP clients for the OpenAI-compatible transcription, diarization and
//! voice-embedding endpoints.
//!
//! All requests go straight from the Stratum backend to the configured
//! endpoint — no intermediate services.

use crate::parse::{parse_diarization_json, parse_transcription_json};
use crate::types::{DiarizationResult, Transcript};
use pkm_core::endpoint::validate_endpoint_safe;
use pkm_core::{PkmError, PkmResult};
use std::path::Path;
use std::time::Duration;

/// Long clip transcription can take minutes on CPU-bound servers.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(600);

/// Common client configuration for all STT-related endpoints.
#[derive(Debug, Clone)]
pub struct SttEndpoint {
    /// Base URL without trailing slash, e.g. `http://127.0.0.1:8081`.
    pub base_url: String,
    /// Optional bearer token.
    pub api_key: Option<String>,
}

impl SttEndpoint {
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> PkmResult<Self> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        validate_endpoint_safe(&base_url).map_err(|e| PkmError::Ai(e.to_string()))?;
        Ok(Self { base_url, api_key })
    }

    fn http(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("reqwest client build should not fail")
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn auth(&self) -> Option<String> {
        self.api_key.as_ref().map(|k| format!("Bearer {k}"))
    }
}

/// Client for `POST /v1/audio/transcriptions`.
pub struct Transcriber {
    ep: SttEndpoint,
}

impl Transcriber {
    pub fn new(ep: SttEndpoint) -> Self {
        Self { ep }
    }

    /// Transcribe an audio file, returning `verbose_json` segments.
    pub async fn transcribe(
        &self,
        file_path: &Path,
        model: &str,
        language: Option<&str>,
    ) -> PkmResult<Transcript> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio");
        let bytes = std::fs::read(file_path)
            .map_err(|e| PkmError::Audio(format!("failed to read {}: {e}", file_path.display())))?;

        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(file_name.to_string())
            .mime_str("audio/flac")
            .map_err(|e| PkmError::Ai(format!("multipart error: {e}")))?;

        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", model.to_string())
            .text("response_format", "verbose_json".to_string());
        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        }

        let mut req = self
            .ep
            .http()
            .post(self.ep.url("/v1/audio/transcriptions"))
            .multipart(form);
        if let Some(auth) = self.ep.auth() {
            req = req.header("Authorization", auth);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("transcription request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(PkmError::Ai(format!(
                "transcription failed (HTTP {status}): {}",
                truncate(&body, 300)
            )));
        }
        parse_transcription_json(&body)
    }
}

/// Client for `POST /v1/audio/diarization`.
pub struct Diarizer {
    ep: SttEndpoint,
}

impl Diarizer {
    pub fn new(ep: SttEndpoint) -> Self {
        Self { ep }
    }

    /// Diarize an audio file into speaker-labelled segments.
    ///
    /// Returns [`PkmError::Unsupported`] when the endpoint does not provide
    /// the diarization route or model — callers fall back to a flat
    /// transcript in that case.
    pub async fn diarize(
        &self,
        file_path: &Path,
        model: &str,
        language: Option<&str>,
    ) -> PkmResult<DiarizationResult> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio");
        let bytes = std::fs::read(file_path)
            .map_err(|e| PkmError::Audio(format!("failed to read {}: {e}", file_path.display())))?;

        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(file_name.to_string())
            .mime_str("audio/flac")
            .map_err(|e| PkmError::Ai(format!("multipart error: {e}")))?;

        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", model.to_string())
            .text("response_format", "verbose_json".to_string());
        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        }

        let mut req = self
            .ep
            .http()
            .post(self.ep.url("/v1/audio/diarization"))
            .multipart(form);
        if let Some(auth) = self.ep.auth() {
            req = req.header("Authorization", auth);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("diarization request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status == reqwest::StatusCode::NOT_FOUND
            || status == reqwest::StatusCode::NOT_IMPLEMENTED
            || status == reqwest::StatusCode::BAD_REQUEST
        {
            return Err(PkmError::Unsupported(format!(
                "diarization not available (HTTP {status}): {}",
                truncate(&body, 200)
            )));
        }
        if !status.is_success() {
            return Err(PkmError::Ai(format!(
                "diarization failed (HTTP {status}): {}",
                truncate(&body, 300)
            )));
        }
        parse_diarization_json(&body)
    }
}

/// Client for speaker recognition via `POST /v1/voice/embed`.
///
/// Enrollment clips and probe slices are sent as base64 data URIs (the
/// endpoint's convention); matching happens in-process on the returned
/// embeddings, so the registry survives endpoint restarts.
pub struct VoiceIdClient {
    ep: SttEndpoint,
    model: String,
}

impl VoiceIdClient {
    pub fn new(ep: SttEndpoint, model: impl Into<String>) -> Self {
        Self {
            ep,
            model: model.into(),
        }
    }

    /// Extract the 192-dim speaker embedding for an audio file.
    pub async fn embed(&self, file_path: &Path) -> PkmResult<Vec<f32>> {
        let bytes = std::fs::read(file_path)
            .map_err(|e| PkmError::Audio(format!("failed to read {}: {e}", file_path.display())))?;
        let mime = match file_path.extension().and_then(|e| e.to_str()) {
            Some("wav") => "audio/wav",
            Some("flac") => "audio/flac",
            Some("mp3") => "audio/mpeg",
            Some("ogg") => "audio/ogg",
            Some("m4a") => "audio/mp4",
            _ => "application/octet-stream",
        };
        let data_uri = format!("data:{mime};base64,{}", base64(&bytes));

        #[derive(serde::Serialize)]
        struct Req<'a> {
            model: &'a str,
            audio: String,
        }
        let payload = Req {
            model: &self.model,
            audio: data_uri,
        };

        let mut req = self
            .ep
            .http()
            .post(self.ep.url("/v1/voice/embed"))
            .json(&payload);
        if let Some(auth) = self.ep.auth() {
            req = req.header("Authorization", auth);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("voice embed request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status == reqwest::StatusCode::NOT_FOUND
            || status == reqwest::StatusCode::NOT_IMPLEMENTED
        {
            return Err(PkmError::Unsupported(format!(
                "voice recognition not available (HTTP {status})"
            )));
        }
        if !status.is_success() {
            return Err(PkmError::Ai(format!(
                "voice embed failed (HTTP {status}): {}",
                truncate(&body, 300)
            )));
        }
        #[derive(serde::Deserialize)]
        struct Resp {
            #[serde(default)]
            embedding: Vec<f32>,
        }
        let resp: Resp = serde_json::from_str(&body)
            .map_err(|e| PkmError::Ai(format!("voice embed parse error: {e}")))?;
        if resp.embedding.is_empty() {
            return Err(PkmError::Ai("voice embed returned empty embedding".into()));
        }
        Ok(resp.embedding)
    }
}

/// Cosine similarity in [0, 1] (higher = more similar).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += f64::from(*x) * f64::from(*y);
        na += f64::from(*x) * f64::from(*x);
        nb += f64::from(*y) * f64::from(*y);
    }
    let denom = (na * nb).sqrt();
    if denom <= 0.0 {
        0.0
    } else {
        (dot / denom).clamp(0.0, 1.0)
    }
}

/// Identify a probe embedding against enrolled `(name, embedding)` pairs.
///
/// `min_score` is a cosine-similarity cutoff below which a probe is
/// considered unknown (returns an empty result). Matches sorted by score
/// descending.
pub fn identify(
    probe: &[f32],
    enrolled: &[(String, Vec<f32>)],
    min_score: f64,
) -> Vec<crate::types::VoiceMatch> {
    let mut matches: Vec<crate::types::VoiceMatch> = enrolled
        .iter()
        .map(|(name, emb)| crate::types::VoiceMatch {
            name: name.clone(),
            score: cosine_similarity(probe, emb),
        })
        .filter(|m| m.score >= min_score)
        .collect();
    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matches
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn base64(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-9);
        assert!(cosine_similarity(&a, &c) < 1e-9);
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&a, &[1.0]), 0.0); // length mismatch
    }

    #[test]
    fn test_identify_ranking_and_threshold() {
        let enrolled = vec![
            ("Bob".to_string(), vec![0.6, 0.8]),
            ("Alice".to_string(), vec![1.0, 0.0]),
        ];
        let probe = vec![0.99, 0.01];
        let matches = identify(&probe, &enrolled, 0.5);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].name, "Alice");
        assert!(matches[0].score > matches[1].score);

        let weak = identify(&probe, &enrolled, 1.0);
        assert!(weak.is_empty(), "below threshold -> unknown");
    }
}
