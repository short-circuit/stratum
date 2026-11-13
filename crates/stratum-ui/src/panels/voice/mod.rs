//! Shared voice-capture control for the native shell.
//!
//! The stage machine lives in [`stage`] and the gpui entity in [`capture`].
//! Both are re-exported here so existing `panels::voice::{VoiceStage,
//! VoiceCapture}` imports keep working. The journal header owns one instance
//! and the page header reuses it in run B.
//!
//! All user-facing labels resolve through [`stratum_i18n::t_global`]. The
//! DictationPanel's diarize/summarize/identify toggles stay out of scope: the
//! provider transcribes with the configured vault defaults.

mod capture;
mod stage;

pub use capture::VoiceCapture;
pub use stage::{format_timer, VoiceStage};
