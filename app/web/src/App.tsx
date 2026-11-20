import { useEffect, useState } from "react";
import type { State } from "../../server/src/types";
import type { ProcessView } from "./api";
import { api } from "./api";
import { Canvas } from "./Canvas";
import { Sidebar } from "./Sidebar";

type Tab = "design" | "process";

export function App() {
  const [state, setState] = useState<State | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("design");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [processView, setProcessView] = useState<ProcessView | null>(null);

  // Run an API call, store the new state, show errors in the top bar.
  const run = async (call: () => Promise<State>) => {
    setBusy(true);
    try {
      setState(await call());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => { run(api.state); }, []);

  useEffect(() => {
    if (tab !== "process") return;
    api.process().then((view) => {
      setProcessView(view);
      setError(null);
    }).catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, [tab]);

  if (!state) {
    return <div className="boot">{error ?? "Loading design…"}</div>;
  }

  const crumbs: { id: string; label: string }[] = [];
  let cursor = open;
  while (cursor) {
    const box = state.design.box.find((item) => item.id === cursor);
    if (!box) break;
    crumbs.unshift({ id: box.id, label: box.id });
    cursor = box.parent ?? null;
  }

  return (
    <div className="app">
      <header>
        <h1>Scaffold Studio <span>{state.design.project.name}</span></h1>
        <nav>
          <button className={tab === "design" ? "tab on" : "tab"} onClick={() => setTab("design")}>Design</button>
          <button className={tab === "process" ? "tab on" : "tab"} onClick={() => setTab("process")}>Process</button>
        </nav>
        {error && <p className="error">{error}</p>}
      </header>
      <main>
        {tab === "design" ? (
          <div className="canvas">
            <div className="breadcrumbs">
              <button onClick={() => setOpen(null)}>top</button>
              {crumbs.map((crumb) => <span key={crumb.id}> › <button onClick={() => setOpen(crumb.id)}>{crumb.label}</button></span>)}
            </div>
            <Canvas
              state={state}
              selected={selected}
              onSelect={setSelected}
              open={open}
              onOpen={setOpen}
              onPositionSave={(id, x, y) => run(() => api.savePosition(id, x, y))}
            />
          </div>
        ) : (
          <section className="process-list">
            {!processView ? <p>Loading process…</p> : processView.process.steps.map((step, index) => {
              const lints = processView.lints.filter((lint) => lint.path === `step:${step.id}`);
              const notePath = `steps/${String(index + 1).padStart(2, "0")}-${step.id}.md`;
              const tokens = processView.note_tokens.find(([path]) => path === notePath)?.[1] ?? 0;
              return <article className={`process-step ${lints.some((lint) => lint.severity === "error") ? "bad" : ""}`} key={step.id}>
                <h2>{index + 1}. {step.id}</h2>
                <p>{step.trigger}</p>
                {step.finish.map((item) => <p key={item}>✓ {item}</p>)}
                <div className="token-track"><div className="token-bar" style={{ width: `${Math.min(tokens / 4, 200)}px` }} /></div>
                <small>{tokens} estimated note tokens</small>
                {lints.map((lint, i) => <p className={lint.severity === "warning" ? "warning" : "problem"} key={`${lint.check}-${i}`}>{lint.message}</p>)}
              </article>;
            })}
            {processView?.lints.filter((lint) => !lint.path.startsWith("step:")).map((lint, i) => <p className={lint.severity === "warning" ? "warning" : "problem"} key={`${lint.check}-${i}`}>{lint.message}</p>)}
          </section>
        )}
        <Sidebar
          state={state}
          selected={selected}
          busy={busy}
          onIntent={(t) => run(() => api.intent(t))}
          onAccept={() => run(api.accept)}
          onReject={() => run(api.reject)}
          onUndo={() => run(api.undo)}
          onToggleLock={(id, locked) => run(() => api.setLocked(id, locked))}
        />
      </main>
    </div>
  );
}
