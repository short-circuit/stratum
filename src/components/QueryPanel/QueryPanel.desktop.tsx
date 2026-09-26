import { useState } from 'react';
import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Alert from '@mui/material/Alert';
import Table from '@mui/material/Table';
import TableHead from '@mui/material/TableHead';
import TableBody from '@mui/material/TableBody';
import TableRow from '@mui/material/TableRow';
import TableCell from '@mui/material/TableCell';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemText from '@mui/material/ListItemText';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import CircularProgress from '@mui/material/CircularProgress';
import Divider from '@mui/material/Divider';
import EditIcon from '@mui/icons-material/Edit';
import DeleteIcon from '@mui/icons-material/Delete';
import SaveIcon from '@mui/icons-material/Save';
import RefreshIcon from '@mui/icons-material/Refresh';
import { useDatalogQuery, useSavedQueries, formatUpdatedAt } from './QueryPanel.shared';
import SavedQueryDialogs from './SavedQueryDialogs';
import type { SavedQueryDialogState } from './SavedQueryDialogs';
import ConfirmDialog from '../ui/ConfirmDialog';
import type { SavedQuery } from '../../lib/types';

function SavedQueryRow({
  q,
  onLoad,
  onRename,
  onDelete,
}: {
  q: SavedQuery;
  onLoad: () => void;
  onRename: () => void;
  onDelete: () => void;
}) {
  return (
    <ListItemButton
      dense
      onClick={onLoad}
      sx={{ borderRadius: 1, '&:hover .sq-actions': { opacity: 1 } }}
    >
      <ListItemText
        primary={q.name}
        slotProps={{
          primary: { variant: 'body2', noWrap: true },
          secondary: { variant: 'caption', color: 'text.disabled' as const },
        }}
        secondary={formatUpdatedAt(q.updated_at)}
      />
      <Box
        className="sq-actions"
        sx={{ display: 'flex', opacity: 0, transition: 'opacity 0.15s', ml: 1 }}
      >
        <Tooltip title="Rename" arrow>
          <IconButton
            size="small"
            aria-label={`Rename saved query ${q.name}`}
            onClick={(e) => { e.stopPropagation(); onRename(); }}
            sx={{ p: 0.5 }}
          >
            <EditIcon fontSize="inherit" />
          </IconButton>
        </Tooltip>
        <Tooltip title="Delete" arrow>
          <IconButton
            size="small"
            aria-label={`Delete saved query ${q.name}`}
            onClick={(e) => { e.stopPropagation(); onDelete(); }}
            sx={{ p: 0.5, color: 'error.main' }}
          >
            <DeleteIcon fontSize="inherit" />
          </IconButton>
        </Tooltip>
      </Box>
    </ListItemButton>
  );
}

export default function QueryPanelDesktop() {
  const { datalog, setDatalog, result, error, running, doQuery, resetQuery } = useDatalogQuery();
  const saved = useSavedQueries(datalog);
  const [dialog, setDialog] = useState<SavedQueryDialogState | null>(null);
  const [renameTarget, setRenameTarget] = useState<SavedQuery | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);

  const handleLoad = (q: SavedQuery) => {
    setDatalog(q.query);
  };

  const handleSaveSubmit = async (name: string) => {
    await saved.saveCurrent(name);
    setDialog(null);
  };

  const handleRenameSubmit = async (name: string) => {
    if (renameTarget) {
      await saved.renameQuery(renameTarget.name, name);
    }
    setRenameTarget(null);
    setDialog(null);
  };

  return (
    <>
      <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <Box sx={{ flex: 1, overflow: 'auto', p: 3, maxWidth: 800, mx: 'auto' }}>
          <Typography variant="h5" sx={{ fontWeight: 600, mb: 2 }}>Datalog Query</Typography>

          <TextField
            multiline
            minRows={6}
            value={datalog}
            onChange={e => setDatalog(e.target.value)}
            placeholder="Enter Datalog query..."
            fullWidth
            sx={{ mb: 1.5, '& .MuiInputBase-root': { fontFamily: 'monospace', fontSize: '0.875rem' } }}
          />

          <Box sx={{ display: 'flex', gap: 1, mb: 2 }}>
            <Button variant="contained" onClick={doQuery} disabled={running}>
              {running ? 'Running...' : 'Run Query'}
            </Button>
            <Button
              variant="outlined"
              startIcon={<SaveIcon />}
              onClick={() => setDialog({ kind: 'save', initial: '' })}
            >
              Save Query
            </Button>
            <Box sx={{ flex: 1 }} />
            <Button variant="text" onClick={resetQuery}>
              Reset
            </Button>
          </Box>

          {error && (
            <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>
          )}

          {result && result.rows.length > 0 && (
            <Box sx={{ overflow: 'auto' }}>
              <Table size="small">
                <TableHead>
                  <TableRow>
                    {result.columns.map((col, i) => (
                      <TableCell key={i} sx={{ fontWeight: 600 }}>{col}</TableCell>
                    ))}
                  </TableRow>
                </TableHead>
                <TableBody>
                  {result.rows.map((row, i) => (
                    <TableRow key={i}>
                      {row.map((cell, j) => (
                        <TableCell key={j}>{cell}</TableCell>
                      ))}
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </Box>
          )}

          {result && result.rows.length === 0 && (
            <Typography variant="body2" color="text.secondary">Query returned no results.</Typography>
          )}

          <Divider sx={{ my: 3 }} />

          <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1 }}>
            <Typography variant="h6" sx={{ fontWeight: 600 }}>Saved Queries ({saved.saved.length})</Typography>
            <Tooltip title="Refresh list" arrow>
              <IconButton size="small" onClick={() => void saved.refresh()} aria-label="Refresh saved queries">
                <RefreshIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Box>

          {saved.loadError && (
            <Alert severity="error" sx={{ mb: 1.5 }} onClose={() => saved.refresh()}>
              Could not load saved queries: {saved.loadError}
            </Alert>
          )}
          {saved.actionError && (
            <Alert severity="warning" sx={{ mb: 1.5 }} onClose={saved.clearActionError}>
              {saved.actionError}
            </Alert>
          )}

          {saved.loading ? (
            <CircularProgress size={22} sx={{ display: 'block', mx: 'auto', my: 3 }} />
          ) : saved.saved.length === 0 ? (
            <Typography variant="body2" color="text.secondary">
              No saved queries yet. Write a query above and click “Save Query”.
            </Typography>
          ) : (
            <List dense disablePadding>
              {saved.saved.map(q => (
                <SavedQueryRow
                  key={q.name}
                  q={q}
                  onLoad={() => handleLoad(q)}
                  onRename={() => { setRenameTarget(q); setDialog({ kind: 'rename', initial: q.name }); }}
                  onDelete={() => setDeleteTarget(q.name)}
                />
              ))}
            </List>
          )}
        </Box>
      </Box>

      <SavedQueryDialogs
        dialog={dialog}
        target={renameTarget}
        onDismiss={() => { setDialog(null); setRenameTarget(null); }}
        onSubmit={dialog?.kind === 'rename' ? handleRenameSubmit : handleSaveSubmit}
      />

      <ConfirmDialog
        open={deleteTarget !== null}
        title="Delete saved query"
        message={`Delete “${deleteTarget ?? ''}”? This cannot be undone.`}
        confirmLabel="Delete"
        confirmColor="error"
        onConfirm={async () => {
          if (deleteTarget) await saved.deleteQuery(deleteTarget);
          setDeleteTarget(null);
        }}
        onCancel={() => setDeleteTarget(null)}
      />
    </>
  );
}
