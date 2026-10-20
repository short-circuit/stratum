# Stratum v1.0 beta — announcement kit

Ready-to-post copy for Stratum v1.0.0-beta.3. Publish where you like; every
snippet is self-contained and can be copy-pasted verbatim.

- Repo: https://github.com/short-circuit/stratum
- Docs: https://short-circuit.github.io/stratum/docs/
- Download: https://github.com/short-circuit/stratum/releases/tag/v1.0.0-beta.3
- License: AGPL-3.0-only
- Version: v1.0.0-beta.3 (2026-10-20)

---

## Hacker News — post (title + URL)

Title (keep under 80 chars):

```
Show HN: Stratum — offline-first PKM, plain .md files, Git sync (AGPL)
```

Submission URL: `https://github.com/short-circuit/stratum`

First comment (HN accepts a single url submission; post this as your comment
so the story is not self-promotional-flavored only):

> Stratum is a personal knowledge management system I've been building for
> about two years. Every note is a plain `.md` file on disk — no proprietary
> database you can't read, no cloud dependency. All features work offline;
> AI is optional (bring your own Ollama/LM Studio/LocalAI endpoint).
>
> The thing I keep coming back to is the block outliner: every paragraph is
> an addressable block with a UUID, so you can link to a single thought, not
> just a page. On top of that: wiki-links with backlinks and unlinked
> mentions, a force-directed knowledge graph, Datalog queries (Logseq-style),
> a daily journal, templates, flashcards (SM-2), whiteboards (Excalidraw),
> KaTeX + Mermaid, full-text search (Tantivy, sub-100ms at 10k notes), and
> Git sync with auto-commit.
>
> It is AGPL-3.0, and the whole pipeline — parsing, indexing, graph, search,
> sync — runs in Rust. Desktop (Linux/macOS/Windows) and Android build
> targets exist; the UI is localized in en/de/it/fr/es (zh/ja/el in progress).
> v1.0.0-beta.3 ships Linux .deb/.rpm/.AppImage + checksums, macOS and
> Windows bundles.
>
> Happy to answer anything — especially if you're coming from Obsidian or
> Logseq and hit a missing piece.

## r/selfhosted — title + body

Title:

```
Stratum — offline-first PKM with plain .md files and Git sync (AGPL)
```

Body:

> Hi all — I've been developing Stratum for a couple of years and just put
> out v1.0.0-beta.3. It's a privacy-first personal knowledge management
> system that does NOT lock you in: every note is a plain Markdown file on
> disk, so if the app disappeared tomorrow you'd lose nothing. No cloud
> dependency, fully offline, AI optional (Ollama/LM Studio/LocalAI).
>
> Feature highlights:
>
> - Block outliner (Logseq-style, every paragraph is an addressable block)
> - Wiki-links, backlinks, unlinked mentions, autocomplete
> - Knowledge graph (force-directed, connected components, orphans)
> - Datalog queries in Logseq-compatible EDN syntax
> - Daily journal, templates, flashcards (SM-2), whiteboards (Excalidraw)
> - KaTeX equations + Mermaid diagrams inline
> - Full-text search (Tantivy, sub-100ms at 10k notes)
> - Git sync: auto-commit, push/pull — version history built in
> - Importers for Obsidian, Logseq, and generic Markdown vaults
> - MCP server so external AI clients can query your notes
> - WASM plugin runtime + plugin directory/theme hub
>
> Ships as Linux .deb/.rpm/.AppImage (with SHA256SUMS), macOS and Windows
> bundles, and an Android build. AGPL-3.0, Rust core.
>
> Repo: https://github.com/short-circuit/stratum
> Docs: https://short-circuit.github.io/stratum/docs/
>
> Feedback welcome, especially around migration from other tools.

## r/ObsidianMD — title + body (community-aware angle)

Title:

```
[Discussion] Stratum — an AGPL, offline-first Obsidian alternative that stores notes as plain .md (with a block outliner)

```

Body:

