/**
 * Web Worker that runs a d3-force-3d simulation to pre-compute node positions
 * off the main thread. Posts back positions when the simulation stabilises.
 *
 * Used by GraphPanel.shared.tsx for Phase 5 layout offload.
 *
 * The worker receives the full node/edge set, runs 300 simulation ticks,
 * and returns the final {x, y, z} positions for every node.
 */

import { forceSimulation, forceLink, forceManyBody, forceCenter } from 'd3-force-3d';

interface LayoutNode {
  id: string;
}

interface LayoutLink {
  source: string;
  target: string;
}

interface LayoutMessage {
  nodes: LayoutNode[];
  links: LayoutLink[];
  chargeStrength: number;
  linkDistance: number;
  alphaDecay: number;
  velocityDecay: number;
}

interface PositionResult {
  id: string;
  x: number;
  y: number;
  z: number;
}

interface PositionMessage {
  positions: PositionResult[];
}

self.onmessage = (event: MessageEvent<LayoutMessage>) => {
  const { nodes, links, chargeStrength, linkDistance, alphaDecay, velocityDecay } = event.data;

  // Create a mutable copy with position fields added
  const simNodes: any[] = nodes.map((n) => ({ ...n }));
  const simLinks: any[] = links.map((l) => ({ ...l }));

  // Build and run the simulation — mirrors the d3-force config used by
  // react-force-graph-3d / react-force-graph-2d on the main thread.
  const simulation = forceSimulation(simNodes)
    .force(
      'link',
      forceLink(simLinks)
        .id((d: any) => d.id)
        .distance(linkDistance),
    )
    .force('charge', forceManyBody().strength(chargeStrength))
    .force('center', forceCenter())
    .alphaDecay(alphaDecay)
    .velocityDecay(velocityDecay)
    .stop();

  // Tick 300 times — enough to reach near-stable positions for most graphs
  const TICKS = 300;
  for (let i = 0; i < TICKS; i++) {
    simulation.tick();
  }

  // Extract positions
  const positions: PositionResult[] = simNodes.map((n: any) => ({
    id: n.id,
    x: n.x ?? 0,
    y: n.y ?? 0,
    z: n.z ?? 0,
  }));

  const response: PositionMessage = { positions };
  self.postMessage(response);
};
