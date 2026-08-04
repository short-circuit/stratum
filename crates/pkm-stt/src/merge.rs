//! Merge transcription segments with diarization speaker turns.
//!
//! Each transcript segment is assigned the speaker of the diarization
//! segment it overlaps most with. Consecutive segments from the same
//! speaker are grouped into a single [`SpeakerTurn`] when the gap between
//! them is small enough to be one utterance.

use crate::types::{DiarizationResult, SpeakerTurn, Transcript};

/// Max gap (seconds) between consecutive same-speaker transcript segments
/// that are still merged into one utterance.
const MERGE_GAP_SECS: f64 = 2.0;

/// Minimum overlap fraction required to consider a diarization segment the
/// match for a transcript segment before falling back to the best overlap.
const MIN_OVERLAP_FRACTION: f64 = 0.0;

/// Assign speakers to transcript segments and group them into turns.
pub fn assign_speakers(transcript: &Transcript, diar: &DiarizationResult) -> Vec<SpeakerTurn> {
    if transcript.segments.is_empty() {
        return Vec::new();
    }

    let mut turns: Vec<SpeakerTurn> = Vec::with_capacity(transcript.segments.len());
    let mut last_speaker: Option<String> = None;

    for seg in &transcript.segments {
        let speaker = best_speaker(seg.start, seg.end, diar, MIN_OVERLAP_FRACTION)
            .or_else(|| last_speaker.clone());
        if speaker.is_some() {
            last_speaker = speaker.clone();
        }
        turns.push(SpeakerTurn {
            speaker,
            start: seg.start,
            end: seg.end,
            text: seg.text.clone(),
        });
    }

    merge_consecutive(turns)
}

/// Find the diarization segment with the greatest overlap with `[start, end]`.
/// Returns `None` when the best overlap is zero or below `min_fraction` of the
/// transcript segment's duration.
fn best_speaker(
    start: f64,
    end: f64,
    diar: &DiarizationResult,
    min_fraction: f64,
) -> Option<String> {
    let duration = (end - start).max(1e-6);
    let mut best: Option<(String, f64)> = None;
    for ds in &diar.segments {
        let overlap = (end.min(ds.end) - start.max(ds.start)).max(0.0);
        if overlap > 0.0 && overlap / duration >= min_fraction {
            if best.as_ref().map(|(_, o)| overlap > *o).unwrap_or(true) {
                best = Some((ds.speaker.clone(), overlap));
            }
        }
    }
    best.map(|(s, _)| s)
}

/// Merge consecutive turns with the same speaker when the gap is small.
fn merge_consecutive(turns: Vec<SpeakerTurn>) -> Vec<SpeakerTurn> {
    let mut out: Vec<SpeakerTurn> = Vec::with_capacity(turns.len());
    for t in turns {
        if let Some(prev) = out.last_mut() {
            let same_speaker = match (&prev.speaker, &t.speaker) {
                (Some(a), Some(b)) => a == b,
                // None speakers (no diarization) merge into one flat text.
                (None, None) => true,
                _ => false,
            };
            let gap = (t.start - prev.end).max(0.0);
            if same_speaker && gap <= MERGE_GAP_SECS {
                prev.end = t.end;
                if !t.text.is_empty() {
                    if !prev.text.is_empty() {
                        prev.text.push(' ');
                    }
                    prev.text.push_str(t.text.trim());
                }
                continue;
            }
        }
        out.push(t);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DiarizationSegment, TranscriptSegment};

    fn transcript(segs: &[(f64, f64, &str)]) -> Transcript {
        Transcript {
            text: String::new(),
            language: None,
            segments: segs
                .iter()
                .map(|(s, e, t)| TranscriptSegment {
                    start: *s,
                    end: *e,
                    text: t.to_string(),
                })
                .collect(),
        }
    }

    fn diar(segs: &[(&str, f64, f64)]) -> DiarizationResult {
        DiarizationResult {
            num_speakers: 2,
            duration: None,
            segments: segs
                .iter()
                .map(|(sp, s, e)| DiarizationSegment {
                    speaker: sp.to_string(),
                    start: *s,
                    end: *e,
                    text: None,
                })
                .collect(),
        }
    }

    #[test]
    fn test_interleaved_speakers_preserved() {
        // A speaks 0-2, B speaks 2-4, A speaks 4-6
        let t = transcript(&[(0.0, 2.0, "first"), (2.2, 4.0, "second"), (4.2, 6.0, "third")]);
        let d = diar(&[("SPEAKER_00", 0.0, 2.1), ("SPEAKER_01", 2.1, 4.1), ("SPEAKER_00", 4.1, 6.0)]);
        let turns = assign_speakers(&t, &d);
        assert_eq!(turns.len(), 3);
        assert_eq!(turns[0].speaker.as_deref(), Some("SPEAKER_00"));
        assert_eq!(turns[1].speaker.as_deref(), Some("SPEAKER_01"));
        assert_eq!(turns[2].speaker.as_deref(), Some("SPEAKER_00"));
    }

    #[test]
    fn test_consecutive_same_speaker_merged() {
        let t = transcript(&[(0.0, 1.0, "hello"), (1.2, 2.0, "world")]);
        let d = diar(&[("SPEAKER_00", 0.0, 2.0)]);
        let turns = assign_speakers(&t, &d);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].text, "hello world");
        assert!((turns[0].end - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_large_gap_not_merged() {
        let t = transcript(&[(0.0, 1.0, "first"), (5.0, 6.0, "later")]);
        let d = diar(&[("SPEAKER_00", 0.0, 6.0)]);
        let turns = assign_speakers(&t, &d);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].text, "first");
        assert_eq!(turns[1].text, "later");
    }

    #[test]
    fn test_no_diarization_flat_fallback() {
        let t = transcript(&[(0.0, 1.0, "one"), (1.2, 2.0, "two")]);
        let d = DiarizationResult::default();
        let turns = assign_speakers(&t, &d);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].speaker, None);
        assert_eq!(turns[0].text, "one two");
    }

    #[test]
    fn test_speaker_carry_forward_on_gap() {
        // Second segment has no diarization overlap; inherits last speaker.
        let t = transcript(&[(0.0, 2.0, "a"), (10.0, 12.0, "b")]);
        let d = diar(&[("SPEAKER_01", 0.0, 2.0)]);
        let turns = assign_speakers(&t, &d);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[1].speaker.as_deref(), Some("SPEAKER_01"));
    }

    #[test]
    fn test_empty_transcript() {
        let t = Transcript::default();
        let d = diar(&[("SPEAKER_00", 0.0, 1.0)]);
        assert!(assign_speakers(&t, &d).is_empty());
    }
}
