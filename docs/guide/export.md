# Export

Stratum can export your vault to portable formats.

<!-- SCREENSHOT: [export-dialog] Export options in the interface -->

## Exporting

### From the Interface

The export actions are available in **Settings → Vault → Export** (desktop) and
under the **Vault** section on mobile. The sidebar **Export** button still
exports HTML to a temp folder (`/tmp/stratum-export`), and the CLI writes
`export.html` into the vault directory.

The Export section provides four actions, each with an output-path field, a
**Browse** button to choose the destination, and an **Open folder** button on
success:

- **Export HTML** — static HTML snapshot (optionally into a chosen directory)
- **Export JSON** — structured JSON data (optionally into a chosen directory)
- **Export page as PDF** — the currently open page (requires a page to be open)
- **Export vault as ZIP** — portable vault archive

### From the CLI

```bash
# Export to HTML (default)
stratum export html

# Export to JSON
stratum export json
```

PDF and ZIP export are available from the app UI only — not from the CLI.

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

## PDF Export

Exports the currently open page to a single PDF file named
`export-<slug>.pdf` in the vault directory.

- **Renderer**: Stratum shells out to a headless Chromium binary to print the
  page. This is a *runtime* dependency, not a bundled library.
- **Math**: LaTeX is pre-rendered to MathML (`$…$` and `$$…$$` both supported)
  so equations appear as real math, not raw markup.
- **Mermaid**: ` ```mermaid ` fenced blocks are rendered by the embedded
  Mermaid runtime.
- **Images**: `![[assets/…]]` embeds are resolved against the vault `assets/`
  directory and included in the output.

### Chromium runtime dependency

A Chromium-compatible binary must be installed and discoverable on the system.
Stratum looks for it, in order:

1. The `STRATUM_CHROMIUM` environment variable (explicit path)
2. `chromium`, `chromium-browser`, `google-chrome`, or `google-chrome-stable`
   on your `PATH`
3. `/usr/bin/chromium`, `/usr/bin/google-chrome`, `/usr/bin/google-chrome-stable`
4. A Chromium in Playwright's cache (`~/.cache/ms-playwright/`)

If none is found the export reports an error telling you to install Chromium or
set `STRATUM_CHROMIUM`.

## ZIP Export

Exports the whole vault to a portable `export.zip` archive for backup or
transfer.

- Includes all user content: plain `.md` files, `assets/`, `templates/`, and a
  `manifest.json` describing the archive.
- **Excludes** the `.pkm/` and `.git/` directories (rebuildable cache and
  version history) and any prior `export.html`/`export.json`/`export.zip`/
  `export.pdf` artifacts at the vault root.
- The archive is re-importable via the [importers](../guide/importers.md).

The `manifest.json` lists the format, version, export timestamp, page count,
and file count:

```json
{
  "format": "stratum-vault-zip",
  "version": "1",
  "exported_at": "2026-10-14T00:00:00Z",
  "page_count": 12,
  "file_count": 34
}
```

## Use Cases

- **JSON export** — import into other tools, build custom views, analyze your knowledge graph
- **HTML export** — share a readable snapshot of your vault with others
- **PDF export** — share a single note as a fixed-format document
- **ZIP export** — portable backup or vault transfer
- **Plain .md files** — the files themselves are the most portable format. Just copy them.
