import { expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { readConversation, recordProposalOutcome, retrieveConversation, retrieveSimilarConversation, writeConversation } from "./conversation";

test("retrieves matching conversation turns in their original order", () => {
  const messages = [
    { role: "user" as const, content: "add a payment service" },
    { role: "assistant" as const, content: "I kept payments as a separate service." },
    { role: "user" as const, content: "change the checkout name" },
    { role: "assistant" as const, content: "I updated checkout." },
  ];
  expect(retrieveConversation(messages, "payments service")).toEqual(messages.slice(0, 2));
});

test("retrieves semantically closest turns and keeps each answer with its request", () => {
  const first = Array.from({ length: 256 }, (_, index) => index === 0 ? 0 : index === 1 ? 1 : 0);
  const second = Array.from({ length: 256 }, (_, index) => index === 0 ? 1 : 0);
  const messages = [
    { role: "user" as const, content: "older unrelated request", embedding: first },
    { role: "assistant" as const, content: "older answer" },
    { role: "user" as const, content: "similar request", embedding: second },
    { role: "assistant" as const, content: "similar answer" },
  ];
  expect(retrieveSimilarConversation(messages, second).map(({ role, content }) => ({ role, content }))).toEqual([
    { role: "user", content: "similar request" },
    { role: "assistant", content: "similar answer" },
  ]);
});

test("persists preferences and bounds saved conversation history", () => {
  const directory = mkdtempSync(join(tmpdir(), "studio-conversation-"));
  const path = join(directory, "conversation.json");
  try {
    const messages = Array.from({ length: 42 }, (_, index) => ({ role: "user" as const, content: `request ${index}` }));
    writeConversation(path, { messages, preferences: "Keep designs simple." });
    const saved = readConversation(path);
    expect(saved.messages).toHaveLength(40);
    expect(saved.messages[0].content).toBe("request 2");
    expect(saved.preferences).toBe("Keep designs simple.");
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("keeps proposal acceptance as feedback in the conversation memory", () => {
  const conversation = {
    messages: [
      { role: "user" as const, content: "add a small module" },
      { role: "assistant" as const, content: "I used a small module.", rationale: "I kept the design small." },
    ],
    preferences: "",
  };
  recordProposalOutcome(conversation, "The proposed change was accepted and applied.");
  expect(conversation.messages[1].content).toBe("I kept the design small.\n\nThe proposed change was accepted and applied.");
  expect(conversation.messages[1].rationale).toBe("I kept the design small.");
});
