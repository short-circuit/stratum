//! Shared data types for speech-to-text, diarization and voice recognition.

use serde::{Deserialize, Serialize};

/// A single transcription segment with timestamps (seconds).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// Full transcription result from `POST /v1/audio/transcriptions`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Transcript {
    pub text: String,
    pub language: Option<String>,
    pub segments: Vec<TranscriptSegment>,
}

/// A speaker-labelled utterance after merging transcription with diarization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeakerTurn {
    /// Endpoint speaker id (e.g. "SPEAKER_00"). None = no diarization data.
    pub speaker: Option<String>,
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// A speaker turn from `POST /v1/audio/diarization`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiarizationSegment {
    /// Normalized speaker id, e.g. "SPEAKER_00".
    pub speaker: String,
    pub start: f64,
    pub end: f64,
    /// Per-segment transcript when the backend provides one.
    pub text: Option<String>,
}

/// Diarization result (verbose_json).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DiarizationResult {
    pub num_speakers: usize,
    pub duration: Option<f64>,
    pub segments: Vec<DiarizationSegment>,
}

/// One candidate from a voice identification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceMatch {
    pub name: String,
    /// Cosine similarity in [0, 1]. Higher = better.
    pub score: f64,
}

/// Result of identifying a probe clip against enrolled voices.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct IdentifyResult {
    /// Matches sorted by score descending.
    pub matches: Vec<VoiceMatch>,
}

impl Transcript {
    /// Concatenated plain text of all segments (or the top-level text).
    pub fn full_text(&self) -> String {
        if self.segments.is_empty() {
            return self.text.clone();
        }
        self.segments
            .iter()
            .map(|s| s.text.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}
