import json
import sys
from pathlib import Path

from faster_whisper import WhisperModel


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: transcribe.py <audio-file>", file=sys.stderr)
        return 2

    model = WhisperModel("small", device="cpu", compute_type="int8", cpu_threads=4)
    segments, _ = model.transcribe(str(Path(sys.argv[1])), language="ar", vad_filter=True)
    print(json.dumps({"text": " ".join(segment.text.strip() for segment in segments).strip()}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
