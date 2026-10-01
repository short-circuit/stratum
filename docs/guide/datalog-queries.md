# Datalog Queries

Stratum includes a Datalog query engine that compiles Datalog patterns into SQL queries against the block database.

<!-- SCREENSHOT: [datalog-query-panel] Query panel with a running example and results table -->

## Opening the Query Panel

Click **Query** in the sidebar or navigate to `/query`.

## Query Syntax

Queries use Datalog syntax in EDN format:

```clojure
{:query [:find ?variable1 ?variable2
         :where [?entity :attribute value]
                [?entity :attribute ?variable]]}
```

### Components

| Element | Description |
|---------|-------------|
| `:find` | Variables to return (prefixed with `?`) |
| `:where` | Patterns that must match (entity-attribute-value triples) |
| `?entity` | A variable representing a block or page |
| `:attribute` | A block attribute (see table below) |
| `value` | A literal value or variable |

## Block Attributes

The following attributes are available for querying:

| Attribute | Type | Description |
|-----------|------|-------------|
| `:block/id` | UUID | Block unique identifier |
| `:block/content` | String | Block text content |
| `:block/marker` | String | Task marker (TODO, DOING, DONE) |
| `:block/priority` | String | Priority (A, B, C) |
| `:block/parent` | UUID | Parent block ID |
| `:block/page` | String | Containing page path |
| `:block/heading` | Number | Heading level (1-3) |
| `:block/collapsed` | Boolean | Is collapsed |
| `:block/properties` | Map | Key-value properties |
| `:block/tags` | Set | Block-level tags |
| `:page/title` | String | Page title from frontmatter |
| `:page/path` | String | Page file path |
| `:page/tags` | Set | Page frontmatter tags |
| `:page/block_count` | Number | Number of blocks |
| `:page/links` | Set | Wiki-link targets |
| `:page/backlinks` | Set | Incoming wiki-links |

## Examples

### Find all TODO items

```clojure
{:query [:find ?block ?content
         :where [?block :block/marker "TODO"]
                [?block :block/content ?content]]}
```

### Find high-priority tasks

```clojure
{:query [:find ?block ?content ?priority
         :where [?block :block/marker "TODO"]
                [?block :block/priority ?priority]
                [?block :block/content ?content]]}
```

### Find blocks by tag

```clojure
{:query [:find ?block ?content
         :where [?block :block/tags "project"]
                [?block :block/content ?content]]}
```

### Find all pages with their block counts

```clojure
{:query [:find ?page ?title ?count
         :where [?page :page/title ?title]
                [?page :page/block_count ?count]]}
```

### Find recently modified pages

```clojure
{:query [:find ?page ?title ?modified
         :where [?page :page/title ?title]
                [?page :page/modified ?modified]]
         :order-by [[?modified :desc]]}
```

## Query Results

Results are displayed in a table with:

- **Columns** — labeled by your `:find` variables
- **Rows** — each matching combination of values
- Click a page path to navigate to that page

## Saved Queries

You can save the current query under a name and reload it later. Saved queries are stored in `.pkm/saved_queries.json` inside your vault and sync with the vault, so they are available on every device where the vault is synced.

### Saving the current query

With the query you want to keep in the editor:

1. Click **Save Query** (desktop) or **Save** (mobile) above the editor.
2. Enter a name in the dialog and click **Save**.

The name must not be empty. Leading and trailing whitespace is trimmed and names are limited to 256 characters.

Saving under a name that already exists **overwrites** the previous version of that saved query — it does not create a duplicate.

### Viewing saved queries

The **Saved Queries** list below the results shows every saved query with its name and the last-updated date. Use the refresh button to reload the list.

### Loading a saved query

Click a saved query in the list to load its query text into the editor. The query is **not executed automatically** — click **Run Query** (or **Run**) to run it.

### Renaming a saved query

Use the rename (pencil) icon next to a saved query, enter the new name, and confirm. Renaming to a name that already exists fails and keeps the original name.

### Deleting a saved query

Use the delete (trash) icon next to a saved query, then confirm in the dialog. Deletion cannot be undone.

### Error handling and limitations

- A missing or corrupt `saved_queries.json` is treated as an empty list; the next save rewrites a valid file.
- Writes are atomic, so a crash or power loss cannot leave a half-written file behind.
- Errors loading the list are shown in an error banner; errors during save, rename, or delete are shown in a warning banner.
- Saving an empty query is allowed — the store does not validate the query syntax.

## Tips

- Start with the example queries by clicking **Reset**
- Use `?block` as your first variable to identify blocks
- Combine markers and tags for powerful filtering: `?block :block/marker "TODO"` + `?block :block/tags "project"`
- Results are live — run queries after editing to see changes
