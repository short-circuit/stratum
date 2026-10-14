import { describe, it, expect } from 'vitest';
import {
  buildDatalog,
  parseDatalogToRows,
  FIELDS,
  OPS,
  type FilterSpec,
  type FieldId,
  type OpId,
} from './datalogBuilder';

function spec(partial: Partial<FilterSpec> & { field: FieldId; op: OpId }): FilterSpec {
  return { id: partial.id ?? 'r1', value: partial.value ?? '', combiner: partial.combiner ?? 'and', ...partial };
}

/** Strip the repeated auto-injected prefix and return the :where items. */
function whereItems(datalog: string): string {
  return datalog
    .replace(/^\{:query \[:find \?b \?content :where/, '')
    .replace(/\]\}$/, '')
    .trim();
}
describe('buildDatalog — golden EDN', () => {
  it('emits the DEFAULT_QUERY for empty specs (valid)', () => {
    const r = buildDatalog([]);
    expect(r.valid).toBe(true);
    expect(r.error).toBeNull();
    expect(r.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/marker "TODO"] [?b :block/content ?content]]}',
    );
  });

  it('eq on marker → plain triple, content bindings injected', () => {
    const r = buildDatalog([spec({ field: 'marker', op: 'eq', value: 'TODO' })]);
    expect(r.valid).toBe(true);
    expect(r.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/marker "TODO"]]}',
    );
  });

  it('ne on content → bound triple + (not= ...) predicate list', () => {
    const r = buildDatalog([spec({ field: 'content', op: 'ne', value: 'done' })]);
    expect(r.valid).toBe(true);
    expect(r.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/content ?v0] [(not= ?v0 "done")]]}',
    );
  });

  it('contains on content → (clojure.string/includes? ...)', () => {
    const r = buildDatalog([spec({ field: 'content', op: 'contains', value: 'groceries' })]);
    expect(r.valid).toBe(true);
    expect(r.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/content ?v0] [(clojure.string/includes? ?v0 "groceries")]]}',
    );
  });

  it('starts_with on content → (clojure.string/starts-with? ...)', () => {
    const r = buildDatalog([spec({ field: 'content', op: 'starts_with', value: 'meet' })]);
    expect(r.valid).toBe(true);
    expect(r.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/content ?v0] [(clojure.string/starts-with? ?v0 "meet")]]}',
    );
  });

  it('gt on priority → bound triple + (> ...) predicate', () => {
    const r = buildDatalog([spec({ field: 'priority', op: 'gt', value: 'B' })]);
    expect(r.valid).toBe(true);
    expect(r.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/priority ?v0] [(> ?v0 "B")]]}',
    );
  });

  it('gte / lte operators emit >= / <=', () => {
    const g = buildDatalog([spec({ field: 'page_block_count', op: 'gte', value: '3' })]);
    expect(g.valid).toBe(true);
    expect(g.datalog).toContain('[(>= ?v0 "3")]');
    expect(g.datalog).toContain('[?p :page/block_count ?v0]');
    expect(g.datalog).toContain('[?b :block/page ?p]');
    const l = buildDatalog([spec({ field: 'page_block_count', op: 'lte', value: '9' })]);
    expect(l.valid).toBe(true);
    expect(l.datalog).toContain('[(<= ?v0 "9")]');
  });

  it('present → (present? ...) without value; absent → (absent? ...)', () => {
    const p = buildDatalog([spec({ field: 'marker', op: 'present' })]);
    expect(p.valid).toBe(true);
    expect(p.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/marker ?v0] [(present? ?v0)]]}',
    );
    const a = buildDatalog([spec({ field: 'marker', op: 'absent' })]);
    expect(a.valid).toBe(true);
    expect(a.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/marker ?v0] [(absent? ?v0)]]}',
    );
  });

  it('has on tags → containment triple', () => {
    const r = buildDatalog([spec({ field: 'tags', op: 'has', value: 'project' })]);
    expect(r.valid).toBe(true);
    expect(r.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/tags "project"]]}',
    );
  });

  it('not_has on tags → (not [...])', () => {
    const r = buildDatalog([spec({ field: 'tags', op: 'not_has', value: 'project' })]);
    expect(r.valid).toBe(true);
    expect(r.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/content ?content] (not [?b :block/tags "project"])]}',
    );
  });

  it('has_wikilink (page field) → page binding + page/links containment', () => {
    const r = buildDatalog([spec({ field: 'has_wikilink', op: 'has', value: 'my-page' })]);
    expect(r.valid).toBe(true);
    expect(r.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/page ?p] [?p :page/links "my-page"]]}',
    );
  });

  it('not_has wikilink → (not [?p :page/links ...]) with page binding', () => {
    const r = buildDatalog([spec({ field: 'has_wikilink', op: 'not_has', value: 'my-page' })]);
    expect(r.valid).toBe(true);
    expect(r.datalog).toBe(
      '{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/page ?p] (not [?p :page/links "my-page"])]}',
    );
  });
});

