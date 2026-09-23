import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import Button from '@mui/material/Button';
import LinearProgress from '@mui/material/LinearProgress';

// ---------------------------------------------------------------------------
// Mobile developer tools section.
// Extracted from SettingsPage.mobile.tsx during the mobile/desktop parity pass.
// Mirrors the desktop DeveloperTab surface — reindex, repair, normalize-all,
// and live reindex progress.
// ---------------------------------------------------------------------------

export interface MobileDeveloperSectionProps {
  fetching: boolean;
  onReindex: () => void;
  onRepair: () => void;
  onNormalizeAll: () => void;
  reindexProgress: { message: string; percent: number } | null;
}

export default function MobileDeveloperSection({
  fetching,
  onReindex,
  onRepair,
  onNormalizeAll,
  reindexProgress,
}: MobileDeveloperSectionProps) {
  return (
    <Box sx={{ px: 2, pt: 3 }}>
      <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
        Developer
      </Typography>
      <Divider sx={{ mb: 1.5 }} />
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
        Re-sync all pages from disk into the database. Useful after importing new notes or
        recovering from a corrupted database. This operation is idempotent — running it multiple
        times produces the same result.
      </Typography>
      <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap', mb: 1 }}>
        <Button
          variant="contained"
          color="error"
          onClick={onReindex}
          disabled={fetching}
          size="small"
          sx={{ textTransform: 'none' }}
        >
          {fetching ? 'Reindexing...' : 'Reindex All'}
        </Button>
        <Button
          variant="outlined"
          color="error"
          onClick={onRepair}
          disabled={fetching}
          size="small"
          sx={{ textTransform: 'none' }}
        >
          {fetching ? 'Repairing...' : 'Repair DB from disk'}
        </Button>
      </Box>
      {reindexProgress && (
        <Box sx={{ mb: 2 }}>
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
            {reindexProgress.message}
          </Typography>
          <LinearProgress
            variant="determinate"
            value={reindexProgress.percent * 100}
            sx={{ height: 6, borderRadius: 3 }}
          />
        </Box>
      )}
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
        Parse every .md file through the block parser and re-serialize. This normalizes
        indentation, block syntax, and frontmatter across your entire vault.
      </Typography>
      <Button
        variant="contained"
        color="warning"
        onClick={onNormalizeAll}
        disabled={fetching}
        size="small"
        sx={{ textTransform: 'none' }}
      >
        {fetching ? 'Normalizing...' : 'Normalize All Files'}
      </Button>
    </Box>
  );
}
