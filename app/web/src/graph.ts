// Pure view-model: State -> what to draw. No React, no fetching, easy to test.
import type { State } from "../../server/src/types";

export interface GNode { id: string; layer?: string; text: string; ghost: boolean; bad: boolean; locked: boolean }
export interface GEdge { id: string; from: string; to: string; kind: string; ghost: boolean; bad: boolean }

/** `openBox` is the box shown as the current level, or null for the top level. */
export function buildGraph(state: State, openBox: string | null = null): { nodes: GNode[]; edges: GEdge[] } {
  const verdict = state.pending?.verdict;
  // Draw the preview when there is one: it contains the current design plus the ghosts.
  const shown = verdict?.preview ?? state.design;
  const haveBox = new Set(state.design.box.map((b) => b.id));
  const haveEdge = new Set(state.design.edge.map((e) => `${e.from}|${e.to}|${e.kind}`));

  const problems = [...state.violations, ...(verdict?.violations ?? [])];
  const redBoxes = new Set(problems.flatMap((v) => v.boxes));
  const redEdge = (from: string, to: string) => problems.some((v) => v.boxes.includes(from) && v.boxes.includes(to));

  const boxes = shown.box.filter((box) => (box.parent ?? null) === openBox);
  const visible = new Set(boxes.map((box) => box.id));
  return {
    nodes: boxes.map((b) => ({
      id: b.id,
      layer: b.layer,
      text: b.responsibility,
      ghost: !haveBox.has(b.id),
      bad: redBoxes.has(b.id),
      locked: b.locked,
    })),
    edges: shown.edge.filter((e) => visible.has(e.from) && visible.has(e.to)).map((e) => ({
      id: `${e.from}|${e.to}|${e.kind}`,
      from: e.from,
      to: e.to,
      kind: e.kind,
      ghost: !haveEdge.has(`${e.from}|${e.to}|${e.kind}`),
      bad: redEdge(e.from, e.to),
    })),
  };
}
