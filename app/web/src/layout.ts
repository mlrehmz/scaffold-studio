// Automatic layout with elkjs, so nothing is hand-placed and the model never gives coordinates.
import ELK from "elkjs/lib/elk.bundled.js";
import type { GEdge, GNode } from "./graph";

export const NODE_W = 210;
export const NODE_H = 96;
const elk = new ELK();

export async function layout(nodes: GNode[], edges: GEdge[]): Promise<Record<string, { x: number; y: number }>> {
  const result = await elk.layout({
    id: "root",
    layoutOptions: {
      "elk.algorithm": "layered",
      "elk.direction": "DOWN",
      "elk.spacing.nodeNode": "40",
      "elk.layered.spacing.nodeNodeBetweenLayers": "70",
    },
    children: nodes.map((n) => ({ id: n.id, width: NODE_W, height: NODE_H })),
    edges: edges.map((e) => ({ id: e.id, sources: [e.from], targets: [e.to] })),
  });
  const out: Record<string, { x: number; y: number }> = {};
  for (const c of result.children ?? []) out[c.id] = { x: c.x ?? 0, y: c.y ?? 0 };
  return out;
}
