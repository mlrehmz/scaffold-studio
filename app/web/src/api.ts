// Calls to the Bun server. Every call returns the full State; the UI only renders it.
import type { Lint, NoteTokenCount, Process, State } from "../../server/src/types";

export interface ProcessView { process: Process; lints: Lint[]; note_tokens: NoteTokenCount[] }
export interface EmbeddingInput { text: string; kind: "query" | "document" }
export interface HistoryEmbedding { index: number; embedding: number[] }

let embeddingWorker: Worker | null = null;
let embeddingRequestId = 0;
const embeddingRequests = new Map<number, { resolve: (vectors: number[][]) => void; reject: (error: Error) => void }>();

function createEmbeddingWorker(): Worker {
  const worker = new Worker(new URL("./embedding.worker.ts", import.meta.url), { type: "module" });
  worker.addEventListener("message", (event: MessageEvent<{ id: number; vectors?: number[][]; error?: string }>) => {
    const request = embeddingRequests.get(event.data.id);
    if (!request) return;
    embeddingRequests.delete(event.data.id);
    if (event.data.error) request.reject(new Error(event.data.error));
    else request.resolve(event.data.vectors ?? []);
  });
  worker.addEventListener("error", (event) => {
    for (const request of embeddingRequests.values()) request.reject(new Error(event.message));
    embeddingRequests.clear();
    worker.terminate();
    embeddingWorker = null;
  });
  return worker;
}

function embed(inputs: EmbeddingInput[]): Promise<number[][]> {
  embeddingWorker ??= createEmbeddingWorker();
  const id = ++embeddingRequestId;
  return new Promise((resolve, reject) => {
    embeddingRequests.set(id, { resolve, reject });
    embeddingWorker?.postMessage({ id, inputs });
  });
}

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
  intent: (text: string, embedding?: number[], historyEmbeddings?: HistoryEmbedding[]) =>
    call("POST", "intent", { text, embedding, historyEmbeddings }),
  embed,
  preferences: (preferences: string) => call("POST", "preferences", { preferences }),
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
  transcribe: async (audio: Blob): Promise<string> => {
    const form = new FormData();
    form.append("audio", audio, "recording.webm");
    const res = await fetch("/api/transcribe", { method: "POST", body: form });
    const data = await res.json() as { text?: string; error?: string };
    if (!res.ok) throw new Error(data.error ?? res.statusText);
    if (typeof data.text !== "string") throw new Error("speech service returned no transcript");
    return data.text;
  },
};
