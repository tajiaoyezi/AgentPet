import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { onHealth, type HealthPayload } from "../health";

export default function StatusPanel() {
  const [last, setLast] = useState<HealthPayload | null>(null);

  useEffect(() => {
    const unlisten = onHealth((p) => setLast(p));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <main className="page">
      <h1>AgentPet 状态面板</h1>
      <p className="muted">M0 骨架占位。多 Agent 会话与事件列表将在后续里程碑接入。</p>
      <section className="card">
        <h2>事件总线自检</h2>
        <button onClick={() => void invoke("health")}>触发 health</button>
        <p>
          最近 health：
          {last
            ? `#${last.tick} @ ${new Date(last.ts_ms).toLocaleTimeString()}`
            : "（未收到）"}
        </p>
      </section>
    </main>
  );
}
