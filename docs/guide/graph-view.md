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
| Charge Strength | Node repulsion (more negative = more spread) | `-30` |
| Link Distance | Preferred edge length | `100` |
| Alpha Decay | How quickly the simulation stabilizes | `0.02` |
| Velocity Decay | Damping factor | `0.4` |

These settings can be adjusted in **Settings → Graph** or by editing `.pkm/config.toml`.

## Tips

- **Start with orphans** — opening the orphan view first helps find pages that need linking
- **Larger nodes are hubs** — highly connected pages are the most important in your knowledge graph
- **Color reveals topics** — if all your nodes are the same color, you might need more diverse tagging
