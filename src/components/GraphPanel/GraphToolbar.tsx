import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';
import TextField from '@mui/material/TextField';
import IconButton from '@mui/material/IconButton';
import Button from '@mui/material/Button';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import SettingsIcon from '@mui/icons-material/Settings';
import type { ComponentDto, OrphanDto, GraphDataDto } from '../../lib/types';

interface GraphToolbarProps {
  viewMode: 'full' | 'component' | 'orphans';
  onViewModeChange: (mode: 'full' | 'component' | 'orphans') => void;
  loading: boolean;
  onRefresh: () => void;
  components: ComponentDto[];
  selectedComponent: number;
  onSelectedComponentChange: (index: number) => void;
  search: string;
  onSearchChange: (value: string) => void;
  settingsOpen: boolean;
  onSettingsToggle: () => void;
  nodes: { length: number };
  edges: { length: number };
  orphans: OrphanDto[];
  graphData: GraphDataDto | null;
  saveStatus: 'saved' | 'unsaved';
}

export default function GraphToolbar({
  viewMode,
  onViewModeChange,
  loading,
  onRefresh,
  components,
  selectedComponent,
  onSelectedComponentChange,
  search,
  onSearchChange,
  settingsOpen,
  onSettingsToggle,
  nodes,
  edges,
  orphans,
  graphData,
  saveStatus,
}: GraphToolbarProps) {
  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, px: 1.5, py: 0.75, borderBottom: 1, borderColor: 'divider', bgcolor: 'background.paper', flexShrink: 0 }}>
      <Button size="small" variant="contained" onClick={onRefresh} disabled={loading}>
        {loading ? 'Loading...' : 'Refresh'}
      </Button>

      <ToggleButtonGroup
        value={viewMode}
        exclusive
        onChange={(_, v) => v && onViewModeChange(v)}
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
            onChange={(e) => onSelectedComponentChange(Number(e.target.value))}
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
          onSearchChange(e.target.value);
          if (e.target.value) onViewModeChange('full');
        }}
        sx={{ width: 180, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
      />

      <IconButton
        size="small"
        onClick={onSettingsToggle}
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
  );
}
