//! Shared logic for GraphPanel — state management, data loading, filtering, navigation.
//! Provides the useGraphPanel hook consumed by both desktop and mobile variants.

import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import * as api from '../../lib/commands';
import type { GraphSettings, GraphDataDto, ComponentDto, OrphanDto } from '../../lib/types';
import { DEFAULT_SETTINGS, type GraphNode } from './GraphCanvas';

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

  // ---- settings persistence ----

  // Load graph settings from config on mount
  useEffect(() => {
    api.getSettings()
      .then((s) => {
        if (s.graph) setGraphSettings(s.graph);
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
  const { nodes: filteredNodes, edges: filteredEdges } = useMemo(() => {
    if (!graphData) {
      return { nodes: [] as GraphNode[], edges: [] as { source: string; target: string }[] };
    }

    // View mode: component
    if (viewMode === 'component' && components.length > 0) {
      const idx = Math.min(selectedComponent, components.length - 1);
      const comp = components[idx];
      const idSet = new Set(comp.nodes.map((n) => n.id));
      return {
        nodes: comp.nodes as GraphNode[],
        edges: graphData.edges.filter((e) => idSet.has(e.source) && idSet.has(e.target)),
      };
    }

    // View mode: orphans
    if (viewMode === 'orphans') {
      const orphanIds = new Set(orphans.map((o) => o.slug));
      return {
        nodes: graphData.nodes.filter((n) => orphanIds.has(n.id)) as GraphNode[],
        edges: [],
      };
    }

    // Visibility toggles (only applies in 'full' mode)
    let filteredNodes = graphData.nodes;
    if (!graphSettings.show_connected && !graphSettings.show_orphaned) {
      filteredNodes = [];
    } else if (!graphSettings.show_connected && graphSettings.show_orphaned) {
      const orphanIds = new Set(orphans.map((o) => o.slug));
      filteredNodes = graphData.nodes.filter((n) => orphanIds.has(n.id));
    } else if (graphSettings.show_connected && !graphSettings.show_orphaned) {
      const orphanIds = new Set(orphans.map((o) => o.slug));
      filteredNodes = graphData.nodes.filter((n) => !orphanIds.has(n.id));
    }

    // Text search filter
    if (search) {
      const q = search.toLowerCase();
      filteredNodes = filteredNodes.filter(
        (n) =>
          n.title.toLowerCase().includes(q) ||
          n.id.toLowerCase().includes(q) ||
          n.tags.some((t) => t.toLowerCase().includes(q)),
      );
      const matched = new Set(filteredNodes.map((n) => n.id));
      return {
        nodes: filteredNodes as GraphNode[],
        edges: graphData.edges.filter((e) => matched.has(e.source) || matched.has(e.target)),
      };
    }

    return {
      nodes: filteredNodes as GraphNode[],
      edges: graphData.edges,
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
  ]);

  // Data shaped for ForceGraph consumption
  const graphDataProp = useMemo(
    () => ({ nodes: filteredNodes, links: filteredEdges }),
    [filteredNodes, filteredEdges],
  );

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
  };
}
