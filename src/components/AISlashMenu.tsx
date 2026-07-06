import { useCallback, useEffect, useRef, useState } from 'react';
import { useBlockNoteEditor } from '@blocknote/react';
import {
  getDefaultReactSlashMenuItems,
  SuggestionMenuController,
  type DefaultReactSuggestionItem,
} from '@blocknote/react';
import { filterSuggestionItems } from '@blocknote/core';
import * as api from '../lib/commands';
import type { AiAction } from '../lib/types';
import AILoadingOverlay from './ui/AILoadingOverlay';
import MathEditorModal from './MathEditorModal';

interface Props {
  pagePath: string;
}

function isJournal(path: string) {
  return path.startsWith('journals/');
}

export default function AISlashMenu({ pagePath }: Props) {
  const editor = useBlockNoteEditor();
  const [loading, setLoading] = useState<string | null>(null);
  const [mathModal, setMathModal] = useState<{
    onSave: (latex: string) => void;
  } | null>(null);
  const capturedRef = useRef('');

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
              const { content } = await api.aiTransformBlock(text, action, pagePath);
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
              const { content } = await api.aiTransformBlock(allText, action, pagePath);
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
        makeItem('Rewrite', 'Improve clarity and flow', 'rewrite', 'selection'),
        makeItem('Format Selection', 'Clean up formatting', 'format', 'selection'),
        makeItem('Summarize', 'Condense into key points', 'summarize', 'selection'),
      );
    }

    aiItems.push({
      title: 'Research with Web',
      subtext: 'Search the web and write research notes',
      aliases: ['research', 'search', 'web'],
      group: 'AI',
      icon: <span style={{ fontSize: '0.75rem', opacity: 0.6 }}>🌐</span>,
      onItemClick: async () => {
        setLoading('Researching...');
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
          const result = await api.aiResearch(query);
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
        makeItem('Structure Journal', 'Organize daily notes into sections', 'structure', 'page'),
        makeItem('Format Notes', 'Clean up formatting and markdown', 'format', 'page'),
      );
      // Custom interlink item that searches vault for existing notes
      aiItems.push({
        title: 'Interlink Notes',
        subtext: 'Search vault and add [[wiki-links]] to related notes',
        aliases: ['interlink', 'wikilinks', 'connect'],
        group: 'AI',
        icon: <span style={{ fontSize: '0.75rem', opacity: 0.6 }}>🔗</span>,
        onItemClick: async () => {
          setLoading('Interlinking...');
          try {
            const md = editor.blocksToMarkdownLossy();
            if (!md.trim()) return;
            const { content } = await api.aiInterlinkNotes(md, pagePath);
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
        makeItem('Format & Structure', 'Organize and clean up the page', 'structure', 'page'),
        makeItem('Summarize Page', 'Create a concise page summary', 'summarize', 'page'),
      );
      aiItems.push({
        title: 'Interlink Notes',
        subtext: 'Search vault and add [[wiki-links]] to related notes',
        aliases: ['interlink', 'wikilinks', 'connect'],
        group: 'AI',
        icon: <span style={{ fontSize: '0.75rem', opacity: 0.6 }}>🔗</span>,
        onItemClick: async () => {
          setLoading('Interlinking...');
          try {
            const md = editor.blocksToMarkdownLossy();
            if (!md.trim()) return;
            const { content } = await api.aiInterlinkNotes(md, pagePath);
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
      title: 'Math Equation',
      subtext: 'Insert a LaTeX math equation',
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
      title: 'Generate Mermaid Diagram',
      subtext: 'Create a diagram from a description',
      aliases: ['mermaid', 'diagram', 'chart'],
      group: 'AI',
      icon: <span style={{ fontSize: '0.75rem', opacity: 0.6 }}>📊</span>,
      onItemClick: async () => {
        setLoading('Generating Mermaid...');
        try {
          const prompt = capturedRef.current || extractText(editor.document as unknown);
          capturedRef.current = '';
          if (!prompt.trim()) return;
          const code = await api.generateMermaid(prompt);
          if (code.trim()) {
            const pos = editor.getTextCursorPosition();
            editor.insertBlocks(
              [{ type: 'mermaid' as any, props: { language: 'mermaid' }, content: [{ type: 'text', text: code, styles: {} }] }],
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
  }, [editor, pagePath]);

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
