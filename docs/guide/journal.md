# Journal / Daily Notes

The Journal provides daily notes — automatically created pages for each day.

<!-- SCREENSHOT: [journal-panel] Journal panel showing the current day's note -->

## Opening the Journal

Click **:material-calendar-month: Journal** in the sidebar or navigate to `/journal`.

## How it Works

- Each day gets its own page at `journals/YYYY-MM-DD.md`
- The journal auto-opens to today's date
- Yesterday's journal is still accessible via the page list
- Journal pages are normal `.md` files — they appear in search, graph, and backlinks

## Creating Journal Entries

Journal pages are just block editors like any other page. Start typing to write:

```markdown
# Standup

TODO Review PR #42
DOING Documentation rewrite
DONE Database migration

## Notes

Investigated the indexing performance issue.
Root cause: Tantivy segment merge scheduling.
See [[Indexing Performance]] for details.
```

## Journal Workflow

### Morning Pages

Use the journal for daily standups, morning pages, or free writing. Wiki-link to project pages for context:

```markdown
Worked on [[Block Editor]] improvements.
Need to fix the drag-and-drop reordering for deeply nested blocks.
```

### Meeting Notes

Create a journal entry during meetings, then link to detailed notes:

```markdown
Met with team about [[Sprint 24 Planning]].
Key decisions: use [[Tantivy 0.22]] for search indexing.
```

### Capture and Process

Use the journal to quickly capture thoughts, then process them into proper pages:

1. Write freely in the journal
2. Later, extract sections into dedicated pages
3. Link back from the new page to the journal entry

## Journal File Format

Journal pages follow the same format as all other pages — plain Markdown with optional frontmatter:

```markdown
---
title: 2026-06-22
tags: [journal]
---

# 2026-06-22

Content...
```

## Tips

- Use tags like `#standup` or `#meeting` in journal entries for easy searching
- The journal is great for temporary notes that don't need their own page yet
- Old journals are automatically indexed and searchable
