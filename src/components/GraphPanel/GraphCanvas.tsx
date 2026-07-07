import { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import ForceGraph3D from 'react-force-graph-3d';
import * as THREE from 'three';
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
  graphRef: React.MutableRefObject<any>;
  refreshKey: number;
}

interface GraphStats {
  fps: number;
  frameTime: number;
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
  refreshKey,
}: GraphCanvasProps) {
  const navigate = useNavigate();

  const [graphStats, setGraphStats] = useState<GraphStats>({ fps: 0, frameTime: 0 });
  const fpsRef = useRef({ frameCount: 0, lastSampleTime: 0 });
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const r = fpsRef.current;
    r.lastSampleTime = performance.now();
    let rafId: number;
    const tick = (now: number) => {
      r.frameCount++;
      const elapsed = now - r.lastSampleTime;
      if (elapsed >= 500) {
        const fps = Math.round((r.frameCount / elapsed) * 1000);
        setGraphStats({ fps, frameTime: Math.round(elapsed / r.frameCount) });
        r.frameCount = 0;
        r.lastSampleTime = now;
      }
      rafId = requestAnimationFrame(tick);
    };
    rafId = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafId);
  }, []);

  // Sync canvas bitmap size with container dimensions
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    canvas.width = width;
    canvas.height = height;
  }, [width, height]);

  // Canvas 2D label overlay — projects node positions every frame
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const vec = new THREE.Vector3();
    let rafId: number;

    const render = () => {
      const fg = graphRef.current;
      if (!fg || !canvas) {
        rafId = requestAnimationFrame(render);
        return;
      }

      ctx.clearRect(0, 0, canvas.width, canvas.height);

      const camera = fg.camera();
      if (!camera) {
        rafId = requestAnimationFrame(render);
        return;
      }

      const fgNodes: GraphNode[] = fg.graphData().nodes || [];
      if (fgNodes.length === 0) {
        rafId = requestAnimationFrame(render);
        return;
      }

      const w = canvas.width;
      const h = canvas.height;

      ctx.font = 'bold 11px Inter, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';

      for (let i = 0; i < fgNodes.length; i++) {
        const n = fgNodes[i];
        if (n.x === undefined || n.y === undefined || n.z === undefined) continue;

        vec.set(n.x, n.y, n.z);

        vec.project(camera);

        // Skip nodes behind the camera
        if (vec.z > 1) continue;

        const sx = (vec.x * 0.5 + 0.5) * w;
        const sy = (-vec.y * 0.5 + 0.5) * h;

        // Skip off-screen nodes (with generous margin)
        if (sx < -50 || sx > w + 50 || sy < -50 || sy > h + 50) continue;

        ctx.fillStyle = textColor;
        ctx.fillText(n.title, sx, sy + 14);
      }

      rafId = requestAnimationFrame(render);
    };

    rafId = requestAnimationFrame(render);
    return () => cancelAnimationFrame(rafId);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [textColor]);

  const edgeCount = graphDataProp?.links?.length ?? 0;

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
            key={`fg3d-${refreshKey}`}
            ref={graphRef}
            graphData={graphDataProp}
            width={width}
            height={height}
            backgroundColor={bgColor}
            nodeThreeObject={(n: GraphNode) => {
              const radius = Math.min(5, 2.5 + (n.degree || 0) * 0.2);
              return new THREE.Mesh(
                new THREE.SphereGeometry(radius, 16, 16),
                new THREE.MeshBasicMaterial({ color: nodeColor(n) })
              );
            }}
            onNodeClick={(n: GraphNode) => handleNodeClick(n)}
            onNodeRightClick={(n: GraphNode) => handleNodeRightClick(n)}
            linkColor={() => textColor}
            linkDirectionalArrowLength={nodes.length > 500 ? 0 : 3.5}
            linkDirectionalArrowRelPos={nodes.length > 500 ? 0 : 1}
            linkWidth={0.5}
            linkOpacity={0.35}
            linkCurvature={graphSettings.link_curvature}
            d3AlphaDecay={graphSettings.alpha_decay}
            d3VelocityDecay={graphSettings.velocity_decay}
            enableNodeDrag={true}
            enableNavigationControls={true}
            showNavInfo={false}
          />
          <canvas
            ref={canvasRef}
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              height: '100%',
              pointerEvents: 'none',
              zIndex: 5,
            }}
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

      {/* Stats overlay — always visible */}
      <Box
        sx={{
          position: 'absolute',
          bottom: 8,
          right: 8,
          zIndex: 20,
          bgcolor: 'rgba(0,0,0,0.6)',
          borderRadius: 1,
          px: 1.5,
          py: 0.75,
          fontFamily: 'monospace',
          fontSize: '0.7rem',
          color: '#ccc',
          lineHeight: 1.6,
          pointerEvents: 'none',
          userSelect: 'none',
        }}
      >
        <div>FPS: {graphStats.fps}</div>
        <div>Frame: {graphStats.frameTime}ms</div>
        <div>Nodes: {nodes.length}</div>
        <div>Edges: {edgeCount}</div>
      </Box>
    </Box>
  );
}
