//! Local AI endpoint discovery and SearXNG instance pinging.
//!
//! This module probes well-known local AI servers (Ollama / LM Studio / LocalAI)
//! plus any user-configured extra host, and reports which endpoints are
//! reachable and what kind they are. It also provides a connectivity ping for
//! user-entered SearXNG instances. All the logic lives here (not in the Tauri
//! command glue) so it is fully testable against a `wiremock` server and never
//! blocks a single host for more than [`HOST_WINDOW`].

use pkm_core::{DiscoveredEndpoint, SearchPingResult};
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Per-HTTP-request timeout when probing a candidate endpoint URL.
pub const PROBE_TIMEOUT: Duration = Duration::from_millis(800);

/// Overall per-host cap so no single host can block discovery for long.
pub const HOST_WINDOW: Duration = Duration::from_millis(1000);

/// Timeout for a SearXNG instance connectivity ping.
pub const PING_TIMEOUT: Duration = Duration::from_secs(2);

/// Well-known local AI endpoints probed on every discovery run.
pub const DEFAULT_ENDPOINTS: &[(&str, &str)] = &[
    ("Ollama", "http://127.0.0.1:11434"),
    ("LM Studio", "http://127.0.0.1:1234"),
    ("LocalAI", "http://127.0.0.1:8081"),
];

/// Normalize a raw "host" entry into a canonical base URL.
///
/// Trims whitespace, prepends `http://` when no scheme is present, and strips a
/// trailing `/`. Returns an empty string for an input that is empty after
/// trimming (so callers can skip it).
pub fn normalize_host(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };
    with_scheme.trim_end_matches('/').to_string()
}

/// Extract model names from an endpoint JSON body.
///
/// Handles both the Ollama `{"models":[{"name":...}]}` shape and the
/// OpenAI-compatible `{"data":[{"id":...}]}` shape, plus a
/// `{"models":[{"id"|"name":...}]}` variant. Returns an empty vec when no
/// model name can be extracted.
pub fn parse_models(body: &serde_json::Value) -> Vec<String> {
    // OpenAI-compatible: { "data": [ { "id": "model-a" }, ... ] }
    if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        let ids: Vec<String> = data
            .iter()
            .filter_map(|m| m.get("id").and_then(|id| id.as_str()))
            .map(str::to_string)
            .collect();
        if !ids.is_empty() {
            return ids;
        }
    }
    // Ollama / generic: { "models": [ { "name": ... } | { "id": ... } ] }
    if let Some(models) = body.get("models").and_then(|m| m.as_array()) {
        return models
            .iter()
            .filter_map(|m| {
                m.get("name")
                    .or_else(|| m.get("id"))
                    .and_then(|v| v.as_str())
            })
            .map(str::to_string)
            .collect();
    }
    Vec::new()
}

/// Outcome of probing a single candidate URL.
enum ProbeOutcome {
    /// HTTP 2xx with a parseable model list.
    Success {
        models: Vec<String>,
        kind: &'static str,
        elapsed: Duration,
    },
    /// HTTP 2xx but the body had no parseable model list.
    RespondedNoModels { elapsed: Duration },
    /// A non-2xx HTTP response (the server responded, but not with this API).
    HttpStatus(u16),
    /// A transport-level error (connection refused, DNS failure, timeout...).
    Transport(String),
}

/// Probe a single candidate URL.
async fn probe_url(client: &reqwest::Client, url: &str) -> ProbeOutcome {
    let started = Instant::now();
    match client.get(url).send().await {
        Ok(resp) => {
            let elapsed = started.elapsed();
            if !resp.status().is_success() {
                return ProbeOutcome::HttpStatus(resp.status().as_u16());
            }
            let body: serde_json::Value = match resp.json().await {
                Ok(b) => b,
                Err(_) => return ProbeOutcome::RespondedNoModels { elapsed },
            };
            let models = parse_models(&body);
            if models.is_empty() {
                ProbeOutcome::RespondedNoModels { elapsed }
            } else {
                let kind = if url.ends_with("/api/tags") {
                    "ollama"
                } else {
                    "openai_compatible"
                };
                ProbeOutcome::Success {
                    models,
                    kind,
                    elapsed,
                }
            }
        }
        Err(e) => ProbeOutcome::Transport(e.to_string()),
    }
}