describe('buildDatalog — escaping', () => {
  it('escapes double quotes in values', () => {
    const r = buildDatalog([spec({ field: 'content', op: 'eq', value: 'say "hi"' })]);
    expect(r.datalog).toContain('"say \\"hi\\""');
  });

  it('escapes backslashes in values', () => {
    const r = buildDatalog([spec({ field: 'content', op: 'eq', value: 'a\\b' })]);
    expect(r.datalog).toContain('"a\\\\b"');
  });
});

describe('buildDatalog — combiners / or-groups', () => {
  it('groups consecutive or rows into one (or ...)', () => {
    const r = buildDatalog([
      spec({ field: 'marker', op: 'eq', value: 'TODO' }),
      spec({ id: 'r2', field: 'marker', op: 'eq', value: 'DOING', combiner: 'or' }),
    ]);
    expect(r.valid).toBe(true);
    expect(whereItems(r.datalog)).toContain(
      '(or [?b :block/marker "TODO"] [?b :block/marker "DOING"])',
    );
    expect(r.datalog).toContain('(or [?b :block/marker "TODO"] [?b :block/marker "DOING"])');
  });

  it('emits exactly one (or ...) per group', () => {
    const r = buildDatalog([
      spec({ field: 'marker', op: 'eq', value: 'TODO' }),
      spec({ id: 'r2', field: 'marker', op: 'eq', value: 'DOING', combiner: 'or' }),
      spec({ id: 'r3', field: 'marker', op: 'eq', value: 'DONE', combiner: 'or' }),
    ]);
    expect(r.datalog.match(/\(or /g)?.length).toBe(1);
    expect(r.datalog).toContain(
      '(or [?b :block/marker "TODO"] [?b :block/marker "DOING"] [?b :block/marker "DONE"])',
    );
  });

  it('two separate or groups emit two (or ...)', () => {
    const r = buildDatalog([
      spec({ field: 'marker', op: 'eq', value: 'TODO' }),
      spec({ id: 'r2', field: 'priority', op: 'eq', value: 'A', combiner: 'or' }),
      spec({ id: 'r3', field: 'content', op: 'contains', value: 'x', combiner: 'and' }),
      spec({ id: 'r4', field: 'marker', op: 'eq', value: 'DOING', combiner: 'or' }),
    ]);
    expect(r.datalog.match(/\(or /g)?.length).toBe(2);
    expect(r.valid).toBe(true);
  });

  it('an or row with a partner forms a valid group', () => {
    const rr = buildDatalog([
      spec({ field: 'marker', op: 'eq', value: 'TODO' }),
      spec({ id: 'r2', field: 'content', op: 'eq', value: 'x', combiner: 'or' }),
    ]);
    expect(rr.valid).toBe(true);
    expect(rr.datalog).toContain('(or [?b :block/marker "TODO"] [?b :block/content "x"])');
  });

  it('a lone or row (no partner) → valid=false builder_or_needs_two', () => {
    const rr = buildDatalog([spec({ field: 'content', op: 'eq', value: 'x', combiner: 'or' })]);
    expect(rr.valid).toBe(false);
    expect(rr.error).toBe('query.builder_or_needs_two');
  });

  it('not_has splits an or-run (engine cannot nest (not ...) in (or ...))', () => {
    const r = buildDatalog([
      spec({ field: 'marker', op: 'eq', value: 'TODO' }),
      spec({ id: 'r2', field: 'tags', op: 'not_has', value: 'project', combiner: 'or' }),
      spec({ id: 'r3', field: 'marker', op: 'eq', value: 'DOING', combiner: 'or' }),
    ]);
    expect(r.valid).toBe(true);
    expect(r.datalog.match(/\(or /g)?.length).toBe(1);
    expect(r.datalog).toContain(
      '(or [?b :block/marker "TODO"] [?b :block/marker "DOING"])',
    );
    expect(r.datalog).toContain('(not [?b :block/tags "project"])');
  });
});

describe('buildDatalog — page join injection', () => {
  it('injects [?b :block/page ?p] once for page fields (dedupe)', () => {
    const r = buildDatalog([
      spec({ field: 'page_title', op: 'eq', value: 'Home' }),
      spec({ id: 'r2', field: 'page_path', op: 'eq', value: 'x' }),
    ]);
    expect(r.datalog.match(/\[\?b :block\/page \?p\]/g)?.length).toBe(1);
  });

  it('does not inject page binding for block-only fields', () => {
    const r = buildDatalog([spec({ field: 'marker', op: 'eq', value: 'TODO' })]);
    expect(r.datalog).not.toContain(':block/page');
  });
});

describe('buildDatalog — validation', () => {
  it('empty value for value-needing op → valid=false builder_requires_value', () => {
    const r = buildDatalog([spec({ field: 'content', op: 'eq', value: '   ' })]);
    expect(r.valid).toBe(false);
    expect(r.error).toBe('query.builder_requires_value');
  });

  it('unknown field → valid=false defensive', () => {
    const r = buildDatalog([spec({ field: 'bogus' as FieldId, op: 'eq', value: 'x' })]);
    expect(r.valid).toBe(false);
    expect(r.error).toBe('query.builder_invalid_condition');
  });

  it('unknown op → valid=false defensive', () => {
    const r = buildDatalog([spec({ field: 'content', op: 'bogus' as OpId, value: 'x' })]);
    expect(r.valid).toBe(false);
    expect(r.error).toBe('query.builder_invalid_condition');
  });
});

describe('parseDatalogToRows — round trip', () => {
  it('round-trips an eq row', () => {
    const r = buildDatalog([spec({ field: 'marker', op: 'eq', value: 'TODO' })]);
    const parsed = parseDatalogToRows(r.datalog);
    expect(parsed).not.toBeNull();
    expect(parsed?.ok).toBe(true);
    expect(parsed?.specs).toEqual([
      { id: 'p0', field: 'marker', op: 'eq', value: 'TODO', combiner: 'and' },
    ]);
  });

  it('round-trips a ne row', () => {
    const r = buildDatalog([spec({ field: 'content', op: 'ne', value: 'done' })]);
    const parsed = parseDatalogToRows(r.datalog);
    expect(parsed?.specs).toEqual([
      { id: 'p0', field: 'content', op: 'ne', value: 'done', combiner: 'and' },
    ]);
  });

  it('round-trips a contains row', () => {
    const r = buildDatalog([spec({ field: 'content', op: 'contains', value: 'groceries' })]);
    const parsed = parseDatalogToRows(r.datalog);
    expect(parsed?.specs[0]?.op).toBe('contains');
    expect(parsed?.specs[0]?.value).toBe('groceries');
  });

  it('round-trips a gt row', () => {
    const r = buildDatalog([spec({ field: 'priority', op: 'gt', value: 'B' })]);
    const parsed = parseDatalogToRows(r.datalog);
    expect(parsed?.specs[0]?.field).toBe('priority');
    expect(parsed?.specs[0]?.op).toBe('gt');
    expect(parsed?.specs[0]?.value).toBe('B');
  });

  it('round-trips present/absent', () => {
    const r = buildDatalog([spec({ field: 'marker', op: 'present' })]);
    const parsed = parseDatalogToRows(r.datalog);
    expect(parsed?.specs[0]?.op).toBe('present');
    const r2 = buildDatalog([spec({ field: 'marker', op: 'absent' })]);
    expect(parseDatalogToRows(r2.datalog)?.specs[0]?.op).toBe('absent');
  });

  it('round-trips a page-field query (join injected then skipped)', () => {
    const r = buildDatalog([spec({ field: 'has_wikilink', op: 'has', value: 'my-page' })]);
    const parsed = parseDatalogToRows(r.datalog);
    expect(parsed?.specs).toEqual([
      { id: 'p0', field: 'has_wikilink', op: 'has', value: 'my-page', combiner: 'and' },
    ]);
  });

  it('round-trips an or-group with and/or combiners', () => {
    const r = buildDatalog([
      spec({ field: 'marker', op: 'eq', value: 'TODO' }),
      spec({ id: 'r2', field: 'marker', op: 'eq', value: 'DOING', combiner: 'or' }),
    ]);
    const parsed = parseDatalogToRows(r.datalog);
    expect(parsed?.specs?.map((s) => s.combiner)).toEqual(['and', 'or']);
    expect(parsed?.specs?.map((s) => s.value)).toEqual(['TODO', 'DOING']);
  });

  it('round-trips not_has → not_has op', () => {
    const r = buildDatalog([spec({ field: 'tags', op: 'not_has', value: 'project' })]);
    const parsed = parseDatalogToRows(r.datalog);
    expect(parsed?.specs[0]?.op).toBe('not_has');
    expect(parsed?.specs[0]?.value).toBe('project');
  });

  it('round-trips the DEFAULT query (hand-written style)', () => {
    const d = '{:query [:find ?b ?content :where [?b :block/marker "TODO"] [?b :block/content ?content]]}';
    const parsed = parseDatalogToRows(d);
    expect(parsed?.specs).toEqual([
      { id: 'p0', field: 'marker', op: 'eq', value: 'TODO', combiner: 'and' },
    ]);
  });

  it('round-trips a hand-written predicate query with not= :combiners', () => {
    const d = '{:query [:find ?b ?content :where [?b :block/marker ?m] [(not= ?m "DONE")] [?b :block/content ?content]]}';
    const parsed = parseDatalogToRows(d);
    expect(parsed?.specs[0]).toEqual({
      id: 'p0', field: 'marker', op: 'ne', value: 'DONE', combiner: 'and',
    });
  });
});

describe('parseDatalogToRows — rejection', () => {
  it('rejects pull find specs', () => {
    const d = '{:query [:find (pull ?b [*]) :where [?b :block/marker ?m]]}';
    expect(parseDatalogToRows(d)).toBeNull();
  });

  it('rejects unknown attributes', () => {
    const d = '{:query [:find ?b ?content :where [?b :block/unknown "x"]]}';
    expect(parseDatalogToRows(d)).toBeNull();
  });

  it('rejects JSON syntax', () => {
    const d = '{"query": [":find", ["?b"], ":where", [["?b", ":block/marker", "TODO"]]]}';
    expect(parseDatalogToRows(d)).toBeNull();
  });

  it('rejects malformed / empty input', () => {
    expect(parseDatalogToRows('')).toBeNull();
    expect(parseDatalogToRows('{:query []}')).toBeNull();
  });

  it('rejects unknown predicate ops', () => {
    const d = '{:query [:find ?b ?content :where [?b :block/marker ?m] [(bogus? ?m)]]}';
    expect(parseDatalogToRows(d)).toBeNull();
  });
});

describe('metadata', () => {
  it('FIELDS is a whitelist with 10 fields and all have ops', () => {
    expect(FIELDS.length).toBe(10);
    for (const f of FIELDS) {
      expect(f.ops.length).toBeGreaterThan(0);
      expect(OPS[f.ops[0]]).toBeDefined();
    }
  });

  it('OPS metadata marks set ops and value needs', () => {
    expect(OPS.present.needsValue).toBe(false);
    expect(OPS.absent.needsValue).toBe(false);
    expect(OPS.has.needsValue).toBe(true);
    expect(OPS.has.isSetOp).toBe(true);
    expect(OPS.not_has.isSetOp).toBe(true);
    expect(OPS.eq.needsValue).toBe(true);
  });
});
