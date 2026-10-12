import { useState } from 'react';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import CloseIcon from '@mui/icons-material/Close';
import { useTranslation } from 'react-i18next';
import type { KanbanBlockDto } from '../lib/types';

const MARKERS = ['TODO', 'DOING', 'DONE', 'NOW', 'LATER', 'WAITING', 'CANCELLED'] as const;

interface Props {
  block: KanbanBlockDto;
  onSave: (block: KanbanBlockDto) => void;
  onCancel: () => void;
}

export default function KanbanEditDialog({ block, onSave, onCancel }: Props) {
  const { t } = useTranslation();
  const [content, setContent] = useState(block.content);
  const [marker, setMarker] = useState(block.marker ?? '');
  const [priority, setPriority] = useState(block.priority ?? '');

  const handleSave = () => {
    onSave({
      ...block,
      content,
      marker: marker || null,
      priority: priority || null,
    });
  };

  return (
    <Dialog open maxWidth="sm" fullWidth onClose={onCancel}>
      <DialogTitle sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Typography variant="body1" sx={{ fontWeight: 600 }}>{t('kanban_edit_dialog.edit_card')}</Typography>
        <IconButton size="small" onClick={onCancel}>
          <CloseIcon fontSize="small" />
        </IconButton>
      </DialogTitle>

      <DialogContent sx={{ display: 'flex', flexDirection: 'column', gap: 2.5, pt: '20px !important' }}>
        <TextField
          label={t('kanban_edit_dialog.content')}
          multiline
          minRows={3}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          fullWidth
          autoFocus
          slotProps={{ input: { sx: { fontSize: '0.9rem' } } }}
        />

        <Box>
          <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', mb: 0.75, display: 'block' }}>
            {t('kanban_edit_dialog.marker')}
          </Typography>
          <ToggleButtonGroup
            value={marker}
            exclusive
            onChange={(_, v) => setMarker(v ?? '')}
            size="small"
          >
            {MARKERS.map((m) => (
              <ToggleButton key={m} value={m} sx={{ fontSize: '0.7rem', px: 1.5 }}>
                {m}
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
        </Box>

        <Box>
          <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', mb: 0.75, display: 'block' }}>
            {t('kanban_edit_dialog.priority')}
          </Typography>
          <ToggleButtonGroup
            value={priority}
            exclusive
            onChange={(_, v) => setPriority(v ?? '')}
            size="small"
          >
            <ToggleButton value="" sx={{ fontSize: '0.7rem', px: 1.5 }}>{t('kanban_edit_dialog.none')}</ToggleButton>
            {['A', 'B', 'C'].map((p) => (
              <ToggleButton key={p} value={p} sx={{ fontSize: '0.7rem', px: 1.5 }}>
                {p}
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
        </Box>

        <Typography variant="caption" color="text.disabled">
          {t('flashcards.source')} {block.page_title || block.page_path}
        </Typography>
      </DialogContent>

      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button onClick={onCancel}>{t('kanban_edit_dialog.cancel')}</Button>
        <Button variant="contained" onClick={handleSave}>
          {t('kanban_edit_dialog.save')}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
