import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const LOCAL_AI = join(import.meta.dir, "../../local-ai");

export function isAudioRecording(value: FormDataEntryValue | null): value is File {
  if (!(value instanceof File) || value.size === 0) return false;
  return value.type.startsWith("audio/") ||
    (value.type === "video/webm" && value.name.toLowerCase().endsWith(".webm")) ||
    ((value.type === "" || value.type === "application/octet-stream") && value.name.toLowerCase().endsWith(".webm"));
}

export async function transcribe(audio: File): Promise<string> {
  const python = join(LOCAL_AI, ".venv/bin/python");
  const temp = mkdtempSync(join(tmpdir(), "scaffold-studio-audio-"));
  const input = join(temp, "recording.webm");
  try {
    writeFileSync(input, Buffer.from(await audio.arrayBuffer()));
    const process = Bun.spawn(
      [python, join(LOCAL_AI, "transcribe.py"), input],
      { stdout: "pipe", stderr: "pipe" },
    );
    const [output, error, code] = await Promise.all([
      new Response(process.stdout).text(),
      new Response(process.stderr).text(),
      process.exited,
    ]);
    if (code !== 0) throw new Error(error.trim() || "speech transcription failed");
    const result: unknown = JSON.parse(output);
    if (typeof result !== "object" || result === null || !("text" in result) || typeof result.text !== "string") {
      throw new Error("speech model returned an invalid response");
    }
    return result.text;
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}
