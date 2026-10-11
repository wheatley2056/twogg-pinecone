# Plan: Eigener Minecraft Launcher in Rust

## Context
Greenfield-Projekt (das Arbeitsverzeichnis enthält nur die LATEST-Plugin-Config, keinen Launcher-Code). Ziel: allgemeiner Launcher für beliebige Vanilla-Versionen und Mod-Loader, mit mehreren Profilen, Microsoft-Login plus Offline-Modus. GUI per Tauri mit Web-Frontend, Plattformen Windows und Linux.

Name: **TwoGG Pinecone**, Kurzname Pinecone (Crate-Präfix `pinecone-`, Binary `pinecone`). Gemeint ist der Launcher der TwoGG-Community, nicht des LATEST-Events. Logo-Idee: Pixel-Art-Tannenzapfen. Hinweis: "Pinecone" ist auch eine Vektordatenbank-Firma, bei öffentlicher Verteilung (Domain, Repo) auf Verwechslung achten, deshalb immer mit Präfix "TwoGG".

Neues Projekt außerhalb der Config: `C:\Users\wheatley\Desktop\twogg_pinecone\` (Pfad vor Start bestätigen).

## Architektur
Cargo-Workspace. Der Kern kennt keine GUI und ist auch per CLI testbar:

```
launcher/
  crates/
    core/      # Bibliothek: Manifeste, Downloads, Java, Launch, Auth, Profile
    cli/       # dünne CLI über core (Entwicklung, Tests, Debugging)
  src-tauri/   # Tauri-Backend: Commands + Events, ruft core auf
  ui/          # Frontend (Svelte + TS, Vite)
