use super::*;

#[test]
fn extracts_a_mermaid_fence() {
    let body = mermaid_source("```mermaid\nflowchart LR\nA-->B\n```").expect("mermaid");
    assert_eq!(body, "flowchart LR\nA-->B");
    assert!(mermaid_source("```rust\nfn main() {}\n```").is_none());
    assert!(mermaid_source("plain").is_none());
}

#[test]
fn renders_valid_mermaid_to_svg() {
    let mut cache = MermaidCache::new();
    let svg = cache
        .render("flowchart LR; A-->B", "modern")
        .expect("render");
    assert!(svg.contains("<svg"));
}

#[test]
fn invalid_mermaid_is_a_render_error() {
    let mut cache = MermaidCache::new();
    let error = cache
        .render("flowchart LR; A -->", "modern")
        .expect_err("invalid mermaid must fail");
    assert!(matches!(error, PkmError::Render(_)));
}

#[test]
fn cache_hits_misses_and_evicts() {
    let mut cache = MermaidCache::with_capacity(2);
    cache
        .render("flowchart LR; A-->B", "modern")
        .expect("first");
    cache
        .render("flowchart LR; A-->B", "modern")
        .expect("cached");
    assert_eq!(cache.hits(), 1);
    assert_eq!(cache.misses(), 1);

    cache
        .render("flowchart LR; B-->C", "modern")
        .expect("second");
    cache
        .render("flowchart LR; C-->D", "modern")
        .expect("third");
    assert!(cache.len() <= 2);
    assert_eq!(cache.misses(), 3);
}

#[test]
fn theme_is_part_of_the_cache_key() {
    assert_ne!(
        cache_key("flowchart LR; A-->B", "modern"),
        cache_key("flowchart LR; A-->B", "dark")
    );
}
