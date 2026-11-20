import { existsSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import type { ConversationMessage } from "./types";

export interface StoredMessage extends ConversationMessage { embedding?: number[] }
export interface Conversation { messages: StoredMessage[]; preferences: string }
const EMBEDDING_DIMENSIONS = 256;

export function isEmbedding(value: unknown): value is number[] {
  return Array.isArray(value) && value.length === EMBEDDING_DIMENSIONS
    && value.every((item) => typeof item === "number" && Number.isFinite(item));
}

export function retrieveConversation(messages: StoredMessage[], query: string, queryEmbedding?: number[]): ConversationMessage[] {
  if (queryEmbedding) return retrieveSimilarConversation(messages, queryEmbedding);
  const terms = new Set(query.toLowerCase().match(/[a-z0-9_]{3,}/g) ?? []);
  const turns: { score: number; order: number; messages: ConversationMessage[] }[] = [];
  for (let i = 0; i < messages.length; i += 1) {
    if (messages[i].role !== "user") continue;
    const answer = messages[i + 1]?.role === "assistant" ? messages[i + 1] : undefined;
    const content = `${messages[i].content} ${answer?.content ?? ""}`.toLowerCase();
    const score = [...terms].reduce((total, term) => total + (content.includes(term) ? 1 : 0), 0);
    if (score > 0) turns.push({ score, order: i, messages: answer ? [messages[i], answer] : [messages[i]] });
  }
  return turns.sort((a, b) => b.score - a.score || b.order - a.order).slice(0, 4)
    .sort((a, b) => a.order - b.order).flatMap((turn) => turn.messages);
}

export function retrieveSimilarConversation(messages: StoredMessage[], queryEmbedding: number[]): ConversationMessage[] {
  const turns: { score: number; order: number; messages: ConversationMessage[] }[] = [];
  for (let i = 0; i < messages.length; i += 1) {
    const question = messages[i];
    if (question.role !== "user" || !question.embedding || question.embedding.length !== queryEmbedding.length) continue;
    const score = question.embedding.reduce((sum, value, index) => sum + value * queryEmbedding[index], 0);
    if (score > 0) {
      const answer = messages[i + 1]?.role === "assistant" ? messages[i + 1] : undefined;
      turns.push({ score, order: i, messages: answer ? [question, answer] : [question] });
    }
  }
  return turns.sort((a, b) => b.score - a.score || b.order - a.order).slice(0, 4)
    .sort((a, b) => a.order - b.order).flatMap((turn) => turn.messages);
}

function isMessage(value: unknown): value is ConversationMessage {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  const message = value as Record<string, unknown>;
  return (message.role === "user" || message.role === "assistant")
    && typeof message.content === "string"
    && (message.rationale === undefined || typeof message.rationale === "string")
    && (message.embedding === undefined || isEmbedding(message.embedding));
}

export function readConversation(path: string): Conversation {
  if (!existsSync(path)) return { messages: [], preferences: "" };
  const value: unknown = JSON.parse(readFileSync(path, "utf8"));
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("conversation file must be an object");
  const record = value as Record<string, unknown>;
  if (!Array.isArray(record.messages) || !record.messages.every(isMessage) || typeof record.preferences !== "string") {
    throw new Error("conversation file has an invalid format");
  }
  return { messages: record.messages.slice(-40) as StoredMessage[], preferences: record.preferences };
}

export function writeConversation(path: string, conversation: Conversation): void {
  conversation.messages = conversation.messages.slice(-40);
  const temporary = `${path}.tmp`;
  writeFileSync(temporary, JSON.stringify({
    messages: conversation.messages,
    preferences: conversation.preferences,
  }, null, 2));
  renameSync(temporary, path);
}

export function recordProposalOutcome(conversation: Conversation, outcome: string): void {
  const response = conversation.messages.at(-1);
  if (response?.role !== "assistant" || !response.rationale) return;
  response.content = `${response.rationale}\n\n${outcome}`;
}
