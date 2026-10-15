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

function makeMetaBlock(
  id: string,
  content: string,
  opts: Partial<Pick<BlockDto, 'marker' | 'priority' | 'properties' | 'heading_level'>>,
  parentId: string | null = null,
): BlockDto {
  return {
    ...makeBlock(id, content, parentId),
    marker: opts.marker ?? null,
    priority: opts.priority ?? null,
    properties: opts.properties ?? [],
    heading_level: opts.heading_level ?? null,
  };
}

/** Round-trip through dtoToBlockNote → blockNoteToDto and assert fields survive byte-stably. */
function dtoIdentityRoundTrip(dtos: BlockDto[]): BlockDto[] {
  const meta = new Map<string, BlockMeta>();
  const bn = dtoToBlockNote(dtos, meta);
  return blockNoteToDto(structuredClone(bn), meta);
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

describe('block metadata/id preservation (E10.F5 byte-identity)', () => {
  it.each([
    ['mermaid', '```mermaid\ngraph TD;\n  A-->B;\n```'],
    ['codeBlock', '```python\nprint(1)\n```'],
  ])('preserves id + marker + priority + properties for %s blocks', (_type, content) => {
    const dto = makeMetaBlock('abc-123', content, {
      marker: 'TODO',
      priority: 'B',
      properties: [['propa', 'va'], ['propb', 'vb']],
    });
    const meta = new Map<string, BlockMeta>();
    const bn = dtoToBlockNote([dto], meta);
    // The converted block must carry the stable id (no random UUID churn).
    expect(bn[0].id).toBe('abc-123');
    const out = blockNoteToDto(bn, meta)[0];
    expect(out.id).toBe('abc-123');
    expect(out.marker).toBe('TODO');
    expect(out.priority).toBe('B');
    expect(out.properties).toEqual([['propa', 'va'], ['propb', 'vb']]);
    expect(out.content).toBe(content);
  });

  it.each([
    ['image', '![[assets/foo.png]]', '![[assets/foo.png]]'],
    // file blocks intentionally lose the leading `!` on serialize (existing behavior).
    ['file', '![[assets/bar.pdf]]', '[[assets/bar.pdf]]'],
  ])('preserves id + marker + priority + properties for %s blocks', (_type, content, roundTripContent) => {
    const dto = makeMetaBlock('img-file-1', content, {
      marker: 'DOING',
      priority: 'A',
      properties: [['k', 'v']],
    });
    const meta = new Map<string, BlockMeta>();
    const bn = dtoToBlockNote([dto], meta);
    expect(bn[0].id).toBe('img-file-1');
    const out = blockNoteToDto(bn, meta)[0];
    expect(out.id).toBe('img-file-1');
    expect(out.marker).toBe('DOING');
    expect(out.priority).toBe('A');
    expect(out.properties).toEqual([['k', 'v']]);
    expect(out.content).toBe(roundTripContent);
  });

  it('round-trips mermaid/code/image/file blocks through dtoIdentityRoundTrip with identical fields', () => {
    const dtos = [
      makeMetaBlock('m1', '```mermaid\ngraph LR;\n  X-->Y;\n```', {
        marker: 'DONE', priority: 'C', properties: [['p', '1']],
      }),
      makeMetaBlock('c1', '```js\nconst x = 1;\n```', {
        marker: 'LATER', priority: 'A', properties: [],
      }),
      makeMetaBlock('i1', '![[assets/foo.png]]', {
        marker: 'NOW', priority: 'B', properties: [['alt', 'desc']],
      }),
      makeMetaBlock('f1', '![[assets/bar.pdf]]', {
        marker: null, priority: 'A', properties: [['size', '12']],
      }),
    ];
    const out = dtoIdentityRoundTrip(dtos);
    expect(out).toHaveLength(dtos.length);
    const byId = new Map(out.map((b) => [b.id, b]));
    for (const dto of dtos) {
      const got = byId.get(dto.id);
      expect(got).toBeDefined();
      // file blocks serialize without the leading `!` (existing behavior).
      expect(got!.content).toBe(dto.id === 'f1' ? '[[assets/bar.pdf]]' : dto.content);
      expect(got!.marker).toBe(dto.marker);
      expect(got!.priority).toBe(dto.priority);
      expect(got!.properties).toEqual(dto.properties);
      expect(got!.heading_level).toBe(dto.heading_level);
      expect(got!.parent_id).toBe(dto.parent_id);
    }
  });

  it('preserves children + their markers when converting mermaid/code blocks', () => {
    const dtos = [
      makeMetaBlock('m1', '```mermaid\ngraph TD;\n  A-->B;\n```', { marker: 'TODO' }),
      makeMetaBlock('m1c', 'child of mermaid', { marker: 'DONE' }, 'm1'),
    ];
    const meta = new Map<string, BlockMeta>();
    const bn = dtoToBlockNote(dtos, meta);
    expect(bn[0].children.length).toBe(1);
    expect(bn[0].children[0].id).toBe('m1c');
    const out = blockNoteToDto(bn, meta)[0];
    expect(out.id).toBe('m1');
    expect(out.marker).toBe('TODO');
  });
});
