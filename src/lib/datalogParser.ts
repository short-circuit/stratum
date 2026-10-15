//! Best-effort EDN subset scanner + round-trip parser for builder-generated
//! Datalog queries. Pure TypeScript, no React/DOM.
//!
//! Accepts only the EDN subset the visual builder emits (triples, predicate
//! lists, `(or ...)` groups, `(not ...)` negation, whitelisted attributes and
//! operators). Returns `null` for anything else — pull find specs, unknown
//! attributes or operators, JSON syntax, or malformed input.
//!
//! Standalone module (shared whitelist from `datalogFields.ts`) so every file
//! stays under the <500-line gate; `datalogBuilder.ts` re-exports
//! `parseDatalogToRows` for API stability.

import {
  QUERY_KW,
  FIND_KW,
  WHERE_KW,
  CONTENT_ATTR,
  CONTENT_VAR,
  PAGE_BIND_ATTR,
  OP_EQ,
  OP_NE,
  OP_NE_ALT,
  OP_CONTAINS,
  OP_INCLUDES,
  OP_STR_INCLUDES,
  OP_STARTS,
  OP_STR_STARTS,
  OP_PRESENT,
  OP_ABSENT,
  fieldIdByAttr,
  type FieldId,
  type OpId,
  type FilterSpec,
} from './datalogFields';

// ---------------------------------------------------------------------------
// EDN scanner (subset)
// ---------------------------------------------------------------------------

type ParsedItem =
  | { kind: 'triple'; entity: string; attr: string; value: string }
  | { kind: 'pred'; op: string; args: string[] }
  | { kind: 'or'; items: ParsedItem[] }
  | { kind: 'not'; item: ParsedItem }
  | null;

interface Tok {
  kind: 'delim' | 'str' | 'kw' | 'sym';
  text: string;
}

interface Node {
  kind: 'str' | 'kw' | 'sym' | 'vec';
  text: string;
  kids?: Node[];
}

interface ScanResult {
  nodes: Node[];
  next: number;
}

const SYMBOL_CHARS = /[A-Za-z0-9?_\-./!=<>]/;

function tokenize(input: string): Tok[] | null {
  const toks: Tok[] = [];
  let i = 0;
  while (i < input.length) {
    const c = input[i];
    if (c === '{' || c === '}' || c === '[' || c === ']' || c === '(' || c === ')') {
      toks.push({ kind: 'delim', text: c });
      i += 1;
    } else if (c === '"') {
      i += 1;
      let s = '';
      while (i < input.length && input[i] !== '"') {
        if (input[i] === '\\' && i + 1 < input.length) {
          s += input[i + 1];
          i += 2;
        } else {
          s += input[i];
          i += 1;
        }
      }
      if (i >= input.length) return null; // unterminated string
      i += 1;
      toks.push({ kind: 'str', text: s });
    } else if (c === ':') {
      i += 1;
      let s = '';
      while (i < input.length && SYMBOL_CHARS.test(input[i]) && input[i] !== '?') {
        s += input[i];
        i += 1;
      }
      if (!s) return null;
      toks.push({ kind: 'kw', text: s });
    } else if (c === ';') {
      while (i < input.length && input[i] !== '\n') i += 1;
    } else if (c.trim() === '') {
      i += 1;
    } else if (SYMBOL_CHARS.test(c)) {
      let s = '';
      while (i < input.length && SYMBOL_CHARS.test(input[i])) {
        s += input[i];
        i += 1;
      }
      toks.push({ kind: 'sym', text: s });
    } else {
      return null; // unsupported character (guards JSON syntax)
    }
  }
  return toks;
}

function scan(toks: Tok[], from: number): ScanResult | null {
  const nodes: Node[] = [];
  let i = from;
  while (i < toks.length) {
    const t = toks[i];
    if (t.kind === 'delim') {
      if (t.text === '[' || t.text === '(') {
        const inner = scan(toks, i + 1);
        if (!inner) return null;
        nodes.push({ kind: 'vec', text: '', kids: inner.nodes });
        i = inner.next;
      } else if (t.text === ']' || t.text === ')') {
        return { nodes, next: i + 1 };
      } else {
        // Map braces `{` / `}` are structural only — skip them.
        i += 1;
      }
    } else if (t.kind === 'kw') {
      nodes.push({ kind: 'kw', text: ':' + t.text });
      i += 1;
    } else if (t.kind === 'str') {
      nodes.push({ kind: 'str', text: t.text });
      i += 1;
    } else {
      nodes.push({ kind: 'sym', text: t.text });
      i += 1;
    }
  }
  return { nodes, next: i };
}

