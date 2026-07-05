import { useNavigate } from 'react-router-dom';
import ForceGraph3D from 'react-force-graph-3d';
import SpriteText from 'three-spritetext';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import type { GraphNodeDto, GraphDataDto, GraphSettings } from '../../lib/types';

export interface GraphNode extends GraphNodeDto {
  x?: number;
  y?: number;
  z?: number;
}

interface GraphDataProp {
  nodes: GraphNode[];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
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

export const DEFAULT_SETTINGS: GraphSettings = {
  show_connected: true,
  show_orphaned: true,
  show_tags: true,
  charge_strength: -8,
  link_distance: 40,
  alpha_decay: 0.08,
  velocity_decay: 0.3,
  link_curvature: 0.15,
};

interface GraphCanvasProps {
  graphDataProp: GraphDataProp;
  width: number;
  height: number;
  bgColor: string;
  textColor: string;
  handleNodeClick: (node: GraphNode) => void;
  handleNodeRightClick: (node: GraphNode) => void;
  loading: boolean;
  error: string | null;
  nodes: GraphNode[];
  graphData: GraphDataDto | null;
  graphSettings: GraphSettings;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  graphRef: React.MutableRefObject<any>;
}

export default function GraphCanvas({
  graphDataProp,
  width,
  height,
  bgColor,
  textColor,
  handleNodeClick,
  handleNodeRightClick,
  loading,
  error,
  nodes,
  graphData,
  graphSettings,
  graphRef,
}: GraphCanvasProps) {
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
          <ForceGraph3D
            ref={graphRef}
            graphData={graphDataProp}
            width={width}
            height={height}
            backgroundColor={bgColor}
            nodeThreeObject={(n: GraphNode) => {
              const sprite = new SpriteText(n.title);
              sprite.textHeight = Math.min(11, 5 + (n.degree || 0) * 0.35);
              sprite.color = textColor;
              sprite.backgroundColor = nodeColor(n) + '44';
              sprite.padding = [3, 7];
              sprite.fontWeight = 'bold';
              sprite.borderRadius = 6;
              return sprite;
            }}
            onNodeClick={(n: GraphNode) => handleNodeClick(n)}
            onNodeRightClick={(n: GraphNode) => handleNodeRightClick(n)}
            linkColor={() => textColor}
            linkDirectionalArrowLength={3.5}
            linkDirectionalArrowRelPos={1}
            linkWidth={0.5}
            linkOpacity={0.35}
            linkCurvature={graphSettings.link_curvature}
            d3AlphaDecay={graphSettings.alpha_decay}
            d3VelocityDecay={graphSettings.velocity_decay}
            enableNodeDrag={true}
            enableNavigationControls={true}
            showNavInfo={false}
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
