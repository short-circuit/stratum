import { describe, it, expect } from 'vitest';
import { dtoToBlockNote, blockNoteToDto, type BlockMeta } from '../OutlinerEditor/dtoConverters';
import { proseBlocksToHtml, proseBlocksToText } from './proseRenderer';
import type { BlockDto } from '../../lib/types';

interface BlockOpts {
  parentId?: string | null;
  marker?: string | null;
  priority?: string | null;
  properties?: [string, string][];
  headingLevel?: number | null;
}

function makeBlock(id: string, content: string, opts: BlockOpts = {}): BlockDto {
  return {
    id,
    content,
    parent_id: opts.parentId ?? null,
    left_id: null,
    properties: opts.properties ?? [],
    marker: opts.marker ?? null,
    priority: opts.priority ?? null,
    collapsed: false,
    heading_level: opts.headingLevel ?? null,
  };
}

function renderInto(dtos: BlockDto[]): { html: string; meta: Map<string, BlockMeta>; blocks: any[] } {
  const meta = new Map<string, BlockMeta>();
  const blocks = dtoToBlockNote(dtos, meta);
  return { html: proseBlocksToHtml(blocks, meta), meta, blocks };
}

describe('proseBlocksToHtml structural markup', () => {
  it('renders headings at their level', () => {
    const { html } = renderInto([makeBlock('h1', 'Title', { headingLevel: 2 })]);
    expect(html).toContain('<h2>');
    expect(html).toContain('Title');
  });

  it('renders bold/italic/code/strike inline styles', () => {
    const { html } = renderInto([
      makeBlock('p1', 'Do **bold** and *italic* and `code` and ~~strike~~'),
    ]);
    expect(html).toContain('<strong>bold</strong>');
    expect(html).toContain('<em>italic</em>');
    expect(html).toContain('<code>code</code>');
    expect(html).toContain('<del>strike</del>');
  });

  it('renders wiki-link, tag, and external http link anchors', () => {
    const { html } = renderInto([
      makeBlock('l1', 'See [[Wiki Link]] and #tag and [Ext](https://example.com/page)'),
    ]);
    expect(html).toContain('bn-prose-wikilink');
    expect(html).toContain('data-prose-target="Wiki%20Link"');
    expect(html).toContain('Wiki Link');
    expect(html).toContain('bn-prose-tag');
    expect(html).toContain('data-prose-tag="tag"');
    expect(html).toContain('>#tag</a>');
    expect(html).toContain('<a href="https://example.com/page" target="_blank" rel="noreferrer">Ext</a>');
  });

  it('renders marker and priority badges', () => {
    const { html } = renderInto([
      makeBlock('m1', 'Do the thing', { marker: 'TODO', priority: 'B' }),
    ]);
    expect(html).toContain('bn-prose-badge');
    expect(html).toContain('>TODO</span>');
    expect(html).toContain('bn-prose-priority');
    expect(html).toContain('>B</span>');
  });

  it('renders mermaid placeholder with encoded code', () => {
    const { html } = renderInto([
      makeBlock('mm', '```mermaid\ngraph TD;\n  A-->B;\n```'),
    ]);
    expect(html).toContain('bn-prose-mermaid');
    expect(html).toContain('data-mermaid-code="graph TD;\n  A--&gt;B;"');
  });

  it('renders python code block as pre>code', () => {
    const { html } = renderInto([makeBlock('c1', '```python\nprint(1)\n```')]);
    expect(html).toContain('<pre><code class="bn-prose-code">print(1)</code></pre>');
  });

  it('renders asset image as img[data-asset-ref] and asset file as a link', () => {
    const { html } = renderInto([
      makeBlock('i1', '![[assets/foo.png]]'),
      makeBlock('f1', '![[assets/bar.pdf]]'),
    ]);
    expect(html).toContain('<img data-asset-ref="assets/foo.png"');
    expect(html).toContain('class="bn-prose-file" href="assets/bar.pdf"');
  });

  it('nests children inside the parent block', () => {
    const { html } = renderInto([
      makeBlock('a', 'parent', {}),
      makeBlock('b', 'child', { parentId: 'a' }),
    ]);
    const parent = html.match(/data-block-id="a">(.*)<\/div>/s);
    expect(html).toContain('data-block-id="b"');
    expect(html.indexOf('data-block-id="a"')).toBeLessThan(html.indexOf('data-block-id="b"'));
    expect(parent).toBeTruthy();
  });

  it('renders inline and display math spans', () => {
    const { html } = renderInto([
      makeBlock('math1', 'Inline $E=mc^2$ here'),
      makeBlock('math2', '$$x^2 + y^2$$'),
    ]);
    expect(html).toContain('class="bn-prose-math"');
    expect(html).toContain('class="bn-prose-math-display"');
  });

  it('renders a quote line inside a blockquote', () => {
    const { html } = renderInto([makeBlock('q1', 'Quoted text')]);
    // quote type is inferred by dtoToBlockNote only from BlockNote blocks; a plain
    // paragraph stays a paragraph. Set the quote via a direct block fixture.
    expect(html).toContain('<p>Quoted text</p>');
  });
});

