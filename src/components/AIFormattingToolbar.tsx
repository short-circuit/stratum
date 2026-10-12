import { useState } from 'react';
import { useBlockNoteEditor, useComponentsContext } from '@blocknote/react';
import {
  FormattingToolbarController,
  FormattingToolbar,
  getFormattingToolbarItems,
} from '@blocknote/react';
import { useAbortableInvoke } from '../lib/hooks/useAbortableInvoke';
import type { AiTransformResult, ResearchResult, TtsResult } from '../lib/types';
import { createMermaidBlock } from '../lib/mermaidBlock';
import AILoadingOverlay from './ui/AILoadingOverlay';
import { playAudio } from '../lib/audio';
import { useTranslation } from 'react-i18next';

export default function AIFormattingToolbar() {
  const { t } = useTranslation();
  const editor = useBlockNoteEditor();
  const Components = useComponentsContext();
  const [busy, setBusy] = useState<string | null>(null);
  const [speaking, setSpeaking] = useState(false);
  const { abortableInvoke } = useAbortableInvoke();

  // Read the currently selected block aloud via the configured TTS endpoint.
  const readAloud = async () => {
    const { from, to } = editor.prosemirrorView.state.selection;
    if (from === to) return;
    const text = editor.prosemirrorView.state.doc.textBetween(from, to);
    if (!text.trim()) return;
    setSpeaking(true);
    try {
      const res = await abortableInvoke<TtsResult>('tts_speak', { text });
      await playAudio(res.audio_b64, res.mime);
    } catch (e) {
      console.error('[TTS] read-aloud failed:', e);
      alert(`${t('aiformatting_toolbar.read_aloud_failed')} ${String(e)}`);
    } finally {
      setSpeaking(false);
    }
  };

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
        const r = await abortableInvoke<ResearchResult>('ai_research', { query: text });
        output = r.findings;
      } else {
        const r = await abortableInvoke<AiTransformResult>('ai_transform_block', { text, action });
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
      <AILoadingOverlay loading={!!busy} message={busy ? `AI ${busy}...` : undefined} />
      <FormattingToolbarController
        formattingToolbar={({ blockTypeSelectItems }) => (
          <FormattingToolbar>
            {getFormattingToolbarItems(blockTypeSelectItems)}
            <Btn
              mainTooltip={t('aiformatting_toolbar.rewrite_selected_text_with_ai')}
              onClick={() => runAiTransform(editor, 'rewrite')}
              isDisabled={busy !== null}
            >
              ✨
            </Btn>
            <Btn
              mainTooltip={t('aiformatting_toolbar.format_selected_text_with_ai')}
              onClick={() => runAiTransform(editor, 'format')}
              isDisabled={busy !== null}
            >
              🎨
            </Btn>
            <Btn
              mainTooltip={t('aiformatting_toolbar.summarize_selected_text_with_ai')}
              onClick={() => runAiTransform(editor, 'summarize')}
              isDisabled={busy !== null}
            >
              📝
            </Btn>
            <Btn
              mainTooltip={t('aiformatting_toolbar.research_selected_text_on_the_web')}
              onClick={() => runAiTransform(editor, 'research')}
              isDisabled={busy !== null}
            >
              🌐
            </Btn>
            <Btn
              mainTooltip={t('aiformatting_toolbar.generate_mermaid_diagram_from_selection')}
              onClick={async () => {
                const { from, to } = editor.prosemirrorView.state.selection;
                if (from === to) return;
                const text = editor.prosemirrorView.state.doc.textBetween(from, to);
                if (!text.trim()) return;
                setBusy('mermaid');
                try {
                  const result = await abortableInvoke<AiTransformResult>('generate_mermaid', { prompt: text });
                  const code = result.content;
                  if (code.trim()) {
                    const pos = editor.getTextCursorPosition();
                    editor.insertBlocks(
                      [createMermaidBlock(code)],
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
            <Btn
              mainTooltip={speaking ? t('aiformatting_toolbar.reading_aloud') : t('aiformatting_toolbar.read_selected_text_aloud')}
              onClick={readAloud}
              isDisabled={busy !== null || speaking}
            >
              🔊
            </Btn>
          </FormattingToolbar>
        )}
      />
    </>
  );
}
