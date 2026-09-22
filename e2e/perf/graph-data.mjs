/**
 * Graph data generator for the graph-view performance probe.
 *
 * Mirrors the shape of the real `get_graph_panel_data` Rust command
 * (GraphPanelDataDto: { graph: { nodes, edges, node_count, edge_count, vault_path },
 * components, orphans }) so the frontend receives payloads that are identical
 * in structure (and representative in density) to production. No layout numbers
 * are pre-seeded — the d3 force simulation starts from the same unpositioned
 * state as a real load, so warm-up cost is measured, not skipped.
 *
 * Graph model (scale-free-ish, local):
 *  - `N` nodes, ids "node-000000".."node-<N-1>".
 *  - `E ~= floor(N * cfg.edgesPerNode)` directed edges drawn locally: each node
 *    i links to (i%N)+1, (i*7+3)%N, ... This yields sparse, representative
 *    edge lists (like real note graphs) rather than degenerate random graphs.
 *  - One connected component (a ring through all nodes) so the component/BFS path
 *    is representative at size, plus `cfg.orphanFraction` orphan nodes (no edges).
 *  - tags: `cfg.tagPool` tags assigned deterministically so the tag-driven node
 *    palette is exercised.
 */

/** Deterministic PRNG (mulberry32) so runs are reproducible. */
function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a |= 0; a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function generateGraph(cfg) {
  const N = cfg.nodes;
  const ePer = cfg.edgesPerNode ?? 2;
  const orphanFrac = cfg.orphanFraction ?? 0;
  const tagPool = Math.max(1, cfg.tagPool ?? 8);
  const rand = mulberry32(cfg.seed ?? 1);

  const nodeIds = [];
  const nodePath = new Map();
  const nodeTags = new Map();
  const nodeDegree = new Map();

  for (let i = 0; i < N; i++) {
    const id = `node-${String(i).padStart(6, '0')}`;
    nodeIds.push(id);
    nodePath.set(id, `pages/${id}.md`);
    const tags = [];
    if (i % 3 === 0) tags.push(`tag-${i % tagPool}`);
    if (i % 7 === 0) tags.push(`tag-${(i + 1) % tagPool}`);
    nodeTags.set(id, tags);
    nodeDegree.set(id, 0);
  }

  const edges = [];
  const linked = new Set(); // `src|tgt`

  const addEdge = (src, tgt) => {
    if (src === tgt) return;
    const key = `${src}|${tgt}`;
    if (linked.has(key)) return;
    linked.add(key);
    edges.push({ source: src, target: tgt, label: null });
    nodeDegree.set(src, (nodeDegree.get(src) ?? 0) + 1);
    if (tgt !== src) nodeDegree.set(tgt, (nodeDegree.get(tgt) ?? 0) + 1);
  };

  // Deterministic local links — connected ring + k-regular-ish edges.
  for (let i = 0; i < N; i++) {
    const src = nodeIds[i];
    const tgtA = nodeIds[(i + 1) % N]; // ring — guarantees one connected component
    addEdge(src, tgtA);
    for (let k = 1; k < ePer; k++) {
      const j = Math.floor(rand() * N);
      addEdge(src, nodeIds[j]);
    }
  }

  // Mark some nodes as orphans (no edges) if requested.
  const orphanIds = new Set();
  if (orphanFrac > 0) {
    const orphanCount = Math.floor(N * orphanFrac);
    for (let k = 0; k < orphanCount; k++) {
      const j = Math.floor(rand() * N);
      orphanIds.add(nodeIds[j]);
    }
  }

  const nodes = nodeIds.map((id) => ({
    id,
    title: `Note ${id.replace('node-', '')}`,
    path: nodePath.get(id),
    degree: nodeDegree.get(id) ?? 0,
    tags: nodeTags.get(id) ?? [],
  }));

  return { nodes, edges };
}

/** Build the full GraphPanelDataDto payload for the invoke mock. */
export function buildPanelPayload(cfg) {
  const { nodes, edges } = generateGraph(cfg);
  const connected = nodes.filter((n) => n.degree > 0);
  const orphanNodes = nodes.filter((n) => n.degree === 0);
  const orphanIds = new Set(orphanNodes.map((n) => n.id));

  return {
    graph: {
      nodes,
      edges,
      node_count: nodes.length,
      edge_count: edges.length,
      vault_path: '/mock/vault',
    },
    components:
      connected.length > 0
        ? [{ nodes: connected, size: connected.length }]
        : [],
    orphans: orphanNodes.map((n) => ({ slug: n.id, title: n.title, path: n.path })),
  };
}
