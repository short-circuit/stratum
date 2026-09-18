# Links and Backlinks

Stratum supports several types of links to connect your knowledge graph.

## Wiki-Links

Wiki-links connect pages together. They format as `[[Page Name]]` or `[[Page Name|Display Text]]`.

### Creating Wiki-Links

1. Type `[[` in the editor — autocomplete appears
2. Start typing the target page name
3. Select a page from the dropdown
4. Or type a full page name and close with `]]`

<!-- SCREENSHOT: [wiki-link-autocomplete] Autocomplete dropdown in the editor -->

### Syntax

```markdown
# Basic link to another page
[[Target Page]]

# Link with custom display text
[[Target Page|Display Text]]

# Link to a page by its file path
[[notes/my-page]]

# Inline in text
See [[Related Concept]] for more details.
```

### Navigating

- **Ctrl+click** (or Cmd+click) a wiki-link to navigate to the target page
- Links render with an underline and accent color
- Hover shows a tooltip preview

<!-- SCREENSHOT: [link-preview] Hover preview tooltip for a wiki-link -->

## Block References

You can reference specific blocks by their UUID using `((uuid))` syntax:

```markdown
As discussed in ((65f8a1e2-3a4b-4c5d-8e6f-7a8b9c0d1e2f)), the approach is sound.
```

### Viewing Block References

When a block has incoming references, a count badge appears next to it. Click the badge to see references.

## Page Embeds

Embed an entire page's content inline using `{{embed [[Page Name]]}}`:

```markdown
## Project Status

{{embed [[Current Status]]}}
```

The embedded content renders as a nested card within the current page.

## Backlinks Panel

Every page has a **Backlinks** panel at the bottom. It shows:

- **Linked references** — pages that explicitly `[[link]]` to this page
- **Unlinked mentions** — pages that mention the page title but don't have a formal wiki-link

<!-- SCREENSHOT: [backlinks-panel] Backlinks panel showing linked and unlinked references -->

Each backlink shows:
- The source page title
- A snippet of context around the link/mention
- A click to navigate to the source

### Backlink Snippet API

The backend exposes a snippet endpoint that returns the exact backlinked block
plus a bounded window of surrounding context from the source note, with note
title, note id, and anchor id. This powers the hover preview and any surface
that needs to display the note content around a backlink anchor.

```text
GET /api/notes/:id/backlink-snippet?ref=<backlink-ref>
```

- `:id` — vault-relative path of the note containing the anchor
- `ref` — the block id (UUID) of the backlinked anchor

Response payload:

```
{
  "note_id":        "pages/project-x.md",
  "note_title":     "Project X",
  "anchor_id":      "8f3e…uuid…",
  "anchor_content": "See [[Project X]] for the launch timeline.",
  "context":        ["Preceding paragraph", "See [[Project X]] for the launch timeline.", "Follow-up paragraph"]
}
```

A missing note or missing anchor returns a 404-style error (`Note not found` /
`Block not found`). The `context` array carries the anchor block itself plus up
to two adjacent blocks on each side, in document order, so the UI can render
the backlinked portion with surrounding context without shipping the whole
note.

## Suggested Connections

The **Suggested Connections** panel (below backlinks) uses AI to find pages that are semantically related but not yet linked:

1. Click **Find** to scan for suggestions
2. Review suggested pages with similarity scores
3. Click **+ Link** to automatically insert a `[[wiki-link]]` on the current page

<!-- SCREENSHOT: [suggested-connections] Suggested connections panel with results -->

## Unlinked Mentions

Stratum detects when the title of page B appears in page A without a wiki-link. These appear as **unlinked mentions** in the backlinks panel. You can convert them to proper links directly from the panel.
