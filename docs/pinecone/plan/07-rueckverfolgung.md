# 07 – Rückverfolgung

Jede Funktion aus `01` steht hier **genau einmal**.

| Spalte | Inhalt |
|---|---|
| Screen | Abschnitt in `03` |
| Modul | Rust-Modul (`core/…` = `crates/core/src`, `tauri/…` = `src-tauri/src`) und Frontend-Modul (`ui/…` = `ui/src/components` bzw. `ui/src/lib`). „neu“ wie in `02` §3 |
| API-Endpunkt | **twogg.app** `/api/v1/…` bzw. OAuth (`04` §4) · **PS** = Pinecone-Server (`02` §8, `04` §6.3) · **extern** = Dienst Dritter (`02` §5) · „–“ = rein lokal |
| Phase | Block aus `05` |

| Funktion | Screen (`03`) | Modul Rust | Modul Frontend | API-Endpunkt | Phase (`05`) |
|---|---|---|---|---|---|
| F-01 Microsoft-Login | §2.9 Dialog Microsoft-Login, §2.7 Konten | `core/msa`, `core/accounts`, `tauri/commands` | `ui/LoginDialog.svelte`, `ui/settings/AccountsSection.svelte` | extern: Microsoft OAuth Device Code, Xbox Live, XSTS, Minecraft Services | MVP-D |
| F-02 Mehrfachkonten | §2.1 Spielleiste (Konto), §2.7 Konten | `core/accounts` | `ui/AccountMenu.svelte`, `ui/shell/PlayBar.svelte` | – | Phase 0 |
| F-03 Offline-Konto | §2.7 Konten | `core/auth`, `core/accounts` | `ui/settings/AccountsSection.svelte` | – | v1-6 |
| F-04 Java-Laufzeit | §2.7 Java & Arbeitsspeicher | `core/java` | `ui/settings/JavaSection.svelte` | extern: Mojang Java-Laufzeit-Manifest | Phase 0 |
| F-05 Spielstart | §2.1 Spielleiste, §4.6 | `core/launcher`, `core/launch`, `core/install`, `tauri/game` | `ui/shell/PlayBar.svelte`, `ui/lib/run*.svelte.ts` | extern: Mojang piston-meta, Libraries, Assets | Phase 0 |
| F-06 Erststart und Intro | §2.10 | `tauri/splash`, `core/settings` | `ui/splash/`, `ui/pages/PlayPage.svelte` (Willkommensansicht) | – | MVP-D |
| F-07 TwoGG-Konto verknüpfen | §2.7 TwoGG-Verknüpfung, §4.3 | `core/oauth` (neu), `core/twogg_app`, `tauri/twogg_app` | `ui/settings/TwoggSection.svelte` | twogg.app: `GET /oauth/authorize`, `POST /oauth/token`, `POST /oauth/revoke`, `GET /api/v1/me`, `POST /api/v1/me/minecraft/challenge`, `POST /api/v1/me/minecraft/verify`, `DELETE /api/v1/me/minecraft`, `GET /api/v1/health` | MVP-B |
| F-08 Events-Seite | §3.1 | `core/twogg_app`, `core/eventcache` (neu) | `ui/pages/EventsPage.svelte`, `ui/lib/events.svelte.ts` | twogg.app: `GET /api/v1/events`, `GET /api/v1/events/{eventId}` | MVP-B |
| F-09 Anmelde-, Team- und Gildenstatus | §3.1 „Dein Stand“, §2.2 Status-Chip | `core/twogg_app`, `core/eventcache` (neu) | `ui/pages/EventsPage.svelte`, `ui/pages/PlayPage.svelte` | twogg.app: `GET /api/v1/me/participations` | MVP-B |
| F-10 Event-Instanz aus signiertem Manifest | §3.1 „Event-Pack“, §4.4 | `core/eventpack` (neu), `core/modpack`, `core/twogg`, `tauri/packs` | `ui/pages/EventsPage.svelte`, `ui/shell/Panels.svelte` | PS: `GET /api/v1/packs/{id}/versions/{v}/manifest`, `GET /api/v1/packs/{id}/manifest/latest`, `POST /api/v1/packs/{id}/versions/{v}/download` | MVP-A |
| F-11 Pflicht-Mods sperren und Integrität | §2.3 Inhaltstabelle (gesperrte Zeilen), §4.6 Schritt 4 | `core/eventpack` (neu), `core/content`, `tauri/content` | `ui/library/ContentTable.svelte` | – (lokal; Reparatur über die F-10-Endpunkte) | MVP-A |
| F-12 Event-Pack aktuell halten | §2.9 Pack-Update, §4.5 | `core/modpack` (Update, Snapshots), `core/eventpack` (neu) | `ui/PackUpdateDialog.svelte`, `ui/pages/EventsPage.svelte` | PS: `GET /api/v1/packs/{id}/manifest/latest` | MVP-A |
| F-13 Event-Server-Beitritt | §3.1 „Server beitreten“, §4.4 | `core/twogg_app`, `core/launch` (Quick Play), `tauri/game` (`event_join` neu) | `ui/pages/EventsPage.svelte` | twogg.app: `GET /api/v1/events/{eventId}/join` | MVP-B |
| F-14 Countdown und Desktop-Hinweis | §3.1 Countdown, §2.7 Allgemein (Event-Hinweise) | `core/reminders` (neu), `tauri/notify` (neu) | `ui/pages/EventsPage.svelte`, `ui/settings/GeneralSection.svelte` | – (Daten aus dem F-08/F-09-Cache) | MVP-C |
| F-15 Startseite | §2.2 | `core/twogg` (News), `core/stats`, `core/twogg_app` | `ui/pages/PlayPage.svelte` | PS: `GET /api/v1/news` | Phase 0 |
| F-16 Community-Server-Schnellbeitritt | §2.2 Kachel „TwoGG Server“ | `core/twogg`, `core/servers`, `core/launch` | `ui/pages/PlayPage.svelte` | PS: `GET /api/v1/info` | Phase 0 |
| F-17 Freundesanzeige | §2.1 Spielleiste (Freunde) | `core/twogg`, `tauri/friends` | `ui/shell/Friends.svelte`, `ui/lib/friends.svelte.ts` | PS: `POST /api/v1/auth/challenge`, `POST /api/v1/auth/verify`, `POST /api/v1/presence`, `GET /api/v1/friends`, `POST /api/v1/friends/requests`, `POST /api/v1/friends/requests/{uuid}/accept`, `DELETE /api/v1/friends/requests/{uuid}`, `DELETE /api/v1/friends/{uuid}`, `DELETE /api/v1/me` (neu) | v1-6 |
| F-18 Verweise auf twogg.app und Discord | §2.2 Bewerbungsbanner, §3.1 Links | `core/twogg_app` | `ui/pages/PlayPage.svelte`, `ui/pages/EventsPage.svelte` | twogg.app: `GET /api/v1/community` | MVP-B |
| F-19 Instanzen anlegen und bearbeiten | §2.3, §2.9 Neue Instanz | `core/profiles`, `core/loader`, `core/forge` | `ui/ProfileDialog.svelte`, `ui/library/InstanceForm.svelte` | extern: Mojang Versionsmanifest, Fabric-/Quilt-Meta, Forge-/NeoForge-Maven | v1-1 |
| F-20 Version oder Loader wechseln | §2.9 Version wechseln | `core/migrate`, `tauri/migrate` | `ui/MigrateDialog.svelte` | extern: Modrinth API (Versionen) | v1-1 |
| F-21 Entdecken und Modrinth | §2.4 | `core/modrinth`, `core/content` | `ui/pages/DiscoverPage.svelte` | extern: Modrinth API v2 · PS: `GET /api/v1/recommendations` | v1-2 |
| F-22 CurseForge-Inhalte | §2.4 (Quelle CurseForge), §2.9 Fehlende Dateien | `core/curseforge`, `core/content` | `ui/pages/DiscoverPage.svelte`, `ui/MissingDialog.svelte` | extern: CurseForge API (ggf. über PS-Proxy, Q-06) | v1-2 |
| F-23 Inhaltsverwaltung | §2.3 Tab Inhalt | `core/content`, `tauri/content` | `ui/library/ContentTable.svelte`, `ui/library/AddContentDialog.svelte` | extern: Modrinth/CurseForge (Hash-Erkennung, Updates) | v1-2 |
| F-24 `.mrpack`-Import und Updates | §2.9 Import/Vorschau, Pack-Update | `core/modpack`, `tauri/packs` | `ui/ImportDialog.svelte`, `ui/PackUpdateDialog.svelte` | extern: Modrinth API, erlaubte Download-Hosts | v1-3 |
| F-25 `.mrpack`-Export und Teilen | §2.9 Teilen/Exportieren | `core/modpack` (Export), `core/share` | `ui/ShareDialog.svelte` | – | v1-3 |
| F-26 CurseForge-Modpack-Import | §2.9 Import, Fehlende Dateien | `core/modpack` (`read_curseforge`), `core/curseforge` | `ui/ImportDialog.svelte`, `ui/MissingDialog.svelte` | extern: CurseForge API | v1-3 |
| F-27 Serverliste einer Instanz | §2.3 Tab Server | `core/servers`, `core/nbt`, `tauri/instance` | `ui/library/ServersTab.svelte` | extern: Server List Ping zum jeweiligen Server | v1-4 |
| F-28 Welten | §2.3 Tab Welten | `core/worlds`, `tauri/instance` | `ui/library/WorldsTab.svelte` | – | v1-4 |
| F-29 Screenshots | §2.3 Tab Screenshots | `core/screenshots`, `tauri/screenshots` | `ui/library/ScreenshotsTab.svelte` | – | v1-4 |
| F-30 Skins | §3.2 | `core/skins`, `core/msa` (Upload), `tauri/skins` | `ui/pages/SkinsPage.svelte`, `ui/SkinPreview.svelte`, `ui/SkinEditor.svelte` | extern: Minecraft Services (Skin, Umhang), Mojang-Profil (Import per Name) | v1-5 |
| F-31 Sync-Packs | §2.3 Sync-Pack-Ansicht | `core/syncpacks`, `tauri/syncpacks` | `ui/library/SyncPackView.svelte`, `ui/library/OptionsFromProfileDialog.svelte` | extern: Modrinth/CurseForge | v1-5 |
| F-32 Arbeitsspeicher und JVM-Presets | §2.7 Java & Arbeitsspeicher, §2.3 Einstellungen der Instanz | `core/jvm`, `core/settings` | `ui/settings/JvmPresets.svelte`, `ui/library/InstanceForm.svelte` | – | v1-1 |
| F-33 Themes und Akzente | §2.7 Darstellung | `core/settings` | `ui/settings/AppearanceSection.svelte`, `ui/lib/settings.svelte.ts`, `ui/app.css` | – | Phase 0 |
| F-34 Launcher-Auto-Update | §2.7 Allgemein, §2.8 Benachrichtigungen | `tauri/update` | `ui/settings/GeneralSection.svelte`, `ui/lib/updates.svelte.ts` | extern: GitHub Releases (`latest.json`) bzw. `PINECONE_UPDATE_URL` | MVP-D |
| F-35 Absturz- und Log-Ansicht | §2.3 Tab Logs, §2.9 Absturztafel, §4.7 | `core/crash` (neu), `tauri/game` | `ui/library/LogsTab.svelte` | – | MVP-C |
| F-36 Diagnose-Export | §2.7 Allgemein, §2.9 Absturztafel | `core/diagnostics` | `ui/lib/diagnostics.ts`, `ui/settings/GeneralSection.svelte` | – | v1-7 |
| F-37 Benachrichtigungen | §2.8 Benachrichtigungen | `tauri/notify` (neu) | `ui/shell/Panels.svelte`, `ui/lib/notifications.svelte.ts` | – (Auslöser aus F-09, F-14, F-17, F-34, F-35) | MVP-C |
| F-38 Barrierefreiheit | §1, §5 (alle Screens) | `core/settings` (`reduceMotion`) | `ui/app.css`, alle Komponenten | – | MVP-D |
| F-39 Deep Links und Dateizuordnung | §2.9 Import/Vorschau, §3.1 (Ziel `event/<id>`) | `core/share`, `tauri/open` | `ui/src/App.svelte` (`openExternal`) | – (Links werden lokal aufgelöst, danach Endpunkte von F-10/F-24) | v1-7 |
| F-40 Spielzeit | §2.3 Header (Meta-Chip), §2.2 Zuletzt gespielt | `core/stats` | `ui/library/InstanceView.svelte`, `ui/pages/PlayPage.svelte` | – | v1-4 |
| F-41 Discord Rich Presence | §2.7 Allgemein (Schalter, später) | `tauri/discord` (später) | `ui/settings/GeneralSection.svelte` | extern: lokale Discord-IPC | später |
| F-42 Import aus anderen Launchern | §2.3 Bibliotheksliste („Modpack importieren“ → „Aus anderem Launcher“) | `core/importers` (später) | `ui/ImportDialog.svelte` | – | später |

## Prüfung

- **Funktionen:** 42, `F-01` bis `F-42`, jede genau einmal in der Tabelle. Prüfbar per `grep -c "^| F-" 07-rueckverfolgung.md` → `42`.
- **Phasen:** Die Zuordnung deckt sich mit `05` §9. Phase 0 und MVP-A bis MVP-D enthalten genau die 21 MVP-Funktionen aus `01`.
- **Endpunkte:**
  - Jeder in der Tabelle genannte twogg.app-Endpunkt ist in `04` §4 bzw. §4.2 mit Request, Response und Fehlercodes beschrieben.
  - Jeder Pinecone-Server-Endpunkt steht entweder als Bestand in `02` §8 oder als neu in `04` §6.3.
