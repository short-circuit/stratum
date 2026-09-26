# Endpoint Validation: OpenAI-Compatible AI / Embedding / STT

**Task:** t_b60ea4ea — Validate the OpenAI-compatible endpoint and capture response shapes.
**Date:** 2026-09-22
**Reviewer's job:** use these *captured real* request/response examples (not spec assumptions) when
writing parsers in t_585b3ad9.

---

## 1. Working Endpoint

| Field | Value |
|-------|-------|
| Base URL | `http://localhost:8081/v1` |
| Host type | LocalAI (Docker, `localagi-localai-1`, `0.0.0.0:8081->8080`) |
| API key | `HERMES_CUSTOM_LOCALHOST_8081_API_KEY` env var (present; value not printed here) |
| Version | `vs_20250624` signature confirmed via `/v1/models` |
| Health | `/readyz` → HTTP 200, `/metrics` → HTTP 200 |

> **Auth caveat:** `/v1/models` answers **HTTP 200 with a valid JSON body even with no
> `Authorization` header or with a wrong key.** So "fetch models" succeeding does **not** prove the
> key is right. Actual auth enforcement on real data routes was not exercised (server is configured
> permissive); some routes may still reject a bad key. Always treat `Test Connection` / model list
> success as *transport* confidence only, not auth confidence.

### Verified working models (HTTP 200 on the relevant route)

| Route | Working model(s) | Failing model(s) / reason |
|-------|------------------|---------------------------|
| `POST /chat/completions` (non-stream) | `Gemma-4-E4B-Uncensored-HauhauCS-Aggressive`; `qwen3.8-27b-uncensored-aggressive` (returns empty content on trivial prompt) | `ace-step-turbo` (CUDA OOM), `qwen3-vl-8b-instruct` (CUDA OOM), `qwen3.6-27b-fable-…` (model file load fail) |
| `POST /chat/completions` (stream) | **none emit `content`** — see deviation S1 | — |
| `POST /embeddings` | `text-embedding-ada-002` (384-dim) | `Qwen3-VL-Embedding-2B-GGUF` (model load fail), `jina-reranker-v1-base-en` (`NaN` in response marshal) |
| `POST /audio/transcriptions` | `whisper-1`, `whisperx-tiny`, `moss-transcribe-cpp-0.9b` | — |
| `POST /audio/diarization` | `pyannote-diarization`, `audio-cpp-sortformer-diarization` | — |
| `POST /voice/embed` | `speechbrain-ecapa-tdnn` (192-dim) | — |

**GPU context:** 17.7/24.5 GiB in use (≈72% occupied) — any model needing >~5 GiB free fails with
CUDA OOM. This is a *host* constraint, not a code problem, but it explains why several curated
models in `/v1/models` 500 on every call.

---

## 2. Exact Captured Shapes (real bodies)

All bodies below are **captured verbatim** from the live endpoint (files in `/tmp/stratum_captures/`
for the raw bytes; sample payloads reproduced here).

### 2.1 Chat completion — non-streaming (works)

**Request**
```json
{
  "model": "Gemma-4-E4B-Uncensored-HauhauCS-Aggressive",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant. Reply in one short sentence."},
    {"role": "user", "content": "What is 2+2? Reply with just the number."}
  ],
  "temperature": 0.0,
  "max_tokens": 64,
  "stream": false
}
```

**Response (HTTP 200)** — `chat_Gemma-4-E4B-Uncensored-HauhauCS-Aggressive.json`
```json
{
  "created": 1790106874,
  "object": "chat.completion",
  "id": "a3d928da-7333-4737-a693-f19a80477960",
  "model": "Gemma-4-E4B-Uncensored-HauhauCS-Aggressive",
  "choices": [
    {
      "index": 0,
      "finish_reason": "stop",
      "message": {"role": "assistant", "content": "OK"}
    }
  ],
  "usage": {"prompt_tokens": 20, "completion_tokens": 2, "total_tokens": 22}
}
```
→ **Conforms to OpenAI schema.** Fields the parser needs (`choices[].message.content`, `usage`)
present. No deviation. `message.additional_properties` not emitted.

---

### 2.2 Chat completion — error envelope (the shape that breaks parsing)

`ace-step-turbo` (and the other OOM models) return **HTTP 500** with:

`chat_nostream.json`
```json
{
  "error": {
    "code": 500,
    "message": "failed to load model with internal loader: could not load model (no success): DiT init failed: ... CUDA out of memory. ...",
    "type": ""
  }
}
```
→ **Major deviation:** a non-2xx with an OpenAI `{"error": {"message": ...}}` envelope at the **top
level**. The OpenAI provider's `chat()` deserializes into `OpenAIResponse { choices, usage }`
via `resp.json().await` — an error body with no `choices` becomes a *parse error*, masking the real
reason. The **embeddings client** already handles this correctly (`error_message()`); the **chat
provider does not.**