```

Module in `core`:
- `manifest` – Mojang `version_manifest_v2.json`, Version-JSON (libraries, rules, arguments, assetIndex, javaVersion), `inheritsFrom`-Merge für Loader.
- `download` – paralleler Downloader (tokio + reqwest), SHA-1-Prüfung, Retry, Fortschritts-Events, Cache-Wiederverwendung.
- `libraries` / `natives` – Rule-Auswertung (os/arch/features), Classpath-Bau, Natives entpacken.
- `assets` – Asset-Index und Objekte von `resources.download.minecraft.net/<hh>/<hash>`.
- `java` – installierte Java suchen, sonst Mojangs Java-Runtime-Manifest bzw. Adoptium laden; passende Major-Version aus `javaVersion`.
- `launch` – Argumentbau (JVM + Game, Platzhalter-Substitution), Prozess starten, stdout/stderr streamen.
- `auth` – Microsoft Device-Code-Flow → Xbox Live → XSTS → Minecraft-Services → Profil; Refresh-Token im OS-Keyring (`keyring`); Offline-Account mit UUID v3 aus `OfflinePlayer:<name>`.
- `loader` – Fabric (meta.fabricmc.net Profile-JSON) und Quilt zuerst; Forge/NeoForge später (Installer-Prozessoren sind aufwendig).
- `profiles` – Instanzen als Ordner mit `profile.json` (Version, Loader, JVM-Args, RAM, Java-Pfad), je eigenes `.minecraft`-Verzeichnis.
- `paths` / `config` – `directories`-Crate, ein Datenverzeichnis pro OS.

Crates: tokio, reqwest (rustls), serde/serde_json, sha1, zip, thiserror/anyhow, tracing, directories, keyring, futures, uuid.

## Phasen
1. **Gerüst + Kern ohne Login**: Workspace, Manifest laden, Version-JSON parsen, Libraries/Assets/Client laden, Offline-Start einer Vanilla-Version über CLI. Meilenstein: Minecraft startet.
2. **Java-Management + Rules/Natives sauber**: automatisches Java, ältere Versionen (Natives, LWJGL-Varianten, Legacy-Argumente `minecraftArguments`), Linux/Windows-Pfade.
3. **Microsoft-Login**: Device-Code-Flow, Token-Refresh, Keyring-Ablage, Profil/Skin-Abruf.
4. **Profile + Fabric/Quilt**: Instanzverwaltung, Loader-Auswahl, `inheritsFrom`-Merge.
5. **Tauri-GUI**: Profilliste, Play-Button, Fortschrittsbalken, Log-Fenster, Account-Wechsel, Einstellungen (RAM, Java, JVM-Args). Core-Events via Tauri-Event-Channel.
6. **Dashboard + Personalisierung**: Spielzeit-Tracking, `servers.dat`-Parser, Quick-Join, Server-Status, Themes, Widget-Layout.
7. **Modpacks**: Modrinth-Import/-Suche, CurseForge-Import/-Suche, Pack erstellen/exportieren, Update mit Snapshot und Settings-Erhalt.
7b. **Pack-Sharing**: Share-Code/Deep-Link, Datei-Export-Fallback, dann der Share-Dienst, danach abonnierte Packs mit Update-Hinweis.
8. **Screenshots + Skins**: Galerie mit Watcher, Skin-Upload/Bibliothek/Capes, dann Skin-Editor mit 3D-Vorschau.
9. **Politur**: Logfile-Export, Self-Update, Installer/Bundles (MSI/NSIS, AppImage/deb).
10. **Optional**: Forge/NeoForge, eigenes TwoGG-Server-Profil mit Auto-Update (z. B. Horizons Unbound).

## Feature-Module (Erweiterung)
Zusätzliche `core`-Module, jeweils mit eigenem Tauri-Command-Satz und UI-Seite:

**Skins (`skins`)**
- Ändern: `POST api.minecraftservices.com/minecraft/profile/skins` (PNG-Upload, Variante classic/slim oder per URL), Cape aktivieren/deaktivieren über `/minecraft/profile/capes/active`. Braucht Online-Account mit Profil-Scope.
- Skin-Bibliothek lokal (PNG + Name + Variante), Import per Datei, Spielername oder URL.
- Editor im Frontend: 64x64-Pixel-Canvas (Pinsel, Fill, Pipette, Layer Basis/Overlay, Undo), 3D-Vorschau mit three.js, Slim/Classic-Umschalter. Offline-Accounts nutzen die Skins nur lokal.

**Modpack-Verwaltung (`packs`)**
- Import/Installation: Modrinth `.mrpack` (`modrinth.index.json` + `overrides/`) und CurseForge-Zip (`manifest.json`); Modrinth-API v2 und CurseForge-API zum Suchen/Installieren direkt aus der UI. CurseForge braucht einen API-Key, und Mods mit gesperrter Drittanbieter-Distribution (kein Download-URL) werden als „manuell herunterladen“ markiert.
- Eigene Packs erstellen: Profil + Mod-Liste (Quelle, Projekt-ID, Version, Hash) als `pack.lock.json`; Export als `.mrpack`; Abhängigkeitsprüfung und Loader/MC-Kompatibilität.
- Updates mit Settings-Erhalt: vor dem Update Snapshot des Profils (Backup), dann Drei-Wege-Vergleich per Hash: vom Pack gelieferte Dateien werden ersetzt, vom Nutzer geänderte Config-Dateien (`options.txt`, `config/*`, `servers.dat`) bleiben, selbst hinzugefügte Mods bleiben, entfernte Pack-Mods werden gelöscht. Konflikte werden in der UI zur Auswahl angezeigt. Rollback aus dem Snapshot.

**Modpack-Sharing mit Freunden (`share`)**
- Teilen per Klick: „Teilen“ am Profil erzeugt einen kurzen **Share-Code** und einen Link `pinecone://pack/<code>`. Der Freund fügt den Code in Pinecone ein (oder öffnet den Link, Deep-Link-Handler über Tauri), sieht eine Vorschau (Name, MC-Version, Loader, Mod-Anzahl, Größe) und installiert mit einem Klick.
- Inhalt: nur das `pack.lock.json` (Mods als Modrinth-/CurseForge-IDs + Hash) plus die geteilten `overrides` (Configs, Resourcepacks, Shader; Opt-in pro Ordner, `servers.dat` und Account-Daten nie). Mod-Dateien werden nicht selbst gehostet, sondern vom Original-Anbieter geladen (Lizenz/Distribution). Nicht-öffentliche Mods (manuell/lokal) werden als „fehlt“ markiert.
- Transport, zwei Wege: (1) kleiner **Share-Dienst** (REST, z. B. unter einer TwoGG-Domain; Upload ohne Account, Codes mit Ablauf, Größenlimit) für den Normalfall; (2) **Datei-Export/-Import** `.mrpack` als Fallback ohne Server.
- **Abonnierte Packs**: Der Ersteller kann eine neue Version veröffentlichen, abonnierte Freunde sehen im Dashboard „Update verfügbar“ und nutzen den normalen Update-Ablauf (Snapshot, eigene Settings bleiben).
- Sicherheit: Hash-Prüfung aller Downloads, Hinweis beim Import fremder Packs („Inhalte von Dritten“), keine automatische Ausführung, nur Dateien in die Profil-Sandbox schreiben (Path-Traversal-Schutz beim Entpacken von `overrides`).

**Screenshots (`screenshots`)**
- Dateiüberwachung (`notify`) auf `<profil>/screenshots`, Thumbnail-Cache, Galerie über alle Profile mit Filter nach Profil/Datum, Aktionen: kopieren, löschen, Ordner öffnen, Datei teilen.

**Dashboard (`stats`, `servers`)**
- Zuletzt gespielt: Profil, Zeitpunkt und Spielzeit (Start/Ende des Prozesses mitschreiben, in `stats.json` bzw. SQLite).
- Letzter Server mit Quick-Join: Auswertung von `servers.dat` (NBT) und Log; Start mit `--quickPlayMultiplayer <host>` ab 1.20, davor `--server/--port`. Server-Status (MOTD, Spieler, Ping) per Server List Ping.
- Widgets: Zuletzt gespielt, Quick-Join, neueste Screenshots, Modpack-Updates verfügbar.

**Personalisierung (`settings` + UI)**
- Themes über CSS-Variablen (hell/dunkel/System, Akzentfarbe, Presets), Hintergrundbild, Anordnung der Dashboard-Widgets, Profil-Icons/Farben, Sprache (de/en), alles in `settings.json` und als Theme-Datei exportierbar.

## Risiken / offene Punkte
- **Share-Dienst**: braucht einen kleinen Server (Hosting, Missbrauchsschutz, Limits). Offene Entscheidung: selbst hosten (z. B. bei TwoGG) oder zunächst nur Datei-Sharing. Empfehlung: Datei-Sharing und Deep-Links zuerst, Dienst danach.
- **Azure-App-Registrierung**: Für Minecraft-Auth braucht es eine eigene Client-ID; neue Apps müssen bei Mojang freigeschaltet werden (Antragsformular). Bis dahin Phase 3 mit Offline-Modus testen.
- Offline-Modus nur für Tests/LAN; echte Server verlangen Online-Accounts.
- Forge/NeoForge-Installer sind der größte Einzelaufwand, deshalb nach hinten gestellt.
- Mojang-Marke/EULA: Launcher darf keine Mojang-Assets weiterverbreiten, nur von deren Servern laden; Name/Logo nicht wie offiziell gestalten.

## Verifikation
- Unit-Tests: Rule-Auswertung, Argument-Substitution, Offline-UUID, `inheritsFrom`-Merge gegen echte JSONs (1.8, 1.12, 1.20.x, aktuelle Version).
- Integration: CLI startet Vanilla 1.21.x und eine Legacy-Version (1.8.9) bis ins Hauptmenü, auf Windows und Linux (CI-Matrix / VM).
- Fabric-Profil mit einem Modrinth-Mod startet und lädt den Mod (im Log sichtbar).
- Login: Device-Code-Flow mit eigenem Account, Neustart ohne erneuten Login (Refresh).
- GUI: `cargo tauri dev`, Spielstart per Klick, Fortschritt und Logs sichtbar; `cargo tauri build` erzeugt Installer.
- `cargo clippy` und `cargo test` grün in CI.

## Erster Umsetzungsschritt nach Freigabe
Workspace anlegen (`core`, `cli`), `manifest`- und `download`-Module mit Tests, dann CLI-Befehl `pinecone install <version>` und `pinecone run <version> --offline <name>`.
