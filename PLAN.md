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
7c. **Versionswechsel + Sync-Packs**: Profil/Modpack auf andere Minecraft-Version oder anderen Loader umziehen (Mods automatisch passend suchen), dann Sync-Packs (Vorlagen und schaltbare Zusatz-Inhalte je Profil). Wechsel auf Forge/NeoForge erst mit deren Loader-Unterstützung (siehe 10, dafür vorziehen).
8. **Screenshots + Skins**: Galerie mit Watcher, Skin-Upload/Bibliothek/Capes, dann Skin-Editor mit 3D-Vorschau.
9. **Politur**: Logfile-Export, Self-Update, Installer/Bundles (MSI/NSIS, AppImage/deb).
10. **Optional**: Forge/NeoForge, eigenes TwoGG-Server-Profil mit Auto-Update (z. B. Horizons Unbound).
11. **Redesign**: Neue Oberfläche nach dem Pinecone-Design-System (`docs/design/`), in Schritten 11a–11h (siehe unten).

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

**Versionswechsel (`migrate`)**
- Am Profil „Version wechseln …“: Ziel-Minecraft-Version und Loader (Vanilla, Fabric, Quilt, später Forge/NeoForge) wählen. Standard ist eine Kopie als neues Profil, auf Wunsch im selben Profil mit Snapshot (Rollback wie bei Pack-Updates).
- Stammt das Profil aus einem Modpack, zuerst prüfen, ob das Pack selbst eine Version für das Ziel hat; die wird bevorzugt angeboten (Profil bleibt dann mit Pack-Updates verbunden).
- Sonst Inhalt für Inhalt: Modrinth-Mods über `POST /v2/version_files/update` (Hash + Ziel-Loader + Ziel-Version) bzw. Projekt-Versionen, CurseForge über `files` mit Versions-/Loader-Filter. Gewählt wird die neueste stabile passende Version, Abhängigkeiten werden mit aufgelöst (wie bei `content install`).
- Loader-Wechsel mit Ersatzliste bekannter Gegenstücke, z. B. Fabric API ↔ Quilted Fabric API, Sodium ↔ Embeddium/Sodium (NeoForge), Iris ↔ Oculus; Fabric-Mods laufen unter Quilt meist direkt (bevorzugt Quilt-Version, sonst Fabric-Version). Eigene Mods ohne Quelle per Hash suchen, sonst als „nicht übertragbar“ markieren.
- Ressourcenpakete und Shader bleiben (Hinweis bei abweichendem `pack_format`), Configs werden übernommen, Welten nur mit Warnung (Upgrade ist einseitig, Downgrade kann Welten beschädigen; deshalb vorher gesichert).
- Vor dem Umzug eine Vorschau wie beim Pack-Update: ✓ gefunden (alt → neu), ↺ Ersatz gefunden, ✗ nicht verfügbar. Bei ✗ entscheidet der Nutzer: ohne den Mod weitermachen, deaktiviert mitnehmen oder abbrechen. Optional „auf Updates warten“: Pinecone merkt sich fehlende Mods und meldet, sobald es sie für die Version gibt.
- Abgeleitete Profile (Pack-Inhalte, aber eigene Version) zeigen das im Pack-Hinweis an und bekommen keine Pack-Updates mehr, außer das Pack bietet später die Zielversion an.