fn endpoint_host(base_norm: &str, base_has_v1: bool, kind: &str) -> String {
    if base_has_v1 || kind == "ollama" {
        base_norm.to_string()
    } else {
        format!("{base_norm}/v1")
    }
}

/// Probe a single local AI host and build a [`DiscoveredEndpoint`] report.
///
/// `base` must be a normalized base URL (scheme present, no trailing slash).
/// The three well-known candidate URLs are requested concurrently; the whole
/// per-host probe is bounded by [`HOST_WINDOW`] so a hung host cannot stall
/// discovery for more than ~1 second.
pub async fn probe_host(name: &str, base: &str) -> DiscoveredEndpoint {
    let client = match reqwest::Client::builder().timeout(PROBE_TIMEOUT).build() {
        Ok(c) => c,
        Err(_) => return unreachable_endpoint(name, base),
    };
    let base_norm = base.trim_end_matches('/');
    let base_has_v1 = base_norm.ends_with("/v1");
    let candidates = [
        format!("{base_norm}/api/tags"),
        format!("{base_norm}/v1/models"),
        format!("{base_norm}/models"),
    ];

    let fut = async {
        let outcomes = futures::future::join_all(candidates.iter().map(|url| {
            let client = client.clone();
            let url = url.clone();
            async move { probe_url(&client, &url).await }
        }))
        .await;
        build_endpoint(name, base_norm, base_has_v1, outcomes)
    };

    match tokio::time::timeout(HOST_WINDOW, fut).await {
        Ok(ep) => ep,
        Err(_) => unreachable_endpoint(name, base_norm),
    }
}

fn unreachable_endpoint(name: &str, base: &str) -> DiscoveredEndpoint {
    DiscoveredEndpoint {
        host: base.to_string(),
        name: name.to_string(),
        kind: "unknown".to_string(),
        models: Vec::new(),
        status: "unreachable".to_string(),
        latency_ms: 0,
        error: Some("no response".to_string()),
    }
}

