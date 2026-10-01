# Graph View

The graph visualizes connections between your notes as an interactive force-directed network.

<!-- SCREENSHOT: [graph-view] Full graph view showing connected nodes -->

## Opening the Graph

Click **Graph** in the sidebar or navigate to `/graph`. The graph loads automatically with all pages and their `[[wiki-link]]` connections.

## Graph Elements

| Element | Description |
|---------|-------------|
| **Nodes** | Each page in your vault |
| **Edges** | `[[wiki-link]]` connections between pages |
| **Node size** | Scaled by degree (number of connections) |
| **Node color** | Colored by most prominent tag |
| **Tag nodes** | Optional tag nodes shown as diamonds (configurable) |

## Interacting with the Graph

| Action | Result |
|--------|--------|
| **Click a node** | Navigate to that page |
| **Drag a node** | Reposition manually |
| **Scroll** | Zoom in/out |
| **Pan** | Click and drag empty space |
| **Hover** | Highlight connections and show label |

<!-- SCREENSHOT: [graph-interaction] Hover highlight showing connected nodes -->

## Graph Views

### Connected Components

Toggle to show only nodes that are part of connected clusters. Isolates the "conversations" happening in your vault.

<!-- SCREENSHOT: [graph-components] Connected components view -->

### Orphaned Notes

Toggle to show only nodes with zero incoming or outgoing connections. These pages aren't linked to anything — great for finding content that needs integration.

<!-- SCREENSHOT: [graph-orphans] Orphaned notes view -->

### Filter by Search

Type in the search field within the graph panel to filter nodes by name. Only matching nodes and their immediate connections are shown.

## Graph Settings

| Setting | Description | Default |
|---------|-------------|---------|
| Show Connected | Show connected components only | `true` |
| Show Orphaned | Show orphaned notes | `true` |
| Show Tags | Display tag nodes in the graph | `true` |
| 3D View (mobile) | Render the graph in 3D instead of the 2D force layout (see [3D View on Mobile](#3d-view-on-mobile)) | `false` |
| Charge Strength | Node repulsion (more negative = more spread) | `-30` |
| Link Distance | Preferred edge length | `100` |
| Alpha Decay | How quickly the simulation stabilizes | `0.02` |
| Velocity Decay | Damping factor | `0.4` |
| Node cap | Maximum number of nodes rendered. `0` (Unlimited) renders the full vault below 2,000 notes and **automatically caps to 2,000** on larger vaults so the graph stays responsive. | `0` |

These settings can be adjusted in **Settings → Graph** or by editing `.pkm/config.toml`.

## 3D View on Mobile

On mobile, the graph starts in the **2D force layout** for performance. You can opt in to a **3D view** with the "3D view" toggle in the graph settings menu (bottom sheet). The 3D layout supports the same interactions: drag to pan, pinch to zoom, tap a node to open the page, and drag nodes to reposition them.

The toggle is only shown where it applies:

- **Mobile**: the "3D view" toggle is available in the settings bottom sheet.
- **Desktop**: the graph always renders in 3D; the toggle is not shown.

### Automatic fallback to 2D

Stratum checks hardware capability before enabling 3D. On devices that cannot render 3D (WebGL unavailable), or on very constrained hardware (≤2 GB RAM or ≤2 CPU cores), Stratum automatically falls back to the 2D layout and shows a notice explaining that 3D isn't supported on the device. Your preference is still saved — if you later open the graph on a capable device, 3D is used. The 3D view is disabled in the settings when the device can't render it.

## Performance

Graph loading and rendering are optimized for large vaults:

- **Authoritative links table.** Graph edges are read directly from the `links` table (the same index that powers backlinks), not re-parsed from block content on every load. Every save/edit reconciles this table in the same transaction, so the graph always reflects the latest edits. On first launch after upgrading, the app self-heals any vault whose links table predates this wiring.
- **Cached responses.** The computed graph is cached per vault and served cheaply on subsequent opens, invalidated automatically after any page or block edit.
- **Safe default node cap for large vaults.** With the **Node cap** set to `0` (Unlimited), the full vault is rendered below 2,000 notes; on larger vaults the rendered and simulated set is automatically capped to the 2,000 most-connected notes so the force simulation and rendering stay interactive. You can still reveal more by raising the cap in **Settings → Graph** — the most-connected notes are always kept first, so the visible structure remains meaningful.

## Tips

- **Start with orphans** — opening the orphan view first helps find pages that need linking
- **Larger nodes are hubs** — highly connected pages are the most important in your knowledge graph
- **Color reveals topics** — if all your nodes are the same color, you might need more diverse tagging
