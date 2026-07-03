import { useState } from 'react';
import { useBlockNoteEditor, useComponentsContext } from '@blocknote/react';
import {
  FormattingToolbarController,
  FormattingToolbar,
  getFormattingToolbarItems,
} from '@blocknote/react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import CircularProgress from '@mui/material/CircularProgress';
import * as api from '../lib/commands';

export default function AIFormattingToolbar() {
  const editor = useBlockNoteEditor();
  const Components = useComponentsContext();
  const [busy, setBusy] = useState<string | null>(null);

  const runAiTransform = async (
    ed: { prosemirrorView: { state: { selection: { from: number; to: number }; doc: { textBetween: (f: number, t: number) => string } } }; pasteMarkdown: (md: string) => void },
    action: string,
  ) => {
    const { from, to } = ed.prosemirrorView.state.selection;
    if (from === to) return;
    const text = ed.prosemirrorView.state.doc.textBetween(from, to);
    if (!text.trim()) return;

    setBusy(action);
    try {
      let output = '';
      if (action === 'research') {
        const r = await api.aiResearch(text);
        output = r.findings;
      } else {
        const r = await api.aiTransformBlock(text, action as 'rewrite' | 'format' | 'summarize');
        output = r.content;
      }
      if (!output.trim()) return;
      ed.pasteMarkdown(output);
    } catch (e) {
      console.error('[AI] action failed:', e);
    } finally {
      setBusy(null);
    }
  };

  if (!Components) {
    return <FormattingToolbarController />;
  }

  const Btn = Components.FormattingToolbar.Button;

  return (
    <>
      {busy && (
        <Box sx={{ position: 'fixed', inset: 0, zIndex: 9998, bgcolor: 'rgba(0,0,0,0.1)', display: 'flex', justifyContent: 'center', pt: 16 }}>
          <Box sx={{ bgcolor: 'background.paper', border: 1, borderColor: 'divider', borderRadius: 2, boxShadow: 4, px: 3, py: 2, display: 'flex', alignItems: 'center', gap: 1.5 }}>
            <CircularProgress size={18} />
            <Typography variant="body2">AI {busy}...</Typography>
          </Box>
        </Box>
      )}
      <FormattingToolbarController
        formattingToolbar={({ blockTypeSelectItems }) => (
          <FormattingToolbar>
            {getFormattingToolbarItems(blockTypeSelectItems)}
            <Btn
              mainTooltip="Rewrite selected text with AI"
              onClick={() => runAiTransform(editor, 'rewrite')}
              isDisabled={busy !== null}
            >
              ✨
            </Btn>
            <Btn
              mainTooltip="Format selected text with AI"
              onClick={() => runAiTransform(editor, 'format')}
              isDisabled={busy !== null}
            >
              🎨
            </Btn>
            <Btn
              mainTooltip="Summarize selected text with AI"
              onClick={() => runAiTransform(editor, 'summarize')}
              isDisabled={busy !== null}
            >
              📝
            </Btn>
            <Btn
              mainTooltip="Research selected text on the web"
              onClick={() => runAiTransform(editor, 'research')}
              isDisabled={busy !== null}
            >
              🌐
            </Btn>
            <Btn
              mainTooltip="Generate Mermaid diagram from selection"
              onClick={async () => {
                const { from, to } = editor.prosemirrorView.state.selection;
                if (from === to) return;
                const text = editor.prosemirrorView.state.doc.textBetween(from, to);
                if (!text.trim()) return;
                setBusy('mermaid');
                try {
                  const code = await api.generateMermaid(text);
                  if (code.trim()) {
                    // eslint-disable-next-line @typescript-eslint/no-explicit-any
                    const pos = (editor as any).getTextCursorPosition();
                    // eslint-disable-next-line @typescript-eslint/no-explicit-any
                    (editor as any).insertBlocks(
                      [{ type: 'mermaid', props: { language: 'mermaid' }, content: [{ type: 'text', text: code, styles: {} }] }],
                      pos.block,
                      'after',
                    );
                  }
                } catch (e) {
                  console.error('[AI] mermaid generation failed:', e);
                } finally {
                  setBusy(null);
                }
              }}
              isDisabled={busy !== null}
            >
              📊
            </Btn>
          </FormattingToolbar>
        )}
      />
    </>
  );
}
