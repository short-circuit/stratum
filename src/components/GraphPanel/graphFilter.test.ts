import { describe, it, expect } from 'vitest';
import {
  LARGE_VAULT_THRESHOLD,
  DEFAULT_LARGE_VAULT_CAP,
  effectiveNodeCap,
  capNodesByDegree,
  edgesWithin,
} from './graphFilter';

describe('graphFilter', () => {
  describe('effectiveNodeCap', () => {
    it('returns explicit user cap when set, at any vault size', () => {
      expect(effectiveNodeCap(500, 10)).toBe(500);
      expect(effectiveNodeCap(500, 10000)).toBe(500);
      expect(effectiveNodeCap(10000, 10000)).toBe(10000);
    });

    it('returns 0 (no cap) for normal vaults with unlimited setting', () => {
      expect(effectiveNodeCap(0, 0)).toBe(0);
      expect(effectiveNodeCap(0, 5)).toBe(0);
      expect(effectiveNodeCap(0, LARGE_VAULT_THRESHOLD)).toBe(0); // not strictly above
    });

    it('applies the safe default cap only above the large-vault threshold when unlimited', () => {
      expect(effectiveNodeCap(0, LARGE_VAULT_THRESHOLD + 1)).toBe(DEFAULT_LARGE_VAULT_CAP);
      expect(effectiveNodeCap(0, 10000)).toBe(DEFAULT_LARGE_VAULT_CAP);
      expect(DEFAULT_LARGE_VAULT_CAP).toBeLessThanOrEqual(LARGE_VAULT_THRESHOLD);
    });

    it('never caps to zero', () => {
      expect(effectiveNodeCap(0, 1)).toBe(0);
      expect(effectiveNodeCap(0, LARGE_VAULT_THRESHOLD + 1)).toBeGreaterThan(0);
    });
  });

  describe('capNodesByDegree', () => {
    const nodes = [
      { id: 'a', degree: 1 },
      { id: 'b', degree: 5 },
      { id: 'c', degree: 3 },
      { id: 'd', degree: 0 },
      { id: 'e', degree: 2 },
    ];

    it('returns nodes unchanged when cap is 0, negative, or >= length', () => {
      expect(capNodesByDegree(nodes, 0)).toBe(nodes);
      expect(capNodesByDegree(nodes, -1)).toBe(nodes);
      expect(capNodesByDegree(nodes, nodes.length)).toBe(nodes);
      expect(capNodesByDegree(nodes, 999)).toBe(nodes);
    });

    it('keeps only the highest-degree nodes up to cap', () => {
      const capped = capNodesByDegree(nodes, 2);
      expect(capped.map((n) => n.id).sort()).toEqual(['b', 'c']);
    });

    it('preserves original source order in the kept subset', () => {
      const capped = capNodesByDegree(nodes, 3);
      const kept = capped.map((n) => n.id);
      // b, c, e are the top-3 (degrees 5,3,2) — and original order is b,c,e
      expect(kept).toEqual(['b', 'c', 'e']);
    });

    it('handles empty arrays', () => {
      expect(capNodesByDegree([], 5)).toEqual([]);
    });

    it('is deterministic across calls', () => {
      const a = capNodesByDegree(nodes, 3).map((n) => n.id);
      const b = capNodesByDegree(nodes, 3).map((n) => n.id);
      expect(b).toEqual(a);
    });
  });

  describe('edgesWithin', () => {
    const edges = [
      { source: 'a', target: 'b' },
      { source: 'b', target: 'c' },
      { source: 'a', target: 'x' }, // x not in set
      { source: 'x', target: 'a' }, // x not in set
    ];

    it('keeps only edges whose source AND target are in the set', () => {
      const out = edgesWithin(edges, new Set(['a', 'b', 'c']));
      expect(out).toEqual([
        { source: 'a', target: 'b' },
        { source: 'b', target: 'c' },
      ]);
    });

    it('returns an empty array when no edge is fully inside the set', () => {
      expect(edgesWithin(edges, new Set(['a']))).toEqual([]);
    });

    it('returns an empty array for an empty set / empty edges', () => {
      expect(edgesWithin([], new Set(['a']))).toEqual([]);
      expect(edgesWithin(edges, new Set())).toEqual([]);
    });
  });
});