fn build_endpoint(
    name: &str,
    base_norm: &str,
    base_has_v1: bool,
    outcomes: Vec<ProbeOutcome>,
) -> DiscoveredEndpoint {
    let mut first_success: Option<(Vec<String>, &'static str)> = None;
    let mut min_success_elapsed: Option<Duration> = None;
    let mut any_2xx = false;
    let mut no_models_elapsed: Option<Duration> = None;
    let mut statuses: Vec<u16> = Vec::new();
    let mut transport_err: Option<String> = None;

    for outcome in outcomes {
        match outcome {
            ProbeOutcome::Success {
                models,
                kind,
                elapsed,
            } => {
                any_2xx = true;
                if first_success.is_none() {
                    first_success = Some((models, kind));
                }
                min_success_elapsed = Some(match min_success_elapsed {
                    Some(prev) => prev.min(elapsed),
                    None => elapsed,
                });
            }
            ProbeOutcome::RespondedNoModels { elapsed } => {
                any_2xx = true;
                if no_models_elapsed.is_none() {
                    no_models_elapsed = Some(elapsed);
                }
            }
            ProbeOutcome::HttpStatus(s) => statuses.push(s),
            ProbeOutcome::Transport(e) => {
                if transport_err.is_none() {
                    transport_err = Some(e);
                }
            }
        }
    }

    if let Some((models, kind)) = first_success {
        let host = endpoint_host(base_norm, base_has_v1, kind);
        return DiscoveredEndpoint {
            host,
            name: name.to_string(),
            kind: kind.to_string(),
            models,
            status: "available".to_string(),
            latency_ms: min_success_elapsed.unwrap_or_default().as_millis() as u64,
            error: None,
        };
    }

    if any_2xx {
        // A 2xx response existed but no parseable model list — assume it is an
        // OpenAI-compatible endpoint and report it as "unknown" kind.
        return DiscoveredEndpoint {
            host: if base_has_v1 {
                base_norm.to_string()
            } else {
                format!("{base_norm}/v1")
            },
            name: name.to_string(),
            kind: "unknown".to_string(),
            models: Vec::new(),
            status: "available".to_string(),
            latency_ms: no_models_elapsed.unwrap_or_default().as_millis() as u64,
            error: None,
        };
    }

    let error = if !statuses.is_empty() {
        let status = statuses[0];
        Some(format!("HTTP {status} on /api/tags, /v1/models, /models"))
    } else {
        transport_err.or_else(|| Some("no response".to_string()))
    };

    DiscoveredEndpoint {
        host: base_norm.to_string(),
        name: name.to_string(),
        kind: "unknown".to_string(),
        models: Vec::new(),
        status: "unreachable".to_string(),
        latency_ms: 0,
        error,
    }
}

/// Discover local AI endpoints.
///
/// Merges [`DEFAULT_ENDPOINTS`] with the user-provided `extra_hosts` (deduped by
/// normalized host, with the defaults always first), probes every host
/// concurrently, and returns one [`DiscoveredEndpoint`] per host in the same
/// order. Never panics and never blocks longer than ~[`HOST_WINDOW`] per host.
pub async fn discover_local_ai(extra_hosts: &[String]) -> Vec<DiscoveredEndpoint> {
    let mut hosts: Vec<(String, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for (name, url) in DEFAULT_ENDPOINTS {
        let norm = normalize_host(url);
        if norm.is_empty() || !seen.insert(norm.clone()) {
            continue;
        }
        hosts.push((name.to_string(), norm));
    }
    for raw in extra_hosts {
        let norm = normalize_host(raw);
        if norm.is_empty() || !seen.insert(norm.clone()) {
            continue;
        }
        // Extra hosts have no built-in friendly name; use the host itself.
        hosts.push((norm.clone(), norm));
    }

    futures::future::join_all(hosts.iter().map(|(name, base)| probe_host(name, base))).await
}

/// Ping a SearXNG instance to test reachability.
///
/// Normalizes the URL (prepending `https://` when no scheme is present),
/// validates it with `pkm_core::endpoint::validate_endpoint_safe` (so plain
/// `http://` to external hosts is rejected), then issues a `GET
/// {url}/search?q=stratum&format=json`. Any HTTP status below 500 (including
/// 429 rate-limiting) counts as reachable.
pub async fn ping_search_instance(url: &str) -> SearchPingResult {
    let trimmed = url.trim();
    let raw = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let normalized = raw.trim_end_matches('/').to_string();

    if let Err(e) = pkm_core::endpoint::validate_endpoint_safe(&normalized) {
        return SearchPingResult {
            ok: false,
            latency_ms: 0,
            error: Some(e.to_string()),
        };
    }

    let ping_url = format!("{normalized}/search?q=stratum&format=json");
    let started = Instant::now();
    let client = match reqwest::Client::builder().timeout(PING_TIMEOUT).build() {
        Ok(c) => c,
        Err(e) => {
            return SearchPingResult {
                ok: false,
                latency_ms: 0,
                error: Some(e.to_string()),
            }
        }
    };

    match client.get(&ping_url).send().await {
        Ok(resp) => {
            let elapsed = started.elapsed().as_millis() as u64;
            let ok = resp.status().as_u16() < 500;
            SearchPingResult {
                ok,
                latency_ms: elapsed,
                error: if ok {
                    None
                } else {
                    Some(format!("HTTP {}", resp.status().as_u16()))
                },
            }
        }
        Err(e) => SearchPingResult {
            ok: false,
            latency_ms: 0,
            error: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_discover_ollama_tags() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": [
                    { "name": "llama3.2:latest" },
                    { "name": "nomic-embed-text" }
                ]
            })))
            .mount(&server)
            .await;

        let ep = probe_host("Ollama", &server.uri()).await;
        assert_eq!(ep.status, "available");
        assert_eq!(ep.kind, "ollama");
        assert_eq!(ep.models, vec!["llama3.2:latest", "nomic-embed-text"]);
        assert_eq!(ep.host, server.uri());
    }

    #[tokio::test]
    async fn test_discover_openai_compatible() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "object": "list",
                "data": [
                    { "id": "model-a" },
                    { "id": "model-b" }
                ]
            })))
            .mount(&server)
            .await;

        let ep = probe_host("LM Studio", &server.uri()).await;
        assert_eq!(ep.kind, "openai_compatible");
        assert_eq!(ep.host, format!("{}/v1", server.uri()));
        assert_eq!(ep.models, vec!["model-a", "model-b"]);
    }

    #[tokio::test]
    async fn test_discover_unknown_server() {
        let server = MockServer::start().await;
        // No mounts -> wiremock returns 404 for all candidate paths.
        let ep = probe_host("Unknown", &server.uri()).await;
        assert_eq!(ep.status, "unreachable");
        let err = ep.error.expect("unreachable must carry an error");
        assert!(err.contains("404"), "error should mention 404, got: {err}");
    }

    #[tokio::test]
    async fn test_discover_unreachable_no_server() {
        // Bind a listener, grab its port, then drop it so the port is closed.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);

        let started = Instant::now();
        let ep = probe_host("Gone", &format!("http://127.0.0.1:{port}")).await;
        let elapsed = started.elapsed();
        assert_eq!(ep.status, "unreachable");
        assert!(elapsed < Duration::from_millis(1500));
        assert!(ep.error.is_some());
    }

    #[tokio::test]
    async fn test_discover_extra_host_and_dedupe() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "object": "list",
                "data": [{ "id": "extra-model" }]
            })))
            .mount(&server)
            .await;
        let port = server.uri().rsplit(':').next().unwrap().to_string();
        let host = format!("127.0.0.1:{port}");

        let results = discover_local_ai(&[
            host.clone(),
            String::new(),            // empty -> skipped
            format!("http://{host}"), // dup -> skipped
        ])
        .await;

        assert_eq!(results.len(), 4);
        assert_eq!(results[0].name, "Ollama");
        assert_eq!(results[1].name, "LM Studio");
        assert_eq!(results[2].name, "LocalAI");
        let extra = &results[3];
        assert_eq!(extra.status, "available");
        assert_eq!(extra.kind, "openai_compatible");
        assert_eq!(extra.name, format!("http://{host}"));
    }

    #[test]
    fn test_normalize_host() {
        assert_eq!(normalize_host("127.0.0.1:1234"), "http://127.0.0.1:1234");
        assert_eq!(normalize_host("http://x/"), "http://x");
        assert_eq!(normalize_host(" https://y "), "https://y");
    }

    #[tokio::test]
    async fn test_ping_search_instance_ok() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search"))
            .and(query_param("q", "stratum"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "results": [] })))
            .mount(&server)
            .await;

        let result = ping_search_instance(&server.uri()).await;
        assert!(result.ok);
        // Mock server may answer in <1ms; latency is only meaningful as >= 0.
        assert!(result.latency_ms >= 0);
    }

    #[tokio::test]
    async fn test_ping_rate_limited_ok() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let result = ping_search_instance(&server.uri()).await;
        // 429 counts as reachable per spec.
        assert!(result.ok);
    }

    #[tokio::test]
    async fn test_ping_external_http_rejected() {
        // Plain http to an external (public) host is rejected by validation.
        let result = ping_search_instance("http://example.com").await;
        assert!(!result.ok);
        let err = result.error.as_ref().expect("validation error expected");
        assert!(err.contains("not allowed") || err.contains("HTTP URL"));
    }

    #[tokio::test]
    async fn test_ping_unreachable() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);
        let result = ping_search_instance(&format!("http://127.0.0.1:{port}")).await;
        assert!(!result.ok);
        assert!(result.error.is_some());
    }
}
