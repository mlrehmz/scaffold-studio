// Bun server: holds history/pending state and talks to the Rust gatekeeper.
// Routes: GET /api/state, POST /api/intent {text}, /api/accept, /api/reject, /api/undo,
// /api/lock/:id, /api/unlock/:id. All return the full State.
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { RuleBasedProposer, type Proposer } from "./proposer";
import * as studio from "./studio";
import type { Pending, State } from "./types";

const ROOT = join(import.meta.dir, "../../..");
// Work on a copy so the example in git stays clean. Delete .studio/ to reset.
const DESIGN = process.env.STUDIO_DESIGN ?? join(ROOT, ".studio/design.toml");
const PROCESS = join(dirname(DESIGN), "process.toml");
const NOTES = join(dirname(DESIGN), "agent");
if (!existsSync(DESIGN)) {
  mkdirSync(dirname(DESIGN), { recursive: true });
  copyFileSync(join(ROOT, "examples/shop/design.toml"), DESIGN);
}

const proposer: Proposer = new RuleBasedProposer();

let pending: Pending | null = null;
const history: string[] = []; // snapshots of design.toml text; undo = restore the last one

function state(): State {
  const { design, violations } = studio.show(DESIGN);
  return { design, violations, pending, canUndo: history.length > 0 };
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
      const lints = studio.lint(PROCESS, NOTES);
      return json({ process, lints });
    }

    if (req.method === "POST" && path === "/api/intent") {
      const { text } = (await req.json()) as { text: string };
      const proposal = await proposer.propose(text, studio.show(DESIGN).design);
      pending = { proposal, verdict: studio.evaluate(DESIGN, proposal) };
      return json(state());
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

    return json({ error: "not found" }, 404);
  } catch (e) {
    return json({ error: String(e instanceof Error ? e.message : e) }, 500);
  }
}

const port = Number(process.env.PORT ?? 3001);
Bun.serve({ port, fetch: handle });
console.log(`studio server on http://localhost:${port}  (design: ${DESIGN})`);
