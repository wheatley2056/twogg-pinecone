# 02 – Architektur

Quellenhinweise zu Drittanbietern stehen in Klammern. Was nicht in einer offiziellen Dokumentation belegt ist, trägt `[ungeprüft]` und ist in `06` §4 gesammelt. „Neu“ markiert Module und Befehle, die es in 0.9.5 noch nicht gibt.

## 1. Überblick

```
┌──────────────────────────── Pinecone (Tauri 2, ein Prozess) ────────────────────────────┐
│  WebView (WebView2 / WebKitGTK)                                                         │
│  ┌───────────────────────────────────────────────────────────────────────────────────┐  │
│  │ Svelte 5 UI  ui/src                                                               │  │
│  │  Shell (Kopfleiste, Spielleiste) · Pages · Dialoge · Stores (*.svelte.ts)         │  │
│  │  lib/api.ts  ── invoke()/Channel ──┐   (CSP: connect-src ipc: – kein fetch nach   │  │
│  └────────────────────────────────────┼──  außen, alles Netz läuft über Rust)        │  │
│                                       ▼                                                 │
│  src-tauri (dünne Befehlsschicht): commands, game, packs, twogg_app, friends, update …  │
│                                       │                                                 │
│  crates/core (pinecone-core, ohne GUI): Konten, Java, Instanzen, Loader, Downloads,     │
│                Inhalte, Modpacks, Event-Packs, twogg.app-/Server-Clients, Sync-Packs …  │
│                                       │                                                 │
│  crates/cli  (pinecone-cli): gleiche Funktionen auf der Kommandozeile, für Tests        │
└───────────────────────────────────────┼─────────────────────────────────────────────────┘
                                        ▼
  Microsoft/Xbox/Minecraft Services · Mojang (piston-meta, resources, libraries)
  Fabric/Quilt-Meta · Forge/NeoForge-Maven · Modrinth API · CurseForge API
  twogg.app API (Events, Teilnahmen, Konto)  ·  Pinecone-Server pinecone.twogg.app
  (Event-Packs, Manifeste, Neuigkeiten, TwoGG-Auswahl, Community-Server, Freunde)
```

Grundsätze. Sie sind bereits so umgesetzt und werden beibehalten:

1. **Kern ohne GUI.** Die gesamte Logik liegt in `crates/core` und ist mit `cargo test` und der CLI ohne Fenster prüfbar.
2. **Tauri nur als Befehlsschicht.** Befehle in `src-tauri` prüfen Vorbedingungen, rufen den Kern und wandeln Fehler in Texte.
3. **Netz nur aus Rust.** Die WebView darf laut CSP keine fremden Hosts ansprechen. Bilder sind nur von festgelegten CDNs erlaubt.
4. **twogg.app ist die alleinige Quelle für Events, Teilnahmen und Konto.** Pinecone speichert davon nur einen Lese-Cache (§9.3).
5. **Der Pinecone-Server ist die Quelle für Event-Packs und Launcher-Inhalte**, also Neuigkeiten, TwoGG-Auswahl, Community-Server und Freunde. Diese Aufteilung behält der Plan bei. Ob Event-Packs langfristig zu twogg.app wandern, ist offen (Q-02 in `06`).

## 2. Frontend-Framework

**Entscheidung: Svelte 5 (Runes) mit Vite und TypeScript.** Es bleibt beim Bestand: `svelte` 5.57.2, `vite` 8.3.4, `typescript` 5.9.3, `@tauri-apps/api` 2.12.3.

| Kriterium | Svelte 5 | React | Vue 3 | Vanilla/Web Components |
|---|---|---|---|---|
| Vorhandenes HTML/CSS 1:1 übernehmen | ✔ Komponenten sind HTML + `<style>` mit Scope. Die Referenz-HTML aus `docs/design/referenz/` lässt sich fast wörtlich einsetzen | ◐ JSX verlangt `className`, Styles brauchen CSS-Module oder Ähnliches | ✔ SFC ähnlich wie Svelte | ✔, aber ohne Reaktivität |
| Bundle-Größe und Startzeit in der WebView | klein, kein Laufzeit-Framework | größer | mittel | minimal |
| Reaktive Stores für Spielstatus und Downloads | Runes (`$state`, `$derived`) | Hooks plus Bibliothek | Pinia | selbst bauen |
| Bestand | ganze UI vorhanden | Neubau | Neubau | Neubau |

Begründung: Das Design liegt als HTML/CSS vor. Svelte-Komponenten bestehen aus genau diesen Teilen, ohne Umschreiben in JSX. Ein Wechsel würde die vorhandene, getestete UI wegwerfen.

**Wie das Design übernommen wird:**

