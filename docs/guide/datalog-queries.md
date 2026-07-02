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

## Tips

- Start with the example queries by clicking **Reset**
- Use `?block` as your first variable to identify blocks
- Combine markers and tags for powerful filtering: `?block :block/marker "TODO"` + `?block :block/tags "project"`
- Results are live — run queries after editing to see changes
