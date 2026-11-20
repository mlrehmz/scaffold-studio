import { Background, Controls, ReactFlow, type Edge, type Node } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useEffect, useMemo, useState } from "react";
import type { State } from "../../server/src/types";
import { buildGraph } from "./graph";
import { layout, NODE_H, NODE_W } from "./layout";

interface Props {
  state: State;
  selected: string | null;
  onSelect: (id: string | null) => void;
  open: string | null;
  onOpen: (id: string) => void;
  onPositionSave: (id: string, x: number, y: number) => void;
}

export function Canvas({ state, selected, onSelect, open, onOpen, onPositionSave }: Props) {
  const graph = useMemo(() => buildGraph(state, open), [state, open]);
  // Positions come with the shape they were computed for, so we never draw stale ones.
  const [placed, setPlaced] = useState<{ shape: string; positions: Record<string, { x: number; y: number }> } | null>(null);

  // Re-run the layout only when the set of boxes/edges changes.
  const shape = graph.nodes.map((n) => n.id).join(",") + "#" + graph.edges.map((e) => e.id).join(",");
  useEffect(() => {
    let cancelled = false;
    layout(graph.nodes, graph.edges).then((positions) => !cancelled && setPlaced({ shape, positions }));
    return () => { cancelled = true; };
  }, [shape]); // eslint-disable-line react-hooks/exhaustive-deps

  // Wait for the layout of THIS shape; fitView then runs once on correct positions.
  if (!placed || placed.shape !== shape) return null;
  const positions = placed.positions;

  const nodes: Node[] = graph.nodes.map((n) => ({
    id: n.id,
    position: state.layout[n.id] ?? positions[n.id] ?? { x: 0, y: 0 },
    width: NODE_W,
    height: NODE_H,
    selected: n.id === selected,
    className: ["box", n.ghost && "ghost", n.bad && "bad", n.locked && "locked"].filter(Boolean).join(" "),
    data: {
      label: (
        <div className="box-body">
          <strong>{n.id}</strong>
          {n.layer && <span className="layer">{n.layer}</span>}
          <p>{n.text}</p>
        </div>
      ),
    },
  }));

  const edges: Edge[] = graph.edges.map((e) => {
    const color = e.bad ? "var(--danger)" : e.ghost ? "var(--accent)" : "var(--ink-soft)";
    return {
      id: e.id,
      source: e.from,
      target: e.to,
      label: e.kind,
      animated: false,
      style: { stroke: color, strokeWidth: 2, strokeDasharray: e.ghost ? "6 5" : undefined },
      labelStyle: { fill: color, fontSize: 12 },
    };
  });

  return (
    <ReactFlow
      key={shape}
      defaultNodes={nodes}
      defaultEdges={edges}
      onNodeClick={(_, n) => onSelect(n.id)}
      onNodeDoubleClick={(_, n) => onOpen(n.id)}
      onNodeDragStop={(_, n) => onPositionSave(n.id, n.position.x, n.position.y)}
      onPaneClick={() => onSelect(null)}
      nodesDraggable
      nodesConnectable={false}
      onlyRenderVisibleElements
      fitView
      proOptions={{ hideAttribution: true }}
    >
      <Background gap={24} color="var(--grid)" />
      <Controls showInteractive={false} />
    </ReactFlow>
  );
}
