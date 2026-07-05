import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';

interface ResearchTabProps {
  research: { searxng_endpoint: string; max_results: number; max_depth: number };
  onResearchChange: (patch: Partial<ResearchTabProps['research']>) => void;
}

export default function ResearchTab({ research, onResearchChange }: ResearchTabProps) {
  return (
    <Box>
      <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
        Web Research (SearXNG)
      </Typography>

      <Box sx={{ maxWidth: 480, display: 'flex', flexDirection: 'column', gap: 2 }}>
        <TextField
          label="SearXNG Endpoint URL"
          placeholder="http://localhost:8888"
          value={research.searxng_endpoint}
          onChange={e => onResearchChange({ searxng_endpoint: e.target.value })}
          size="small"
          helperText="The URL of your SearXNG instance (e.g. http://localhost:8888)"
        />

        <TextField
          label="Max Results Per Search"
          type="number"
          value={research.max_results}
          onChange={e => onResearchChange({ max_results: parseInt(e.target.value) || 3 })}
          size="small"
          slotProps={{ htmlInput: { min: 1, max: 10 } }}
          sx={{ width: 200 }}
        />

        <TextField
          label="Research Depth (search-read cycles)"
          type="number"
          value={research.max_depth}
          onChange={e => onResearchChange({ max_depth: parseInt(e.target.value) || 2 })}
          size="small"
          slotProps={{ htmlInput: { min: 1, max: 5 } }}
          sx={{ width: 200 }}
        />
      </Box>
    </Box>
  );
}
