# Mermaid Diagrams

Create diagrams inline in your notes using Mermaid syntax.

<!-- SCREENSHOT: [mermaid-diagram] A flowchart rendered inline in the editor -->

## Creating a Mermaid Diagram

Write a fenced code block with the language set to `mermaid`:

````markdown
```mermaid
graph TD
    A[Start] --> B{Is it working?}
    B -->|Yes| C[Great!]
    B -->|No| D[Debug]
    D --> B
```
````

The diagram renders automatically below the code block.

## Supported Diagram Types

### Flowchart

```mermaid
graph LR
    A[Note] --> B[Idea]
    B --> C{Connect?}
    C -->|Yes| D[[Wiki-link]]
    C -->|No| E[Tag it]
```

### Sequence Diagram

```mermaid
sequenceDiagram
    User->>Stratum: Write note
    Stratum->>Index: Index blocks
    Index->>Search: Update search
    User->>Search: Query
    Search->>User: Results
```

### Class Diagram

```mermaid
classDiagram
    class Block {
        +UUID id
        +String content
        +insert()
        +delete()
        +move()
    }
    class Page {
        +String path
        +Frontmatter frontmatter
    }
    Page "1" --> "*" Block
```

### Gantt Chart

```mermaid
gantt
    title Project Timeline
    dateFormat  YYYY-MM-DD
    section Research
    Literature review      :done, 2026-01-01, 30d
    section Development
    Core implementation    :active, 2026-02-01, 60d
    Testing                :2026-04-01, 30d
```

### State Diagram

```mermaid
stateDiagram-v2
    [*] --> Draft
    Draft --> Review
    Review --> Published
    Review --> Draft
    Published --> Archived
```

### Pie Chart

```mermaid
pie title Note Distribution
    "Projects" : 40
    "Journal" : 25
    "Reference" : 20
    "Archive" : 15
```

## Editing Diagrams

- **Click the code** section to edit the Mermaid source
- **Click the diagram** to view it (toggle between code and diagram view)
- The diagram auto-renders as you type
- Use the grab cursor to pan within large diagrams

## Diagram Settings

Mermaid respects your theme setting:

- **Light mode** — default theme
- **Dark mode** — dark theme (auto-detected)

## Tips

- Use `graph TD` for top-down flowcharts, `graph LR` for left-right
- Sequence diagrams are great for documenting workflows and processes
- Gantt charts work well for project planning directly in your knowledge base
- Combine diagrams with wiki-links for deep documentation
