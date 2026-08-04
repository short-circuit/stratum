//! Voice dictation pipeline: transcribe → diarize → merge → identify →
//! summarize → link/tag → render a memo for the current note.
//!
//! Crate logic only — no Tauri types. The command handler in the Tauri
//! shell wires this into the UI and inserts the rendered markdown into the
//! current page.

pub mod enrich;
pub mod pipeline;
pub mod render;
pub mod speakers;
pub mod tags;

pub use pipeline::{run, Pipeline, PipelineOptions, PipelineOutput, Stage, VOICE_MATCH_MIN_SCORE};
pub use render::{render_memo, speaker_label, MemoMeta};
pub use speakers::{SpeakerEntry, SpeakerRegistry};

/// Format seconds as "M:SS" or "H:MM:SS".
pub fn format_duration_secs(secs: f64) -> String {
    let total = secs.round() as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_format_duration_secs() {
        assert_eq!(super::format_duration_secs(0.0), "0:00");
        assert_eq!(super::format_duration_secs(272.0), "4:32");
        assert_eq!(super::format_duration_secs(7261.0), "2:01:01");
    }
}
