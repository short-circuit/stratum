import { useEffect, useRef } from 'react';
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
import { useDatalogQuery } from './QueryPanel.shared';

export default function QueryPanelMobile() {
  const { datalog, setDatalog, result, error, running, doQuery, resetQuery } = useDatalogQuery();
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  return (
    <Box sx={{ p: 2 }}>
      <Typography variant="h6" sx={{ fontWeight: 600, mb: 1.5 }}>Datalog Query</Typography>

      <TextField
        inputRef={inputRef}
        multiline
        minRows={4}
        value={datalog}
        onChange={e => setDatalog(e.target.value)}
        placeholder="Enter Datalog query..."
        fullWidth
        sx={{ mb: 1.5, '& .MuiInputBase-root': { fontFamily: 'monospace', fontSize: '0.875rem' } }}
      />

      <Box sx={{ display: 'flex', gap: 1, mb: 2 }}>
        <Button variant="contained" size="small" onClick={doQuery} disabled={running}>
          {running ? 'Running...' : 'Run'}
        </Button>
        <Button variant="text" size="small" onClick={resetQuery}>
          Reset
        </Button>
      </Box>

      {error && (
        <Alert severity="error" sx={{ mb: 1.5 }}>{error}</Alert>
      )}

      {result && result.rows.length > 0 && (
        <Box sx={{ overflow: 'auto' }}>
          <Table size="small" padding="none">
            <TableHead>
              <TableRow>
                {result.columns.map((col, i) => (
                  <TableCell key={i} sx={{ fontWeight: 600, fontSize: '0.75rem', px: 0.5 }}>{col}</TableCell>
                ))}
              </TableRow>
            </TableHead>
            <TableBody>
              {result.rows.map((row, i) => (
                <TableRow key={i}>
                  {row.map((cell, j) => (
                    <TableCell key={j} sx={{ fontSize: '0.75rem', px: 0.5 }}>{cell}</TableCell>
                  ))}
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Box>
      )}

      {result && result.rows.length === 0 && (
        <Typography variant="body2" color="text.secondary">No results.</Typography>
      )}
    </Box>
  );
}
