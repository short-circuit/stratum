import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import { useTheme } from '@mui/material/styles';
import * as api from '../../lib/commands';
import type { GraphSettings, GraphDataDto, ComponentDto, OrphanDto } from '../../lib/types';
import GraphToolbar from './GraphToolbar';
import GraphSettingsPanel from './GraphSettings';
import GraphCanvas, { DEFAULT_SETTINGS, type GraphNode } from './GraphCanvas';

export default function GraphPanel() {
  const navigate = useNavigate();
  const muiTheme = useTheme();
  const bgColor = muiTheme.palette.background.default;
  const textColor = muiTheme.palette.text.primary;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const graphRef = useRef<any>(null);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [graphData, setGraphData] = useState<GraphDataDto | null>(null);
  const [components, setComponents] = useState<ComponentDto[]>([]);
  const [orphans, setOrphans] = useState<OrphanDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<'full' | 'component' | 'orphans'>('full');
  const [selectedComponent, setSelectedComponent] = useState<number>(0);
  const [search, setSearch] = useState('');
  const [width, setWidth] = useState(800);
  const [height, setHeight] = useState(600);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [graphSettings, setGraphSettings] = useState<GraphSettings>(DEFAULT_SETTINGS);
  const [saveStatus, setSaveStatus] = useState<'saved' | 'unsaved'>('saved');

  // Load graph settings from config
  useEffect(() => {
    api.getSettings().then(s => {
      if (s.graph) {
        setGraphSettings(s.graph);
      }
    }).catch(() => {});
  }, []);

  // Save graph settings whenever they change (debounced)
  const initialized = useRef(false);
  useEffect(() => {
    if (!initialized.current) { initialized.current = true; return; }
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      setSaveStatus('unsaved');
      api.saveGraphSettings(graphSettings).then(() => setSaveStatus('saved')).catch(() => {});
    }, 600);
  }, [graphSettings]);

  const updateSetting = useCallback(<K extends keyof GraphSettings>(key: K, value: GraphSettings[K]) => {
    setGraphSettings(prev => ({ ...prev, [key]: value }));
  }, []);

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

  useEffect(() => {
    const fetch = async () => {
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
    };
    fetch();
  }, []);

  useEffect(() => {
    const updateSize = () => {
      const settingsH = settingsOpen ? 160 : 0;
      setWidth(window.innerWidth - 240);
      setHeight(window.innerHeight - 48 - settingsH);
    };
    updateSize();
    window.addEventListener('resize', updateSize);
    return () => window.removeEventListener('resize', updateSize);
  }, [settingsOpen]);

  // Configure d3 forces when settings or graph data changes
  useEffect(() => {
    if (!graphRef.current || !graphData) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
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

  // Build the filtered node/edge list
  const { nodes, edges } = useMemo(() => {
    if (!graphData) return { nodes: [] as GraphNode[], edges: [] as { source: string; target: string }[] };

    // View mode filter
    if (viewMode === 'component' && components.length > 0) {
      const idx = Math.min(selectedComponent, components.length - 1);
      const comp = components[idx];
      const idSet = new Set(comp.nodes.map((n) => n.id));
      return {
        nodes: comp.nodes as GraphNode[],
        edges: graphData.edges.filter((e) => idSet.has(e.source) && idSet.has(e.target)),
      };
    }

    if (viewMode === 'orphans') {
      const orphanIds = new Set(orphans.map((o) => o.slug));
      return {
        nodes: graphData.nodes.filter((n) => orphanIds.has(n.id)) as GraphNode[],
        edges: [],
      };
    }

    // Visibility filters
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
  }, [graphData, viewMode, selectedComponent, components, orphans, search, graphSettings.show_connected, graphSettings.show_orphaned]);

  const graphDataProp = useMemo(() => ({ nodes, links: edges }), [nodes, edges]);

  const handleNodeClick = useCallback(
    (node: GraphNode) => {
      navigate(`/page/${encodeURIComponent(node.path)}`);
    },
    [navigate],
  );

  const handleNodeRightClick = useCallback(
    (node: GraphNode) => {
      if (!graphRef.current || node.x === undefined || node.y === undefined) return;
      const cam = graphRef.current.camera();
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const controls = graphRef.current.controls() as any;
      const target = { x: node.x, y: node.y, z: node.z || 0 };
      const lookAt = controls?.target || { x: 0, y: 0, z: 0 };
      graphRef.current.cameraPosition(
        { x: cam.position.x + target.x - lookAt.x, y: cam.position.y + target.y - lookAt.y, z: cam.position.z + target.z - lookAt.z },
        target,
        800,
      );
    },
    [],
  );

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <GraphToolbar
        viewMode={viewMode}
        onViewModeChange={setViewMode}
        loading={loading}
        onRefresh={loadData}
        components={components}
        selectedComponent={selectedComponent}
        onSelectedComponentChange={setSelectedComponent}
        search={search}
        onSearchChange={(v) => {
          setSearch(v);
          if (v) setViewMode('full');
        }}
        settingsOpen={settingsOpen}
        onSettingsToggle={() => setSettingsOpen(o => !o)}
        nodes={nodes}
        edges={edges}
        orphans={orphans}
        graphData={graphData}
        saveStatus={saveStatus}
      />

      <GraphSettingsPanel
        settingsOpen={settingsOpen}
        graphSettings={graphSettings}
        updateSetting={updateSetting}
      />

      <GraphCanvas
        graphDataProp={graphDataProp}
        width={width}
        height={height}
        bgColor={bgColor}
        textColor={textColor}
        handleNodeClick={handleNodeClick}
        handleNodeRightClick={handleNodeRightClick}
        loading={loading}
        error={error}
        nodes={nodes}
        graphData={graphData}
        graphSettings={graphSettings}
        graphRef={graphRef}
      />
    </Box>
  );
}
