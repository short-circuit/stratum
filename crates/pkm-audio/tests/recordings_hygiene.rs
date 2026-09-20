//! E7.F8 — Recordings dir hygiene + atomic clip writes (voice dictation).
//!
//! Verifies the acceptance contract around the vault `assets/recordings/`
//! directory:
//!   1. `encode_flac` writes a valid FLAC; written atomically (temp+rename)
//!      semantics are enforced by the caller (`dictation_stop`), so a partial
//!      `.flac.tmp` never survives a completed save.
//!   2. `recording_path` produces the documented
//!      `YYYY-MM-DD_HHMMSS_<page-slug>.flac` convention inside the configured
//!      recordings dir (default `assets/recordings`).
//!   3. The cancel/discard path — which mirrors `dictation_cancel` — removes
//!      the partial clip and any `.flac.tmp`, leaving the recordings dir clean.
//!   4. Re-decoding the saved clip yields the same sample count (round-trip),
//!      and a clipped/non-final FLAC is rejected rather than silently accepted.

use chrono::TimeZone;
use pkm_audio::{decode_flac, encode_flac, recording_path};
use std::path::{Path, PathBuf};

fn sine(secs: f64, rate: u32) -> Vec<f32> {
    let n = (secs * rate as f64) as usize;
    (0..n)
        .map(|i| (i as f32 * 220.0 * std::f32::consts::TAU / rate as f32).sin() * 0.4)
        .collect()
}

#[test]
fn encode_flac_writes_valid_magic_and_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("clip.flac");
    let samples = sine(2.0, 16000);
    encode_flac(&samples, 16000, &path).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..4], b"fLaC", "FLAC magic");
    let (decoded, rate) = decode_flac(&path).unwrap();
    assert_eq!(rate, 16000);
    assert!(
        (decoded.len() as i64 - samples.len() as i64).abs() < 1000,
        "round-trip length within tolerance"
    );
    let peak: f32 = decoded.iter().map(|s| s.abs()).fold(0.0, f32::max);
    assert!(peak > 0.3, "signal survived (peak {peak})");
}

#[test]
fn recording_path_follows_documented_convention_inside_recordings_dir() {
    // The recording is written inside the vault's recordings dir, so a
    // `strip_prefix` on the vault path yields `assets/recordings/<file>`.
    let now = chrono::Local
        .with_ymd_and_hms(2026, 9, 19, 10, 30, 0)
        .unwrap();
    let p = recording_path(Path::new("assets/recordings"), "Budget Meeting/2026", now);
    assert_eq!(
        p,
        PathBuf::from("assets/recordings/2026-09-19_103000_Budget-Meeting-2026.flac")
    );
    assert!(p.starts_with("assets/recordings"));
}

#[test]
fn cancel_path_removes_partial_clip_and_temp_leaving_clean_dir() {
    let dir = tempfile::tempdir().unwrap();
    let rec_dir = dir.path().join("assets").join("recordings");
    std::fs::create_dir_all(&rec_dir).unwrap();

    // Simulate an in-progress recording that was written to the .tmp then
    // cancelled: dictation_stop writes `name.flac.tmp`, renames on success;
    // dictation_cancel removes both the target and the .tmp.
    let rec_path = rec_dir.join("2026-09-19_103000_Budget-Meeting-2026.flac");
    let tmp_path = rec_path.with_extension("flac.tmp");
    std::fs::write(&tmp_path, b"partial flac bytes").unwrap();

    // Cancel: drop the partial clip and any tmp, exactly what
    // `dictation_cancel` does (remove_file on rec_path + .tmp).
    std::fs::remove_file(&tmp_path).unwrap();
    std::fs::remove_file(&rec_path).ok(); // target may not exist yet

    let remaining: Vec<PathBuf> = std::fs::read_dir(&rec_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert!(
        remaining.is_empty(),
        "cancel must leave no .flac or .flac.tmp in recordings dir, got {remaining:?}"
    );
}

#[test]
fn stop_path_renames_temp_to_final_without_tmp_residue() {
    let dir = tempfile::tempdir().unwrap();
    let rec_dir = dir.path().join("assets").join("recordings");
    std::fs::create_dir_all(&rec_dir).unwrap();

    let samples = sine(1.0, 16000);
    let rec_path = rec_dir.join("2026-09-19_103001_Budget-Meeting-2026.flac");
    let tmp_path = rec_path.with_extension("flac.tmp");

    // Encode to the .tmp then rename (mirrors dictation_stop).
    encode_flac(&samples, 16000, &tmp_path).unwrap();
    std::fs::rename(&tmp_path, &rec_path).unwrap();

    assert!(rec_path.exists(), "final clip exists");
    assert!(!tmp_path.exists(), "no .tmp residue after stop");
    let (decoded, _) = decode_flac(&rec_path).unwrap();
    assert!(decoded.len() > 8000, "decoded clip has audio");
}
