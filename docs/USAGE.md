# Stratum Usage Guide

## Setup

### Nix (recommended)

One command to get all dependencies (Rust, Node, Tauri system libraries):

```bash
nix develop ./nix
```

Or use [direnv](https://direnv.net/) for automatic activation:

```bash
direnv allow   # auto-loads the flake on every cd
```

### Manual Setup

Requires Rust 1.75+, Node.js 22+, and Tauri v2 system libraries.
See [contributing.md](./contributing.md) for platform-specific instructions.

### First Launch

```bash
npm install            # one-time frontend dependency install
cargo tauri dev        # starts the desktop app
```

The app creates a vault at `~/StratumVault` on first run. All notes live there as plain `.md` files.

---

## Vault

Your vault is a directory of Markdown files. Stratum never locks you in — open your notes in any text editor.

```
~/StratumVault/
├── note-a.md
├── note-b.md
├── journals/
│   └── 2026-06-22.md
├── .pkm/                 # metadata cache (rebuildable)
│   ├── blocks.db         # SQLite block storage
│   ├── search.idx        # Tantivy search index
│   └── config.toml       # settings
```

---

## Sidebar Navigation

The left sidebar provides access to all features. It can be collapsed (◀) and expanded (▶).

| Tab | Shortcut | Purpose |
|-----|----------|---------|
| 📅 Journal | `/journal` | Daily notes |
| 📄 Pages | `/` | Page list, create/delete |
| 🔗 Graph | `/graph` | Force-directed note graph |
| 🔍 Search | `/search` | Full-text block search |
| ▷ Query | `/query` | Datalog queries |
| 📋 Templates | `/templates` | Reusable page templates |
| 🃏 Flashcards | `/flashcards` | Spaced repetition cards |
| 🎨 Whiteboards | `/whiteboards` | Spatial canvas (Excalidraw) |
| ⚙ Settings | `/settings` | App configuration |

---

## Pages

### Creating a Page

1. Click **Pages** in the sidebar
2. Click **+ New**
3. Enter a path (e.g., `notes/my-page.md`) and optional title
4. Click **Create**

The page is saved as a `.md` file in your vault. You can also create `.md` files manually in the vault directory — they appear automatically after refreshing.

### Editing a Page

Click any page in the list to open it. The editor supports:

- **Blocks** — every paragraph is an addressable block with a UUID
- **`[[Wiki-links]]`** — link to other pages: `[[Target Page]]` or `[[Target Page|Display Text]]`
- **`#tags`** — inline tags and frontmatter `tags:` lists
- **Task markers** — `TODO`, `DOING`, `DONE`
- **Indent/outdent** — Tab / Shift+Tab

### Deleting a Page

Hover a page in the sidebar and click the × button. Confirm the deletion.

---

## Graph Engine

The graph visualizes connections between your notes.

### How It Works

- Stratum scans all `.md` files in your vault
- It extracts `[[wiki-links]]` to build a directed graph (nodes = pages, edges = links)
- Nodes are sized by their **degree** (number of incoming + outgoing links)
- Nodes are colored by their **tags** (deterministic hue from tag name)

### Views

| Mode | Description |
|------|-------------|
| **Full Graph** | All nodes and edges |
| **Connected Components** | Groups of interlinked notes (clusters). Use the dropdown to cycle through components sorted by size |
| **Orphaned Notes** | Pages with zero connections — candidates for linking or deletion |

### Interactions

- **Drag** nodes to reposition them
- **Hover** a node to highlight it and its connected neighbors
- **Click** a node to navigate to that page
- **Scroll** to zoom in/out
- **Search** to filter nodes by title, slug, or tag
- **Refresh** to rebuild the graph from disk

### If You See No Nodes

The graph is empty because your vault has no `.md` files yet. Create pages first:

```bash
# Quick test — create two linked notes
mkdir -p ~/StratumVault
echo -e '---\ntitle: Alpha\n---\nSee also [[Beta]].' > ~/StratumVault/alpha.md
echo -e '---\ntitle: Beta\n---\nLinked from [[Alpha]].' > ~/StratumVault/beta.md
```

Refresh the Graph tab to see two connected nodes.

---

## Search

Full-text search across all block content, powered by Tantivy.

1. Click **Search** in the sidebar
2. Type a query (supports keywords and regex)
3. Results show matching blocks with context snippets and scores

Use the **Rebuild Index** button to re-index all blocks from SQLite if search results seem stale.

---

## Datalog Queries

Query your notes with Datalog, compiled to SQL against the block database.

### Example Queries

```clojure
;; Find all blocks containing "rust"
[:find ?content ?page
 :where [?b :block/content ?content]
        [?b :block/page-path ?page]
        [(stratum/like ?content "%rust%")]]

;; Find all pages with the "project" tag
[:find ?title ?path
 :where [?p :page/title ?title]
        [?p :page/path ?path]
        [?p :page/tags "project"]]

;; Pull full block data
[:find (pull ?b [:block/id :block/content :block/marker])
 :where [?b :block/marker "TODO"]]
```

Results appear as a table below the query input.

---

## Templates

Create reusable page templates with variable substitution.

1. Click **Templates** in the sidebar
2. Create a template with `{{variable}}` placeholders
3. Apply it to a new page, providing values for each variable

---

## Flashcards

Spaced-repetition flashcards generated from your notes.

- **Generate** cards from markdown content (supports `Q:` / `A:` format)
- **Review** cards with quality ratings (0–5)
- The system schedules reviews based on your performance (SM-2 algorithm)

---

## Whiteboards

A spatial canvas powered by [Excalidraw](https://excalidraw.com) for freeform drawing and diagramming.

- Create, rename, and delete whiteboards
- Drawing tools, shapes, text, and connectors
- Saved as files in your vault

---

## Settings

Configure the app via the ⚙ Settings tab or by editing `.pkm/config.toml` directly.

| Section | Options |
|---------|---------|
| **Vault** | Vault path on disk |
| **Theme** | Dark mode toggle, primary/secondary colors, font size |
| **AI** | Provider (Ollama / OpenAI / Anthropic / Custom), model, endpoint, RAG settings |
| **Sync** | Git remote URL, branch, sync mode, auto-commit interval |

---

## Wiki-Link Syntax

Stratum supports Logseq-style wiki-links anywhere in your notes:

| Syntax | Result |
|--------|--------|
| `[[Page Name]]` | Link to a page (resolved if the `.md` file exists) |
| `[[Page Name|Display Text]]` | Link with custom display text |
| `[[Page\|Name]]` | Pipe in page name (escaped with `\|`) |
| `[[Target\|mid\|Display]]` | Multiple pipes — last one is display text |

Links are extracted from both frontmatter and body text. The graph engine uses them to build connections.

---

## Git Sync

Stratum can sync your vault with a remote Git repository.

### Modes

| Mode | Behavior |
|------|----------|
| **Manual** | Click Sync to pull, merge, and push |
| **Auto-commit** | Every save triggers a commit |
| **Auto-sync** | Auto-commit + periodic push/pull |

Configure remote URL and branch in Settings. Conflicts are detected and reported.

---

## Export

Export your vault as static HTML or JSON:

- **HTML** — each page becomes an `index.html` in its own directory, with navigation
- **JSON** — full vault dump for backup or processing

Access via the Export button in the sidebar footer (◀ ▶ area).

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Indent block |
| `Shift+Tab` | Outdent block |
| `Enter` | New block |
| `Ctrl+Enter` | Toggle task marker (TODO → DOING → DONE → none) |
| `Backspace` | Delete block (when empty) |

---

## Troubleshooting

### Graph shows no nodes
Make sure your vault has `.md` files. The vault path is displayed in the empty state message. Create pages or add `.md` files manually, then click **Refresh**.

### Search returns no results
Click **Rebuild Index** in the Search tab. The search index is initialized from `.md` files at startup.

### npm install fails
The project needs node_modules in the workspace root. Run `npm install` once before `cargo tauri dev`.

### Nix flake fails
Make sure the flake files are git-tracked:
```bash
git add nix/ rust-toolchain.toml .envrc
```
Nix flakes only see files known to Git.
