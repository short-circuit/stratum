import Box from '@mui/material/Box';
import Chip from '@mui/material/Chip';
import Typography from '@mui/material/Typography';
import type { PluginInfoDto } from '../../lib/types';

/**
 * Renders a plugin's manifest display metadata (spec §2): description, author,
 * and declared-enabled hooks (spec §8). Returns null when the manifest declares
 * none of these so plugin cards with absent metadata stay compact.
 */
export function PluginManifestMeta({ plugin }: { plugin: PluginInfoDto }) {
  const hooks = plugin.hooks ?? [];
  const hasManifest = Boolean(plugin.description || plugin.author || hooks.length > 0);
  if (!hasManifest) return null;

  return (
    <Box sx={{ mt: 1 }}>
      {plugin.description && (
        <Typography variant="body2" color="text.secondary" sx={{ mb: 0.5 }}>
          {plugin.description}
        </Typography>
      )}
      {plugin.author && (
        <Typography variant="caption" color="text.disabled" sx={{ display: 'block' }}>
          By {plugin.author}
        </Typography>
      )}
      {hooks.length > 0 && (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, flexWrap: 'wrap', mt: 0.5 }}>
          <Typography variant="caption" color="text.disabled">Hooks:</Typography>
          {hooks.map(hook => (
            <Chip key={hook} label={hook} size="small" variant="outlined" sx={{ height: 18, fontSize: '0.62rem' }} />
          ))}
        </Box>
      )}
    </Box>
  );
}
