import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface HealthPayload {
  ok: boolean;
  tick: number;
  ts_ms: number;
}

export const HEALTH_EVENT = "agentpet://health";

/** Subscribe to the backend health broadcast. Returns the unlisten handle. */
export function onHealth(
  cb: (payload: HealthPayload) => void,
): Promise<UnlistenFn> {
  return listen<HealthPayload>(HEALTH_EVENT, (event) => cb(event.payload));
}
