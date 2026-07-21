import { useMemo, useRef, useState, useCallback } from 'react';
import katex from 'katex';
import DOMPurify from 'dompurify';
import Box from '@mui/material/Box';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import Alert from '@mui/material/Alert';
import CloseIcon from '@mui/icons-material/Close';
import MathSymbolPalette from './MathSymbolPalette';

interface Props {
  initialLatex: string;
  onSave: (latex: string) => void;
  onCancel: () => void;
}

export default function MathEditorModal({ initialLatex, onSave, onCancel }: Props) {
  const [latex, setLatex] = useState(initialLatex);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const { html, error } = useMemo(() => {
    if (!latex.trim()) {
      return { html: null, error: null };
    }
    try {
      const rendered = katex.renderToString(latex, { displayMode: true, throwOnError: true });
      return { html: rendered, error: null };
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      return { html: null, error: msg };
    }
  }, [latex]);

  const insertAtCursor = useCallback((sym: string) => {
    const ta = textareaRef.current;
    if (!ta) return;
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const before = latex.slice(0, start);
    const after = latex.slice(end);
    const next = before + sym + after;
    setLatex(next);
    requestAnimationFrame(() => {
      ta.focus();
      const pos = start + sym.length;
      ta.setSelectionRange(pos, pos);
    });
  }, [latex]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      onSave(latex);
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      onCancel();
    }
  };

  return (
    <Dialog open maxWidth="md" fullWidth onClose={onCancel}>
      <DialogTitle sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Typography variant="body1" sx={{ fontWeight: 500 }}>Edit Equation</Typography>
        <IconButton size="small" onClick={onCancel}>
          <CloseIcon fontSize="small" />
        </IconButton>
      </DialogTitle>

      <MathSymbolPalette onInsert={insertAtCursor} />

      <DialogContent sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        <TextField
          inputRef={(node) => {
            textareaRef.current = node;
            node?.focus();
          }}
          multiline
          minRows={4}
          value={latex}
          onChange={(e) => setLatex(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="E = mc^2"
          spellCheck={false}
          sx={{ '& .MuiInputBase-root': { fontFamily: 'monospace', fontSize: '0.875rem' } }}
        />

        <Box>
          <Typography variant="caption" color="text.disabled" sx={{ textTransform: 'uppercase', fontWeight: 500, display: 'block', mb: 0.5 }}>
            Preview
          </Typography>
          <Box
            sx={{ minHeight: 60, p: 2, border: 1, borderColor: 'divider', borderRadius: 1, bgcolor: 'action.hover', display: 'flex', alignItems: 'center', justifyContent: 'center', overflowX: 'auto' }}
            dangerouslySetInnerHTML={{
              __html: html
                ? DOMPurify.sanitize(html)
                : error
                  ? DOMPurify.sanitize(`<span style="color:#f44336;font-size:0.875rem">${error}</span>`)
                  : '<span style="color:#9e9e9e;font-size:0.875rem">Preview</span>',
            }}
          />
        </Box>

        {error && (
          <Alert severity="error" sx={{ fontSize: '0.75rem' }}>{error}</Alert>
        )}

        <Typography variant="caption" color="text.disabled" sx={{ lineHeight: 1.6 }}>
          Ctrl+Enter to save · Esc to cancel<br />
          <Box
            component="a"
            href="https://katex.org/docs/supported.html"
            target="_blank"
            rel="noopener noreferrer"
            sx={{ color: 'primary.main', '&:hover': { textDecoration: 'underline' } }}
          >
            KaTeX supported functions
          </Box>
        </Typography>
      </DialogContent>

      <DialogActions>
        <Button onClick={onCancel}>Cancel</Button>
        <Button variant="contained" onClick={() => onSave(latex)}>Save</Button>
      </DialogActions>
    </Dialog>
  );
}
