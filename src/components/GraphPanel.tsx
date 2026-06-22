import { useEffect, useState, useCallback, useMemo } from 'react';
import ForceGraph2D from 'react-force-graph-2d';
import { useNavigate } from 'react-router-dom';
import * as api from '../lib/commands';
import type { GraphNodeDto, GraphEdgeDto, GraphDataDto, ComponentDto, OrphanDto } from '../lib/types';

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

export default function GraphPanel() {
  const navigate = useNavigate();
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

  const { nodes, edges } = useMemo(() => {
    if (!graphData) return { nodes: [] as GraphNode[], edges: [] as GraphEdgeDto[] };

    if (viewMode === 'component' && components.length > 0) {
      const idx = Math.min(selectedComponent, components.length - 1);
      const comp = components[idx];
      const idSet = new Set(comp.nodes.map((n) => n.id));
      return {
        nodes: comp.nodes as GraphNode[],
        edges: graphData.edges.filter(
          (e) => idSet.has(e.source) && idSet.has(e.target),
        ),
      };
    }

    if (viewMode === 'orphans') {
      const orphanIds = new Set(orphans.map((o) => o.slug));
      return {
        nodes: graphData.nodes.filter((n) => orphanIds.has(n.id)) as GraphNode[],
        edges: [],
      };
    }

    if (search) {
      const q = search.toLowerCase();
      const matched = new Set(
        graphData.nodes
          .filter(
            (n) =>
              n.title.toLowerCase().includes(q) ||
              n.id.toLowerCase().includes(q) ||
              n.tags.some((t) => t.toLowerCase().includes(q)),
          )
          .map((n) => n.id),
      );
      return {
        nodes: graphData.nodes.filter((n) => matched.has(n.id)) as GraphNode[],
        edges: graphData.edges.filter(
          (e) => matched.has(e.source) || matched.has(e.target),
        ),
      };
    }

    return {
      nodes: graphData.nodes as GraphNode[],
      edges: graphData.edges,
    };
  }, [graphData, viewMode, selectedComponent, components, orphans, search]);

  const handleNodeClick = useCallback(
    (node: GraphNode) => {
      navigate(`/page/${encodeURIComponent(node.path)}`);
    },
    [navigate],
  );

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
        ctx.fillText(node.title, node.x || 0, (node.y || 0) - HIGHLIGHT_R - 4);
      }
    },
    [highlight],
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
          placeholder="Search nodes..."
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            if (e.target.value) setViewMode('full');
          }}
          className="text-xs px-2 py-1 w-48 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-700)] text-[var(--secondary-900)] dark:text-[var(--secondary-100)]"
        />

        {graphData && (
          <span className="text-xs text-[var(--secondary-400)] whitespace-nowrap">
            {graphData.node_count} notes · {graphData.edge_count} links
            {orphans.length > 0 && ` · ${orphans.length} orphaned`}
          </span>
        )}
      </div>

      {/* Graph canvas */}
      <div className="flex-1 relative bg-[var(--secondary-100)] dark:bg-[var(--secondary-900)]">
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-[var(--secondary-100)]/70 dark:bg-[var(--secondary-900)]/70 z-10">
            <div className="text-sm text-[var(--secondary-500)]">
              Building graph...
            </div>
          </div>
        )}

        {error && (
          <div className="absolute top-4 left-1/2 -translate-x-1/2 bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-200 px-3 py-1.5 rounded text-xs z-10">
            {error}
          </div>
        )}

        {nodes.length > 0 && (
          <>
            {graphData && graphData.node_count > 0 && graphData.edge_count === 0 && (
              <div className="absolute top-4 left-1/2 -translate-x-1/2 bg-amber-100 dark:bg-amber-900 text-amber-800 dark:text-amber-200 px-3 py-1.5 rounded text-xs z-10 shadow">
                Pages found but no wiki-links yet. Add <code className="bg-amber-200 dark:bg-amber-800 px-1 rounded">[[Page Name]]</code> in your notes to create connections.
              </div>
            )}
            <ForceGraph2D
            graphData={{ nodes, links: edges }}
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
            linkColor={() => '#6b728066'}
            linkDirectionalArrowLength={3}
            linkDirectionalArrowRelPos={1}
            linkWidth={0.5}
            cooldownTicks={100}
            d3AlphaDecay={0.02}
            d3VelocityDecay={0.3}
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
              <button
                onClick={() => navigate('/')}
                className="text-xs px-3 py-1 rounded bg-[var(--primary-500)] text-white hover:bg-[var(--primary-600)]"
              >
                Go to Pages
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
