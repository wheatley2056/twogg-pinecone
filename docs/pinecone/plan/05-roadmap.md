# 05 – Roadmap

## 1. Ausgangslage

- Pinecone 0.9.5 deckt die meisten Funktionen bereits ab (Ist-Stand je Funktion in `01`).
- Die Roadmap plant daher zwei Arten von Arbeit:
  - **Abnahme:** Vorhandene Funktionen werden gegen ihre Akzeptanzkriterien geprüft. Abweichungen werden behoben.
  - **Lücken:** Was als *teilweise* oder *fehlt* markiert ist, wird gebaut.
- Reihenfolge gemäß Vorgabe:
  1. zuerst der schmale MVP „TwoGG-Events spielen“,
  2. dann der allgemeine Launcher-Umfang (v1.x),
  3. dann „später“.

## 2. Annahmen für die Aufwandsspannen

Alle Spannen sind **Personentage (PT) Entwicklungszeit** und gelten nur unter diesen Annahmen:

1. Es arbeiten [ANZAHL ENTWICKLER] Personen am Launcher. Die Spannen rechnen mit **einer** Person, die Rust, Svelte und das Projekt kennt.
2. Die twogg.app-Arbeiten (T-1 bis T-11 aus `04` §7) macht das twogg.app-Team. Sie sind getrennt ausgewiesen, weil das twogg.app-Repository nicht vorlag. Die Spannen dafür sind deshalb **unsicherer** (Faktor ×2 eingerechnet).
3. Kalenderzeit = PT ÷ verfügbare Tage pro Woche. Bei einem Ehrenamt mit [STUNDEN PRO WOCHE] lässt sich daraus keine Kalenderzeit ableiten, ohne diese Zahl zu kennen.
4. Freigaben Dritter sind **nicht** eingerechnet: Microsoft/Minecraft-App-Freigabe, Zertifikate, CurseForge-Schlüssel. Ihre Wartezeit ist unbekannt (`06`).
5. In den Spannen enthalten: Tests und Review. Nicht enthalten: Gestaltung neuer Grafiken, juristische Prüfung.

## 3. Phasen im Überblick

```
Phase 0  Abnahme Bestand ─────────────┐
                                      ▼
MVP-A  Echte Event-Packs ──┐      MVP-B  twogg.app v1 (braucht T-1…T-8)
   (Signatur, Sperre,      │         (OAuth+PKCE, Cache, Gilde,
    Pflicht-Update)        │          Beitritt, Links)
                           ▼              │
                     MVP-C  Hinweise & Absturz ◄┘
                           ▼
                     MVP-D  Release-Härtung → 1.0 „Events spielen“
                           ▼
v1.x  Allgemeiner Launcher (Abnahme + Lücken) → 1.x
                           ▼
v2-A Komfort ──► v2-B Social ─┐   v2-D Admin (parallel)
                              ▼
                 v2-C Events (braucht T-12…T-14)  → 2.0
                           ▼
später  macOS, Import aus Fremd-Launchern, SSE
```

## 4. Phase 0 – Abnahme des Bestands

- **Ziel:** Für jede Funktion mit Ist-Stand *vorhanden* sind die Akzeptanzkriterien geprüft. Abweichungen sind als Aufgaben erfasst.
- **Funktionen (MVP-relevant):** F-02, F-04, F-05, F-15, F-16, F-33
- **Aufgaben:**
  - Testprotokoll je AK, manuell und über die CLI.
  - Kontrastprüfung als CI-Schritt (AK-33.3, AK-38.3).
  - Altlasten entfernen: `dashboard`-Befehl, `settings.widgets`, `admin/dist` im Launcher-Repo.
  - `tauri.conf.json` `longDescription` um Forge und NeoForge ergänzen.
- **Abhängigkeiten:** keine
- **Aufwand:** 4–8 PT

## 5. MVP – „TwoGG-Events spielen“ (Ziel: Version 1.0)

**Fertig, wenn:**

- Ein neues Community-Mitglied kommt ohne Anleitung vom Erststart bis auf den Eventserver von REVIVAL: Avalon: anmelden, verbinden, Pack installieren, beitreten.
- Alle AK der MVP-Funktionen sind „Ja“.

### MVP-A – Echte Event-Packs

