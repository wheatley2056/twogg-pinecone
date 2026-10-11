import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

// Im normalen Browser (ohne Tauri) während der Entwicklung mit Beispieldaten arbeiten.
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
  await import("./lib/mock");
}

export default mount(App, { target: document.getElementById("app")! });
