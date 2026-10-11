# Website

Landing Page von Pinecone, statisch, ohne Build-Schritt. Wird bei Änderungen auf `main` über
GitHub Pages veröffentlicht (`.github/workflows/pages.yml`).

- Download-Links holt die Seite beim Laden aus dem neuesten GitHub-Release
  (`api.github.com/repos/…/releases/latest`); ohne Release zeigen sie auf die Release-Seite.
- Schriften liegen in `fonts/` (OFL, aus `@fontsource`), es werden keine Google Fonts geladen.
- Screenshots in `img/` stammen aus dem Mock-Modus der App (`npm run dev`, 1280 × 800), als WebP.

Lokal ansehen: `python3 -m http.server -d site 8000`.
