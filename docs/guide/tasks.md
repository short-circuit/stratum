# Tasks

Stratum supports block-level task markers for simple task management.

## Task Markers

Three task states are supported, typed directly at the start of a block:

| Marker | State | Example |
|--------|-------|---------|
| `TODO` | Not started | `TODO Review pull request` |
| `DOING` | In progress | `DOING Write documentation` |
| `DONE` | Completed | `DONE Fix memory leak` |

```markdown
TODO Set up CI/CD pipeline
DOING Implement authentication
DONE Refactor database layer
TODO Review API design
```

## Priority Markers

Blocks can also have priority markers: `A`, `B`, or `C`.

```markdown
A TODO Fix critical security vulnerability
B TODO Add input validation
C TODO Update README
```

## Finding Tasks

### Via Search

Open **Search** and search for `TODO`, `DOING`, or `DONE` to find all task blocks.

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
