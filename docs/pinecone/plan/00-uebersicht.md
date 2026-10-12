# 00 – Übersicht

> Teil des Plans in `docs/pinecone/plan/`. Stand: 12.10.2026, Codebasis Pinecone **0.9.5** (Launcher) und Pinecone-Server **0.9.0**.
> Die Designvorgaben stehen in `docs/design/design-system.md`. Die Aufgabe nennt `docs/pinecone/design-system.md`; diese Datei existiert nicht, gemeint ist dieselbe Datei.

## Wie dieser Plan zu lesen ist

Pinecone ist nicht mehr auf dem Reißbrett: Die Beta 0.9.x läuft mit Instanzen, Modpacks, Events-Seite, Freundesliste, Skins und Auto-Update. Der Plan beschreibt deshalb den **Zielzustand** und markiert für jede Funktion ihren **Ist-Stand** im Code:

| Markierung | Bedeutung |
|---|---|
| **vorhanden** | im Code umgesetzt, mit Fundstelle |
| **teilweise** | angelegt, aber eine Akzeptanzbedingung aus `01` ist nicht erfüllt |
| **fehlt** | nicht im Code |

Die Roadmap (`05`) plant nur die Lücken. Bereits Gebautes wird nicht neu geplant, sondern gegen die Akzeptanzkriterien abgenommen.

| Datei | Inhalt |
|---|---|
| `00-uebersicht.md` | Vision, Zielgruppe, Nicht-Ziele, Alleinstellungsmerkmale |
| `01-funktionen.md` | Funktionskatalog `F-01` bis `F-63` mit Akzeptanzkriterien (`F-43` bis `F-63`: Plan v2) |
| `02-architektur.md` | Tauri-2-Aufbau, Module, IPC, Daten, Downloads, Sicherheit, Releases |
| `03-ui-und-ablaeufe.md` | Screens mit Zuständen, Kernabläufe, Events- und Skins-Seite |
| `04-twogg-app-anbindung.md` | Zuständigkeiten, OAuth 2.0 + PKCE, API-Vertrag, Event-Pack-Manifest, DSGVO |
| `05-roadmap.md` | Phasen MVP → v1 → v2 → später mit Abhängigkeiten und Aufwandsspannen |
| `06-risiken-und-offene-fragen.md` | Externe Hürden mit Quellen, Risiken, `[ungeprüft]`-Sammlung, offene Fragen |
| `07-rueckverfolgung.md` | Funktion → Screen → Modul → Endpunkt → Phase |
| `anhang-datenschutzerklaerung-entwurf.md` | Entwurf der Datenschutzerklärung (juristisch zu prüfen) |

## Vision (3 Sätze)

1. Pinecone bringt TwoGG-Spieler mit einem Klick vom Eventkalender auf den Eventserver: richtiges Modpack, richtige Version, richtiger Server, ohne Anleitung im Discord.
2. Die Anmeldung, das Team und die Gilde bleiben auf twogg.app. Pinecone zeigt deinen Stand und erinnert dich rechtzeitig, statt eine zweite Anmeldestelle zu werden.
3. Für alles außerhalb der Events ist Pinecone ein vollwertiger, warmer und eckiger Minecraft-Launcher mit Modrinth, CurseForge und Mod-Loadern. Er sieht nach TwoGG aus, nicht nach einem Klon.

## Zielgruppe

| Gruppe | Bedarf | Wichtigste Funktionen |
|---|---|---|
| **Event-Teilnehmende** (Kern) | Am Eventabend pünktlich und mit dem richtigen Pack auf dem Server sein | F-07 bis F-16 |
| **Wiederkehrende TwoGG-Spieler** | Zwischen Events eigene Instanzen spielen, Freunde sehen | F-17, F-19 bis F-32 |
| **Neue Community-Mitglieder** | Ohne Vorwissen starten (Microsoft-Login, Java, Modpack) | F-01, F-04, F-06, F-10 |
| **TwoGG-Team** (16 Personen, Bauteam + Orga-Team) | Event-Packs veröffentlichen, Server-Adresse und Neuigkeiten pflegen | Pinecone-Server-Admin (`02` §8), twogg.app-Pflege (`04` §7) |

Systemvoraussetzungen und Plattformen stehen in `02` §10.

## Nicht-Ziele

- **Keine Anmeldung, keine Team-Erstellung, keine Bewerbungen und kein Looking-for-Group im Launcher.** Das macht twogg.app. Pinecone zeigt den Status und verlinkt auf twogg.app (`04` §1).
- **Keine eigene Benutzerdatenbank für TwoGG-Konten.** Das TwoGG-Konto liegt bei twogg.app. Pinecone hält dazu nur Tokens im OS-Schlüsselbund.
- **Keine Gildenwahl im Launcher.** Gildenwahl ist eine Event-Anmeldeentscheidung und gehört zu twogg.app. Pinecone zeigt die gewählte Gilde an.
- **Kein Mod-Hosting für Drittinhalte.** Modrinth- und CurseForge-Dateien kommen direkt von deren Servern. TwoGG hostet nur eigene Event-Packs.
- **Kein Cheat-Client, keine eigenen Ingame-Mods und keine Cosmetics** (Abgrenzung zu NoRisk Client).
- **Keine Werbung, kein Tracking, keine Telemetrie ohne Zustimmung.**
- **Kein Ersatz für den Discord** (twogg.chat). Pinecone verlinkt nur dorthin.
- **Kein Hosting oder Verwalten der Minecraft-Server.** Die Eventserver des Teams sind nicht Teil dieses Plans. Pinecone kennt nur ihre Adresse.

