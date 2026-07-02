# CLI Reference

The `stratum` CLI provides terminal access to vault operations.

## Installation

```bash
# Build from source
cargo build -p pkm-cli

# The binary is at:
./target/debug/stratum
```

## Global Options

| Flag | Description |
|------|-------------|
| `-p, --vault <PATH>` | Vault path (default: current directory) |

## Commands

### `init`

Initialize a new vault in the specified directory.

```bash
stratum init
stratum -p ~/my-vault init
```

Creates:
- Vault directory structure
- `.pkm/config.toml` with defaults
- `notes/welcome.md` — a getting-started note

### `list`

List all notes in the vault.

```bash
stratum list

# Filter by tag
stratum list --tag project
```

### `show`

Display a note's content and metadata.

```bash
stratum show notes/my-page.md
```

Shows:
- Title
- Path
- Tags
- Wiki-links
- Body content

### `create`

Create a new note.

```bash
stratum create notes/new-page.md
stratum create notes/new-page.md --title "My New Page"
```

Creates a `.md` file with frontmatter (title, creation date, tags).

### `search`

Search note contents.

```bash
stratum search "search query"
```

Returns matching notes with context snippets.

### `stats`

Show vault statistics.

```bash
stratum stats
```

Displays:
- Total notes
- Total tags
- Total wiki-links
- Total file size
- Vault path

### `graph`

Show the knowledge graph structure.

```bash
stratum graph
```

Displays:
- Node and edge counts
- Orphaned notes (no connections)
- Up to 20 edge samples

### `tags`

Show the tag cloud.

```bash
stratum tags
```

Displays tags sorted by frequency with a visual bar chart.

### `sync`

Git sync operations.

```bash
# Check sync status
stratum sync status

# Push changes
stratum sync push

# Pull changes
stratum sync pull

# Full sync
stratum sync sync
```

### `export`

Export the vault.

```bash
# Export to HTML (default)
stratum export

# Export to JSON
stratum export json
```

### `ask`

Ask an AI question about your notes.

```bash
stratum ask "What are my notes about project X?"
```

Requires a configured LLM provider in `.pkm/config.toml`.

### `config`

Display the current configuration.

```bash
stratum config
```

Shows `.pkm/config.toml` contents.

### `help`

Display help information.

```bash
stratum --help
stratum <command> --help
```