function nodeText(node: Node): string | null {
  if (node.kind === 'kw' || node.kind === 'str' || node.kind === 'sym') return node.text;
  return null;
}

function parseQueryItems(datalog: string): { where: Node[] } | null {
  const toks = tokenize(datalog);
  if (!toks) return null;
  const root = scan(toks, 0);
  if (!root || root.next !== toks.length) return null;

  // `{:query [...]}` — the brace delimiters are stripped by the scanner, so
  // the query keyword and its vector appear as siblings at the top level.
  let queryVec: Node | null = null;
  for (let i = 0; i < root.nodes.length - 1; i += 1) {
    if (
      root.nodes[i].kind === 'kw' &&
      root.nodes[i].text === QUERY_KW &&
      root.nodes[i + 1].kind === 'vec'
    ) {
      queryVec = root.nodes[i + 1];
      break;
    }
  }
  if (!queryVec || !queryVec.kids) return null;

  const kids = queryVec.kids;
  const where: Node[] = [];
  let foundFind = false;
  for (let i = 0; i < kids.length; i += 1) {
    const k = kids[i];
    if (k.kind !== 'kw') return null;
    if (k.text === FIND_KW) {
      let j = i + 1;
      let count = 0;
      while (j < kids.length && kids[j].kind !== 'kw') {
        // Find vars must be bare symbols; a `(pull ...)` vector is rejected.
        if (kids[j].kind !== 'sym' || !kids[j].text.startsWith('?')) return null;
        count += 1;
        j += 1;
      }
      if (count === 0) return null;
      foundFind = true;
      i = j - 1;
    } else if (k.text === WHERE_KW) {
      let j = i + 1;
      while (j < kids.length && kids[j].kind !== 'kw') {
        if (kids[j].kind !== 'vec') return null;
        where.push(kids[j]);
        j += 1;
      }
      i = j - 1;
    } else if (k.text === ':' + 'order-by' || k.text === ':' + 'in') {
      if (i + 1 < kids.length) i += 1;
    } else {
      return null; // unknown clause
    }
  }
  if (!foundFind) return null;
  return { where };
}

function parsePredicateNode(node: Node): { op: string; args: string[] } | null {
  if (node.kind !== 'vec' || !node.kids || node.kids.length === 0) return null;
  const op = nodeText(node.kids[0]);
  if (!op || op.startsWith('?') || op.startsWith(':')) return null;
  const args: string[] = [];
  for (let j = 1; j < node.kids.length; j += 1) {
    const a = nodeText(node.kids[j]);
    if (a === null) return null;
    args.push(a);
  }
  return { op, args };
}

function parseWhereItem(node: Node): ParsedItem {
  if (node.kind !== 'vec' || !node.kids) return null;
  const kids = node.kids;
  if (kids[0]?.kind === 'sym' && kids[0].text === 'or') {
    const items: ParsedItem[] = [];
    for (const sub of kids.slice(1)) {
      const parsed = parseWhereItem(sub);
      if (!parsed || parsed.kind === 'or' || parsed.kind === 'not') return null;
      items.push(parsed);
    }
    return { kind: 'or', items };
  }
  if (kids[0]?.kind === 'sym' && kids[0].text === 'not') {
    if (kids.length !== 2) return null;
    const inner = parseWhereItem(kids[1]);
    if (!inner || inner.kind === 'or' || inner.kind === 'not') return null;
    return { kind: 'not', item: inner };
  }
  if (kids[0]?.kind === 'sym' && kids[0].text.startsWith('?') && kids.length === 3) {
    const attr = nodeText(kids[1]);
    const value = nodeText(kids[2]);
    if (!attr || !attr.startsWith(':') || value === null) return null;
    return { kind: 'triple', entity: kids[0].text, attr, value };
  }
  if (kids[0]?.kind === 'vec') {
    // Predicate list: `[(not= ?m "DONE")]` / `[(clojure.string/includes? ?c "x")]`.
    if (kids.length !== 1) return null;
    const pred = parsePredicateNode(kids[0]);
    if (!pred) return null;
    return { kind: 'pred', op: pred.op, args: pred.args };
  }
  return null;
}

