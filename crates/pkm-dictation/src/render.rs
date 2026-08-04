//! Render a diarized transcript into note-ready markdown.

use chrono::{DateTime, Local};
use pkm_stt::SpeakerTurn;

/// Metadata for the memo header block.
#[derive(Debug, Clone)]
pub struct MemoMeta<'a> {
    /// Page slug used in the recording file name.
    pub page_slug: &'a str,
    pub recorded_at: DateTime<Local>,
    /// Vault-relative path to the clip, e.g. `assets/recordings/x.flac`.
    pub clip_rel_path: &'a str,
    pub duration_secs: f64,
    pub speakers: usize,
}

/// Turn a speaker id like `SPEAKER_00` or `0` into a human label "Speaker 1".
pub fn speaker_label(id: &str) -> String {
    let num = id
        .rsplit('_')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .map(|n| n + 1)
        .unwrap_or(0);
    format!("Speaker {num}")
}

/// Render the full memo section markdown.
///
/// Structure:
/// ```markdown
/// ## 🎙️ Voice Memo — 2026-08-04 19:34
///
/// [🔊 Listen to recording](assets/recordings/...flac) · 4:32 · 2 speakers
///
/// > **Summary:** ...
///
/// **Alice:** ...
///
/// **Bob:** ...
///
/// **Related:** [[note one]], [[note two]]
///
/// #tag1 #tag2
/// ```
pub fn render_memo(
    meta: &MemoMeta<'_>,
    turns: &[SpeakerTurn],
    speaker_names: &std::collections::HashMap<String, String>,
    summary: Option<&str>,
    related: &[String],
    tags: &[String],
) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "## 🎙️ Voice Memo — {}\n\n",
        meta.recorded_at.format("%Y-%m-%d %H:%M")
    ));
    out.push_str(&format!(
        "[🔊 Listen to recording]({}) · {} · {} speaker{}\n\n",
        meta.clip_rel_path,
        crate::format_duration_secs(meta.duration_secs),
        meta.speakers.max(1),
        if meta.speakers == 1 { "" } else { "s" }
    ));

    // Summary
    if let Some(summary) = summary {
        if !summary.trim().is_empty() {
            out.push_str(&format!("> **Summary:** {}\n\n", summary.trim()));
        }
    }

    // Transcript turns
    for turn in turns {
        let text = turn.text.trim();
        if text.is_empty() {
            continue;
        }
        match &turn.speaker {
            Some(id) => {
                let name = speaker_names
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| speaker_label(id));
                out.push_str(&format!("**{name}:** {text}\n\n"));
            }
            None => {
                out.push_str(&format!("{text}\n\n"));
            }
        }
    }

    // Related notes (backlinks)
    if !related.is_empty() {
        let links = related
            .iter()
            .map(|t| format!("[[{}]]", t.trim()))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("**Related:** {links}\n\n"));
    }

    // Tags
    if !tags.is_empty() {
        let tag_line = tags
            .iter()
            .map(|t| format!("#{}", t.trim().trim_start_matches('#')))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!("{tag_line}\n"));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use pkm_stt::SpeakerTurn;
    use std::collections::HashMap;

    fn meta<'a>() -> MemoMeta<'a> {
        MemoMeta {
            page_slug: "meeting",
            recorded_at: Local.with_ymd_and_hms(2026, 8, 4, 19, 34, 0).unwrap(),
            clip_rel_path: "assets/recordings/2026-08-04_193400_meeting.flac",
            duration_secs: 272.0,
            speakers: 2,
        }
    }

    fn turns() -> Vec<SpeakerTurn> {
        vec![
            SpeakerTurn {
                speaker: Some("SPEAKER_00".into()),
                start: 0.0,
                end: 3.0,
                text: "Let's ship the voice feature.".to_string(),
            },
            SpeakerTurn {
                speaker: Some("SPEAKER_01".into()),
                start: 3.5,
                end: 6.0,
                text: "Agreed. Endpoint connection it is.".to_string(),
            },
        ]
    }

    #[test]
    fn test_speaker_label() {
        assert_eq!(speaker_label("SPEAKER_00"), "Speaker 1");
        assert_eq!(speaker_label("SPEAKER_07"), "Speaker 8");
        assert_eq!(speaker_label("0"), "Speaker 1");
        assert_eq!(speaker_label("unknown"), "Speaker 0");
    }

    #[test]
    fn test_render_full_memo() {
        let mut names = HashMap::new();
        names.insert("SPEAKER_00".to_string(), "Alice".to_string());
        let md = render_memo(
            &meta(),
            &turns(),
            &names,
            Some("Decided to ship voice dictation."),
            &["voice dictation".to_string()],
            &["meeting".to_string(), "voice".to_string()],
        );
        assert!(md.contains("## 🎙️ Voice Memo — 2026-08-04 19:34"));
        assert!(md.contains("[🔊 Listen to recording](assets/recordings/2026-08-04_193400_meeting.flac) · 4:32 · 2 speakers"));
        assert!(md.contains("> **Summary:** Decided to ship voice dictation."));
        assert!(md.contains("**Alice:** Let's ship the voice feature."));
        assert!(md.contains("**Speaker 2:** Agreed. Endpoint connection it is."));
        assert!(md.contains("**Related:** [[voice dictation]]"));
        assert!(md.contains("#meeting #voice"));
    }

    #[test]
    fn test_render_flat_no_summary_no_tags() {
        let md = render_memo(
            &meta(),
            &[SpeakerTurn {
                speaker: None,
                start: 0.0,
                end: 2.0,
                text: "Just a single speaker rambling.".to_string(),
            }],
            &HashMap::new(),
            None,
            &[],
            &[],
        );
        assert!(md.contains("· 2 speakers"));
        assert!(md.contains("Just a single speaker rambling."));
        assert!(!md.contains("**Summary:**"));
        assert!(!md.contains("**Related:**"));
        assert!(!md.contains("#meeting"));
    }

    #[test]
    fn test_render_empty_turns_keeps_header() {
        let md = render_memo(&meta(), &[], &HashMap::new(), None, &[], &[]);
        assert!(md.contains("## 🎙️ Voice Memo"));
        assert!(md.contains("🔊 Listen to recording"));
    }
}
