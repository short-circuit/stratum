import { useState, useCallback, useRef, useEffect, useMemo, memo } from 'react';
import { useNavigate } from 'react-router-dom';
import ForceGraph3D from 'react-force-graph-3d';
import SpriteText from 'three-spritetext';
import { forceCollide } from 'd3-force-3d';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Alert from '@mui/material/Alert';
import type { GraphDataDto } from '../../lib/types';

export interface GraphNode {
  id: string; title: string; path: string; degree: number; tags: string[];
  x?: number; y?: number; z?: number; fx?: number; fy?: number; fz?: number;
}
interface GraphDataProp { nodes: GraphNode[]; links: { source: string; target: string }[]; }

const PALETTE = ['#fbbf24','#60a5fa','#34d399','#f472b6','#a78bfa','#fb923c','#2dd4bf','#e879f9'];
function nodeColor(n: any): string {
  if (!n) return '#ffffff';
  const key = n.tags?.length > 0 ? n.tags[0] : n.id || String(n);
  return PALETTE[key.split('').reduce((a: number, c: string) => a + c.charCodeAt(0), 0) % PALETTE.length];
}

export const DEFAULT_SETTINGS = {
  show_connected: true, show_orphaned: true, show_tags: true,
  charge_strength: -120, link_distance: 50, alpha_decay: 0.15,
  velocity_decay: 0.4, link_curvature: 0.15, node_cap: 0,
};

interface Props {
  graphDataProp: GraphDataProp;
  width: number; height: number; bgColor: string; textColor: string;
  handleNodeClick: (n: GraphNode) => void;
  handleNodeRightClick?: (n: GraphNode) => void;
  loading: boolean; error: string | null;
  nodes: GraphNode[]; graphData: GraphDataDto | null;
  graphSettings: typeof DEFAULT_SETTINGS;
  graphRef: React.MutableRefObject<any>;
  layoutReady?: boolean;
}

