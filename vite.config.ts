import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;
const page = (file: string) => fileURLToPath(new URL(file, import.meta.url));

// Multi-page: one HTML entry per window (pet-overlay / status-panel / settings).
// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Tauri dev/build tweaks:
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1430,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1431,
        }
      : undefined,
    // 3. tell Vite to ignore watching `src-tauri`
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    rollupOptions: {
      input: {
        "pet-overlay": page("pet-overlay.html"),
        "status-panel": page("status-panel.html"),
        settings: page("settings.html"),
      },
    },
  },
}));
