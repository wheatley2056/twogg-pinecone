// Startanimation: einmal abspielen, dann dem Hauptprogramm Bescheid geben (siehe src-tauri/src/splash.rs).
import { invoke } from "@tauri-apps/api/core";
import "./splash.css";

declare global {
  interface Window {
    __PINECONE_REDUCE_MOTION__?: boolean;
  }
}

/** Ende des Schriftzugs (68,75 % von 5 s) plus ein kurzer Moment auf dem Endbild. */
const ANIMATION_MS = 3700;
const STILL_MS = 1200;

const stage = document.querySelector<HTMLElement>(".stage")!;

function fit() {
  stage.style.setProperty("--scale", String(Math.min(window.innerWidth / 960, window.innerHeight / 600)));
}
fit();
window.addEventListener("resize", fit);

const still = window.__PINECONE_REDUCE_MOTION__ === true || window.matchMedia("(prefers-reduced-motion: reduce)").matches;
if (still) document.body.classList.add("still");

// Erst loslegen, wenn die Schriften da sind, sonst springt der Schriftzug.
document.fonts.ready.then(() => {
  setTimeout(() => {
    invoke("splash_finished").catch(() => {
      // Ohne Tauri (Vorschau im Browser) gibt es nichts zu melden.
    });
  }, still ? STILL_MS : ANIMATION_MS);
});
