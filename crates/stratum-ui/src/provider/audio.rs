//! Read-aloud audio bridge for the real provider.
//!
//! [`AppProvider::speak_text`] synthesizes through the service layer and hands
//! the decoded buffer to a detached playback thread. Playback failures are
//! logged only: audio is a best-effort side effect and must not turn an
//! otherwise successful read-aloud request into an error.

use super::AppProvider;

impl AppProvider {
    /// Synthesize `text` and start playback on a detached thread.
    ///
    /// The thread owns the `OutputStream` for the whole playback, since dropping
    /// it stops audio.
    pub(super) fn speak_text(&self, text: String) -> Result<(), String> {
        let result = self.runtime.block_on(self.service.tts_speak(text))?;
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(result.audio_b64.as_bytes())
            .map_err(|error| format!("TTS audio decode failed: {error}"))?;
        std::thread::spawn(move || {
            if let Err(error) = play_audio(bytes) {
                log::warn!("read-aloud playback failed: {error}");
            }
        });
        Ok(())
    }
}

/// Play one synthesized audio buffer on a detached thread.
///
/// Shared by read-aloud and the Settings TTS test button. Playback failures are
/// logged only: the probe already succeeded once the endpoint answered, so a
/// missing audio device must not fail the test.
pub(super) fn play_tts_bytes(bytes: Vec<u8>) {
    std::thread::spawn(move || {
        if let Err(error) = play_audio(bytes) {
            log::warn!("TTS playback failed: {error}");
        }
    });
}

/// Decode and play one synthesized audio buffer.
///
/// The `OutputStream` is kept alive on the stack for the duration of playback:
/// dropping it stops the audio device, so it must outlive `sleep_until_end`.
fn play_audio(bytes: Vec<u8>) -> Result<(), String> {
    use std::io::Cursor;

    use rodio::{Decoder, OutputStream, Sink};

    let (_stream, handle) = OutputStream::try_default().map_err(|error| error.to_string())?;
    let sink = Sink::try_new(&handle).map_err(|error| error.to_string())?;
    let source = Decoder::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
