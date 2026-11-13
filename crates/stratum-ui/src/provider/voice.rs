//! Voice-capture provider mapping for the real service-backed provider.
//!
//! [`AppProvider`] forwards the [`super::ShellData`] voice seam onto the
//! `stratum_app` dictation methods. Recording start/stop and clip insertion are
//! synchronous service calls; transcription is async and is bridged through the
//! provider's Tokio runtime. Progress events are discarded with
//! [`stratum_app::NoopSink`] because the native shell drives its own stage
//! labels.

use stratum_app::dictation::DictationOptsDto;

use super::AppProvider;

impl AppProvider {
    /// Report whether an STT endpoint is configured.
    ///
    /// The check mirrors the shipped voice-capture hook: a non-empty
    /// `stt.endpoint` is the only requirement, and an unreadable settings file
    /// counts as unconfigured.
    pub(super) fn stt_is_configured(&self) -> bool {
        self.runtime
            .block_on(self.service.get_settings())
            .map(|settings| !settings.stt.endpoint.trim().is_empty())
            .unwrap_or(false)
    }

    /// Start a recording destined for `page_path`.
    pub(super) fn begin_recording(&self, page_path: &str) -> Result<String, String> {
        self.service
            .dictation_start(page_path.to_string())
            .map(|start| start.recording_path)
    }

    /// Stop the active recording and return its path and duration.
    pub(super) fn finish_recording(&self) -> Result<(String, f64), String> {
        self.service
            .dictation_stop()
            .map(|stop| (stop.recording_path, stop.duration_secs))
    }

    /// Transcribe a saved clip into `page_path`, returning the inserted count.
    ///
    /// The shipped DictationPanel exposes diarize/summarize/identify toggles;
    /// the native shell deliberately uses the configured defaults
    /// (`DictationOptsDto::default()` resolves to the vault config).
    pub(super) fn transcribe_clip(
        &self,
        recording_path: &str,
        page_path: &str,
    ) -> Result<usize, String> {
        self.runtime
            .block_on(self.service.dictation_transcribe(
                recording_path.to_string(),
                page_path.to_string(),
                Some(DictationOptsDto::default()),
                &stratum_app::NoopSink,
            ))
            .map(|result| result.inserted_block_ids.len())
    }

    /// Insert a clip-reference placeholder after a failed or unconfigured
    /// transcription so the recording is never lost.
    pub(super) fn insert_clip(
        &self,
        recording_path: &str,
        page_path: &str,
        seconds: f64,
    ) -> Result<(), String> {
        self.service
            .dictation_insert_clip_ref(
                page_path.to_string(),
                recording_path.to_string(),
                seconds,
                &stratum_app::NoopSink,
            )
            .map(|_| ())
    }
}
