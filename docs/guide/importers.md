# Importers

Stratum can import notes from other PKM tools in one click — from an
**Obsidian**, **Logseq**, or a generic **Markdown** folder. The import is
designed to be safe and reversible: it only ever **adds** files, never modifies
or deletes anything, and never touches your source vault.

<!-- SCREENSHOT: [import-dialog] The one-click import dialog -->

## Getting Started

1. Open the **Import notes** button from the Pages home screen.
2. **Browse…** to select the folder you want to import from.
3. Optionally choose the **Format**. The default is **Auto-detect**, which
   inspects the folder's structure:
   - A `.obsidian/` directory → **Obsidian**
   - Top-level `pages` **and** `journals` directories → **Logseq**
   - Anything else → **Generic Markdown**
4. Click **Preview** to see exactly what would be imported.
5. Click **Import** to copy the notes into your vault.

## Where Files Land

All imported content is placed under a single subdirectory in your vault:

```
imported/<source-name>/
```

For example, importing an Obsidian vault named `my-vault` creates files under
`imported/my-vault/`. Attachments are copied to `imported/<source-name>/assets/`.
You can override the destination subdirectory in the import options.

## Preview & Dry-run

The **Preview** step never writes anything. It shows:

- The number of pages and assets that would be imported,
- Any **duplicates** — pages that already exist in your vault with identical
  content (these are skipped),
- Any **conflicts** — destination files that already exist with **different**
  content (these are skipped and never overwritten).

## Duplicates & Conflicts

Import is **idempotent**. Re-importing the same source folder creates no
duplicate notes:

| Status        | Meaning                                                                  | Action        |
|---------------|--------------------------------------------------------------------------|---------------|
| New           | The destination file does not exist.                                     | Imported      |
| Duplicate     | The destination already has byte-identical content.                      | Skipped       |
| Conflict      | The destination exists with **different** content.                       | Skipped, listed |

Imported notes are never merged or edited — only new files are written, so
conflicts are never destructive.

## Undo

Because import only ever adds new files under `imported/<name>/`, nothing is
modified or deleted. To undo an import, simply delete that folder. Your source
folder is never modified by the import.

## Per-Format Conversion

Each format is converted to Stratum's block-based Markdown format on import:

| Aspect             | Obsidian                              | Logseq                               | Generic        |
|--------------------|---------------------------------------|--------------------------------------|----------------|
| Frontmatter        | Preserved (title, tags, aliases, extras) | Preserved; inline `#tag`s and `tags::`/`file::` values folded into frontmatter tags | Copied verbatim |
| Wiki-links `[[…]]` | `[[path/to/Note]]` resolved to the `Note` slug | Kept as-is                            | Kept as-is     |
| Embeds `![[…]]`    | Rewritten to `imported/<name>/assets/…` | Rewritten to the assets path          | —              |
| Block refs `((…))` | Kept as-is                              | Kept as-is                             | —              |
| Callouts `> [!…]`  | Converted to a block with a `callout` property | —                               | —              |
| Task markers       | `- [ ]` → TODO, `- [x]` → DONE          | `- TODO`/`- NOW`/… → matching marker  | Kept as-is     |
| Logseq properties  | —                                       | `key:: value` → `.key: value`         | —              |
| Hierarchy          | Preserved via indentation                | Preserved via indentation             | Copied verbatim |

The Generic importer copies `.md` files **byte-for-byte** with no
transformation, which is the safe fallback for any folder.
