import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { onHealth } from "../health";

export default function PetOverlay() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const ctx = canvasRef.current?.getContext("2d");
    if (!ctx) return;
    // Placeholder pet; the real Codex spritesheet renderer arrives in M1.
    ctx.clearRect(0, 0, 192, 208);
    ctx.fillStyle = "#7c5cff";
    ctx.beginPath();
    ctx.arc(96, 92, 56, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#ffffff";
    ctx.font = "16px system-ui, sans-serif";
    ctx.textAlign = "center";
    ctx.fillText("AgentPet", 96, 178);
  }, []);

  useEffect(() => {
    const unlisten = onHealth((p) => setTick(p.tick));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <div
      className="overlay-root"
      title="拖动移动 · 双击触发 health"
      onMouseDown={(e) => {
        if (e.buttons === 1) void getCurrentWindow().startDragging();
      }}
      onDoubleClick={() => void invoke("health")}
    >
      <canvas ref={canvasRef} width={192} height={208} />
      {tick > 0 && <span className="overlay-badge">#{tick}</span>}
    </div>
  );
}
