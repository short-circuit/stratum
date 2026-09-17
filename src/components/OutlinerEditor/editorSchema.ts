/**
 * BlockNote editor schema for the Outliner editor.
 *
 * Extracted from OutlinerEditor.shared.tsx during the E6 sizing-gate refactor
 * (§2.2 of .sisyphus/refactoring-plan.md): the Shiki highlighter singleton and
 * the BlockNote schema (code block + mermaid specs) live here so the shared
 * module stays a thin barrel for hook importers.
 */

import { BlockNoteSchema, defaultBlockSpecs, createCodeBlockSpec } from '@blocknote/core';
import { createHighlighter } from 'shiki';
import { createMermaidSpec } from '../MermaidBlock';

// ---------------------------------------------------------------------------
// Shiki highlighter — singleton promise created at module scope
// ---------------------------------------------------------------------------

const highlighterPromise = createHighlighter({
  themes: ['github-dark', 'github-light'],
  langs: [
    'javascript', 'typescript', 'python', 'rust', 'json', 'html', 'css',
    'bash', 'yaml', 'toml', 'sql', 'markdown', 'xml', 'shell', 'diff',
  ],
});

const supportedLanguages = {
  typescript: { name: 'TypeScript', aliases: ['ts', 'typescript'] },
  javascript: { name: 'JavaScript', aliases: ['js', 'javascript'] },
  python:     { name: 'Python',     aliases: ['py', 'python'] },
  rust:       { name: 'Rust',       aliases: ['rs', 'rust'] },
  json:       { name: 'JSON',       aliases: ['json'] },
  html:       { name: 'HTML',       aliases: ['html', 'htm'] },
  css:        { name: 'CSS',        aliases: ['css'] },
  bash:       { name: 'Bash',       aliases: ['bash', 'sh', 'shell'] },
  yaml:       { name: 'YAML',       aliases: ['yaml', 'yml'] },
  toml:       { name: 'TOML',       aliases: ['toml'] },
  sql:        { name: 'SQL',        aliases: ['sql'] },
  markdown:   { name: 'Markdown',   aliases: ['md', 'markdown'] },
  xml:        { name: 'XML',        aliases: ['xml'] },
  diff:       { name: 'Diff',       aliases: ['diff'] },
};

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

export const schema = BlockNoteSchema.create({
  blockSpecs: {
    ...defaultBlockSpecs,
    codeBlock: createCodeBlockSpec({
      defaultLanguage: 'text',
      supportedLanguages,
      createHighlighter: () => highlighterPromise,
    }),
    mermaid: createMermaidSpec(),
  },
});