**Umgesetzt (14.10.2026, Launcher-PR #43–#46, Server-PR #13).** Offen ist nur der echte Schlüssel (Q-07): mit `pinecone eventpack keygen` erzeugen und als `PINECONE_EVENTPACK_KEYS` in GitHub und auf dem Server setzen. Das Vorab-Laden von Pflicht-Updates folgt mit MVP-C.

- **Funktionen:** F-10, F-11, F-12
- **Aufgaben Launcher:**
  - Modul `eventpack` (`02` §3.1): Manifest laden, Ed25519 prüfen, `locked` speichern.
  - Integritätsprüfung als Startstufe.
  - Sperre in `content_set_enabled` und `content_remove` (AK-11.1).
  - Hash-Cache (`02` §6).
  - Pack-Cache für die Offline-Reparatur.
  - Pflicht-Update vor dem Start (AK-12.2).
  - `event_pack_status` und `event_pack_install`.
  - SHA-512-Prüfung.
- **Aufgaben Pinecone-Server:**
  - Manifest-Endpunkte (`04` §6.3).
  - Upload mit Signaturprüfung, Felder `required`, `allowExtraMods`, `minLauncherVersion`.
- **Aufgaben Werkzeug:**
  - `pinecone-cli eventpack sign`.
  - Schlüsselpaar erzeugen und sicher ablegen (`06` Q-07).
- **Abhängigkeiten:** Phase 0
- **Aufwand:** Launcher 8–14 PT · Server 3–5 PT · Werkzeug 2–3 PT

### MVP-B – twogg.app-Anbindung v1

- **Funktionen:** F-07, F-08, F-09, F-13, F-18
- **Aufgaben Launcher:**
  - Modul `oauth` (PKCE, Loopback) und `twogg_app_connect`/`_link_minecraft`/`_disconnect`.
  - Discord-WebView und Passwort-Login entfernen.
  - Client-Typ `pinecone`.
  - Modul `eventcache` mit ETag-Polling (`04` §5).
  - Gilde im Status-Chip.
  - `event_join`: Event-Instanz statt Spielleisten-Auswahl (AK-13.1).
  - Bewerbungsbanner und LFG-Link.
  - `/api/v1` mit Rückfall auf die alten Endpunkte bis 1.0.
- **Aufgaben twogg.app:** T-1 bis T-8 (`04` §7)
- **Abhängigkeiten:**
  - T-1 bis T-8 müssen auf einer Testumgebung von twogg.app laufen, bevor die Launcher-Seite abgenommen werden kann.
  - Die Launcher-Seite lässt sich vorher gegen einen nachgebauten Server entwickeln (vorhandene Testhilfe `crates/core/tests/common`).
- **Aufwand:** Launcher 8–13 PT · twogg.app 10–24 PT (unsicher, Annahme 2)

### MVP-C – Hinweise und Absturz

- **Funktionen:** F-14, F-37, F-35
- **Aufgaben:**
  - `tauri-plugin-notification` einbinden (unter Windows nur für installierte Apps, siehe v2.tauri.app „Notification“).
  - Modul `reminders` mit Einstellungen der Vorlaufzeiten.
  - Benachrichtigungsarten Event, Freund, twogg.app-Status.
  - Modul `crash` mit 4 Ursachenregeln und Absturztafel.
  - Vorab-Laden von Pflicht-Updates beim ersten Hinweis.
- **Abhängigkeiten:**
  - MVP-B für Termine und Status aus dem Cache.
  - MVP-A für das Vorab-Laden.
- **Aufwand:** 6–10 PT

### MVP-D – Release-Härtung

- **Funktionen:** F-01, F-06, F-34, F-38
- **Aufgaben:**
  - Freigabe der Azure-App-ID für die Minecraft-Services bestätigen (`06` H-01).
  - Geführter Erststart (AK-06.4, AK-06.5).
  - Updater: periodische Prüfung, keine Installation während des Spiels.
  - Windows-Authenticode einrichten (`02` §10.2).
  - User-Agent mit Kontakt (`02` §5).
  - Tastatur-Prüfliste (`03` §5) abarbeiten.
  - Datenschutzerklärung verlinken (`04` §9.2).
  - „Alle lokalen Daten löschen“.
- **Abhängigkeiten:**
  - MVP-A bis MVP-C abgeschlossen.
  - Zertifikat bzw. Signaturdienst bestellt (Wartezeit unbekannt).
- **Aufwand:** 5–9 PT (ohne Wartezeiten)

**Summe MVP** (ohne Phase 0):

- Launcher, Server und Werkzeug: 32–54 PT
- twogg.app: 10–24 PT

## 6. v1.x – Allgemeiner Launcher

**Fertig, wenn:** Alle AK der v1.x-Funktionen sind „Ja“. Vieles davon ist vorhanden; die Arbeit besteht vor allem aus Abnahme und den genannten Lücken.

| Block | Funktionen | Lücken / Aufgaben | Abhängigkeit | Aufwand |
|---|---|---|---|---|
| v1-1 Instanzen | F-19, F-20, F-32 | Abnahme. AK-20.3 und AK-32.4 umgesetzt (Launcher-PR #46) | MVP-A | 2–4 PT |
| v1-2 Inhalte | F-21, F-22, F-23 | 429-Behandlung. `allowExtraMods` umgesetzt (Launcher-PR #46). CurseForge-Proxy ist umgesetzt (12.10.2026) | MVP-A | 2–4 PT |
| v1-3 Modpacks | F-24, F-25, F-26 | Abnahme. AK-25.3 umgesetzt (Launcher-PR #46) | MVP-A | 1–2 PT |
| v1-4 Spielwelt | F-27, F-28, F-29, F-40 | Abnahme | – | 1–2 PT |
| v1-5 Persönliches | F-30, F-31 | Abnahme, Sync-Packs vs. Pflicht-Mods testen (AK-31.3) | MVP-A | 1–2 PT |
| v1-6 Konten und Freunde | F-03, F-17 | „Für Freunde unsichtbar“ (AK-17.4), Freunde-Daten löschen (`04` §6.3) | – | 3–5 PT |
| v1-7 Betrieb | F-36, F-39 | Deep Link `pinecone://event/<id>` (AK-39.2) | MVP-B | 1–2 PT |
| v1-8 Downloads | (Querschnitt F-04, F-05, F-10, F-21, F-24) | Fortsetzen per `Range` (`02` §6), Begrenzung je Host | – | 3–5 PT |

**Summe v1.x:** 14–26 PT

## 7. v2 – Komfort, Social, Community, Orga

**Fertig, wenn:** Alle AK von F-41 und F-43 bis F-63 sind „Ja“. Reihenfolge: v2-A zuerst (keine externen Abhängigkeiten), v2-B und v2-D parallel, v2-C sobald twogg.app T-12 bis T-14 liefert (setzt Q-01 voraus).

| Block | Funktionen | Aufgaben | Abhängigkeit | Aufwand |
|---|---|---|---|---|
| v2-A Komfort | F-48, F-49, F-50, F-51, F-52, F-53, F-54, F-55 | **Umgesetzt (12.10.2026, PR #24–#30).** Restpunkte: AK-51.2, globale Start-Befehle (F-55), Speicherplatzprüfung (F-48) | v1.x | 1–2 PT Rest |
| v2-B Social | F-41, F-56, F-57, F-58 | **Umgesetzt (12.10.2026, Server-PR #5, Launcher-PR #31–#33).** Rest: AK-56.4 mit MVP-B; Discord-Application-ID anlegen | v1-6 (F-17) | – |
| v2-C Events | F-43, F-44, F-45, F-46, F-47 | Ergebnisse, Lobby (Server-Endpunkt `lobby`), Gilden-Seite, Galerie mit Freigabe, Archiv mit Welt-Download | MVP-B, T-12 bis T-14, Q-15 | 12–20 PT + twogg.app |
| v2-D Admin | F-59, F-60, F-61, F-62, F-63 | **Umgesetzt (13.10.2026, Server-PR #6–#11, Launcher-PR #35, #36).** Server-Pack mit Fabric-Starter, andere Loader mit Installer-Anleitung | – | – |

**Summe v2:** 38–63 PT (ohne Arbeit auf twogg.app-Seite)

## 8. Später (nicht eingeplant)

| Funktion / Thema | Voraussetzung | Grobe Spanne |
|---|---|---|
| F-42 Import aus anderen Launchern | Ordnerformate prüfen (U-08) | 6–12 PT |
| macOS-Auslieferung | Apple-Developer-Account (99 $/Jahr), Testgerät, Notarisierung | 5–10 PT + Wartezeit |
| SSE-Kanal `/api/v1/me/stream` | Bedarf belegt (Q-04), twogg.app-Hosting geeignet | Launcher 2–3 PT, twogg.app unbekannt |
| Teilen-Dienst `pinecone://pack/<code>` | Hosting-Entscheidung | 4–8 PT |
| Hintergrunddienst/Tray für Event-Hinweise bei geschlossenem Launcher | Q-09 | 3–6 PT |

## 9. Abhängigkeiten außerhalb des Codes

| Abhängigkeit | Benötigt für | Wer | Stand |
|---|---|---|---|
| Freigabe der Azure-App-ID für Minecraft-Services | F-01 (alle Logins) | [VERANTWORTLICHE PERSON] | beantragt, wartet (Q-12) |
| twogg.app T-1 bis T-8 | MVP-B | twogg.app-Team | offen |
| Ed25519-Schlüssel und Ablage | MVP-A | Orga-Team | offen (Q-07) |
| Code-Signing (Azure Artifact Signing oder OV-Zertifikat) | MVP-D | [VERANTWORTLICHE PERSON] | offen (U-06) |
| CurseForge | v1-2 | [VERANTWORTLICHE PERSON] | Proxy umgesetzt; Schlüssel beantragen und auf dem Server setzen, Rückfrage Lockfile (Q-06) |
| twogg.app T-12 bis T-14 | v2-C | twogg.app-Team | offen |
| Datenschutzerklärung | MVP-D | Verantwortliche Stelle TwoGG | Entwurf liegt vor, juristisch prüfen und veröffentlichen (Q-08) |

## 10. Zuordnung aller Funktionen zu Phasen

| Phase | Funktionen |
|---|---|
| Phase 0 + MVP | F-01, F-02, F-04, F-05, F-06, F-07, F-08, F-09, F-10, F-11, F-12, F-13, F-14, F-15, F-16, F-18, F-33, F-34, F-35, F-37, F-38 |
| v1.x | F-03, F-17, F-19, F-20, F-21, F-22, F-23, F-24, F-25, F-26, F-27, F-28, F-29, F-30, F-31, F-32, F-36, F-39, F-40 |
| v2 | F-41, F-43, F-44, F-45, F-46, F-47, F-48, F-49, F-50, F-51, F-52, F-53, F-54, F-55, F-56, F-57, F-58, F-59, F-60, F-61, F-62, F-63 |
| später | F-42 |

Die genaue Phase je Funktion (z. B. „MVP-B“) steht in `07`.