**Sync-Packs und Vorlagen (`syncpacks`)**
- Eigene, benannte Sammlungen von Inhalten: Mods, Ressourcenpakete, Shader, Config-Dateien und Standard-Einstellungen (z. B. Tastenbelegung, FOV in `options.txt`). Gespeichert unter `<data-dir>/syncpacks/<id>/syncpack.json`; Plattform-Inhalte als Projekt-ID ohne feste Version, eigene Dateien liegen im Sync-Pack-Ordner.
- Zwei Arten: **Vorlage** wird einmal beim Anlegen eines Profils oder Modpack-Imports hineinkopiert und ist danach normaler Profil-Inhalt. **Sync-Pack** bleibt verbunden: Änderungen am Sync-Pack kommen in alle Profile, die es nutzen.
- Pro Profil beliebig viele Sync-Packs per Schalter an/aus (Profilseite, Tab „Inhalte“ oder eigener Bereich). Optional global: „automatisch bei allen neuen Profilen“ oder nur bei bestimmten Loadern/Versionen (z. B. alle Fabric-Profile ab 1.21).
- Beim Aktivieren und vor jedem Start wird für die Version und den Loader des Profils die passende Datei gesucht und installiert (Logik wie beim Versionswechsel). Nicht verfügbare Inhalte werden übersprungen und angezeigt, ohne den Start zu blockieren.
- Herkunft steht im `pack.lock.json` (`sync:<id>`), damit Pinecone Sync-Inhalte von Pack- und eigenen Inhalten unterscheiden kann: Ausschalten entfernt nur diese Dateien, Pack-Updates fassen sie nicht an (zählen als Nutzer-Inhalt).
- Konflikte: Bringt das Modpack denselben Mod schon mit, gewinnt das Modpack und der Sync-Eintrag ruht. Ressourcenpakete aus Sync-Packs werden in `options.txt` (`resourcePacks`) in der gewählten Reihenfolge aktiviert. Config-Dateien aus Sync-Packs überschreiben nur, solange der Nutzer sie im Profil nicht geändert hat (gleicher Hash-Vergleich wie bei Pack-Updates).
- Teilen: Sync-Packs lassen sich als kleine Datei bzw. später über den Share-Dienst weitergeben; beim Export eines Profils wählbar, ob Sync-Inhalte mit ins `.mrpack` kommen.

**Screenshots (`screenshots`)**
- Dateiüberwachung (`notify`) auf `<profil>/screenshots`, Thumbnail-Cache, Galerie über alle Profile mit Filter nach Profil/Datum, Aktionen: kopieren, löschen, Ordner öffnen, Datei teilen.

**Dashboard (`stats`, `servers`)**
- Zuletzt gespielt: Profil, Zeitpunkt und Spielzeit (Start/Ende des Prozesses mitschreiben, in `stats.json` bzw. SQLite).
- Letzter Server mit Quick-Join: Auswertung von `servers.dat` (NBT) und Log; Start mit `--quickPlayMultiplayer <host>` ab 1.20, davor `--server/--port`. Server-Status (MOTD, Spieler, Ping) per Server List Ping.
- Widgets: Zuletzt gespielt, Quick-Join, neueste Screenshots, Modpack-Updates verfügbar.

**Personalisierung (`settings` + UI)**
- Themes über CSS-Variablen (hell/dunkel/System, Akzentfarbe, Presets), Hintergrundbild, Anordnung der Dashboard-Widgets, Profil-Icons/Farben, Sprache (de/en), alles in `settings.json` und als Theme-Datei exportierbar.

**Redesign (Phase 11)**

Vorgaben: `docs/design/design-system.md` (Regeln, Tokens, Komponenten) und der Prototyp `docs/design/prototyp.html` mit fünf Screens (Spielen, Bibliothek/Instanz, Entdecken, Einstellungen › Darstellung, Marke; als PNG daneben). Bei Abweichungen gilt das Design-System. Kurzfassung: warmes Rinden-Braun, Harz-Amber nur für die Hauptaktion, Grün nur als Status, Schuppen-Schnitt statt Rundungen, Kanten statt Schatten, Pixelify Sans für Titel, Figtree für Text, JetBrains Mono für Versionen. Keine Emojis, keine Verläufe, kein linker Icon-Rail.

*Neue Informationsarchitektur*

| Bisher | Neu |
|---|---|
| Linke Icon-Leiste (Start, Bibliothek, Entdecken, Screenshots, Skins, Sync-Packs, Einstellungen) | Kopfleiste mit Tabs **Spielen · Bibliothek · Entdecken · Events · Skins**, rechts Downloads, Benachrichtigungen, Einstellungen |
| Play-Button pro Profil | **Spielleiste** unten auf jedem Screen: Instanzwahl (öffnet nach oben), SPIELEN, Freunde, Konto |
| Dashboard mit frei anordenbaren Widgets | Spielen-Seite: Event-Teaser (Hero), Event-Kalender, „Zuletzt gespielt“ mit TwoGG-Server-Schnellbeitritt |
| Profilseite mit Tabs Inhalt/Log | Instanz-Detail in der Bibliothek: Header mit Meta-Chips, Unter-Tabs **Inhalt · Welten · Server · Screenshots · Logs · Einstellungen** |
| Screenshots als eigene Seite | Tab in der Instanz; die Galerie über alle Profile bleibt über die Bibliothek erreichbar |
| Sync-Packs als eigene Seite | Gruppe „Sync-Packs“ in der Bibliotheksliste; Schalter pro Instanz im Tab Inhalt |
| Einstellungen als eine Seite | Einstellungen mit Nav: Allgemein, Darstellung, Java & Arbeitsspeicher, Konten, Downloads & Quellen, TwoGG-Verknüpfung |