- **Tokens:** `ui/src/app.css` ist die Übertragung von `design-system.md` §14.
  - Theme Rinde liegt auf `:root`.
  - Kiefer und Winterland liegen auf `:root[data-theme=…]`.
  - Die Akzentfarbe wird zur Laufzeit per `style.setProperty('--accent…')` gesetzt.
  - Die Tokens `--harz*` werden für den gewählten Akzent als `--accent*` geführt. Harz ist dabei der Standardwert.
- **Schnitte:** `--cut-1`, `--cut-2`, das Oktagon nur für SPIELEN.
- **Kanten:** `--edge-harz`.
- **Schriften:** über `@fontsource`, lokal gebündelt statt Google Fonts. Der Grund: offline nutzbar und keine IP-Übertragung an Google (DSGVO, `04` §9).
- **Layout:** Die Shell (`App.svelte`) setzt `grid-template-rows: 52px 1fr 88px`. Kopfleiste (`shell/TitleBar.svelte`) und Spielleiste (`shell/PlayBar.svelte`) sind auf jedem Screen gleich.
- **Bausteine:** Wiederverwendbar unter `ui/src/components/ui/`, z. B. `Segmented`, `Switch`, `Dropdown`, `SearchField`, `ContextMenu`, `PixelAvatar`, `Logo`, `Forest`. Neue Screens dürfen nur diese Bausteine und die Tokens nutzen (`03` §1).
- **Mock-Modus:** `npm run dev` ohne Tauri lädt `ui/src/lib/mock.ts`. Damit lassen sich Screens im Browser prüfen und per Playwright testen.

## 3. Rust-Module

### 3.1 `crates/core` (pinecone-core)

| Modul | Verantwortung | Schnittstelle (Auszug) | Funktionen |
|---|---|---|---|
| `paths` | Datenordner je OS, Pfad-Helfer, `safe_relative` gegen `../` | `Paths::new`, `instance_dir`, `game_dir` | alle |
| `settings` | `settings.json`, Theme, Kanal, Sprache, JVM-Presets, Hinweis-Einstellungen | `SettingsStore::load/save` | F-33, F-34, F-37, F-38 |
| `accounts` | Kontoliste, Schlüsselbund-Zugriff | `Accounts::add/activate/remove`, `refresh` | F-01, F-02 |
| `auth` | Account-Typen, Offline-UUID | `Account::offline` | F-03 |
| `msa` | Microsoft-Device-Code-Flow, Xbox/XSTS/Minecraft, Profil, Skin/Umhang | `start`, `poll`, `refresh`, `upload_skin` | F-01, F-30 |
| `java` | Java finden, Mojang-Laufzeit laden | `installed`, `ensure` | F-04 |
| `manifest`, `install`, `rules` | Versions-Manifest, Bibliotheken, Assets, OS-Regeln | `Installer::install` | F-05, F-19 |
| `loader`, `forge` | Fabric/Quilt-Meta, Forge/NeoForge-Installer samt Prozessoren | `LoaderManager::install/finish` | F-19 |
| `launch`, `launcher` | Startbefehl, Quick Play, Ablauf `prepare` | `Launcher::prepare` | F-05, F-13, F-16, F-27 |
| `download` | parallele, hash-geprüfte Downloads | `Downloader::fetch_all` | F-04, F-05, F-10, F-21, F-24 |
| `profiles` | Instanzen (`profile.json`) | `ProfileStore::create/update/delete` | F-19 |
| `content` | Inhalte, `pack.lock.json`, an/aus, Updates, Abhängigkeiten | `ContentManager::install/set_enabled/remove` | F-11, F-21 bis F-23 |
| `modrinth`, `curseforge` | API-Clients mit User-Agent, Hash-Lookup | `search`, `versions`, `files` | F-21, F-22 |
| `modpack` | `.mrpack`/CurseForge-Import, Updates, Snapshots, Export, TwoGG-Download | `PackManager::*` | F-10, F-12, F-24 bis F-26 |
| `migrate` | Version oder Loader wechseln | `plan`, `apply` | F-20 |
| `servers`, `nbt` | `servers.dat`, SRV, Server List Ping | `list`, `ping` | F-16, F-27 |
| `twogg` | Client Pinecone-Server: Packs, Neuigkeiten, Info, Auswahl, Freunde | `TwoggServer::*` | F-10, F-15 bis F-17 |
| `twogg_app` | Client twogg.app: Events, Teilnahmen, Konto, Join | `TwoggApp::*` | F-07 bis F-09, F-13, F-18 |
| `syncpacks` | Sync-Packs und Vorlagen | `SyncManager::*` | F-31 |
| `jvm` | JVM-Presets | `launch_args` | F-32 |
| `stats` | Spielzeit, letzter Server | `record_session` | F-40 |
| `skins` | Skin-Bibliothek | `SkinStore::*` | F-30 |
| `screenshots`, `worlds` | Screenshots, Welten | `list`, `backup` | F-28, F-29 |
| `diagnostics` | Diagnose-ZIP mit Schwärzung | `export` | F-36 |
| `share` | `pinecone://`-Links parsen und bauen | `parse`, `link` | F-25, F-39 |
| **`eventpack`** (neu) | Manifest laden, Ed25519-Signatur prüfen, Integritätsprüfung vor Start, Sperrliste der Pflichtdateien | `fetch_manifest`, `verify`, `check_integrity(profile) -> Report` | F-10, F-11, F-12 |
| **`oauth`** (neu) | OAuth 2.0 Authorization Code + PKCE mit Loopback-Listener für twogg.app | `authorize(cfg) -> Tokens`, `refresh`, `revoke` | F-07 |
| **`eventcache`** (neu) | Lese-Cache für Events und Teilnahmen mit Zeitstempel und ETag | `load`, `store`, `stale_since` | F-08, F-09, F-14 |
| **`reminders`** (neu) | Planer für Event-Hinweise, gespeicherte Auslösungen | `schedule(events, prefs) -> Vec<Reminder>` | F-14 |
| **`crash`** (neu) | Crash-Report finden, Ursache erkennen (Regeln) | `analyze(game_dir, log) -> CrashInfo` | F-35 |
| **`importers`** (später) | Fremd-Launcher lesen | `scan`, `import` | F-42 |

