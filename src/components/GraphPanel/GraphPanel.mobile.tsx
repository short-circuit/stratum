import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import AppBar from '@mui/material/AppBar';
import Toolbar from '@mui/material/Toolbar';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import TextField from '@mui/material/TextField';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';
import SwipeableDrawer from '@mui/material/SwipeableDrawer';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import Slider from '@mui/material/Slider';
import FormControlLabel from '@mui/material/FormControlLabel';
import Checkbox from '@mui/material/Checkbox';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import RefreshIcon from '@mui/icons-material/Refresh';
import SettingsIcon from '@mui/icons-material/Settings';
import SearchIcon from '@mui/icons-material/Search';
import { useTheme } from '@mui/material/styles';
import ForceGraph2D from 'react-force-graph-2d';
import { useGraphPanel } from './GraphPanel.shared';
import type { GraphNode } from './GraphCanvas';

const NODE_PALETTE = [
  '#fbbf24', '#60a5fa', '#34d399', '#f472b6',
  '#a78bfa', '#fb923c', '#2dd4bf', '#e879f9',
];
const UNTAGGED_COLOR = '#d4a574';

/** Deterministic colour per node based on first tag — mirrors GraphCanvas logic. */
function nodeColor(n: GraphNode): string {
  if (n.tags.length === 0) return UNTAGGED_COLOR;
  const idx = n.tags[0].split('').reduce((acc, c) => acc + c.charCodeAt(0), 0) % NODE_PALETTE.length;
  return NODE_PALETTE[idx];
}

function SliderSetting({ label, value, min, max, step, display, onChange }: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  display: string;
  onChange: (v: number) => void;
}) {
  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
        <Typography variant="caption">{label}</Typography>
        <Typography variant="caption" color="text.secondary">{display}</Typography>
      </Box>
      <Slider size="small" value={value} min={min} max={max} step={step} onChange={(_, v) => onChange(v as number)} />
    </Box>
  );
}

