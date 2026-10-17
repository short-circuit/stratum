# Vault Management

## What is a Vault?

A vault is a directory of Markdown files that Stratum manages. All your notes live here as plain `.md` files — readable and editable by any text editor. Stratum adds a hidden `.pkm/` directory for metadata cache.

<!-- SCREENSHOT: [vault-picker] The welcome screen showing vault selection -->

## Creating a Vault

On first launch, Stratum shows the **Vault Picker** screen. Click **Choose Vault Folder** and select an empty directory or a new folder.

You can also create a vault from the command line:

```bash
# Create a new vault in the current directory
stratum init

# Or specify a path
stratum -p ~/my-vault init
```

## Opening a Vault

Stratum remembers your last used vault. You can change vaults via **Settings → Vault**:

1. Click **:material-cog: Settings** in the sidebar
2. Go to the **Vault** tab
3. Click **Browse** and select a new vault directory
4. Stratum reloads with the new vault

## Vault Structure

```
your-vault/
├── note-a.md                    # Your notes
├── note-b.md
├── journals/                    # Auto-created daily notes
│   └── 2026-06-22.md
├── whiteboards/                 # Excalidraw whiteboard data
│   └── my-board.excalidraw
├── templates/                   # Reusable page templates
│   └── meeting-notes.md
└── .pkm/                        # Cache directory (rebuildable)
    ├── blocks.db                # SQLite block storage
    ├── search.idx               # Tantivy full-text search index
    └── config.toml              # Configuration file
```

!!! tip "The `.pkm/` cache is fully rebuildable"
    You can safely delete the `.pkm/` directory at any time. Stratum rebuilds it from your `.md` files. No data loss — just a brief reindexing delay.

!!! note "Stale cache self-heals"
    If a page or the journal ever shows stale or empty content (for example after an external edit or a git pull while the app was closed), close and reopen the page or journal — current content is now loaded from your `.md` files automatically, no app restart needed. If you want a forced full rebuild, use **Settings → Developer → Repair DB from disk** or **Reindex All**; the page list refreshes immediately after either one.

## Vault Information

The sidebar header shows your vault stats:

- **Block count** — total number of blocks across all pages
- **Page count** — total number of `.md` files

<!-- SCREENSHOT: [sidebar-vault-info] Sidebar header showing block and page counts -->

## Multiple Vaults

Stratum works with one vault at a time, but you can switch between vaults at any time through Settings. Each vault is completely independent with its own configuration, cache, and Git repository.

## Page Storage Layout

Stratum stores pages under a fixed `pages/` directory inside the vault. The `pages/` prefix is *not* part of a page's title — it is only the storage location:

```
your-vault/
├── pages/                       # All your knowledge pages
│   ├── note-a.md
│   ├── projects/
│   │   ├── project-x.md
│   │   └── archives/
│   │       └── old-note.md
├── journals/                    # Auto-created daily notes
└── .pkm/                        # Cache directory (rebuildable)
```

### Page Path Syntax

Every page is identified by a **vault-relative path** of the form `pages/<subfolder>/<note-name>.md`. Nested folders are allowed and created automatically when you create a page with a slash in its path.

| Part | Description | Example |
|------|-------------|---------|
| `pages/` | Required storage prefix — always present | — |
| `<subfolder>/` | Optional folders (table of contents by topic) | `projects/` |
| `<note-name>` | Base name — becomes the slug | `project-x` |
| `.md` | Required extension — appended if omitted | `page.md` |

### Page List Views

The page list (sidebar **Recent** section and the **Home** page) can be displayed in two layouts. Choose the one you prefer in **Settings → Vault → Pages layout**:

- **Flat list** — every page appears as a flat list, sorted by last modified. This is the default.
- **Folder tree** — pages are grouped into a collapsible folder tree based on their path under `pages/`. Folders are shown first (alphabetical), then pages (alphabetical). Pages in `journals/` are never shown here — use the **Journal** panel for those.

The setting is stored per-vault in `.pkm/config.toml` under `[layout]`, as `pages_view = "flat"` or `pages_view = "tree"`. Changing it takes effect immediately; no reindex is needed.

!!! note "The tree is presentation-only"
    The tree view groups whatever paths you create — it does not enforce that a note "belongs" to a folder. Any valid page exists in both views, and both views operate on exactly the same page set. Both layouts coexist: pages created in tree mode can still be opened and edited in flat mode and vice versa.