## Alleinstellungsmerkmale

Was Pinecone gegenüber allen vier Vorbildern als einziger bietet:

1. **Event-Kreislauf in einem Werkzeug:** Kalender, Anmeldestatus aus twogg.app, Event-Pack, Countdown und Server-Beitritt hängen zusammen. Der Klick auf „Server beitreten“ startet genau die Instanz, die zum Event gehört (F-13).
2. **Gesperrte Pflicht-Mods mit Integritätsprüfung:** Die Event-Instanz bleibt fair, weil vorgegebene Mods weder abschaltbar noch löschbar sind. Sie werden vor jedem Start gegen ein signiertes Manifest geprüft (F-10, F-11).
3. **Community-Daten statt Werbung:** Die „TwoGG-Auswahl“, Neuigkeiten und der Community-Server werden vom Team gepflegt (F-15, F-21).
4. **Eigenes Design-System:** Rinde/Harz, Schuppen-Schnitt und Pixel-Titel. Es ist vertraut für Minecraft-Spieler, aber kein Klon.
5. **Sync-Packs:** Lieblings-Mods, Configs, Tastenbelegungen und Optionen wandern automatisch in jede Instanz und in jedes Event-Pack, ohne die Pflicht-Mods anzutasten (F-31).

### Vergleichstabelle

Legende: ✔ vorhanden · ◐ eingeschränkt · ✘ nicht vorhanden · ? unbekannt.

Die Spalten der vier Fremdprodukte beschreiben deren Funktionsumfang aus allgemeiner Kenntnis. Sie sind **nicht** gegen die offiziellen Seiten der Hersteller geprüft und gelten deshalb als **[ungeprüft]**, gesammelt in `06` als U-01. Die Spalte Pinecone beschreibt den Zielzustand; in Klammern steht der Ist-Stand, wo er abweicht.

| Merkmal | Offizieller Minecraft Launcher [ungeprüft] | Modrinth App [ungeprüft] | CurseForge App [ungeprüft] | NoRisk Client [ungeprüft] | **Pinecone** |
|---|---|---|---|---|---|
| Microsoft-Login, mehrere Konten | ✔ | ✔ | ✔ | ✔ | ✔ (F-01, F-02) |
| Java automatisch | ✔ | ✔ | ✔ | ✔ | ✔ (F-04) |
| Fabric / Quilt / NeoForge / Forge verwalten | ◐ (nur fertige Profile) | ✔ | ✔ | ◐ (eigener Fabric-Client) | ✔ (F-19) |
| Inhalte aus Modrinth | ✘ | ✔ | ✘ | ◐ | ✔ (F-21) |
| Inhalte aus CurseForge | ✘ | ✘ | ✔ | ✘ | ✔ (F-22, mit API-Schlüssel) |
| `.mrpack` Import/Export | ✘ | ✔ | ? | ? | ✔ (F-24, F-25) |
| Community-Event-Kalender | ✘ | ✘ | ✘ | ◐ (eigene Events) | ✔ (F-08) |
| Anmeldestatus einer externen Community-Plattform | ✘ | ✘ | ✘ | ✘ | ✔ (F-09) |
| Event-Pack mit gesperrten Pflicht-Mods | ✘ | ✘ | ✘ | ✘ | ✔ (F-11; Ist: Sperre nur in der UI) |
| Signiertes Pack-Manifest einer Community | ✘ | ✘ | ✘ | ✘ | ✔ (F-10; Ist: fehlt) |
| Server-Beitritt per Klick aus dem Event | ✘ | ◐ (Server-Instanzen) | ✘ | ? | ✔ (F-13) |
| Countdown und Desktop-Hinweis vor Eventstart | ✘ | ✘ | ✘ | ? | ✔ (F-14; Ist: nur Countdown) |
| Freundesanzeige | ✘ | ✔ | ✘ | ✔ | ✔ (F-17) |
| Sync-Packs (Mods/Optionen in alle Instanzen) | ✘ | ✘ | ✘ | ✘ | ✔ (F-31) |
| Themes und Akzentfarben | ✘ | ◐ | ◐ | ✔ | ✔ (F-33) |
| Werbung im Client | ✘ | ◐ (werbefinanziert) | ✔ | ? | ✘ (Nicht-Ziel) |
| Sprache Deutsch, Du-Form | ✔ | ? | ? | ? | ✔ |

Pinecone gewinnt nicht durch die Menge an Funktionen gegen Modrinth oder CurseForge. Der Unterschied liegt in drei Dingen:

- in den Zeilen „Community-Event-Kalender“ bis „Countdown“,
- im Design,
- darin, dass es keine Werbung gibt.
