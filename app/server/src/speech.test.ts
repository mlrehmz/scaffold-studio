import { expect, test } from "bun:test";
import { isAudioRecording } from "./speech";

test("accepts a non-empty audio-only WebM recording with the browser's video MIME type", () => {
  const recording = new File([new Uint8Array([1, 2, 3])], "recording.webm", {
    type: "video/webm",
  });

  expect(isAudioRecording(recording)).toBe(true);
});

test("rejects empty recordings and unrelated file types", () => {
  const empty = new File([], "recording.webm", { type: "audio/webm" });
  const image = new File([new Uint8Array([1])], "recording.png", { type: "image/png" });

  expect(isAudioRecording(empty)).toBe(false);
  expect(isAudioRecording(image)).toBe(false);
});
