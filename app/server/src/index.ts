// Bun server: holds history/pending state and talks to the Rust gatekeeper.
// Routes: GET /api/state, POST /api/intent {text}, /api/accept, /api/reject, /api/undo,
// /api/lock/:id, /api/unlock/:id. All return the full State.
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { LocalModelProposer, RuleBasedProposer, type Proposer } from "./proposer";
import { isAudioRecording, transcribe } from "./speech";
import * as studio from "./studio";
import type { Pending, State } from "./types";

const ROOT = join(import.meta.dir, "../../..");
// Work on a copy so the example in git stays clean. Delete .studio/ to reset.
const DESIGN = process.env.STUDIO_DESIGN ?? join(ROOT, ".studio/design.toml");
const PROCESS = join(dirname(DESIGN), "process.toml");
const LAYOUT = join(dirname(DESIGN), "layout.toml");
const NOTES = join(dirname(DESIGN), "agent");
if (!existsSync(DESIGN)) {
  mkdirSync(dirname(DESIGN), { recursive: true });
  copyFileSync(join(ROOT, "examples/shop/design.toml"), DESIGN);
}

const proposer: Proposer = process.env.STUDIO_PROPOSER === "local"
  ? new LocalModelProposer()
  : new RuleBasedProposer();

let pending: Pending | null = null;
const history: string[] = []; // snapshots of design.toml text; undo = restore the last one

function state(): State {
  const { design, violations } = studio.show(DESIGN);
  const layout = existsSync(LAYOUT)
    ? Bun.TOML.parse(readFileSync(LAYOUT, "utf8")) as { position?: { id: string; x: number; y: number }[] }
    : {};
  return {
    design,
    layout: Object.fromEntries((layout.position ?? []).map(({ id, x, y }) => [id, { x, y }])),
    violations,
    pending,
    canUndo: history.length > 0,
  };
}

const json = (body: unknown, status = 200) => Response.json(body, { status });

async function handle(req: Request): Promise<Response> {
  const url = new URL(req.url);
  const path = url.pathname;
  try {
    if (req.method === "GET" && path === "/api/state") return json(state());

    if (req.method === "GET" && path === "/api/process") {
      const process = studio.deriveProcess(DESIGN, PROCESS);
      studio.notes(DESIGN, PROCESS, NOTES);
      const { lints, note_tokens } = studio.lint(PROCESS, NOTES);
      return json({ process, lints, note_tokens });
    }

    if (req.method === "POST" && path === "/api/intent") {
      const { text } = (await req.json()) as { text: string };
      const proposal = await proposer.propose(text, studio.show(DESIGN).design);
      pending = { proposal, verdict: studio.evaluate(DESIGN, proposal) };
      return json(state());
    }

    if (req.method === "POST" && path === "/api/transcribe") {
      const audio = (await req.formData()).get("audio");
      if (!isAudioRecording(audio)) {
        return json({ error: "expected a non-empty audio recording" }, 400);
      }
      if (audio.size > 25_000_000) return json({ error: "recording is larger than 25 MB" }, 413);
      return json({ text: await transcribe(audio) });
    }

    if (req.method === "POST" && path === "/api/accept") {
      if (!pending) return json({ error: "nothing to accept" }, 409);
      if (!pending.verdict.ok) return json({ error: "blocked by rules or errors" }, 409);
      const snapshot = readFileSync(DESIGN, "utf8");
      const verdict = studio.apply(DESIGN, pending.proposal); // the gatekeeper re-checks
      if (!verdict.ok) return json({ error: "blocked by rules or errors" }, 409);
      history.push(snapshot);
      pending = null;
      return json(state());
    }

    if (req.method === "POST" && path === "/api/reject") {
      pending = null;
      return json(state());
    }

    if (req.method === "POST" && path === "/api/undo") {
      const snapshot = history.pop();
      if (snapshot === undefined) return json({ error: "nothing to undo" }, 409);
      writeFileSync(DESIGN, snapshot);
      pending = null;
      return json(state());
    }

    const lockMatch = path.match(/^\/api\/(lock|unlock)\/([\w-]+)$/);
    if (req.method === "POST" && lockMatch) {
      history.push(readFileSync(DESIGN, "utf8"));
      studio.lock(DESIGN, lockMatch[2], lockMatch[1] === "lock");
      return json(state());
    }

    if (req.method === "POST" && path === "/api/layout") {
      const { id, x, y } = (await req.json()) as { id?: unknown; x?: unknown; y?: unknown };
      if (typeof id !== "string" || !id || typeof x !== "number" || !Number.isFinite(x) || typeof y !== "number" || !Number.isFinite(y)) {
        return json({ error: "expected an id and finite x/y coordinates" }, 400);
      }
      const current = state().layout;
      current[id] = { x, y };
      const position = Object.entries(current).sort(([a], [b]) => a.localeCompare(b)).map(([box, point]) => ({ id: box, ...point }));
      writeFileSync(LAYOUT, Bun.TOML.stringify({ position }));
      return json(state());
    }

    return json({ error: "not found" }, 404);
  } catch (e) {
    return json({ error: String(e instanceof Error ? e.message : e) }, 500);
  }
}

const port = Number(process.env.PORT ?? 3001);
Bun.serve({ port, fetch: handle });
console.log(`studio server on http://localhost:${port}  (design: ${DESIGN})`);
