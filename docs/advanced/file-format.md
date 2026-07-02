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
├── any-page.md               # Pages can be at any depth
├── journals/
│   └── YYYY-MM-DD.md         # Daily notes
├── templates/
│   └── template-name.md      # Reusable templates
├── whiteboards/
│   └── name.excalidraw       # Excalidraw data (JSON)
└── .pkm/                     # Cache (ignore in Git)
```

## Round-Trip Fidelity

Stratum guarantees round-trip fidelity:

1. Read `.md` file → Parse blocks → Store in SQLite
2. Read from SQLite → Serialize blocks → Write `.md` file
3. The output matches the input (modulo UUID assignment for new blocks)

This means you can edit `.md` files externally, and Stratum will preserve your changes.
