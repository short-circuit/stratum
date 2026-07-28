import type { PartialBlock } from '@blocknote/core';

/**
 * Creates a PartialBlock for a Mermaid diagram block.
 *
 * `'mermaid'` is a custom block type not in BlockNote's default
 * `PartialBlock` discriminated union, so a single safe cast is
 * needed at the boundary.
 */
export function createMermaidBlock(code: string): PartialBlock {
  return {
    type: 'mermaid',
    props: { language: 'mermaid' },
    content: [{ type: 'text' as const, text: code, styles: {} }],
  } as unknown as PartialBlock;
}