Also note the streaming flavor of the same error (see S2): the error arrives as an SSE `data:` line.

---

### 2.3 Embeddings (works; minor lenient extras)

**Request**
```json
{
  "model": "text-embedding-ada-002",
  "input": ["The quick brown fox jumps over the lazy dog", "second test string"]
}
```
**Response (HTTP 200)** — `emb_text-embedding-ada-002.json`
```json
{
  "created": 1790106961,
  "object": "list",
  "id": "271fdd81-0328-4eac-8244-ee998b9e052b",
  "model": "text-embedding-ada-002",
  "data": [
    {
      "embedding": [0.022838458, 0.05821619, ...],
      "index": 0,
      "object": "embedding"
    },
    {
      "embedding": [0.029664101, ...],
      "index": 1,
      "object": "embedding"
    }
  ]
}
```
→ **Conforms** to `{ data: [{ embedding, index }] }`; the parser's `EmbeddingsResponse`
(`data`, `model`, optional `usage`) and `EmbeddingData` (`embedding`, `index`) both deserialize
fine — extra `object`/`id`/`created` fields are ignored. **`usage` is absent** (OpenAI always
sends it; LocalAI omits it) — the parser treats it as optional, so OK.

---

### 2.4 STT transcription (works; known timestamp quirk)

The tone WAV (`/tmp/stratum_test_tone.wav`, 16 kHz mono 3 s) transcribed with
`response_format=verbose_json` as Stratum sends it:

`stt_whisper-1_verbose.json`
```json
{
  "segments": [
    {"id": 0, "start": 0, "end": 0, "text": " you", "tokens": []}
  ],
  "text": " you"
}
```
`stt_whisperx-tiny_verbose.json`
```json
{
  "segments": [
    {"id": 0, "start": 0, "end": 1E-9, "text": " Thanks for watching!", "tokens": []}
  ],
  "text": " Thanks for watching!"
}
```
`stt_moss-transcribe-cpp-0.9b_verbose.json`
```json
{
  "segments": [{"id": 0, "start": 0, "end": 0, "text": " [humming]", "tokens": []}],
  "text": " [humming]"
}
```
→ **Conforms** to OpenAI `verbose_json` (`text`, optional `segments[]` with `text`). The
`whisperx-tiny` `end: 1E-9` is the **known LocalAI whisperx broken nanosecond-scale** deviation —
already handled by `normalize_ts()` in `pkm-stt/src/parse.rs` (multiply by 1e9). `tokens` is
present (as `[]`); the parser ignores unknown fields. **No fix needed.**

Long-known nuance captured in `crates/pkm-stt/tests/real_endpoint.rs`:
- whisperx timestamps come back as seconds/1e9 → already normalized.
- whisper.cpp backends emit integer nanoseconds → already normalized.

---

### 2.5 Diarization (works; extra `speakers` field ignored)

`diar_pyannote.json`
```json
{
  "task": "diarize",
  "duration": 3,
  "num_speakers": 1,
  "segments": [{"id": 0, "speaker": "SPEAKER_00", "label": "1", "start": 0.0309687, "end": 3.0515938}],
  "speakers": [{"id": "SPEAKER_00", "label": "1", "total_speech_duration": 3.020625, "segment_count": 1}]
}
```
→ **Conforms** to `parse.rs` (`task`, `duration`, `num_speakers`, `segments[].speaker/label/start/end`).
The extra top-level `speakers` array is ignored. Plain seconds floats — no ts normalize needed.
Also works on `audio-cpp-sortformer-diarization`.

---

### 2.6 Voice embed (works; extra `dim` field)

`voice_embed.json`
```json
{
  "dim": 192,
  "embedding": [-23.79608, 2.7799022, -1.2350072, ...],
  "model": "speechbrain-ecapa-tdnn"
}
```
→ `embedding` present (192-dim) — parser reads only `embedding`, ignores `dim`/`model`. OK.

---

## 3. Deviations from the Expected OpenAI Schema

