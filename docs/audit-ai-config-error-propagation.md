# Task t_8f681a47 — Audit: AI config loading & error propagation in ai.rs commands

Commit audited: b73ac55 (working tree clean). No code changes made in this
audit — this is a findings report only.

Primary scope is `src-tauri/src/commands/ai.rs`; related surfaces traced into
`crates/pkm-ai`, `crates/pkm-stt`, `src-tauri/src/commands/{dictation,tts,settings}.rs`
and `crates/pkm-cli/src/main.rs` because the config and masking behaviour is
shared. All line references are to the current working tree.

---

## 1. Config paths (the sources of truth for AI/STT settings)

### 1.1 `AiConfig` — config file values
- `crates/pkm-core/src/config.rs:200-242` — `AiConfig` struct + defaults.
  Fields: `provider` (enum-line 262-273), `endpoint: Option<String>`,
  `api_key: Option<String>`, `model`, `models` (capability list),
  `rag_enabled`, `rag_chunk_count`, `embedding_dimensions`.
- Defaults (`config.rs:229-241`): provider = Ollama, endpoint =
  `http://localhost:11434`, api_key = None, model = `llama3.2`.

### 1.2 `effective_api_key()` — env fallback (config.rs:244-260)
- Env checked per provider, env wins over the config-file value:
  - OpenAI / CustomOpenAI  -> `OPENAI_API_KEY`
  - Anthropic / CustomAnthropic -> `ANTHROPIC_API_KEY`
  - Google -> `GOOGLE_API_KEY`
  - Ollama / Zai / Custom -> config-file value only, **no env fallback**.
- Consumed at:
  - `crates/pkm-ai/src/provider.rs:1100,1106,1112,1115,1122,1128` (ProviderFactory::create)
  - `crates/pkm-ai/src/embedding.rs:125` (EmbeddingConfig::from_ai_config)
  - `crates/pkm-ai/src/tts.rs:127` (fallback only when `tts.api_key` unset)
  - `src-tauri/src/commands/settings.rs:442` (fetch_models) and 136-145 (get_settings UI flag)
- The UI flags `api_key_from_env` by **presence only**: `std::env::var(...).is_ok()`
  (settings.rs:136-145) — an env var set to an empty string is treated as
  configured, though `effective_api_key()` would also return that empty string.

### 1.3 `[stt]` — STT endpoint config
- `SttConfig` — `config.rs:281-316`; fields `endpoint: String` (empty =
  disabled), `api_key`, `model`, `diarize_model`, `language`, `diarize`,
  `auto_summarize`, `auto_identify`. **No env fallback for the STT key.**
- `crates/pkm-stt/src/client.rs:27-31` — `SttEndpoint::new` trims trailing `/`,
  validates via `validate_endpoint_safe`; errors mapped to `PkmError::Ai`.

### 1.4 `[tts]` — TTS endpoint config
- `TtsConfig` — `config.rs:324-350`; fields `endpoint` (empty = use AI
  endpoint), `api_key`, `voice`, `format`, `speed`.
- `crates/pkm-ai/src/tts.rs:87-138` — `TtsConfigResolved::from_config`: endpoint
  = `tts.endpoint` if set, else `ai` endpoint (+`/v1` for Ollama); api_key =
  `tts.api_key` if set, else `ai.effective_api_key()` (env fallback applies).

### 1.5 config file location / load
- All commands resolve the same path: `<vault>/.pkm/config.toml`.
- `Config::load` = `config.rs:445-457`: read + `toml::from_str`; TOML parse
  errors become `PkmError::Config` via `config.rs:85-88`.
- Load sites: `ai.rs:91-95,163-167,219-223,265-269,407-411`; `tts.rs:52-56,95-99`;
  `dictation.rs:100-106`; `settings.rs:128-134,430-434`; CLI `main.rs:552-563,601-612`.

### 1.6 Endpoint defaulting (ProviderFactory) — provider.rs:1078-1131
- No endpoint set -> per-provider default (provider.rs:1080-1094):
  Ollama `localhost:11434`, OpenAI `api.openai.com/v1`,
  Anthropic `api.anthropic.com`, Custom `localhost:8080/v1/chat/completions`,
  CustomOpenAI `localhost:8080/v1`, CustomAnthropic `api.anthropic.com`,
  Google `generativelanguage.googleapis.com/v1beta`, Zai `api.z.ai`.

---

## 2. Error-masking sites

