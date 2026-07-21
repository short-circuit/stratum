use crate::provider::{ChatConfig, ChatMessage, LlmProvider, ProviderFactory};
use pkm_core::PkmResult;
use pkm_core::validate_endpoint_safe;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Result of a web research session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResult {
    pub findings: String,
    pub sources: Vec<ResearchSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSource {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Multi-step autonomous web research engine using SearXNG.
pub struct ResearchEngine {
    searxng_endpoint: String,
    max_results: usize,
    max_depth: u32,
    llm: Box<dyn LlmProvider>,
    model: String,
}

impl ResearchEngine {
    pub fn new(
        searxng_endpoint: String,
        max_results: usize,
        max_depth: u32,
        ai_config: &pkm_core::AiConfig,
    ) -> PkmResult<Self> {
        let llm = ProviderFactory::create(ai_config)?;
        let model = ai_config.model.clone();
        Ok(Self {
            searxng_endpoint,
            max_results,
            max_depth,
            llm,
            model,
        })
    }

    /// Run multi-step autonomous research on a topic.
    pub async fn research(&self, query: &str) -> PkmResult<ResearchResult> {
        let mut all_sources: Vec<ResearchSource> = Vec::new();
        let mut context = String::new();
        let mut queries = vec![query.to_string()];
        let mut depth = 0u32;

        while depth < self.max_depth && !queries.is_empty() {
            let mut new_queries: Vec<String> = Vec::new();
            let mut round_sources: Vec<ResearchSource> = Vec::new();
            let mut round_context = String::new();

            for q in &queries {
                let results = self.search_searxng(q).await?;
                for r in results.iter().take(self.max_results) {
                    let page_content = self.read_url(&r.url).await.unwrap_or(r.content.clone());
                    let chars: Vec<char> = page_content.chars().collect();
                    let snippet: String = chars.iter().take(500).collect();
                    let body: String = chars.iter().take(3000).collect();

                    round_sources.push(ResearchSource {
                        title: r.title.clone(),
                        url: r.url.clone(),
                        snippet: if chars.len() > 500 {
                            format!("{}...", snippet)
                        } else {
                            snippet
                        },
                    });

                    round_context.push_str(&format!(
                        "\n[Source {}]\nTitle: {}\nURL: {}\nContent:\n{}\n",
                        all_sources.len() + round_sources.len(),
                        r.title,
                        r.url,
                        body
                    ));
                }
            }

            all_sources.extend(round_sources);
            context.push_str(&round_context);

            if depth + 1 < self.max_depth {
                let analysis = self.analyze_progress(query, &context, depth).await?;
                new_queries = analysis.next_queries;

                if !analysis.should_continue {
                    break;
                }
            }

            queries = new_queries;
            depth += 1;
        }

        let findings = self.synthesize(query, &context).await?;

        Ok(ResearchResult {
            findings,
            sources: all_sources,
        })
    }

    /// Search SearXNG and return results.
    async fn search_searxng(&self, query: &str) -> PkmResult<Vec<SearxngResult>> {
        // Validate the SearXNG endpoint against SSRF attacks
        validate_endpoint_safe(&self.searxng_endpoint)?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| pkm_core::PkmError::Ai(format!("HTTP client error: {e}")))?;
        let url = format!(
            "{}/search?q={}&format=json&categories=general",
            self.searxng_endpoint.trim_end_matches('/'),
            urlencoding(query)
        );

        let resp = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| pkm_core::PkmError::Ai(format!("SearXNG request failed: {e}")))?;

        let body: SearxngResponse = resp
            .json()
            .await
            .map_err(|e| pkm_core::PkmError::Ai(format!("SearXNG parse error: {e}")))?;

        Ok(body.results)
    }

    /// Read a webpage and extract text content.
    async fn read_url(&self, url: &str) -> PkmResult<String> {
        // Validate each URL fetched from search results against SSRF attacks
        validate_endpoint_safe(url)?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("StratumPKM/1.0 Research Bot")
            .build()
            .map_err(|e| pkm_core::PkmError::Ai(format!("HTTP client error: {e}")))?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| pkm_core::PkmError::Ai(format!("Failed to fetch {url}: {e}")))?;

        let html = resp
            .text()
            .await
            .map_err(|e| pkm_core::PkmError::Ai(format!("Failed to read {url}: {e}")))?;

        // Simple HTML-to-text extraction
        let text = strip_html(&html);
        // Clean up whitespace
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");

        Ok(text)
    }

    /// Ask the LLM to analyze research progress and decide next steps.
    async fn analyze_progress(
        &self,
        query: &str,
        context: &str,
        _depth: u32,
    ) -> PkmResult<ResearchAnalysis> {
        let system = format!(
            "You are a research analyst. You are researching: {query}\n\n\
             You have gathered the following context so far:\n{context}\n\n\
             Analyze your progress. Decide if you need more information and what to search for next.\n\
             Return your response as JSON with fields:\n\
             - should_continue: boolean (true if more research needed)\n\
             - next_queries: array of strings (up to 3 search queries for the next round)\n\
             - reasoning: brief explanation"
        );

        let messages = vec![ChatMessage::user(&system)];

        let config = ChatConfig::new(&self.model)
            .with_temperature(0.3)
            .with_max_tokens(1024)
            .with_system_prompt(
                "You are a research analyst. Output ONLY valid JSON, no other text.",
            );

        let response = self.llm.chat(&messages, &config).await?;

        // Try to parse JSON from the response
        let content = response.content.trim();
        let json_str = content
            .strip_prefix("```json")
            .or_else(|| content.strip_prefix("```"))
            .map(|s| s.trim_end_matches("```").trim())
            .unwrap_or(content);

        if let Ok(analysis) = serde_json::from_str::<ResearchAnalysis>(json_str) {
            Ok(ResearchAnalysis {
                should_continue: analysis.should_continue,
                next_queries: analysis.next_queries.into_iter().take(3).collect(),
            })
        } else {
            // Default: stop if we can't parse
            Ok(ResearchAnalysis {
                should_continue: false,
                next_queries: vec![],
            })
        }
    }

    /// Synthesize all gathered context into structured notes.
    async fn synthesize(&self, query: &str, context: &str) -> PkmResult<String> {
        let system = format!(
            "You are a research writer. Based on the following research about \"{query}\", \
             write comprehensive, well-structured notes.\n\n\
             Research context:\n{context}\n\n\
             Format your notes with:\n\
             - ## Main heading with the topic\n\
             - ### Subheadings for different aspects\n\
             - Bullet points for key facts\n\
             - [[wiki-links]] for important concepts\n\
             - [Source N] citations where appropriate\n\n\
             Write in a clear, informative style suitable for a personal knowledge base."
        );

        let messages = vec![ChatMessage::user(&system)];
        let config = ChatConfig::new(&self.model)
            .with_temperature(0.4)
            .with_max_tokens(4096);

        let response = self.llm.chat(&messages, &config).await?;
        Ok(response.content)
    }
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            other => format!("%{:02X}", other as u8),
        })
        .collect()
}

