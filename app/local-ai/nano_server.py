"""Run NanoI1 behind the local OpenAI-compatible chat endpoint."""

import json
from http.server import BaseHTTPRequestHandler, HTTPServer
import sys
from typing import Callable

MODEL_ID = "NanoI1-155M-Instruct"
REPOSITORY = "KiteFishAI/NanoI1-155M-Instruct"
REVISION = "feea2d5ec128c5bf874a7f788dfad036fc436318"
MAX_REQUEST_BYTES = 1_000_000


def model_messages(messages: list, response_format: object) -> list:
    if not isinstance(response_format, dict) or response_format.get("type") != "json_schema":
        return messages
    schema = response_format.get("schema")
    if not isinstance(schema, dict):
        raise ValueError("json_schema response format needs a schema object")
    properties = schema.get("properties", {})
    commands = properties.get("commands", {}) if isinstance(properties, dict) else {}
    items = commands.get("items", {}) if isinstance(commands, dict) else {}
    variants = items.get("oneOf", []) if isinstance(items, dict) else []
    operations = [variant["properties"]["op"]["const"] for variant in variants
                  if isinstance(variant, dict) and isinstance(variant.get("properties"), dict)
                  and isinstance(variant["properties"].get("op"), dict)
                  and isinstance(variant["properties"]["op"].get("const"), str)]
    instruction = 'Reply only with one JSON object. Include the top-level fields "commands" and "rationale".'
    if operations:
        instruction += " Command operations: " + ", ".join(operations) + "."
    instruction += " Do not add markdown or text outside the JSON."
    result = [dict(message) for message in messages]
    system = next((message for message in result if message["role"] == "system"), None)
    if system is None:
        result.insert(0, {"role": "system", "content": instruction})
    else:
        system["content"] += "\n\n" + instruction
    return result


def chat_completion(payload: object, generate: Callable) -> dict:
    if not isinstance(payload, dict) or payload.get("stream") is True:
        raise ValueError("expected a non-streaming chat completion request")

    messages = payload.get("messages")
    if not isinstance(messages, list) or not 1 <= len(messages) <= 24:
        raise ValueError("messages must contain between 1 and 24 entries")
    for message in messages:
        if not isinstance(message, dict) or message.get("role") not in {"system", "user", "assistant"}:
            raise ValueError("each message needs a supported role")
        if not isinstance(message.get("content"), str):
            raise ValueError("each message needs text content")

    max_tokens = payload.get("max_tokens", 192)
    temperature = payload.get("temperature", 0.1)
    if type(max_tokens) is not int or not 1 <= max_tokens <= 512:
        raise ValueError("max_tokens must be between 1 and 512")
    if type(temperature) not in {int, float} or not 0 <= temperature <= 2:
        raise ValueError("temperature must be between 0 and 2")

    content = generate(model_messages(messages, payload.get("response_format")), max_tokens, temperature)
    if not isinstance(content, str):
        raise RuntimeError("NanoI1 returned a non-text response")
    return {
        "id": "nanoi1-local",
        "object": "chat.completion",
        "model": MODEL_ID,
        "choices": [{"index": 0, "message": {"role": "assistant", "content": content}, "finish_reason": "stop"}],
    }


def make_handler(generate: Callable):
    class NanoHandler(BaseHTTPRequestHandler):
        def _send_json(self, status: int, payload: dict) -> None:
            body = json.dumps(payload).encode()
            self.send_response(status)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self) -> None:
            if self.path == "/health":
                self._send_json(200, {"status": "ok", "model": MODEL_ID})
            else:
                self._send_json(404, {"error": {"message": "not found"}})

        def do_POST(self) -> None:
            if self.path != "/v1/chat/completions":
                self._send_json(404, {"error": {"message": "not found"}})
                return
            try:
                size = int(self.headers.get("content-length", "0"))
                if not 0 < size <= MAX_REQUEST_BYTES:
                    raise ValueError("request body must be between 1 byte and 1 MB")
                payload = json.loads(self.rfile.read(size))
                self._send_json(200, chat_completion(payload, generate))
            except (ValueError, json.JSONDecodeError) as error:
                self._send_json(400, {"error": {"message": str(error)}})
            except Exception as error:
                print(f"NanoI1 inference failed: {error}", file=sys.stderr)
                self._send_json(500, {"error": {"message": "NanoI1 inference failed"}})

    return NanoHandler


def load_model():
    import torch
    from huggingface_hub import snapshot_download

    torch.set_num_threads(4)
    torch.set_num_interop_threads(1)
    path = snapshot_download(
        REPOSITORY,
        revision=REVISION,
        allow_patterns=["*.py", "config.json", "generation_config.json", "model.safetensors", "tokenizer.json", "tokenizer_config.json"],
    )
    sys.path.insert(0, path)
    from modeling_nanoi1 import chat, from_pretrained

    model, tokenizer = from_pretrained(path, device="cpu", dtype=torch.float32)
    return lambda messages, max_tokens, temperature: chat(
        model,
        tokenizer,
        messages,
        max_new_tokens=max_tokens,
        temperature=temperature,
    )


def main() -> None:
    server = HTTPServer(("127.0.0.1", 9931), make_handler(load_model()))
    print(f"NanoI1 CPU server on http://127.0.0.1:9931 ({MODEL_ID})")
    server.serve_forever()


if __name__ == "__main__":
    main()
