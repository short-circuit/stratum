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

## Phase 1A: Rust Block Engine ✅ COMPLETE

### [x] pkm-block crate (new)
- [x] `block.rs` — Block struct with UUID, content, parent_id, left_id, properties, marker, priority, meta
- [x] `tree.rs` — BlockTree with O(1) insert/delete/move (parent_id + left_id model)
- [x] `ops.rs` — insert_block, delete_block, move_block, indent, outdent, split, merge
- [x] `page.rs` — Page struct (collection of blocks + frontmatter)
- [x] `store.rs` — SQLite storage (blocks.db): read/write blocks, pages, links
- [x] 53 tests, all passing

### [x] pkm-core rewrite
- [x] Add StorageBackend, StorageConfig, VaultLayout
- [x] Add BlockNotFound, PageNotFound, CycleDetected error variants
- [x] Keep Note/Frontmatter/Backlink for backward compat
- [x] 28 tests, all passing

### [x] pkm-markdown rewrite
- [x] Parse block-based markdown (indented bullets, `.id:` properties, task markers)
- [x] Parse `((uuid))` block refs (via existing linker)
- [x] Parse `{{embed ...}}` syntax (block and page embeds)
- [x] Serialize blocks back to `.md` with round-trip fidelity
- [x] Keep existing tag/link extraction logic
- [x] 65 tests, all passing

### [x] pkm-index extension
- [x] Block-level Tantivy schema (BlockIndex: id, content, page_path, marker, priority, properties)
- [x] Block-level graph (BlockGraph: block→block and block→page edges, backlinks, unlinked mentions)
- [x] 45 tests, all passing

### [x] pkm-query crate (new)
- [x] Datalog parser (EDN and JSON dual-mode input)
- [x] Datalog→SQL compiler targeting blocks.db (15 attribute mappings)
- [x] Runtime query execution (QueryEngine)
- [x] Support: :find (vars + pull), :where (e-a-v patterns), page joins
- [x] 16 tests, all passing

**Total: 338 tests, 0 failures**

---

## Phase 1B: Tauri Frontend Foundation ✅ COMPLETE

### [x] Project setup
- [x] Tauri v2 init in `src-tauri/` (integrated with workspace)
- [x] React 19 + TypeScript + Vite frontend in `src/`
- [x] Tailwind CSS v4 via @tailwindcss/vite plugin
- [x] Zustand state management
- [x] React Router v7 with routes: /, /page/:path, /search, /query
- [x] 17 Rust Tauri commands: vault, page CRUD, block CRUD, search, backlinks, datalog query, git sync
- [x] TypeScript types and IPC bindings for all commands

### [x] UI Foundation
- [x] App shell with sidebar (260px) + main content layout
- [x] Sidebar: page tree with create/delete, vault info stats, navigation tabs
- [x] PageView: loads blocks for page, displays header with metadata
- [x] BlockEditor: inline bullet-point editing, task markers (TODO/DOING/DONE)
- [x] SearchPanel: full-text search bar with results list
- [x] QueryPanel: Datalog query input with auto-refresh and result table
- [x] Dark mode support (Tailwind `dark:` variants)
- [x] Tauri IPC types and bindings in `src/lib/`

### Build verification
- Frontend: `npm run build` → 255KB JS + 15KB CSS
- Backend: `cargo build --workspace` → compiles cleanly
- Full Rust test suite: 338 tests, 0 failures

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