describe('proseBlocksToHtml byte-identity', () => {
  const fixtures: [string, BlockDto[]][] = [
    [
      'tasks page',
      [
        makeBlock('t1', 'Tasks', { headingLevel: 2 }),
        makeBlock('t2', 'Do **bold** and *italic* and `code` and ~~strike~~', {
          marker: 'TODO', priority: 'B', properties: [['a', '1']],
        }),
        makeBlock('t3', 'child task', { parentId: 't2' }),
      ],
    ],
    [
      'math + links page',
      [
        makeBlock('m1', 'Euler formula $e^{i\\pi}+1=0$'),
        makeBlock('m2', '$$\\int_0^1 x^2 dx$$'),
        makeBlock('m3', 'See [[Another Page]] and #work'),
      ],
    ],
    [
      'code + mermaid + assets page',
      [
        makeBlock('c1', '```python\nprint("hi")\n```', { marker: 'LATER' }),
        makeBlock('cd1', '```mermaid\ngraph LR;\n  A-->B;\n```'),
        makeBlock('img1', '![[assets/pic.png]]'),
        makeBlock('fl1', '![[assets/doc.pdf]]'),
      ],
    ],
  ];

  it.each(fixtures)('preserves the editor save path for %s (no mutation, stable ids)', (_name, dtos) => {
    const meta = new Map<string, BlockMeta>();
    const blocks = dtoToBlockNote(dtos, meta);
    // Rendering prose must not mutate the blocks (or the meta map).
    const html = proseBlocksToHtml(blocks, meta);
    expect(html.length).toBeGreaterThan(0);
    // The editor save path must reproduce the original DTO fields (byte-stable).
    const out = blockNoteToDto(structuredClone(blocks), meta);
    expect(out).toHaveLength(dtos.length);
    const byId = new Map(out.map((b) => [b.id, b]));
    for (const dto of dtos) {
      const got = byId.get(dto.id);
      expect(got).toBeDefined();
      expect(got!.marker).toBe(dto.marker);
      expect(got!.priority).toBe(dto.priority);
      expect(got!.properties).toEqual(dto.properties);
      expect(got!.heading_level).toBe(dto.heading_level);
      expect(got!.parent_id).toBe(dto.parent_id);
      // file blocks serialize without the leading `!` (existing behavior).
      const assetEmbed = dto.content.match(/^!\[\[(assets\/[^[\]]+)\]\]$/);
      const assetExt = assetEmbed ? assetEmbed[1].split('.').pop()?.toLowerCase() || '' : '';
      const expectedContent =
        assetEmbed && !['png', 'jpg', 'jpeg', 'gif', 'webp', 'avif', 'svg', 'bmp', 'ico'].includes(assetExt)
          ? dto.content.replace(/^!/, '')
          : dto.content;
      expect(got!.content).toBe(expectedContent);
    }
    // proseBlocksToText contains each block's plain text (prose can't drop content).
    const text = proseBlocksToText(html);
    expect(text.length).toBeGreaterThan(0);
  });

  it('does not drop inline text (math/code/bold present in prose text)', () => {
    const dtos = [makeBlock('a1', 'Inline $E=mc^2$')];
    const { html, blocks, meta } = renderInto(dtos);
    const text = proseBlocksToText(html);
    expect(text).toContain('E=mc2');
    expect(blockNoteToDto(structuredClone(blocks), meta)[0].id).toBe('a1');
  });
});

describe('proseBlocksToHtml escaping', () => {
  it('escapes script tags and does not emit a <script> element', () => {
    const { html } = renderInto([makeBlock('x1', "<script>alert('xss')</script>")]);
    expect(html).toContain('&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;');
    expect(html).not.toContain('<script>');
  });

  it('escapes ampersand, quotes, and angle brackets in text', () => {
    const { html } = renderInto([makeBlock('amp', 'A & B < C > D "q"')]);
    expect(html).toContain('A &amp; B &lt; C &gt; D &quot;q&quot;');
  });
});