*Schritte* (jeweils eigener Commit, UI im Mock-Modus mit Playwright gegen die Referenz-PNGs bei 1280 × 800 geprüft)

- **11a Fundament**
  - Tokens aus Abschnitt 14 als CSS-Variablen; die alten Variablen (`--accent`, `--muted`, `--radius` …) werden ersetzt, nicht parallel gehalten.
  - Schriften lokal einbinden (`@fontsource/figtree`, `@fontsource/pixelify-sans`, `@fontsource/jetbrains-mono`, OFL), kein Google-Fonts-Abruf zur Laufzeit.
  - Themes **Rinde** (Standard), **Kiefer**, **Winterland** (hell) statt Hell/Dunkel/System; Akzente Harz, Nadel, Gletscher, Beere, Flechte, Text darauf immer `#1B1206`.
  - Einstellungen erweitern: `theme.name`, `theme.accent` (Standard `#E9A23B`), Schalter Zapfenschuppen-Muster, Pixel-Überschriften, Animationen reduzieren. Alte `settings.json` werden migriert (dunkel → Rinde, hell → Winterland, System → je nach OS; altes Grün → Harz). Das Hintergrundbild entfällt.
  - Gemeinsame Bausteine: Button (Harz, Split, Sekundär, Link, gestrichelt, Icon), Segment-Tabs, Unter-Tabs, Schalter (`aria-pressed`), Eingabefeld, Filter-Dropdown, Karte/Kachel mit Schnitt, Event-Badge, Meta-Chip, Tag, Fortschrittsbalken, Overlay-Tafel, Schuppenmuster, Logo/Wortmarke, Dialog-Rahmen.
  - Icon-Satz auf 24er-Linien-Icons umstellen; Instanz-Motive (Wappen, Block, Haus, Regler, Wellen, Schneeflocke, Server, Blitz) mit den Farbcodes aus Abschnitt 3 als Profil-Icon wählbar, eigene Bilder bleiben möglich.
  - Neues Pixel-Logo als App-Icon (`tauri icon` aus dem SVG), Favicon und Admin-Seite.
- **11b Fensterrahmen**
  - Eigene Kopfleiste (52 px) mit Tabs, Ziehfläche (`data-tauri-drag-region`) und Fenster-Buttons (`decorations: false`). Unter Linux per Einstellung zurück auf den System-Rahmen, weil eigene Rahmen dort je nach Fenstermanager haken.
  - Spielleiste (88 px): Instanzwahl mit Suche und Gruppen, SPIELEN mit allen Zuständen (Installieren mit Fortschritt, Läuft → Stoppen, Update nötig, Konto fehlt), Konto-Menü nach oben (bisheriges AccountMenu).
  - Standardgröße 1280 × 800, Mindestgröße etwa 1024 × 680. Kleinere Fenster: Seitenspalten schmaler, Kalender unter den Hero.
  - Downloads-Panel (Warteschlange mit Fortschritt, Zähler am Icon) und Benachrichtigungen (Updates, Neuigkeiten, fertige Downloads) als Overlays unter der Kopfleiste.
- **11c Spielen**
  - Bühne mit Wald-Illustration (SVG nach Abschnitt 10), Event-Teaser links, Event-Kalender rechts oben, „Zuletzt gespielt“ mit 4 Instanzen und TwoGG-Server-Kachel (Status per Server List Ping, Schnellbeitritt).
  - Ohne Event zeigt der Hero die neueste angeheftete Neuigkeit. Der Widget-Baukasten aus Phase 6/9 entfällt; Einzelbereiche lassen sich in den Einstellungen ausblenden.
