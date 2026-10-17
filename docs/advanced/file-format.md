# File Format Specification

Stratum stores notes as plain Markdown files with an extended block-based syntax.

## Frontmatter

Every note can have YAML frontmatter delimited by `---`:

```markdown
---
title: My Note
tags: [project, documentation]
created: 2026-06-22
custom_field: value
---
```

Standard frontmatter fields:

| Field | Type | Description |
|-------|------|-------------|
| `title` | String | Page title |
| `tags` | Array | List of tags |
| `created` | Date | Creation date |

Any custom fields are preserved but not interpreted by Stratum.

## Block Structure

Blocks are serialized as indented list items. Each block can have properties prefixed with `.`:

```markdown
- Top-level block content
  .id: a1b2c3d4-e5f6-7890-abcd-ef1234567890
  .marker: TODO
  .priority: A
  .collapsed: false
  - Child block content
    .id: b2c3d4e5-f6a7-8901-bcde-f12345678901
```

### Block Properties

| Property | Type | Description |
|----------|------|-------------|
| `.id` | UUID | Block identifier (auto-generated) |
| `.marker` | String | Task marker: `TODO`, `DOING`, `DONE` |
| `.priority` | String | Priority: `A`, `B`, `C` |
| `.collapsed` | Boolean | Whether children are collapsed |
| `.heading` | Number | Heading level: 1, 2, 3 |

Custom properties (any `.key: value` pair) are preserved in round-trip serialization.

## Block Reference Syntax

```markdown
# Block reference
Reference ((a1b2c3d4-e5f6-7890-abcd-ef1234567890)) in text.

# Page embed
{{embed [[Target Page]]}}

# Block embed
{{embed ((block-uuid))}}
```

## Property Syntax

Two forms are supported:

1. **Leading `.`** (internal properties):

```markdown
- Content
  .id: uuid
  .marker: TODO
```

2. **`::` suffix** (flashcard properties):

```markdown
- What is Rust?
  .question:: true
  .answer:: A systems programming language focused on safety and performance.
```

## File Organization

```
vault/
├── pages/                      # Knowledge pages — can be at any depth
│   ├── any-page.md
│   └── projects/
│       └── nested/note.md
├── journals/
│   └── YYYY-MM-DD.md           # Daily notes (not shown in the page tree)
├── templates/
│   └── template-name.md        # Reusable templates
├── whiteboards/
│   └── name.excalidraw         # Excalidraw data (JSON)
└── .pkm/                       # Cache (ignore in Git)
    └── config.toml             # Per-vault configuration (see below)
```

Storage rules:

- **Pages live under `pages/`.** Every page is addressed by its vault-relative path `pages/<subfolder>/<note>.md`. Nested paths are allowed; parent folders are created automatically on create/move.
- **Journals live under `journals/`** and are excluded from the page list tree (use the Journal panel).
- **No maximum nesting depth.** Any path under `pages/` is valid; the tree view is presentation-only and never re-orders files on disk.

## Per-Vault Configuration (`config.toml`)

Stratum stores per-vault settings in `.pkm/config.toml`. Relevant keys:

```toml
[layout]
# Directory where knowledge pages are stored (vault-relative).
pages_dir = "pages"
# Journal files directory (vault-relative).
journals_dir = "journals"
# Recording clips directory (vault-relative).
recordings_dir = "assets/recordings"
# Page list rendering mode: "flat" (default) or "tree" (folder hierarchy).
# Existing vaults load as "flat" unless this key was set explicitly.
pages_view = "flat"
```

`pages_view` does not change the file layout — it only controls how the page list is rendered (flat list vs. folder tree). It is also editable from the UI under **Settings → Vault → Pages layout**.

## Page Identity

A page's identity is its **vault-relative path** — not its title. Details:

- One page = one path = one graph node. Two notes with the same base name in different folders are distinct nodes.
- `[[wiki-links]]` by bare name resolve to exactly one page only when unique; path-qualified targets (`[[pages/folder/note]]`) are always unambiguous (see [Links and Backlinks](../guide/linking-and-backlinks.md)).
- Moving/renaming a page (in-app **Move / rename…**) rewrites inbound path/slug links; external `mv` does not (name links survive, path links dangle until rewritten).



## Round-Trip Fidelity

Stratum guarantees round-trip fidelity:

1. Read `.md` file → Parse blocks → Store in SQLite
2. Read from SQLite → Serialize blocks → Write `.md` file
3. The output matches the input (modulo UUID assignment for new blocks)

This means you can edit `.md` files externally, and Stratum will preserve your changes.