export default function GraphPanelMobile() {
  const navigate = useNavigate();
  const muiTheme = useTheme();

  const {
    state: { graphData, components, loading, error, viewMode,
            selectedComponent, search, graphSettings, saveStatus, graphRef },
    setViewMode, setSelectedComponent, setSearch, loadData,
    handleNodeClick, updateSetting,
    filteredNodes,
    graphDataProp,
  } = useGraphPanel();

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [width, setWidth] = useState(window.innerWidth);
  const [height, setHeight] = useState(window.innerHeight - 56);

  // Resize handler — full viewport minus app bar (56 px)
  useEffect(() => {
    const onResize = () => {
      setWidth(window.innerWidth);
      setHeight(window.innerHeight - 56);
    };
    onResize();
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  const handleSearchChange = useCallback((value: string) => {
    setSearch(value);
    if (value) setViewMode('full');
  }, [setSearch, setViewMode]);

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* ── Header Bar ── */}
      <AppBar position="static" color="default" elevation={1} sx={{ height: 56 }}>
        <Toolbar variant="dense" sx={{ gap: 0.5, minHeight: '56px !important' }}>
          <IconButton edge="start" size="small" onClick={() => navigate(-1)} aria-label="Back">
            <ArrowBackIcon />
          </IconButton>

          <ToggleButtonGroup
            value={viewMode}
            exclusive
            onChange={(_, v) => v && setViewMode(v)}
            size="small"
            sx={{ '& .MuiToggleButton-root': { fontSize: '0.7rem', px: 1, py: 0.25, textTransform: 'none' } }}
          >
            <ToggleButton value="full">All</ToggleButton>
            <ToggleButton value="component">Comp</ToggleButton>
            <ToggleButton value="orphans">Orphs</ToggleButton>
          </ToggleButtonGroup>

          {/* Component selector — visible only in component view mode */}
          {viewMode === 'component' && components.length > 0 && (
            <Select
              value={selectedComponent}
              onChange={(e) => setSelectedComponent(Number(e.target.value))}
              size="small"
              sx={{
                fontSize: '0.7rem', minWidth: 60, maxWidth: 100,
                '& .MuiSelect-select': { py: 0.25 },
              }}
            >
              {components.map((_, i) => (
                <MenuItem key={i} value={i} sx={{ fontSize: '0.75rem' }}>
                  #{i + 1}
                </MenuItem>
              ))}
            </Select>
          )}

          <Box sx={{ flex: 1 }} />

          {searchOpen && (
            <TextField
              autoFocus
              size="small"
              placeholder="Filter…"
              value={search}
              onChange={(e) => handleSearchChange(e.target.value)}
              sx={{ width: 130, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.5 } }}
            />
          )}

          <IconButton size="small" onClick={() => setSearchOpen((o) => !o)} aria-label="Toggle search">
            <SearchIcon fontSize="small" />
          </IconButton>
          <IconButton size="small" onClick={loadData} disabled={loading} aria-label="Refresh graph">
            <RefreshIcon fontSize="small" />
          </IconButton>
          <IconButton
            size="small"
            onClick={() => setSettingsOpen(true)}
            aria-label="Open settings"
          >
            <SettingsIcon fontSize="small" />
            {saveStatus === 'unsaved' && (
              <Typography
                variant="caption"
                sx={{ position: 'absolute', top: 0, right: 2, fontWeight: 700, fontSize: '0.6rem' }}
              >
                *
              </Typography>
            )}
          </IconButton>

          {graphData && (
            <Typography variant="caption" color="text.disabled" sx={{ whiteSpace: 'nowrap', ml: 0.25 }}>
              {filteredNodes.length}/{graphData.node_count}
            </Typography>
          )}
        </Toolbar>
      </AppBar>

      {/* ── Canvas ── */}
      <Box sx={{ flex: 1, position: 'relative', bgcolor: 'background.default' }}>
        {loading && (
          <Box
            sx={{
              position: 'absolute', inset: 0, display: 'flex',
              alignItems: 'center', justifyContent: 'center', zIndex: 10,
            }}
          >
            <CircularProgress size={18} sx={{ mr: 1 }} />
            <Typography variant="body2" color="text.secondary">Building graph...</Typography>
          </Box>
        )}

        {error && (
          <Alert severity="error" sx={{ position: 'absolute', top: 8, left: 8, right: 8, zIndex: 10 }}>
            {error}
          </Alert>
        )}

        {filteredNodes.length > 0 ? (
          <ForceGraph2D
            ref={graphRef}
            graphData={graphDataProp}
            width={width}
            height={height}
            backgroundColor={muiTheme.palette.background.default}
            nodeLabel="title"
            nodeColor={(n: GraphNode) => nodeColor(n)}
            nodeRelSize={6}
            linkColor={() => muiTheme.palette.text.primary}
            linkDirectionalArrowLength={3}
            linkDirectionalArrowRelPos={1}
            linkWidth={0.5}
            linkCurvature={graphSettings.link_curvature}
            d3AlphaDecay={graphSettings.alpha_decay}
            d3VelocityDecay={graphSettings.velocity_decay}
            onNodeClick={(n: GraphNode) => handleNodeClick(n)}
            enableNodeDrag={true}
            enableZoomInteraction={true}
            enablePanInteraction={true}
          />
        ) : (
          !loading && !error && (
            <Box sx={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', p: 3 }}>
              <Box sx={{ textAlign: 'center' }}>
                <Typography variant="body2" sx={{ fontWeight: 500, mb: 0.5 }}>No graph data yet</Typography>
                <Typography variant="caption" color="text.secondary">
                  Create pages with [[wiki-links]] to build connections.
                </Typography>
              </Box>
            </Box>
          )
        )}

        {!loading && !error && graphData && graphData.node_count > 0 && graphData.edge_count === 0 && (
          <Alert severity="warning" icon={false} sx={{ position: 'absolute', top: 8, left: 8, right: 8, zIndex: 10, boxShadow: 3 }}>
            <Typography variant="caption">
              Pages found but no wiki-links yet.
            </Typography>
          </Alert>
        )}
      </Box>

      {/* ── Bottom Sheet: Settings ── */}
      <SwipeableDrawer
        anchor="bottom"
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onOpen={() => setSettingsOpen(true)}
        disableSwipeToOpen={false}
        slotProps={{
          paper: { sx: { maxHeight: '70vh', borderTopLeftRadius: 16, borderTopRightRadius: 16, px: 3, py: 2.5 } },
        }}
      >
        <Box sx={{ width: '100%', maxWidth: 500, mx: 'auto' }}>
          <Typography variant="subtitle2" sx={{ mb: 2, textAlign: 'center', fontWeight: 600 }}>
            Graph Settings
          </Typography>

          <Box sx={{ display: 'flex', gap: 1.5, mb: 2, flexWrap: 'wrap', justifyContent: 'center' }}>
            <FormControlLabel
              control={<Checkbox size="small" checked={graphSettings.show_connected} onChange={(e) => updateSetting('show_connected', e.target.checked)} />}
              label={<Typography variant="caption">Connected</Typography>}
            />
            <FormControlLabel
              control={<Checkbox size="small" checked={graphSettings.show_orphaned} onChange={(e) => updateSetting('show_orphaned', e.target.checked)} />}
              label={<Typography variant="caption">Orphaned</Typography>}
            />
            <FormControlLabel
              control={<Checkbox size="small" checked={graphSettings.show_tags} onChange={(e) => updateSetting('show_tags', e.target.checked)} />}
              label={<Typography variant="caption">Tags</Typography>}
            />
          </Box>

          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
            <SliderSetting label="Repulsion" value={graphSettings.charge_strength} min={-30} max={0} step={1} display={`${Math.round(graphSettings.charge_strength)}`} onChange={(v) => updateSetting('charge_strength', v)} />
            <SliderSetting label="Link distance" value={graphSettings.link_distance} min={10} max={120} step={5} display={`${Math.round(graphSettings.link_distance)}px`} onChange={(v) => updateSetting('link_distance', v)} />
            <SliderSetting label="Alpha decay" value={graphSettings.alpha_decay} min={0.01} max={0.3} step={0.01} display={graphSettings.alpha_decay.toFixed(2)} onChange={(v) => updateSetting('alpha_decay', v)} />
            <SliderSetting label="Friction" value={graphSettings.velocity_decay} min={0.05} max={0.95} step={0.05} display={graphSettings.velocity_decay.toFixed(2)} onChange={(v) => updateSetting('velocity_decay', v)} />
            <SliderSetting label="Curvature" value={graphSettings.link_curvature} min={0} max={0.5} step={0.05} display={graphSettings.link_curvature.toFixed(2)} onChange={(v) => updateSetting('link_curvature', v)} />
          </Box>
        </Box>
      </SwipeableDrawer>
    </Box>
  );
}
