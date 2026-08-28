//! Microphone capture and audio encoding for Stratum voice dictation.
//!
//! This crate is deliberately free of Tauri types so it can be unit-tested
//! standalone. Capture uses [`cpal`] (native ALSA/PipeWire/CoreAudio/WASAPI —
//! the WebKitGTK webview cannot reliably record audio in Tauri on Linux).
//! Clips are persisted as FLAC via [`flacenc`] and read back with [`claxon`].
//!
//! # Modules
//! - [`capture`] — mic recording into an in-memory f32 sample buffer.
//! - [`encode`] — FLAC encode/decode, WAV slice encoding, recording paths.

pub mod capture;
pub mod encode;

pub use capture::{AudioRecorder, CaptureResult, DeviceInfo, RecordingHandle};
pub use encode::{
    decode_flac, encode_flac, encode_wav_slice, format_duration, recording_path, sanitize_slug,
};
