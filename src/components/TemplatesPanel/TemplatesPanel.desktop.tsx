import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Alert from '@mui/material/Alert';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import AddIcon from '@mui/icons-material/Add';
import { useTemplates } from './TemplatesPanel.shared';

export default function TemplatesPanelDesktop() {
  const {
    templates,
    targetPath,
    setTargetPath,
    variables,
    message,
    apply,
    addVariableRow,
    updateVariableKey,
    updateVariableValue,
  } = useTemplates();

  return (
    <Box sx={{ p: 3, maxWidth: 600, mx: 'auto' }}>
      <Typography variant="h5" sx={{ fontWeight: 600, mb: 2 }}>
        Templates
      </Typography>

      <TextField
        label="Target page path"
        placeholder="pages/my-new-page.md"
        value={targetPath}
        onChange={(e) => setTargetPath(e.target.value)}
        fullWidth
        size="small"
        sx={{ mb: 2 }}
      />

      <Box sx={{ mb: 2 }}>
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
          Variables
        </Typography>
        {variables.map(([k, v], i) => (
          <Box key={i} sx={{ display: 'flex', gap: 0.5, mb: 0.5 }}>
            <TextField
              size="small"
              placeholder="key"
              value={k}
              onChange={(e) => updateVariableKey(i, e.target.value)}
              sx={{ flex: 1, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
            />
            <TextField
              size="small"
              placeholder="value"
              value={v}
              onChange={(e) => updateVariableValue(i, e.target.value)}
              sx={{ flex: 1, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
            />
          </Box>
        ))}
        <Button
          size="small"
          startIcon={<AddIcon />}
          onClick={addVariableRow}
          sx={{ textTransform: 'none', fontSize: '0.75rem' }}
        >
          Add variable
        </Button>
      </Box>

      {message && (
        <Alert severity={message.startsWith('Error') ? 'error' : 'success'} sx={{ mb: 2 }}>
          {message}
        </Alert>
      )}

      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
        {templates.map((t) => (
          <Card
            key={t.name}
            variant="outlined"
            sx={{ '&:hover': { borderColor: 'primary.light' } }}
          >
            <CardContent sx={{ py: 1.5, px: 2, '&:last-child': { pb: 1.5 } }}>
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  mb: 0.5,
                }}
              >
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {t.name}
                </Typography>
                <Button
                  size="small"
                  variant="contained"
                  onClick={() => apply(t.name)}
                >
                  Apply
                </Button>
              </Box>
              {t.description && (
                <Typography variant="caption" color="text.secondary">
                  {t.description}
                </Typography>
              )}
            </CardContent>
          </Card>
        ))}
        {templates.length === 0 && (
          <Typography variant="body2" color="text.secondary">
            No templates yet. Save a page as template to get started.
          </Typography>
        )}
      </Box>
    </Box>
  );
}
