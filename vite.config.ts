import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

// Plain object form so vitest can merge this config with its own.
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
// Vite options tailored for Tauri 2 development; see
// https://v2.tauri.app/reference/config
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Tell vite to ignore watching src-tauri
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
  },
});
