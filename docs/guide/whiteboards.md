# Whiteboards

Whiteboards give you a spatial canvas for free-form ideation, diagrams and visual
thinking. Boards are stored as `.excalidraw` files in your vault, so they stay plain
JSON, diff cleanly in Git, and open in Excalidraw if you ever want the web app.

<!-- SCREENSHOT: [whiteboard-canvas] Native whiteboard with shapes, text and a frame -->

## Opening Whiteboards

Open **:material-draw: Whiteboards** from the sidebar, or navigate to `/whiteboards`.
The gallery lists every board in `whiteboards/` with its preview, element count and
last-saved state.

## Creating a Whiteboard

1. Open the **Whiteboards** panel.
2. Enter a name in the field at the top.
3. Click **Create** — the board opens for editing.
4. Boards auto-save about a second after you stop editing; the header shows
   **Saving…** and then **Saved**.

Boards live at:

```
your-vault/
└── whiteboards/
    └── my-board.excalidraw
```

## Canvas tools

| Tool | Function |
|------|----------|
| Select | Select, marquee-select, move and resize elements |
| Hand | Pan the canvas (or hold Space / drag with the middle button) |
| Rectangle | Draw rectangles and squares |
| Diamond | Draw diamonds |
| Ellipse | Draw circles and ellipses |
| Arrow | Draw arrows; ends can be bound to shapes |
| Line | Draw lines and polylines |
| Draw | Free-hand drawing |
| Text | Add text labels (IME input supported) |
| Eraser | Delete elements under the stroke |
| Frame | Draw a named frame that groups and clips its children |

Snapping to the grid and to other elements' edges can be toggled in the toolbar; hold
`Alt` to place an element freely. Zoom with the wheel (0.1×–30×) and use the toolbar
zoom controls to fit the board.

### Editing shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl`/`Cmd` + `Z` / `Shift` + `Ctrl`/`Cmd` + `Z` | Undo / redo |
| `Ctrl`/`Cmd` + `A` | Select all |
| `Ctrl`/`Cmd` + `C` / `V` | Copy / paste the selection |
| `Ctrl`/`Cmd` + `D` | Duplicate the selection |
| `Ctrl`/`Cmd` + `G` / `Shift` + `G` | Group / ungroup |
| `Delete` / `Backspace` | Delete the selection |
| Arrow keys | Nudge the selection |
| `Ctrl`/`Cmd` + `]` / `[` | Bring to front / send to back |
| `Esc` | Cancel the current action or deselect |

The style panel edits stroke colour, background colour, fill style, stroke width and
style, roughness, opacity and font size for the selection, and aligns or distributes
multiple selected elements.

## Frames and links

Frames are the only user-named element, so they double as link targets. With a frame or
element selected, **Copy link** puts `Board#Frame` (or `Board#Element`) on the clipboard;
opening `/whiteboards?board=<board>&element=<frame>` focuses that frame. Frame children
are clipped to the frame and move with it.

## Whiteboard library

Save reusable shapes and groups to your personal library:

1. Select the elements on the canvas.
2. Use **Save selection** in the library drawer.
3. Insert any item into another board from the drawer.

Library data is stored in `whiteboards/library.excalidrawlib`; additional
`.excalidrawlib` files placed in the same folder are loaded automatically, so exported
libraries from Excalidraw work without conversion.

## Export

**Export** writes an open-format SVG (vector, editable in any tool) or a PNG (2× scale)
to `whiteboards/exports/<board>.svg` / `.png`. Exports keep the board's own colours and
background; previews in the gallery adapt to your theme instead.

## Compatibility

Boards written by Excalidraw open unchanged, including unknown fields, embedded images,
bindings, groups, layers and library items. Untouched boards are never rewritten — the
file only changes when you edit it, and every edit keeps the unknown fields intact.

## Tips

- Use whiteboards for architecture diagrams, mind maps and project planning.
- Combine frames with `Board#Frame` links from your notes.
- Board JSON diffs cleanly in Git, so whiteboard changes sync like any other file.
