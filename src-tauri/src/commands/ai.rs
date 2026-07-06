use crate::commands::vault::AppState;
use pkm_ai::provider::{ChatConfig, ChatMessage, ProviderFactory};
use pkm_ai::research::ResearchEngine;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct AiTransformResult {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiAction {
    Rewrite,
    Format,
    Structure,
    Summarize,
    Connect,
    Mermaid,
}

fn system_prompt_for(action: &AiAction) -> &'static str {
    match action {
        AiAction::Rewrite => {
            "You are a writing assistant. Rewrite the following text to improve clarity, flow, and \
             readability while preserving the original meaning and key information. \
             Return ONLY the rewritten text, no explanations or metadata."
        }
        AiAction::Format => {
            "You are a formatting assistant. Clean up the following notes: fix markdown syntax, \
             ensure consistent heading levels, add proper spacing, and make the text well-formatted. \
             Return ONLY the formatted text, no explanations or metadata."
        }
        AiAction::Structure => {
            "You are a note organization assistant. Organize the following notes into a clear \
             hierarchical structure: group related content under appropriate headings, create \
             consistent sections, and improve the overall organization. Use markdown headings (##, ###). \
             Return ONLY the structured notes, no explanations or metadata."
        }
        AiAction::Summarize => {
            "You are a summarization assistant. Summarize the following text concisely while \
             preserving the key points and important details. Return ONLY the summary, \
             no explanations or metadata."
        }
        AiAction::Connect => {
            "You are a knowledge connection assistant. Analyze the following notes and suggest \
             [[wiki-links]] to related concepts, topics, or notes that would create meaningful \
             connections. Add relevant wiki-links inline where they make sense contextually. \
             Return the text with wiki-links added, no explanations or metadata."
        }
        AiAction::Mermaid => {
            "You are a diagram generation assistant. Generate a Mermaid.js diagram based on the \
             user's description. Return ONLY the raw mermaid code without markdown fences, \
             without explanation, and without any surrounding text. \
             Use appropriate diagram types: graph (flowchart), sequenceDiagram, classDiagram, \
             stateDiagram, gantt, pie, erDiagram, or journey.\n\n\
             IMPORTANT SYNTAX RULES:\n\
             - Never use parentheses () inside node labels — use commas or dashes instead. \
             Parentheses have special meaning in mermaid (rounded nodes).\n\
             - Never use curly braces {} inside node labels — they create rhombus/diamond nodes.\n\
             - Keep labels short and simple. For example, write `Component Library` instead of \
             `Component Library (e.g., Material UI)`.\n\
             - Use proper node shapes: A[text] for rectangle, A{text} for diamond, \
             A(text) for rounded, A>text] for asymmetric."
        }
    }
}

