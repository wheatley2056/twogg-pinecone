# Website

Landing Page von Pinecone, statisch, ohne Build-Schritt. Wird bei Änderungen auf `main` über
GitHub Pages veröffentlicht (`.github/workflows/pages.yml`).

- Download-Links holt die Seite beim Laden aus dem neuesten GitHub-Release
  (`api.github.com/repos/…/releases/latest`); ohne Release zeigen sie auf die Release-Seite.
- Schriften liegen in `fonts/` (OFL, aus `@fontsource`), es werden keine Google Fonts geladen.
- Screenshots in `img/` stammen aus dem Mock-Modus der App (`npm run dev`, 1280 × 800), als WebP.

Lokal ansehen: `python3 -m http.server -d site 8000`.

## Englisch und Lizenzen

- `en/index.html` ist die englische Startseite. Änderungen an `index.html` bitte dort nachziehen.
- `lizenzen.html` und `licenses.html` zeigen alle Fremdkomponenten des Launchers aus `licenses.json`.
  Die Datei erzeugt `python3 scripts/licenses.py` (aus `Cargo.lock`, `node_modules` und `ui/vendor`).
  Nach Änderungen an Abhängigkeiten und vor jedem Release neu erzeugen und mit committen.
