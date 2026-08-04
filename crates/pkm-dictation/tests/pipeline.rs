//! End-to-end pipeline test with a mocked STT endpoint (wiremock) and a
//! scripted LLM provider. No network, no models, no Tauri.

use async_trait::async_trait;
use chrono::{DateTime, Local};
use futures::stream::BoxStream;
use pkm_ai::provider::{ChatConfig, ChatMessage, ChatResponse, LlmProvider, TokenUsage};
use pkm_block::BlockStore;
use pkm_core::{PkmError, PkmResult};
use pkm_dictation::speakers::{SpeakerEntry, SpeakerRegistry};
use pkm_dictation::{run, Pipeline, PipelineOptions, Stage};
use pkm_stt::SttEndpoint;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Scripted LLM: returns canned answers depending on the system prompt.
#[derive(Debug)]
struct MockLlm {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl LlmProvider for MockLlm {
    async fn chat(
        &self,
        _messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> PkmResult<ChatResponse> {
        let _ = self.calls.fetch_add(1, Ordering::SeqCst);
        let system = config.system_prompt.as_deref().unwrap_or("");
        let content = if system.contains("summarization assistant") {
            "Decided to ship voice dictation with diarization."
        } else if system.contains("knowledge connection assistant") {
            "[[voice dictation]]\n[[rust notes]]"
        } else if system.contains("tagging assistant") {
            "[\"meeting\", \"voice\"]"
        } else {
            "unknown prompt"
        };
        Ok(ChatResponse {
            content: content.to_string(),
            usage: TokenUsage::default(),
        })
    }

    async fn stream_chat(
        &self,
        _messages: &[ChatMessage],
        _config: &ChatConfig,
    ) -> PkmResult<BoxStream<'static, PkmResult<pkm_ai::provider::ChatDelta>>> {
        Err(PkmError::Unsupported("not used in tests".into()))
    }
}

fn fixture_vault() -> (tempfile::TempDir, BlockStore) {
    let dir = tempdir().unwrap();
    let vault = dir.path();

    // two existing pages with tags + one matching the transcript keywords
    for (rel, content) in [
        ("pages/rust-notes.md", "Notes about #rust development.\n"),
        ("pages/homelab.md", "Homelab #homelab stuff.\n"),
        ("pages/meeting.md", "meeting notes\n"),
        ("pages/voice-dictation.md", "Voice feature design and audio notes.\n"),
    ] {
        let p = vault.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, content).unwrap();
    }
    std::fs::create_dir_all(vault.join(".pkm")).unwrap();
    let store = BlockStore::open(&vault.join(".pkm/blocks.db")).unwrap();
    for (rel, content) in [
        ("pages/rust-notes.md", "rust notes"),
        ("pages/homelab.md", "homelab stuff"),
        ("pages/meeting.md", "meeting notes"),
        ("pages/voice-dictation.md", "Voice feature design and audio notes"),
    ] {
        store
            .insert_block(&pkm_block::Block::new(uuid::Uuid::new_v4(), content.to_string()), rel)
            .unwrap();
        let page = pkm_block::Page {
            path: vault.join(rel),
            rel_path: rel.into(),
            slug: Path::new(rel).file_stem().unwrap().to_string_lossy().to_string(),
            frontmatter: pkm_block::PageFrontmatter::default(),
            block_tree: pkm_block::tree::BlockTree::default(),
            block_order: Vec::new(),
            size_bytes: 0,
            modified_at: chrono::Utc::now(),
        };
        store.upsert_page(&page).unwrap();
    }
    (dir, store)
}

fn make_clip(dir: &Path) -> std::path::PathBuf {
    // 3 seconds of synthetic audio as FLAC (2s speaker A-ish, 1s B-ish —
    // content is irrelevant; the mock endpoint ignores it).
    let rate = 16000u32;
    let samples: Vec<f32> = (0..rate as usize * 3)
        .map(|i| ((i as f32 * 220.0 * std::f32::consts::TAU / rate as f32).sin() * 0.4))
        .collect();
    let clip = dir.join("recording.flac");
    pkm_audio::encode_flac(&samples, rate, &clip).unwrap();
    clip
}

