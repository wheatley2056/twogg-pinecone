# Releases

## Einmalig: Signaturschlüssel für Updates

Updates werden nur installiert, wenn sie mit dem eigenen Schlüssel signiert sind.

```sh
npx tauri signer generate -w ~/.tauri/pinecone.key
```

- Inhalt von `~/.tauri/pinecone.key` → GitHub-Secret (in der Umgebung `TAURI` oder als Repository-Secret) `TAURI_SIGNING_PRIVATE_KEY`,
  das Passwort → `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. **Nie einchecken**, sicher aufbewahren:
  ohne ihn lassen sich keine Updates mehr ausliefern.
- Inhalt von `~/.tauri/pinecone.key.pub` → GitHub-Variable `PINECONE_UPDATE_PUBKEY`
  (Settings → Secrets and variables → Actions → Variables).
- Optional: `PINECONE_MSA_CLIENT_ID` (Azure-App, siehe README) als Secret; sie wird in die
  Release-Builds einkompiliert.
- **Kein** `PINECONE_CURSEFORGE_API_KEY`: Die CurseForge-API-Bedingungen verbieten, den Schlüssel
  weiterzugeben. Er liegt nur auf dem Pinecone-Server, der Launcher fragt über dessen Proxy. Ein
  altes Secret dieses Namens bitte in den GitHub-Einstellungen löschen.

## Neue Version

Versionsschema: `0.9.x` während der Beta, danach `1.0.0` usw. Keine Vorab-Kennungen wie `-beta.1`
in der Versionsnummer: Der MSI-Installer akzeptiert nur Zahlen.

**Update-Kanäle:** In den Einstellungen wählt man „Release“ oder „Beta“. Der Launcher sucht in den
GitHub-Releases dieses Repos die neueste veröffentlichte Version des Kanals:

- als **Pre-release** veröffentlicht → nur für den Kanal Beta,
- als normales Release (**„Set as the latest release“**) → für beide Kanäle.

Entwürfe sieht der Updater nie. Launcher 0.9.0 und 0.9.1 kennen die Kanäle noch nicht und schauen
nur auf `releases/latest`; sie bekommen ein Update erst, wenn ein normales Release veröffentlicht ist.

1. `CHANGELOG.md` um die neue Version ergänzen.
2. Version an drei Stellen gleich setzen: `Cargo.toml` (`[workspace.package] version`),
   `src-tauri/tauri.conf.json` (`version`) und `package.json` (`version`), danach `cargo check`
   (aktualisiert `Cargo.lock`) und `npm install` (aktualisiert `package-lock.json`).
3. Committen und taggen: `git tag v0.9.1 && git push origin v0.9.1`.
4. Der Workflow „Release“ baut Windows (NSIS-.exe, .msi) und Linux (AppImage, .deb, .rpm),
   signiert die Update-Pakete und legt ein **Release als Entwurf** mit `latest.json` an.
5. Entwurf prüfen (Installer auf Windows und Linux ausprobieren), Release-Notizen ergänzen und
   veröffentlichen: für den Beta-Kanal mit „Set as a pre-release“, für alle als normales Release.
   Erst dann sehen installierte Launcher das Update.

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
- `latest.json` und die Installer auf einem eigenen Server ablegen (z. B. bei TwoGG) und die
  GitHub-Variable `PINECONE_UPDATE_URL` auf diese Adresse setzen (der Workflow nimmt sie statt der
  Releases dieses Repos; `{channel}` in der Adresse wird zu `release` bzw. `beta`). Die Adressen in `latest.json`
  müssen dann ebenfalls auf diesen Server zeigen.
