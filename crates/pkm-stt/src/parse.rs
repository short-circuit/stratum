//! Parsing of OpenAI-compatible `verbose_json` responses.
//!
//! Timestamp units differ wildly between backends:
//! - whisper.cpp: integer **nanoseconds** (e.g. `9_640_000_000`)
//! - LocalAI whisperx: seconds erroneously scaled by 1e-9 (e.g. `3e-09` = 3 s)
//! - diarization endpoint + OpenAI: plain **seconds** floats (`0.0`, `2.34`)
//!
//! [`normalize_ts`] disambiguates by magnitude.

use crate::types::{DiarizationResult, Transcript};
use pkm_core::{PkmError, PkmResult};
use serde::Deserialize;

/// Normalize a backend timestamp to seconds.
///
/// - `v <= 0` → 0
/// - `|v| >= 1e6` → nanoseconds, divide by 1e9
/// - `|v| < 1e-3` → broken whisperx scale (seconds * 1e-9), multiply by 1e9
/// - otherwise → seconds as-is
pub fn normalize_ts(v: f64) -> f64 {
    if !v.is_finite() || v <= 0.0 {
        return 0.0;
    }
    if v >= 1e6 {
        v / 1e9
    } else if v < 1e-3 {
        v * 1e9
    } else {
        v
    }
}

#[derive(Debug, Deserialize)]
struct RawTranscription {
    #[serde(default)]
    text: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    segments: Vec<RawSegment>,
}

#[derive(Debug, Deserialize)]
struct RawSegment {
    #[serde(default)]
    start: f64,
    #[serde(default)]
    end: f64,
    #[serde(default)]
    text: String,
}

/// Parse a `verbose_json` transcription response.
pub fn parse_transcription_json(body: &str) -> PkmResult<Transcript> {
    let raw: RawTranscription = serde_json::from_str(body)
        .map_err(|e| PkmError::Ai(format!("transcription parse error: {e}")))?;
    let segments = raw
        .segments
        .into_iter()
        .map(|s| crate::types::TranscriptSegment {
            start: normalize_ts(s.start),
            end: normalize_ts(s.end),
            text: s.text.trim().to_string(),
        })
        .filter(|s| !s.text.is_empty())
        .collect();
    Ok(Transcript {
        text: raw.text.trim().to_string(),
        language: raw.language,
        segments,
    })
}

#[derive(Debug, Deserialize)]
struct RawDiarization {
    #[serde(default)]
    num_speakers: usize,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    segments: Vec<RawDiarSegment>,
}

#[derive(Debug, Deserialize)]
struct RawDiarSegment {
    #[serde(default)]
    speaker: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    start: f64,
    #[serde(default)]
    end: f64,
    #[serde(default)]
    text: Option<String>,
}