#[tokio::test]
async fn test_pipeline_full_flow() {
    let server = MockServer::start().await;
    // transcriptions → whisperx-style broken-scale timestamps
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "Let's ship the voice feature. Agreed.",
            "segments": [
                {"id": 0, "start": 0, "end": 2e-09, "text": " Let's ship the voice feature."},
                {"id": 1, "start": 3e-09, "end": 5e-09, "text": "Agreed."}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;
    // diarization → two speakers, aligned with the segments above
    Mock::given(method("POST"))
        .and(path("/v1/audio/diarization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "task": "diarize",
            "duration": 5.0,
            "num_speakers": 2,
            "segments": [
                {"id": 0, "speaker": "SPEAKER_00", "label": "0", "start": 0.0, "end": 2.2},
                {"id": 1, "speaker": "SPEAKER_01", "label": "1", "start": 2.8, "end": 5.0}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let (dir, store) = fixture_vault();
    let clip = make_clip(dir.path());

    // enrolled voice that will match SPEAKER_00's slice (mock not called —
    // identification runs only against registry entries; keep it empty so no
    // /v1/voice/embed calls happen, and assert names stay labels).
    let registry = SpeakerRegistry::default();

    let llm = MockLlm {
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let stages = std::cell::RefCell::new(Vec::new());
    let pipeline = Pipeline {
        endpoint: SttEndpoint::new(server.uri(), None).unwrap(),
        store: &store,
        vault_path: dir.path(),
        index_path: &dir.path().join(".pkm/search"),
        llm: &llm,
        llm_model: "test-model",
        transcribe_model: "whisper-1",
        diarize_model: "vibevoice-cpp-asr",
        language: None,
        registry: &registry,
        on_stage: Some(&|s| stages.borrow_mut().push(s)),
    };
    let opts = PipelineOptions {
        clip_path: &clip,
        clip_rel_path: "assets/recordings/recording.flac",
        page_slug: "meeting",
        recorded_at: Local::now(),
        duration_secs: 5.0,
        summarize: true,
        diarize: true,
        identify: true,
    };

    let out = run(&pipeline, &opts).await.unwrap();

    assert!(out.diarized);
    assert_eq!(out.num_speakers, 2);
    assert_eq!(out.turns.len(), 2);
    assert_eq!(out.turns[0].speaker.as_deref(), Some("SPEAKER_00"));
    assert_eq!(out.turns[1].speaker.as_deref(), Some("SPEAKER_01"));
    assert!(out.speaker_names.is_empty(), "no registry -> no names");

    // summary + links + tags from the mock LLM
    assert_eq!(
        out.summary.as_deref(),
        Some("Decided to ship voice dictation with diarization.")
    );
    assert_eq!(out.related, vec!["voice dictation", "rust notes"]);
    assert_eq!(out.tags, vec!["meeting", "voice"]);

    // rendered markdown
    assert!(out.markdown.contains("**Speaker 1:** Let's ship the voice feature."));
    assert!(out.markdown.contains("**Speaker 2:** Agreed."));
    assert!(out.markdown.contains("**Related:** [[voice dictation]], [[rust notes]]"));
    assert!(out.markdown.contains("#meeting #voice"));
    assert!(out.markdown.contains("> **Summary:** Decided to ship voice dictation"));

    // stage order
    assert_eq!(
        stages.into_inner(),
        vec![
            Stage::Transcribing,
            Stage::Diarizing,
            Stage::Summarizing,
            Stage::Linking
        ]
    );
}

#[tokio::test]
async fn test_pipeline_falls_back_to_flat_when_no_diarization() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "Hello world.",
            "segments": [
                {"id": 0, "start": 0, "end": 1.0, "text": "Hello"},
                {"id": 1, "start": 1.2, "end": 2.0, "text": "world."}
            ]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/diarization"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let (dir, store) = fixture_vault();
    let clip = make_clip(dir.path());
    let registry = SpeakerRegistry::default();
    let llm = MockLlm {
        calls: Arc::new(AtomicUsize::new(0)),
    };

    let pipeline = Pipeline {
        endpoint: SttEndpoint::new(server.uri(), None).unwrap(),
        store: &store,
        vault_path: dir.path(),
        index_path: &dir.path().join(".pkm/search"),
        llm: &llm,
        llm_model: "test-model",
        transcribe_model: "whisper-1",
        diarize_model: "vibevoice-cpp-asr",
        language: None,
        registry: &registry,
        on_stage: None,
    };
    let opts = PipelineOptions {
        clip_path: &clip,
        clip_rel_path: "assets/recordings/recording.flac",
        page_slug: "meeting",
        recorded_at: Local::now(),
        duration_secs: 2.0,
        summarize: false,
        diarize: true,
        identify: false,
    };

    let out = run(&pipeline, &opts).await.unwrap();
    assert!(!out.diarized);
    assert_eq!(out.num_speakers, 0);
    // flat fallback merges into one unlabelled paragraph
    assert!(out.markdown.contains("Hello world."));
    assert!(!out.markdown.contains("**Speaker"));
}

#[tokio::test]
async fn test_pipeline_identifies_enrolled_speakers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "a b",
            "segments": [
                {"id": 0, "start": 0, "end": 2.0, "text": "a"},
                {"id": 1, "start": 3.0, "end": 5.0, "text": "b"}
            ]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/diarization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "task": "diarize",
            "duration": 5.0,
            "num_speakers": 1,
            "segments": [
                {"id": 0, "speaker": "SPEAKER_00", "label": "0", "start": 0.0, "end": 5.0}
            ]
        })))
        .mount(&server)
        .await;
    // voice embed returns a probe embedding; registry embedding chosen to
    // match it exactly → cosine 1.0
    let probe_embedding: Vec<f32> = (0..16).map(|i| i as f32 / 16.0).collect();
    Mock::given(method("POST"))
        .and(path("/v1/voice/embed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "embedding": probe_embedding,
            "dim": 16,
            "model": "speechbrain/spkrec-ecapa-voxceleb"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let (dir, store) = fixture_vault();
    let clip = make_clip(dir.path());

    let mut registry = SpeakerRegistry::default();
    registry.upsert(SpeakerEntry {
        name: "Alice".to_string(),
        clip: None,
        embedding: Some(probe_embedding),
        enrolled_at: Some("2026-08-04T19:00:00Z".to_string()),
    });

    let llm = MockLlm {
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let pipeline = Pipeline {
        endpoint: SttEndpoint::new(server.uri(), None).unwrap(),
        store: &store,
        vault_path: dir.path(),
        index_path: &dir.path().join(".pkm/search"),
        llm: &llm,
        llm_model: "test-model",
        transcribe_model: "whisper-1",
        diarize_model: "vibevoice-cpp-asr",
        language: None,
        registry: &registry,
        on_stage: None,
    };
    let opts = PipelineOptions {
        clip_path: &clip,
        clip_rel_path: "assets/recordings/recording.flac",
        page_slug: "meeting",
        recorded_at: Local::now(),
        duration_secs: 5.0,
        summarize: false,
        diarize: true,
        identify: true,
    };

    let out = run(&pipeline, &opts).await.unwrap();
    assert_eq!(out.speaker_names.get("SPEAKER_00").map(|s| s.as_str()), Some("Alice"));
    assert!(out.markdown.contains("**Alice:** a"));
}

#[test]
fn test_registry_roundtrip_integration() {
    let dir = tempdir().unwrap();
    let path = dir.path().join(".pkm/speakers.toml");
    let mut reg = SpeakerRegistry::default();
    reg.upsert(SpeakerEntry {
        name: "Bob".to_string(),
        clip: Some("assets/speakers/bob.flac".to_string()),
        embedding: Some(vec![0.5; 192]),
        enrolled_at: None,
    });
    reg.save(&path).unwrap();
    let loaded = SpeakerRegistry::load(&path).unwrap();
    assert_eq!(loaded.get("BOB").unwrap().embedding, Some(vec![0.5; 192]));
    let _ = DateTime::<Local>::default();
}
