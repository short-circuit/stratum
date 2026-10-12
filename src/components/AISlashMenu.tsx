import { useCallback, useEffect, useRef, useState } from 'react';
import { useBlockNoteEditor } from '@blocknote/react';
import {
  getDefaultReactSlashMenuItems,
  SuggestionMenuController,
  type DefaultReactSuggestionItem,
} from '@blocknote/react';
import { filterSuggestionItems } from '@blocknote/core';
import { useAbortableInvoke } from '../lib/hooks/useAbortableInvoke';
import type { AiAction, AiTransformResult, ResearchResult } from '../lib/types';
import { createMermaidBlock } from '../lib/mermaidBlock';
import AILoadingOverlay from './ui/AILoadingOverlay';
import MathEditorModal from './MathEditorModal';
import { useTranslation } from 'react-i18next';

interface Props {
  pagePath: string;
}

function isJournal(path: string) {
  return path.startsWith('journals/');
}

export default function AISlashMenu({ pagePath }: Props) {
  const { t } = useTranslation();
  const editor = useBlockNoteEditor();
  const [loading, setLoading] = useState<string | null>(null);
  const [mathModal, setMathModal] = useState<{
    onSave: (latex: string) => void;
  } | null>(null);
  const capturedRef = useRef('');
  const { abortableInvoke } = useAbortableInvoke();

  // Capture selected text before "/" key replaces it
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === '/') {
        try {
          const dom = editor.prosemirrorView?.dom;
          if (!dom || !dom.contains(e.target as Node)) return;
          const sel = editor.getSelection();
          if (sel) {
            capturedRef.current = editor.getSelectedText();
          } else {
            capturedRef.current = '';
          }
        } catch {
          capturedRef.current = '';
        }
      }
    };
    document.addEventListener('keydown', onKeyDown, { capture: true });
    return () => document.removeEventListener('keydown', onKeyDown, { capture: true });
  }, [editor]);

  const getItems = useCallback(async (query: string) => {
    let defaultItems: DefaultReactSuggestionItem[] = [];
    try {
      defaultItems = getDefaultReactSlashMenuItems(editor);
    } catch {
      // default items not available
    }

    const aiItems: DefaultReactSuggestionItem[] = [];
    const capturedText = capturedRef.current;

    const makeItem = (
      title: string,
      subtext: string,
      action: AiAction,
      scope: 'selection' | 'page',
    ): DefaultReactSuggestionItem => ({
      title,
      subtext,
      aliases: [],
      group: 'AI',
      icon: <span style={{ fontSize: '0.75rem', opacity: 0.6 }}>✨</span>,
      onItemClick: () => {
        setLoading(title);
        (async () => {
          try {
            if (scope === 'selection') {
              const text = capturedRef.current;
              capturedRef.current = '';
              if (!text.trim()) return;
              const { content } = await abortableInvoke<AiTransformResult>('ai_transform_block', { text, action, pagePath });
              if (content.trim()) {
                editor.pasteMarkdown(content);
              }
            } else {
              const doc = editor.document;
              const allText = (doc as unknown[])
                .map((b: unknown) => extractText(b))
                .filter(Boolean)
                .join('\n\n');
              if (!allText.trim()) return;
              const { content } = await abortableInvoke<AiTransformResult>('ai_transform_block', { text: allText, action, pagePath });
              if (content.trim()) {
                const blocks = editor.tryParseMarkdownToBlocks(content);
                if (blocks.length > 0) {
                  editor.replaceBlocks(doc, blocks);
                }
              }
            }
          } catch (e) {
            console.error('[AI] action failed:', e);
          } finally {
            setLoading(null);
          }
        })();
      },
    });

    if (capturedText.trim()) {
      aiItems.push(
        makeItem(t('aislash_menu.rewrite'), t('aislash_menu.improve_clarity_and_flow'), 'rewrite', 'selection'),
        makeItem(t('aislash_menu.format_selection'), t('aislash_menu.clean_up_formatting'), 'format', 'selection'),
        makeItem(t('aislash_menu.summarize'), t('aislash_menu.condense_into_key_points'), 'summarize', 'selection'),
      );
    }

    aiItems.push({
      title: t('aislash_menu.research_with_web'),
      subtext: t('aislash_menu.search_the_web_and_write_research_notes'),
      aliases: ['research', 'search', 'web'],
      group: 'AI',
      icon: <span style={{ fontSize: '0.75rem', opacity: 0.6 }}>🌐</span>,
      onItemClick: async () => {
        setLoading(t('aislash_menu.researching'));
        try {
          const doc = editor.document;
          const allText = (doc as unknown[])
            .map((b: unknown) => extractText(b))
            .filter(Boolean)
            .join('\n\n')
            .trim();
          const query = allText || capturedRef.current || pagePath;
          capturedRef.current = '';
          if (!query.trim()) return;
          const result = await abortableInvoke<ResearchResult>('ai_research', { query });
          if (result.findings.trim()) {
            const blocks = editor.tryParseMarkdownToBlocks(result.findings);
            if (blocks.length > 0) {
              editor.replaceBlocks(doc, blocks);
            } else {
              editor.pasteMarkdown(result.findings);
            }
          }
        } catch (e) {
          console.error('[AI] research failed:', e);
        } finally {
          setLoading(null);
        }
      },
    });

    if (isJournal(pagePath)) {
      aiItems.push(
        makeItem(t('aislash_menu.structure_journal'), t('aislash_menu.organize_daily_notes_into_sections'), 'structure', 'page'),
        makeItem(t('aislash_menu.format_notes'), t('aislash_menu.clean_up_formatting_and_markdown'), 'format', 'page'),
      );
      // Custom interlink item that searches vault for existing notes
      aiItems.push({
        title: t('aislash_menu.interlink_notes'),
        subtext: t('aislash_menu.search_vault_and_add_wiki_links_to_related_notes'),
        aliases: ['interlink', 'wikilinks', 'connect'],
        group: 'AI',
        icon: <span style={{ fontSize: '0.75rem', opacity: 0.6 }}>🔗</span>,
        onItemClick: async () => {
          setLoading(t('aislash_menu.interlinking'));
          try {
            const md = editor.blocksToMarkdownLossy();
            if (!md.trim()) return;
            const { content } = await abortableInvoke<AiTransformResult>('ai_interlink_notes', { text: md, pagePath });
            if (content.trim()) {
              const blocks = editor.tryParseMarkdownToBlocks(content);
              if (blocks.length > 0) {
                editor.replaceBlocks(editor.document, blocks);
              } else {
                editor.pasteMarkdown(content);
              }
            }
          } catch (e) {
            console.error('[AI] interlink failed:', e);
          } finally {
            setLoading(null);
          }
        },
      });
    } else {
      aiItems.push(
        makeItem(t('aislash_menu.format_structure'), t('aislash_menu.organize_and_clean_up_the_page'), 'structure', 'page'),
        makeItem(t('aislash_menu.summarize_page'), t('aislash_menu.create_a_concise_page_summary'), 'summarize', 'page'),
      );
      aiItems.push({
        title: t('aislash_menu.interlink_notes'),
        subtext: t('aislash_menu.search_vault_and_add_wiki_links_to_related_notes'),
        aliases: ['interlink', 'wikilinks', 'connect'],
        group: 'AI',
        icon: <span style={{ fontSize: '0.75rem', opacity: 0.6 }}>🔗</span>,
        onItemClick: async () => {
          setLoading(t('aislash_menu.interlinking'));
          try {
            const md = editor.blocksToMarkdownLossy();
            if (!md.trim()) return;
            const { content } = await abortableInvoke<AiTransformResult>('ai_interlink_notes', { text: md, pagePath });
            if (content.trim()) {
              const blocks = editor.tryParseMarkdownToBlocks(content);
              if (blocks.length > 0) {
                editor.replaceBlocks(editor.document, blocks);
              } else {
                editor.pasteMarkdown(content);
              }
            }
          } catch (e) {
            console.error('[AI] interlink failed:', e);
          } finally {
            setLoading(null);
          }
        },
      });
    }

    // Math equation item (always available)
    aiItems.push({
      title: t('editor.math_equation_label'),
      subtext: t('aislash_menu.insert_a_latex_math_equation'),
      aliases: ['math', 'equation', 'latex', 'formula'],
      group: 'Blocks',
      icon: <span style={{ fontSize: '0.75rem', opacity: 0.6 }}>Σ</span>,
      onItemClick: () => {
        capturedRef.current = '';
        setMathModal({
          onSave: (latex) => {
            setMathModal(null);
            if (!latex.trim()) return;
            editor.focus();
            editor.pasteMarkdown(`$${latex}$`);
          },
        });
      },
    });

    // Mermaid generation item (always available)
    aiItems.push({
      title: t('aislash_menu.generate_mermaid_diagram'),
      subtext: t('aislash_menu.create_a_diagram_from_a_description'),
      aliases: ['mermaid', 'diagram', 'chart'],
      group: 'AI',
      icon: <span style={{ fontSize: '0.75rem', opacity: 0.6 }}>📊</span>,
      onItemClick: async () => {
        setLoading(t('aislash_menu.generating_mermaid'));
        try {
          const prompt = capturedRef.current || extractText(editor.document as unknown);
          capturedRef.current = '';
          if (!prompt.trim()) return;
          const result = await abortableInvoke<AiTransformResult>('generate_mermaid', { prompt });
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
          setLoading(null);
        }
      },
    });

    const customGroups = new Set(aiItems.map((i) => i.group));
    return filterSuggestionItems(
      [...aiItems, ...defaultItems.filter((i) => !customGroups.has(i.group))],
      query,
    );
  }, [editor, pagePath, abortableInvoke, t]);

  return (
    <>
      <AILoadingOverlay loading={!!loading} message={loading ? `${loading}...` : undefined} />
      {mathModal && (
        <MathEditorModal
          initialLatex=""
          onSave={mathModal.onSave}
          onCancel={() => setMathModal(null)}
        />
      )}
      <SuggestionMenuController
        triggerCharacter="/"
        getItems={getItems}
        minQueryLength={0}
      />
    </>
  );
}

function extractText(block: any): string {
  let text = '';
  if (block.content) {
    if (typeof block.content === 'string') text = block.content;
    else if (Array.isArray(block.content)) {
      text = block.content.map((c: { text?: string } | string) => {
        if (typeof c === 'string') return c;
        return c?.text || '';
      }).join('');
    }
  }
  if (Array.isArray(block.children) && block.children.length > 0) {
    const childTexts = block.children.map(extractText).filter(Boolean);
    if (childTexts.length) text += '\n' + childTexts.join('\n');
  }
  return text;
}
