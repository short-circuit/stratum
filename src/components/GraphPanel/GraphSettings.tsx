import { memo } from 'react';
import Collapse from '@mui/material/Collapse';
import Box from '@mui/material/Box';
import FormControlLabel from '@mui/material/FormControlLabel';
import Checkbox from '@mui/material/Checkbox';
import Typography from '@mui/material/Typography';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import SliderRow from '../ui/SliderRow';
import type { GraphSettings } from '../../lib/types';

interface GraphSettingsPanelProps {
  settingsOpen: boolean;
  graphSettings: GraphSettings;
  updateSetting: <K extends keyof GraphSettings>(key: K, value: GraphSettings[K]) => void;
}

const GraphSettingsPanel = memo(function GraphSettingsPanel({ settingsOpen, graphSettings, updateSetting }: GraphSettingsPanelProps) {
  return (
    <Collapse in={settingsOpen}>
      <Box sx={{ px: 2, py: 1.5, borderBottom: 1, borderColor: 'divider', bgcolor: 'background.paper' }}>
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

          <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gapX: 3, gapY: 1 }}>
          <SliderRow label="Repulsion" value={graphSettings.charge_strength} min={-30} max={0} step={1} onChange={v => updateSetting('charge_strength', v)} display={`${Math.round(graphSettings.charge_strength)}`} />
          <SliderRow label="Link distance" value={graphSettings.link_distance} min={10} max={120} step={5} onChange={v => updateSetting('link_distance', v)} display={`${Math.round(graphSettings.link_distance)}px`} />
          <SliderRow label="Alpha decay" value={graphSettings.alpha_decay} min={0.01} max={0.3} step={0.01} onChange={v => updateSetting('alpha_decay', v)} display={graphSettings.alpha_decay.toFixed(2)} />
          <SliderRow label="Friction" value={graphSettings.velocity_decay} min={0.05} max={0.95} step={0.05} onChange={v => updateSetting('velocity_decay', v)} display={graphSettings.velocity_decay.toFixed(2)} />
          <SliderRow label="Curvature" value={graphSettings.link_curvature} min={0} max={0.5} step={0.05} onChange={v => updateSetting('link_curvature', v)} display={graphSettings.link_curvature.toFixed(2)} />
        </Box>

        <Box sx={{ mt: 1.5, display: 'flex', alignItems: 'center', gap: 1.5 }}>
          <Typography variant="caption" sx={{ color: 'text.secondary', width: 100, textAlign: 'right', flexShrink: 0 }}>
            Node cap
          </Typography>
          <Select
            size="small"
            value={graphSettings.node_cap}
            onChange={(e) => updateSetting('node_cap', Number(e.target.value))}
            sx={{ fontSize: '0.8rem', minWidth: 120 }}
          >
            <MenuItem value={500}>500</MenuItem>
            <MenuItem value={1000}>1000</MenuItem>
            <MenuItem value={2000}>2000</MenuItem>
            <MenuItem value={5000}>5000</MenuItem>
            <MenuItem value={10000}>10000</MenuItem>
            <MenuItem value={0}>Unlimited</MenuItem>
          </Select>
        </Box>
      </Box>
    </Collapse>
  );
});

export default GraphSettingsPanel;
