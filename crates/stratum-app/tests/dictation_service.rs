//! Headless tests for the voice dictation service commands:
//! `dictation_start`, `dictation_stop`, `dictation_cancel`,
//! `dictation_transcribe`, `dictation_insert_clip_ref`, `speaker_list`,
//! `speaker_assign`, `speaker_delete`, `mic_permission_status`,
//! `request_mic_permission`, `stt_test_connection`.

mod common;

use common::{audio_available, block_dto, create_test_vault, make_flac_clip, RecordingSink};
use stratum_app::dictation::{DictationOptsDto, SpeakerTurnDto};
use stratum_app::DictationSession;

/// Write the STT section the dictation commands load from config.
fn seed_stt_config(vault: &common::TestVault, endpoint: &str, ai_endpoint: Option<&str>) {
    let mut config = vault.config();
    config.stt.endpoint = endpoint.to_string();
    config.stt.model = "whisper-1".into();
    config.stt.diarize_model = "pyannote-diarization".into();
    config.stt.diarize = true;
    if let Some(ai) = ai_endpoint {
        config.ai.provider = pkm_core::AiProvider::CustomOpenAI;
        config.ai.endpoint = Some(ai.to_string());
        config.ai.api_key = Some("sk-test".into());
        config.ai.model = "test-model".into();
    }
    vault.save_config(&config);
}

fn seed_registry(vault: &common::TestVault, name: &str) {
    let reg = pkm_dictation::speakers::SpeakerRegistry {
        speakers: vec![pkm_dictation::speakers::SpeakerEntry {
            name: name.to_string(),
            clip: None,
            embedding: None,
            enrolled_at: None,
        }],
    };
    reg.save(&vault.vault_path.join(".pkm/speakers.toml"))
        .unwrap();
}

#[tokio::test]
async fn dictation_cancel_with_no_recording_returns_guard_error() {
    let vault = create_test_vault();

    let err = vault.service.dictation_cancel().unwrap_err();

    assert!(err.contains("No recording"), "guard error: {err}");
}

#[tokio::test]
async fn dictation_stop_with_no_recording_returns_guard_error() {
    let vault = create_test_vault();

    let err = vault.service.dictation_stop().unwrap_err();

    assert!(err.contains("No recording"), "guard error: {err}");
}

#[tokio::test]
async fn dictation_start_stop_lifecycle_when_audio_is_available() {
    if !audio_available() {
        eprintln!("SKIP: no live audio input service on this host");
        return;
    }

    let vault = create_test_vault();
    vault.write_file("pages/Welcome.md", "# Welcome\n");

    let started = vault
        .service
        .dictation_start("pages/Welcome.md".into())
        .expect("start ok");
    assert!(
        started.recording_path.contains("assets/recordings")
            || started.recording_path.contains("recordings"),
        "clip lands in the recordings dir: {}",
        started.recording_path
    );
    assert!(vault
        .service
        .dictation_start("pages/Welcome.md".into())
        .is_err());

    let stopped = vault.service.dictation_stop().expect("stop ok");
    assert!(stopped.duration_secs >= 0.0);
    let clip = std::path::PathBuf::from(&stopped.recording_path);
    assert!(clip.exists(), "clip written on disk");
    assert!(!clip.with_extension("flac.tmp").exists(), "no tmp residue");
    let magic = &std::fs::read(&clip).unwrap()[..4];
    assert_eq!(magic, b"fLaC", "clip is real FLAC");
    assert!(clip.starts_with(&vault.vault_path), "clip stays in vault");
}

#[tokio::test]
async fn dictation_cancel_discards_the_clip_when_audio_is_available() {
    if !audio_available() {
        eprintln!("SKIP: no live audio input service on this host");
        return;
    }

    let vault = create_test_vault();
    let started = vault
        .service
        .dictation_start("pages/Welcome.md".into())
        .expect("start ok");
    let clip = std::path::PathBuf::from(&started.recording_path);

    vault.service.dictation_cancel().expect("cancel ok");

    assert!(!clip.exists(), "cancelled clip removed");
    assert!(!clip.with_extension("flac.tmp").exists(), "no tmp residue");
}

