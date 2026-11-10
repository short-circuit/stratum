//! Pure voice-capture stage machine.
//!
//! [`VoiceStage`] mirrors the shipped `VoiceCaptureButton.shared.tsx` pipeline
//! (Idle → Recording → Transcribing → Inserted | NeedsConfig | Fallback | Error,
//! with retry) and [`format_timer`] renders the recording duration. The stage
//! labels resolve through [`stratum_i18n::t_global`].

/// One step of the voice-capture pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceStage {
    /// No capture in flight.
    Idle,
    /// Microphone recording in progress.
    Recording,
    /// Transcription running on the background executor.
    Transcribing,
    /// A memo was inserted into the page.
    Inserted,
    /// STT is not configured; a clip placeholder was inserted.
    NeedsConfig,
    /// Transcription failed; a clip placeholder was inserted and retry is
    /// offered.
    Fallback,
    /// Recording start/stop failed.
    Error,
}

impl VoiceStage {
    /// Whether a new recording can start from this stage.
    pub fn can_start(self) -> bool {
        matches!(
            self,
            VoiceStage::Idle
                | VoiceStage::Inserted
                | VoiceStage::NeedsConfig
                | VoiceStage::Fallback
                | VoiceStage::Error
        )
    }

    /// Whether the stop action applies.
    pub fn can_stop(self) -> bool {
        self == VoiceStage::Recording
    }

    /// Whether the retry action applies.
    pub fn can_retry(self) -> bool {
        matches!(self, VoiceStage::Fallback | VoiceStage::Error)
    }

    /// Whether the pipeline is busy.
    pub fn is_busy(self) -> bool {
        matches!(self, VoiceStage::Recording | VoiceStage::Transcribing)
    }

    /// The localized label key for this stage.
    pub fn label_key(self) -> &'static str {
        match self {
            VoiceStage::Idle => "voice_capture.record",
            VoiceStage::Recording => "voice_capture.recording",
            VoiceStage::Transcribing => "dictation.transcribing",
            VoiceStage::Inserted => "voice_capture.memo_inserted",
            VoiceStage::NeedsConfig => "voice_capture.configure_stt_prompt",
            VoiceStage::Fallback => "voice_capture.error_fallback_inserted",
            VoiceStage::Error => "voice_capture.error_retry",
        }
    }

    /// The stage reached after a successful recording start.
    pub fn after_start(self) -> VoiceStage {
        VoiceStage::Recording
    }

    /// The stage reached after stop, given whether STT is configured.
    pub fn after_stop(self, stt_configured: bool) -> VoiceStage {
        if stt_configured {
            VoiceStage::Transcribing
        } else {
            VoiceStage::NeedsConfig
        }
    }

    /// The stage reached after a transcription attempt.
    pub fn after_transcribe(self, inserted: bool) -> VoiceStage {
        if inserted {
            VoiceStage::Inserted
        } else {
            VoiceStage::Fallback
        }
    }

    /// The stage reached after a recording/transcription error.
    pub fn after_error(self) -> VoiceStage {
        VoiceStage::Error
    }
}

/// Format seconds as `m:ss` for the recording timer.
pub fn format_timer(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// Result of the background stop/transcribe pipeline.
#[cfg(test)]
mod tests {
    use super::*;
    use stratum_i18n::t_global;
    // `gpui_kit::*` re-exports gpui's `test` attribute macro, which shadows the
    // built-in attribute. Import the built-in explicitly so these plain unit
    // tests stay cheap (no GPUI app context is created).
    use ::core::prelude::v1::test;

    #[test]
    fn stages_expose_the_shipped_actions() {
        assert!(VoiceStage::Idle.can_start());
        assert!(!VoiceStage::Idle.can_stop());
        assert!(VoiceStage::Recording.can_stop());
        assert!(!VoiceStage::Recording.can_start());
        assert!(VoiceStage::Fallback.can_retry());
        assert!(VoiceStage::Error.can_retry());
        assert!(!VoiceStage::Inserted.can_retry());
        assert!(VoiceStage::Transcribing.is_busy());
    }

    #[test]
    fn transitions_follow_the_pipeline() {
        assert_eq!(VoiceStage::Idle.after_start(), VoiceStage::Recording);
        assert_eq!(
            VoiceStage::Recording.after_stop(true),
            VoiceStage::Transcribing
        );
        assert_eq!(
            VoiceStage::Recording.after_stop(false),
            VoiceStage::NeedsConfig
        );
        assert_eq!(
            VoiceStage::Transcribing.after_transcribe(true),
            VoiceStage::Inserted
        );
        assert_eq!(
            VoiceStage::Transcribing.after_transcribe(false),
            VoiceStage::Fallback
        );
        assert_eq!(VoiceStage::Recording.after_error(), VoiceStage::Error);
    }

    #[test]
    fn every_stage_has_a_catalog_label() {
        for stage in [
            VoiceStage::Idle,
            VoiceStage::Recording,
            VoiceStage::Transcribing,
            VoiceStage::Inserted,
            VoiceStage::NeedsConfig,
            VoiceStage::Fallback,
            VoiceStage::Error,
        ] {
            assert_ne!(t_global(stage.label_key()), stage.label_key());
        }
    }

    #[test]
    fn timer_formats_minutes_and_seconds() {
        assert_eq!(format_timer(0), "0:00");
        assert_eq!(format_timer(9), "0:09");
        assert_eq!(format_timer(65), "1:05");
        assert_eq!(format_timer(600), "10:00");
    }
}
