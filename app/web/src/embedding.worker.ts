import { AutoConfig, AutoModel, AutoTokenizer } from "@huggingface/transformers";

const MODEL = "onnx-community/embeddinggemma-2-ONNX";
const DIMENSIONS = 256;
type Input = { text: string; kind: "query" | "document" };
type Request = { id: number; inputs: Input[] };
let model: Awaited<ReturnType<typeof AutoModel.from_pretrained>> | null = null;
let tokenizer: Awaited<ReturnType<typeof AutoTokenizer.from_pretrained>> | null = null;
let activeDevice: "webgpu" | "wasm" | null = null;

async function load(device: "webgpu" | "wasm") {
  const config = await AutoConfig.from_pretrained(MODEL);
  Reflect.set(config, "vision_config", null);
  Reflect.set(config, "audio_config", null);
  tokenizer = await AutoTokenizer.from_pretrained(MODEL);
  model = await AutoModel.from_pretrained(MODEL, { config, device, dtype: "q4" });
  activeDevice = device;
}

function normalizedVectors(data: ArrayLike<number>, rows: number, width: number): number[][] {
  const vectors: number[][] = [];
  for (let row = 0; row < rows; row += 1) {
    const values = Array.from({ length: DIMENSIONS }, (_, index) => data[row * width + index]);
    const norm = Math.hypot(...values);
    if (!values.every(Number.isFinite) || !Number.isFinite(norm) || norm === 0) {
      throw new Error("embedding model returned an invalid vector");
    }
    vectors.push(values.map((value) => value / norm));
  }
  return vectors;
}

async function infer(inputs: Input[]): Promise<number[][]> {
  if (!model || !tokenizer) throw new Error("embedding model did not load");
  const texts = inputs.map(({ text, kind }) => kind === "query"
    ? `task: search result | query: ${text.slice(0, 4000)}`
    : `title: none | text: ${text.slice(0, 4000)}`);
  const tokens = await tokenizer(texts);
  const output = await model(tokens);
  const embedding = "sentence_embedding" in output ? output.sentence_embedding : undefined;
  if (!embedding || embedding.dims.length !== 2 || embedding.dims[0] !== inputs.length || embedding.dims[1] < DIMENSIONS) {
    throw new Error("embedding model returned an unexpected vector shape");
  }
  return normalizedVectors(embedding.data, inputs.length, embedding.dims[1]);
}

async function embed(inputs: Input[]): Promise<number[][]> {
  if (!model || !tokenizer) {
    try {
      await load("webgpu");
    } catch {
      await load("wasm");
    }
  }
  try {
    return await infer(inputs);
  } catch (error) {
    if (activeDevice !== "webgpu") throw error;
    model = null;
    tokenizer = null;
    activeDevice = null;
    await load("wasm");
    return infer(inputs);
  }
}

self.addEventListener("message", (event: MessageEvent<Request>) => {
  void embed(event.data.inputs).then(
    (vectors) => self.postMessage({ id: event.data.id, vectors }),
    (error: unknown) => self.postMessage({ id: event.data.id, error: String(error) }),
  );
});
