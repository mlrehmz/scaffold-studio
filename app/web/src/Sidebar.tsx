import { useState } from "react";
import type { State } from "../../server/src/types";
import { VoiceRecorder } from "./VoiceRecorder";

interface Props {
  state: State;
  selected: string | null;
  busy: boolean;
  onTranscribe: (audio: Blob) => Promise<string>;
  onIntent: (text: string) => void;
  onAccept: () => void;
  onReject: () => void;
  onUndo: () => void;
  onToggleLock: (id: string, locked: boolean) => void;
}

export function Sidebar(p: Props) {
  const [text, setText] = useState("");
  const [view, setView] = useState<"inspector" | "scratchpad">("inspector");
  const [messages, setMessages] = useState<string[]>([]);
  const { pending, violations, design } = p.state;
  const box = design.box.find((b) => b.id === p.selected);
  // The Rust gatekeeper already decided; the UI only shows its verdict.
  const v = pending?.verdict;
  const problems = [...(v?.errors ?? []), ...(v?.violations.map((x) => x.message) ?? [])];

  return (
    <aside className="sidebar">
      <div className="panel-switch" role="tablist" aria-label="Side panel">
        <button className={view === "inspector" ? "tab on" : "tab"} onClick={() => setView("inspector")}>Inspector</button>
        <button className={view === "scratchpad" ? "tab on" : "tab"} onClick={() => setView("scratchpad")}>Scratchpad</button>
      </div>
      {view === "scratchpad" && (
        <>
          <div className="scratchpad-intro"><strong>Design conversation</strong><span>Voice is transcribed locally. Review the text before sending.</span></div>
          <div className="conversation" aria-live="polite">
            {messages.length === 0 ? <p className="muted">Describe a design change. Proposals will appear here for review.</p> : messages.map((message, index) => <p className="message" key={index}>{message}</p>)}
          </div>
          <form onSubmit={(e) => {
            e.preventDefault();
            const prompt = text.trim();
            if (!prompt) return;
            setMessages((items) => [...items, prompt]);
            p.onIntent(prompt);
            setText("");
          }}>
            <label htmlFor="scratchpad-prompt">Your prompt</label>
            <textarea id="scratchpad-prompt" value={text} onChange={(e) => setText(e.target.value)} placeholder="Describe what you want to design…" disabled={p.busy} />
            <div className="row"><button disabled={p.busy || !text.trim()}>Send for proposal</button><VoiceRecorder busy={p.busy} onTranscribe={p.onTranscribe} onTranscript={(transcript) => setText((current) => [current, transcript].filter(Boolean).join("\n"))} /></div>
          </form>
          {p.state.pending && <section className={`panel ${v?.ok ? "ok" : "blocked"}`}><h2>Proposal review</h2><p>{p.state.pending.proposal.rationale}</p><pre className="proposal-commands">{JSON.stringify(p.state.pending.proposal.commands, null, 2)}</pre>{problems.map((m) => <p key={m} className="problem">{m}</p>)}<div className="row"><button onClick={p.onAccept} disabled={!v?.ok || p.busy}>Accept</button><button className="quiet" onClick={p.onReject} disabled={p.busy}>Reject</button></div></section>}
        </>
      )}
      {view === "inspector" && <>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (text.trim()) p.onIntent(text.trim());
          setText("");
        }}
      >
        <label htmlFor="intent">What should change?</label>
        <input id="intent" value={text} onChange={(e) => setText(e.target.value)} placeholder="add payments" disabled={p.busy} />
      </form>

      {pending && (
        <section className={`panel ${v?.ok ? "ok" : "blocked"}`}>
          <h2>Proposal</h2>
          <p>{pending.proposal.rationale}</p>
          <ul className="cmds">
            {pending.proposal.commands.map((c, i) => (
              <li key={i}><code>{c.op}</code></li>
            ))}
          </ul>
          {v?.questions.map((q) => <p key={q} className="question">{q}</p>)}
          {problems.map((m) => <p key={m} className="problem">{m}</p>)}
          <div className="row">
            <button onClick={p.onAccept} disabled={!v?.ok || p.busy}>Accept</button>
            <button className="quiet" onClick={p.onReject} disabled={p.busy}>Reject</button>
          </div>
        </section>
      )}

      {violations.length > 0 && (
        <section className="panel blocked">
          <h2>Problems in the design</h2>
          {violations.map((x) => <p key={x.message} className="problem">{x.message}</p>)}
        </section>
      )}

      {box && (
        <section className="panel">
          <h2>{box.id}</h2>
          <p>{box.responsibility}</p>
          {box.members.length > 0 && <ul>{box.members.map((m) => <li key={m}><code>{m}</code></li>)}</ul>}
          <button className="quiet" onClick={() => p.onToggleLock(box.id, !box.locked)} disabled={p.busy}>
            {box.locked ? "Unlock box" : "Lock box"}
          </button>
        </section>
      )}

      <div className="row bottom">
        <button className="quiet" onClick={p.onUndo} disabled={!p.state.canUndo || p.busy}>Undo</button>
      </div>
      </>}
    </aside>
  );
}