function opFromEdn(op: string): OpId | null {
  if (op === OP_EQ) return 'eq';
  if (op === OP_NE || op === OP_NE_ALT) return 'ne';
  if (op === '>') return 'gt';
  if (op === '<') return 'lt';
  if (op === '>=') return 'gte';
  if (op === '<=') return 'lte';
  if (op === OP_CONTAINS || op === OP_INCLUDES || op === OP_STR_INCLUDES) return 'contains';
  if (op === OP_STARTS || op === OP_STR_STARTS) return 'starts_with';
  if (op === OP_PRESENT) return 'present';
  if (op === OP_ABSENT) return 'absent';
  return null;
}

interface ParserState {
  counter: number;
  specs: FilterSpec[];
}

/**
 * Convert parsed where items into filter rows. `groupMode` marks items inside
 * an `(or ...)` group: the first row of the group gets `and`, all following
 * rows get `or`. Unmatched bind-variables (a triple whose var is never used in
 * a predicate) are dropped.
 */
function itemsToSpecs(items: ParsedItem[], state: ParserState, groupMode: boolean): boolean {
  let pending: { field: FieldId; varName: string } | null = null;
  let firstInGroup = true;

  const pushRow = (field: FieldId, op: OpId, value: string): void => {
    const combiner = groupMode && !firstInGroup ? 'or' : 'and';
    firstInGroup = false;
    state.specs.push({
      id: 'p' + String(state.counter),
      field,
      op,
      value,
      combiner,
    });
    state.counter += 1;
  };

  for (const item of items) {
    if (!item) return false;
    if (item.kind === 'pred') {
      const opId = opFromEdn(item.op);
      if (!opId || !pending || item.args[0] !== pending.varName) return false;
      pushRow(pending.field, opId, item.args[1] ?? '');
      pending = null;
    } else if (item.kind === 'triple') {
      if (item.attr === PAGE_BIND_ATTR && item.value.startsWith('?')) continue; // injected page binding
      if (item.attr === CONTENT_ATTR && item.value === CONTENT_VAR && item.entity.startsWith('?')) continue; // injected content binding
      const field = fieldIdByAttr[item.attr];
      if (!field) return false;
      if (item.value.startsWith('?')) {
        pending = { field, varName: item.value };
      } else {
        pending = null;
        pushRow(field, field === 'tags' || field === 'has_wikilink' ? 'has' : 'eq', item.value);
      }
    } else if (item.kind === 'or') {
      pending = null;
      if (!itemsToSpecs(item.items, state, true)) return false;
      firstInGroup = false;
    } else {
      // (not <triple>): top-level negation.
      pending = null;
      const inner = item.item;
      if (!inner || inner.kind !== 'triple') return false;
      const field = fieldIdByAttr[inner.attr];
      if (!field) return false;
      pushRow(field, field === 'tags' || field === 'has_wikilink' ? 'not_has' : 'ne', inner.value);
    }
  }
  return true;
}

/**
 * Best-effort round-trip of builder-generated (or hand-written equivalent)
 * queries back into filter rows.
 *
 * Accepts only the EDN subset the builder emits (triples, predicate lists,
 * `(or ...)` groups, `(not ...)` negation, whitelisted attributes/operators).
 * Returns `null` for anything else — pull find specs, unknown attributes or
 * operators, JSON syntax, or malformed input.
 */
export function parseDatalogToRows(datalog: string): { specs: FilterSpec[]; ok: boolean } | null {
  const parsed = parseQueryItems(datalog);
  if (!parsed) return null;
  const state: ParserState = { counter: 0, specs: [] };
  const items: ParsedItem[] = [];
  for (const node of parsed.where) {
    const item = parseWhereItem(node);
    if (!item) return null;
    items.push(item);
  }
  if (!itemsToSpecs(items, state, false)) return null;
  return { specs: state.specs, ok: true };
}
