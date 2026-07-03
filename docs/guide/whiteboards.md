# Whiteboards

Whiteboards provide a spatial canvas for free-form ideation, diagrams, and visual thinking — powered by Excalidraw.

<!-- SCREENSHOT: [whiteboard-canvas] Excalidraw whiteboard with shapes, text, and drawings -->

## Opening Whiteboards

Click **:material-draw: Whiteboards** in the sidebar or navigate to `/whiteboards`.

## Creating a Whiteboard

1. Open the **Whiteboards** panel
2. Enter a name in the text field
3. Click **Create**
4. The canvas opens for editing

## Canvas Tools

The Excalidraw toolbar provides:

| Tool | Function |
|------|----------|
| Selection | Select, move, resize elements |
| Rectangle | Draw rectangles and squares |
| Diamond | Draw diamond shapes |
| Ellipse | Draw circles and ellipses |
| Arrow | Draw arrows and connectors |
| Line | Draw lines |
| Text | Add text labels |
| Image | Add images to the canvas |
| Hand | Pan the canvas |
| Laser | Highlight temporarily |

<!-- SCREENSHOT: [whiteboard-tools] Excalidraw toolbar showing all tools -->

## Saving

Whiteboards auto-save as you work. Each whiteboard is stored in your vault at:

```
your-vault/
└── whiteboards/
    └── my-board.excalidraw
```

The data is JSON-based Excalidraw format, versioned alongside your notes via Git.

## Whiteboard Library

You can save shapes and elements to a personal library for reuse across whiteboards:

1. Select elements on the canvas
2. Click **Add to Library**
3. Access library items in any whiteboard

Library data is stored at `.pkm/library.excalidrawlib`.

## Tips

- Use whiteboards for architecture diagrams, mind maps, project planning
- Whiteboards complement wiki-links for visual thinkers
- Save library items for commonly used shapes and icons
- Whiteboard files are plain JSON — they diff cleanly in Git
