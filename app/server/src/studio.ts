// Thin wrapper around the Rust `studio` CLI. The ONLY place that spawns it.
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { Design, Lint, Process, Proposal, Verdict, Violation } from "./types";

const BIN = process.env.STUDIO_BIN ?? join(import.meta.dir, "../../../core/target/debug/studio");

function run(args: string[]): { code: number; out: string; err: string } {
  const p = Bun.spawnSync([BIN, ...args]);
  return { code: p.exitCode ?? 2, out: p.stdout.toString(), err: p.stderr.toString() };
}

/** Writes the proposal to a temp file (the CLI reads files, not stdin). */
function proposalFile(p: Proposal): string {
  const file = join(mkdtempSync(join(tmpdir(), "studio-")), "proposal.json");
  writeFileSync(file, JSON.stringify(p));
  return file;
}

export function show(designPath: string): { design: Design; violations: Violation[] } {
  const r = run(["show", designPath]);
  if (r.code !== 0) throw new Error(`design is invalid: ${r.out || r.err}`);
  return JSON.parse(r.out);
}

export function evaluate(designPath: string, p: Proposal): Verdict {
  const r = run(["evaluate", designPath, proposalFile(p)]);
  if (r.code === 2) throw new Error(r.err);
  return JSON.parse(r.out); // exit code 1 still prints a Verdict
}

/** Writes designPath only if the verdict is ok. */
export function apply(designPath: string, p: Proposal): Verdict {
  const r = run(["apply", designPath, proposalFile(p)]);
  if (r.code === 2) throw new Error(r.err);
  return JSON.parse(r.out);
}

export function lock(designPath: string, id: string, locked: boolean): void {
  const r = run(["lock", designPath, id, ...(locked ? [] : ["--unlock"])]);
  if (r.code !== 0) throw new Error(r.err);
}

export function deriveProcess(designPath: string, processPath: string): Process {
  const r = run(["process", designPath, processPath]);
  if (r.code !== 0) throw new Error(r.err || r.out);
  return JSON.parse(r.out);
}

export function notes(designPath: string, processPath: string, notesDir: string): void {
  const r = run(["notes", designPath, processPath, notesDir]);
  if (r.code !== 0) throw new Error(r.err || r.out);
}

export function lint(processPath: string, notesDir: string): Lint[] {
  const r = run(["lint", processPath, notesDir]);
  if (r.code > 1) throw new Error(r.err || r.out);
  return JSON.parse(r.out).lints;
}
