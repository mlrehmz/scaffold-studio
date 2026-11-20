// Mirrors the Rust types (core/crates/studio-core). If you change a Rust type, change it here.
// The web app imports these too (type-only), so there is one copy on the TypeScript side.

export type Kind = "system" | "module" | "component" | "contract" | "external";
export type EdgeKind = "calls" | "implements" | "publishes" | "uses";

export interface DesignBox {
  id: string;
  kind: Kind;
  parent?: string;
  layer?: string;
  responsibility: string;
  members: string[];
  locked: boolean;
}
export interface Edge { from: string; to: string; kind: EdgeKind }
export interface Rule { id: string; kind: string; reason: string }

export interface HelperAgent {
  spawn_when: string;
  do_not_spawn_when: string;
  context_files: string[];
  return_format: string;
  must_not_touch: string[];
}
export interface Step {
  id: string;
  box: string;
  trigger: string;
  finish: string[];
  tools: string[];
  draft: boolean;
  helper_agent?: HelperAgent;
}
export interface Process { steps: Step[] }
export type Lint = { check: string; severity: "error" | "warning"; path: string; message: string };
export type NoteTokenCount = [path: string, tokens: number];

// TOML/JSON keys are singular on purpose: box, edge, rule.
export interface Design {
  project: { name: string; target: string };
  box: DesignBox[];
  edge: Edge[];
  rule: Rule[];
}

export type Command =
  | { op: "add_node"; id: string; kind: Kind; parent?: string; layer?: string; responsibility?: string }
  | { op: "add_member"; node: string; signature: string }
  | { op: "add_edge"; from: string; to: string; kind: EdgeKind }
  | { op: "apply_pattern"; pattern: string; target: string }
  | { op: "set_trigger"; step: string; trigger: string }
  | { op: "ask_question"; text: string };

export type Source = "user" | "local" | "frontier" | "sketch";

export interface Proposal {
  id: string;
  source: Source;
  commands: Command[];
  rationale: string;
}

export interface Violation { rule_id: string; message: string; boxes: string[] }

export interface Verdict {
  ok: boolean;
  errors: string[];
  violations: Violation[];
  questions: string[];
  preview: Design | null;
}

export interface Pending { proposal: Proposal; verdict: Verdict }

/** Everything the UI needs. The UI only renders this; it never computes rules. */
export interface State {
  design: Design;
  layout: Record<string, { x: number; y: number }>;
  violations: Violation[]; // problems in the current design
  pending: Pending | null;
  canUndo: boolean;
  conversation: ConversationMessage[];
  preferences: string;
  localModelEnabled: boolean;
}

export interface ConversationMessage {
  role: "user" | "assistant";
  content: string;
  rationale?: string;
  hasEmbedding?: boolean;
}