- **11d Bibliothek + Instanz**
  - Liste links (280 px) mit Gruppen TwoGG Events, Eigene Instanzen, Sync-Packs, Filter, „Modpack importieren“ (gestrichelt), „+“ für neue Instanz.
  - Header mit Schuppenmuster, Icon 96 px, Pixel-Titel, Meta-Chips (Loader + Version, RAM, Pack-Status „Synchron mit Event-Pack“), Aktionen Ordner, Exportieren, Menü (Teilen, Version wechseln, Pack-Update, Duplizieren, Löschen).
  - Tab Inhalt als Tabelle Name · Status · Quelle · Aktiv mit Segment-Filter, Suche, „Updates prüfen“, „Inhalt hinzufügen“. Dateien aus einem TwoGG-Pack sind gesperrt (Schloss statt Schalter), Sync-Pack-Inhalte gekennzeichnet.
  - Neu im Kern: **Welten** (Liste aus `saves/` mit Name, Spielmodus, letzte Änderung, Größe; Sichern, Ordner öffnen, Löschen) und **Server** (aus `servers.dat` mit Status und Schnellbeitritt). Logs und Screenshots aus den bisherigen Seiten.
- **11e Entdecken**
  - Segment-Tabs Modpacks, Mods, Ressourcenpakete, Shader, **Datenpakete** (neu: Modrinth-Typ `datapack`, Installation in eine gewählte Welt).
  - Filter Loader, Version, Kategorie, **Quelle (Alle, Modrinth, CurseForge, TwoGG)**; „Alle“ fragt die Quellen parallel ab und mischt die Treffer. Sortieren. Rechts „Installieren in [Instanz]“.
  - Streifen „TwoGG-Auswahl“ mit 3 Empfehlungen vom Server, Ergebnisraster mit 3 Spalten, Karten mit Holen / Installiert / Fortschritt.
- **11f Einstellungen**
  - Nav links (230 px), Seiten aus den bisherigen Einstellungen aufgeteilt.
  - Darstellung wie im Prototyp mit Live-Vorschau. Neue Seite TwoGG-Verknüpfung (Server-Adresse, Neuigkeiten an/aus, später Konto-Verknüpfung mit twogg.app). Versionsfeld und Update-Kanal unten in der Nav.
- **11g Events und Skins**
  - Server: Ressource **Events** (Titel, Untertitel, Badge-Text, Motiv, Status geplant/läuft/beendet, Termin oder „Termin folgt“, Beschreibung, Links Anmeldung/Discord, zugehöriges TwoGG-Pack, Server-Adresse) und **Empfehlungen** (Verweis auf TwoGG-Pack oder Modrinth-/CurseForge-Projekt) in API und Admin-Seite.
  - Launcher: Events-Seite (Liste, Detail mit Countdown, Anmeldung über twogg.app, Event-Pack installieren, Server beitreten). Gildenwahl läuft über twogg.app, solange es dafür keine API gibt.
  - Skins-Seite im neuen Stil; Events und Skins sind im Prototyp noch nicht gestaltet, deshalb erst Entwurf im Mock-Modus zur Abnahme.
- **11h Dialoge, Feinschliff, Barrierefreiheit**
  - Alle Dialoge (Import, Teilen, Pack-Update, Versionswechsel, Login, Bestätigen …) auf den neuen Rahmen.
  - Kontraste (4.5:1) in allen drei Themes und mit allen Akzenten prüfen, Tastaturbedienung und Fokusrahmen, `aria`-Rollen, „Animationen reduzieren“ und `prefers-reduced-motion`.
  - Englische Texte nachziehen; deutsche UI in Du-Form.

*Stand*: 11a–11h umgesetzt (Branch `phase-11-redesign`). Alle Screens aus dem Prototyp sind 1:1 nachgebaut, die alten Seiten entfernt. Events und Skins sind im Stil des Design-Systems neu entworfen (im Prototyp nicht gestaltet).

*Entscheidungen*
- **Freunde**: eigene Pinecone-Freundesliste auf dem Pinecone-Server, Identität über das Minecraft-Konto (Mojang `join`/`hasJoined`), Anfragen per Spielername, Präsenz mit Aktivität alle 45 s.
- **twogg.app**: Events, Anmeldungen, Teilnahme und Freunde direkt über `api.twogg.app` (Discord-Login per Webview-Handoff oder E-Mail). Die API kennt nur die Client-Typen web/desktop/mobile; Pinecone meldet sich als `desktop`. Ein eigener Typ `pinecone` in der twogg.app-API wäre sauberer.
- **TwoGG-Auswahl** und **Community-Server** werden in der Admin-Seite gepflegt; Packs bekommen optional eine twogg.app-Event-ID.
- Unter Linux lässt sich der System-Fensterrahmen einschalten.

