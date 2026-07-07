import { useNavigate } from 'react-router-dom';
import ForceGraph2D from 'react-force-graph-2d';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import type { GraphNodeDto, GraphDataDto, GraphSettings } from '../../lib/types';

export interface GraphNode extends GraphNodeDto {
  x?: number;
  y?: number;
}

interface GraphDataProp {
  nodes: GraphNode[];
  links: any[];
}

// Vibrant palette optimised for dark backgrounds — high saturation, 65-75% lightness
const NODE_PALETTE = [
  '#fbbf24', // amber
  '#60a5fa', // blue
  '#34d399', // emerald
  '#f472b6', // pink
  '#a78bfa', // violet
  '#fb923c', // orange
  '#2dd4bf', // teal
  '#e879f9', // fuchsia
];

// Visible warm colour for untagged or low-degree nodes (pops against #1a1a2e)
const UNTAGGED_COLOR = '#d4a574';

function nodeColor(n: GraphNode): string {
  const fromTags = n.tags.length > 0;
  if (!fromTags) return UNTAGGED_COLOR;
  // Pick palette entry by tag hash so the colour is deterministic per tag
  const idx = n.tags[0].split('').reduce((acc, c) => acc + c.charCodeAt(0), 0) % NODE_PALETTE.length;
  return NODE_PALETTE[idx];
}

// eslint-disable-next-line react-refresh/only-export-components
export const DEFAULT_SETTINGS: GraphSettings = {
  show_connected: true,
  show_orphaned: true,
  show_tags: true,
  charge_strength: -4,
  link_distance: 40,
  alpha_decay: 0.15,
  velocity_decay: 0.4,
  link_curvature: 0.15,
  node_cap: 0,
};

interface GraphCanvas2DProps {
  graphDataProp: GraphDataProp;
  width: number;
  height: number;
  textColor: string;
  handleNodeClick: (node: GraphNode) => void;
  loading: boolean;
  error: string | null;
  nodes: GraphNode[];
  graphData: GraphDataDto | null;
  graphSettings: GraphSettings;
  graphRef: React.MutableRefObject<any>;
}

export default function GraphCanvas2D({
  graphDataProp,
  width,
  height,
  textColor,
  handleNodeClick,
  loading,
  error,
  nodes,
  graphData,
  graphSettings,
  graphRef,
}: GraphCanvas2DProps) {
  const navigate = useNavigate();

  return (
    <Box sx={{ flex: 1, position: 'relative', bgcolor: 'background.default' }}>
      {loading && (
        <Box sx={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', bgcolor: 'rgba(0,0,0,0.03)', zIndex: 10 }}>
          <CircularProgress size={20} sx={{ mr: 1 }} />
          <Typography variant="body2" color="text.secondary">Building graph...</Typography>
        </Box>
      )}

      {error && (
        <Alert severity="error" sx={{ position: 'absolute', top: 16, left: '50%', transform: 'translateX(-50%)', zIndex: 10 }}>
          {error}
        </Alert>
      )}

      {nodes.length > 0 && (
        <>
          {graphData && graphData.node_count > 0 && graphData.edge_count === 0 && (
            <Alert severity="warning" icon={false} sx={{ position: 'absolute', top: 16, left: '50%', transform: 'translateX(-50%)', zIndex: 10, boxShadow: 3 }}>
              <Typography variant="caption">
                Pages found but no wiki-links yet. Add <Box component="code" sx={{ bgcolor: 'warning.light', px: 0.5, borderRadius: 0.5, fontSize: '0.7rem' }}>[[Page Name]]</Box> in your notes to create connections.
              </Typography>
            </Alert>
          )}
          <ForceGraph2D
            ref={graphRef}
            graphData={graphDataProp}
            width={width}
            height={height}
            nodeColor={(n) => nodeColor(n as GraphNode)}
            nodeCanvasObjectMode={() => 'replace'}
            nodeCanvasObject={(n, ctx, globalScale) => {
              const node = n as GraphNode;
              const label = node.title;
              const fontSize = Math.min(11 / globalScale, 5 + (node.degree || 0) * 0.35 / globalScale);
              const baseSize = Math.max(2, Math.min(4, 1.5 + (node.degree || 0) * 0.12));
              const r = baseSize;

              ctx.font = `bold ${fontSize}px Inter, sans-serif`;
              const textWidth = ctx.measureText(label).width;
              const padding = 3 / globalScale;
              const boxW = textWidth + padding * 2;
              const boxH = fontSize * 1.4;
              const boxX = node.x!;
              const boxY = node.y!;

              // Draw node circle
              ctx.beginPath();
              ctx.arc(boxX, boxY - boxH / 2, r, 0, 2 * Math.PI);
              ctx.fillStyle = nodeColor(node);
              ctx.fill();

              // Fade label when zooming out: invisible below 0.4×, full at 0.8× and above
              const labelAlpha = Math.min(1, Math.max(0, (globalScale - 0.4) / (0.8 - 0.4)));

              if (labelAlpha > 0.01) {
                // Draw label background
                const bgAlpha = Math.min(1, 2 * globalScale) * labelAlpha;
                ctx.fillStyle = nodeColor(node) + Math.round(bgAlpha * 68).toString(16).padStart(2, '0');
                const lx = boxX - boxW / 2;
                const ly = boxY - boxH / 2;
                ctx.beginPath();
                ctx.roundRect(lx, ly, boxW, boxH, [4 / globalScale]);
                ctx.fill();

                // Draw label text
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';
                ctx.globalAlpha = labelAlpha;
                ctx.fillStyle = textColor;
                ctx.fillText(label, boxX, boxY);
                ctx.globalAlpha = 1;
              }
            }}
            onNodeClick={(n) => handleNodeClick(n as GraphNode)}
            linkColor={() => textColor}
            linkDirectionalArrowLength={nodes.length > 500 ? 0 : 3.5}
            linkDirectionalArrowRelPos={nodes.length > 500 ? 0 : 1}
            linkWidth={0.5}
            linkCurvature={graphSettings.link_curvature}
            d3AlphaDecay={graphSettings.alpha_decay}
            d3VelocityDecay={graphSettings.velocity_decay}
            enableNodeDrag={true}
            minZoom={0.5}
            maxZoom={4}
          />
        </>)}
      {!loading && !error && nodes.length === 0 && (
        <Box sx={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <Box sx={{ textAlign: 'center', maxWidth: 320 }}>
            <Typography variant="body2" sx={{ fontWeight: 500, mb: 0.5 }}>No graph data yet</Typography>
            {graphData ? (
              <>
                <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
                  Vault at <Box component="code" sx={{ bgcolor: 'grey.200', px: 0.5, borderRadius: 0.5 }}>{graphData.vault_path}</Box> has
                  {' '}0 .md files. Create pages first, then add{' '}
                  <Box component="code" sx={{ bgcolor: 'grey.200', px: 0.5, borderRadius: 0.5 }}>[[wiki-links]]</Box>{' '}
                  between them to build the graph.
                </Typography>
              </>
            ) : (
              <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
                Could not load graph data. Check that your vault has .md files with wiki-links.
              </Typography>
            )}
            <Button variant="outlined" size="small" onClick={() => navigate('/')}>
              Go to Pages
            </Button>
          </Box>
        </Box>
      )}
    </Box>
  );
}
