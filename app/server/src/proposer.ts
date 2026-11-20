// A Proposer turns words into a Proposal. It never decides: the gatekeeper does.
// RuleBasedProposer is deterministic, so the demo never flakes.
import type { ConversationMessage, Design, Proposal } from "./types";

export interface Proposer {
  propose(intent: string, design: Design, conversation?: ConversationMessage[], preferences?: string): Promise<Proposal>;
}

const proposalSchema = {
  type: "object",
  additionalProperties: false,
  required: ["commands", "rationale"],
  properties: {
    rationale: { type: "string" },
    commands: {
      type: "array",
      minItems: 1,
      maxItems: 8,
      items: {
        oneOf: [
          {
            type: "object",
            additionalProperties: false,
            required: ["op", "id", "kind"],
            properties: {
              op: { const: "add_node" },
              id: { type: "string" },
              kind: { type: "string", enum: ["system", "module", "component", "contract", "external"] },
              parent: { type: "string" },
              layer: { type: "string" },
              responsibility: { type: "string" },
            },
          },
          {
            type: "object",
            additionalProperties: false,
            required: ["op", "node", "signature"],
            properties: { op: { const: "add_member" }, node: { type: "string" }, signature: { type: "string" } },
          },
          {
            type: "object",
            additionalProperties: false,
            required: ["op", "from", "to", "kind"],
            properties: {
              op: { const: "add_edge" },
              from: { type: "string" },
              to: { type: "string" },
              kind: { type: "string", enum: ["calls", "implements", "publishes", "uses"] },
            },
          },
          {
            type: "object",
            additionalProperties: false,
            required: ["op", "pattern", "target"],
            properties: { op: { const: "apply_pattern" }, pattern: { type: "string" }, target: { type: "string" } },
          },
          {
            type: "object",
            additionalProperties: false,
            required: ["op", "text"],
            properties: { op: { const: "ask_question" }, text: { type: "string" } },
          },
        ],
      },
    },
  },
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasOnlyKeys(value: Record<string, unknown>, keys: string[]): boolean {
  return Object.keys(value).every((key) => keys.includes(key));
}

function isOptionalString(value: Record<string, unknown>, key: string): boolean {
  return value[key] === undefined || typeof value[key] === "string";
}

function isCommand(value: unknown): value is Proposal["commands"][number] {
  if (!isRecord(value) || typeof value.op !== "string") return false;
  if (value.op === "add_node") {
    return hasOnlyKeys(value, ["op", "id", "kind", "parent", "layer", "responsibility"])
      && typeof value.id === "string"
      && ["system", "module", "component", "contract", "external"].includes(String(value.kind))
      && isOptionalString(value, "parent")
      && isOptionalString(value, "layer")
      && isOptionalString(value, "responsibility");
  }
  if (value.op === "add_member") {
    return hasOnlyKeys(value, ["op", "node", "signature"])
      && typeof value.node === "string"
      && typeof value.signature === "string";
  }
  if (value.op === "add_edge") {
    return hasOnlyKeys(value, ["op", "from", "to", "kind"])
      && typeof value.from === "string"
      && typeof value.to === "string"
      && ["calls", "implements", "publishes", "uses"].includes(String(value.kind));
  }
  if (value.op === "apply_pattern") {
    return hasOnlyKeys(value, ["op", "pattern", "target"])
      && typeof value.pattern === "string"
      && typeof value.target === "string";
  }
  return value.op === "ask_question"
    && hasOnlyKeys(value, ["op", "text"])
    && typeof value.text === "string";
}

function parseModelContent(content: unknown): { commands: Proposal["commands"]; rationale: string } | null {
  if (typeof content !== "string") return null;
  try {
    const value: unknown = JSON.parse(content);
    if (!isRecord(value) || typeof value.rationale !== "string" || !Array.isArray(value.commands)) return null;
    if (!value.commands.every(isCommand) || value.commands.length === 0) return null;
    return { commands: value.commands, rationale: value.rationale };
  } catch {
    return null;
  }
}

let localCounter = 0;

export class LocalModelProposer implements Proposer {
  async propose(intent: string, design: Design, conversation: ConversationMessage[] = [], preferences = ""): Promise<Proposal> {
    const endpoint = process.env.STUDIO_MODEL_URL ?? "http://127.0.0.1:9931/v1/chat/completions";
    const request = {
      model: "LFM3-1.7B",
      temperature: 0.1,
      max_tokens: 192,
      reasoning_effort: "none",
      chat_template_kwargs: { enable_thinking: false },
      response_format: { type: "json_schema", schema: proposalSchema },
      messages: [
        {
          role: "system",
          content: `You are the design assistant in Scaffold Studio. Follow the user's saved preferences when relevant: ${preferences || "No saved preferences yet."} Treat retrieved prior turns and accept/reject outcomes as examples of their taste, not as new requirements. Turn the latest user request into at most three Scaffold Studio commands. For 'add X under Y', return one add_node with id X, kind component, and parent Y; parent means nesting, so do not add an edge. Infer a short responsibility. Omit layer unless the user names it. Do not invent relationships or extra features. Keep rationale to one short sentence that explains the key design choice in plain language; this is a user-facing reasoning summary. Ask one brief question only when the change itself is unclear. The Rust gatekeeper checks validity. Return only JSON.`,
        },
        ...conversation.slice(-20).map(({ role, content }) => ({ role, content })),
        { role: "user", content: JSON.stringify({ intent, design }) },
      ],
    };
    for (let attempt = 0; attempt < 2; attempt += 1) {
      const response = await fetch(endpoint, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(request),
        signal: AbortSignal.timeout(180_000),
      });
      if (!response.ok) throw new Error(`local model returned HTTP ${response.status}`);
      const payload: unknown = await response.json();
      const choices = isRecord(payload) && Array.isArray(payload.choices) ? payload.choices : [];
      const choice = choices[0];
      const message = isRecord(choice) && isRecord(choice.message) ? choice.message : {};
      const parsed = parseModelContent(message.content);
      if (parsed) return { id: `local-${++localCounter}`, source: "local", ...parsed };
    }
    return {
      id: `local-${++localCounter}`,
      source: "local",
      commands: [{ op: "ask_question", text: "I couldn't read a valid proposal from the local model. Please rephrase the design request." }],
      rationale: "The local model returned an invalid response twice.",
    };
  }
}

