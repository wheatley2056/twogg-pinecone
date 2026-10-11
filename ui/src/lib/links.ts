import { openUrl } from "@tauri-apps/plugin-opener";

/** Klicks auf Links in Server-HTML (Neuigkeiten, Beschreibungen) im Browser öffnen statt im App-Fenster. */
export function externalLinks(node: HTMLElement) {
  const onClick = (e: MouseEvent) => {
    const a = (e.target as HTMLElement).closest("a");
    if (!a) return;
    e.preventDefault();
    const href = a.getAttribute("href") ?? "";
    if (/^https?:\/\//i.test(href)) openUrl(href).catch(() => {});
  };
  node.addEventListener("click", onClick);
  return { destroy: () => node.removeEventListener("click", onClick) };
}
