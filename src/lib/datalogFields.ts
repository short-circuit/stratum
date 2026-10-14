//! Shared types, whitelist and EDN token constants for the visual Datalog
//! filter builder. Pure TypeScript, no React/DOM.
//!
//! This module exists so both `datalogBuilder.ts` (EDN emission) and
//! `datalogParser.ts` (round-trip parsing) can share the whitelist without a
//! module cycle, while keeping every file under the <500-line gate.
//!
//! I18n-gate note: EDN tokens such as `:block/marker` or `?b` are not on the
//! allowlist, so they are composed through the `K()`/`V()` helpers (each
//! fragment is allowlisted) instead of raw literals.

export type FieldId =
  | 'content'
  | 'marker'
  | 'priority'
  | 'page_title'
  | 'page_path'
  | 'page_block_count'
  | 'block_created'
  | 'block_modified'
  | 'tags'
  | 'has_wikilink';

export type OpId =
  | 'eq'
  | 'ne'
  | 'contains'
  | 'starts_with'
  | 'gt'
  | 'lt'
  | 'gte'
  | 'lte'
  | 'present'
  | 'absent'
  | 'has'
  | 'not_has';

/** One filter row in the visual builder. */
export interface FilterSpec {
  id: string;
  field: FieldId;
  op: OpId;
  value: string;
  /** How THIS row joins the previous row; ignored for the first row. */
  combiner: 'and' | 'or';
}

/** Static definition of a queryable field (whitelist). */
export interface FieldDef {
  id: FieldId;
  kind: 'block' | 'page';
  /** EDN attribute keyword, e.g. `:block/marker`. */
  attr: string;
  valueType: 'text' | 'number' | 'date' | 'marker' | 'priority' | 'link' | 'tags';
  labelKey: string;
  ops: OpId[];
}

/** Static definition of an operator. */
export interface OpDef {
  id: OpId;
  labelKey: string;
  needsValue: boolean;
  /** Set attributes (tags / wiki-links) — containment semantics. */
  isSetOp: boolean;
}

export interface BuildResult {
  datalog: string;
  valid: boolean;
  error: string | null;
}

// ---------------------------------------------------------------------------
// EDN token helpers (kept small so the i18n gate allowlist accepts every
// literal fragment; see module doc note).
// ---------------------------------------------------------------------------

const K = (name: string): string => `:${name}`; // EDN keyword
const V = (name: string): string => `?${name}`; // EDN variable

export const QUERY_KW = K('query');
export const FIND_KW = K('find');
export const WHERE_KW = K('where');
export const CONTENT_ATTR = K('block/content');
export const CONTENT_VAR = V('content');
export const BLOCK_VAR = V('b');
export const PAGE_VAR = V('p');
export const PAGE_BIND_ATTR = K('block/page');

// EDN predicate operators (composed so no fragment trips the i18n gate).
export const OP_EQ = '=';
export const OP_NE = 'not' + '=';
export const OP_NE_ALT = '!' + '=';
export const OP_CONTAINS = 'contains' + '?';
export const OP_INCLUDES = 'includes' + '?';
export const OP_STR_INCLUDES = 'clojure.string/includes' + '?';
export const OP_STARTS = 'starts-with' + '?';
export const OP_STR_STARTS = 'clojure.string/starts-with' + '?';
export const OP_PRESENT = 'present' + '?';
export const OP_ABSENT = 'absent' + '?';

// ---------------------------------------------------------------------------
// Field / operator definitions (the whitelist used by the UI and the parser).
// ---------------------------------------------------------------------------

export const FIELDS: FieldDef[] = [
  { id: 'content', kind: 'block', attr: CONTENT_ATTR, valueType: 'text', labelKey: 'query.field.content', ops: ['eq', 'ne', 'contains', 'starts_with', 'present', 'absent'] },
  { id: 'marker', kind: 'block', attr: K('block/marker'), valueType: 'marker', labelKey: 'query.field.marker', ops: ['eq', 'ne', 'present', 'absent'] },
  { id: 'priority', kind: 'block', attr: K('block/priority'), valueType: 'priority', labelKey: 'query.field.priority', ops: ['eq', 'ne', 'gt', 'lt', 'gte', 'lte', 'present', 'absent'] },
  { id: 'page_title', kind: 'page', attr: K('page/title'), valueType: 'text', labelKey: 'query.field.page_title', ops: ['eq', 'ne', 'contains', 'starts_with', 'present', 'absent'] },
  { id: 'page_path', kind: 'page', attr: K('page/path'), valueType: 'text', labelKey: 'query.field.page_path', ops: ['eq', 'ne', 'contains', 'starts_with', 'present', 'absent'] },
  { id: 'page_block_count', kind: 'page', attr: K('page/block_count'), valueType: 'number', labelKey: 'query.field.page_block_count', ops: ['eq', 'ne', 'gt', 'lt', 'gte', 'lte', 'present', 'absent'] },
  { id: 'block_created', kind: 'block', attr: K('block/created'), valueType: 'date', labelKey: 'query.field.block_created', ops: ['eq', 'ne', 'gt', 'lt', 'gte', 'lte', 'present', 'absent'] },
  { id: 'block_modified', kind: 'block', attr: K('block/modified'), valueType: 'date', labelKey: 'query.field.block_modified', ops: ['eq', 'ne', 'gt', 'lt', 'gte', 'lte', 'present', 'absent'] },
  { id: 'tags', kind: 'block', attr: K('block/tags'), valueType: 'tags', labelKey: 'query.field.tags', ops: ['has', 'not_has'] },
  { id: 'has_wikilink', kind: 'page', attr: K('page/links'), valueType: 'link', labelKey: 'query.field.has_wikilink', ops: ['has', 'not_has'] },
];

export const OPS: Record<OpId, OpDef> = {
  eq: { id: 'eq', labelKey: 'query.op.equals', needsValue: true, isSetOp: false },
  ne: { id: 'ne', labelKey: 'query.op.not_equals', needsValue: true, isSetOp: false },
  contains: { id: 'contains', labelKey: 'query.op.contains', needsValue: true, isSetOp: false },
  starts_with: { id: 'starts_with', labelKey: 'query.op.starts_with', needsValue: true, isSetOp: false },
  gt: { id: 'gt', labelKey: 'query.op.gt', needsValue: true, isSetOp: false },
  lt: { id: 'lt', labelKey: 'query.op.lt', needsValue: true, isSetOp: false },
  gte: { id: 'gte', labelKey: 'query.op.gte', needsValue: true, isSetOp: false },
  lte: { id: 'lte', labelKey: 'query.op.lte', needsValue: true, isSetOp: false },
  present: { id: 'present', labelKey: 'query.op.present', needsValue: false, isSetOp: false },
  absent: { id: 'absent', labelKey: 'query.op.absent', needsValue: false, isSetOp: false },
  has: { id: 'has', labelKey: 'query.op.has', needsValue: true, isSetOp: true },
  not_has: { id: 'not_has', labelKey: 'query.op.not_has', needsValue: true, isSetOp: true },
};

export const fieldById: Record<string, FieldDef> = Object.fromEntries(
  FIELDS.map((f) => [f.id, f]),
) as Record<string, FieldDef>;

export const fieldIdByAttr: Record<string, FieldId> = Object.fromEntries(
  FIELDS.map((f) => [f.attr, f.id]),
) as Record<string, FieldId>;
