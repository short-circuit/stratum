/**
 * Pure graph filtering / capping helpers for the GraphPanel.
 *
 * Extracted from useGraphPanel so the capping policy is deterministic,
 * unit-testable, and reviewable independently of React state. The desktop and
 * mobile variants both consume the same policy via useGraphPanel.
 */

/** Vaults at or above this node count are considered "large" and get a safe
 *  default render cap even when the user has left node_cap at 0 (unlimited). */
export const LARGE_VAULT_THRESHOLD = 2000;

/** Safe default cap applied to large vaults when node_cap === 0. Bounds the
 *  p0 SpriteText + d3 simulation cost to a size the main thread can still
 *  warm up to quiet (measured: 2k nodes reaches quiet in ~17 s on RTX 4090;
 *  10k untouched never quiets within 124 s). */
export const DEFAULT_LARGE_VAULT_CAP = 2000;

/**
 * Compute the effective node cap to apply to the rendered set.
 *
 * Policy:
 *  - An explicit user cap (node_cap > 0) always wins.
 *  - node_cap === 0 ("unlimited") means: unlimited for normal vaults, but a
 *    safe default cap is applied once the vault exceeds LARGE_VAULT_THRESHOLD.
 *    This keeps small vaults rendering in full (no regression) while bounding
 *    the worst-case cost at 10k nodes.
 *
 * @param nodeCap     the configured node_cap (0 = unlimited)
 * @param totalNodes  the number of nodes before capping
 * @returns the effective cap to apply (>0 means cap is active)
 */
export function effectiveNodeCap(nodeCap: number, totalNodes: number): number {
  if (nodeCap > 0) return nodeCap;
  if (totalNodes > LARGE_VAULT_THRESHOLD) return DEFAULT_LARGE_VAULT_CAP;
  return 0; // no cap
}

/**
 * Reduce `nodes` to at most `cap` entries, preferring higher-degree nodes so
 * the "most connected" notes stay visible when a cap is applied.
 *
 * Stable ordering: never reorders within the kept subset (keeps the original
 * array order for kept nodes so nothing else that depends on source order
 * regresses). Uncap is handled by the caller before calling.
 *
 * @param nodes the candidate nodes (already order-stable)
 * @param cap   maximum kept; <=0 or >= nodes.length returns nodes unchanged
 */
export function capNodesByDegree<T extends { id: string; degree?: number }>(nodes: T[], cap: number): T[] {
  if (cap <= 0 || cap >= nodes.length) return nodes;
  // Indices of the highest-degree nodes, ties broken by original order.
  const idx = nodes
    .map((n, i) => ({ i, degree: n.degree ?? 0 }))
    .sort((a, b) => b.degree - a.degree || a.i - b.i)
    .slice(0, cap)
    .sort((a, b) => a.i - b.i)
    .map((x) => x.i);
  return idx.map((i) => nodes[i]);
}

/**
 * Filter `edges` to those whose source AND target are present in `ids`.
 * Returns a fresh array; used to keep the link set consistent with the
 * capped/in-view node set.
 */
export function edgesWithin(edges: { source: string; target: string }[], ids: Set<string>): { source: string; target: string }[] {
  return edges.filter((e) => ids.has(e.source) && ids.has(e.target));
}
