//! Visual Datalog filter builder — pure TypeScript, no React/DOM.
//!
//! Turns UI filter rows ([`FilterSpec`]) into an EDN Datalog query accepted by
//! the extended pkm-query grammar (triples, predicate lists, `(or ...)` groups
//! and `(not ...)` negation). The round-trip parser lives in
//! `datalogParser.ts` and is re-exported here for API stability.
//!
//! Grammar notes (engine, pkm-query):
//! - Predicate lists: `[(not= ?m "DONE")]`, `[(clojure.string/includes? ?c "x")]`.
//! - `(or <clause>+)` clauses are triples or predicate lists — `(not ...)`
//!   cannot nest inside an `(or ...)` group, so `not_has` rows (which need
//!   negation) are always emitted at the top level and split or-runs.
//! - `(not <triple>)` with exactly one clause.

import {
  FIELDS,
  OPS,
  fieldById,
  CONTENT_ATTR,
  CONTENT_VAR,
  BLOCK_VAR,
  PAGE_VAR,
  PAGE_BIND_ATTR,
  OP_NE,
  OP_STR_INCLUDES,
  OP_STR_STARTS,
  OP_PRESENT,
  OP_ABSENT,
  type FilterSpec,
  type BuildResult,
} from './datalogFields';

export type {
  FieldId,
  OpId,
  FilterSpec,
  FieldDef,
  OpDef,
  BuildResult,
} from './datalogFields';

// ---------------------------------------------------------------------------
// EDN rendering
// ---------------------------------------------------------------------------

