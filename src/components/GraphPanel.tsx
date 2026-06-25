import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import ForceGraph2D from 'react-force-graph-2d';
import { useNavigate } from 'react-router-dom';
import * as api from '../lib/commands';
import type { GraphSettings, GraphNodeDto, GraphEdgeDto, GraphDataDto, ComponentDto, OrphanDto } from '../lib/types';

interface GraphNode extends GraphNodeDto {
  x?: number;
  y?: number;
}

function tagColor(tags: string[]): string {
  if (tags.length === 0) return '#6b7280';
  const h = tags[0].split('').reduce((acc, c) => acc + c.charCodeAt(0), 0) % 360;
  return `hsl(${h}, 50%, 55%)`;
}

const NODE_R = 5;
const HIGHLIGHT_R = 8;

const DEFAULT_SETTINGS: GraphSettings = {
  show_connected: true,
  show_orphaned: true,
  show_tags: true,
  charge_strength: -8,
  link_distance: 40,
  alpha_decay: 0.08,
  velocity_decay: 0.3,
};

export default function GraphPanel() {
  const navigate = useNavigate();
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const graphRef = useRef<any>(null);
  const saveTimer = useRef<ReturnType<typeof setTimeout>>();
  const [graphData, setGraphData] = useState<GraphDataDto | null>(null);
  const [components, setComponents] = useState<ComponentDto[]>([]);
  const [orphans, setOrphans] = useState<OrphanDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<'full' | 'component' | 'orphans'>('full');
  const [selectedComponent, setSelectedComponent] = useState<number>(0);
  const [highlight, setHighlight] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState('');
  const [width, setWidth] = useState(800);
  const [height, setHeight] = useState(600);
  const [velocityDecay, setVelocityDecay] = useState(0.3);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [graphSettings, setGraphSettings] = useState<GraphSettings>(DEFAULT_SETTINGS);
  const [saveStatus, setSaveStatus] = useState<'saved' | 'unsaved'>('saved');

  // Load graph settings from config
  useEffect(() => {
    api.getSettings().then(s => {
      if (s.graph) {
        setGraphSettings(s.graph);
        setVelocityDecay(s.graph.velocity_decay);
      }
    }).catch(() => {});
  }, []);

  // Save graph settings whenever they change (debounced)
  const initialized = useRef(false);
  useEffect(() => {
    if (!initialized.current) { initialized.current = true; return; }
    if (saveTimer.current) clearTimeout(saveTimer.current);
    // eslint-disable-next-line react-hooks/set-state-in-effect
    saveTimer.current = setTimeout(() => {
      setSaveStatus('unsaved');
      api.saveGraphSettings(graphSettings).then(() => setSaveStatus('saved')).catch(() => {});
    }, 600);
  }, [graphSettings]);

  const updateSetting = useCallback(<K extends keyof GraphSettings>(key: K, value: GraphSettings[K]) => {
    setGraphSettings(prev => {
      const next = { ...prev, [key]: value };
      if (key === 'velocity_decay') setVelocityDecay(value as number);
      return next;
    });
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
      console.debug('[GraphPanel]', { nodes: data.node_count, edges: data.edge_count, vault: data.vault_path, components: comps.length, orphans: orphs.length });
      setGraphData(data);
      setComponents(comps);
      setOrphans(orphs);
    } catch (e) {
      console.error('[GraphPanel] load failed:', e);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    loadData();
  }, [loadData]);

  useEffect(() => {
    const updateSize = () => {
      setWidth(window.innerWidth - 240);
      setHeight(window.innerHeight - 48);
    };
    updateSize();
    window.addEventListener('resize', updateSize);
    return () => window.removeEventListener('resize', updateSize);
  }, []);

  // Configure d3 forces when settings or graph data changes
  useEffect(() => {
    if (!graphRef.current || !graphData) return;
    const g = graphRef.current;
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
    if (!graphData) return { nodes: [] as GraphNode[], edges: [] as GraphEdgeDto[] };

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

  const handleNodeDragStart = useCallback(() => {
    setVelocityDecay(0.85);
    const g = graphRef.current;
    if (g) {
      try {
        const charge = g.d3Force('charge');
        // Near-zero charge during drag: the internal d3ReheatSimulation
        // (alpha=1) has negligible repulsion so orphaned nodes stay put.
        if (charge) charge.strength(-0.01);
      } catch { /* d3Force getter may throw */ }
    }
  }, []);

  const handleNodeDragEnd = useCallback(() => {
    setVelocityDecay(graphSettings.velocity_decay);
    const g = graphRef.current;
    if (g) {
      try {
        const charge = g.d3Force('charge');
        if (charge) charge.strength(graphSettings.charge_strength);
      } catch { /* d3Force getter may throw */ }
    }
  }, [graphSettings.velocity_decay, graphSettings.charge_strength]);

  const handleNodeHover = useCallback(
    (node: GraphNode | null) => {
      if (!node) {
        setHighlight(new Set());
        return;
      }
      const connected = new Set<string>([node.id]);
      edges.forEach((e) => {
        if (e.source === node.id) connected.add(e.target);
        if (e.target === node.id) connected.add(e.source);
      });
      setHighlight(connected);
    },
    [edges],
  );

  const paintNode = useCallback(
    (node: GraphNode, ctx: CanvasRenderingContext2D) => {
      const r = highlight.has(node.id) ? HIGHLIGHT_R : NODE_R;
      const color = tagColor(node.tags);
      const isDimmed = highlight.size > 0 && !highlight.has(node.id);

      ctx.beginPath();
      ctx.arc(node.x || 0, node.y || 0, r, 0, 2 * Math.PI);
      ctx.fillStyle = isDimmed ? `${color}33` : color;
      ctx.fill();
      ctx.strokeStyle = isDimmed ? '#99999944' : '#ffffff88';
      ctx.lineWidth = 1;
      ctx.stroke();

      if (highlight.has(node.id)) {
        ctx.font = '10px sans-serif';
        ctx.fillStyle = '#e5e7eb';
        ctx.textAlign = 'center';
        const label = graphSettings.show_tags && node.tags.length > 0
          ? `${node.title}  [${node.tags.join(', ')}]`
          : node.title;
        ctx.fillText(label, node.x || 0, (node.y || 0) - HIGHLIGHT_R - 4);
      }
    },
    [highlight, graphSettings.show_tags],
  );

  return (
    <div className="flex flex-col h-full">
      {/* Toolbar */}
      <div className="flex items-center gap-3 px-4 py-2 border-b border-[var(--secondary-200)] dark:border-[var(--secondary-700)] bg-[var(--secondary-50)] dark:bg-[var(--secondary-800)] shrink-0">
        <button
          onClick={loadData}
          disabled={loading}
          className="text-xs px-2 py-1 rounded bg-[var(--primary-500)] text-white hover:bg-[var(--primary-600)] disabled:opacity-50"
        >
          {loading ? 'Loading...' : 'Refresh'}
        </button>

        <select
          value={viewMode}
          onChange={(e) => setViewMode(e.target.value as typeof viewMode)}
          className="text-xs px-2 py-1 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-700)] text-[var(--secondary-900)] dark:text-[var(--secondary-100)]"
        >
          <option value="full">Full Graph</option>
          <option value="component">Connected Components</option>
          <option value="orphans">Orphaned Notes</option>
        </select>

        {viewMode === 'component' && components.length > 0 && (
          <>
            <select
              value={selectedComponent}
              onChange={(e) => setSelectedComponent(Number(e.target.value))}
              className="text-xs px-2 py-1 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-700)] text-[var(--secondary-900)] dark:text-[var(--secondary-100)]"
            >
              {components.map((c, i) => (
                <option key={i} value={i}>
                  Component {i + 1} ({c.size} notes)
                </option>
              ))}
            </select>
            <span className="text-xs text-[var(--secondary-500)]">
              {components.length} groups total
            </span>
          </>
        )}

        <div className="flex-1" />

        <input
          type="text"
          placeholder="Filter by title/tag…"
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            if (e.target.value) setViewMode('full');
          }}
          className="text-xs px-2 py-1 w-44 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-700)] text-[var(--secondary-900)] dark:text-[var(--secondary-100)]"
        />

        <button
          onClick={() => setSettingsOpen(o => !o)}
          className={`text-xs px-2 py-1 rounded border ${settingsOpen ? 'bg-[var(--primary-100)] dark:bg-[var(--primary-900)]/30 border-[var(--primary-300)] dark:border-[var(--primary-700)]' : 'border-[var(--secondary-300)] dark:border-[var(--secondary-600)]'} hover:bg-[var(--secondary-200)] dark:hover:bg-[var(--secondary-700)]`}
          title="Graph settings"
        >
          ⚙ {saveStatus === 'unsaved' ? '*' : ''}
        </button>

        {graphData && (
          <span className="text-xs text-[var(--secondary-400)] whitespace-nowrap">
            {nodes.length}/{graphData.node_count} n · {edges.length} e
            {orphans.length > 0 && ` · ${orphans.length} o`}
          </span>
        )}
      </div>

      {/* Collapsible settings panel */}
      {settingsOpen && (
        <div className="px-4 py-3 border-b border-[var(--secondary-200)] dark:border-[var(--secondary-700)] bg-[var(--secondary-50)] dark:bg-[var(--secondary-800)] text-xs space-y-3 shrink-0">
          {/* Visibility toggles */}
          <div className="flex flex-wrap gap-4">
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input type="checkbox" checked={graphSettings.show_connected} onChange={e => updateSetting('show_connected', e.target.checked)} className="accent-[var(--primary-500)]" />
              Connected notes
            </label>
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input type="checkbox" checked={graphSettings.show_orphaned} onChange={e => updateSetting('show_orphaned', e.target.checked)} className="accent-[var(--primary-500)]" />
              Orphaned notes
            </label>
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input type="checkbox" checked={graphSettings.show_tags} onChange={e => updateSetting('show_tags', e.target.checked)} className="accent-[var(--primary-500)]" />
              Tags on hover
            </label>
          </div>

          {/* Force sliders */}
          <div className="grid grid-cols-2 gap-x-6 gap-y-2">
            <SliderRow label="Repulsion" value={graphSettings.charge_strength} min={-30} max={0} step={1} onChange={v => updateSetting('charge_strength', v)} display={`${Math.round(graphSettings.charge_strength)}`} />
            <SliderRow label="Link distance" value={graphSettings.link_distance} min={10} max={120} step={5} onChange={v => updateSetting('link_distance', v)} display={`${Math.round(graphSettings.link_distance)}px`} />
            <SliderRow label="Alpha decay" value={graphSettings.alpha_decay} min={0.01} max={0.3} step={0.01} onChange={v => updateSetting('alpha_decay', v)} display={graphSettings.alpha_decay.toFixed(2)} />
            <SliderRow label="Friction" value={graphSettings.velocity_decay} min={0.05} max={0.95} step={0.05} onChange={v => updateSetting('velocity_decay', v)} display={graphSettings.velocity_decay.toFixed(2)} />
          </div>
        </div>
      )}

      {/* Graph canvas */}
      <div className="flex-1 relative bg-[var(--secondary-100)] dark:bg-[var(--secondary-900)]">
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-[var(--secondary-100)]/70 dark:bg-[var(--secondary-900)]/70 z-10">
            <div className="text-sm text-[var(--secondary-500)]">Building graph...</div>
          </div>
        )}

        {error && (
          <div className="absolute top-4 left-1/2 -translate-x-1/2 bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-200 px-3 py-1.5 rounded text-xs z-10">{error}</div>
        )}

        {nodes.length > 0 && (
          <>
            {graphData && graphData.node_count > 0 && graphData.edge_count === 0 && (
              <div className="absolute top-4 left-1/2 -translate-x-1/2 bg-amber-100 dark:bg-amber-900 text-amber-800 dark:text-amber-200 px-3 py-1.5 rounded text-xs z-10 shadow">
                Pages found but no wiki-links yet. Add <code className="bg-amber-200 dark:bg-amber-800 px-1 rounded">[[Page Name]]</code> in your notes to create connections.
              </div>
            )}
            <ForceGraph2D
            ref={graphRef}
            graphData={graphDataProp}
            width={width}
            height={height}
            nodeLabel={(n: GraphNode) =>
              `${n.title}\n${n.tags.join(', ') || 'no tags'}`
            }
            nodeCanvasObject={(n: GraphNode, ctx: CanvasRenderingContext2D) =>
              paintNode(n, ctx)
            }
            nodePointerAreaPaint={(n: GraphNode, color: string, ctx: CanvasRenderingContext2D) => {
              ctx.beginPath();
              ctx.arc(n.x || 0, n.y || 0, HIGHLIGHT_R + 2, 0, 2 * Math.PI);
              ctx.fillStyle = color;
              ctx.fill();
            }}
            onNodeClick={(n: GraphNode) => handleNodeClick(n)}
            onNodeHover={(n: GraphNode | null) => handleNodeHover(n)}
            onNodeDragStart={handleNodeDragStart}
            onNodeDragEnd={handleNodeDragEnd}
            linkColor={() => '#6b728066'}
            linkDirectionalArrowLength={3}
            linkDirectionalArrowRelPos={1}
            linkWidth={0.5}
            d3AlphaDecay={graphSettings.alpha_decay}
            d3VelocityDecay={velocityDecay}
            enableNodeDrag={true}
            enableZoomInteraction={true}
            minZoom={0.2}
            maxZoom={8}
          />
          </>)}

        {!loading && !error && nodes.length === 0 && (
          <div className="absolute inset-0 flex items-center justify-center">
            <div className="text-sm text-[var(--secondary-400)] text-center max-w-sm px-4">
              <p className="mb-1 font-medium">No graph data yet</p>
              {graphData ? (
                <p className="text-xs mb-3">
                  Vault at <code className="bg-[var(--secondary-200)] dark:bg-[var(--secondary-700)] px-1 rounded">{graphData.vault_path}</code> has
                  0 .md files. Create pages first, then add{' '}
                  <code className="bg-[var(--secondary-200)] dark:bg-[var(--secondary-700)] px-1 rounded">[[wiki-links]]</code>{' '}
                  between them to build the graph.
                </p>
              ) : (
                <p className="text-xs mb-3">
                  Could not load graph data. Check that your vault has .md files with wiki-links.
                </p>
              )}
              <button onClick={() => navigate('/')} className="text-xs px-3 py-1 rounded bg-[var(--primary-500)] text-white hover:bg-[var(--primary-600)]">
                Go to Pages
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function SliderRow({ label, value, min, max, step, onChange, display }: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
  display: string;
}) {
  return (
    <label className="flex items-center gap-2">
      <span className="w-24 text-right text-[var(--secondary-500)] shrink-0">{label}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={e => onChange(parseFloat(e.target.value))}
        className="flex-1 accent-[var(--primary-500)] h-1.5"
      />
      <span className="w-12 text-left text-[var(--secondary-400)] font-mono tabular-nums">{display}</span>
    </label>
  );
}
