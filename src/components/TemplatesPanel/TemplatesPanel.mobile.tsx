import { useState, useCallback } from 'react';
import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Alert from '@mui/material/Alert';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import IconButton from '@mui/material/IconButton';
import CloseIcon from '@mui/icons-material/Close';
import AddIcon from '@mui/icons-material/Add';
import PlaylistAddIcon from '@mui/icons-material/PlaylistAdd';
import { useTemplates, type TemplateDto } from './TemplatesPanel.shared';

export default function TemplatesPanelMobile() {
  const {
    templates,
    targetPath,
    setTargetPath,
    variables,
    message,
    setMessage,
    apply,
    addVariableRow,
    updateVariableKey,
    updateVariableValue,
  } = useTemplates();

  const [dialogOpen, setDialogOpen] = useState(false);
  const [activeTemplate, setActiveTemplate] = useState<TemplateDto | null>(null);

  const handleOpenApply = useCallback((t: TemplateDto) => {
    setActiveTemplate(t);
    setMessage('');
    setDialogOpen(true);
  }, [setMessage]);

  const handleClose = useCallback(() => {
    setDialogOpen(false);
    setActiveTemplate(null);
    setTargetPath('');
    setMessage('');
  }, [setTargetPath, setMessage]);

  const handleApply = useCallback(async () => {
    if (!activeTemplate) return;
    await apply(activeTemplate.name);
    if (!message.startsWith('Error')) {
      setTimeout(handleClose, 1200);
    }
  }, [activeTemplate, apply, message, handleClose]);

  return (
    <Box sx={{ p: 2 }}>
      <Typography variant="h6" sx={{ fontWeight: 600, mb: 2 }}>
        Templates
      </Typography>

      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
        {templates.map((t) => (
          <Card
            key={t.name}
            variant="outlined"
            onClick={() => handleOpenApply(t)}
            sx={{ '&:hover': { borderColor: 'primary.light' }, cursor: 'pointer' }}
          >
            <CardContent sx={{ py: 1.5, px: 2, '&:last-child': { pb: 1.5 } }}>
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <Box>
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    {t.name}
                  </Typography>
                  {t.description && (
                    <Typography variant="caption" color="text.secondary">
                      {t.description}
                    </Typography>
                  )}
                </Box>
                <PlaylistAddIcon fontSize="small" color="primary" />
              </Box>
            </CardContent>
          </Card>
        ))}
        {templates.length === 0 && (
          <Typography variant="body2" color="text.secondary">
            No templates yet. Save a page as template to get started.
          </Typography>
        )}
      </Box>

      <Dialog
        open={dialogOpen}
        onClose={handleClose}
        fullWidth
        slotProps={{
          paper: {
            sx: {
              borderRadius: '16px 16px 0 0',
              m: 0,
              maxHeight: '85vh',
              width: '100%',
            },
          },
        }}
        sx={{
          '& .MuiDialog-container': {
            alignItems: 'flex-end',
          },
        }}
      >
        <DialogTitle sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', pb: 1 }}>
          <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
            Apply: {activeTemplate?.name ?? ''}
          </Typography>
          <IconButton size="small" onClick={handleClose}>
            <CloseIcon />
          </IconButton>
        </DialogTitle>

        <DialogContent sx={{ pb: 1 }}>
          <TextField
            label="Target page path"
            placeholder="pages/my-new-page.md"
            value={targetPath}
            onChange={(e) => setTargetPath(e.target.value)}
            fullWidth
            size="small"
            sx={{ mb: 2, mt: 1 }}
            autoFocus
          />

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

          {message && (
            <Alert severity={message.startsWith('Error') ? 'error' : 'success'} sx={{ mt: 2 }}>
              {message}
            </Alert>
          )}
        </DialogContent>

        <DialogActions sx={{ px: 3, pb: 3 }}>
          <Button onClick={handleClose} color="inherit" sx={{ textTransform: 'none' }}>
            Cancel
          </Button>
          <Button
            onClick={handleApply}
            variant="contained"
            disabled={!targetPath}
            sx={{ textTransform: 'none' }}
          >
            Apply
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
