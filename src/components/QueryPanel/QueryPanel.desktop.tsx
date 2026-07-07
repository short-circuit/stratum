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

export default function QueryPanelDesktop() {
  const { datalog, setDatalog, result, error, running, doQuery, resetQuery } = useDatalogQuery();

  return (
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
      </Box>
    </Box>
  );
}
