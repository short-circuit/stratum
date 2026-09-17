# AI Features

Stratum integrates with LLM providers for AI-powered note operations, chat, and research.

<!-- SCREENSHOT: [ai-formatting-toolbar] AI action buttons in the editor formatting toolbar -->

## Setting Up AI

### Prerequisites

You need a running LLM provider. Options:

| Provider | Setup | Notes |
|----------|-------|-------|
| **Ollama** (recommended) | Install [Ollama](https://ollama.com), pull a model | Fully offline, free |
| OpenAI | Get an API key | Cloud-based, paid |
| Anthropic | Get an API key | Cloud-based, paid |
| Google AI | Get an API key | Cloud-based |
| Custom | Any OpenAI-compatible endpoint | Self-hosted or third-party |

### Configuration

1. Open **Settings → AI**
2. Select your **Provider**
3. Enter the **API Endpoint** (for Ollama/custom) or **API Key** (for cloud)
4. Set the **Default Chat Model**
5. Click **Fetch Models** to refresh the model list
6. Assign **capabilities** to each model: `chat`, `embedding`
7. Toggle **RAG** on for retrieval-augmented generation
8. Click **Save**

<!-- SCREENSHOT: [settings-ai-tab] AI configuration tab in Settings -->

## Embeddings & RAG

Embedding generation (used by RAG to retrieve and re-rank your notes by
semantic similarity) calls the **same configured AI endpoint** you set up
above — no separate configuration is required.

- Stratum sends `POST {endpoint}/v1/embeddings` with a JSON body of the form
  `{ "model": "<embedding-model>", "input": ["...", ...] }`.
- For **Ollama**, Stratum automatically appends `/v1` to your configured
  endpoint (e.g. `http://localhost:11434` → `http://localhost:11434/v1`) and
  uses `embedding`‑capable models like `nomic-embed-text`.
- For **OpenAI / custom OpenAI-compatible endpoints**, the endpoint is used as
  configured. Point it at the `/v1` base (e.g. `https://api.openai.com/v1`).
- The **model** is selected from the model list you assign the `embedding`
  capability to in Settings → AI. If no model has that capability, the default
  chat model is used instead.
- An **API key** (when configured) is sent as `Authorization: Bearer <key>`.
  For OpenAI-compatible providers the `OPENAI_API_KEY` environment variable is
  respected as a fallback.

Requests carry a 120‑second timeout, are retried with capped exponential
backoff on transient failures and HTTP 429/5xx responses, and errors include
the endpoint's message so callers can surface it to the user.

**Requirements**

- The endpoint must implement the OpenAI-compatible `POST /v1/embeddings`
  contract (returns `{ data: [{ embedding: [...] }] }`). LocalAI, Ollama and
  most OpenAI-compatible servers do.
- Assign the `embedding` capability to an embedding model in Settings → AI —
  a small embedding model shouldn't be used for chat, and vice versa.
- Only `https://` URLs, or `http://` URLs pointing at local/private hosts
  (`localhost`, `127.0.0.1`, `10.x`, `172.16‑31.x`, `192.168.x`, `.local`),
  are accepted, to prevent SSRF.

## Text-to-Speech (TTS)

Stratum can synthesize speech from text using the **same configured AI
endpoint** you set up in Settings → AI, via the OpenAI-compatible
`POST {endpoint}/v1/audio/speech` route. A `tts` capability on a model in the
AI settings is required.

- **Trigger**: In **Settings → AI → Text-to-Speech**, click **Test / Play
  voice**. Stratum sends a sample sentence to the endpoint and plays the
  returned audio in place.
- **Endpoint**: Empty TTS endpoint = use the AI endpoint. To use a different
  server, set an explicit TTS endpoint and optional API key.
- **Model**: selected from the model list you assign the `tts` capability to.
  If no model has that capability, the default chat model is used.
- **Voice / format / speed**: the OpenAI speech API shape
  (`voice`, `response_format`, `speed`) is used; see the
  [Configuration reference](../getting-started/configuration.md).

Requests carry a 120‑second timeout and are retried with capped exponential
backoff on transient failures and HTTP 429/5xx responses. Errors surface the
endpoint's message.

**Requirements**

- A model with the `tts` capability assigned in Settings → AI.
- The endpoint must implement the OpenAI-compatible `POST /v1/audio/speech`
  contract (returns raw audio bytes). LocalAI, OpenAI and compatible servers
  do.

## AI Transform Actions

The AI can transform block content directly in the editor. Select text and choose an action:

| Action | Description |
|--------|-------------|
| **Rewrite** | Improve clarity and flow while preserving meaning |
| **Format** | Clean up markdown, fix syntax, consistent headings |
| **Structure** | Organize notes into hierarchical sections |
| **Summarize** | Condense text while preserving key points |
| **Connect** | Add relevant `[[wiki-links]]` to related concepts |
| **Generate Mermaid** | Create a diagram from a text description |

## AI Chat (Slash Menu)

Type `/` in the editor to open the AI slash menu. This gives you access to inline AI operations.

<!-- SCREENSHOT: [ai-slash-menu] AI slash menu with available actions -->

## RAG Chat — "Ask your notes"

When RAG is enabled, you can ask questions grounded in your own vault using the
**Ask Notes** panel (sidebar → **Ask Notes**). The pipeline is:

1. Your question is embedded with the configured embedding model
2. Relevant blocks are retrieved from the vault search index
3. Results are re-ranked by semantic similarity to your question
4. The LLM answers using the retrieved chunks as context, with citations to
   the source notes shown in the panel

When no matching notes are retrieved, the panel answers from general
knowledge and tells you no sources were found. If the configured endpoint is
down or misconfigured, the error is surfaced in the panel.

The same RAG engine is available from the command line:

```
stratum rag "your question" [--index] [--top-k N]
```

<!-- SCREENSHOT: [ai-ask-notes] Ask your notes panel with a cited answer -->

This means the AI answers based on *your* knowledge, not just its training data.

## Text-to-Speech — Read Aloud

Beyond the Settings test button, TTS playback is wired into two entry points:

- **Formatting toolbar** — select text in the editor and click **Read aloud**
  (🔊) to synthesize and play the selection via the configured endpoint.
- **Ask Notes panel** — click the speaker icon next to an answer to hear it
  read aloud.

Both surface endpoint errors inline if synthesis fails. Audio is synthesized
per request and played in place — nothing is written to disk.

## Interlink Notes

The **Connect** action scans a block and suggests `[[wiki-links]]` to related pages in your vault. This is useful for:

- Backfilling links when importing notes
- Discovering connections between separate topics
- Building out your knowledge graph automatically

## Tips

- **Ollama recommendation**: Use `llama3.2` for chat and `nomic-embed-text` for embeddings
- **RAG chunk count**: Start with 5 chunks. Increase for broader context, decrease for faster responses
- **Model capabilities**: Be intentional about which models get which capabilities — a small embedding model shouldn't be used for chat
- **Privacy**: With Ollama, everything runs locally — no data leaves your machine