| # | Endpoint / mode | Deviation | Impact on Stratum code |
|---|-----------------|-----------|------------------------|
| **S1** | Chat completions — **streaming**, reasoning-capable models | Deltas carry `reasoning` tokens with `content: null`; **zero actual `content`** across the whole stream, then `[DONE]` (HTTP 200 throughout) | `OpenAIProvider::stream_chat` reads only `delta.content` → streamed answer is **silently empty**. This is the highest-severity finding. `include_reasoning:false` did **not** suppress it on this server. |
| **S2** | Chat completions — **streaming**, model-load error | Error delivered as an SSE `data:` line containing `{"error":{"code":"server_error","message":"...","type":"server_error"}}`, then a bare `data: ` line (no `[DONE]`) | Stream parser treats the error line as an empty delta → silent empty result instead of a surfaced error |
| **S3** | Chat completions — **non-stream**, model-load error | HTTP 500 + top-level `{"error":{"code":500,"message":"...","type":""}}` (no `choices`) | `chat()` calls `resp.json()` into a struct requiring `choices` → surfaces as *parse error*, masking root cause (should surface the endpoint's `error.message`) |
| **S4** | `/v1/models` auth | HTTP 200 + valid body with **no/ wrong** bearer key | `stt_test_connection` / "Fetch models" can falsely report success; not a parse bug, but a false sense of config validity |
| **S5** | Embeddings | Extra `object:"embedding"` per item; top-level `id`, `created`; **no `usage`** | **None** — parser is lenient; `usage` already optional |
| **S6** | Diarization / voice embed | Extra `speakers` / `dim` fields | **None** — lenient ignore |

---

## 4. Impact Assessment For the Downstream Fix (t_585b3ad9)

These are the **only** deviations that affect correctness; everything else parses cleanly:

1. **Streaming is broken for reasoning-emitting models (S1).** Any reasoning-capable model on an
   OpenAI-compatible server (DeepSeek-style, qwen3 reasoning, Gemma-4) streams `reasoning`
   instead of `content`. The OpenAI provider must either (a) read `delta.reasoning` when
   `content` is null, or (b) request non-reasoning, or (c) detect zero-content streams and
   surface a warning. **This precisely matches "AI not working at all (popup, chat)".**
2. **Streaming error channel (S2)** must be parsed as an error, not an empty delta.
3. **Non-stream error bodies (S3)** should extract `error.message` for a clear surfaced error
   (the embeddings client already does this — mirror it in `chat()`).
4. Non-reasoning non-stream chat, embeddings, STT, diarization, voice embed **all conform** —
   no parser changes needed on the happy path.

## 5. Test Vault

Created at `/tmp/stratum-test-vault/` with `.pkm/config.toml`:

```toml
vault_path = "/tmp/stratum-test-vault"

[ai]
provider = "CustomOpenAI"
endpoint = "http://localhost:8081/v1"
api_key = "localai-test-key"
model = "ace-step-turbo"
rag_enabled = true
rag_chunk_count = 3
```

> NOTE adapted during validation: `model = ace-step-turbo` **fails to load (CUDA OOM)** on this host.
> Use `Gemma-4-E4B-Uncensored-HauhauCS-Aggressive` (non-stream) or `text-embedding-ada-002`
> (embeddings) for end-to-end tests. `whisper-1` for STT, `pyannote-diarization` for diarization,
> `speechbrain-ecapa-tdnn` for voice embed.

---

## 6. Raw Capture Files

All raw bodies preserved in `/tmp/stratum_captures/`:

```
chat_Gemma-4-E4B-Uncensored-HauhauCS-Aggressive.json   HTTP 200 full chat response
chat_nostream.json                                      HTTP 500 error envelope (ace-step-turbo)
chat_nostr_noreasoning.json                             HTTP 200 include_reasoning=false
chat_stream_sse.raw                                     SSE stream (Gemma) — reasoning only
chat_stream_noreasoning.raw                             SSE stream include_reasoning=false — still reasoning only
chat_stream_qwen3.8-27b-uncensored-aggressive.raw       SSE stream (qwen3.8) — reasoning only
chat_stream_qwen3-vl-8b-instruct.raw                    SSE stream error data line (server_error)
emb_text-embedding-ada-002.json                         HTTP 200 embeddings response (384-dim)
emb_Qwen3-VL-Embedding-2B-GGUF.json                     HTTP 500 (model load failed)
emb_jina-reranker-v1-base-en.json                       HTTP 500 (NaN marshal)
stt_whisper-1_verbose.json / whisperx / moss            HTTP 200 verbose_json
diar_pyannote.json / diar_sortformer.json               HTTP 200 diarization
voice_embed.json                                        HTTP 200 192-dim embedding
```

Re-probe script (idempotent, re-runnable):
`/home/shrtcrct/.hermes/profiles/qa-engineer/cache/scratch/cap_*.sh`

---

*Endpoint validation recorded by qa-engineer — the working endpoint URL for end-to-end testing is
`http://localhost:8081/v1` (API key in `HERMES_CUSTOM_LOCALHOST_8081_API_KEY`).*
