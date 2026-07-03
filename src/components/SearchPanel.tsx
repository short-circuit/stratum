import { useState, useEffect, useCallback, startTransition } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import SearchIcon from '@mui/icons-material/Search';
import * as api from '../lib/commands';
import type { SearchResultDto } from '../lib/types';

export default function SearchPanel() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const [query, setQuery] = useState(searchParams.get('q') || '');
  const [results, setResults] = useState<SearchResultDto[]>([]);
  const [searching, setSearching] = useState(false);
  const [indexing, setIndexing] = useState(false);
  const [indexMsg, setIndexMsg] = useState('');

  const doSearch = useCallback(async (q: string) => {
    if (!q.trim()) return;
    setSearching(true);
    try {
      if (q.startsWith('#')) {
        const { results: r } = await api.searchByTag(q.slice(1));
        setResults(r);
      } else {
        const { results: r } = await api.searchBlocks(q, 20);
        setResults(r);
      }
    } catch (e) {
      console.error(e);
    } finally {
      setSearching(false);
    }
  }, []);

  // Auto-search when URL has ?q= param on mount or change
  useEffect(() => {
    const q = searchParams.get('q');
    if (q) {
      startTransition(() => { doSearch(q); });
    }
  }, [searchParams, doSearch]);

  const handleSearch = () => {
    if (!query.trim()) return;
    setSearchParams(query ? { q: query } : {});
    doSearch(query);
  };

  const doReindex = async () => {
    setIndexing(true);
    try {
      const msg = await api.rebuildSearchIndex();
      setIndexMsg(msg);
    } catch (e) {
      setIndexMsg(`Failed: ${e}`);
    } finally {
      setIndexing(false);
    }
  };

  return (
    <Box sx={{ p: 3, maxWidth: 700, mx: 'auto' }}>
      <Box sx={{ display: 'flex', gap: 1, mb: 1 }}>
        <TextField
          size="small"
          placeholder="Search blocks... (prefix with # to search tags)"
          value={query}
          onChange={e => setQuery(e.target.value)}
          onKeyDown={e => { if (e.key === 'Enter') handleSearch(); }}
          fullWidth
          slotProps={{ input: { startAdornment: <SearchIcon fontSize="small" sx={{ mr: 0.5, color: 'text.disabled' }} /> } }}
        />
        <Button variant="contained" onClick={handleSearch} disabled={searching} sx={{ whiteSpace: 'nowrap' }}>
          {searching ? '...' : 'Search'}
        </Button>
      </Box>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, mb: 2 }}>
        <Button size="small" variant="text" onClick={doReindex} disabled={indexing} sx={{ color: 'text.secondary', textTransform: 'none', fontSize: '0.75rem' }}>
          {indexing ? 'Indexing...' : 'Rebuild search index'}
        </Button>
        {indexMsg && <Typography variant="caption" color="text.secondary">{indexMsg}</Typography>}
      </Box>

      <List disablePadding>
        {results.map((r, i) => (
          <ListItemButton
            key={i}
            onClick={() => navigate(`/page/${encodeURIComponent(r.page_path)}`)}
            sx={{ borderRadius: 1, mb: 0.5, border: 1, borderColor: 'divider', flexDirection: 'column', alignItems: 'flex-start' }}
          >
            <Typography variant="caption" color="text.secondary" sx={{ mb: 0.25 }}>
              {r.page_path} · score: {r.score.toFixed(2)}
            </Typography>
            <Typography variant="body2">{r.snippet}</Typography>
          </ListItemButton>
        ))}
        {results.length === 0 && query && !searching && (
          <Typography variant="body2" color="text.secondary">No results found.</Typography>
        )}
      </List>
    </Box>
  );
}