/** Escape a string value per EDN rules (backslash and double-quote). */
function ednString(value: string): string {
  return '"' + value.replace(/\\/g, '\\\\').replace(/"/g, '\\"') + '"';
}

function triple(entity: string, attr: string, value: string): string {
  return `[${entity} ${attr} ${value}]`;
}

function predicate(op: string, ...args: string[]): string {
  return `[(${[op, ...args].join(' ')})]`;
}

const MARKER_ATTR = ':' + 'block/marker';

/** The default query used when the builder has no conditions. */
const DEFAULT_QUERY = `{:query [:find ${BLOCK_VAR} ${CONTENT_VAR} :where ${triple(BLOCK_VAR, MARKER_ATTR, ednString('TODO'))} ${triple(BLOCK_VAR, CONTENT_ATTR, CONTENT_VAR)}]}`;

interface Segment {
  kind: 'single' | 'group' | 'not';
  rows: number[];
}

/**
 * Group rows into emittable segments.
 *
 * Consecutive rows joined by `or` form one logical or-run. `not_has` rows
 * cannot be expressed inside an `(or ...)` group by the engine grammar, so
 * every `not_has` row in the run is pulled out as a top-level `(not ...)`
 * clause; the remaining normal rows of the run are still merged into a single
 * `(or ...)` group (>= 2 rows) — the "OR" intent is preserved as best the
 * grammar allows. A lone `or` row (a run of exactly one row whose combiner is
 * `or`) is invalid: the UI never produces it (it treats the first row's
 * combiner as AND), and we report `builder_or_needs_two`.
 */
function segmentRows(specs: FilterSpec[]): Segment[] {
  const segments: Segment[] = [];
  let i = 0;
  while (i < specs.length) {
    let j = i;
    while (j + 1 < specs.length && specs[j + 1].combiner === 'or') {
      j += 1;
    }
    const run: number[] = [];
    for (let k = i; k <= j; k += 1) run.push(k);
    const normal = run.filter((idx) => specs[idx].op !== 'not_has');
    const nots = run.filter((idx) => specs[idx].op === 'not_has');
    if (normal.length >= 2) {
      segments.push({ kind: 'group', rows: normal });
    } else if (normal.length === 1) {
      segments.push({ kind: 'single', rows: normal });
    }
    for (const idx of nots) segments.push({ kind: 'not', rows: [idx] });
    i = j + 1;
  }
  return segments;
}

interface RowClause {
  /** EDN items for this row (one triple, or bind-triple + predicate list). */
  items: string[] | null;
  /** Non-null for `not_has` rows: the negated triple. */
  negated: string | null;
}

function clauseFor(row: FilterSpec, index: number): RowClause | null {
  const field = fieldById[row.field];
  const op = OPS[row.op];
  if (!field || !op) return null;
  if (op.needsValue && row.value.trim() === '') return null;

  const entity = field.kind === 'page' ? PAGE_VAR : BLOCK_VAR;
  const attr = field.attr;
  const bound = '?' + 'v' + String(index);
  const lit = ednString(row.value);

  switch (row.op) {
    case 'eq':
    case 'has':
      return { items: [triple(entity, attr, lit)], negated: null };
    case 'not_has':
      return { items: null, negated: triple(entity, attr, lit) };
    case 'ne':
      return { items: [triple(entity, attr, bound), predicate(OP_NE, bound, lit)], negated: null };
    case 'contains':
      return { items: [triple(entity, attr, bound), predicate(OP_STR_INCLUDES, bound, lit)], negated: null };
    case 'starts_with':
      return { items: [triple(entity, attr, bound), predicate(OP_STR_STARTS, bound, lit)], negated: null };
    case 'gt':
      return { items: [triple(entity, attr, bound), predicate('>', bound, lit)], negated: null };
    case 'lt':
      return { items: [triple(entity, attr, bound), predicate('<', bound, lit)], negated: null };
    case 'gte':
      return { items: [triple(entity, attr, bound), predicate('>=', bound, lit)], negated: null };
    case 'lte':
      return { items: [triple(entity, attr, bound), predicate('<=', bound, lit)], negated: null };
    case 'present':
      return { items: [triple(entity, attr, bound), predicate(OP_PRESENT, bound)], negated: null };
    case 'absent':
      return { items: [triple(entity, attr, bound), predicate(OP_ABSENT, bound)], negated: null };
    default:
      return null;
  }
}

/**
 * Build the EDN query for the given filter rows.
 *
 * The output is always `{:query [:find ?b ?content :where ...]}`: the content
 * binding is auto-injected once, and a `[?b :block/page ?p]` binding is
 * auto-injected once whenever any page field is used (deduplicated). Empty
 * input produces the default example query with `valid: true`.
 */
export function buildDatalog(specs: FilterSpec[]): BuildResult {
  if (specs.length === 0) {
    return { datalog: DEFAULT_QUERY, valid: true, error: null };
  }

  // --- validation ---------------------------------------------------------
  const segments = segmentRows(specs);
  for (const seg of segments) {
    if (seg.kind === 'single' && specs[seg.rows[0]].combiner === 'or') {
      return { datalog: '', valid: false, error: 'query.builder_or_needs_two' };
    }
  }
  for (let i = 0; i < specs.length; i += 1) {
    const field = fieldById[specs[i].field];
    const op = OPS[specs[i].op];
    if (!field || !op) {
      return { datalog: '', valid: false, error: 'query.builder_invalid_condition' };
    }
    if (op.needsValue && specs[i].value.trim() === '') {
      return { datalog: '', valid: false, error: 'query.builder_requires_value' };
    }
  }

  // --- emission -----------------------------------------------------------
  const where: string[] = [triple(BLOCK_VAR, CONTENT_ATTR, CONTENT_VAR)];
  const usesPage = specs.some((s) => fieldById[s.field]?.kind === 'page');
  if (usesPage) where.push(triple(BLOCK_VAR, PAGE_BIND_ATTR, PAGE_VAR));

  for (const seg of segments) {
    if (seg.kind === 'not') {
      const clause = clauseFor(specs[seg.rows[0]], seg.rows[0]);
      if (!clause || clause.negated === null) {
        return { datalog: '', valid: false, error: 'query.builder_invalid_condition' };
      }
      where.push('(not ' + clause.negated + ')');
    } else if (seg.kind === 'group') {
      const items: string[] = [];
      for (const idx of seg.rows) {
        const clause = clauseFor(specs[idx], idx);
        if (!clause || clause.items === null) {
          return { datalog: '', valid: false, error: 'query.builder_invalid_condition' };
        }
        items.push(...clause.items);
      }
      where.push('(or ' + items.join(' ') + ')');
    } else {
      const clause = clauseFor(specs[seg.rows[0]], seg.rows[0]);
      if (!clause || clause.items === null) {
        return { datalog: '', valid: false, error: 'query.builder_invalid_condition' };
      }
      where.push(...clause.items);
    }
  }

  const datalog = `{:query [:find ${BLOCK_VAR} ${CONTENT_VAR} :where ${where.join(' ')}]}`;
  return { datalog, valid: true, error: null };
}

// Re-export the whitelist and the round-trip parser so the documented public
// surface stays stable even though the implementation is split across files.
export { FIELDS, OPS };
export { parseDatalogToRows } from './datalogParser';
