# Releases

## Einmalig: Signaturschlüssel für Updates

Updates werden nur installiert, wenn sie mit dem eigenen Schlüssel signiert sind.

```sh
npx tauri signer generate -w ~/.tauri/pinecone.key
```

- Inhalt von `~/.tauri/pinecone.key` → GitHub-Secret `TAURI_SIGNING_PRIVATE_KEY`,
  das Passwort → `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. **Nie einchecken**, sicher aufbewahren:
  ohne ihn lassen sich keine Updates mehr ausliefern.
- Inhalt von `~/.tauri/pinecone.key.pub` → GitHub-Variable `PINECONE_UPDATE_PUBKEY`
  (Settings → Secrets and variables → Actions → Variables).
- Optional: `PINECONE_MSA_CLIENT_ID` (Azure-App, siehe README) und `PINECONE_CURSEFORGE_API_KEY`
  als Secrets; sie werden in die Release-Builds einkompiliert.

## Neue Version

1. Version an drei Stellen gleich setzen: `Cargo.toml` (`[workspace.package] version`),
   `src-tauri/tauri.conf.json` (`version`) und `package.json` (`version`), danach `cargo check`
   (aktualisiert `Cargo.lock`) und `npm install` (aktualisiert `package-lock.json`).
2. Committen und taggen: `git tag v0.2.0 && git push origin v0.2.0`.
3. Der Workflow „Release“ baut Windows (NSIS-.exe, .msi) und Linux (AppImage, .deb, .rpm),
   signiert die Update-Pakete und legt ein **Release als Entwurf** mit `latest.json` an.
4. Entwurf prüfen (Installer auf Windows und Linux ausprobieren), Release-Notizen ergänzen,
   veröffentlichen. Erst dann sehen installierte Launcher das Update
   (`releases/latest/download/latest.json`).

## Lokal bauen

```sh
npm ci
npm run tauri build                         # Installer für das eigene System, ohne Updater-Signatur
npm run tauri build -- --bundles deb        # nur ein Paket
```

Lokale Builds ohne `PINECONE_UPDATE_PUBKEY`/`PINECONE_UPDATE_URL` haben das Selbst-Update
abgeschaltet (Einstellungen zeigen „nicht eingerichtet“).

## Wichtig: Wo die Updates liegen

Der Updater lädt `latest.json` und die Pakete **ohne Anmeldung**. Aus einem privaten
GitHub-Repo geht das nicht. Möglichkeiten:

- Repo (oder ein separates Release-Repo) öffentlich machen, oder
- `latest.json` und die Installer auf einem eigenen Server ablegen (z. B. bei TwoGG) und
  `PINECONE_UPDATE_URL` im Workflow auf diese Adresse setzen. Die Adressen in `latest.json`
  müssen dann ebenfalls auf diesen Server zeigen.