### 3.2 `src-tauri`

| Modul | Verantwortung |
|---|---|
| `main.rs` | App-Aufbau, Plugins (updater, dialog, deep-link, single-instance, opener), Logging, Befehlsregistrierung |
| `commands.rs`, `content.rs`, `instance.rs`, `packs.rs`, `migrate.rs`, `syncpacks.rs`, `skins.rs`, `screenshots.rs` | Befehle je Bereich |
| `game.rs` | Spielprozesse, Live-Log, Laufzustand (`running`), Spielende |
| `twogg_app.rs` | twogg.app-Befehle. Wird umgebaut: Discord-WebView und Passwort-Login entfallen, `oauth` kommt dazu (F-07) |
| `friends.rs` | Freunde-Sitzung (Mojang-Beweis), Präsenz |
| `update.rs` | Updater mit Kanälen |
| `splash.rs` | Intro-Fenster |
| `open.rs` | Deep Links, Dateizuordnung, zweite Instanz |
| `dashboard.rs` | Altlast (Widget-Dashboard). Wird aufgelöst: Einstellungen nach `commands.rs`, `dashboard`/`widgets` entfernen |
| **`notify.rs`** (neu) | OS-Benachrichtigungen über `tauri-plugin-notification`, Planer-Schleife für `reminders` |
| `dto.rs` | Datenobjekte für die UI |

## 4. IPC

### 4.1 Befehle (`invoke`)

Bestand in 0.9.5: 121 Befehle, registriert in `src-tauri/src/main.rs`. Gruppiert:

| Gruppe | Befehle |
|---|---|
| App | `app_info`, `get_settings`, `save_settings`, `export_theme`, `import_theme`, `jvm_builtin_presets`, `splash_finished`, `diagnostics_export`, `open_logs_folder`, `last_log` |
| Konten | `list_accounts`, `add_offline_account`, `set_active_account`, `remove_account`, `login_start`, `login_finish`, `login_cancel` |
| Java | `java_list` |
| Instanzen | `list_versions`, `loader_versions`, `list_profiles`, `create_profile`, `update_profile`, `delete_profile`, `open_profile_folder`, `migrate_plan`, `migrate_apply` |
| Spiel | `launch(id, server?)`, `stop_game`, `ping_server`, `servers_list` |
| Inhalte | `content_list`, `content_identify`, `content_set_enabled`, `content_remove`, `content_add_local`, `content_add_dropped`, `content_install`, `content_updates`, `content_update`, `modrinth_search`, `modrinth_versions`, `modrinth_categories`, `curseforge_search`, `curseforge_files`, `content_install_curseforge`, `datapack_install` |
| Packs | `pack_install_modrinth`, `pack_install_curseforge`, `pack_install_twogg`, `pack_preview_file`, `pack_preview_link`, `pack_preview_cancel`, `pack_import_pending`, `pack_share_link`, `pack_info`, `pack_check_update`, `pack_updates_all`, `pack_fill_icons`, `pack_plan_update`, `pack_apply_update`, `pack_snapshots`, `pack_rollback`, `pack_export`, `twogg_packs`, `twogg_pack`, `twogg_news` |
| TwoGG-Server | `twogg_server_info`, `twogg_recommendations`, `friends_list`, `friends_presence`, `friends_request`, `friends_accept`, `friends_decline`, `friends_remove` |
| twogg.app | `twogg_app_events`, `twogg_app_me`, `twogg_app_login_discord`, `twogg_app_login_password`, `twogg_app_logout`, `twogg_app_participations`, `twogg_app_join`, `twogg_app_friends` |
| Sync-Packs | `syncpack_list`, `_create`, `_save`, `_delete`, `_add_file`, `_add_from_profile`, `_for_profile`, `_set_enabled`, `_sync`, `_profiles`, `_profile_options` |
| Welten, Screenshots, Skins | `worlds_list`, `world_backup`, `world_delete`, `world_open_folder`, `screenshots_list`, `screenshot_thumb`, `screenshot_delete`, `screenshot_reveal`, `screenshot_save_as`, `open_screenshot`, `skins_list`, `skin_data`, `skin_import_file`, `skin_import_player`, `skin_save`, `skin_rename`, `skin_set_model`, `skin_delete`, `skin_export`, `skin_account`, `skin_apply`, `skin_set_cape`, `skin_import_account` |
| Update, Links | `update_check`, `update_install`, `take_open_requests` |
| Altlast | `dashboard` (entfällt) |

