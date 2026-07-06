# Tasks

Stratum supports block-level task markers for simple task management.

## Task Markers

Seven task states are supported, typed directly at the start of a block:

| Marker | State | Example |
|--------|-------|---------|
| `TODO` | Not started | `TODO Review pull request` |
| `DOING` | In progress | `DOING Write documentation` |
| `DONE` | Completed | `DONE Fix memory leak` |
| `NOW` | Active focus | `NOW Draft project proposal` |
| `LATER` | Backlog | `LATER Research database options` |
| `WAITING` | Blocked | `WAITING Get feedback from design` |
| `CANCELLED` | Abandoned | `CANCELLED Old feature experiment` |

```markdown
TODO Set up CI/CD pipeline
DOING Implement authentication
DONE Refactor database layer
NOW Deploy staging environment
LATER Research migration options
WAITING Review API contract
CANCELLED Experimental v1 feature
```

## Priority Markers

Blocks can also have priority markers: `A`, `B`, or `C`.

```markdown
A TODO Fix critical security vulnerability
B TODO Add input validation
C TODO Update README
```

## Interactive Features

### Autocomplete

Typing a marker prefix (like `TOD` or `DON`) triggers an autocomplete popup with all available marker and priority options. Select one from the menu to insert it at the cursor. Priority markers `A`, `B`, and `C` are also available here.

<!-- SCREENSHOT: [marker-autocomplete] Autocomplete popup showing marker and priority options -->

### Inline Badge Chips

Markers render as colored badge chips directly in the editor:

- **TODO** — gray outline chip
- **DOING** — blue chip
- **DONE** — green chip with strikethrough
- **NOW** — cyan chip
- **LATER** — gray chip
- **WAITING** — amber chip
- **CANCELLED** — red chip with strikethrough

Priority markers also appear as inline chips:

- **A** — red chip
- **B** — amber chip
- **C** — blue chip

A block can show both a marker chip and a priority chip side by side, like `A TODO` or `C LATER`.

<!-- SCREENSHOT: [marker-badge-chips] Colored badge chips for markers and priorities in the editor -->

### Toggling

Click any marker badge to clear it from the block. This gives you a quick way to advance a task — click the `DONE` badge off, then type a new marker, or click `DOING` off when you are ready to move on.

## Finding Tasks

### Via Search

Open **Search** and search for any marker name (`TODO`, `DOING`, `NOW`, `LATER`, etc.) to find all task blocks with that status.

### Via Datalog Queries

The most powerful way to manage tasks is through Datalog queries:

```clojure
{:query [:find ?block ?content
         :where [?block :block/marker "TODO"]
                [?block :block/content ?content]]}
```

<!-- SCREENSHOT: [datalog-task-query] Datalog query results showing TODO tasks -->

See the [Datalog Queries](datalog-queries.md) guide for more examples.

### Pre-built Queries

The **Query** panel has a reset button that provides example queries. Modify them to suit your workflow.

## Task Workflow Tips

- Use `DOING` to track work-in-progress — limits multitasking
- Run a Datalog query before standups for a quick status report
- Combine tags and tasks: `#project/alpha TODO Ship MVP` — then search `#project/alpha` to find all tasks for that project
