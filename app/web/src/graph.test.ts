import { expect, test } from "bun:test";
import type { Design, State } from "../../server/src/types";
import { buildGraph } from "./graph";

const box = (id: string, parent?: string) => ({ id, kind: "component" as const, parent, responsibility: "", members: [], locked: false });
const design = (boxes: Design["box"], edge: Design["edge"] = []): Design => ({
  project: { name: "t", target: "typescript" }, box: boxes, edge, rule: [],
});
const state = (d: Design, pending: State["pending"] = null, violations: State["violations"] = []): State => ({
  design: d, layout: {}, violations, pending, canUndo: false,
});

test("a box that exists only in the preview is a ghost; boxes in violations are bad", () => {
  const current = design([box("web"), box("db")]);
  const preview = design([box("web"), box("db"), box("payments")]);
  const g = buildGraph(state(current, {
    proposal: { id: "p", source: "user", commands: [], rationale: "" },
    verdict: { ok: false, errors: [], questions: [], preview, violations: [{ rule_id: "r1", message: "m", boxes: ["web", "db"] }] },
  }));
  expect(g.nodes.map((n) => [n.id, n.ghost, n.bad])).toEqual([["web", false, true], ["db", false, true], ["payments", true, false]]);
});

// T20: remove `.skip` and make it pass.
test("drill-down shows only the children of the open box", () => {
  const d = design(
    [box("shop"), box("checkout", "shop"), box("payments", "shop")],
    [{ from: "checkout", to: "payments", kind: "calls" }],
  );
  expect(buildGraph(state(d), null).nodes.map((n) => n.id)).toEqual(["shop"]);
  const inside = buildGraph(state(d), "shop");
  expect(inside.nodes.map((n) => n.id)).toEqual(["checkout", "payments"]);
  expect(inside.edges.length).toBe(1);
  expect(buildGraph(state(d), null).edges.length).toBe(0); // the edge's ends are hidden at the top level
});
