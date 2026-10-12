# Attachments

Stratum supports attaching images and arbitrary files to your notes. Attachments are stored
as plain files in the `assets/` directory of your vault — no binary data is ever put inside
the SQLite database. The `.md` note simply references the asset by a stable, human-readable
path, which keeps your vault fully portable, sync-friendly (Git), and editable by any tool.

<!-- SCREENSHOT: [attachment-panel] Attachment panel listing files on a note -->

## How attachments work

- Attached files live in `<vault>/assets/`. This directory is automatically copied during
  HTML export, and is excluded from note-link indexing.
- A note references an asset with one of two wiki-link forms:
  - `![[assets/photo.png]]` — **inline embed** (renders the image inside the editor)
  - `[[assets/report.pdf]]` — **file link** (renders as a clickable attachment)

Both forms are plain markdown, so they survive Git sync, export, and round-trip through the
block parser/serializer unchanged.

## Attaching a file

1. Open a note.
2. In the attachment panel (below the editor), click **Attach**.
3. Select one or more files from your system.
4. Stratum imports each file into `assets/` with a deterministic name and inserts an
   `![[assets/...]]` (image) or `[[assets/...]]` (file) reference into the note.

You can also **paste** an image directly into the editor (Ctrl/Cmd+V), or **drag & drop**
an image or file onto the editor — BlockNote turns it into an embedded image or attachment
card automatically.

## Deterministic naming & deduplication

Imported assets are named `assets/<stem>-<sha256-prefix>.<ext>`. Because the name embeds a
hash of the file content, importing the same file twice always produces the same reference
and never overwrites an existing file. Attaching two different files with the same stem
results in distinct, collision-free names.

## Viewing & managing attachments

- The attachment panel on each note lists every asset referenced by that note, with its
  MIME type and size.
- Use the delete icon to remove an asset.
- Attachments are **not** notes: `![[assets/...]]` references do not appear in the graph
  or in backlinks as pages. They are tracked separately in the block store's `links` table
  with type `asset_ref`.

## Size limit & compression

- Images larger than **8 MB** are re-encoded client-side (downscaled to at most
  2048 px and converted to JPEG/WebP) before import, keeping the vault lean.
- Non-image files (e.g. PDFs) above the 8 MB cap are rejected with a
  "File larger than 8 MB" error.

## Export

The HTML export already copies the `assets/` directory next to the exported notes, and
`![[assets/...]]` references in exported HTML point to those copied files, so a full export
remains self-contained.

## File format

The reference in the `.md` file is exactly what you see in your text editor:

```markdown
- A diagram I made: ![[assets/diagram-a1b2c3d4e5f6.png]]
- The full report: [[assets/report-9f8e7d6c5b4a.pdf]]
```

No special encoding, no binary blobs, no database-only state — just plain files and plain
markdown.
