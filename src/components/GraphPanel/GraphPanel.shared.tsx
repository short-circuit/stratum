//! Shared logic for GraphPanel — state management, data loading, filtering, navigation.
//! Provides the useGraphPanel hook consumed by both desktop and mobile variants.

import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import * as api from '../../lib/commands';
import type { GraphSettings, GraphDataDto, GraphNodeDto, ComponentDto, OrphanDto } from '../../lib/types';
import { DEFAULT_SETTINGS, type GraphNode } from './GraphCanvas';

/** Number of nodes added per progressive-rendering batch. */
const CHUNK_SIZE = 200;

/** Minimum node count before the Web Worker layout offload kicks in. */
const LAYOUT_WORKER_MIN_NODES = 50;

/**
 * Return type for the useGraphPanel hook.
 * All state, setters, and derived data consumed by both desktop and mobile variants.
 */
export interface UseGraphPanelReturn {
  state: {
    graphData: GraphDataDto | null;
    components: ComponentDto[];
    orphans: OrphanDto[];
    loading: boolean;
    error: string | null;
    viewMode: 'full' | 'component' | 'orphans';
    selectedComponent: number;
    search: string;
    graphSettings: GraphSettings;
    saveStatus: 'saved' | 'unsaved';
    graphRef: React.MutableRefObject<any>;
  };
  setViewMode: (mode: 'full' | 'component' | 'orphans') => void;
  setSelectedComponent: (index: number) => void;
  setSearch: (value: string) => void;
  loadData: () => Promise<void>;
  /** Navigate to the page linked by a node — used by both 2D and 3D force graphs. */
  handleNodeClick: (node: GraphNode) => void;
  /** Focus camera on a node (3D-only — used by desktop variant). */
  handleNodeRightClick: (node: GraphNode) => void;
  updateSetting: <K extends keyof GraphSettings>(key: K, value: GraphSettings[K]) => void;
  /** Nodes that pass the current view-mode / visibility / text filters. */
  filteredNodes: GraphNode[];
  /** Edges whose source AND target are both in the filtered node set. */
  filteredEdges: { source: string; target: string }[];
  /** Data shaped for ForceGraph[2D|3D] consumption ({ nodes, links }). */
  graphDataProp: { nodes: GraphNode[]; links: { source: string; target: string }[] };
  /** Whether a node cap is actively limiting rendered nodes. */
  nodeCapActive: boolean;
  /** Node count before node_cap was applied (for the warning banner). */
  preCapNodeCount: number;
  /** Whether progressive rendering is still revealing node batches. */
  progressiveLoading: boolean;
  /** Progress of the progressive render: {current, total} nodes revealed so far. */
  progress: { current: number; total: number };
}

/**
 * Shared hook for GraphPanel state, data loading, filtering, and navigation.
 *
 * Desktop variant uses ForceGraph3D + GraphCanvas, GraphToolbar, GraphSettingsPanel.
 * Mobile variant uses ForceGraph2D + inline UI (bottom sheet, compact toolbar).
 * This hook provides the shared behaviour so both variants stay in sync.
 */