#[tokio::test]
async fn dictation_transcribe_inserts_a_diarized_memo() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let vault = create_test_vault();
    let server = wiremock::MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "Let's ship the voice feature.",
            "segments": [
                {"id": 0, "start": 0.0, "end": 2.0, "text": " Let's ship the voice feature."}
            ]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/diarization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "task": "diarize",
            "duration": 2.0,
            "num_speakers": 1,
            "segments": [
                {"id": 0, "speaker": "SPEAKER_00", "label": "0", "start": 0.0, "end": 2.0}
            ]
        })))
        .mount(&server)
        .await;
    // Enrichment chat (related/tags) is tolerated either way, but mount it so
    // the provider has a deterministic upstream.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{ "message": { "role": "assistant", "content": "[[voice feature]]" } }],
            "usage": { "prompt_tokens": 1, "completion_tokens": 1 }
        })))
        .mount(&server)
        .await;

    seed_stt_config(&vault, &server.uri(), Some(&format!("{}/v1", server.uri())));
    vault
        .service
        .save_blocks(
            "pages/meeting.md".into(),
            vec![block_dto("existing note")],
            Some("Meeting".into()),
        )
        .await
        .unwrap();
    let clip = make_flac_clip(&vault.vault_path.join("assets/recordings/memo.flac"), 2);

    let sink = RecordingSink::new();
    let result = vault
        .service
        .dictation_transcribe(
            clip.to_string_lossy().to_string(),
            "pages/meeting.md".into(),
            Some(DictationOptsDto {
                summarize: Some(false),
                diarize: Some(true),
                identify: Some(false),
            }),
            &sink,
        )
        .await
        .expect("transcribe ok");

    assert!(result.diarized, "diarization must run");
    assert!(result.num_speakers >= 1);
    assert!(
        !result.inserted_block_ids.is_empty(),
        "memo blocks inserted"
    );
    assert!(
        result.markdown.contains("Speaker"),
        "rendered memo must label speakers: {}",
        result.markdown
    );
    assert_eq!(result.clip_rel_path, "assets/recordings/memo.flac");
    assert!(sink.has_event("dictation:stage"), "stage progress emitted");
    assert!(sink.has_event("dictation:done"), "completion emitted");

    // The session is retained so speakers can be renamed afterwards.
    assert!(vault
        .service
        .lock()
        .unwrap()
        .dictation_sessions
        .contains_key(&clip.to_string_lossy().to_string()));

    // The memo was persisted to the page.
    let stored = vault
        .service
        .get_blocks("pages/meeting.md".into())
        .await
        .unwrap();
    assert!(
        stored
            .blocks
            .iter()
            .any(|b| b.content.contains("Voice Memo") || b.content.contains("ship")),
        "memo blocks must be on the page: {stored:?}"
    );
}

#[tokio::test]
async fn dictation_transcribe_rejects_missing_stt_configuration() {
    let vault = create_test_vault();
    vault
        .service
        .save_blocks("pages/meeting.md".into(), vec![block_dto("x")], None)
        .await
        .unwrap();
    let clip = make_flac_clip(&vault.vault_path.join("assets/recordings/memo.flac"), 1);
    let sink = RecordingSink::new();

    let err = vault
        .service
        .dictation_transcribe(
            clip.to_string_lossy().to_string(),
            "pages/meeting.md".into(),
            None,
            &sink,
        )
        .await
        .unwrap_err();

    assert!(
        err.contains("config") || err.contains("STT"),
        "error must explain the missing config: {err}"
    );
}

#[tokio::test]
async fn dictation_insert_clip_ref_writes_a_placeholder_and_emits_done() {
    let vault = create_test_vault();
    vault
        .service
        .save_blocks("pages/meeting.md".into(), vec![block_dto("x")], None)
        .await
        .unwrap();
    let sink = RecordingSink::new();

    let ids = vault
        .service
        .dictation_insert_clip_ref(
            "pages/meeting.md".into(),
            "assets/recordings/memo.flac".into(),
            65.0,
            &sink,
        )
        .expect("fallback insert ok");

    assert!(!ids.is_empty());
    let stored = vault
        .service
        .get_blocks("pages/meeting.md".into())
        .await
        .unwrap();
    assert!(
        stored
            .blocks
            .iter()
            .any(|b| b.content.contains("transcription pending")),
        "placeholder block must be inserted: {stored:?}"
    );
    let events = sink.events();
    let done = events
        .iter()
        .find(|(name, _)| name == "dictation:done")
        .expect("done event");
    assert_eq!(done.1["fallback"], serde_json::json!(true));
}

#[tokio::test]
async fn speaker_list_and_delete_roundtrip_through_the_registry() {
    let vault = create_test_vault();
    seed_registry(&vault, "Alice");

    let speakers = vault.service.speaker_list().unwrap();
    assert_eq!(speakers.len(), 1);
    assert_eq!(speakers[0].name, "Alice");

    vault.service.speaker_delete("Alice".into()).unwrap();

    let on_disk = pkm_dictation::speakers::SpeakerRegistry::load(
        &vault.vault_path.join(".pkm/speakers.toml"),
    )
    .unwrap();
    assert!(on_disk.names().is_empty(), "delete persists");
    assert!(vault.service.speaker_list().unwrap().is_empty());
}

