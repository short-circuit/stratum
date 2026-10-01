//! Regression tests pinning the EXACT verbatim response bodies captured from
//! the live OpenAI-compatible endpoint during endpoint validation
//! (task t_b60ea4ea, 2026-09-22). These are raw payloads — not hand-crafted
//! mocks — preserved from `curl` against `http://localhost:8081/v1`
//! (LocalAI). They lock in the real response shapes so the parser contract the
//! backend relies on cannot silently drift.
//!
//! Captures live in the validation report:
//! `docs/development/endpoint-validation.md` (§2.4–2.6), raw files under
//! `/tmp/stratum_captures/` at capture time.

use pkm_stt::{assign_speakers, parse_diarization_json, parse_transcription_json};

/// Verbatim body of `POST /v1/audio/transcriptions` with
/// `model=whisperx-tiny`, `response_format=verbose_json` (3 s tone clip).
/// NOTE: `end` is the known LocalAI whisperx broken nanosecond-scale value
/// `1E-9` (== 1s after normalize_ts).
const STT_WHISPERX_TINY_REAL: &str = r#"{"segments":[{"id":0,"start":0,"end":1E-9,"text":" Thanks for watching!","tokens":[]}],"text":" Thanks for watching!"}"#;

/// Verbatim body of `POST /v1/audio/transcriptions` with `model=whisper-1`
/// (same clip). Timestamps come back as plain small seconds.
const STT_WHISPER1_REAL: &str =
    r#"{"segments":[{"id":0,"start":0,"end":0,"text":" you","tokens":[]}],"text":" you"}"#;

/// Verbatim body of `POST /v1/audio/diarization` with
/// `model=pyannote-diarization` (same clip; 1 speaker). Contains the extra
/// top-level `speakers` array that the parser must ignore.
const DIAR_PYANNOTE_REAL: &str = r#"{"task":"diarize","duration":3,"num_speakers":1,"segments":[{"id":0,"speaker":"SPEAKER_00","label":"1","start":0.03096874989569187,"end":3.051593780517578}],"speakers":[{"id":"SPEAKER_00","label":"1","total_speech_duration":3.0206250306218863,"segment_count":1}]}"#;

#[test]
fn real_whisperx_tiny_verbose_json_parses() {
    let t = parse_transcription_json(STT_WHISPERX_TINY_REAL).expect("parses");
    assert!(t.text.contains("Thanks for watching!"));
    assert_eq!(t.segments.len(), 1);
    // `1E-9` → broken whisperx scale → normalize_ts multiplies by 1e9 → 1s
    assert!(
        (t.segments[0].end - 1.0).abs() < 1e-6,
        "1E-9 must normalize to ~1s, got {}",
        t.segments[0].end
    );
}

#[test]
fn real_whisper1_verbose_json_parses() {
    let t = parse_transcription_json(STT_WHISPER1_REAL).expect("parses");
    assert_eq!(t.text.trim(), "you");
    assert_eq!(t.segments.len(), 1);
    // plain seconds are left untouched
    assert_eq!(t.segments[0].start, 0.0);
    assert_eq!(t.segments[0].end, 0.0);
}

#[test]
fn real_pyannote_diarization_parses_and_ignores_speakers_meta() {
    let d = parse_diarization_json(DIAR_PYANNOTE_REAL).expect("parses");
    assert_eq!(d.num_speakers, 1);
    assert_eq!(d.segments.len(), 1);
    assert_eq!(d.segments[0].speaker, "SPEAKER_00"); // `label`/speaker fallback resolved in parse
                                                     // plain seconds stay seconds
    assert!((d.segments[0].end - 3.051593780517578).abs() < 1e-9);
    assert!((d.duration.unwrap() - 3.0).abs() < 1e-9);
}

#[test]
fn real_pyannote_diarization_merges_with_transcript() {
    // End-to-end speaker assignment against the real 1-speaker clip: the
    // whisperx-tiny transcript (normalized end=1s) overlaps the diarization
    // segment (0.03–3.05s) and is assigned to SPEAKER_00.
    let transcript = parse_transcription_json(STT_WHISPERX_TINY_REAL).expect("transcript parses");
    let diar = parse_diarization_json(DIAR_PYANNOTE_REAL).expect("diarization parses");

    let turns = assign_speakers(&transcript, &diar);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].speaker.as_deref(), Some("SPEAKER_00"));
}
