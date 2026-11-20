// Calls to the Bun server. Every call returns the full State; the UI only renders it.
import type { Lint, NoteTokenCount, Process, State } from "../../server/src/types";

export interface ProcessView { process: Process; lints: Lint[]; note_tokens: NoteTokenCount[] }

async function call(method: "GET" | "POST", path: string, body?: unknown): Promise<State> {
  const res = await fetch(`/api/${path}`, {
    method,
    headers: { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const data = await res.json();
  if (!res.ok) throw new Error(data.error ?? res.statusText);
  return data as State;
}

export const api = {
  state: () => call("GET", "state"),
  intent: (text: string) => call("POST", "intent", { text }),
  accept: () => call("POST", "accept"),
  reject: () => call("POST", "reject"),
  undo: () => call("POST", "undo"),
  setLocked: (id: string, locked: boolean) => call("POST", `${locked ? "lock" : "unlock"}/${id}`),
  savePosition: (id: string, x: number, y: number) => call("POST", "layout", { id, x, y }),
  process: async (): Promise<ProcessView> => {
    const res = await fetch("/api/process");
    const data = await res.json();
    if (!res.ok) throw new Error(data.error ?? res.statusText);
    return data as ProcessView;
  },
};