### 2.1 "AI not configured" early-returns mask real config/parse/endpoint errors
Each command checks `config_path.exists()` and, on absence, returns a generic
"AI not configured…" string; on presence it calls `Config::load(...)?` which
**does propagate** read+TOML-parse errors (so `2.1` is the missing-file path
only). Failure mode: if the file exists but TOML is malformed, the real
`PkmError::Config` does surface — good; the mask is only raised for a missing
file, where the real reason is "no config file", not necessarily "AI not
configured".

| # | Location (ai.rs) | Early-return | Underlying cause replaced |
|---|---|---|---|
| M1 | 91-95 (`ai_transform_block`) | "AI not configured. Please configure AI provider in Settings." | missing `config.toml`; note: file could be missing for any reason |
| M2 | 163-167 (`ai_research`) | same | same; also masks that `[research]` may be the real gap |
| M3 | 219-223 (`generate_mermaid`) | same | missing file only |
| M4 | 265-269 (`ai_interlink_notes`) | `"AI not configured."` (shorter variant) | missing file |
| M5 | 407-411 (`ai_rag_query`) | "AI not configured. Configure the AI provider in Settings → AI." | missing file |

Note (research): `ai_research` requires `config.research.searxng_endpoint`;
it goes through `ResearchEngine::new(searxng_endpoint, …)` — provider creation
via `ProviderFactory::create` (research.rs:37). If only the AI config is
missing, but the file exists, M2's guard passes, then `ProviderFactory::create`
can return `PkmError::Config("…requires an API key")` which **is** propagated
(unlike the missing-file path).

### 2.2 Same mask in STT commands (dictation.rs) — masks parse errors too
- `dictation.rs:100-106` — `load_config`: missing file -> `"STT not
  configured. Set the transcription endpoint in Settings."`; `Config::load`
  errors (`PkmError::Config`, i.e. TOML parse) are **swallowed and re-prefixed
  to the same generic STT-not-configured string**, then returned as `Err(String)`.
  This is a **real error-masking bug**: a corrupt/malformed `config.toml`
  (or an IO error) is reported to the user as "STT not configured", hiding the
  underlying parse error.
- `dictation.rs:108-114` — `endpoint_for`: empty `stt.endpoint` -> same generic
  STT-not-configured string; non-empty but SSRF-invalid -> `"Invalid STT
  endpoint: {parse error}"` (surfaced properly).
- `stt_test_connection` (dictation.rs:525-563): calls `load_config` +
  `endpoint_for` (same masking), then `GET {base}/v1/models`. Non-2xx or
  transport errors are surfaced verbatim. The `/v1/models` GET is also a
  **different contract** than the actual transcription path
  (`POST /v1/audio/transcriptions`) — a 200 on models ≠ the endpoint actually
  able to transcribe (documented as a likely-false-positive "success").

### 2.3 Same mask in TTS commands (tts.rs)
- `tts.rs:52-56` & `95-99` — missing config file -> `"AI not configured.
  Configure the AI provider in Settings → AI."`. Note: TTS may be deliberately
  using a `[tts]` endpoint rather than `[ai]`, but the mask always blames AI.
- `TtsConfigResolved::from_config` errors (empty endpoint, no TTS model,
  SSRF-invalid) are **propagated** (tts.rs:93-104) — not masked.

### 2.4 CLI (crates/pkm-cli/src/main.rs) — masking, deliberate + partial
- `main.rs:552-563` (`cmd_ask`) — missing config -> `PkmError::Config("AI not
  configured. No config found at <path>. Run stratum init…")`. This is
  **explicit and actionable** (includes the path + next step), so it's the
  well-behaved variant.
- `main.rs:601-612` (`cmd_rag`) — missing config -> `PkmError::Config("No
  config found at … Run stratum init…")` (no "AI not configured" prefix).
- Both propagate `Config::load` parse errors (wrapped `main.rs:553-555,602-604`).
- `main.rs:130-136` — top-level error handler: prints to stderr, exits non-zero.
  **Sync/Ask/Rag no-op successes were previously masked as "ok"** (comment at
  131-133 documents this was already fixed for the AI path); not itself an
  error-masker.

### 2.5 Endpoint validation errors are silently dropped (swallowed) here
- `ai_interlink_notes` — `ai.rs:282-289`: `RelatedFinder::find_related(...).ok()
  .unwrap_or_default()` **discards the retrieval error**; if the Tantivy index
  is missing/corrupt, related notes silently come back empty and the command
  returns the original text unchanged (ai.rs:293-296) — no error surfaced, and
  the `warn!` is the only trace of the real cause.
- `dictation.rs:552` — `resp.json().await.unwrap_or(Models { data: vec![] })`:
  a malformed `/v1/models` body in `stt_test_connection` is treated as
  "success with zero models" instead of surfacing the parse error.

