//! Microphone capture via `cpal`.
//!
//! Records the default input device into a growable f32 sample buffer
//! (mono, native sample rate). Designed for voice dictation: a recording is
//! started, left running while the user speaks, and stopped to obtain the
//! samples plus duration.

use pkm_core::{PkmError, PkmResult};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, warn};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Description of the input device chosen for recording.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Human-readable device name.
    pub name: String,
    /// Sample rate the device is configured for.
    pub sample_rate: u32,
}

/// Finished capture: raw mono samples plus metadata.
#[derive(Debug, Clone)]
pub struct CaptureResult {
    /// Interleaved-free mono samples in f32 `[-1.0, 1.0]`.
    pub samples: Vec<f32>,
    /// Sample rate the samples were captured at.
    pub sample_rate: u32,
    /// Wall-clock duration of the capture in seconds.
    pub duration_secs: f64,
}

/// An active recording. Drop or call [`AudioRecorder::stop`] to finish.
pub struct RecordingHandle {
    stream: Option<Box<dyn Send + Sync + 'static>>,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    started: Instant,
}

/// Safety cap: drop captured samples beyond 6 hours to bound memory
/// (~6h × 48 kHz × 4 bytes ≈ 4 GB worst case, typically far less at 16 kHz).
const MAX_SAMPLES: usize = 6 * 60 * 60 * 48_000;

/// Records from the default input device.
#[derive(Debug, Default)]
pub struct AudioRecorder;

impl AudioRecorder {
    /// Resolve the default input device and its configuration.
    pub fn default_input() -> PkmResult<DeviceInfo> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| PkmError::Audio("no default input device found".into()))?;
        let config = device
            .default_input_config()
            .map_err(|e| PkmError::Audio(format!("failed to read input config: {e}")))?;
        Ok(DeviceInfo {
            name: device.name().unwrap_or_else(|_| "unknown".into()),
            sample_rate: config.sample_rate().0,
        })
    }

    /// Start recording from the default input device.
    pub fn start(&self) -> PkmResult<RecordingHandle> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| PkmError::Audio("no default input device found".into()))?;
        let config = device
            .default_input_config()
            .map_err(|e| PkmError::Audio(format!("failed to read input config: {e}")))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let buf = Arc::clone(&buffer);

        let err_fn = |e| warn!("audio stream error: {e}");

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| push_mono(&buf, data, channels),
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| push_mono(&buf, data, channels),
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _| push_mono(&buf, data, channels),
                err_fn,
                None,
            ),
            other => {
                return Err(PkmError::Audio(format!(
                    "unsupported sample format {other:?}"
                )))
            }
        }
        .map_err(|e| PkmError::Audio(format!("failed to open input stream: {e}")))?;

        stream
            .play()
            .map_err(|e| PkmError::Audio(format!("failed to start input stream: {e}")))?;
        debug!("recording started rate={sample_rate} channels={channels}");

        Ok(RecordingHandle {
            stream: Some(Box::new(stream)),
            buffer,
            sample_rate,
            started: Instant::now(),
        })
    }

    /// Stop an active recording and return the captured samples.
    pub fn stop(&self, mut handle: RecordingHandle) -> CaptureResult {
        if let Some(stream) = handle.stream.take() {
            drop(stream);
        }
        let samples = std::mem::take(&mut *handle.buffer.lock().unwrap_or_else(|p| p.into_inner()));
        let duration_secs = handle.started.elapsed().as_secs_f64();
        debug!(
            "recording stopped samples={} duration={duration_secs:.1}s",
            samples.len()
        );
        CaptureResult {
            samples,
            sample_rate: handle.sample_rate,
            duration_secs,
        }
    }
}

/// Copy the first channel of a multi-channel frame block into the mono
/// buffer, normalizing integer formats to f32.
fn push_mono<T>(buffer: &Arc<Mutex<Vec<f32>>>, data: &[T], channels: usize)
where
    T: Copy + Sample,
{
    let mut buf = buffer.lock().unwrap_or_else(|p| p.into_inner());
    if buf.len() >= MAX_SAMPLES {
        return;
    }
    let frames = data.len() / channels;
    let take = frames.min(MAX_SAMPLES - buf.len());
    for frame in 0..take {
        buf.push(data[frame * channels].to_f32());
    }
}

/// Conversion helper implemented for the cpal sample formats we support.
pub trait Sample {
    fn to_f32(self) -> f32;
}

impl Sample for f32 {
    #[inline]
    fn to_f32(self) -> f32 {
        self
    }
}

impl Sample for i16 {
    #[inline]
    fn to_f32(self) -> f32 {
        f32::from(self) / 32768.0
    }
}

impl Sample for u16 {
    #[inline]
    fn to_f32(self) -> f32 {
        (f32::from(self) - 32768.0) / 32768.0
    }
}

impl Drop for RecordingHandle {
    fn drop(&mut self) {
        let len = self.buffer.lock().map(|b| b.len()).unwrap_or_default();
        let secs = self.started.elapsed().as_secs_f64();
        if len > 0 {
            debug!(
                "recording dropped without stop ({} samples, {secs:.1}s)",
                len
            );
        }
    }
}

/// Convert a [`Duration`]-like seconds value into "M:SS" or "H:MM:SS".
pub fn duration_label(secs: f64) -> String {
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

    #[test]
    fn test_sample_conversions() {
        assert_eq!(0.0f32.to_f32(), 0.0);
        assert_eq!(32767i16.to_f32(), 32767.0 / 32768.0);
        assert_eq!(0u16.to_f32(), -1.0);
        assert_eq!(65535u16.to_f32(), 32767.0 / 32768.0);
    }

    #[test]
    fn test_duration_label() {
        assert_eq!(duration_label(0.0), "0:00");
        assert_eq!(duration_label(4.0 * 60.0 + 32.0), "4:32");
        assert_eq!(duration_label(2.0 * 3600.0 + 61.0), "2:01:01");
    }

    #[test]
    fn test_recording_requires_device() {
        // CI boxes usually lack an input device; this exercises the error
        // path when no device exists and otherwise captures briefly.
        let recorder = AudioRecorder;
        match AudioRecorder::default_input() {
            Ok(info) => {
                // default_input() may succeed even when the backend can't
                // actually open a stream (e.g. PulseAudio socket missing).
                let handle = match recorder.start() {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("skipped device test — start failed: {e}");
                        return;
                    }
                };
                std::thread::sleep(std::time::Duration::from_millis(150));
                let result = recorder.stop(handle);
                assert!(!result.samples.is_empty());
                assert!(result.duration_secs >= 0.14);
                assert_eq!(result.sample_rate, info.sample_rate);
            }
            Err(e) => {
                eprintln!("skipped device test: {e}");
            }
        }
    }
}
