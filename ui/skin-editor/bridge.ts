// Startet den unveränderten MC Skin Editor (ui/vendor/mc-skin-editor) und verbindet ihn per
// postMessage mit Pinecone. Nur Funktionen, die der Editor selbst exportiert, werden genutzt.

import { mount } from "svelte";
import "../vendor/mc-skin-editor/src/app.css";
import App from "../vendor/mc-skin-editor/src/App.svelte";
import { createNewSkin, ensureInitialSkin, importSkinAsNew } from "../vendor/mc-skin-editor/src/lib/stores/savedSkins";
import { skinStore } from "../vendor/mc-skin-editor/src/lib/stores/skin";
import { resolutionFromDims } from "../vendor/mc-skin-editor/src/lib/skin/resolutions";
import { dataUrlToImageData, imageDataToDataUrl } from "../vendor/mc-skin-editor/src/lib/png";
import { SOURCE, type FromEditor, type ToEditor, type Unsourced } from "./protocol";

const target = document.getElementById("app");
if (target == null) throw new Error("Missing #app element");
mount(App, { target });

function reply(message: Unsourced<FromEditor>) {
  window.parent.postMessage({ source: SOURCE, ...message }, window.location.origin);
}

async function handle(msg: ToEditor) {
  switch (msg.type) {
    case "load": {
      const imageData = await dataUrlToImageData(msg.dataUrl);
      const resolution = resolutionFromDims(imageData.width, imageData.height);
      if (resolution == null) throw new Error(`Unsupported skin size ${imageData.width}×${imageData.height}`);
      importSkinAsNew(msg.name, resolution, msg.model, imageData);
      break;
    }
    case "new":
      createNewSkin(msg.name, "standard", msg.model);
      break;
    case "export": {
      const snap = skinStore.snapshot();
      const imageData = new ImageData(new Uint8ClampedArray(snap.pixels), snap.width, snap.height);
      reply({ type: "png", dataUrl: imageDataToDataUrl(imageData), model: snap.model, width: snap.width, height: snap.height });
      break;
    }
  }
}

window.addEventListener("message", (e: MessageEvent) => {
  // Nur Nachrichten der eigenen App (gleicher Ursprung, Elternfenster) annehmen.
  if (e.origin !== window.location.origin || e.source !== window.parent) return;
  const msg = e.data as ToEditor;
  if (msg?.source !== SOURCE) return;
  handle(msg).catch((err: unknown) => reply({ type: "error", message: err instanceof Error ? err.message : String(err) }));
});

// Erst melden, wenn der Editor seinen Startzustand geladen hat; sonst könnte er einen
// gerade übergebenen Skin wieder mit dem zuletzt bearbeiteten überschreiben.
ensureInitialSkin().then(() => reply({ type: "ready" }));
