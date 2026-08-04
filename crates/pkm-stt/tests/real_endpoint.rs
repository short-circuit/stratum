//! Regression tests against REAL endpoint payloads captured from a live
//! LocalAI instance (whisper-1 for transcription, pyannote-3.0/sherpa-onnx
//! for diarization on a 2-speaker clip).

use pkm_stt::{assign_speakers, parse_diarization_json, parse_transcription_json};

const TRANSCRIPT_REAL: &str = r#"{"text":" Grant was only a few miles away, but although the commander in chief, he knew nothing of the hardest fought battle of the Civil War until it was over. Fuchs brought up a sack of potatoes, and a piece of cured pork from the cellar. And Grandmother packed some loaves of Saturday's bread, a jar of butter, and several pumpkin pies in the straw of the wagon.","segments":[{"id":0,"start":0,"end":8.0,"text":" Grant was only a few miles away, but although the commander in chief, he knew nothing of the hardest fought battle of the Civil War until it was over."},{"id":1,"start":10.0,"end":15.0,"text":" Fuchs brought up a sack of potatoes, and a piece of cured pork from the cellar."},{"id":2,"start":16.0,"end":24.0,"text":" And Grandmother packed some loaves of Saturday's bread, a jar of butter, and several pumpkin pies in the straw of the wagon."}]}"#;

const DIARIZATION_REAL: &str = r#"{"task":"diarize","duration":24.91,"num_speakers":2,"segments":[{"id":0,"speaker":"SPEAKER_00","label":"0","start":0.42,"end":8.67},{"id":1,"speaker":"SPEAKER_01","label":"1","start":10.54,"end":24.94}]}"#;

#[test]
fn test_real_payloads_merge_into_two_speakers() {
    let transcript = parse_transcription_json(TRANSCRIPT_REAL).expect("transcript parses");
    let diar = parse_diarization_json(DIARIZATION_REAL).expect("diarization parses");

    assert_eq!(transcript.segments.len(), 3);
    assert_eq!(diar.num_speakers, 2);
    // timestamps are plain seconds here
    assert!((transcript.segments[1].start - 10.0).abs() < 1e-6);
    assert!((diar.segments[1].end - 24.94).abs() < 1e-6);

    let turns = assign_speakers(&transcript, &diar);
    // segments 2-3 are consecutive SPEAKER_01 turns with a 1s gap → merged
    assert_eq!(turns.len(), 2);
    // segment 1 overlaps SPEAKER_00 (0.42-8.67); segments 2-3 overlap SPEAKER_01
    assert_eq!(turns[0].speaker.as_deref(), Some("SPEAKER_00"));
    assert_eq!(turns[1].speaker.as_deref(), Some("SPEAKER_01"));
    assert!(turns[0].text.contains("Grant was only a few miles away"));
    assert!(turns[1].text.contains("pumpkin pies"));
    assert!(turns[1].text.contains("sack of potatoes"));
}

#[test]
fn test_real_whisperx_broken_nanosecond_scale() {
    // Payload captured from LocalAI whisper-1 (whisperx backend) where
    // timestamps come back as seconds/1e9 (e.g. 3e-09 == 3s).
    let raw = r#"{"text":"hello world","segments":[{"id":0,"start":0,"end":3e-09,"text":" hello"},{"id":1,"start":4e-09,"end":6e-09,"text":" world"}]}"#;
    let t = parse_transcription_json(raw).expect("parses");
    assert!((t.segments[0].end - 3.0).abs() < 1e-6, "3e-09 => 3s");
    assert!((t.segments[1].start - 4.0).abs() < 1e-6, "4e-09 => 4s");
}

#[test]
fn test_real_whisper_cpp_nanoseconds() {
    // whisper.cpp backends emit integer nanoseconds.
    let raw = r#"{"text":"hi","segments":[{"id":0,"start":0,"end":9640000000,"text":" hi"}]}"#;
    let t = parse_transcription_json(raw).expect("parses");
    assert!((t.segments[0].end - 9.64).abs() < 1e-6);
}
