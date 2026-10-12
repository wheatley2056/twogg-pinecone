# 06 – Risiken und offene Fragen

**Kennzeichnung:**

- **H-xx** = externe Hürde
- **R-xx** = Risiko
- **U-xx** = ungeprüfte Angabe
- **Q-xx** = offene Frage

Quellen wurden am **12.10.2026** abgerufen. Das Datum in Klammern ist der Stand der jeweiligen Seite, soweit sie ihn angibt. „Offiziell“ heißt Hersteller oder Normungsgremium; „Community“ heißt z. B. minecraft.wiki.

## 1. Externe Hürden

### H-01 Zulassung der App für den Microsoft-/Minecraft-Login

**Was gilt:**

- **Öffentlicher Client:** Desktop-Apps sind öffentliche Clients ohne Secret. Für den Device-Code-Flow muss „Allow public client flows“ aktiv sein.
  - Quelle (offiziell): https://learn.microsoft.com/en-us/entra/identity-platform/msal-client-applications (Stand 14.05.2025)
- **Device-Code-Flow:** `POST https://login.microsoftonline.com/{tenant}/oauth2/v2.0/devicecode`. Der Nutzer hat standardmäßig 15 Minuten. Abgefragt wird nicht schneller als `interval`.
  - Quelle (offiziell): https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-device-code (Stand 04.01.2025)
- **Tenant und Scope:** Tenant `consumers` und Scope `XboxLive.signin` sind Pflicht; mit `common` oder einer Tenant-ID kommen Fehler.
  - Quelle (Community): https://minecraft.wiki/w/Microsoft_authentication [ungeprüft, U-04]
- **Freigabe neuer Azure-Apps:** Neu angelegte Azure-Apps müssen die Nutzung der Minecraft-API per Formular beantragen. Ohne Freigabe antwortet `api.minecraftservices.com` mit 403.
  - Quelle (Community): minecraft.wiki (s. o.)
  - Das Formular https://aka.ms/mce-reviewappid leitet auf ein Microsoft-Forms-Formular weiter (selbst aufgerufen).
  - Der offizielle Hilfeartikel https://help.minecraft.net/hc/en-us/articles/16254801392141 ließ sich ohne JavaScript nicht lesen. Wortlaut, Kriterien und Bearbeitungsdauer sind daher **ungeprüft** (U-15).
- **Zweite mögliche 403-Ursache:** Ein Thread auf Microsoft Q&A (09.02.2026) berichtet von `403 "Invalid app registration"` bereits beim Xbox-Scope. Die Antwort stammt von einem Freiwilligen, nicht von Microsoft. Welche 403 an welcher Stelle kommt, ist nicht offiziell beschrieben.
  - Quelle: https://learn.microsoft.com/en-gb/answers/questions/5768276/

**Folgen:**

- Ohne freigegebene Client-ID funktioniert kein Microsoft-Login. F-01 ist dann gesperrt (AK-01.5).
- Release-Builds bekommen die ID über `PINECONE_MSA_CLIENT_ID`. Die Freigabe ist beantragt und steht aus (Q-12, Stand 12.10.2026).

**Maßnahme:**

- Freigabestatus festhalten (wer, wann, welche App-ID) und die App-ID in einer Team-Doku ablegen.
- Fehlermeldungen für beide 403-Fälle unterscheiden.

### H-02 CurseForge-API-Schlüssel und nicht verteilbare Mods

**Was gilt:**

- **API:** Basis `https://api.curseforge.com`, Kopfzeile `x-api-key`. Schlüssel für Drittanbieter-Tools gibt es per Antragsformular; Konten, die keinen Schlüssel beantragt haben, werden gelöscht.
  - Quelle (offiziell): https://docs.curseforge.com/rest-api/
- **Antrag:** Er wird von Overwolf geprüft und muss die API-Bedingungen akzeptieren. Die Prüfung berücksichtigt:
  - Auswirkungen auf die Einnahmen der Autoren,
  - Last auf CDN und Servern,
  - ob die Zustimmung der Autoren zur Drittverteilung beachtet wird.
  - Quelle (offiziell): https://support.curseforge.com/support/solutions/articles/9000208346
