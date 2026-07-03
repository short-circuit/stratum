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

## Vault Information

The sidebar header shows your vault stats:

- **Block count** — total number of blocks across all pages
- **Page count** — total number of `.md` files

<!-- SCREENSHOT: [sidebar-vault-info] Sidebar header showing block and page counts -->

## Multiple Vaults

Stratum works with one vault at a time, but you can switch between vaults at any time through Settings. Each vault is completely independent with its own configuration, cache, and Git repository.

## Moving a Vault

To move a vault:

1. Close Stratum
2. Move the entire vault directory (including `.pkm/`) to the new location
3. Open Stratum, go to **Settings → Vault**, and update the vault path
4. Stratum re-indexes and reloads

All your notes and metadata are preserved.
