import unittest

from nano_server import MODEL_ID, chat_completion


class NanoServerTests(unittest.TestCase):
    def test_chat_completion_forwards_messages_and_returns_openai_shape(self):
        received = []

        def generate(messages, max_tokens, temperature):
            received.extend((messages, max_tokens, temperature))
            return 'Hi! What would you like to design?'

        response = chat_completion(
            {
                "messages": [
                    {"role": "system", "content": "Return a short answer."},
                    {"role": "user", "content": "hi"},
                ],
                "max_tokens": 64,
                "temperature": 0,
            },
            generate,
        )

        self.assertEqual(received[0][-1]["content"], "hi")
        self.assertEqual(received[1:], [64, 0])
        self.assertEqual(response["model"], MODEL_ID)
        self.assertEqual(response["choices"][0]["message"]["content"], "Hi! What would you like to design?")

    def test_chat_completion_rejects_invalid_messages(self):
        called = False

        def generate(_messages, _max_tokens, _temperature):
            nonlocal called
            called = True
            return "unexpected"

        with self.assertRaises(ValueError):
            chat_completion({"messages": [{"role": "tool", "content": "run code"}]}, generate)
        self.assertFalse(called)

    def test_chat_completion_passes_json_schema_instructions_to_model(self):
        received = []

        def generate(messages, _max_tokens, _temperature):
            received.extend(messages)
            return '{"commands": [], "rationale": "Hello."}'

        chat_completion(
            {
                "messages": [{"role": "user", "content": "hi"}],
                "response_format": {"type": "json_schema", "schema": {"type": "object", "required": ["commands", "rationale"]}},
            },
            generate,
        )

        self.assertEqual(received[0]["role"], "system")
        self.assertIn('"commands"', received[0]["content"])
        self.assertIn("JSON", received[0]["content"])


if __name__ == "__main__":
    unittest.main()
