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

## RAG Chat

When RAG is enabled, AI operations include context from your notes:

1. Search your vault for relevant blocks
2. Concatenate matching content as context
3. Send the context + your prompt to the LLM
4. Return results with citations to source notes

<!-- SCREENSHOT: [ai-rag-chat] AI chat with RAG context showing source citations -->

This means the AI answers based on *your* knowledge, not just its training data.

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