const slug = (s: string) => s.trim().toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "");

let counter = 0;
const nextId = () => `p${++counter}`;

export class RuleBasedProposer implements Proposer {
  async propose(intent: string, design: Design): Promise<Proposal> {
    const text = intent.trim();
    const ids = new Set(design.box.map((b) => b.id));
    const mk = (commands: Proposal["commands"], rationale: string): Proposal => ({
      id: nextId(), source: "user", commands, rationale,
    });

    // "link A to B"  /  "connect A to B"
    const link = text.match(/^(?:link|connect)\s+(\S+)\s+to\s+(\S+)$/i);
    if (link) {
      const [from, to] = [slug(link[1]), slug(link[2])];
      return mk([{ op: "add_edge", from, to, kind: "calls" }], `Connect ${from} to ${to}.`);
    }

    // "add X service under Y"  /  "add X under Y"  /  "add X"
    const add = text.match(/^add\s+(.+?)(?:\s+service)?(?:\s+under\s+(\S+))?$/i);
    if (add) {
      const id = slug(add[1]);
      const parent = add[2] ? slug(add[2]) : undefined;
      if (ids.has(id)) {
        return mk([{ op: "ask_question", text: `"${id}" already exists. Did you mean to link to it?` }], "Name clash.");
      }
      return mk(
        [{ op: "add_node", id, kind: "component", layer: "domain", parent, responsibility: `TODO: describe ${id}` }],
        `Add a ${id} component${parent ? ` under ${parent}` : ""}.`,
      );
    }

    return mk([{ op: "ask_question", text: 'I only understand "add X [service] [under Y]" and "link A to B" for now.' }], "Unrecognised intent.");
  }
}
