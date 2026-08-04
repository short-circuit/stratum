//! LLM enrichment for dictation memos: summary, related notes, tags.
//!
//! Reuses the existing `pkm_ai::provider::LlmProvider` abstraction — the
//! same provider configured in Settings powers these calls.

use pkm_ai::provider::{ChatConfig, ChatMessage, LlmProvider, Role};
use pkm_core::{PkmError, PkmResult};

/// Temperature for deterministic extraction prompts.
const TEMPERATURE: f32 = 0.2;

async fn chat(llm: &dyn LlmProvider, model: &str, system: &str, user: &str) -> PkmResult<String> {
    let config = ChatConfig::new(model)
        .with_temperature(TEMPERATURE)
        .with_system_prompt(system);
    let messages = vec![ChatMessage::new(Role::User, user)];
    let response = llm
        .chat(&messages, &config)
        .await
        .map_err(|e| PkmError::Ai(format!("LLM enrichment failed: {e}")))?;
    Ok(response.content.trim().to_string())
}

/// Summarize a diarized transcript into a short paragraph.
pub async fn summarize(
    llm: &dyn LlmProvider,
    model: &str,
    transcript_text: &str,
) -> PkmResult<String> {
    let system = "You are a summarization assistant. Summarize the following diarized \
                  transcript into a concise paragraph (2-4 sentences) capturing the key \
                  points, decisions and action items. Do NOT mention speaker labels. \
                  Return ONLY the summary text, no explanations or metadata.";
    chat(llm, model, system, transcript_text).await
}

/// Ask the LLM which existing notes relate to the transcript.
/// Returns note titles (without `[[ ]]`), max 3.
pub async fn suggest_related(
    llm: &dyn LlmProvider,
    model: &str,
    transcript_text: &str,
    candidates: &[String],
) -> PkmResult<Vec<String>> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    let list = candidates
        .iter()
        .map(|c| format!("- [[{}]]", c.trim()))
        .collect::<Vec<_>>()
        .join("\n");
    let system = format!(
        "You are a knowledge connection assistant. From the transcript below, pick at most \
         3 notes from the vault list that are genuinely related — the transcript must \
         actually discuss the note's main topic, not just share a keyword.\n\n\
         RULES:\n\
         - Return ONLY wiki-links, one per line, no explanations, no bullets.\n\
         - Never invent notes that are not in the list.\n\
         - Return nothing if none are genuinely related.\n\n\
         Available notes:\n{list}"
    );
    let out = chat(llm, model, &system, transcript_text).await?;
    Ok(parse_wiki_link_lines(&out))
}

/// Ask the LLM which existing tags apply to the transcript (max 3).
pub async fn suggest_tags(
    llm: &dyn LlmProvider,
    model: &str,
    transcript_text: &str,
    existing: &[String],
) -> PkmResult<Vec<String>> {
    if existing.is_empty() {
        return Ok(Vec::new());
    }
    let list = existing.join(", ");
    let system = format!(
        "You are a note tagging assistant. Assign 1-3 tags from the existing tag list that \
         genuinely apply to the transcript. Do NOT invent new tags.\n\n\
         Existing tags: {list}\n\n\
         Return ONLY a JSON array of strings, e.g. [\"meeting\", \"voice\"]. No explanations."
    );
    let out = chat(llm, model, &system, transcript_text).await?;
    Ok(parse_tag_json(&out))
}

/// Parse lines of `[[wiki-link]]` (or `[[link|display]]`) entries.
pub fn parse_wiki_link_lines(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in s.lines() {
        let line = line.trim();
        for cap in line.match_indices("[[") {
            let start = cap.0;
            let rest = &line[start + 2..];
            if let Some(end_rel) = rest.find("]]") {
                let inner = &rest[..end_rel];
                let target = inner.split('|').next().unwrap_or("").trim();
                if !target.is_empty() && !out.iter().any(|t| t == target) {
                    out.push(target.to_string());
                }
            }
        }
    }
    out.truncate(3);
    out
}

/// Parse a JSON array of tag strings, tolerating markdown fences and junk.
pub fn parse_tag_json(s: &str) -> Vec<String> {
    let cleaned = s
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(cleaned) else {
        return Vec::new();
    };
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };
    let mut out: Vec<String> = arr
        .iter()
        .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .collect();
    out.truncate(3);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wiki_link_lines() {
        assert_eq!(
            parse_wiki_link_lines("[[rust notes]]\n[[homelab|Home Lab]]\n- [[meeting]]"),
            vec!["rust notes", "homelab", "meeting"]
        );
        assert_eq!(parse_wiki_link_lines("no links here"), Vec::<String>::new());
        assert_eq!(
            parse_wiki_link_lines("[[a]] [[a]] [[b]]"),
            vec!["a", "b"],
            "dedupe + dedupe order"
        );
        let many = (0..6).map(|i| format!("[[n{i}]]")).collect::<Vec<_>>().join("\n");
        assert_eq!(parse_wiki_link_lines(&many).len(), 3, "capped at 3");
    }

    #[test]
    fn test_parse_tag_json() {
        assert_eq!(parse_tag_json("[\"meeting\", \"voice\"]"), vec!["meeting", "voice"]);
        assert_eq!(
            parse_tag_json("```json\n[\"a\", \"b\", \"c\", \"d\"]\n```"),
            vec!["a", "b", "c"],
            "fences stripped, capped at 3"
        );
        assert_eq!(parse_tag_json("nothing"), Vec::<String>::new());
        assert_eq!(parse_tag_json("{}"), Vec::<String>::new());
        assert_eq!(parse_tag_json("[\"a\", 42]"), vec!["a"], "non-strings ignored");
    }
}
