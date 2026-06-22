# Stratum Master Plan

Transform Stratum from a page-centric PKM into a Logseq-style block-based outliner PKM with Tauri + React frontend and Rust backend.

## Architecture Decisions

| Decision | Choice |
|----------|--------|
| Storage | Hybrid: SQLite + .md files |
| Format | Own block syntax (Logseq-inspired, `.id:` prefix) |
| UI | Tauri v2 + React + TypeScript |
| Editor | BlockNote (ProseMirror/Tiptap) |
| State management | Zustand |
| Styling | Tailwind CSS |
| Queries | Datalog from day one |
| Sync | Git (files) + custom protocol (blocks) |
| Whiteboards | Phase 2 (Tldraw) |

## Data Format

Blocks serialized as indented markdown bullets with `.`-prefixed internal properties:

```markdown
---
title: My Page
tags: [project, rust]
---
- First block content
  .id: 65f8a1e2-3a4b-...
  .priority: A
  - Child block with **markdown**
    .id: 65f8a1e3-...
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│                  Tauri Desktop Shell                      │
│  ┌──────────────────────────────────────────────────┐   │
│  │         React + TypeScript Frontend               │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────────┐  │   │
│  │  │ BlockNote │ │  Graph   │ │  Search / Chat   │  │   │
│  │  │ Outliner  │ │  (D3)    │ │                  │  │   │
│  │  └────┬─────┘ └────┬─────┘ └────────┬─────────┘  │   │
│  │       │           │                │             │   │
│  │  ┌────┴───────────┴────────────────┴──────────┐  │   │
│  │  │   State (Zustand) + Tauri invoke()          │  │   │
│  │  └────────────────────┬───────────────────────┘  │   │
│  └───────────────────────┼──────────────────────────┘   │
│                           │ Tauri IPC (invoke)            │
│  ┌───────────────────────┼──────────────────────────┐   │
│  │            Rust Backend (same process)             │   │
│  │  ┌──────────┬──────────┬──────────┬────────────┐  │   │
│  │  │ pkm-     │ pkm-     │ pkm-     │ pkm-query  │  │   │
│  │  │ block    │ markdown │ index    │ (Datalog)  │  │   │
│  │  ├──────────┼──────────┼──────────┼────────────┤  │   │
│  │  │ pkm-sync │ pkm-     │ pkm-ai   │ pkm-plugin │  │   │
│  │  │          │ watcher  │          │            │  │   │
│  │  └──────────┴──────────┴──────────┴────────────┘  │   │
│  └────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

---

## Phase 1A: Rust Block Engine

### [ ] pkm-block crate (new)
- [ ] `block.rs` — Block struct with UUID, content, parent_id, left_id, properties, marker, priority, meta
- [ ] `tree.rs` — BlockTree with O(1) insert/delete/move (parent_id + left_id model)
- [ ] `ops.rs` — insert_block, delete_block, move_block, indent, outdent, split, merge
- [ ] `page.rs` — Page struct (collection of blocks + frontmatter)
- [ ] `store.rs` — SQLite storage (blocks.db): read/write blocks, pages, links
- [ ] `sync.rs` — Block-level change journal for sync protocol

### [ ] pkm-core rewrite
- [ ] Replace `Note` with `Page` structure
- [ ] Add `BlockRef`, `Embed` types
- [ ] Keep Config, Error, Types, VaultPath

### [ ] pkm-markdown rewrite
- [ ] Parse block-based markdown (indented bullets, `.id:` properties, task markers)
- [ ] Parse `((uuid))` block refs
- [ ] Parse `{{embed ...}}` syntax (block and page embeds)
- [ ] Serialize blocks back to `.md` with round-trip fidelity
- [ ] Keep existing tag/link extraction logic

### [ ] pkm-index extension
- [ ] Block-level Tantivy schema (id, content, page, parent_id, properties, marker)
- [ ] Block-level graph (block→block and block→page edges)
- [ ] Block-level backlinks and unlinked mentions

### [ ] pkm-query crate (new)
- [ ] Datalog parser (Logseq-compatible subset)
- [ ] Datalog→SQL compiler targeting blocks.db
- [ ] Runtime query execution
- [ ] Support: find, where, pull, or, not, and

### [ ] pkm-sync update
- [ ] Keep git engine as-is
- [ ] Add SyncJournal for block-level changes

---

## Phase 1B: Tauri Frontend Foundation

### [ ] Project setup
- [ ] Tauri v2 init in stratum/
- [ ] React + TypeScript + Vite frontend
- [ ] Tailwind CSS setup
- [ ] Zustand state management
- [ ] Rust Tauri commands: open_page, save_page, insert_block, move_block, delete_block, search_blocks, run_query, get_graph, get_backlinks, sync_vault

### [ ] UI Foundation
- [ ] App shell with sidebar + main content layout
- [ ] Sidebar: page tree, journal calendar, tag list
- [ ] Routing: journal, page, graph, search, query, settings
- [ ] Theme support (light/dark)
- [ ] Tauri IPC types and bindings

---

## Phase 2: Full Outliner Experience

### [ ] BlockNote Editor
- [ ] Custom block types: task, heading, code, quote
- [ ] Slash menu: /TODO, /page, /embed, /query, /template
- [ ] Keyboard shortcuts: Enter, Shift+Enter, Tab/Shift+Tab, Ctrl+Enter, Backspace
- [ ] Block drag handle for reordering
- [ ] Block context menu (copy ref, embed, delete, properties)
- [ ] Inline autocomplete: [[ pages, (( blocks, # tags
- [ ] Collapse/expand toggle on parent blocks
- [ ] Multi-block selection for bulk operations

### [ ] Journal System
- [ ] Today's journal auto-opens on launch
- [ ] Calendar sidebar for date navigation
- [ ] Journal stored as journals/YYYY_MM_DD.md
- [ ] Daily note template support

### [ ] Properties Panel
- [ ] Click block → properties sidebar
- [ ] Inline property rendering in blocks
- [ ] Property types: text, number, date, checkbox, URL, page ref, block ref

### [ ] Backlinks & References
- [ ] Bottom of page: linked references + unlinked mentions
- [ ] Block-level backlinks with context snippet
- [ ] Click to navigate, hover for preview

### [ ] Datalog Query View
- [ ] Query input with syntax highlighting
- [ ] Result rendering: table view, list view
- [ ] Saved queries
- [ ] Query templates

---

## Phase 3: Polish & Advanced

### [ ] Whiteboards
- [ ] Tldraw-based spatial canvas
- [ ] Block/page drag onto canvas
- [ ] Drawing tools

### [ ] Flashcards / SRS
- [ ] Card generation from blocks
- [ ] Spaced repetition scheduling

### [ ] PDF Annotation
- [ ] PDF.js viewer
- [ ] Highlight-to-block references

### [ ] Templates
- [ ] Reusable block templates
- [ ] Variable substitution

### [ ] Export
- [ ] Static site generation
- [ ] JSON, Markdown export

---

## Implementation Order

```
Week 1-2:   pkm-block (model + tree + ops + store)
Week 2-3:   pkm-markdown rewrite + pkm-core rewrite
Week 3-4:   pkm-index extension + pkm-sync update
Week 4-5:   pkm-query (Datalog engine)
Week 5-6:   Tauri project setup, Rust commands, types
Week 6-8:   React frontend foundation (sidebar, routing, state)
Week 8-10:  BlockNote outliner editor integration
Week 10-12: Journal, properties, backlinks, autocomplete
Week 12-14: Search, graph (D3), query view, settings
Week 14-16: Task management, testing, bug fixes
Week 16-20: Whiteboards, flashcards, templates, polish
```