*Offene Punkte*
- **Gilde** im Instanz-Header und „Gilde wählen“: brauchen eine Schnittstelle zu twogg.app; bis dahin nur Link zur Anmeldung.
- **TwoGG-Server-Adresse und Event-Daten** kommen vom Pinecone-Server und werden nicht im Launcher fest eingetragen (Platzhalter `[SERVER-ADRESSE]`, solange nichts gepflegt ist).

## Risiken / offene Punkte
- **Share-Dienst**: braucht einen kleinen Server (Hosting, Missbrauchsschutz, Limits). Offene Entscheidung: selbst hosten (z. B. bei TwoGG) oder zunächst nur Datei-Sharing. Empfehlung: Datei-Sharing und Deep-Links zuerst, Dienst danach.
- **Azure-App-Registrierung**: Für Minecraft-Auth braucht es eine eigene Client-ID; neue Apps müssen bei Mojang freigeschaltet werden (Antragsformular). Bis dahin Phase 3 mit Offline-Modus testen.
- Offline-Modus nur für Tests/LAN; echte Server verlangen Online-Accounts.
- Forge/NeoForge-Installer sind der größte Einzelaufwand, deshalb nach hinten gestellt. Der Versionswechsel zwischen Loader-Familien (Fabric/Quilt ↔ Forge/NeoForge) hängt davon ab.
- Versionswechsel: Viele Mods gibt es nicht für jede Version/jeden Loader; die Ersatzliste für Loader-Gegenstücke muss gepflegt werden. Welten nach einem Downgrade können kaputtgehen, deshalb immer Sicherung und klare Warnung.
- Sync-Packs: Automatisches Installieren vor dem Start braucht Netzwerk; ohne Netz mit dem zuletzt installierten Stand starten.
- Redesign: Eigener Fensterrahmen ist unter Linux (Wayland, Tiling-WMs) fehleranfällig, deshalb abschaltbar. Die Wald-Illustration und das Schuppenmuster als SVG halten, nicht als großes Bild, damit sie mit Themes und Akzenten mitgehen.
- Mojang-Marke/EULA: Launcher darf keine Mojang-Assets weiterverbreiten, nur von deren Servern laden; Name/Logo nicht wie offiziell gestalten.

## Verifikation
- Unit-Tests: Rule-Auswertung, Argument-Substitution, Offline-UUID, `inheritsFrom`-Merge gegen echte JSONs (1.8, 1.12, 1.20.x, aktuelle Version).
- Integration: CLI startet Vanilla 1.21.x und eine Legacy-Version (1.8.9) bis ins Hauptmenü, auf Windows und Linux (CI-Matrix / VM).
- Fabric-Profil mit einem Modrinth-Mod startet und lädt den Mod (im Log sichtbar).
- Versionswechsel: Fabulously Optimized von der aktuellen auf eine ältere Minecraft-Version und von Fabric auf Quilt umziehen, Vorschau mit gefundenen/fehlenden Mods prüfen, Profil startet.
- Sync-Pack mit einem Ressourcenpaket und einem Mod an zwei Profilen unterschiedlicher Versionen aktivieren, jeweils passende Datei installiert; ausschalten entfernt nur diese Dateien, ein Pack-Update lässt sie unangetastet.
- Login: Device-Code-Flow mit eigenem Account, Neustart ohne erneuten Login (Refresh).
- GUI: `cargo tauri dev`, Spielstart per Klick, Fortschritt und Logs sichtbar; `cargo tauri build` erzeugt Installer.
- Redesign: Mock-UI bei 1280 × 800 je Screen neben die Referenz aus `docs/design/` legen (Playwright-Screenshots), alle drei Themes und fünf Akzente durchklicken, Tastatur-Durchlauf ohne Maus, Kontrast-Check.
- `cargo clippy` und `cargo test` grün in CI.

## Erster Umsetzungsschritt nach Freigabe
Workspace anlegen (`core`, `cli`), `manifest`- und `download`-Module mit Tests, dann CLI-Befehl `pinecone install <version>` und `pinecone run <version> --offline <name>`.
