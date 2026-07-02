# Block Editor

Stratum's editor is a **block-based outliner** — every paragraph is a first-class entity with a UUID. Think Logseq-style editing with the power of BlockNote (ProseMirror).

<!-- SCREENSHOT: [block-editor] The block editor showing indented blocks with different markers -->

## What is a Block?

A block is a single unit of content — a paragraph, a heading, a list item, a task, or a code block. Each block has:

- **Content** — the text/markdown content
- **UUID** — a unique identifier for linking and referencing
- **Parent** — optional parent block (for nesting)
- **Children** — any number of child blocks (indented below)
- **Marker** — task state (TODO, DOING, DONE) or priority (A, B, C)
- **Properties** — key-value metadata
- **Collapse state** — collapsed or expanded

## Creating Blocks

Just start typing. Every new line is a new block. Hit **Enter** to create a sibling block below the current one.

## Indenting and Outdenting

| Action | Shortcut |
|--------|----------|
| Indent (make child) | `Tab` |
| Outdent (make sibling) | `Shift + Tab` |

Indenting makes a block a child of the block above it. This creates hierarchical outlines:

```markdown
- Main topic
  - Subtopic 1
    - Detail A
    - Detail B
  - Subtopic 2
- Another topic
```

## Moving Blocks

You can rearrange blocks by dragging the drag handle on the left side of each block.

<!-- SCREENSHOT: [block-drag] Showing the drag handle and reorder animation -->

## Collapsing Blocks

Click the collapse arrow ▶ next to a block with children to collapse/expand the subtree. This helps manage large documents.

## Block Properties

Properties are key-value pairs attached to a block. In the editor, they appear as a properties menu. In the source file, they're serialized as `.property_name: value` lines:

```markdown
- Meeting notes
  .date: 2026-06-22
  .attendees: Alice, Bob
```

## Block Markers

Markers indicate task state or priority:

| Marker | Meaning |
|--------|---------|
| `TODO` | Task not started |
| `DOING` | Task in progress |
| `DONE` | Task completed |
| `A` / `B` / `C` | Priority level |

To apply a marker, type it at the start of a block (e.g., `TODO Review PR`).

## Auto-Save

The editor auto-saves changes after a brief pause. A status indicator shows save state:

- **Saving...** — changes pending
- **Saved** — all changes persisted

## Editor Features

- **Rich text** — bold, italic, underline, code, strikethrough
- **Headings** — H1–H3 via the formatting toolbar
- **Code blocks** — fenced code with syntax highlighting
- **Block quotes** — quoted text blocks
- **Dividers** — horizontal rules
- **Links** — external URLs and internal `[[wiki-links]]`
- **Images** — embedded images (displayed inline)

## Wiki-Links and Tags in the Editor

Type `[[` to trigger wiki-link autocomplete. Type `#` followed by a tag name for tag autocomplete. Both render as clickable elements inline.

<!-- SCREENSHOT: [wiki-link-autocomplete] Autocomplete dropdown in the editor -->

## Ctrl+Click Navigation

Hold `Ctrl` (or `Cmd` on macOS) and click a `[[wiki-link]]` to navigate directly to that page. Ctrl+click a `#tag` to search for all pages with that tag.
