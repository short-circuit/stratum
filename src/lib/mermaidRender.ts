/**
 * Shared Mermaid diagram rendering helper.
 *
 * Extracted from MermaidBlock so both the editor block and the read-mode
 * ProseView hydrate diagrams through one code path. Initialises mermaid once
 * (respecting the active theme) and returns a DOMPurify-sanitised SVG string.
 *
 * @module mermaidRender
 */

import mermaid from 'mermaid';
import DOMPurify from 'dompurify';

let mermaidInitialized = false;

function ensureMermaid() {
  if (mermaidInitialized) return;
  const dark =
    typeof document !== 'undefined' &&
    document.documentElement.classList.contains('dark');
  mermaid.initialize({
    startOnLoad: false,
    theme: dark ? 'dark' : 'default',
    securityLevel: 'strict',
    fontFamily: 'sans-serif',
  });
  mermaidInitialized = true;
}

/**
 * Render a Mermaid diagram source to a sanitised SVG string.
 *
 * @param code - The Mermaid diagram source.
 * @param renderId - Optional DOM id for the rendered diagram. Auto-generated when
 *   omitted (used by callers who render multiple diagrams in one view).
 * @returns A DOMPurify-sanitised SVG markup string.
 * @throws If mermaid fails to parse/render the diagram, with a descriptive message.
 */
export async function renderMermaidSvg(
  code: string,
  renderId?: string,
): Promise<string> {
  ensureMermaid();
  const id = renderId ?? `bm-${Date.now()}`;
  try {
    const { svg } = await mermaid.render(id, code);
    return DOMPurify.sanitize(svg);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    throw new Error(`Mermaid render failed: ${message}`, { cause: err });
  }
}
