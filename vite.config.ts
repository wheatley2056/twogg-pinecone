import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";

// Tauri erwartet einen festen Port und lädt im Release-Build aus `dist/`.
export default defineConfig({
  root: "ui",
  // Vite-Root ist `ui/`, die Svelte-Konfiguration liegt im Projektwurzelverzeichnis.
  plugins: [svelte({ configFile: fileURLToPath(new URL("./svelte.config.js", import.meta.url)) })],
  // Versionsanzeige im „About“-Dialog des eingebundenen Skin-Editors (siehe ui/vendor/mc-skin-editor).
  define: { APP_VERSION: JSON.stringify("1.3.0") },
  clearScreen: false,
  server: { port: 5173, strictPort: true, watch: { ignored: ["**/src-tauri/**", "**/target/**"] } },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    target: "es2022",
    // three.js für den Skin-Editor ist allein gut 500 kB; der Editor lädt es erst bei Bedarf.
    chunkSizeWarningLimit: 700,
    // Weitere Seiten: der eingebettete Skin-Editor (läuft im iframe) und die Startanimation.
    rolldownOptions: {
      input: {
        main: fileURLToPath(new URL("./ui/index.html", import.meta.url)),
        skinEditor: fileURLToPath(new URL("./ui/skin-editor.html", import.meta.url)),
        splash: fileURLToPath(new URL("./ui/splash.html", import.meta.url)),
      },
    },
  },
});
