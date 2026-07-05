# Kanban

Stratum includes a Kanban board for visual task management. Cards on the board are synced with task markers in your notes, so you can manage tasks from the board or your notes and they stay in sync.

<!-- SCREENSHOT: [kanban-board] Kanban board showing three columns with cards -->

## Opening Kanban

Click **:material-view-column: Kanban** in the sidebar or navigate to `/kanban`.

## Columns

The board starts with three default columns:

| Column | Purpose |
|--------|---------|
| **To Do** | Tasks not yet started |
| **In Progress** | Active work items |
| **Done** | Completed or cancelled items |

Cards are placed into columns based on their task marker:

| Marker | Column |
|--------|--------|
| `TODO` | To Do |
| `NOW` | To Do |
| `LATER` | To Do |
| `WAITING` | To Do |
| `DOING` | In Progress |
| `DONE` | Done |
| `CANCELLED` | Done |

```markdown
TODO Set up CI/CD pipeline    →  To Do column
DOING Implement auth          →  In Progress column
DONE Refactor database layer  →  Done column
CANCELLED Deprecated feature  →  Done column
```

## Adding Cards

1. Click **Add Card** at the top of any column
2. Type the task description
3. Press Enter to create the card

A new block is added to your daily note (or the page you specify) with the `TODO` marker.

## Moving Cards

Drag and drop a card from one column to another. The task marker updates automatically:

- Dropping on **To Do** sets marker to `TODO`
- Dropping on **In Progress** sets marker to `DOING`
- Dropping on **Done** sets marker to `DONE`

The underlying block in your note is updated to reflect the new marker.

## Editing Cards

Click on a card to open the edit dialog. You can change:

- **Content** - the task description
- **Marker** - override the marker manually (TODO, DOING, DONE, etc.)
- **Priority** - add or change the priority marker (A, B, C)

<!-- SCREENSHOT: [kanban-edit] Edit dialog for a Kanban card -->

## Context Menu

Right-click a card to open the context menu:

- **Edit** - opens the edit dialog
- **Delete** - removes the card and deletes the underlying block

## Viewing Source

Each card shows its source note path at the bottom. Click the path to open the note where the block lives. This makes it easy to add more context around a task.

<!-- SCREENSHOT: [kanban-source] Card showing source note path -->

## Integration with Tasks

Kanban is a visual frontend for the same task markers you use in your notes. Every card is a block with a marker. Changes on the board update the note, and changes in the note update the board on next load.

This means you can:

- Create tasks in your notes with `TODO` and see them appear on the board
- Drag cards on the board to change their status without opening the note
- Use the full power of your editor (tags, wiki-links, nested blocks) alongside Kanban

```markdown
- TODO Review pull request
- DOING Write documentation
- DONE Fix memory leak
- A TODO Fix critical security vulnerability
```

## Templates

You can create tasks in your notes that automatically appear in the correct column by using the right marker. This is useful for daily journal templates or project planning templates:

```markdown
## Today's Tasks

- TODO Morning standup prep
- DOING Sprint review docs

## Backlog

- LATER Investigate new framework
- WAITING Client feedback on design
```

Use Datalog to find all tasks and review them on the board. See the [Datalog Queries](datalog-queries.md) guide for task-related queries.

<!-- SCREENSHOT: [kanban-full] Full Kanban board with cards across all columns -->

## Tips

- Use the board for daily standups - drag cards to update status in real time
- Combine with the Journal panel for morning task planning
- Cards with `WAITING` stay in To Do until you're ready to start them
- Delete cards from the context menu to clean up stale tasks
- Click the source path on a card to add notes, tags, or sub-tasks
