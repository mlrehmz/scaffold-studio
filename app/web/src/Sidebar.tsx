import { useEffect, useRef, useState } from "react";
import type { State } from "../../server/src/types";
import { VoiceRecorder } from "./VoiceRecorder";

interface Props {
  state: State;
  selected: string | null;
  busy: boolean;
  onTranscribe: (audio: Blob) => Promise<string>;
  onIntent: (text: string) => void;
  onPreferences: (preferences: string) => void;
  onAccept: () => void;
  onReject: () => void;
  onUndo: () => void;
  onToggleLock: (id: string, locked: boolean) => void;
}

export function Sidebar(p: Props) {
  const [text, setText] = useState("");
  const [sending, setSending] = useState<string | null>(null);
  const [view, setView] = useState<"inspector" | "scratchpad">("inspector");
  const [preferences, setPreferences] = useState(p.state.preferences);
  const conversationRef = useRef<HTMLDivElement>(null);
  const { pending, violations, design } = p.state;
  const latestMessage = p.state.conversation.at(-1);
  const box = design.box.find((b) => b.id === p.selected);
  // The Rust gatekeeper already decided; the UI only shows its verdict.
  const v = pending?.verdict;
  const problems = [...(v?.errors ?? []), ...(v?.violations.map((x) => x.message) ?? [])];

  useEffect(() => {
    const conversation = conversationRef.current;
    if (conversation) conversation.scrollTop = conversation.scrollHeight;
  }, [p.state.conversation.length, p.busy]);

  useEffect(() => {
    if (!p.busy) setSending(null);
  }, [p.busy]);

  useEffect(() => setPreferences(p.state.preferences), [p.state.preferences]);

  return (
    <aside className="sidebar">
      <div className="panel-switch" role="tablist" aria-label="Side panel">
        <button className={view === "inspector" ? "tab on" : "tab"} onClick={() => setView("inspector")}>Inspector</button>
        <button className={view === "scratchpad" ? "tab on" : "tab"} onClick={() => setView("scratchpad")}>Scratchpad</button>
      </div>
      {view === "scratchpad" && (
        <>
          <div className="scratchpad-intro"><strong>Design conversation</strong><span>Local model context is saved with this design.</span>{p.state.localModelEnabled && <small>The first request may download the 175 MB embedding model.</small>}</div>
          <div className="conversation" aria-live="polite" ref={conversationRef}>
            {p.state.conversation.length === 0 ? <p className="muted">Describe a design change. Proposals will appear here for review.</p> : p.state.conversation.map((message, index) => (
              <article className={`chat-message ${message.role}`} key={`${index}-${message.role}`}>
                <strong>{message.role === "user" ? "You" : "Scaffold Studio"}</strong>
                <p>{message.content}</p>
                {message.role === "assistant" && <div className="chat-actions">
                  {message.rationale && <details><summary>Reasoning summary</summary><p>{message.rationale}</p></details>}
                  <button className="icon-button" type="button" aria-label="Read response aloud" title="Read response aloud" disabled={!("speechSynthesis" in window)} onClick={() => {
                    window.speechSynthesis.cancel();
                    window.speechSynthesis.speak(new SpeechSynthesisUtterance(message.content));
                  }}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9v6h4l5 4V5L8 9H4Zm12.5 3a4.5 4.5 0 0 0-2.5-4.03v8.05A4.5 4.5 0 0 0 16.5 12Zm-2.5-9v2.06a7 7 0 0 1 0 13.88V21a9 9 0 0 0 0-18Z" /></svg></button>
                </div>}
              </article>
            ))}
            {sending && p.busy && (latestMessage?.role !== "user" || latestMessage.content !== sending) && <article className="chat-message user"><strong>You</strong><p>{sending}</p></article>}
            {p.busy && <p className="assistant-typing" role="status">Preparing a proposal…</p>}
          </div>
          <form onSubmit={(e) => {
            e.preventDefault();
            const prompt = text.trim();
            if (!prompt) return;
            setSending(prompt);
            p.onIntent(prompt);
            setText("");
          }}>
            <label htmlFor="scratchpad-prompt">Your prompt</label>
            <textarea id="scratchpad-prompt" value={text} onChange={(e) => setText(e.target.value)} placeholder="Describe what you want to design…" disabled={p.busy} />
            <div className="row"><button disabled={p.busy || !text.trim()}>Send for proposal</button><VoiceRecorder busy={p.busy} onTranscribe={p.onTranscribe} onTranscript={(transcript) => setText((current) => [current, transcript].filter(Boolean).join("\n"))} /></div>
          </form>
          <details className="memory-settings">
            <summary>Memory and preferences</summary>
            <p className="muted">Tell the model what you prefer. This stays in the local project folder.</p>
            <textarea aria-label="Saved preferences" value={preferences} maxLength={2000} onChange={(event) => setPreferences(event.target.value)} placeholder="For example: prefer simple modules and clear trade-offs." />
            <button type="button" className="quiet" disabled={p.busy} onClick={() => p.onPreferences(preferences)}>Save preferences</button>
          </details>
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