**Änderungen laut Plan:**

| Befehl | Art | Signatur (vereinfacht) | Funktion |
|---|---|---|---|
| `twogg_app_connect` | neu | `() -> AppUser` – startet OAuth + PKCE im Standardbrowser | F-07 |
| `twogg_app_link_minecraft` | neu | `(accountUuid) -> AppUser` – Mojang-Beweis an twogg.app | F-07 |
| `twogg_app_disconnect` | ersetzt `twogg_app_logout` | `() -> ()` inkl. Revoke | F-07 |
| `twogg_app_login_discord`, `twogg_app_login_password` | entfällt | – | F-07 (AK-07.1, AK-07.2) |
| `twogg_app_events` | geändert | `() -> Cached<Vec<Event>>` mit `staleSince` | F-08 |
| `twogg_app_participations` | geändert | `() -> Cached<Vec<Participation>>` inkl. `guild` | F-09 |
| `twogg_app_community` | neu | `() -> Community` (Bewerbungsphase, Links) | F-18 |
| `event_pack_status` | neu | `(eventId) -> EventPackStatus` (installiert, Version, Update, `required`) | F-10, F-12 |
| `event_pack_install` | neu, ersetzt `pack_install_twogg` für Events | `(eventId, channel) -> Profile` | F-10 |
| `event_join` | neu | `(eventId) -> ()` – Event-Instanz wählen, prüfen, starten mit Server | F-13 |
| `event_integrity_check` | neu | `(profileId) -> IntegrityReport` | F-11 |
| `crash_info` | neu | `(profileId) -> Option<CrashInfo>` | F-35 |
| `reminder_settings` | neu | in `save_settings` integriert (`notifications`-Block) | F-14, F-37 |
| `content_set_enabled`, `content_remove` | geändert | lehnen Pflichtdateien ab (`Error::Locked`) | F-11 |

### 4.2 Ereignisse und Kanäle (Rust → UI)

| Name | Art | Inhalt | Funktion |
|---|---|---|---|
| Startfortschritt | `Channel<PrepareEvent>` bei `launch` | Stufe, Download-Fortschritt, Java | F-05 |
| Spielausgabe und Spielende | `Channel` bei `launch` | Logzeilen, Exit-Code | F-05, F-35 |
| Installationsfortschritt | `Channel` bei Pack- und Inhaltsinstallation | Fortschritt | F-10, F-21, F-24 |
| `open-pack` | Event | Deep Link oder Datei | F-39 |
| `twogg-app-login` | Event (Bestand) | Login-Ergebnis. Wird durch `twogg-app-connected` ersetzt | F-07 |
| `event-reminder` (neu) | Event | `eventId`, Vorlauf | F-14 |
| `participation-changed` (neu) | Event | geänderte Teilnahmen nach Abfrage | F-09, F-37 |
| `crash-detected` (neu) | Event | `profileId`, `CrashInfo` | F-35, F-37 |

## 5. Externe Quellen und Spielstart

| Zweck | Quelle | Beleg |
|---|---|---|
| Versionsliste | `https://piston-meta.mojang.com/mc/game/version_manifest_v2.json` | Community-Doku minecraft.wiki „Version manifest.json“ [ungeprüft: keine offizielle Mojang-Doku] |
| Java-Laufzeiten | `https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96…/all.json`; Komponente aus `javaVersion.component` im Client-JSON | Endpunkt antwortet (geprüft 12.10.2026), aber [ungeprüft: nicht offiziell dokumentiert] |
| Fabric | `meta.fabricmc.net/v2/versions/loader/{mc}` und `/{mc}/{loader}/profile/json` | Routenliste auf meta.fabricmc.net, Endpunkte antworten |
| Quilt | `meta.quiltmc.org/v3/versions/loader/{mc}` und `/profile/json` | Swagger-UI auf meta.quiltmc.org |
| NeoForge | `maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge`, Installer-JAR aus `releases/…` | Endpunkt antwortet. Das Installer-Muster ist [ungeprüft: nur per Abruf, nicht dokumentiert] |
| Forge | `maven.minecraftforge.net/…/maven-metadata.xml`, `files.minecraftforge.net/…/promotions_slim.json` | wie NeoForge, [ungeprüft] |
| Modrinth | `https://api.modrinth.com/v2` | docs.modrinth.com/api: User-Agent muss die Anwendung eindeutig erkennbar machen, 300 Anfragen/min je IP, Header `X-Ratelimit-*` |
| CurseForge | `https://api.curseforge.com`, Header `x-api-key` | docs.curseforge.com/rest-api |
| Microsoft-Login | siehe §7.1 | Microsoft Learn, minecraft.wiki |

