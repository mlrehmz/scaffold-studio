import { expect, test } from "bun:test";
import { LocalModelProposer, RuleBasedProposer } from "./proposer";
import type { Design } from "./types";

const design: Design = {
  project: { name: "t", target: "typescript" },
  box: [{ id: "checkout", kind: "component", responsibility: "", members: [], locked: false }],
  edge: [],
  rule: [],
};
const p = new RuleBasedProposer();

function mockFetch(body: unknown, onCall: () => void): typeof fetch {
  return Object.assign(
    async (_input: RequestInfo | URL, _init?: RequestInit) => { onCall(); return Response.json(body); },
    { preconnect: async () => {} },
  );
}

test("add X service under Y", async () => {
  const r = await p.propose("add Payments service under checkout", design);
  expect(r.commands[0]).toMatchObject({ op: "add_node", id: "payments", parent: "checkout" });
});

test("link A to B", async () => {
  const r = await p.propose("link web to orders_db", design);
  expect(r.commands[0]).toEqual({ op: "add_edge", from: "web", to: "orders_db", kind: "calls" });
});

test("existing name and gibberish become questions", async () => {
  expect((await p.propose("add checkout", design)).commands[0].op).toBe("ask_question");
  expect((await p.propose("make it pretty", design)).commands[0].op).toBe("ask_question");
});

test("local model output becomes a local proposal and invalid output asks a question", async () => {
  const originalFetch = globalThis.fetch;
  let calls = 0;
  globalThis.fetch = mockFetch({ choices: [{ message: { content: JSON.stringify({ commands: [{ op: "add_edge", from: "web", to: "checkout", kind: "calls" }], rationale: "Connect the web layer." }) } }] }, () => { calls += 1; });
  try {
    const proposal = await new LocalModelProposer().propose("connect web to checkout", design);
    expect(proposal).toMatchObject({ source: "local", commands: [{ op: "add_edge", from: "web", to: "checkout", kind: "calls" }] });

    const invalid = JSON.stringify({ commands: [{ op: "add_node", id: "refunds", kind: "component", target: "orders_db" }], rationale: "Add refunds." });
    globalThis.fetch = mockFetch({ choices: [{ message: { content: invalid } }] }, () => { calls += 1; });
    const fallback = await new LocalModelProposer().propose("change something", design);
    expect(fallback.commands[0].op).toBe("ask_question");
    expect(calls).toBe(3);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("local model requests use NanoI1", async () => {
  const originalFetch = globalThis.fetch;
  let request: unknown = null;
  globalThis.fetch = Object.assign(
    async (_input: RequestInfo | URL, init?: RequestInit) => {
      request = JSON.parse(String(init?.body));
      return Response.json({ choices: [{ message: { content: JSON.stringify({ commands: [{ op: "ask_question", text: "Hello! What would you like to design?" }], rationale: "I greeted the user." }) } }] });
    },
    { preconnect: async () => {} },
  );
  try {
    const proposal = await new LocalModelProposer().propose("add payments", design);
    expect(proposal.commands[0]).toEqual({ op: "ask_question", text: "Hello! What would you like to design?" });
    expect((request as { model: string }).model).toBe("NanoI1-155M-Instruct");
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("local proposer answers a greeting without calling the model", async () => {
  const originalFetch = globalThis.fetch;
  let calls = 0;
  globalThis.fetch = Object.assign(
    async () => { calls += 1; throw new Error("unexpected model request"); },
    { preconnect: async () => {} },
  );
  try {
    const proposal = await new LocalModelProposer().propose("hi", design);
    expect(proposal.commands[0]).toEqual({ op: "ask_question", text: "Hi! What would you like to design?" });
    expect(calls).toBe(0);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("local proposer uses a supported intent when NanoI1 returns invalid output", async () => {
  const originalFetch = globalThis.fetch;
  let calls = 0;
  globalThis.fetch = Object.assign(
    async () => { calls += 1; return Response.json({ choices: [{ message: { content: "not valid proposal JSON" } }] }); },
    { preconnect: async () => {} },
  );
  try {
    const proposal = await new LocalModelProposer().propose("add payments", design);
    expect(proposal.commands[0]).toMatchObject({ op: "add_node", id: "payments" });
    expect(calls).toBe(2);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("local model receives retrieved turns and saved preferences", async () => {
  const originalFetch = globalThis.fetch;
  let request: unknown = null;
  globalThis.fetch = Object.assign(
    async (_input: RequestInfo | URL, init?: RequestInit) => {
      request = JSON.parse(String(init?.body));
      return Response.json({ choices: [{ message: { content: JSON.stringify({ commands: [{ op: "ask_question", text: "Which one?" }], rationale: "I need one detail." }) } }] });
    },
    { preconnect: async () => {} },
  );
  try {
    await new LocalModelProposer().propose("add another service", design, [
      { role: "user", content: "Use small modules." },
      { role: "assistant", content: "I kept the design modular." },
    ], "Prefer small, clear modules.");
    expect(JSON.stringify(request)).toContain("Prefer small, clear modules.");
    expect(JSON.stringify(request)).toContain("Use small modules.");
    expect(JSON.stringify(request)).toContain("I kept the design modular.");
  } finally {
    globalThis.fetch = originalFetch;
  }
});
