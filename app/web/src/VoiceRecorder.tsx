import { useEffect, useRef, useState } from "react";

interface Props {
  busy: boolean;
  onTranscribe: (audio: Blob) => Promise<string>;
  onTranscript: (text: string) => void;
}

export function VoiceRecorder({ busy, onTranscribe, onTranscript }: Props) {
  const recorder = useRef<MediaRecorder | null>(null);
  const stream = useRef<MediaStream | null>(null);
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => () => {
    if (recorder.current) {
      recorder.current.onstop = null;
      recorder.current.stop();
    }
    stream.current?.getTracks().forEach((track) => track.stop());
  }, []);

  const toggle = async () => {
    setError(null);
    if (recorder.current && recording) {
      recorder.current.stop();
      return;
    }
    try {
      stream.current = await navigator.mediaDevices.getUserMedia({ audio: true });
      const chunks: Blob[] = [];
      const current = new MediaRecorder(stream.current);
      recorder.current = current;
      current.ondataavailable = (event) => { if (event.data.size > 0) chunks.push(event.data); };
      current.onstop = async () => {
        stream.current?.getTracks().forEach((track) => track.stop());
        stream.current = null;
        recorder.current = null;
        setRecording(false);
        const text = await onTranscribe(new Blob(chunks, { type: current.mimeType }));
        if (text) onTranscript(text);
      };
      current.start();
      setRecording(true);
    } catch (e) {
      stream.current?.getTracks().forEach((track) => track.stop());
      stream.current = null;
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return <>
    <button type="button" className="quiet" onClick={toggle} disabled={busy} aria-pressed={recording}>
      {recording ? "Stop recording" : "Record voice"}
    </button>
    {error && <small className="problem" role="alert">{error}</small>}
  </>;
}
