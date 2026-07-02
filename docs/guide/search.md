# Search

Stratum uses Tantivy, a Rust full-text search engine, for fast local search. Queries return results in under 100ms even with 10,000+ notes.

## Opening Search

Click **Search** in the sidebar or navigate to `/search`.

<!-- SCREENSHOT: [search-panel] Search panel with query and results -->

## Basic Search

Type your query and press Enter or click **Search**:

```text
machine learning transformer
```

Results show:
- **Content** — the matched text with highlights
- **Page** — the page containing the match
- **Score** — relevance ranking

Click any result to navigate to that page.

## Tag Search

Prefix your query with `#` to search by tag:

```text
#project
#rust
#meeting
```

This finds all blocks tagged with the given tag.

## Search Results

Each result shows:

- **Snippet** — the matched content with keyword highlighting
- **Page path** — which page contains the match
- **Score** — search relevance score (higher = more relevant)

## Rebuilding the Index

If you've made manual changes to your `.md` files outside Stratum, you can rebuild the search index:

1. Open **Search**
2. Click **Rebuild Index**
3. The index is rebuilt from all blocks in the vault

<!-- SCREENSHOT: [search-rebuild] Rebuild index button with completion message -->

You can also rebuild from **Settings → Developer → Reindex All**.

## Index Storage

The search index is stored in `.pkm/search.idx`. It's fully rebuildable from your blocks — no data loss if deleted.

## Tips

- **Search is block-level**, not just page-level — you'll find specific paragraphs
- **Prefix `#`** for tag search (much faster than general search)
- **Rebuild the index** if you've made large file imports or moved your vault