- **API-Bedingungen** (Stand 14.08.2024), Quelle (offiziell): https://support.curseforge.com/en/support/solutions/articles/9000207405
  - Der Schlüssel ist „non-transferable and may not be shared with any third party“.
  - Daten aus der API dürfen nicht gespeichert oder gecacht werden („save or cache any data obtained through the API“).
  - Kein Produkt, das „competes, directly or indirectly, with CF“.
- **Feld `allowModDistribution`:** Es ist nur als „Is mod allowed to be distributed“ beschrieben.
  - Dass `downloadUrl` genau dann `null` ist, wenn die Verteilung verboten ist, steht **nicht** in der offiziellen Doku. Das beschreiben nur Drittwerkzeuge (U-16).
  - Rate-Limits sind offiziell nicht beziffert (U-16).

**Folgen und Konflikte im Bestand:**

1. Pinecone 0.9.5 baut den Schlüssel in den Client ein (`PINECONE_CURSEFORGE_API_KEY`). Wer das Programm hat, kann ihn auslesen. Das steht in Spannung zu „may not be shared“.
2. `pack.lock.json` speichert CurseForge-Projekt- und Datei-IDs sowie Namen. Ob das als verbotenes „caching“ gilt, ist unklar.
3. Ein Launcher mit CurseForge-Inhalten könnte als „indirekt konkurrierend“ gelten. Ob Overwolf das bei Launchern so sieht, ist offen.