const GraphCanvas = memo(function GraphCanvas({
  graphDataProp, width, height, bgColor, textColor,
  handleNodeClick, loading, error, nodes, graphData, graphSettings, graphRef, layoutReady,
}: Props) {
  const navigate = useNavigate();

  // ── Highlight state ──────────────────────────────────────────────
  const [hlRaw, setHlRaw] = useState<any>(null); // hovered node
  const hlNode = hlRaw as any;
  const tweenRef = useRef<number | null>(null);

  // Cancel any ongoing camera lerp
  const cancelLerp = useCallback(() => {
    if (tweenRef.current !== null) {
      cancelAnimationFrame(tweenRef.current);
      tweenRef.current = null;
    }
  }, []);

  // Enrich data: cross-link node objects + seed 3D positions given by the hash
  const enriched = useMemo(() => {
    if (!graphDataProp.nodes.length) return null;
    const d = {
      nodes: graphDataProp.nodes.map((n: any) => ({ ...n })),
      links: graphDataProp.links.map((l: any) => ({ ...l })),
    };
    // Seed deterministic positions from node ID to ensure initial 3D spread.
    // Seed deterministic positions from node ID to ensure initial 3D spread.
    d.nodes.forEach((n: any) => {
      if (n.z === undefined || n.z === 0) {
        const zh = (n.id || '').split('').reduce((a: number, c: string) => a + c.charCodeAt(0), 0);
        n.z = ((zh * 7 + 13) % 120) - 60;
      }
      if (n.x === undefined) {
        const xh = (n.id || '').split('').reduce((a: number, c: string) => a + c.charCodeAt(0) * 3, 0);
        n.x = ((xh * 11 + 7) % 120) - 60;
      }
      if (n.y === undefined) {
        const yh = (n.id || '').split('').reduce((a: number, c: string) => a + c.charCodeAt(0) * 7, 0);
        n.y = ((yh * 13 + 3) % 120) - 60;
      }
    });
    const byId = new Map(d.nodes.map((n: any) => [n.id, n]));
    d.nodes.forEach((n: any) => { n.neighbors = []; n.links = []; });
    d.links.forEach((l: any) => {
      const s = byId.get(l.source), t = byId.get(l.target);
      if (s && t) { s.neighbors.push(t); t.neighbors.push(s); s.links.push(l); t.links.push(l); }
    });
    return d;
  }, [graphDataProp]);

  const hlNodes = useMemo(() => {
    if (!hlNode || !enriched) return new Set();
    const s = new Set([hlNode]);
    (hlNode.neighbors || []).forEach((n: any) => s.add(n));
    return s;
  }, [hlNode, enriched]);
  const hlLinks = useMemo(() => {
    if (!hlNode || !enriched) return new Set();
    const s = new Set();
    (hlNode.links || []).forEach((l: any) => s.add(l));
    return s;
  }, [hlNode, enriched]);

  // ── Node appearance ──────────────────────────────────────────────
  const colorAccessor = useCallback((n: any) => {
    if (!hlNode) return nodeColor(n);
    return hlNodes.has(n) ? nodeColor(n) : 'rgba(200,200,200,0.12)';
  }, [hlNode, hlNodes]);

  const nodeThreeObj = useCallback((n: any) => {
    const sprite = new SpriteText(n.title || n.id);
    sprite.material.depthWrite = false;
    sprite.color = textColor || '#ffffff';
    sprite.textHeight = 6;
    sprite.center.y = -0.5;
    return sprite;
  }, [textColor]);

  const linkColorAccessor = useCallback(() => textColor, [textColor]);
  const linkWidthAcc = useCallback((l: any) => {
    if (!hlNode) return 0.5;
    return hlLinks.has(l) ? 2 : 0.1;
  }, [hlNode, hlLinks]);

  // ── Interaction handlers ─────────────────────────────────────────
  const onClick = useCallback((n: any) => handleNodeClick(n), [handleNodeClick]);

  const onRightClick = useCallback((n: any) => {
    if (!graphRef.current || n.x === undefined || n.y === undefined) return;
    cancelLerp();
    const cam = graphRef.current.camera();
    const ctrl = (graphRef.current as any).controls();
    const tgt = { x: n.x, y: n.y, z: n.z || 0 };
    const lookAt = ctrl?.target || { x: 0, y: 0, z: 0 };
    const dest = {
      x: cam.position.x + tgt.x - lookAt.x,
      y: cam.position.y + tgt.y - lookAt.y,
      z: cam.position.z + tgt.z - lookAt.z,
    };
    const startPos = { x: cam.position.x, y: cam.position.y, z: cam.position.z };
    const startTarget = { x: lookAt.x, y: lookAt.y, z: lookAt.z };
    const duration = 500;
    const startTime = performance.now();
    const tick = (now: number) => {
      const t = Math.min((now - startTime) / duration, 1);
      const e = 1 - Math.pow(1 - t, 3);
      cam.position.set(
        startPos.x + (dest.x - startPos.x) * e,
        startPos.y + (dest.y - startPos.y) * e,
        startPos.z + (dest.z - startPos.z) * e,
      );
      if (ctrl?.target) {
        ctrl.target.set(
          startTarget.x + (tgt.x - startTarget.x) * e,
          startTarget.y + (tgt.y - startTarget.y) * e,
          startTarget.z + (tgt.z - startTarget.z) * e,
        );
      }
      if (t < 1) {
        tweenRef.current = requestAnimationFrame(tick);
      } else {
        tweenRef.current = null;
      }
    };
    tweenRef.current = requestAnimationFrame(tick);
  }, [graphRef, cancelLerp]);

  const onHover = useCallback((n: any) => {
    setHlRaw(n || null);
  }, []);

  const onDragEnd = useCallback((n: any) => {
    n.fx = n.x;
    n.fy = n.y;
    n.fz = n.z;
  }, []);

  // ── Force config ─────────────────────────────────────────────────
  useEffect(() => {
    const fg = graphRef.current;
    if (!fg || !enriched) return;
    try {
      const charge = fg.d3Force('charge');
      if (charge) charge.strength(graphSettings.charge_strength);
      const link = fg.d3Force('link');
      if (link) link.distance(graphSettings.link_distance);
      fg.d3Force('collide', forceCollide(fg.nodeRelSize()));
      // zSpring force preserves initial 3D spread during simulation
      const zTargets = new Map<string, number>();
      enriched.nodes.forEach((n: any) => zTargets.set(n.id, n.z || 0));
      fg.d3Force('zSpring', () => {
        fg.graphData().nodes.forEach((n: any) => {
          const tz = zTargets.get(n.id);
          if (tz !== undefined && n.z !== undefined) n.vz += (tz - n.z) * 0.08;
        });
      });
      fg.d3ReheatSimulation();
    } catch {}
  }, [enriched, graphSettings.charge_strength, graphSettings.link_distance, graphRef]);

  // Listen for controls interaction start → cancel camera lerp
  useEffect(() => {
    const fg = graphRef.current;
    if (!fg) return;
    let ctrl: any;
    try { ctrl = fg.controls(); } catch {}
    if (!ctrl || !ctrl.addEventListener) return;
    const onStart = () => cancelLerp();
    ctrl.addEventListener('start', onStart);
    return () => {
      try { ctrl.removeEventListener('start', onStart); } catch {}
    };
  }, [graphRef, cancelLerp]);

  // ── Render ───────────────────────────────────────────────────────
  return (
    <Box sx={{ flex: 1, position: 'relative' }}>
      {loading && (
        <Box sx={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 10 }}>
          <Typography variant="body2" color="white">Building graph...</Typography>
        </Box>
      )}
      {error && <Alert severity="error" sx={{ position: 'absolute', top: 16, left: '50%', transform: 'translateX(-50%)', zIndex: 10 }}>{error}</Alert>}

      {nodes.length > 0 && enriched ? (
        <ForceGraph3D
          ref={graphRef}
          graphData={enriched}
          width={width} height={height}
          backgroundColor={bgColor}
          nodeColor={colorAccessor}
          nodeThreeObject={nodeThreeObj}
          nodeThreeObjectExtend={true}
          linkColor={linkColorAccessor}
          linkWidth={linkWidthAcc}
          linkDirectionalParticles={3}
          linkDirectionalParticleWidth={1}
          linkDirectionalParticleSpeed={0.005}
          linkCurvature={graphSettings.link_curvature}
          d3AlphaDecay={graphSettings.alpha_decay}
          d3VelocityDecay={graphSettings.velocity_decay}
          onNodeClick={onClick}
          onNodeRightClick={onRightClick}
          onNodeHover={onHover}
          onNodeDragEnd={onDragEnd}
          enableNodeDrag={true}
          enableNavigationControls={true}
          controlType="trackball"
          showNavInfo={false}
          nodeResolution={8}
          numDimensions={3}
          warmupTicks={0}
          cooldownTicks={300}
        />
      ) : !loading && !error ? (
        <Box sx={{ position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 2 }}>
          <Typography variant="body2" color="white">No graph data yet</Typography>
          {graphData && <Button variant="outlined" size="small" onClick={() => navigate('/')}>Go to Pages</Button>}
        </Box>
      ) : null}
    </Box>
  );
});

export default GraphCanvas;