### 2.6 Parsing/parse-error swallowing inside the providers (crates/pkm-ai/src/provider.rs)
- **Ollama stream** (provider.rs:385-396): any chunk that fails to parse as
  `OllamaStreamChunk` is replaced by an **empty-content, done=false** delta —
  the parse failure is silently swallowed; not even a `warn!`. A malformed
  stream yields silently-empty output.
- **OpenAI stream** (provider.rs:633-648): same swallow — a line that doesn't
  start with `data: ` is dropped via `filter_map` (some error paths just get
  omitted), JSON parse failure -> empty delta (done=false). malformed SSE =
  silent empty output.
- **Anthropic stream** (provider.rs:867-879): same pattern — parse failure ->
  empty delta (done=false).
- **CustomProvider non-stream** (provider.rs:1029-1051): response parse failure
  is **propagated** (`Custom provider parse error: {e}`), but an ambiguous
  shape where no field parses yields `.unwrap_or_default()` = empty content
  **without error** (provider.rs:1051) — a "successful" empty response.
  Streaming: `custom` provider hard-errors `Unsupported("Streaming not
  supported…")` (provider.rs:1064-1067) — a caller that requires streaming gets
  a functional error, fine.
- Non-stream http errors are mostly propagated with context
  (provider.rs:272,289,363,492,519,598,739,764) — but `reqwest::Response::json()`
  does **not** re-check `status()`; a non-2xx HTML/error body surfaces as a
  confusing "parse error" rather than an HTTP status error (relevant in
  `chat()`, e.g. provider.rs:286-289).

### 2.7 HTTP status is not checked before body parse in non-stream chat
- Ollama (provider.rs:266-289), OpenAI (provider.rs:485-519), Anthropic
  (provider.rs:731-764): `resp.json()` is called on any status; a 4xx/5xx JSON
  error body fails to deserialize into the success shape -> the true
  server-side error is masked as `PkmError::Ai("<provider> parse error: …")`.
  e.g. an OpenAI 401 with `{"error":{"message":"Invalid API key"}}` becomes
  "OpenAI parse error: missing field `choices`" — the **real config failure
  (bad key / env key) is hidden** behind a parse message.
- CustomProvider is the exception: it checks `status().is_success()`
  (provider.rs:991-999) and surfaces the body (first 200 chars).