**Maßnahme (umgesetzt, Launcher-PR #22, Server-PR #4):** Weg (a) Proxy.

- Der Schlüssel liegt nur auf dem Pinecone-Server.
- Der Proxy leitet nur vier Abfragen weiter, die Spiel-ID ist fest Minecraft.
- Er speichert nichts und antwortet mit `Cache-Control: no-store`.
- Ohne Schlüssel blendet der Launcher CurseForge aus (AK-22.1).

Offen bleibt der Schlüsselantrag und die Rückfrage zum Lockfile (Q-06).

Nicht verteilbare Dateien werden weiter nie von Fremdquellen geladen (AK-22.2).

### H-03 Modrinth-API-Regeln

**Was gilt:**

- **User-Agent** (Wortlaut): „you must use provide a uniquely-identifying User-Agent header. Providing a user agent that only identifies your HTTP client library … increases the likelihood that we will block your traffic.“ Kontaktangabe empfohlen, Beispiel `github_username/project_name/1.56.0 (launcher.com)`.
- **Rate-Limit:** „currently 300 requests per minute“ je IP. Kopfzeilen `X-Ratelimit-Limit`, `-Remaining`, `-Reset`. Abgeschaltete API-Versionen antworten mit 410.
  - Quelle (offiziell) für beide Punkte: https://docs.modrinth.com/api/
- **Nutzungsbedingungen „API Usage“:** begrenzte, widerrufliche Lizenz.
  - Quelle (offiziell): https://modrinth.com/legal/terms (Stand 13.08.2026)
- **`.mrpack`-Format:**
  - `modrinth.index.json` mit `formatVersion` 1, `files[]` mit `hashes` (SHA-1 **und** SHA-512 Pflicht), `env`, `downloads` (HTTPS), `fileSize`; Ordner `overrides/` und `client-overrides/`.
  - Beim Hochladen auf Modrinth sind nur Downloads von `cdn.modrinth.com`, `github.com`, `raw.githubusercontent.com` und `gitlab.com` erlaubt. Andere Plattformen „SHOULD have a defined set of whitelisted domains“.
  - Die aktuelle Supportseite https://support.modrinth.com/en/articles/8802351 war für Abrufe gesperrt (403). Geprüft wurde stattdessen gegen die archivierte offizielle Formatdefinition (github.com/modrinth/docs, `docs/modpacks/format_definition.md`, Stand 08/2023) und den Quelltext der Modrinth App (`packages/app-lib/src/api/pack/install_from.rs`).

**Folgen:**

- Der User-Agent `pinecone/<version>` ist zu knapp. Ziel ist `TwoGG-Pinecone/<v> (+https://pinecone.twogg.app; [KONTAKT-ADRESSE])` (`02` §5).
- Pinecone hat eine Host-Liste (`crates/core/src/modrinth.rs` `ALLOWED_HOSTS`) und prüft SHA-1. Die Prüfung von SHA-512 kommt mit MVP-A.

### H-04 Minecraft-Nutzungsrichtlinien, Name und Branding

**Was gilt** (offiziell: https://www.minecraft.net/en-us/usage-guidelines und https://www.minecraft.net/en-us/eula, ohne Datumsangabe):

- Nichts darf wirken, als sei es „official or approved by, endorsed by, associated with, supported by, or connected to us“.
- **Pflichthinweis** wie „NOT AN OFFICIAL MINECRAFT [PRODUCT/SERVICE]. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT“, mit Angabe, wer verantwortlich ist und wie man ihn erreicht. „Chat and forum links are not acceptable methods“, ein Discord-Link reicht also nicht.
- **Name:** „Minecraft“ nur als Nebenbezeichnung oder in Beschreibungen, nie als dominanter oder unterscheidender Teil. Keine Bestandteile der Marke als Logo oder im Logo.
- **Weitergabe:** Spiel und Spieldateien nicht weitergeben. Spiel-Downloads und -Updates müssen „come from a source that we authorize“. Mods dürfen verteilt werden, modifizierte Spielversionen nicht.
- Werkzeuge und Dienste sind erlaubt, solange sie nicht offiziell wirken.

**Folgen für Pinecone:**

- **Name „TwoGG Pinecone“:** enthält „Minecraft“ nicht, ✔. Das Logo ist ein eigener Kiefernzapfen, ✔.
- **Pflichthinweis (umgesetzt 12.10.2026):** steht in Einstellungen › Allgemein („Über Pinecone“), auf der Landing-Page (`site/`) und im Fuß der Pack-Seiten. Der Kontakt per E-Mail und das Impressum werden im Admin gepflegt (Einstellungen › Rechtliches) und erscheinen dann automatisch. **Offen:** die Werte eintragen ([KONTAKT-ADRESSE], [URL IMPRESSUM]). Auf der statischen Landing-Page müssen sie von Hand ergänzt werden.
- **Spieldateien:** Pinecone lädt sie direkt von Mojang, ✔.
- **Event-Packs:** Sie dürfen keine Minecraft-JARs oder Mojang-Assets in `overrides/` enthalten. Das prüft der Server beim Upload (neue Prüfregel, MVP-A).

**Zusatzrisiko:** „Pinecone“ ist auch der Name eines Vektordatenbank-Unternehmens (laut `PLAN.md`; deshalb der Zusatz „TwoGG“). Eine Markenprüfung wurde nicht gemacht (U-17).

### H-05 Code-Signing und Vertrauensmeldungen

**Was gilt:**

- **Windows:** Ohne Signatur warnt SmartScreen, die App läuft trotzdem. Dokumentiert sind OV-Zertifikate (nur vor dem 01.06.2023 gekaufte), Azure Key Vault und Azure Artifact Signing (vormals Trusted Signing). EV-Zertifikate bringen seit 2024 keinen sofortigen SmartScreen-Ruf mehr.
  - Quelle (offiziell): https://v2.tauri.app/distribute/sign/windows/ (Stand 11.09.2026)
- **macOS:** Apple-Developer-Konto für 99 $/Jahr. Developer-ID-Zertifikat, Notarisierung Pflicht.
  - Quelle (offiziell): https://v2.tauri.app/distribute/sign/macos/ (Stand 17.05.2026)
- **Updater:** Die Signaturprüfung lässt sich nicht abschalten. Geht der Schlüssel verloren, sind keine Updates für Bestandsinstallationen mehr möglich.
  - Quelle (offiziell): https://v2.tauri.app/plugin/updater/ (Stand 28.11.2025)

**Folgen:**

- Die Kosten für Windows-Signierung sind nicht geprüft (U-06).
- Ob SmartScreen nach der Signierung sofort den Herausgeber ohne Warnung zeigt, hängt vom Ruf ab und ist nicht garantiert (AK-34.4, U-06).

### H-06 Plattformbesonderheiten

- **Benachrichtigungen unter Windows:** „Only works for installed apps“. In der Entwicklung erscheinen Name und Symbol von PowerShell.
  - Quelle (offiziell): https://v2.tauri.app/plugin/notification/ (Stand 15.06.2026)
  - Folge: F-14 lässt sich nur mit installierten Builds testen.
- **Deep Links:** Unter Windows und Linux kommt der Link als Argument eines neuen Prozesses, gebündelt per Single-Instance. Unter macOS gibt es nur Konfiguration und Test im installierten Bundle.
  - Quelle (offiziell): https://v2.tauri.app/plugin/deep-linking/ (Stand 15.06.2026)
- **Schlüsselbund:** `keyring` nutzt Windows Credential Manager, macOS Keychain und Secret Service. Version 4.x ist in `keyring-core` und Plattform-Crates aufgeteilt.
  - Quelle: https://docs.rs/keyring (4.2.0, 29.08.2026)
  - Folge: Linux ohne Secret Service kann keine Konten speichern (`02` §7.3).
- **Microsoft-Redirects:** Microsoft empfiehlt `127.0.0.1` statt `localhost` und unterstützt `[::1]` nicht. Für twogg.app gilt RFC 8252 §8.3 (IP-Literal).
  - Quelle (offiziell): https://learn.microsoft.com/en-us/entra/identity-platform/reply-url (Stand 14.05.2025)

## 2. Lizenzen

| Bestandteil | Lizenz | Pflicht für Pinecone |
|---|---|---|
| Pinecone (Launcher, Server) | MIT OR Apache-2.0 (`Cargo.toml`, `LICENSE-*`) | – |
| MC Skin Editor (vendored, `ui/vendor/mc-skin-editor`) | Apache-2.0 (`LICENSE` im Ordner) | Lizenztext mitliefern, Änderungen kennzeichnen (`VENDORED.md` vorhanden). Im Info-Dialog nennen |
| skinview3d | MIT (`package.json`) | Hinweis im Info-Dialog |
| @jaames/iro | MPL-2.0 (`package.json`) | Datei-Copyleft: Änderungen an iro-Dateien müssten veröffentlicht werden. Unveränderte Nutzung ist unkritisch |
| Schriften Figtree, Pixelify Sans, JetBrains Mono (über @fontsource) | laut Design über Google Fonts angeboten, Lizenz je Schrift [ungeprüft, U-18] | Lizenztexte mitliefern |
| Mods in Event-Packs | je Mod verschieden | Event-Packs referenzieren Mods als Download von erlaubten Hosts. In `overrides/` dürfen nur Dateien liegen, deren Lizenz die Weitergabe erlaubt, und eigene Configs. Prüfung durch das Bauteam je Pack (Checkliste im Admin) |
| Mojang-Spieldateien | EULA | nie weitergeben (H-04) |

**Neue Aufgabe:** Eine Seite „Über Pinecone“ mit Lizenzhinweisen aller Abhängigkeiten, automatisch aus `cargo about`/`license-checker` erzeugt [Werkzeugwahl offen], plus Pflichthinweis H-04.

## 3. Risiken

| ID | Risiko | Wahrscheinlichkeit | Auswirkung | Gegenmaßnahme |
|---|---|---|---|---|
| R-01 | Microsoft-Freigabe fehlt, wird entzogen oder verzögert sich | mittel | kein Login, Launcher unbrauchbar | H-01-Maßnahmen, Status dokumentieren, Offline-Konto nur für Tests |
| R-02 | twogg.app-Team hat keine Kapazität für T-1 bis T-8 | mittel | MVP-B blockiert | Launcher-Seite gegen Nachbau-Server fertigstellen. Übergangsweise bleiben die heutigen Endpunkte, aber **ohne** Passwort-Login (nur Discord-Handoff im Standardbrowser statt WebView, falls twogg.app das kann) |
| R-03 | CurseForge sperrt den Schlüssel | gering (Schlüssel nur noch auf dem Server) | CurseForge-Inhalte fallen aus | Proxy umgesetzt (H-02), CurseForge ist nicht MVP |
| R-04 | Signaturschlüssel für Event-Packs geht verloren oder wird kompromittiert | gering | keine neuen Packs bzw. gefälschte Packs | Schlüssel doppelt gesichert, Rotation (`04` §6.2), Sperrliste per Launcher-Update |
| R-05 | Updater-Schlüssel geht verloren | gering | keine Updates mehr für Bestandsnutzer | Offline-Sicherung bei zwei Personen |
| R-06 | Mojang ändert Startargumente, Auth oder Manifeste | hoch (laufend) | Start bricht für neue Versionen | Mock-Tests, schnelle Patch-Releases, Beta-Kanal für frühe Tests |
| R-07 | Mod-Loader ändern Installer oder Meta | mittel | neue Loader-Versionen starten nicht | wie R-06 |
| R-08 | Kleines Team, Wissen bei einer Person (Bus-Faktor 1) | hoch | Stillstand | Doku (`PLAN.md`, `RELEASING.md`, dieser Plan), Secrets-Verwahrung bei zwei Personen, CI als Sicherheitsnetz |
| R-09 | Event-Abend: viele gleichzeitige Pack-Downloads überlasten den Pinecone-Server | mittel | Spieler kommen zu spät | Pflicht-Update vorab laden (F-14), Packs über ein CDN ausliefern [Option, Kosten offen] |
| R-10 | SmartScreen- und Antivirus-Fehlalarme | mittel | Nutzer brechen die Installation ab | Code-Signing (H-05), Installationsanleitung im Discord |
| R-11 | Datenschutzmängel, z. B. bei Minderjährigen | gering bis mittel | Beschwerden, Vertrauensverlust | `04` §9, juristische Prüfung (Q-08) |
| R-12 | Linux-Fensterrahmen bzw. WebKitGTK-Fehler | mittel | Darstellungsfehler | System-Rahmen als Option (vorhanden), `WEBKIT_DISABLE_DMABUF_RENDERER` |

### Wartungslast für ein kleines Team

| Bereich | Wiederkehrend | Häufigkeit (Annahme) |
|---|---|---|
| Neue Minecraft-Versionen und Snapshots prüfen | Start, Quick Play, Java-Komponente | je Release |
| Loader-Kompatibilität (Fabric, Quilt, Forge, NeoForge) | Installer und Profile testen | je größerem Loader-Update |
| Abhängigkeiten (Rust-Crates, npm, Tauri) | Updates, Sicherheitsmeldungen | monatlich |
| Event-Packs | bauen, signieren, hochladen, testen | je Event |
| API-Änderungen (Modrinth Versionen/410, CurseForge, twogg.app) | Clients anpassen | bei Ankündigung |
| Support | Diagnose-ZIPs auswerten | laufend |

**Empfehlung:** Zuständigkeiten im Team festlegen ([ANZAHL ENTWICKLER] Personen):

- Launcher-Pflege
- Event-Pack-Pflege (Bauteam)
- twogg.app-API (twogg.app-Team)
- Release und Secrets (zwei Personen)

## 4. Sammlung aller `[ungeprüft]`-Stellen

| ID | Angabe | Fundstellen | Grund |
|---|---|---|---|
| U-01 | Funktionsumfang offizieller Minecraft Launcher, Modrinth App, CurseForge App, NoRisk Client | `00` Vergleichstabelle | Nicht gegen offizielle Seiten der Hersteller geprüft |
| U-02 | Mojang-Versionsmanifest und Java-Laufzeit-Manifest (URL mit Hash, Komponentenfeld) | `01` F-04, `02` §5 | Nur Community-Doku bzw. selbst abgerufen, keine offizielle Mojang-Doku |
| U-03 | Installer-URL-Muster Forge/NeoForge | `02` §5 | Nur per Abruf bestätigt, nicht dokumentiert |
| U-04 | Microsoft-Kette Xbox → XSTS → Minecraft Services, Tenant `consumers`, Scope `XboxLive.signin`, XSTS-`XErr`-Codes, Minecraft-Token-Laufzeit 86400 s | `02` §7.1, `06` H-01 | Nur minecraft.wiki (Community) |
| U-05 | Größengrenze für Geheimnisse im Windows Credential Manager | `02` §7.3 | Keine offizielle Quelle gefunden |
| U-06 | Kosten Windows-Code-Signing, SmartScreen-Anzeige nach Signierung | `01` AK-34.4, `02` §10.2, `06` H-05 | Preise nicht recherchiert. Ruf-Aufbau nicht garantiert |
| U-07 | Mojang-Session `join`/`hasJoined` als Besitzbeweis, Längengrenze `serverId` | `04` §2.4, `04` §4 (`McChallenge`) | Nur Community-Doku. Funktioniert aber im Pinecone-Server |
| U-08 | Ordnerformate von Prism, MultiMC, Modrinth App, CurseForge App, offiziellem Launcher | `01` F-42 | Nicht untersucht (Phase „später“) |
| U-09 | Hosting, Framework und Proxy-Timeouts von twogg.app/api.twogg.app | `04` Einleitung, `04` §5, §7 | Nur Kopfzeilen beobachtet, Repo nicht gelesen |
| U-10 | Quick-Play-Argumente (`--quickPlayMultiplayer` ab 1.20/23w14a, Entfall `--server`/`--port`) | `02` §5 | Nur minecraft.wiki. Im Client-JSON 26.3 selbst gesehen |
| U-11 | Einzelabschnitte von RFC 8252 §8.4, RFC 7009, RFC 9457, RFC 8785, RFC 6749 §4.1.2.1 | `04` §2.2, §3, §4, §4.2, §6.2 | Nur RFC 8252 (§7.1, §7.3, §8.3, §8.12) und RFC 7636 wurden gelesen |
| U-12 | Eignung von `oidc-provider` für twogg.app | `04` §7 | Nur als Kandidat genannt |
| U-13 | Ob der Installer beim Deinstallieren Schlüsselbund-Einträge entfernt | `04` §9.1 | Nicht getestet |
| U-14 | Java-Laufzeiten für `windows-arm64` | `02` §10.1 | Nur Schlüssel im Manifest gesehen |
| U-15 | Wortlaut, Kriterien und Dauer der Minecraft-API-Freigabe | `06` H-01 | Offizieller Hilfeartikel nicht lesbar |
| U-16 | `downloadUrl = null` bei verbotener Verteilung, Rate-Limits der CurseForge-API | `06` H-02 | Nicht in offizieller Doku |
| U-17 | Markenrechtliche Lage des Namens „Pinecone“ | `06` H-04 | Keine Markenrecherche |
| U-18 | Lizenzen der drei Schriften | `06` §2 | Nicht je Schrift nachgesehen |

## 5. Offene Fragen

| ID | Frage | Warum offen | Wer entscheidet | Betrifft |
|---|---|---|---|---|
| Q-01 | Wie sehen die echten twogg.app-Datenmodelle aus (Nutzer, Event, Registrierung, Team, Gilde)? Wie entstehen `mcUuid`/`mcUsername` heute und sind sie geprüft? | Das Repo ist bekannt (`https://git.core01.twogg.de/TwoGG/twogg-app.git`), aber die Netzwerkfreigabe der Arbeitsumgebung blockiert den Host (Stand 12.10.2026). Der Vertrag in `04` stützt sich bis dahin nur auf die vom Launcher gelesenen Felder | Abgleich, sobald der Host freigegeben ist | `04` §4, T-3 bis T-5 |
| Q-02 | ~~Wo wird ein Event-Pack einem Event zugeordnet?~~ **Entschieden (12.10.2026):** in der Admin-Oberfläche des Pinecone-Servers (`pinecone.twogg.app/admin`, Repo `wheatley2056/twogg-pinecone-server`) über das Feld `eventId`. twogg.app braucht kein Feld `pineconePackId` | – | Orga-Team | F-10, `02` §8 |
| Q-03 | Welche Statuswerte gibt es in twogg.app wirklich (Registrierung, Team, Vormerkung, Einladung, Bewerbung), und wie werden sie zu `Participation.status` abgebildet? | Statuswerte der heutigen Antwort sind im Launcher nur teilweise ausgewertet, die Wertemenge ist unbekannt | twogg.app-Team | F-09 |
| Q-04 | Reicht Polling (60 s vor Events), oder braucht das Orga-Team sofortige Freigaben (SSE)? | Bedarf nicht bekannt | Orga-Team | `04` §5 |
| Q-05 | Werden die Pinecone-Freunde (Minecraft-Identität, Pinecone-Server) mit den twogg.app-Freunden zusammengeführt, oder bleiben es zwei Listen? | Zwei Systeme existieren. Der twogg.app-Endpunkt ist im Launcher angebunden, aber ohne UI | Orga-Team | F-17 |
| Q-06 | ~~Wie wird CurseForge legal angebunden?~~ **Umgesetzt (12.10.2026):** Der Schlüssel liegt nur auf dem Pinecone-Server (`PINECONE_CURSEFORGE_API_KEY`), der Launcher fragt über dessen Proxy `/api/v1/curseforge/...`; Release-Builds enthalten keinen Schlüssel mehr, der Proxy speichert nichts (`no-store`). **Noch offen:** einen eigenen Schlüssel für „TwoGG Pinecone“ über das CurseForge-Formular beantragen; bei Overwolf nachfragen, ob das Speichern von Projekt-/Datei-IDs in `pack.lock.json` (Daten der Nutzer-Installation) als „caching“ gilt | Die Auslegung der Bedingungen für Launcher ist nicht öffentlich dokumentiert | [VERANTWORTLICHE PERSON] mit Overwolf | F-22, F-26, H-02 |
| Q-07 | Wer verwahrt den Ed25519-Schlüssel für Event-Packs, und wer darf signieren? | Organisatorische Entscheidung | Orga-Team | F-10, `04` §6.2 |
| Q-08 | Rechtsgrundlagen, Aufbewahrungsfristen, Umgang mit Minderjährigen, Verantwortliche Stelle | **Teilweise umgesetzt (12.10.2026):** Löschen der Freunde-Daten durch den Spieler, automatische Löschung inaktiver Profile nach 365 Tagen, Felder für Impressum/Datenschutz/Kontakt im Admin, Entwurf einer Datenschutzerklärung (`anhang-datenschutzerklaerung-entwurf.md`). Offen bleiben Verantwortliche Stelle, juristische Prüfung des Entwurfs und Minderjährige | Verantwortliche Stelle TwoGG | `04` §9 |
| Q-09 | Sollen Event-Hinweise auch bei geschlossenem Launcher kommen (Tray, Autostart)? | Abwägung Nutzen gegen „läuft im Hintergrund“ | Orga-Team | F-14 |
| Q-10 | Datenordner unter Windows: `%APPDATA%` (Roaming, heute) oder `%LOCALAPPDATA%`? | Roaming-Profile in Schul- und Firmennetzen könnten große Assets synchronisieren. Ein Umzug bricht bestehende Installationen ohne Migration | Launcher-Team | `02` §9.1 |
| Q-11 | Fehler- bzw. Gefahrenfarbe: ins Design-System aufnehmen oder `--danger` aus dem Code entfernen? | Das Design-System kennt keine Fehlerfarbe, der Code nutzt `--danger` | Design-Verantwortliche | `03` §1 |
| Q-12 | Ist die in Releases verwendete Azure-App-ID für die Minecraft-Services freigegeben? | **Beantragt, Freigabe steht aus (Stand 12.10.2026).** Bis dahin funktioniert der Microsoft-Login mit dieser App-ID nicht | [VERANTWORTLICHE PERSON] | F-01, H-01 |
| Q-13 | Ist die Gilde in REVIVAL: Avalon ein eigenes Datenfeld oder ein Team mit Typ „Gilde“? | Datenmodell unbekannt (Q-01) | twogg.app-Team | F-09, `Participation.guild` |
| Q-14 | Server-Adressen: Wo werden `[SERVER-ADRESSE]` (Community-Server) und die Eventserver gepflegt, und wer darf sie ändern? | Heute Admin des Pinecone-Servers bzw. twogg.app `/join`. Zuständigkeit nicht schriftlich festgelegt | Orga-Team | F-13, F-16 |

## 6. Abweichungen der Design-Referenz von festen Entscheidungen

| Stelle | Abweichung | Umgang im Plan |
|---|---|---|
| `design-system.md` §10.1 und `referenz/01-spielen.html`: Button „Gilde wählen“ | widerspricht „keine Anmeldelogik im Launcher“ | entfällt, wird zu „Zum Event“. Die Gilde wird nur angezeigt (`03` §2.2). Im Code schon so umgesetzt (CHANGELOG 0.9.1) |
| `design-system.md` §10: „Events-Seite (Anmeldung, Gildenwahl, Countdown)“ | Anmeldung und Gildenwahl gehören zu twogg.app | Events-Seite zeigt Status und verlinkt (`03` §3.1) |
| Pfad `docs/pinecone/design-system.md` in der Aufgabe | die Datei liegt unter `docs/design/design-system.md` | genutzt wurde die vorhandene Datei, nichts verschoben |
