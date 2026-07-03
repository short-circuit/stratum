---

# Stratum

**A privacy-first, offline-capable personal knowledge management system.**

Notes are plain Markdown files on disk — zero vendor lock-in.
Every paragraph is an addressable block. Wiki-links, graph visualization, Datalog queries,
Git sync, and AI-augmented search — all powered by a Rust core.

<div class="grid cards" markdown>

-   :material-file-document-outline: **Plain Markdown**

    Your notes are `.md` files readable by any editor. No proprietary format.

-   :material-lock-outline: **Fully Offline**

    No cloud dependency. All features work without internet.

-   :material-graph: **Knowledge Graph**

    Interactive force-directed graph of `[[wiki-links]]` with connected components.

-   :material-database-search: **Full-Text Search**

    Tantivy-powered search, sub-100ms at 10,000+ notes.

-   :material-code-braces: **Datalog Queries**

    Query your knowledge graph with Datalog — EDN or JSON syntax.

-   :material-robot: **AI-Powered**

    Pluggable LLM providers. Rewrite, summarize, link, research with RAG.

-   :material-source-branch: **Git Sync**

    Auto-commit, push, pull. Your version history is built-in.

-   :material-puzzle: **Extensible**

    WASM plugin runtime. Customize without forking.

</div>

---

## Quick Start

```bash
# Nix (recommended)
nix develop ./nix

# Or build manually
cargo build --workspace
npm install
cargo tauri dev
```

See the [Installation Guide](getting-started/installation.md) for detailed setup.

## Features at a Glance

| Feature | Description |
|---------|-------------|
| :material-format-list-bulleted: **Block Outliner** | Every paragraph is a UUID-addressed block. Indent, outdent, drag, collapse. |
| :material-link-variant: **Wiki-Links** | `[[Page Name]]`, `[[Target\|Display]]`, `((block-ref))`, `{{embed}}` |
| :material-pound: **Tags** | `#tag` inline and YAML frontmatter tags |
| :material-checkbox-marked: **Tasks** | `TODO`, `DOING`, `DONE` markers with priorities |
| :material-graph: **Graph View** | Force-directed graph, connected components, orphan detection |
| :material-magnify: **Search** | Full-text block search via Tantivy. Tag search with `#tag` |
| :material-table: **Datalog** | Query blocks with Datalog: `:find`, `:where`, `:pull` |
| :material-calendar: **Journal** | Daily notes, auto-created |
| :material-file-code: **Templates** | Reusable templates with variable substitution |
| :material-cards: **Flashcards** | Spaced repetition (SRS) with `question::`/`answer::` syntax |
| :material-draw: **Whiteboards** | Excalidraw spatial canvas |
| :material-sigma: **Math** | KaTeX inline and display equations with symbol palette |
| :material-graph: **Mermaid** | Flowcharts, sequence diagrams, Gantt charts inline |
| :material-brain: **AI** | Rewrite, structure, summarize, connect with AI. RAG chat |
| :material-web: **Web Research** | SearXNG-powered multi-depth research |
| :material-source-branch: **Git Sync** | Auto-commit, remote push/pull, conflict detection |
| :material-export: **Export** | HTML and JSON export |
| :material-console: **CLI** | Full CLI: init, list, search, stats, graph, sync, export |
| :material-puzzle: **Plugins** | WASM plugin runtime |

---

## Why Stratum?

**No vendor lock-in.** Your knowledge is stored as plain Markdown files. If Stratum disappears tomorrow, you still have all your notes — readable by any text editor.

**Privacy by design.** Everything runs locally. AI providers are optional and self-hosted (Ollama) or configured to your own API keys.

**Performance.** Rust backend means sub-100ms search, instant graph rendering, and minimal memory usage (<80MB idle).

**Block-based.** Like Logseq, every paragraph is a first-class entity with a UUID. Link to specific blocks, embed them in other pages, query them with Datalog.

---

## Project Status

Stratum is in active development. Version 0.4.2. All core features are implemented and working.

- **License:** AGPL-3.0-only
- **Repository:** [github.com/short-circuit/stratum](https://github.com/short-circuit/stratum)
