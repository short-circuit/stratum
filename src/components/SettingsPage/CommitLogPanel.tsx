import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import type { CommitLogEntry } from '../../lib/types';

// ---------------------------------------------------------------------------
// CommitLogPanel — collapsible recent-commits table for the Sync settings tab.
// Extracted from SyncTab.tsx during the E6 sizing-gate refactor.
// ---------------------------------------------------------------------------

export default function CommitLogPanel({
  commits,
  commitsOpen,
  onToggleCommits,
}: {
  commits: CommitLogEntry[];
  commitsOpen: boolean;
  onToggleCommits: () => void;
}) {
  return (
    <Box>
      <Box
        component="button"
        onClick={onToggleCommits}
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 0.5,
          bgcolor: 'transparent',
          border: 'none',
          cursor: 'pointer',
          color: 'text.secondary',
          fontSize: '0.8rem',
          fontWeight: 500,
          p: 0,
          '&:hover': { color: 'text.primary' },
        }}
      >
        <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary' }}>
          {commitsOpen ? '▼' : '▶'} Recent Commits
        </Typography>
      </Box>
      {commitsOpen && (
        <Box sx={{ mt: 0.75, overflow: 'auto' }}>
          {commits.length === 0 ? (
            <Typography variant="caption" color="text.disabled">
              No commits yet.
            </Typography>
          ) : (
            <Box sx={{ minWidth: 500 }}>
              <Box
                sx={{
                  display: 'flex',
                  borderBottom: 1,
                  borderColor: 'divider',
                  pb: 0.5,
                  mb: 0.5,
                }}
              >
                {['Hash', 'Author', 'Message', 'Date'].map(h => (
                  <Typography
                    key={h}
                    variant="caption"
                    sx={{
                      fontWeight: 600,
                      color: 'text.secondary',
                      flex:
                        h === 'Hash'
                          ? '0 0 80px'
                          : h === 'Author'
                            ? '0 0 120px'
                            : h === 'Date'
                              ? '0 0 160px'
                              : 1,
                    }}
                  >
                    {h}
                  </Typography>
                ))}
              </Box>
              {commits.map(entry => (
                <Box
                  key={entry.hash}
                  sx={{ display: 'flex', py: 0.5, '&:hover': { bgcolor: 'action.hover' } }}
                >
                  <Typography
                    variant="caption"
                    sx={{
                      flex: '0 0 80px',
                      fontFamily: 'monospace',
                      color: 'var(--primary-500)',
                    }}
                  >
                    {entry.hash.slice(0, 7)}
                  </Typography>
                  <Typography
                    variant="caption"
                    sx={{
                      flex: '0 0 120px',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {entry.author}
                  </Typography>
                  <Typography
                    variant="caption"
                    sx={{
                      flex: 1,
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                      maxWidth: 200,
                    }}
                  >
                    {entry.message}
                  </Typography>
                  <Typography
                    variant="caption"
                    sx={{ flex: '0 0 160px', color: 'text.disabled' }}
                  >
                    {new Date(entry.timestamp).toLocaleString()}
                  </Typography>
                </Box>
              ))}
            </Box>
          )}
        </Box>
      )}
    </Box>
  );
}