- **User-Agent** für alle Anfragen: `TwoGG-Pinecone/<version> (+https://pinecone.twogg.app; [KONTAKT-ADRESSE])`. Das erfüllt die Modrinth-Vorgabe „eindeutig erkennbar, Kontakt empfohlen“. Heute ist es `pinecone/<version>`; der Kontakt fehlt (Aufgabe in `05`).
- **Rate-Limit:** Bei HTTP 429 oder `X-Ratelimit-Remaining: 0` wartet der Client bis `X-Ratelimit-Reset`.
- **Quick Play:** `--quickPlayMultiplayer <adresse[:port]>` seit Minecraft 1.20 (Snapshot 23w14a). Davor gilt `--server`/`--port`, das in 23w14a entfernt wurde. Ab 26.x wird das Argument über Feature-Flags im Client-JSON eingetragen. Quelle: minecraft.wiki „Quick Play“ (Community) [ungeprüft: keine offizielle Doku]. Vorhanden in `launch.rs`.

## 6. Download-Pipeline

| Eigenschaft | Ist (0.9.5, `download.rs`) | Ziel |
|---|---|---|
| Parallelität | `buffer_unordered`, 16 gleichzeitig, doppelte Ziele zusammengefasst | bleibt. Je Host höchstens 8, damit Modrinth (300/min) und Mojang-CDN nicht überlastet werden. Die Zahl wird im Betrieb geprüft |
| Hash beim Laden | SHA-1 während des Streams, bei Abweichung `.part` löschen | zusätzlich SHA-512, wo die Quelle ihn liefert: `.mrpack` und Event-Manifest. Die mrpack-Spezifikation verlangt SHA-1 **und** SHA-512 je Datei (Modrinth-Formatdefinition) |
| Vorhandene Dateien | Größe, optional SHA-1 (`--verify`) | Hash-Cache `cache/hashes.json` mit Pfad → (Größe, mtime, sha1, sha512). Für die Integritätsprüfung (F-11) ohne erneutes Hashen |
| Wiederholung | 3 Versuche, exponentiell ab 500 ms, kein Retry bei 4xx | bleibt. 429 wird wie oben behandelt |
| Fortsetzen | fehlt: `.part` wird neu angelegt | **neu:** besteht `.part`, fordert der Client mit `Range: bytes=<n>-` und `If-Range: <ETag/Last-Modified>` an. Antwortet der Server mit 206, wird angehängt, sonst neu geladen. Der Hash läuft über die ganze Datei. Grenze: Ab 1 MiB lohnt sich das Fortsetzen; kleinere Dateien werden neu geladen |
| Atomik | `.part` → `rename` | bleibt |
| Zeitlimits | Verbindung 15 s, Lesen 60 s | bleibt |
| Abbruch | Stufe bricht ab | Abbruch-Token je Vorgang. `.part` bleibt für das Fortsetzen liegen und wird nach 7 Tagen weggeräumt |
| Cache | `cache/packs/` für Pack-Archive | zusätzlich `cache/eventpacks/<packId>/<version>/` mit Manifest und `.mrpack`. Nötig für die Offline-Reparatur (F-11) |

## 7. Sicherheit

### 7.1 Microsoft-Login (F-01)

- **Flow:** OAuth Device Code gegen `https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode` und `/token`, Scope `XboxLive.signin offline_access`. Der Nutzer hat standardmäßig 15 Minuten. Der Client fragt nicht schneller ab als das gelieferte `interval`.
  - Quelle: Microsoft Learn „OAuth 2.0 device authorization grant“ (Stand 04.01.2025).
  - Der Tenant `consumers` und der Scope `XboxLive.signin` sind nur von der Community dokumentiert (minecraft.wiki „Microsoft authentication“) [ungeprüft].
- **Kette:** Xbox Live `user.auth.xboxlive.com/user/authenticate` → XSTS `xsts.auth.xboxlive.com/xsts/authorize` (RelyingParty `rp://api.minecraftservices.com/`) → `api.minecraftservices.com/authentication/login_with_xbox` → `/entitlements/mcstore` → `/minecraft/profile`. Quelle: minecraft.wiki (Community) [ungeprüft].
  - XSTS-Fehler `XErr` 2148916233 (kein Xbox-Konto), 2148916235 (Land), 2148916236/7 (Altersprüfung), 2148916238 (Kinderkonto ohne Familie) werden in verständliche Texte übersetzt [ungeprüft].
