import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import ForceGraph2D from 'react-force-graph-2d';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';
import TextField from '@mui/material/TextField';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import Collapse from '@mui/material/Collapse';
import Slider from '@mui/material/Slider';
import FormControlLabel from '@mui/material/FormControlLabel';
import Checkbox from '@mui/material/Checkbox';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import SettingsIcon from '@mui/icons-material/Settings';
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
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
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
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Toolbar */}
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, px: 1.5, py: 0.75, borderBottom: 1, borderColor: 'divider', bgcolor: 'background.paper', flexShrink: 0 }}>
        <Button size="small" variant="contained" onClick={loadData} disabled={loading}>
          {loading ? 'Loading...' : 'Refresh'}
        </Button>

        <ToggleButtonGroup
          value={viewMode}
          exclusive
          onChange={(_, v) => v && setViewMode(v)}
          size="small"
        >
          <ToggleButton value="full" sx={{ textTransform: 'none', fontSize: '0.75rem', px: 1.5 }}>Full</ToggleButton>
          <ToggleButton value="component" sx={{ textTransform: 'none', fontSize: '0.75rem', px: 1.5 }}>Components</ToggleButton>
          <ToggleButton value="orphans" sx={{ textTransform: 'none', fontSize: '0.75rem', px: 1.5 }}>Orphans</ToggleButton>
        </ToggleButtonGroup>

        {viewMode === 'component' && components.length > 0 && (
          <>
            <Select
              value={selectedComponent}
              onChange={(e) => setSelectedComponent(Number(e.target.value))}
              size="small"
              sx={{ fontSize: '0.75rem', minWidth: 160 }}
            >
              {components.map((c, i) => (
                <MenuItem key={i} value={i} sx={{ fontSize: '0.75rem' }}>
                  Component {i + 1} ({c.size} notes)
                </MenuItem>
              ))}
            </Select>
            <Typography variant="caption" color="text.secondary">
              {components.length} groups total
            </Typography>
          </>
        )}

        <Box sx={{ flex: 1 }} />

        <TextField
          size="small"
          placeholder="Filter by title/tag…"
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            if (e.target.value) setViewMode('full');
          }}
          sx={{ width: 180, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
        />

        <IconButton
          size="small"
          onClick={() => setSettingsOpen(o => !o)}
          color={settingsOpen ? 'primary' : 'default'}
          title="Graph settings"
        >
          <SettingsIcon fontSize="small" />
          {saveStatus === 'unsaved' && (
            <Typography variant="caption" sx={{ position: 'absolute', top: 0, right: 2, fontWeight: 700 }}>*</Typography>
          )}
        </IconButton>

        {graphData && (
          <Typography variant="caption" color="text.disabled" sx={{ whiteSpace: 'nowrap' }}>
            {nodes.length}/{graphData.node_count} n · {edges.length} e
            {orphans.length > 0 && ` · ${orphans.length} o`}
          </Typography>
        )}
      </Box>

      {/* Collapsible settings panel */}
      <Collapse in={settingsOpen}>
        <Box sx={{ px: 2, py: 1.5, borderBottom: 1, borderColor: 'divider', bgcolor: 'background.paper' }}>
          {/* Visibility toggles */}
          <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 2, mb: 1.5 }}>
            <FormControlLabel
              control={<Checkbox size="small" checked={graphSettings.show_connected} onChange={e => updateSetting('show_connected', e.target.checked)} />}
              label={<Typography variant="caption">Connected notes</Typography>}
            />
            <FormControlLabel
              control={<Checkbox size="small" checked={graphSettings.show_orphaned} onChange={e => updateSetting('show_orphaned', e.target.checked)} />}
              label={<Typography variant="caption">Orphaned notes</Typography>}
            />
            <FormControlLabel
              control={<Checkbox size="small" checked={graphSettings.show_tags} onChange={e => updateSetting('show_tags', e.target.checked)} />}
              label={<Typography variant="caption">Tags on hover</Typography>}
            />
          </Box>

          {/* Force sliders */}
          <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gapX: 3, gapY: 1 }}>
            <SliderRow label="Repulsion" value={graphSettings.charge_strength} min={-30} max={0} step={1} onChange={v => updateSetting('charge_strength', v)} display={`${Math.round(graphSettings.charge_strength)}`} />
            <SliderRow label="Link distance" value={graphSettings.link_distance} min={10} max={120} step={5} onChange={v => updateSetting('link_distance', v)} display={`${Math.round(graphSettings.link_distance)}px`} />
            <SliderRow label="Alpha decay" value={graphSettings.alpha_decay} min={0.01} max={0.3} step={0.01} onChange={v => updateSetting('alpha_decay', v)} display={graphSettings.alpha_decay.toFixed(2)} />
            <SliderRow label="Friction" value={graphSettings.velocity_decay} min={0.05} max={0.95} step={0.05} onChange={v => updateSetting('velocity_decay', v)} display={graphSettings.velocity_decay.toFixed(2)} />
          </Box>
        </Box>
      </Collapse>

      {/* Graph canvas */}
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
            d3AlphaDecay={graphSettings.alpha_decay}
            d3VelocityDecay={graphSettings.velocity_decay}
            enableNodeDrag={true}
            enableZoomInteraction={true}
            minZoom={0.2}
            maxZoom={8}
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
    </Box>
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
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
      <Typography variant="caption" sx={{ width: 100, textAlign: 'right', color: 'text.secondary', flexShrink: 0 }}>{label}</Typography>
      <Slider
        size="small"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(_, v) => onChange(v as number)}
        sx={{ flex: 1 }}
      />
      <Typography variant="caption" sx={{ width: 48, textAlign: 'left', color: 'text.disabled', fontFamily: 'monospace', fontSize: '0.7rem' }}>{display}</Typography>
    </Box>
  );
}
