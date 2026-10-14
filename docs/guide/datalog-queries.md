# Datalog Queries

Stratum includes a Datalog query engine that compiles Datalog patterns into SQL queries against the block database.

<!-- SCREENSHOT: [datalog-query-panel] Query panel with a running example and results table -->

## Opening the Query Panel

Click **Query** in the sidebar or navigate to `/query`.

The panel has two tabs:

- **Builder** (default) — a visual filter builder: pick fields, operators and values; the generated Datalog is shown live and can be copied into the editor or run directly.
- **Datalog** — the raw EDN editor for hand-written queries.

Both tabs share the same query state (Run, Save Query, Reset and the Saved Queries list work identically).

## Visual Filter Builder

The Builder tab lets you compose a query without writing EDN:

1. Click **Add condition** to append a filter row.
2. Each row has a:
   - **Combiner** — how the row joins the previous one (`AND` / `OR`). The first row is always AND.
   - **Field** — the attribute to filter on (whitelisted, see table below).
   - **Operator** — the comparison for that field (options depend on the field).
   - **Value** — text/number/date input (hidden for "is set"/"is not set").
3. Rows can be reordered with the up/down arrows and removed with ×.
4. The **Generated query** preview shows the exact EDN that will be run. Copy it to the editor with the copy button, or press **Run** to execute it immediately.

### Fields

| Field | Attribute | Value type |
|-------|-----------|------------|
| Block text | `:block/content` | text |
| Marker | `:block/marker` | marker (TODO, DOING, DONE) |
| Priority | `:block/priority` | priority (A, B, C) |
| Page title | `:page/title` | text |
| Page path | `:page/path` | text |
| Block count | `:page/block_count` | number |
| Created | `:block/created` | date |
| Modified | `:block/modified` | date |
| Tags | `:block/tags` | tag |
| Has wiki-link | `:page/links` | page link |

### Operators

| Operator | EDN generated | Meaning |
|----------|---------------|---------|
| equals | `[?b :block/marker "X"]` | equality |
| does not equal | `[(not= ?v "X")]` | inequality |
| contains | `[(clojure.string/includes? ?v "X")]` | substring |
| starts with | `[(clojure.string/starts-with? ?v "X")]` | prefix |
| `>` `<` `≥` `≤` | `[(> ?v "X")]` … | numeric/date comparison |
| is set | `[(present? ?v)]` | value exists |
| is not set | `[(absent? ?v)]` | value is NULL |
| has (tags / wiki-link) | `[?b :block/tags "x"]` | containment |
| does not have (tags / wiki-link) | `(not [?b :block/tags "x"])` | negation |

### Combiners and validity

- Consecutive **OR** rows form a single `(or ...)` group; every OR group needs at least two rows. A lone OR row is reported as invalid and Run is disabled until fixed.
- Every condition that requires a value must have one; otherwise the builder shows an inline error and disables Run.
- "Does not have" (negation) cannot be nested inside an OR group — the builder emits it as a top-level `(not ...)` clause, so a group containing such a row is split.

### Round-trip

When you switch from the Datalog tab back to the Builder, Stratum attempts to parse the editor content into builder rows. Queries the builder can understand (its own output or hand-written equivalents using whitelisted fields/operators) are converted; anything else — `pull` find specs, unknown attributes, JSON query syntax — shows a notice and keeps your current builder rows unchanged.

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

### Extended where-clause grammar

In addition to plain triples, `:where` accepts Logseq-compatible extended forms:

- **Predicate lists** — `[(<op> <arg>...)]`, e.g. `[(not= ?m "DONE")]`, `[(clojure.string/includes? ?c "foo")]`, `[(> ?n "5")]`, `[(present? ?m)]`, `[(absent? ?m)]`.
  Supported operators: `=` `!=` `not=` `>` `<` `>=` `<=` `contains?` `includes?` `clojure.string/includes?` `starts-with?` `clojure.string/starts-with?` `present?` `absent?`. The variable must be bound to a plain column by an earlier triple.
- **Negation** — `(not <triple-or-predicate>)`, e.g. `(not [?b :block/tags "x"])`.
- **Or-groups** — `(or <clause> <clause> ...)` with at least two clauses (triples or predicates), e.g. `(or [?b :block/marker "TODO"] [?b :block/marker "DOING"])`.
- **Ordering** — `:order-by [[?var :asc] ...]` to order results by bound variables.

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
                [?page :page/modified ?modified]
         :order-by [[?modified :desc]]}
```

### Find items with a marker other than DONE

```clojure
{:query [:find ?block ?content
         :where [?block :block/marker ?marker]
                [(not= ?marker "DONE")]
                [?block :block/content ?content]]}
```

### Or-groups and negation

```clojure
{:query [:find ?block ?content
         :where (or [?block :block/marker "TODO"]
                    [?block :block/marker "DOING"])
                [?block :block/content ?content]]}
```

```clojure
{:query [:find ?block ?content
         :where (not [?block :block/tags "project"])
                [?block :block/content ?content]]}
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
- The visual builder only exposes whitelisted fields; for anything else (e.g. `pull` find specs) use the Datalog tab directly