#[tokio::test]
async fn speaker_assign_renames_the_memo_without_enrollment() {
    let vault = create_test_vault();
    seed_stt_config(&vault, "http://127.0.0.1:9", None);
    vault
        .service
        .save_blocks(
            "pages/meeting.md".into(),
            vec![block_dto("existing")],
            Some("Meeting".into()),
        )
        .await
        .unwrap();

    let clip = make_flac_clip(&vault.vault_path.join("assets/recordings/memo.flac"), 1);
    let recording_path = clip.to_string_lossy().to_string();

    vault.service.lock().unwrap().dictation_sessions.insert(
        recording_path.clone(),
        DictationSession {
            page_path: "pages/meeting.md".into(),
            clip_path: clip.clone(),
            clip_rel_path: "assets/recordings/memo.flac".into(),
            recorded_at: chrono::Local::now(),
            duration_secs: 1.0,
            turns: vec![pkm_stt::SpeakerTurn {
                speaker: Some("SPEAKER_00".into()),
                start: 0.0,
                end: 1.0,
                text: "hello there".into(),
            }],
            speaker_names: std::collections::HashMap::new(),
            summary: None,
            related: Vec::new(),
            tags: Vec::new(),
            inserted_block_ids: Vec::new(),
        },
    );

    let result = vault
        .service
        .speaker_assign(
            recording_path.clone(),
            "SPEAKER_00".into(),
            "Alice".into(),
            false,
        )
        .await
        .expect("assign ok");

    assert_eq!(result.name, "Alice");
    assert!(!result.enrolled, "enrollment was not requested");
    assert!(
        result.markdown.contains("**Alice:**"),
        "memo must be re-rendered with the new name: {}",
        result.markdown
    );
    assert!(!result.inserted_block_ids.is_empty());

    let names = vault.service.speaker_list().unwrap();
    assert!(names.iter().any(|s| s.name == "Alice"), "registry updated");
}

#[tokio::test]
async fn speaker_assign_rejects_an_empty_name() {
    let vault = create_test_vault();
    seed_stt_config(&vault, "http://127.0.0.1:9", None);

    let err = vault
        .service
        .speaker_assign(
            "missing.flac".into(),
            "SPEAKER_00".into(),
            "  ".into(),
            false,
        )
        .await
        .unwrap_err();

    assert!(err.contains("Name must not be empty"), "error: {err}");
}

#[tokio::test]
async fn mic_permission_status_is_granted_on_desktop() {
    let vault = create_test_vault();

    let status = vault.service.mic_permission_status();

    #[cfg(not(target_os = "android"))]
    assert!(status.granted, "desktop is always granted");
    assert!(!status.restricted);
}

#[tokio::test]
async fn request_mic_permission_is_a_safe_noop_off_android() {
    let vault = create_test_vault();
    let sink = RecordingSink::new();

    vault.service.request_mic_permission(&sink);

    #[cfg(target_os = "android")]
    assert!(sink.has_event("mic-permission-request"));
    #[cfg(not(target_os = "android"))]
    assert!(sink.events().is_empty(), "no event off Android");
}

#[tokio::test]
async fn stt_test_connection_lists_models_from_the_endpoint() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let vault = create_test_vault();
    let server = wiremock::MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "whisper-1" }, { "id": "vibevoice-cpp-asr" }]
        })))
        .mount(&server)
        .await;
    seed_stt_config(&vault, &server.uri(), None);

    let result = vault.service.stt_test_connection().await.unwrap();

    assert!(result.ok);
    assert!(result.error.is_none());
    assert!(result.models.iter().any(|m| m == "whisper-1"));
}

#[tokio::test]
async fn stt_test_connection_fails_cleanly_on_a_dead_port() {
    let vault = create_test_vault();
    seed_stt_config(&vault, "http://127.0.0.1:9", None);

    let err = vault.service.stt_test_connection().await.unwrap_err();

    assert!(
        err.contains("Connection failed") || err.contains("HTTP"),
        "failure must be surfaced: {err}"
    );
}

// Keep the DTO import used even when the transcribe test is skipped by cfg.
#[allow(dead_code)]
fn _speaker_turn_dto_shape() -> SpeakerTurnDto {
    SpeakerTurnDto {
        speaker: None,
        start: 0.0,
        end: 0.0,
        text: String::new(),
    }
}
