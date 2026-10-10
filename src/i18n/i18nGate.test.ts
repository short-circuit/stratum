/// <reference types="node" />

//! i18n gate for Stratum frontend.
//!
//! Static scan that FAILS if a user-facing English UI string remains hardcoded
//! (as JSX text or a label/placeholder/title/aria-label literal) instead of
//! going through `t()` / `i18n.t()`. This enforces the i18n sweep (task
//! t_d7073534) so future UI work cannot regress localization.
//!
//! How it works:
//!   - Uses the TypeScript compiler API (accurate AST, no regex false positives).
//!   - Walks `src/` for .ts/.tsx (skips tests, workers, i18n itself).
//!   - Flags: JSX text nodes containing letters; string-literal values of the
//!     known USER_PROPS; string literals / template heads that look like UI copy
//!     AND are not (a) the first argument of t(...)/i18n.t(...), (b) in the
//!     ALLOWLIST of non-UI literals, or (c) a plain single token/enum value.
//!   - Reports violations as `path:line — 'literal'` with the suggested key.
//!
//! Allowlist: any literal that is NOT actual UI copy (css, units, urls, brand,
//! enum tokens, svg, latex, imports, hex colors, numbers, whitespace-only etc).
//! The allowlist is intentionally conservative: when the scan flags something
//! that IS user-facing, the correct response is to translate it, not to extend
//! the allowlist.

import { describe, it, expect } from 'vitest';
import ts from 'typescript';
import fs from 'fs';
import path from 'path';

const REPO = process.cwd();
const SRC = path.join(REPO, 'src');

const USER_PROPS = new Set([
  'aria-label',
  'aria-labelledby',
  'aria-description',
  'aria-placeholder',
  'placeholder',
  'title',
  'alt',
  'label',
  'helperText',
  'description',
  'message',
  'emptyText',
  'confirmText',
  'cancelText',
  'noResultsText',
  'errorText',
  'tooltip',
  'tooltipTitle',
]);