/// Parse a `verbose_json` diarization response.
pub fn parse_diarization_json(body: &str) -> PkmResult<DiarizationResult> {
    let raw: RawDiarization = serde_json::from_str(body)
        .map_err(|e| PkmError::Ai(format!("diarization parse error: {e}")))?;
    let segments: Vec<crate::types::DiarizationSegment> = raw
        .segments
        .into_iter()
        .map(|s| crate::types::DiarizationSegment {
            speaker: if s.speaker.is_empty() {
                s.label
            } else {
                s.speaker
            },
            start: normalize_ts(s.start),
            end: normalize_ts(s.end),
            text: s.text.map(|t| t.trim().to_string()),
        })
        .filter(|s| !s.speaker.is_empty())
        .collect();
    Ok(DiarizationResult {
        num_speakers: if raw.num_speakers > 0 {
            raw.num_speakers
        } else {
            // Derive from distinct speaker ids.
            let mut ids: Vec<&String> = segments.iter().map(|s| &s.speaker).collect();
            ids.sort();
            ids.dedup();
            ids.len()
        },
        duration: raw.duration,
        segments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_ts_units() {
        // nanoseconds (whisper.cpp)
        assert_eq!(normalize_ts(9_640_000_000.0), 9.64);
        // broken whisperx scale: 3e-09 -> 3 s
        assert!((normalize_ts(3e-09) - 3.0).abs() < 1e-9);
        // plain seconds
        assert_eq!(normalize_ts(2.34), 2.34);
        assert_eq!(normalize_ts(0.0), 0.0);
        assert_eq!(normalize_ts(-5.0), 0.0);
        assert_eq!(normalize_ts(f64::NAN), 0.0);
        // sub-second seconds must stay seconds
        assert_eq!(normalize_ts(0.25), 0.25);
    }

    #[test]
    fn test_parse_whisperx_transcription() {
        let body = r#"{
            "text": "The birch canoe slid on the smooth planks.",
            "segments": [
                {"id": 0, "start": 0, "end": 3e-09, "text": " The birch canoe slid on the smooth planks."},
                {"id": 1, "start": 4e-09, "end": 6e-09, "text": "Glue the sheet to the dark blue background."}
            ]
        }"#;
        let t = parse_transcription_json(body).unwrap();
        assert_eq!(t.segments.len(), 2);
        assert!((t.segments[0].end - 3.0).abs() < 1e-6);
        assert!((t.segments[1].start - 4.0).abs() < 1e-6);
        assert_eq!(t.full_text(), "The birch canoe slid on the smooth planks. Glue the sheet to the dark blue background.");
    }

    #[test]
    fn test_parse_openai_transcription() {
        let body = r#"{
            "task": "transcribe",
            "language": "english",
            "duration": 4.5,
            "text": "Hello world",
            "segments": [
                {"id": 0, "start": 0.0, "end": 2.5, "text": "Hello"},
                {"id": 1, "start": 2.5, "end": 4.5, "text": "world"}
            ]
        }"#;
        let t = parse_transcription_json(body).unwrap();
        assert_eq!(t.language.as_deref(), Some("english"));
        assert_eq!(t.segments[1].start, 2.5);
        assert_eq!(t.full_text(), "Hello world");
    }

    #[test]
    fn test_parse_transcription_malformed() {
        assert!(parse_transcription_json("not json").is_err());
        // empty body shape still parses to empty transcript
        let t = parse_transcription_json("{}").unwrap();
        assert!(t.segments.is_empty());
    }

    #[test]
    fn test_parse_diarization() {
        let body = r#"{
            "task": "diarize",
            "duration": 12.34,
            "num_speakers": 2,
            "segments": [
                {"id": 0, "speaker": "SPEAKER_00", "label": "0", "start": 0.00, "end": 2.34},
                {"id": 1, "speaker": "SPEAKER_01", "label": "1", "start": 2.34, "end": 4.10, "text": "How are you?"}
            ]
        }"#;
        let d = parse_diarization_json(body).unwrap();
        assert_eq!(d.num_speakers, 2);
        assert_eq!(d.segments.len(), 2);
        assert_eq!(d.segments[0].speaker, "SPEAKER_00");
        assert_eq!(d.segments[1].text.as_deref(), Some("How are you?"));
    }

    #[test]
    fn test_parse_diarization_derives_speaker_count() {
        let body = r#"{
            "segments": [
                {"speaker": "A", "start": 0, "end": 1},
                {"speaker": "B", "start": 1, "end": 2},
                {"speaker": "A", "start": 2, "end": 3}
            ]
        }"#;
        let d = parse_diarization_json(body).unwrap();
        assert_eq!(d.num_speakers, 2);
    }

    #[test]
    fn test_parse_diarization_label_fallback() {
        let body = r#"{
            "segments": [
                {"label": "7", "start": 0, "end": 1}
            ]
        }"#;
        let d = parse_diarization_json(body).unwrap();
        assert_eq!(d.segments[0].speaker, "7");
    }
}
