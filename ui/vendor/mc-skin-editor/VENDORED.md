# MC Skin Editor (eingebunden)

Der Skin-Editor von Pinecone ist **MC Skin Editor** von Eric Heikes, unverändert übernommen:

- Quelle: https://github.com/eheikes/mc-skin-editor (auch unter https://skincrafter.io)
- Version: 1.3.0, Commit `bcbdfad8665baa1e564796b7000f123f6f5c54b7`
- Lizenz: Apache License 2.0, siehe [LICENSE](LICENSE)

In diesem Ordner liegen `src/`, `LICENSE` und `README.md` des Projekts. Einzige Änderung:
`src/components/Viewport3D.svelte` liest die Hintergrundfarbe der 3D-Ansicht aus der CSS-Variable
`--viewport-bg` (sonst wie im Original `#2b2f36`). Die übrige Pinecone-Optik (Farben, Schriften,
Ecken, Werkzeug-Icons) kommt aus `ui/skin-editor/pinecone.css`, ohne den Editor-Code zu ändern.
`ui/public/favicon.svg` ist das Logo des Editors (für dessen „About“-Dialog).

Pinecone lädt den Editor als eigene Seite (`ui/skin-editor.html`) in einem `<iframe>`, damit seine
globalen Styles und sein `localStorage` nicht mit der App kollidieren. Die Verbindung macht
`ui/skin-editor/bridge.ts` über `postMessage` und nutzt nur Funktionen, die der Editor ohnehin
exportiert (`importSkinAsNew`, `skinStore.snapshot`, `resolutionFromDims`, `ensureInitialSkin`).

## Aktualisieren

```sh
git clone https://github.com/eheikes/mc-skin-editor /tmp/mc-skin-editor
rm -rf ui/vendor/mc-skin-editor/src
# Danach die Änderung an Viewport3D.svelte (siehe oben) erneut einspielen.
cp -r /tmp/mc-skin-editor/src /tmp/mc-skin-editor/LICENSE /tmp/mc-skin-editor/README.md ui/vendor/mc-skin-editor/
cp /tmp/mc-skin-editor/public/favicon.svg ui/public/favicon.svg
# Versionen von three und @jaames/iro in package.json an das Upstream-package.json angleichen,
# Commit oben eintragen, dann `npm run build` und den Editor in der App ausprobieren.
```