export function useGraphPanel(): UseGraphPanelReturn {
  const navigate = useNavigate();
  const graphRef = useRef<any>(null);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const initialized = useRef(false);

  // ---- state ----

  const [graphData, setGraphData] = useState<GraphDataDto | null>(null);
  const [components, setComponents] = useState<ComponentDto[]>([]);
  const [orphans, setOrphans] = useState<OrphanDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<'full' | 'component' | 'orphans'>('full');
  const [selectedComponent, setSelectedComponent] = useState<number>(0);
  const [search, setSearch] = useState('');
  const [graphSettings, setGraphSettings] = useState<GraphSettings>(DEFAULT_SETTINGS);
  const [saveStatus, setSaveStatus] = useState<'saved' | 'unsaved'>('saved');

  // ---- progressive rendering state ----

  /** How many chunks of CHUNK_SIZE nodes have been revealed so far. */
  const [displayBatch, setDisplayBatch] = useState(1);

  /** Pre-computed layout positions from the Web Worker, keyed by node id. */
  const [layoutPositions, setLayoutPositions] = useState<Map<string, { x: number; y: number; z: number }>>(new Map());

  // ---- settings persistence ----

  // Load graph settings from config on mount
  useEffect(() => {
    api.getSettings()
      .then((s) => {
        if (s.graph) setGraphSettings({ ...DEFAULT_SETTINGS, ...s.graph });
      })
      .catch(() => {});
  }, []);

  // Save graph settings whenever they change (debounced 600 ms)
  useEffect(() => {
    if (!initialized.current) {
      initialized.current = true;
      return;
    }
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      setSaveStatus('unsaved');
      api.saveGraphSettings(graphSettings).then(() => setSaveStatus('saved')).catch(() => {});
    }, 600);
  }, [graphSettings]);

  // ---- data loading ----

  const updateSetting = useCallback(
    <K extends keyof GraphSettings>(key: K, value: GraphSettings[K]) => {
      setGraphSettings((prev) => ({ ...prev, [key]: value }));
    },
    [],
  );

  const loadData = useCallback(async () => {
    setLoading(true);
    // Clear graph data before fetching so ForceGraph3D unmounts and remounts clean,
    // ensuring links are always processed (workaround for react-force-graph link
    // rendering bug on fresh mount with populated data).
    setGraphData(null);
    setError(null);
    try {
      const [data, comps, orphs] = await Promise.all([
        api.getGraphData(),
        api.getConnectedComponents(),
        api.getOrphanedNotes(),
      ]);
      setGraphData(data);
      setComponents(comps);
      setOrphans(orphs);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  // Fetch graph data on mount
  useEffect(() => {
    loadData();
  }, [loadData]);

  // ---- progressive rendering ----

  // Reset batch-and-a-half progress whenever the underlying graph data changes.
  useEffect(() => {
    if (graphData) {
      setDisplayBatch(1);
      setLayoutPositions(new Map());
    }
  }, [graphData]);

  // ---- Web Worker layout offload ----

  // Spawn a Web Worker to pre-compute d3-force positions off the main thread,
  // then merge the results back into layoutPositions.
  useEffect(() => {
    if (!graphData || graphData.nodes.length < LAYOUT_WORKER_MIN_NODES) return;
    const worker = new Worker(
      new URL('../../workers/force-layout.worker.ts', import.meta.url),
      { type: 'module' },
    );
    const nodes = graphData.nodes.map((n) => ({ id: n.id }));
    const links = graphData.edges.map((e) => ({
      source: e.source,
      target: e.target,
    }));
    worker.postMessage({
      nodes,
      links,
      chargeStrength: graphSettings.charge_strength,
      linkDistance: graphSettings.link_distance,
      alphaDecay: graphSettings.alpha_decay,
      velocityDecay: graphSettings.velocity_decay,
    });
    worker.onmessage = (event) => {
      const { positions } = event.data as { positions: { id: string; x: number; y: number; z: number }[] };
      const map = new Map<string, { x: number; y: number; z: number }>();
      for (const p of positions) {
        map.set(p.id, { x: p.x, y: p.y, z: p.z });
      }
      setLayoutPositions(map);
      worker.terminate();
    };
    worker.onerror = (err) => {
      console.warn('[GraphPanel] layout worker error:', err);
      worker.terminate();
    };
    return () => {
      worker.terminate();
    };
  }, [graphData, graphSettings.charge_strength, graphSettings.link_distance, graphSettings.alpha_decay, graphSettings.velocity_decay]);

  // ---- d3 force config ----

  // Configure d3 forces whenever settings or graph data changes.
  // Works with both ForceGraph2D and ForceGraph3D refs (both expose d3Force).
  useEffect(() => {
    if (!graphRef.current || !graphData) return;
    const g = graphRef.current as any;
    try {
      const charge = g.d3Force('charge');
      if (charge) charge.strength(graphSettings.charge_strength);
      const link = g.d3Force('link');
      if (link) link.distance(graphSettings.link_distance);
    } catch (e) {
      console.warn('[GraphPanel] force config:', e);
    }
  }, [graphData, graphSettings.charge_strength, graphSettings.link_distance]);

  // ---- filtering ----

  /**
   * Applies view-mode (full / component / orphans), visibility toggles
   * (show_connected / show_orphaned), and text search filter.
   */
  const { nodes: filteredNodes, edges: filteredEdges, preCapNodeCount, totalUnbatched } = useMemo(() => {
    if (!graphData) {
      return { nodes: [] as GraphNode[], edges: [] as { source: string; target: string }[], preCapNodeCount: 0, totalUnbatched: 0 };
    }

    let resultNodes: GraphNodeDto[];
    let resultEdges: { source: string; target: string }[];

    // View mode: component
    if (viewMode === 'component' && components.length > 0) {
      const idx = Math.min(selectedComponent, components.length - 1);
      const comp = components[idx];
      const idSet = new Set(comp.nodes.map((n) => n.id));
      resultNodes = comp.nodes;
      resultEdges = graphData.edges.filter((e) => idSet.has(e.source) && idSet.has(e.target));
    }
    // View mode: orphans
    else if (viewMode === 'orphans') {
      const orphanIds = new Set(orphans.map((o) => o.slug));
      resultNodes = graphData.nodes.filter((n) => orphanIds.has(n.id));
      resultEdges = [];
    }
    // Full mode with visibility toggles and search
    else {
      let nodes = graphData.nodes;
      if (!graphSettings.show_connected && !graphSettings.show_orphaned) {
        nodes = [];
      } else if (!graphSettings.show_connected && graphSettings.show_orphaned) {
        const orphanIds = new Set(orphans.map((o) => o.slug));
        nodes = graphData.nodes.filter((n) => orphanIds.has(n.id));
      } else if (graphSettings.show_connected && !graphSettings.show_orphaned) {
        const orphanIds = new Set(orphans.map((o) => o.slug));
        nodes = graphData.nodes.filter((n) => !orphanIds.has(n.id));
      }

      if (search) {
        const q = search.toLowerCase();
        nodes = nodes.filter(
          (n) =>
            n.title.toLowerCase().includes(q) ||
            n.id.toLowerCase().includes(q) ||
            n.tags.some((t) => t.toLowerCase().includes(q)),
        );
        const matched = new Set(nodes.map((n) => n.id));
        resultNodes = nodes;
        resultEdges = graphData.edges.filter((e) => matched.has(e.source) || matched.has(e.target));
      } else {
        resultNodes = nodes;
        resultEdges = graphData.edges;
      }
    }

    // Apply node cap
    const preCap = resultNodes.length;
    if (graphSettings.node_cap > 0 && resultNodes.length > graphSettings.node_cap) {
      resultNodes = resultNodes.slice(0, graphSettings.node_cap);
    }

    // Progressive rendering batch slice — reveals CHUNK_SIZE nodes at a time.
    const totalBeforeBatch = resultNodes.length;
    const batchLimit = displayBatch * CHUNK_SIZE;
    if (resultNodes.length > batchLimit) {
      resultNodes = resultNodes.slice(0, batchLimit);
    }

    return {
      nodes: resultNodes as GraphNode[],
      edges: resultEdges,
      preCapNodeCount: preCap,
      totalUnbatched: totalBeforeBatch,
    };
  }, [
    graphData,
    viewMode,
    selectedComponent,
    components,
    orphans,
    search,
    graphSettings.show_connected,
    graphSettings.show_orphaned,
    graphSettings.node_cap,
    displayBatch,
  ]);

  // Data shaped for ForceGraph consumption
  // Merges Web Worker pre-computed positions (if available) and filters edges
  // to only include connections between currently visible (unbatched) nodes.
  const graphDataProp = useMemo(() => {
    const visibleIds = new Set(filteredNodes.map((n) => n.id));
    const links = filteredEdges.filter(
      (e) => visibleIds.has(e.source) && visibleIds.has(e.target),
    );
    if (layoutPositions.size === 0) {
      return { nodes: filteredNodes, links };
    }
    const nodes = filteredNodes.map((n) => {
      const pos = layoutPositions.get(n.id);
      if (pos) return { ...n, x: pos.x, y: pos.y, z: pos.z };
      return n;
    });
    return { nodes, links };
  }, [filteredNodes, filteredEdges, layoutPositions]);

  // Increment displayBatch every 100 ms until all filtered nodes are shown.
  useEffect(() => {
    if (totalUnbatched === 0 || displayBatch * CHUNK_SIZE >= totalUnbatched) return;
    const timer = setInterval(() => {
      setDisplayBatch((prev) => prev + 1);
    }, 100);
    return () => clearInterval(timer);
  }, [displayBatch, totalUnbatched]);

  // ---- navigation ----

  const handleNodeClick = useCallback(
    (node: GraphNode) => {
      navigate(`/page/${encodeURIComponent(node.path)}`);
    },
    [navigate],
  );

  /** Focus the 3D camera on a node (right-click action for desktop). */
  const handleNodeRightClick = useCallback(
    (node: GraphNode) => {
      if (!graphRef.current || node.x === undefined || node.y === undefined) return;
      const cam = graphRef.current.camera();
      const controls = graphRef.current.controls() as any;
      const target = { x: node.x, y: node.y, z: node.z || 0 };
      const lookAt = controls?.target || { x: 0, y: 0, z: 0 };
      graphRef.current.cameraPosition(
        {
          x: cam.position.x + target.x - lookAt.x,
          y: cam.position.y + target.y - lookAt.y,
          z: cam.position.z + target.z - lookAt.z,
        },
        target,
        800,
      );
    },
    [],
  );

  const nodeCapActive = graphSettings.node_cap > 0 && preCapNodeCount > graphSettings.node_cap;
  const progressiveLoading = graphData !== null && totalUnbatched > displayBatch * CHUNK_SIZE;
  const progress = {
    current: Math.min(filteredNodes.length, totalUnbatched),
    total: totalUnbatched,
  };

  return {
    state: {
      graphData,
      components,
      orphans,
      loading,
      error,
      viewMode,
      selectedComponent,
      search,
      graphSettings,
      saveStatus,
      graphRef,
    },
    setViewMode,
    setSelectedComponent,
    setSearch,
    loadData,
    handleNodeClick,
    handleNodeRightClick,
    updateSetting,
    filteredNodes,
    filteredEdges,
    graphDataProp,
    nodeCapActive,
    preCapNodeCount,
    progressiveLoading,
    progress,
  };
}