> Not a replacement pitch — I use both, but I've been working on Stratum as
> an experiment in what "no lock-in" really means, and v1.0.0-beta.3 is out.
>
> The core difference from Obsidian: notes are stored as plain Markdown on
> disk, but the editor is a **block outliner** (Logseq-style). Every
> paragraph is an addressable block with a UUID, so you can link to and
> query individual thoughts. It's fully offline (AI is pluggable — Ollama,
> LM Studio, LocalAI), and sync is native Git (auto-commit, push/pull).
>
> What I personally think is mature enough to be interesting:
>
> - Wiki-links + backlinks + unlinked mentions (with hover previews)
> - Force-directed knowledge graph with orphan detection
> - Datalog queries in Logseq-compatible syntax
> - Full-text search via Tantivy
> - KaTeX + Mermaid, flashcards (SM-2), templates, whiteboards
> - One-click importers for Obsidian and Logseq vaults
> - MCP server (so you can query your vault from an LLM client)
>
> It's AGPL-3.0 with a Rust core; available for Linux (deb/rpm/AppImage),
> macOS, Windows and Android. UI is localized en/de/it/fr/es.
>
> What would make you actually switch? Genuinely asking — the hardest part
> of any PKM is the migration, and I'd like to know what's missing.
>
> https://github.com/short-circuit/stratum

## Lobsters — post

Title:

```
Stratum — offline-first PKM, plain .md files, Git sync (AGPL)
```

URL: `https://github.com/short-circuit/stratum`

Description field:

> Stratum is a personal knowledge management system that stores every note as
> a plain Markdown file on disk and runs fully offline. Block-based outliner
> where every paragraph has a UUID and can be linked/queried individually;
> wiki-links, backlinks, knowledge graph, Datalog queries, full-text search,
> Git sync, MCP server, WASM plugin runtime, and an optional local AI stack.
> Rust core (Tantivy, git2, SQLite), AGPL-3.0, v1.0.0-beta.3. Desktop
> bundles for Linux/macOS/Windows plus Android build.

## Mastodon — short posts (<=500 chars each)

Post 1:

```
Stratum v1.0.0-beta.3 is out — an offline-first PKM that stores every note
as a plain .md file on disk.

• Block outliner, every block addressable
• Wiki-links, backlinks, knowledge graph
• Datalog queries, flashcards, whiteboards
• Git sync built in
• Fully offline, AGPL-3.0, Rust core

https://github.com/short-circuit/stratum
#opensource #selfhosted #PKM
```

Post 2 (thread continuation, language/localization angle):

```
The UI is localized in en/de/it/fr/es (zh/ja/el in progress) and it ships
Linux .deb/.rpm/.AppImage + macOS/Windows bundles with SHA256 checksums.
AI is optional — point it at your own Ollama/LM Studio/LocalAI instance.

Docs: https://short-circuit.github.io/stratum/docs/
#stratum #notes #markdown
```

## Key Points cheat sheet (for replying in comments)

- AGPL-3.0-only, Rust core, plain .md files, no vendor lock-in
- Fully offline; AI optional via local endpoints (Ollama, LM Studio, LocalAI, SearXNG research)
- v1.0 features: block outliner, wiki-links + backlinks + unlinked mentions, graph (connected components/orphans), Datalog queries (EDN + JSON), journal, templates, flashcards, whiteboards, KaTeX + Mermaid, attachments, reminders, plugins (WASM + directory), MCP server, voice dictation, export (PDF/ZIP), git sync
- i18n: en/de/it/fr/es complete; zh/ja/el placeholders in progress
- Release: v1.0.0-beta.3 — Linux .deb/.rpm/.AppImage + SHA256SUMS, macOS .zip, Windows .exe, Android .apk/.aab
- Performance: <100ms search at 10k notes, <80MB idle memory
- Honest caveats: beta (API may shift), mobile is Android-first, no end-to-end encryption on sync (git-level), no collaborative editing

## Publishing checklist

- [ ] HN: https://news.ycombinator.com/submit (title, url, comment)
- [ ] r/selfhosted: https://www.reddit.com/r/selfhosted/submit (link post)
- [ ] r/ObsidianMD: https://www.reddit.com/r/ObsidianMD/submit (link post; mention it's an alternative, not a replacement)
- [ ] Lobsters: https://lobste.rs/submit (title, url, description)
- [ ] Mastodon: post 1 + post 2 (or a single shorter post if you prefer), mention @stratum if account exists
- [ ] Optional: r/noteapps, r/KnowledgeManagement, Hacker News monthly "Who wants to be hired" (if looking for contributors)
