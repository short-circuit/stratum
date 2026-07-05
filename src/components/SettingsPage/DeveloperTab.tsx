import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

interface DeveloperTabProps {
  fetching: boolean;
  onReindex: () => Promise<void>;
}

export default function DeveloperTab({ fetching, onReindex }: DeveloperTabProps) {
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
      </Box>
    </Box>
  );
}
