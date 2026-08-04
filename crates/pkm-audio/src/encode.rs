//! FLAC/WAV encoding, decoding and recording path helpers.
//!
//! Clips are stored as FLAC (10× smaller than WAV, accepted by every
//! transcription endpoint via ffmpeg). WAV is used for short speaker
//! enrollment slices uploaded to the endpoint's voice recognition API.

use chrono::{DateTime, Local};
use pkm_core::{PkmError, PkmResult};
use std::path::{Path, PathBuf};

/// Encode mono f32 samples (`[-1.0, 1.0]`) to a 16-bit FLAC file.
pub fn encode_flac(samples: &[f32], sample_rate: u32, out_path: &Path) -> PkmResult<()> {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;

    let i32_samples: Vec<i32> = samples
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i32)
        .collect();

    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|(_, e)| PkmError::Audio(format!("flac config invalid: {e}")))?;
    let source =
        flacenc::source::MemSource::from_samples(&i32_samples, 1, 16, sample_rate as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| PkmError::Audio(format!("flac encode failed: {e}")))?;

    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|e| PkmError::Audio(format!("flac serialize failed: {e}")))?;
    let bytes = sink.as_slice().to_vec();

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(PkmError::from)?;
    }
    std::fs::write(out_path, bytes).map_err(PkmError::from)?;
    Ok(())
}

/// Decode a FLAC file into mono f32 samples plus the sample rate.
/// Multi-channel files are downmixed by taking the first channel.
pub fn decode_flac(path: &Path) -> PkmResult<(Vec<f32>, u32)> {
    let mut reader = claxon::FlacReader::open(path)
        .map_err(|e| PkmError::Audio(format!("failed to open {}: {e}", path.display())))?;
    let sample_rate = reader.streaminfo().sample_rate;
    let channels = reader.streaminfo().channels.max(1) as usize;

    let mut samples = Vec::new();
    for (i, s) in reader.samples().enumerate() {
        let s = s.map_err(|e| PkmError::Audio(format!("flac read error: {e}")))?;
        if i % channels == 0 {
            samples.push(s as f32 / 32768.0);
        }
    }
    Ok((samples, sample_rate))
}

/// Encode a mono f32 slice as a 16-bit WAV file (for speaker enrollment).
pub fn encode_wav_slice(samples: &[f32], sample_rate: u32, out_path: &Path) -> PkmResult<()> {
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(PkmError::from)?;
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(out_path, spec)
        .map_err(|e| PkmError::Audio(format!("wav create failed: {e}")))?;
    for &s in samples {
        writer
            .write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .map_err(|e| PkmError::Audio(format!("wav write failed: {e}")))?;
    }
    writer
        .finalize()
        .map_err(|e| PkmError::Audio(format!("wav finalize failed: {e}")))?;
    Ok(())
}

/// Sanitize a page slug for use in file names: keep alphanumerics, `-`, `_`;
/// everything else becomes `-`.
pub fn sanitize_slug(slug: &str) -> String {
    let mut out = String::with_capacity(slug.len());
    let mut has_alnum = false;
    for c in slug.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
            if c.is_alphanumeric() {
                has_alnum = true;
            }
        } else {
            out.push('-');
        }
    }
    if !has_alnum {
        out = "voice-memo".to_string();
    }
    out
}

/// Build the clip path: `<dir>/YYYY-MM-DD_HHMMSS_<slug>.flac`.
pub fn recording_path(recordings_dir: &Path, page_slug: &str, now: DateTime<Local>) -> PathBuf {
    recordings_dir.join(format!(
        "{}_{}.flac",
        now.format("%Y-%m-%d_%H%M%S"),
        sanitize_slug(page_slug)
    ))
}

/// Format seconds as "M:SS" or "H:MM:SS".
pub fn format_duration(secs: f64) -> String {
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
    use super::*;
    use chrono::TimeZone;

    fn sine(secs: f64, rate: u32) -> Vec<f32> {
        let n = (secs * rate as f64) as usize;
        (0..n)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin() * 0.5)
            .collect()
    }

    #[test]
    fn test_flac_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clip.flac");
        let samples = sine(2.0, 44100);
        encode_flac(&samples, 44100, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"fLaC", "FLAC magic");
        assert!(bytes.len() > 10_000);

        let (decoded, rate) = decode_flac(&path).unwrap();
        assert_eq!(rate, 44100);
        assert!((decoded.len() as i64 - samples.len() as i64).abs() < 1000);
        let peak: f32 = decoded.iter().map(|s| s.abs()).fold(0.0, f32::max);
        assert!((peak - 0.5).abs() < 0.1, "peak ~= original amplitude, got {peak}");
    }

    #[test]
    fn test_wav_slice() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("slice.wav");
        let samples = sine(0.5, 16000);
        encode_wav_slice(&samples, 16000, &path).unwrap();
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().sample_rate, 16000);
        assert_eq!(reader.len(), samples.len() as u32);
    }

    #[test]
    fn test_recording_path_and_slug() {
        assert_eq!(sanitize_slug("My Page/Notes"), "My-Page-Notes");
        assert_eq!(sanitize_slug("..."), "voice-memo");
        let now = Local.with_ymd_and_hms(2026, 8, 4, 19, 34, 0).unwrap();
        let p = recording_path(Path::new("assets/recordings"), "meeting", now);
        assert_eq!(
            p,
            PathBuf::from("assets/recordings/2026-08-04_193400_meeting.flac")
        );
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.0), "0:00");
        assert_eq!(format_duration(272.0), "4:32");
        assert_eq!(format_duration(7261.0), "2:01:01");
    }
}
