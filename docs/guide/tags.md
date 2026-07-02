# Tags

Tags provide a flat categorization system that cuts across your page hierarchy.

## Creating Tags

Tags can appear in two places:

### Inline Tags

Type `#tagname` anywhere in a block:

```markdown
This is a #project note about #rust development.

#meeting Notes from the team sync.
```

### Frontmatter Tags

Add tags to a page's YAML frontmatter:

```markdown
---
title: My Page
tags: [project, rust, documentation]
---
```

## Tag Search

Search for all blocks tagged with a specific tag:

1. Open **Search** from the sidebar
2. Prefix your query with `#` (e.g., `#project`)
3. All blocks with that tag are displayed

<!-- SCREENSHOT: [tag-search] Search panel showing results for #project -->

Tags can also be clicked directly in the editor — Ctrl+click a `#tag` to auto-search.

## Tag Aggregation

The system tracks tag usage across all pages. This powers:

- **Tag cloud** — see which tags are most used
- **Auto-complete** — tag names autocomplete as you type `#`
- **Tag coloring** in the graph view — nodes are colored by their most prominent tags

## Tag Naming Conventions

- Tags are case-insensitive: `#Project` and `#project` are the same
- Tags can include hyphens: `#machine-learning`
- Tags can include slashes for hierarchy: `#project/active`
- Tags cannot contain spaces

## Tags vs. Pages

| Aspect | Tags | Pages |
|--------|------|-------|
| Structure | Flat, no hierarchy in indexing | Hierarchical via nesting |
| Navigation | Search-based | Sidebar list |
| Linking | Not directly linkable | `[[Wiki-link]]` |
| Graph | Tag nodes (optional) | Page nodes |