### Creating Pages in Subfolders

Use the **New page** form and type a full path:

```
projects/quarterly-review
```

Stratum creates `pages/projects/quarterly-review.md` and the `projects/` folder automatically. The title is optional; if omitted, the page uses the base name as its title.

!!! tip "Dead-link creation stays flat"
    Creating a page by clicking a dead `[[wiki-link]]` still creates it at `pages/<name>.md` (flat), not inside the current folder. This avoids surprise path splits — folder placement remains a deliberate choice.

## Moving and Renaming Pages

Use **Move / rename…** from:

- the page-row hover actions in the sidebar page list (the move icon), or
- the page **⋯ actions menu** in the top-right of an open page (desktop).

The dialog asks for the new vault-relative path, e.g. `pages/projects/review-2026.md`. It validates that:

- the path is non-empty and starts with `pages/`,
- no other page already exists at the target path,
- the target path is inside the vault (no traversal).

On move, Stratum:

1. Renames the file on disk (creating parent folders as needed) and updates its database rows and search index,
2. **Rewrites every inbound wiki-link** across the whole vault — including in sources you did not ask to touch — to the new path or slug,
3. Refreshes the page list, graph, and backlinks.

Both the page row and the open-page menu run the same backend `move_page` command.

!!! warning "Moving across devices"
    If the source and destination are on different filesystems (e.g. a symlinked subfolder pointing to another drive), the rename fails with "source and destination are on different filesystems". Move the folder on the same filesystem instead; a copy+delete fallback is planned.

### What Move Rewrites

`move_page` rewrites links that refer to the moved page under any of its old identities — but only where the destination identity actually changed:

| Identity | Rewritten? | Example |
|----------|-----------|---------|
| Path links (`[[old/path]]`) | ✅ yes → new path | `[[pages/old/note]]` → `[[pages/new/note]]` |
| Slug links (`[[old-slug]]`) | ✅ if slug changed, → new slug | `[[old-slug]]` → `[[new-slug]]` |
| Title links (`[[Old Title]]`) | ➖ unchanged (title is preserved verbatim on move) | `[[Old Title]]` stays `[[Old Title]]` |
| Block references `((uuid))` | ➖ never rewritten (UUID-based) | stays |

Cases, piped/display forms (`[[old|display]]`), and escaped pipes are handled by the parser. Links in YAML frontmatter are rewritten too. If neither the slug nor the path changed (moving between folders with the same filename), nothing downstream is rewritten — link integrity is already preserved.

### Name vs. Path Identity

This is the key link-integrity model:

- **Name links** (`[[Some Note]]`) resolve by *note name* (slug/title), NOT by folder. You can move a note between folders and every `[[Some Note]]` link keeps working without any rewrite.
- **Path links** (`[[pages/folder/note]]`) resolve by *exact path* — they break if the note is moved.

Consequence: **moving a note inside the vault never breaks name links.** Path links are the only ones that can break, and `move_page` rewrites them for you. Duplicate note names are allowed (in different folders); when the name is ambiguous, both the autocomplete and any resolution treat **path-qualified** targets as authoritative — use `[[pages/folder/note]]` to disambiguate.

### External Moves / Renames

If you rename or move a `.md` file **outside** Stratum (file manager, terminal, Obsidian, git pull):

1. Stratum's file watcher picks up the change and re-indexes.
2. The old page row disappears — no stale "ghost" page remains.
3. **Name links** stay intact (name identity). **Path links** to the old path dangle until you either update them manually or use **Move / rename…** in the UI — which rewrites them for you.

!!! tip "Prefer the in-app move"
    For renames, use **Move / rename…** instead of an external `mv`: it is the only path that rewrites inbound links automatically. For folder-only moves (filename unchanged), an external `mv` is safe — name links are not affected.

## Moving a Vault

To move a vault:

1. Close Stratum
2. Move the entire vault directory (including `.pkm/`) to the new location
3. Open Stratum, go to **Settings → Vault**, and update the vault path
4. Stratum re-indexes and reloads

All your notes and metadata are preserved. (Moving a vault directory ≠ moving pages — page paths are always vault-relative, so they do not change.)
