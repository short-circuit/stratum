import { useMemo, useRef, useState, useCallback } from 'react';
import katex from 'katex';
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
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={onCancel}
    >
      <div
        className="bg-white dark:bg-[var(--secondary-800)] rounded-xl shadow-2xl border border-[var(--secondary-200)] dark:border-[var(--secondary-700)] w-[700px] max-w-[90vw] max-h-[85vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-[var(--secondary-200)] dark:border-[var(--secondary-700)]">
          <h2 className="text-sm font-medium text-[var(--secondary-700)] dark:text-[var(--secondary-300)]">Edit Equation</h2>
          <button
            onClick={onCancel}
            className="text-[var(--secondary-400)] hover:text-[var(--secondary-600)] dark:hover:text-[var(--secondary-300)] text-lg leading-none p-1"
          >
            ✕
          </button>
        </div>

        {/* Symbol palette */}
        <MathSymbolPalette onInsert={insertAtCursor} />

        {/* Body: editor + preview */}
        <div className="flex flex-col gap-3 p-4 overflow-y-auto flex-1">
          {/* Textarea input */}
          <textarea
            ref={(node) => {
              textareaRef.current = node;
              node?.focus();
            }}
            value={latex}
            onChange={(e) => setLatex(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="E = mc^2"
            rows={4}
            className="w-full resize-y font-mono text-sm p-3 rounded-lg border border-[var(--secondary-200)] dark:border-[var(--secondary-700)] bg-[var(--secondary-50)] dark:bg-[var(--secondary-900)] text-[var(--secondary-700)] dark:text-[var(--secondary-300)] placeholder-[var(--secondary-400)] focus:outline-none focus:ring-2 focus:ring-[var(--primary-400)]"
            spellCheck={false}
          />

          {/* Preview */}
          <div className="flex flex-col gap-1">
            <span className="text-[10px] uppercase tracking-wider text-[var(--secondary-400)] font-medium">Preview</span>
            <div
              className="min-h-[60px] p-4 rounded-lg border border-[var(--secondary-200)] dark:border-[var(--secondary-700)] bg-[var(--secondary-50)] dark:bg-[var(--secondary-900)] flex items-center justify-center overflow-x-auto"
              dangerouslySetInnerHTML={{
                __html: html
                  ? html
                  : error
                    ? `<span class="text-red-500 text-sm">${error}</span>`
                    : '<span class="text-[var(--secondary-400)] text-sm">Preview</span>',
              }}
            />
          </div>

          {error && (
            <div className="text-xs text-red-500 bg-red-50 dark:bg-red-900/20 p-2 rounded border border-red-200 dark:border-red-800">
              {error}
            </div>
          )}

          {/* Tips */}
          <div className="text-[10px] text-[var(--secondary-400)] space-y-0.5">
            <div>Ctrl+Enter to save &bull; Esc to cancel</div>
            <div>
              <a
                href="https://katex.org/docs/supported.html"
                target="_blank"
                rel="noopener noreferrer"
                className="underline hover:text-[var(--primary-500)]"
              >
                KaTeX supported functions
              </a>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 px-4 py-3 border-t border-[var(--secondary-200)] dark:border-[var(--secondary-700)]">
          <button
            onClick={onCancel}
            className="px-3 py-1.5 text-xs rounded-md border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] text-[var(--secondary-600)] dark:text-[var(--secondary-300)] hover:bg-[var(--secondary-100)] dark:hover:bg-[var(--secondary-800)] transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={() => onSave(latex)}
            className="px-3 py-1.5 text-xs rounded-md bg-[var(--primary-500)] text-white hover:bg-[var(--primary-600)] transition-colors font-medium"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
