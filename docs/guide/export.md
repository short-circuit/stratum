# Export

Stratum can export your vault to portable formats.

<!-- SCREENSHOT: [export-dialog] Export options in the interface -->

## Exporting

### From the Interface

1. Click the **Export** button in the sidebar
2. The vault exports to HTML format in `/tmp/stratum-export/`

### From the CLI

```bash
# Export to HTML (default)
stratum export html

# Export to JSON
stratum export json
```

## HTML Export

The HTML export generates a single-page document with all notes:

- Page titles as headings
- Full content preserved
- Basic styling for readability
- All notes in one document

Output: `export.html` in your vault directory.

## JSON Export

The JSON export creates structured data suitable for programmatic processing:

```json
[
  {
    "path": "notes/my-page.md",
    "title": "My Page",
    "tags": ["project", "documentation"],
    "body": "Content...",
    "links": ["other-page", "reference"]
  }
]
```

Output: `export.json` in your vault directory.

Each entry contains:

| Field | Description |
|-------|-------------|
| `path` | Vault-relative file path |
| `title` | Page title from frontmatter |
| `tags` | Array of frontmatter tags |
| `body` | Page content (without frontmatter) |
| `links` | Array of wiki-link targets |

## Use Cases

- **JSON export** — import into other tools, build custom views, analyze your knowledge graph
- **HTML export** — share a readable snapshot of your vault with others
- **Plain .md files** — the files themselves are the most portable format. Just copy them.