#[tauri::command]
pub async fn ai_transform_block(
    text: String,
    action: AiAction,
    page_path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<AiTransformResult, String> {
    info!(
        "transform_block action={:?} text_len={} page={:?}",
        &action,
        text.len(),
        &page_path
    );

    let config_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.join(".pkm").join("config.toml")
    };

    let config = if config_path.exists() {
        pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?
    } else {
        return Err("AI not configured. Please configure AI provider in Settings.".into());
    };

    let provider = ProviderFactory::create(&config.ai).map_err(|e| e.to_string())?;

    let system_prompt = system_prompt_for(&action);

    let context_suffix = if let Some(ref path) = page_path {
        let is_journal = path.starts_with("journals/");
        if is_journal {
            " This is a daily journal entry. Organize it like a structured daily log with clear \
             sections for different topics or events."
        } else {
            " This is a knowledge base page. Use appropriate structure for a reference note."
        }
    } else {
        ""
    };

    let full_system = format!("{}{}", system_prompt, context_suffix);

    debug!(
        "calling provider={:?} model={}",
        config.ai.provider, config.ai.model
    );

    let chat_config = ChatConfig::new(&config.ai.model)
        .with_temperature(0.3)
        .with_system_prompt(full_system);

    let messages = vec![ChatMessage::user(&text)];

    let response = provider.chat(&messages, &chat_config).await.map_err(|e| {
        error!("provider error: {}", e);
        e.to_string()
    })?;

    debug!("response len={}", response.content.len());

    Ok(AiTransformResult {
        content: response.content,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResearchResultDto {
    pub findings: String,
    pub sources: Vec<ResearchSourceDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResearchSourceDto {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[tauri::command]
pub async fn ai_research(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<ResearchResultDto, String> {
    info!("research query={}", query);

    let config_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.join(".pkm").join("config.toml")
    };

    let config = if config_path.exists() {
        pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?
    } else {
        return Err("AI not configured. Please configure AI provider in Settings.".into());
    };

    debug!(
        "research searxng={} max_results={} max_depth={}",
        config.research.searxng_endpoint, config.research.max_results, config.research.max_depth
    );

    let engine = ResearchEngine::new(
        config.research.searxng_endpoint.clone(),
        config.research.max_results,
        config.research.max_depth,
        &config.ai,
    )
    .map_err(|e| format!("Failed to create research engine: {e}"))?;

    let result = engine
        .research(&query)
        .await
        .map_err(|e| format!("Research failed: {e}"))?;

    debug!(
        "research done, findings_len={} sources={}",
        result.findings.len(),
        result.sources.len()
    );

    Ok(ResearchResultDto {
        findings: result.findings,
        sources: result
            .sources
            .into_iter()
            .map(|s| ResearchSourceDto {
                title: s.title,
                url: s.url,
                snippet: s.snippet,
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn generate_mermaid(
    prompt: String,
    state: tauri::State<'_, AppState>,
) -> Result<AiTransformResult, String> {
    info!("generate_mermaid prompt_len={}", prompt.len());

    let config_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.join(".pkm").join("config.toml")
    };

    let config = if config_path.exists() {
        pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?
    } else {
        return Err("AI not configured. Please configure AI provider in Settings.".into());
    };

    let provider = ProviderFactory::create(&config.ai).map_err(|e| e.to_string())?;

    let system_prompt = system_prompt_for(&AiAction::Mermaid);

    let chat_config = ChatConfig::new(&config.ai.model)
        .with_temperature(0.3)
        .with_system_prompt(system_prompt);

    let messages = vec![ChatMessage::user(&prompt)];

    let response = provider.chat(&messages, &chat_config).await.map_err(|e| {
        error!("provider error: {}", e);
        e.to_string()
    })?;

    debug!("mermaid response len={}", response.content.len());

    Ok(AiTransformResult {
        content: response.content,
    })
}

#[tauri::command]
pub async fn ai_interlink_notes(
    text: String,
    page_path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<AiTransformResult, String> {
    info!(
        "interlink text_len={} page={:?}",
        text.len(),
        page_path
    );

    let config_path;
    let db_path;
    let index_path;
    {
        let s = state.lock().map_err(|e| e.to_string())?;
        config_path = s.vault_path.join(".pkm").join("config.toml");
        db_path = s.db_path.clone();
        index_path = s.vault_path.join(".pkm").join("search");
    }

    let config = if config_path.exists() {
        pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?
    } else {
        return Err("AI not configured.".into());
    };

    let current_slug = page_path.as_ref().and_then(|p| {
        std::path::Path::new(p)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    });
    debug!("interlink current_slug={:?}", current_slug);

    // Step 1: ensure Tantivy index is built, then search
    let mut related_titles: Vec<String> = Vec::new();

    // Build index from block store, then search
    if let Ok(store) = pkm_block::BlockStore::open(&db_path) {
        if let Ok(pages) = store.list_pages() {
            if let Ok(mut idx) = pkm_index::block_search::BlockIndex::create(&index_path) {
                for path in &pages {
                    if let Ok(blocks) = store.get_blocks_by_page(path) {
                        for b in &blocks {
                            let _ = idx.index_block(b, path);
                        }
                    }
                }
                let _ = idx.flush();
            }
            // Search using Tantivy full-text
            let query_words: Vec<&str> = text
                .split(|c: char| c.is_whitespace() || c == '#' || c == '*' || c == '[' || c == ']')
                .filter(|w| w.len() > 3)
                .collect();
            let query = query_words.join(" ");

            if let Ok(idx) = pkm_index::block_search::BlockIndex::create(&index_path) {
                if let Ok(results) = idx.search(&query, 20) {
                    let mut seen = std::collections::HashSet::new();
                    for r in &results {
                        let page = r.page_path.trim_end_matches(".md");
                        let slug = std::path::Path::new(page)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");
                        if let Some(ref cur) = current_slug {
                            if slug == cur {
                                continue;
                            }
                        }
                        if seen.insert(page.to_string()) {
                            related_titles.push(slug.replace('-', " "));
                        }
                    }
                }
            }

            // Step 2: supplement with keyword matching for any remaining pages
            let keywords: std::collections::HashSet<String> =
                query_words.iter().map(|w| w.to_lowercase()).collect();

            let found: std::collections::HashSet<_> = related_titles.iter().cloned().collect();
            for path in &pages {
                let slug = std::path::Path::new(path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(path);
                if let Some(ref cur) = current_slug {
                    if slug == cur {
                        continue;
                    }
                }
                let title = slug.replace('-', " ");
                if found.contains(&title) {
                    continue;
                }

                let title_lower = title.to_lowercase();
                let mut score: usize =
                    keywords.iter().filter(|k| title_lower.contains(*k)).count() * 3;
                if let Ok(blocks) = store.get_blocks_by_page(path) {
                    for b in &blocks {
                        let block_lower = b.content.to_lowercase();
                        score += keywords.iter().filter(|k| block_lower.contains(*k)).count();
                    }
                }
                if score > 0 {
                    related_titles.push(title);
                }
            }
        }
    }

    debug!("found {} related pages", related_titles.len());

    if related_titles.is_empty() {
        warn!("no related pages found, returning original text");
        return Ok(AiTransformResult { content: text });
    }

    let provider = ProviderFactory::create(&config.ai).map_err(|e| e.to_string())?;

    let notes_list = format!(
        "\n\nExisting notes in your vault that you can link to:\n{}",
        related_titles
            .iter()
            .map(|t| format!("- [[{}]]", t))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let system = format!(
        "You are a knowledge connection assistant. Analyze the markdown text and add [[wiki-links]] \
         to genuinely related notes from the vault list below.\n\n\
         RULES:\n\
         - Only link a word/phrase if IT IS the main topic of the target note, not just a word match.\n\
         - Do NOT link incidental mentions or passing references.\n\
         - Aim for 1-3 high-quality links total, not one per keyword.\n\
         - Preserve ALL existing markdown formatting exactly as-is.\n\
         - Return ONLY the markdown with wiki-links added. No explanations.\n\n\
         Available notes to link to:{notes_list}"
    );

    let chat_config = ChatConfig::new(&config.ai.model)
        .with_temperature(0.2)
        .with_system_prompt(system);

    let messages = vec![ChatMessage::user(&text)];
    let response = provider
        .chat(&messages, &chat_config)
        .await
        .map_err(|e| e.to_string())?;

    debug!("interlink response len={}", response.content.len());

    // Post-process: strip self-links
    let mut result = response.content;
    if let Some(ref slug) = current_slug {
        let self_link = format!("[[{}]]", slug);
        // Replace case-insensitive self-links with just the text
        if let Some(pos) = result.to_lowercase().find(&self_link.to_lowercase()) {
            let end = pos + self_link.len();
            let before = &result[..pos];
            let after = &result[end..];
            result = format!("{}{}{}", before, slug, after);
            debug!("stripped self-link from response");
        }
        // Also handle piped links like [[homelab|something]]
        let piped = format!("[[{}|", slug);
        if let Some(pos) = result.to_lowercase().find(&piped.to_lowercase()) {
            let close = result[pos..]
                .find("]]")
                .map(|p| pos + p + 2)
                .unwrap_or(result.len());
            let before = &result[..pos];
            let after = &result[close..];
            // Extract display text after the pipe
            let inner = &result[pos + piped.len()..close - 2];
            result = format!("{}{}{}", before, inner, after);
            debug!("stripped piped self-link from response");
        }
    }

    Ok(AiTransformResult { content: result })
}