fn strip_html(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if !in_tag {
            if chars[i] == '<' {
                in_tag = true;
                let rest9: String = chars[i..].iter().take(9).collect();
                if rest9.starts_with("</script") {
                    in_script = false;
                } else if rest9.starts_with("</style") {
                    in_style = false;
                } else {
                    let rest7: String = chars[i..].iter().take(7).collect();
                    if rest7.starts_with("<script") {
                        in_script = true;
                    } else if rest7.starts_with("<style") {
                        in_style = true;
                    }
                }
                i += 1;
                continue;
            }
            if !in_script && !in_style {
                result.push(chars[i]);
            }
            i += 1;
        } else {
            if chars[i] == '>' {
                in_tag = false;
                i += 1;
                continue;
            }
            i += 1;
        }
    }

    // Decode HTML entities
    result = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    result
}

#[derive(Debug, Deserialize)]
struct SearxngResponse {
    results: Vec<SearxngResult>,
}

#[derive(Debug, Deserialize)]
struct SearxngResult {
    title: String,
    url: String,
    #[serde(default)]
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResearchAnalysis {
    should_continue: bool,
    next_queries: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html() {
        let html = "<p>Hello <b>world</b></p>";
        assert_eq!(strip_html(html), "Hello world");
    }

    #[test]
    fn test_strip_html_with_script() {
        let html = "<p>Hello</p><script>alert('xss')</script><p>world</p>";
        assert_eq!(strip_html(html), "Helloworld");
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(urlencoding("a&b"), "a%26b");
    }

    #[test]
    fn test_research_analysis_serde() {
        let json = r#"{"should_continue": true, "next_queries": ["test query"], "reasoning": "need more"}"#;
        let analysis: ResearchAnalysis = serde_json::from_str(json).unwrap();
        assert!(analysis.should_continue);
        assert_eq!(analysis.next_queries, vec!["test query"]);
    }
}
