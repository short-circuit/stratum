# Quick Start

Get up and running with Stratum in 5 minutes.

## First Launch

When you start Stratum for the first time, you'll see the **Vault Picker** screen:

<!-- SCREENSHOT: [vault-picker] Welcome screen showing the "Choose Vault Folder" button and onboarding text -->

1. Click **Choose Vault Folder**
2. Select an empty directory (or a new folder) — this will be your **vault**
3. Stratum creates a `.pkm/` cache directory inside the vault and loads the interface

!!! info "What's a vault?"
    A vault is simply a directory of Markdown files. All your notes live here as plain `.md` files.
    Stratum adds a hidden `.pkm/` folder for its cache (SQLite database, search index, config) —
    this can always be rebuilt from your `.md` files.

## Interface Overview

<!-- SCREENSHOT: [sidebar] Full sidebar with navigation tabs and page list -->

The main interface has three areas:

1. **Sidebar** (left) — Navigation tabs + page list
2. **Main area** (center) — The active panel (editor, graph, search, etc.)
3. **Page panels** — Backlinks, properties, suggested connections

### Sidebar Navigation

| Tab | Icon | Function |
|-----|------|----------|
| Journal | :material-calendar-month: | Daily notes |
| Pages | :material-file-document-outline: | Page list, create/delete |
| Graph | :material-hub: | Knowledge graph visualization |
| Search | :material-magnify: | Full-text block search |
| Query | :material-code-tags: | Datalog query interface |
| Templates | :material-clipboard-text-outline: | Reusable page templates |
| Flashcards | :material-cards: | Spaced repetition review |
| Whiteboards | :material-draw: | Excalidraw spatial canvas |
| Settings | :material-cog: | App configuration |

Click any tab to open its panel. The sidebar can be collapsed with the ◀ button.

## Your First Note

1. Click **Pages** in the sidebar (or press `/`)
2. Click **+ New**
3. Enter a path like `notes/my-first-note.md`
4. Optionally enter a title like `My First Note`
5. Click **Create**

<!-- SCREENSHOT: [create-page-dialog] The new page creation dialog with path and title fields -->

The page opens in the block editor. Start typing — every paragraph is a **block**.

```markdown
This is a block.
  This is a child block (indented).
    This is a deeper child.

TODO This is a task

[[Wiki-link to another page]]

#tag
```

## Create a Wiki-Link

Type `[[` to start a wiki-link. Stratum will autocomplete existing pages:

1. Create a second page: `notes/second-page.md`
2. Go back to your first note
3. Type `[[second` and select the autocomplete suggestion
4. The link renders as a clickable reference

<!-- SCREENSHOT: [wiki-link-autocomplete] Autocomplete dropdown showing wiki-link suggestions -->

## View the Graph

1. Click **Graph** in the sidebar
2. You'll see your two pages as connected nodes
3. Click a node to navigate to that page
4. Drag nodes to reposition them

<!-- SCREENSHOT: [graph-view] Force-directed graph showing two connected nodes -->

## What's Next?

- Learn about the [Block Editor](../guide/block-editor.md)
- Explore [Wiki-Links and Backlinks](../guide/linking-and-backlinks.md)
- Set up [Git Sync](../guide/git-sync.md) to backup your vault
- Configure [AI features](../guide/ai-features.md) with Ollama or OpenAI