- **Warum Device Code und nicht PKCE:**
  - Die feste Entscheidung „OAuth + PKCE“ betrifft nur die TwoGG-Verknüpfung.
  - Device Code braucht keine Redirect-URI und bleibt bestehen.
  - Microsoft würde auch Auth Code + PKCE mit Loopback-Redirect unterstützen (Microsoft Learn „auth code flow“, Stand 09.01.2026). Ein späterer Wechsel ist möglich, steht aber nicht im Plan.
- **Voraussetzung:** eigene Azure-App (öffentlicher Client, „Allow public client flows“ für Device Code) und Freigabe der App-ID für die Minecraft-Services-API. Details: `06` H-01.
- **Laufzeiten:**
  - Microsoft-Access-Token 60–90 min (Microsoft Learn „Access tokens“).
  - Refresh-Token 90 Tage, vorzeitiger Widerruf möglich (Microsoft Learn „Refresh tokens“).
  - Minecraft-Token laut Community-Quelle `expires_in: 86400` [ungeprüft].
  - Pinecone liest Laufzeiten immer aus der Antwort und rechnet nicht mit festen Werten.

### 7.2 twogg.app-Verknüpfung (F-07)

- **OAuth 2.0 Authorization Code mit PKCE:**
  - `code_verifier` mit 43–128 Zeichen, `S256` ist Pflicht, wenn der Client es kann (RFC 7636).
  - Redirect über Loopback `http://127.0.0.1:<zufälliger Port>/callback`. Der Server muss jeden Port erlauben; `localhost` ist nicht empfohlen (RFC 8252 §7.3, §8.3).
  - Der Login läuft im Standardbrowser, eingebettete WebViews sind verboten (RFC 8252 §8.12). Damit fällt der heutige Discord-WebView weg.
- **Ablauf, Endpunkte und Gültigkeiten:** `04` §2 und §4.
- **Client-Kennung:** `client_id = pinecone` als öffentlicher Client ohne Secret. Header `X-Twogg-Client: pinecone` und `X-Twogg-Client-Version: <eigene Version>`. Das ersetzt das heutige Auftreten als `desktop` mit fremder Mindestversion.

### 7.3 Token-Speicherung

| Geheimnis | Ort | Schlüssel |
|---|---|---|
| Microsoft-Refresh-Token | OS-Schlüsselbund über `keyring`: Windows Credential Manager, macOS Keychain, Secret Service unter Linux (docs.rs `keyring`, v1-Feature) | Dienst `twogg-pinecone`, Konto `msa-refresh:<uuid>` |
| twogg.app-Refresh-Token | OS-Schlüsselbund | `twogg-app-refresh` |
| Freunde-Sitzung (Pinecone-Server) | OS-Schlüsselbund | `pinecone-friends-<uuid>` |
| Access-Tokens (Minecraft, twogg.app) | nur im Speicher | – |

- **Größe:** Microsoft-Refresh-Tokens sind lang. Eine Größengrenze des Windows Credential Managers ist nicht offiziell belegt [ungeprüft, `06` U-05]. Bestand 0.9.5 speichert erfolgreich; ein Test mit maximal langem Token kommt in die CI.
- **Linux ohne Secret Service** (z. B. minimale Window-Manager): Der Login wird mit dem Hinweis „Kein Schlüsselbund gefunden, bitte installiere gnome-keyring oder KWallet“ abgelehnt. Es gibt keinen Klartext-Fallback.
- **`keyring` 4.x** teilt sich in `keyring-core` und Speicher-Crates je Plattform (docs.rs). Ein Upgrade wird bewusst geplant und läuft nicht automatisch.
- Tauri Stronghold wäre die Alternative (plattformübergreifend, braucht ein Passwort, v2.tauri.app/plugin/stronghold). Pinecone nutzt es nicht, weil der Nutzer sonst ein Passwort bräuchte.

### 7.4 Echtheit von Event-Packs (F-10, F-11)

- **Signatur:** Ed25519 über die kanonische JSON-Form des Manifests (`04` §6).
  - Der öffentliche Schlüssel ist in den Launcher eingebaut (`EVENTPACK_KEYS`, mehrere erlaubt für den Schlüsselwechsel).
  - Der private Schlüssel liegt offline beim Orga-Team ([VERANTWORTLICHE PERSON]). Er liegt **nicht** auf dem Pinecone-Server, damit ein kompromittierter Server keine Packs fälschen kann.
- **Ergänzende Prüfungen:**
  - Die HMAC-Downloadlinks des Servers bleiben als Missbrauchsschutz für die Bandbreite.
  - Sie sind ausdrücklich **kein** Echtheitsbeweis (siehe Kommentar in Server `api.rs`).

### 7.5 WebView-Härtung