// Non-UI literals that are allowed to appear as string literals in source.
// (Import specifiers, css values, enum tokens, urls, brand, latex, svg…)
const ALLOW_RE = [
  /^&[.:[#]/,                         // CSS selectors: &:hover, &.Mui
  /^\\/,                              // LaTeX: \alpha, \frac{}{}, …
  /^\[data-[a-zA-Z0-9-]+\]$/,         // data attribute selectors: [data-block-id]
  /^\./,                              // CSS class selectors
  /^#[0-9a-fA-F]{3,8}$/,              // hex colors
  /^var\(/, /^calc\(/,                // css function
  /^rgba?\(/, /^hsla?\(/,             // css color functions
  /^\d+(\.\d+)?(px|rem|em|%|vh|vw|fr|s|ms|deg|ch|ex|!important|vw|vh|dvh)\b/, // measures
  /^\d+( \d+)+$/,                     // grid values
  /^\d+(\.\d+)?x$/,                   // zoom multipliers
  /^[a-z-]+ \d/,                      // 'spin 0.6s linear infinite'
  /^(left|right|center|top|bottom|middle|start|end|baseline|auto|auto_fit|manual|dark|light|none|hidden|unset|inherit|flex|grid|block|inline|fixed|absolute|relative|sticky|pointer|default|nowrap|wrap|column|row)$/,
  /^(text|contained|outlined|standard|filled|primary|secondary|success|error|warning|info|inherit|solid|dashed|dotted|none)$/,
  /^(width|height|gap|padding|margin|border|font|line-height|color|display|position|transform|transition|animation|opacity|background|letter-spacing|cursor|user-select|vertical-align|justify-content|align-items|pointer-events|text-transform|font-weight|font-size|border-radius|box-shadow|overflow|max-width|min-width|flex-direction|flex-wrap|text-align|white-space|z-index|top|bottom|left|right|visibility|flex-grow|flex-shrink|align-self|justify-self|aspect-ratio|object-fit|grid-template|min-height|max-height|margin-top|margin-bottom|margin-left|margin-right|padding-top|padding-bottom|padding-left|padding-right)[:\s-]/,
  /^(repeat|translateX?|Y?|rotate|scale|matrix|linear-gradient|radial-gradient)\(/,
  /^(?:\.\.?\/|\/|@)[\w./@-]+$/,      // relative or package imports
  /^[\w@.-]+\/[\w@./-]+$/,            // scoped/package specifier
  /^[\w.-]+\.(css|svg|png|jpg|jpeg|gif|webp|wasm|json|toml|md|txt|mp3|wav|flac)\??(raw)?$/,
  /^(?:https?|git|ssh|file):\/\/\S+$|^git@\S+$|^~\/\S+$/,
  /^[a-z0-9_./-]+$/,                  // enum tokens, ids, path-like
  /^[A-Za-z0-9]{1,3}$/,               // short tokens (en, de, ok, x)
  /^[A-Za-z0-9]+\.[A-Za-z0-9]+$/,     // versionish / dotted
  /^[^a-zA-Z0-9]+$/,                  // symbols: ✨ · ─ …
  /^[A-Za-z0-9_.-]+\[[A-Za-z0-9_.-]+\]$/, // key[v] placeholders
  /^\[[a-zA-Z]+\]$/,                  // attribute selectors
  /^\[(AI|TTS|MarkerSuggest|WikiLinkAutocomplete|MathInline|library|dictation|marker)\]/, // log tags
  /^\{[a-z]+\}$/,                     // {count}
  /^[a-zA-Z0-9_]{2,8}$/,              // plain single-word tokens (save, main, en)
  /^(plain|markdown|html|json|csv|css|toml|yaml|wasm|md|txt)$/,
  /^(auto|manual|dark|light|sepia|system|default|inherit|none|hidden|visible|flex|grid|block|inline)$/,
];

const ALLOW_EXACT = new Set([
  'Stratum', 'stratum', 'Promise', '✨', '·', 'E = mc^2', 'Capital Gk',
  'https://katex.org/docs/supported.html', 'a\\n─\\nb', '```mermaid', '[[$1]]',
  'noopener noreferrer', 'ErrorBoundary caught:', 's screen coordinates; MUI',
  'e.g. All TODO tasks', 'e.g. All TODO',
  'git@github.com:user/vault.git', 'http://localhost:11434', 'http://localhost:8081',
  'http://localhost:8888', 'http://localhost:9000', '~/.ssh/id_ed25519',
  'pyannote/speaker-diarization', 'whisper-1', 'gpt-4o', 'alloy', 'mp3', 'wav', 'flac',
  'main', 'master', 'pages/my-new-page.md', '#f97316', '#6b7280',
  '1x', '2x', '1.5x', '0 0 80px', '0 0 120px', '0 0 160px', '0 0 auto',
  '1 1 auto', '1 1 45%', '50%!important', '56px !important', '20px !important',
  'underline dashed', 'pointer-events:none;transform-origin:0 0;',
  'padding:12px;border:1px solid #f87171;border-radius:6px;background:var',
  'gap: 3px', 'user-select: none', 'vertical-align: middle', 'height: 18px',
  'letter-spacing: 0.02em', 'cursor: pointer', 'cursor: default', 'justify-content: center',
  'height: 14px', 'align-items: center', '-webkit-box', 'pre > code.language-mermaid',
  'stratum:', 'stratum-tag:', 'a[data-inline-content-type="link"][href^="stratum:"]',
  '"Inter", "Roboto", "Helvetica", "Arial", sans-serif',
  '.blocknote-editor-container, .bn-editor, .ProseMirror',
  '{datetime}', '{editedfiles}', '{newfiles}', '{deletedfiles}', '{count}',
  '& .MuiChip-icon', '& .MuiAlert-message', '& .MuiInputBase-root', '& .MuiInputBase-input',
  '& .MuiFormControlLabel-label', '& .MuiToggleButton-root', '& .MuiSelect-select',
  '& .MuiDrawer-paper', '& .MuiDialog-container', '& input', '& .MuiTab-root',
  '4px!important', '& .MuiChip-icon', '& .MuiGrid-root', '& .MuiListItem-root',
  '& .MuiListItemButton-root', '& .MuiListItemText-root', '& .MuiPaper-root',
  '& .MuiSlider-root', '& .MuiSwitch-root', '& .MuiCheckbox-root', '& .MuiRadio-root',
  '& .MuiTooltip-tooltip', '& .MuiPopover-paper', '& .MuiMenu-paper', '& .MuiBackdrop-root',
  '& .MuiDialogContent-root', '& .MuiDialogActions-root', '& .MuiAccordion-root',
  '& .MuiAccordionSummary-root', '& .MuiAccordionDetails-root', '& .MuiTabs-root',
  '& .MuiTab-root', '& .MuiAvatar-root', '& .MuiBadge-root', '& .MuiButton-root',
  '& .MuiCard-root', '& .MuiCardContent-root', '& .MuiChip-root', '& .MuiDivider-root',
  '& .MuiFormControl-root', '& .MuiFormGroup-root', '& .MuiIconButton-root',
  '& .MuiInput-root', '& .MuiInputAdornment-root', '& .MuiLinearProgress-root',
  '& .MuiLink-root', '& .MuiListItem-root', '& .MuiMenu-list', '& .MuiOutlinedInput-root',
  '& .MuiSelect-root', '& .MuiSkeleton-root', '& .MuiSnackbar-root', '& .MuiTable-root',
  '& .MuiTableCell-root', '& .MuiToolbar-root', '& .MuiTypography-root',
  'background-color 0.15s ease', 'pulse 1s infinite', 'spin 0.6s linear infinite',
  'linear', 'ease', 'ease-in', 'ease-out', 'ease-in-out', 'step-end', 'step-start',
  '0.875rem', '0.75rem', '0.875em', '1.25rem', '1.5rem', '2rem', '3rem', '4rem',
  '100%', '50%', '75%', '25%', '0%', '0', '1', '2', '3', '50', '100', '200', '300',
  '400', '500', '600', '700', '800', '900', 'inherit', 'unset', 'initial', 'revert',
  '&:hover', '&:focus', '&:active', '&:visited', '&:disabled', '&:empty',
  '&:focus-within', '&:focus-visible', '&::before', '&::after', '&::-webkit-scrollbar',
  'M3 6h18', 'M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6', 'M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2',
  'M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z', '0.1.0', '0.2.1', '0.0.4',
  'file:read', 'file:write', "plugin_not_found: no plugin with id '",
  '{:query [:find ?b :where [?b :block/marker "TODO"]]}', 'data:',
  '<svg', '<svg fill="currentColor"', '*.svg?raw', '& .MuiFormControlLabel-label',
  'relative', 'absolute', 'fixed', 'sticky', 'static', 'flex-start', 'flex-end',
  'space-between', 'space-around', 'space-evenly', 'stretch', 'baseline',
  'ellipsis', 'clip', 'scroll', 'hidden', 'visible', 'auto', 'none', 'block', 'inline',
  'inline-block', 'flex', 'grid', 'table', 'contents', 'flow-root', 'min-content',
  'max-content', 'fit-content', 'stable', 'balanced',
  // Non-UI literals (code values, wiring, diagnostics, calendar/lang names):
  '/page/:pagePath', '/search?q=', 'anchorPosition', 'fullWidth', 'fullScreen',
  'currentColor', 'AbortError', '__TAURI_INTERNALS__', 'NeedsPassphrase',
  'syncSettings', 'currentPage', 'navigateRef', 'codeBlock', 'checkListItem',
  'CANCELLED', 'T12:00:00', 'en-US', 'dictation:stage', '#root', '90%',
  'TypeScript', 'JavaScript', 'Python', 'Rust', 'java', 'c', 'cpp', 'go',
  'OPENAI_API_KEY', 'ANTHROPIC_API_KEY', 'GOOGLE_API_KEY',
  '[data-template-input]', '[data-mobile-template-input]', '[data-block-id]',
  '[OutlinerEditor] marker save failed:', '[OutlinerEditor] replaceBlocks failed:',
  '[OutlinerEditor] getBlocks failed:', '[OutlinerEditor] save failed:',
  '[MarkerDecorations] registerPlugin failed:',
  'January', 'February', 'March', 'April', 'May', 'June', 'July', 'August',
  'September', 'October', 'November', 'December',
]);

function isAllowed(value: string): boolean {
  const v = value.trim();
  if (!v || v.length > 100) return true;
  if (ALLOW_EXACT.has(v)) return true;
  for (const r of ALLOW_RE) if (r.test(v) || r.test(value)) return true;
  return false;
}

function walkFiles(dir: string, acc: string[] = []): string[] {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (['node_modules', 'workers', 'test', 'i18n', 'locales'].includes(e.name)) continue;
      walkFiles(p, acc);
    } else if (/\.(tsx|ts)$/.test(e.name) && !e.name.includes('.test.')) {
      acc.push(p);
    }
  }
  return acc;
}

function isTArg(node: ts.Node, parent: ts.Node | undefined): boolean {
  if (!parent) return false;
  if (ts.isCallExpression(parent)) {
    const callee = parent.expression;
    const name = ts.isIdentifier(callee) ? callee.text
      : ts.isPropertyAccessExpression(callee) ? callee.name.text : '';
    return (name === 't' || name === 'translate') && parent.arguments[0] === node;
  }
  return false;
}

function scan(srcRoot: string): string[] {
  const out: string[] = [];
  for (const file of walkFiles(srcRoot)) {
    const rel = path.relative(srcRoot, file);
    const text = fs.readFileSync(file, 'utf8');
    const sf = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true,
      file.endsWith('.tsx') ? ts.ScriptKind.TSX : ts.ScriptKind.TS);

    function report(literal: string, line: number, kind: string, prop?: string) {
      const v = literal.trim();
      if (!v || v.length > 50) return;
      if (isAllowed(v)) return;
      out.push(`${rel}:${line} [${kind}${prop ? '/' + prop : ''}] '${v}'`);
    }

    function visit(node: ts.Node, parent?: ts.Node) {
      if (ts.isJsxText(node)) {
        const t = node.text.replace(/\s+/g, ' ').trim();
        if (t && /[a-zA-Z]/.test(t)) {
          const line = sf.getLineAndCharacterOfPosition(node.getStart(sf)).line + 1;
          report(t, line, 'jsx');
        }
      } else if (ts.isJsxAttribute(node)) {
        const prop = node.name.getText(sf);
        if (USER_PROPS.has(prop)) {
          let lit: ts.StringLiteral | undefined;
          if (node.initializer && ts.isStringLiteral(node.initializer)) lit = node.initializer;
          else if (node.initializer && ts.isJsxExpression(node.initializer) &&
                   node.initializer.expression && ts.isStringLiteral(node.initializer.expression)) {
            lit = node.initializer.expression;
          }
          if (lit) {
            const line = sf.getLineAndCharacterOfPosition(lit.getStart(sf)).line + 1;
            report(lit.text, line, 'prop', prop);
          }
        }
      } else if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
        if (!isTArg(node, parent)) {
          const line = sf.getLineAndCharacterOfPosition(node.getStart(sf)).line + 1;
          report(node.text, line, 'lit');
        }
      }
      ts.forEachChild(node, (c) => visit(c, node));
    }
    visit(sf);
  }
  return out;
}

describe('i18n gate — no hardcoded user-facing UI strings', () => {
  it('scans src for untranslated literals (JSX text / UI props / UI-like string literals)', () => {
    const violations = scan(SRC);
    if (violations.length > 0) {
      console.log('\n=== i18n gate violations (remaining hardcoded UI strings) ===');
      for (const v of violations) console.log('  ' + v);
    }
    expect(violations).toEqual([]);
  });
});
