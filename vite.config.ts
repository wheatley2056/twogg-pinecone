import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";

// Tauri erwartet einen festen Port und lädt im Release-Build aus `dist/`.
export default defineConfig({
  root: "ui",
  // Vite-Root ist `ui/`, die Svelte-Konfiguration liegt im Projektwurzelverzeichnis.
  plugins: [svelte({ configFile: fileURLToPath(new URL("./svelte.config.js", import.meta.url)) })],
  clearScreen: false,
  server: { port: 5173, strictPort: true, watch: { ignored: ["**/src-tauri/**", "**/target/**"] } },
  build: { outDir: "../dist", emptyOutDir: true, target: "es2022" },
});
