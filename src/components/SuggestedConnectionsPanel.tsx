import { useState } from 'react';
import Box from '@mui/material/Box';
import Accordion from '@mui/material/Accordion';
import AccordionSummary from '@mui/material/AccordionSummary';
import AccordionDetails from '@mui/material/AccordionDetails';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import IconButton from '@mui/material/IconButton';
import LinkIcon from '@mui/icons-material/Link';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import * as api from '../lib/commands';
import type { ConnectionSuggestion } from '../lib/types';
import { useNavigate } from 'react-router-dom';

interface Props {
  pagePath: string;
}

export default function SuggestedConnectionsPanel({ pagePath }: Props) {
  const navigate = useNavigate();
  const [suggestions, setSuggestions] = useState<ConnectionSuggestion[] | null>(null);
  const [busy, setBusy] = useState(false);

  const findConnections = async () => {
    setBusy(true);
    setSuggestions(null);
    try {
      const result = await api.suggestConnections(pagePath);
      setSuggestions(result);
    } catch (e) {
      console.error('Failed to find connections:', e);
    } finally {
      setBusy(false);
    }
  };

  const addLink = async (targetTitle: string) => {
    try {
      await api.insertBlock(pagePath, `See also: [[${targetTitle}]]`);
      findConnections();
    } catch (e) {
      console.error('Failed to add link:', e);
    }
  };

  return (
    <Accordion slotProps={{ transition: { unmountOnExit: true } }}>
      <AccordionSummary expandIcon={<ExpandMoreIcon />}>
        <Typography variant="caption" sx={{ fontWeight: 600, textTransform: 'uppercase', color: 'text.secondary' }}>
          Suggested Connections
        </Typography>
      </AccordionSummary>
      <AccordionDetails sx={{ p: 1.5 }}>
        <Box sx={{ mb: 1 }}>
          <Button size="small" variant="contained" onClick={findConnections} disabled={busy} sx={{ fontSize: '0.7rem' }}>
            {busy ? 'Searching...' : 'Find'}
          </Button>
        </Box>

        {suggestions && suggestions.length === 0 && (
          <Typography variant="caption" color="text.disabled">No connections found.</Typography>
        )}

        {suggestions && suggestions.length > 0 && (
          <List dense disablePadding>
            {suggestions.map(s => (
              <ListItemButton
                key={s.page_path}
                dense
                sx={{ borderRadius: 1, flexDirection: 'column', alignItems: 'flex-start' }}
              >
                <Box sx={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', width: '100%', gap: 1 }}>
                  <Box
                    onClick={() => navigate(`/page/${encodeURIComponent(s.page_path)}`)}
                    sx={{ cursor: 'pointer', '&:hover': { textDecoration: 'underline' } }}
                  >
                    <Typography variant="caption" color="primary" sx={{ fontWeight: 500 }}>{s.title}</Typography>
                  </Box>
                  <IconButton
                    size="small"
                    onClick={() => addLink(s.title)}
                    title="Add [[wiki-link]] to this page"
                    sx={{ p: 0.25 }}
                  >
                    <LinkIcon fontSize="inherit" />
                  </IconButton>
                </Box>
                {s.snippet && (
                  <Typography variant="caption" color="text.disabled" sx={{ display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical', overflow: 'hidden', mt: 0.25 }}>
                    {s.snippet}
                  </Typography>
                )}
                <Typography variant="caption" color="text.disabled" sx={{ fontSize: '0.65rem', mt: 0.25 }}>
                  Relevance: {Math.min(100, s.score)}%
                </Typography>
              </ListItemButton>
            ))}
          </List>
        )}

        {suggestions === null && !busy && (
          <Typography variant="caption" color="text.disabled">
            Click "Find" to discover related notes.
          </Typography>
        )}
      </AccordionDetails>
    </Accordion>
  );
}