### 2.8 Config error inside ProviderFactory is properly translated to Config (not masked)
- provider.rs:1099-1129 — missing API key for OpenAI/Anthropic/CustomOpenAI/
  CustomAnthropic -> `PkmError::Config("<Provider> requires an API key")`.
  Since commands `.map_err(|e| e.to_string())`, the user **does** see the real
  reason (contrast with 2.1's missing-file mask).

---

## 3. Where each AI command gets its config (cmd -> methods) — summary

| Command (ai.rs) | load | provider | llm call |
|---|---|---|---|
| ai_transform_block (73-136) | 86-95 | ProviderFactory::create @97 | provider.chat @126; errors logged `error!` + surfaced |
| ai_research (152-205) | 158-167 | via ResearchEngine::new @174-180 -> ProviderFactory::create (research.rs:37) | engine.research @182-185; errors surfaced |
| generate_mermaid (208-245) | 214-223 | ProviderFactory::create @225 | provider.chat @235; logged + surfaced |
| ai_interlink_notes (248-362) | 255-269 | ProviderFactory::create @298 | provider.chat @326-329; related-finder error swallowed @282-289 |
| ai_rag_query (396-454) | 402-411 | OpenAIEmbeddingClient::from_ai_config @417 + ProviderFactory::create @418 | RagEngine::query @427-430; errors surfaced with "RAG query failed:" prefix |

| Command (dictation.rs) | load | stt endpoint | llm |
|---|---|---|---|
| dictation_transcribe (223-355) | load_config @232 | endpoint_for @233 (early) + @279; `ProviderFactory::create(&config.ai)` @253 (failures surfaced) | pkm_dictation::run @302 |
| speaker_assign (383-507) | load_config @404 + endpoint_for @405 | VoiceIdClient @448 | — |
| stt_test_connection (525-563) | load_config @528 + endpoint_for @529 | GET models @536 | — |

| Command (tts.rs) | load | config resolution |
|---|---|---|
| tts_synthesize (46-83) | 50-57 | TtsConfigResolved::from_config @60 (errors propagated) |
| tts_speak (89-126) | 93-100 | TtsConfigResolved::from_config @103 |

| Command (settings.rs) | load | purpose |
|---|---|---|
| get_settings (123-…) | 124-134 | DTO build, `api_key_from_env` flag @136-145 (env presence-only) |
| save_settings / save_graph_settings | load-or-zero @398-405 | config file writes |
| fetch_models (424-488) | 426-444 | GET /v1/models; uses `effective_api_key` @442; endpoint/url validated @447-452 |

---

## 4. Full failure-mode inventory

| ID | Location | Trigger | Result | Masked? |
|---|---|---|---|---|
| FM-1 | ai.rs:91-95+ | config.toml missing | generic "AI not configured" | masks "no config file" (all AI cmds) |
| FM-2 | dictation.rs:100-106 | config.toml missing OR **Config::load parse/IO error** | generic "STT not configured" | **YES — real parse/IO error replaced by STT-not-configured string** |
| FM-3 | dictation.rs:108-114 | stt.endpoint empty | "STT not configured…" | (accurate) |
| FM-4 | dictation.rs:108-114 | stt.endpoint SSRF-invalid | "Invalid STT endpoint: <e>" | no |
| FM-5 | dictation.rs:552 | /v1/models body malformed | treated as success, 0 models | YES |
| FM-6 | dictation.rs:536-562 | endpoint down / 4xx | "Connection failed / Endpoint responded with HTTP …" | no |
| FM-7 | tts.rs:52-56,95-99 | config.toml missing | "AI not configured …" (blames AI, not [tts]) | masks missing file; wrong attribution when using [tts] endpoint |
| FM-8 | provider.rs:286-289,516-519,761-764 | non-2xx JSON error body | "parse error: missing field …" | **YES — real server error (401 invalid key etc.) hidden behind a JSON parse error** |
| FM-9 | provider.rs:385-396,633-648,867-879 | malformed stream chunk | silently emitted empty delta (done=false) | YES — silent empty output, no warn |
| FM-10 | provider.rs:1051 | custom response w/ no recognizable field | Ok with empty content | YES — empty response reported as success |
| FM-11 | provider.rs:1099-1129 | missing API key (no env, no config) | `Config("<Provider> requires an API key")` | no — surfaced to user (good) |
| FM-12 | ai.rs:282-289 | RelatedFinder retrieval error (index missing/corrupt) | `.ok().unwrap_or_default()` → empty related + returns original text | **YES — retrieval error dropped; no error surfaced** |
| FM-13 | ai.rs:293-296 | no related pages found | returns original text (this is intended) | intended |
| FM-14 | settings.rs:136-145 | env var set but empty | `api_key_from_env=true` though key is empty | YES — presence-only check reports empty key as configured |
| FM-15 | settings.rs:424-487 | fetch_models non-2xx / transport | "API returned status 4xx" / transport `to_string()` | partial — status surfaced, body dropped |
| FM-16 | provider.rs:982-989 | custom 405 retry also fails | retry error surfaced | no |
| FM-17 | main.rs:559-562 | CLI ask, file missing | Config error w/ path + next step (actionable) | no — explicit + actionable |
| FM-18 | provider.rs:1064-1067 | custom provider streaming | `Unsupported("Streaming not supported…")` | no (functional error) |

Worth flagging separately:
- **STT has no env fallback** for its API key (`SttConfig.api_key` only),
  unlike AI/TTS — documented but asymmetric.
- **TTS API-key fallback to AI**, even when the TTS endpoint is configured
  independently: a user pointing TTS at a separate server with a separate key
  and leaving `tts.api_key` empty will silently send the **AI** provider's key
  (from `effective_api_key`, which may even be an env key) to the TTS server
  (tts.rs:124-128). Not an error-mask, but a credential-routing surprise.

---

## 5. Recommendation (future work, no changes made per acceptance criteria)

1. Never replace propagated errors with the generic "not configured" string —
   `Config::load` errors should be prefixed, not swallowed (dictation.rs:105;
   the ai/tts commands already do this correctly).
2. In non-stream `chat()` for Ollama/OpenAI/Anthropic, branch on
   `resp.status()` and surface the server error body before attempting to
   deserialize the success shape (closes FM-8).
3. Emit a `warn!`/`debug!` when a stream chunk fails to parse instead of a
   silent empty delta (FM-9) — ideally surface `Err(PkmError::Ai(...))` so a
   malformed stream stops the UI silently producing nothing.
4. `stt_test_connection` should probe the real transcription contract or at
   least surface the models-parse failure (FM-5/FM-6 ambiguity).
5. `ai_interlink_notes` should propagate `find_related` errors instead of
   `.ok().unwrap_or_default()` (FM-12).
6. Consider an STT env fallback for symmetry and document TTS key-fallback
   semantics (F-flag above).
