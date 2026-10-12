import { describe, it, expect } from 'vitest';
import { dtoToBlockNote, blockNoteToDto, type BlockMeta } from './dtoConverters';
import type { BlockDto } from '../../lib/types';

function makeBlock(id: string, content: string, parentId: string | null = null): BlockDto {
  return {
    id,
    content,
    parent_id: parentId,
    left_id: null,
    properties: [],
    marker: null,
    priority: null,
    collapsed: false,
    heading_level: null,
  };
}

function dtoRoundTrip(dtos: BlockDto[]): string[] {
  const meta = new Map<string, BlockMeta>();
  const bn = dtoToBlockNote(dtos, meta);
  return blockNoteToDto(bn, meta).map(b => b.content);
}

describe('asset image/file block conversion (E9.F2)', () => {
  it('converts `![[assets/foo.png]]` to an image block with a stratum-asset url', () => {
    const bn = dtoToBlockNote([makeBlock('1', '![[assets/foo.png]]')], new Map());
    const block = bn[0];
    expect(block.type).toBe('image');
    expect(block.props.url).toBe('stratum-asset:assets/foo.png');
    expect(block.props.name).toBe('assets/foo.png');
  });

  it('converts `![[assets/bar.pdf]]` to a file block with a basename', () => {
    const bn = dtoToBlockNote([makeBlock('1', '![[assets/bar.pdf]]')], new Map());
    const block = bn[0];
    expect(block.type).toBe('file');
    expect(block.props.url).toBe('stratum-asset:assets/bar.pdf');
    expect(block.props.name).toBe('bar.pdf');
  });

  it('round-trips an asset image block back to `![[assets/foo.png]]`', () => {
    expect(dtoRoundTrip([makeBlock('1', '![[assets/foo.png]]')])).toEqual(['![[assets/foo.png]]']);
  });

  it('round-trips an asset file block back to `[[assets/bar.pdf]]`', () => {
    expect(dtoRoundTrip([makeBlock('1', '![[assets/bar.pdf]]')])).toEqual(['[[assets/bar.pdf]]']);
  });

  it('leaves a non-asset markdown image `![alt](http...)` as a paragraph that round-trips', () => {
    const src = '![alt](https://example.com/a.png)';
    const meta = new Map<string, BlockMeta>();
    const bn = dtoToBlockNote([makeBlock('1', src)], meta);
    expect(bn[0].type).toBe('paragraph');
    expect(dtoRoundTrip([makeBlock('1', src)])).toEqual([src]);
  });

  it('leaves mermaid and code blocks unchanged', () => {
    const mermaid = '```mermaid\ngraph TD;\n  A-->B;\n```';
    const code = '```python\nprint(1)\n```';
    const meta = new Map<string, BlockMeta>();
    const bn = dtoToBlockNote(
      [makeBlock('1', mermaid), makeBlock('2', code)],
      meta,
    );
    expect(bn.map(b => b.type)).toContain('mermaid');
    expect(bn.map(b => b.type)).toContain('codeBlock');
    expect(dtoRoundTrip([makeBlock('1', mermaid)])).toEqual([mermaid]);
    expect(dtoRoundTrip([makeBlock('2', code)])).toEqual([code]);
  });

  it('preserves children when converting an asset image block', () => {
    const dtos = [
      makeBlock('1', '![[assets/foo.png]]'),
      makeBlock('2', 'child text', '1'),
    ];
    const block = dtoToBlockNote(dtos, new Map())[0];
    expect(block.children.length).toBe(1);
    expect(block.children[0].type).toBe('paragraph');
  });
});
