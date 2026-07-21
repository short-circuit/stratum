import { useState, useRef, useEffect, useCallback, useMemo, memo } from 'react';
import { useNavigate } from 'react-router-dom';
import ForceGraph3D from 'react-force-graph-3d';
import * as THREE from 'three';
import { CSS2DRenderer, CSS2DObject } from 'three/addons/renderers/CSS2DRenderer.js';
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

const GraphCanvas = memo(function GraphCanvas({
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
  const instancedMeshRef = useRef<THREE.InstancedMesh | null>(null);

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

  // CSS2DRenderer instance for GPU-composited labels — created once, lives for component lifetime
  const [css2dRenderer] = useState(() => new CSS2DRenderer());

  // Sync CSS2DRenderer size with container dimensions
  useEffect(() => {
    css2dRenderer.setSize(width, height);
  }, [css2dRenderer, width, height]);

  // Stable extraRenderers array — CSS2DRenderer is automatically layered above the WebGL canvas
  const extraRenderers = useMemo(() => [css2dRenderer], [css2dRenderer]);

  // Sync InstancedMesh positions with force simulation every frame
  useEffect(() => {
    const fg = graphRef.current;
    if (!fg) return;
    const updateDummy = new THREE.Object3D();
    let rafId: number;
    const sync = () => {
      rafId = requestAnimationFrame(sync);
      const im = instancedMeshRef.current;
      if (!im || !fg) return;
      const syncNodes: GraphNode[] = fg.graphData().nodes || [];
      const syncCount = Math.min(im.count, syncNodes.length);
      for (let i = 0; i < syncCount; i++) {
        const nn = syncNodes[i];
        const r = Math.min(5, 2.5 + (nn.degree || 0) * 0.2);
        updateDummy.position.set(nn.x ?? 0, nn.y ?? 0, nn.z ?? 0);
        updateDummy.scale.set(r, r, r);
        updateDummy.updateMatrix();
        im.setMatrixAt(i, updateDummy.matrix);
      }
      im.instanceMatrix.needsUpdate = true;
    };
    rafId = requestAnimationFrame(sync);
    return () => cancelAnimationFrame(rafId);
  }, []);

  // InstancedMesh — single draw call for all nodes
  useEffect(() => {
    const fg = graphRef.current;
    if (!fg) return;
    const scene = fg.scene();
    if (!scene) return;

    // Dispose previous mesh (if any)
    if (instancedMeshRef.current) {
      scene.remove(instancedMeshRef.current);
      instancedMeshRef.current.geometry.dispose();
      const prevMat = instancedMeshRef.current.material;
      if (!Array.isArray(prevMat)) prevMat.dispose();
      instancedMeshRef.current = null;
    }

    const fgNodes: GraphNode[] = (fg.graphData().nodes as GraphNode[]) || [];
    if (fgNodes.length === 0) return;

    const count = fgNodes.length;
    const geometry = new THREE.SphereGeometry(1, 16, 16);
    const material = new THREE.MeshBasicMaterial();
    const mesh = new THREE.InstancedMesh(geometry, material, count);
    mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    mesh.frustumCulled = false;

    const dummy = new THREE.Object3D();
    const color = new THREE.Color();
    for (let i = 0; i < count; i++) {
      const n = fgNodes[i];
      const radius = Math.min(5, 2.5 + (n.degree || 0) * 0.2);
      dummy.position.set(n.x ?? 0, n.y ?? 0, n.z ?? 0);
      dummy.scale.set(radius, radius, radius);
      dummy.updateMatrix();
      mesh.setMatrixAt(i, dummy.matrix);
      color.set(nodeColor(n));
      mesh.setColorAt(i, color);
    }
    mesh.instanceMatrix.needsUpdate = true;
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;

    scene.add(mesh);
    instancedMeshRef.current = mesh;

    return () => {
      if (instancedMeshRef.current) {
        scene.remove(instancedMeshRef.current);
        instancedMeshRef.current.geometry.dispose();
        const prevMat = instancedMeshRef.current.material;
        if (!Array.isArray(prevMat)) prevMat.dispose();
        instancedMeshRef.current = null;
      }
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nodes, graphRef]);

  const edgeCount = graphDataProp?.links?.length ?? 0;

  const nodeThreeObj = useCallback((n: GraphNode) => {
    // CSS2D label only — sphere rendered by InstancedMesh
    const group = new THREE.Group();
    const labelEl = document.createElement('div');
    labelEl.textContent = n.title;
    labelEl.style.color = textColor;
    labelEl.style.fontFamily = 'Inter, sans-serif';
    labelEl.style.fontSize = '11px';
    labelEl.style.fontWeight = 'bold';
    labelEl.style.textShadow = '0 1px 3px rgba(0,0,0,0.8), 0 0 6px rgba(0,0,0,0.5)';
    labelEl.style.pointerEvents = 'none';
    labelEl.style.whiteSpace = 'nowrap';
    labelEl.style.userSelect = 'none';
    const label = new CSS2DObject(labelEl);
    label.position.set(0, 0, 0);
    group.add(label);
    return group;
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [textColor]);

  const onNodeClickCb = useCallback((n: GraphNode) => handleNodeClick(n), [handleNodeClick]);

  const onNodeRightClickCb = useCallback((n: GraphNode) => handleNodeRightClick(n), [handleNodeRightClick]);

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
            nodeThreeObject={nodeThreeObj}
            onNodeClick={onNodeClickCb}
            onNodeRightClick={onNodeRightClickCb}
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
            extraRenderers={extraRenderers}
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
});

export default GraphCanvas;
