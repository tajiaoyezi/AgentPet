import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { onHealth, type HealthPayload } from "../health";

export default function Settings() {
  const [last, setLast] = useState<HealthPayload | null>(null);

  useEffect(() => {
    const unlisten = onHealth((p) => setLast(p));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <main className="page">
      <h1>AgentPet 设置</h1>
      <p className="muted">
        M0 骨架占位。Adapter / 宠物 / 通知 / 安全 等设置将在后续里程碑接入。
      </p>
      <section className="card">
        <h2>事件总线自检</h2>
        <button onClick={() => void invoke("health")}>触发 health</button>
        <p>最近 health：{last ? `#${last.tick}` : "（未收到）"}</p>
      </section>
    </main>
  );
}