- CSP `connect-src ipc:`; Bilder nur von festgelegten Hosts. Neuigkeiten-HTML wird serverseitig aus Markdown erzeugt und im Client zusätzlich bereinigt (F-15, AK-15.3).
- Externe Links öffnen den Standardbrowser über `tauri-plugin-opener`. Fremde Seiten werden nie im App-Fenster navigiert.

## 8. Pinecone-Server (pinecone.twogg.app)

- **Rolle:**
  - Event-Packs mit Versionen und künftig signierten Manifesten.
  - Pack-Seiten (HTML).
  - Neuigkeiten, TwoGG-Auswahl, Community-Server-Adresse, Freunde.
  - Admin-Oberfläche (React-Admin) für das Team.
- **Technik:** Rust axum 0.8 mit SQLite (sqlx). Eigenes Repo `split/server`, Docker auf `[HOST]` hinter Nginx Proxy Manager.
- **Endpunkte, die der Launcher nutzt:**
  - `GET /api/v1/info`, `/news`, `/packs`, `/packs/{id}`
  - `POST /api/v1/packs/{id}/versions/{v}/download`
  - `GET /api/v1/recommendations`
  - Freunde: `POST /auth/challenge|verify`, `POST /presence`, `/friends*`
- **Neu für den Plan:**
  - `GET /api/v1/packs/{id}/versions/{v}/manifest` liefert das signierte Manifest aus `04` §6. Der Server speichert die beim Upload hochgeladene Signatur und erzeugt sie **nicht** selbst.
  - Admin: Upload von `.mrpack` **und** `.manifest.sig`. Felder für `required`, `allowExtraMods`, `minLauncherVersion` und `lockedPaths`.
- **Schnittstelle zu twogg.app:** keine Server-zu-Server-Verbindung. Event-Packs verweisen per `eventId` auf twogg.app-Events. twogg.app kennt den Pinecone-Server nur als Link-Ziel. Die Alternative, dass twogg.app die Pack-Zuordnung führt, steht in `06` Q-02.

## 9. Datenablage und Dateiformate

### 9.1 Pfade je OS (`crates/core/src/paths.rs`)

| OS | Datenordner | Anmerkung |
|---|---|---|
| Windows | `%APPDATA%\TwoGG\Pinecone` | Roaming. Große Daten (Assets) wandern damit ins Roaming-Profil. Ob `%LOCALAPPDATA%` besser ist, ist offen (Q-10) |
| Linux | `~/.local/share/twogg-pinecone` | XDG |
| macOS | `~/Library/Application Support/TwoGG Pinecone` | erst ab Phase „später“ ausgeliefert |
| alle | überschreibbar per `PINECONE_HOME` bzw. `--data-dir` | für Tests und portable Nutzung |

### 9.2 Aufbau

```
<data>/
  settings.json            Einstellungen (atomar über .tmp geschrieben)
  accounts.json            Kontoliste ohne Tokens
  stats.json               Spielzeit
  versions/ libraries/ assets/ runtimes/      geteilt zwischen Instanzen
  instances/<id>/
    profile.json           Instanz (siehe 9.4)
    pack.lock.json         Inhalte und Pack-Herkunft (siehe 9.5)
    syncpacks.json         Sync-Pack-Zustand
    snapshots/             Pack-Snapshots (3)
    .minecraft/            Spielordner
  syncpacks/<id>/          Sync-Packs
  skins/                   Skin-Bibliothek
  cache/packs/ cache/eventpacks/ cache/thumbs/ cache/hashes.json (neu)
  cache/twogg-app.json     Lese-Cache Events/Teilnahmen (neu, 9.3)
  logs/pinecone.log.<datum> (7 Tage)
```

### 9.3 Lese-Cache twogg.app (neu, `cache/twogg-app.json`)

```json
{ "version": 1, "user": "<twogg-user-id>", "fetchedAt": "2026-10-12T18:00:00Z",
  "events": { "etag": "\"abc\"", "items": [ /* Event aus 04 §4 */ ] },
  "participations": { "etag": "\"def\"", "items": [ /* Participation */ ] } }
```

- Der Cache enthält nur Daten, die der Nutzer selbst sehen darf. Er wird beim Trennen (F-07) oder Kontowechsel gelöscht.
- Der Cache entscheidet nie über den Zugang: Die Server-Adresse wird nicht zwischengespeichert (AK-13 Offline-Fall).

### 9.4 `profile.json`

```json
{ "name": "REVIVAL: Avalon", "minecraft": "1.21.11",
  "loader": { "kind": "fabric", "version": "0.19.5" },
  "memoryMb": 6144, "jvmArgs": [], "jvmPreset": null, "java": null,
  "resolution": null, "color": null, "icon": "pack" }
```

Die ID ist der Ordnername. `loader.kind` ist eines von `fabric | quilt | forge | neoforge`; fehlt der Loader, ist es Vanilla.

### 9.5 `pack.lock.json`

