//! Speech-to-text, diarization and voice recognition clients.
//!
//! Stratum's voice dictation talks directly to a configurable
//! OpenAI-compatible endpoint:
//! - `POST {endpoint}/v1/audio/transcriptions` — transcription
//! - `POST {endpoint}/v1/audio/diarization` — speaker diarization
//! - `POST {endpoint}/v1/voice/embed` — speaker embeddings for voice→name
//!   assignment (matching itself happens in-process, so enrollments survive
//!   endpoint restarts).
//!
//! No LocalAI-specific plumbing, no intermediate services: any server
//! implementing these routes works.

pub mod client;
pub mod merge;
pub mod parse;
pub mod types;

pub use client::{identify, cosine_similarity, Diarizer, SttEndpoint, Transcriber, VoiceIdClient};
pub use merge::assign_speakers;
pub use parse::{normalize_ts, parse_diarization_json, parse_transcription_json};
pub use types::{
    DiarizationResult, DiarizationSegment, IdentifyResult, SpeakerTurn, Transcript,
    TranscriptSegment, VoiceMatch,
};
