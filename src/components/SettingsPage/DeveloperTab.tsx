import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

interface DeveloperTabProps {
  fetching: boolean;
  onReindex: () => Promise<void>;
  onNormalizeAll: () => Promise<void>;
  reindexProgress: { message: string; percent: number } | null;
}

export default function DeveloperTab({ fetching, onReindex, onNormalizeAll, reindexProgress }: DeveloperTabProps) {
  return (
    <Box>
      <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>Developer Tools</Typography>

      <Box sx={{ maxWidth: 480 }}>
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
          Re-sync all pages from disk into the database. Useful after importing new notes or
          recovering from a corrupted database. This operation is idempotent — running it multiple
          times produces the same result.
        </Typography>
        <Button
          variant="contained"
          color="error"
          onClick={onReindex}
          disabled={fetching}
        >
          {fetching ? 'Reindexing...' : 'Reindex All'}
        </Button>
        {reindexProgress && (
          <Box sx={{ mt: 1.5 }}>
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
      </Box>

      <Box sx={{ maxWidth: 480, mt: 3 }}>
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
          Parse every .md file through the block parser and re-serialize. This normalizes
          indentation, block syntax, and frontmatter across your entire vault.
        </Typography>
        <Button
          variant="contained"
          color="warning"
          onClick={onNormalizeAll}
          disabled={fetching}
        >
          {fetching ? 'Normalizing...' : 'Normalize All Files'}
        </Button>
      </Box>
    </Box>
  );
}