Bestand: `files{ <pfad ohne .disabled>: { kind, source, name, version?, sha1, size, iconUrl?, fromPack, channel?, sync? } }` und `pack?{ name, version, source{type: modrinth|curseforge|twogg|file, …}, files{pfad→sha1} }`.

Erweiterung für Event-Packs (neu):

```json
"pack": { "name": "REVIVAL: Avalon", "version": "1.4.0",
  "source": { "type": "twogg", "packId": "avalon", "eventId": "[TWOGG-EVENT-ID]" },
  "manifest": { "version": "1.4.0", "sha256": "<hash des Manifests>", "keyId": "twogg-2026",
                "required": true, "allowExtraMods": false,
                "locked": { "mods/sodium.jar": "<sha512>", "config/avalon.toml": "<sha512>" } } }
```

`locked` ist die Grundlage der Integritätsprüfung (F-11) und der Sperre in Backend und UI.

## 10. Plattformen, Code-Signing, Updates

### 10.1 Plattformen

| Plattform | Status | Grundlage |
|---|---|---|
| Windows 10/11 x64 | ausgeliefert (NSIS, MSI) | WebView2 ab Windows 7 unterstützt, auf Windows 11 vorinstalliert; der Tauri-Installer stellt es sicher (v2.tauri.app „Webview versions“). Pinecone testet nur Windows 10/11, ältere Versionen sind nicht unterstützt |
| Linux x64 | ausgeliefert (AppImage, deb, rpm) | braucht `webkit2gtk-4.1` (v2.tauri.app „Prerequisites“) |
| macOS (arm64, x64) | später | Bundles laufen standardmäßig ab macOS 10.13 (v2.tauri.app „macOS application bundle“). Signieren und Notarisieren braucht einen Apple-Developer-Account für 99 $/Jahr (v2.tauri.app „macOS code signing“) |
| Windows ARM64, Linux ARM | nicht geplant | Java-Laufzeiten gäbe es laut Java-Manifest für `windows-arm64` [ungeprüft] |

### 10.2 Code-Signing

- **Updater-Signatur** (Bestand): Tauri-Updater mit Minisign-Schlüssel. Die Prüfung lässt sich nicht abschalten. Geht der private Schlüssel verloren, erreichen Updates bestehende Installationen nicht mehr (v2.tauri.app „Updater“). Der Schlüssel liegt als GitHub-Secret und zusätzlich verschlüsselt offline bei [VERANTWORTLICHE PERSON].
- **Windows Authenticode** (neu):
  - Ohne Signatur zeigt SmartScreen Warnungen. Die App läuft trotzdem (v2.tauri.app „Windows code signing“, Stand 11.09.2026).
  - Wahl: **Azure Artifact Signing** (vormals Trusted Signing) oder ein OV-Zertifikat über Azure Key Vault. Beides ist dort dokumentiert. EV-Zertifikate bringen seit 2024 keinen sofortigen SmartScreen-Ruf mehr.
  - Die Kosten sind nicht geprüft [ungeprüft, `06` U-06].
- **macOS:** Developer-ID-Zertifikat plus Notarisierung, erst mit macOS-Auslieferung.
- **Linux:** keine Signatur der Pakete geplant. Die Updater-Signatur schützt AppImage-Updates.

### 10.3 Update-Kanäle

- Tauri kennt keine eingebauten Kanäle. Die Endpunkte werden zur Laufzeit gesetzt (v2.tauri.app „Updater“). Das ist bereits umgesetzt in `update.rs`.
- **Stable:** GitHub-Releases ohne „Pre-release“. **Beta:** zusätzlich Pre-releases. Quelle ist die GitHub-API des öffentlichen Repos oder alternativ `PINECONE_UPDATE_URL` mit `{channel}`.
- Ablauf laut `RELEASING.md`:
  1. Tag `v*` setzen.
  2. Der Workflow baut und legt einen Entwurf mit `latest.json` an.
  3. Veröffentlichen als Pre-release für Beta, als normales Release für beide Kanäle.
- Das Versionsschema bleibt `MAJOR.MINOR.PATCH` ohne Suffix, weil MSI keine Pre-release-Suffixe kann.
- Neu: periodische Prüfung alle 6 h und keine Installation während eines laufenden Spiels (F-34).

## 11. Qualitätssicherung

- **CI** (`.github/workflows/ci.yml`) auf Linux und Windows: Build, `svelte-check`, `cargo fmt --check`, `clippy -D warnings`, `cargo test`.
- **Neu in der CI:**
  - Kontrastprüfung aller Text- und Hintergrund-Token-Paare je Theme (AK-38.3).
  - Playwright-Rauchtest der Screens im Mock-Modus (`03` §5).
  - Signaturtests für Event-Manifeste mit gültigem, manipuliertem und unbekanntem Schlüssel (AK-10.1).
- **Integrationstests** gegen nachgebaute Server sind vorhanden (`crates/core/tests/common`) und werden für twogg.app-OAuth und den Manifestabruf erweitert.
