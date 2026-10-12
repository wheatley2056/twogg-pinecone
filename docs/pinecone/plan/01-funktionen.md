# 01 – Funktionen

## Einordnung

- **IDs** `F-01` bis `F-63` sind stabil. `F-43` bis `F-63` sind der Plan v2 (Abschnitte E–H, Stand 12.10.2026). `07` ordnet jede ID genau einmal einem Screen, Modul, Endpunkt und einer Phase zu.
- **Priorität**
  - **MVP**: nötig für „TwoGG-Events spielen“.
  - **v1.x**: allgemeiner Launcher-Umfang.
  - **v2**: nach 1.0, eingeplant in `05` §7 (Blöcke v2-A bis v2-D).
  - **später**: nach v1, nicht eingeplant.
- **Ist-Stand** ist der Code von 0.9.5: *vorhanden*, *teilweise* oder *fehlt*. Bei *teilweise* steht dabei, welches Akzeptanzkriterium noch nicht erfüllt ist.
- **Akzeptanzkriterien (AK)** lassen sich mit Ja/Nein prüfen. Eine Funktion gilt als fertig, wenn alle AK „Ja“ ergeben.
- **Randfälle** decken immer drei Fälle ab: *offline* (kein Netz oder Gegenstelle nicht erreichbar), *Abbruch* (Nutzer oder System bricht mitten im Vorgang ab) und *Konflikt* (widersprüchliche Zustände). Gibt es für eine Funktion einen dieser Fälle nicht, steht dort „entfällt“ mit Grund.
- **UI-Texte** stehen in Du-Form in Anführungszeichen. Sie sind Vorschläge; maßgeblich sind die Locale-Dateien `ui/src/lib/locales/de.ts` und `en.ts`.
- Begriffe:
  - *Instanz* = Profil mit eigenem Spielordner.
  - *Event-Instanz* = Instanz, deren `pack.lock.json` als Quelle `twogg` trägt.
  - *Pflicht-Mod* = Datei, die das Event-Pack vorgibt.

---

## A. Konto, Java, Start

### F-01 Microsoft-Login

- **Nutzerziel:** Ich melde mich mit meinem Microsoft-Konto an, damit ich Minecraft online spielen kann.
- **Verhalten:**
  1. „Konto hinzufügen“ startet den OAuth-Device-Code-Flow (`02` §7.1).
  2. Pinecone zeigt den Code und öffnet die Microsoft-Seite im Standardbrowser.
  3. Danach läuft die Kette Xbox Live → XSTS → Minecraft Services → Profil durch.
  4. Der Refresh-Token landet im OS-Schlüsselbund, Profilname und UUID in `accounts.json`.
- **Priorität:** MVP
- **Ist-Stand:** vorhanden (`crates/core/src/msa.rs`, `accounts.rs`; `LoginDialog.svelte`). Die Client-ID wird per `PINECONE_MSA_CLIENT_ID` eingebaut.
- **Akzeptanzkriterien:**
  - AK-01.1 Nach erfolgreichem Login erscheint das Konto mit Minecraft-Name und Kopf in der Spielleiste.
  - AK-01.2 Nach einem Neustart des Launchers ist das Konto ohne erneuten Login aktiv.
  - AK-01.3 Kein Access- oder Refresh-Token steht in einer Datei im Datenordner (Prüfung per Volltextsuche).
  - AK-01.4 Ein Konto ohne Minecraft-Lizenz wird abgelehnt mit dem Text „Dieses Microsoft-Konto besitzt Minecraft: Java Edition nicht.“
  - AK-01.5 Ist keine Client-ID konfiguriert, ist „Microsoft-Konto hinzufügen“ deaktiviert und mit Begründung versehen.
- **Randfälle:**
  - *Offline:* Login startet nicht, Hinweis „Keine Verbindung zu Microsoft“. Vorhandene Konten bleiben.
  - *Abbruch:* „Abbrechen“ oder Ablauf des Codes beendet das Abfragen. Es wird kein halbes Konto gespeichert.
  - *Konflikt:* Dasselbe Minecraft-Konto wird erneut hinzugefügt. Der vorhandene Eintrag wird aktualisiert und nicht verdoppelt.

### F-02 Mehrfachkonten und Kontowechsel

- **Nutzerziel:** Ich wechsle zwischen mehreren Minecraft-Konten, ohne mich neu anzumelden.
- **Verhalten:**
  - Das Kontomenü in der Spielleiste listet alle Konten. Ein Klick macht ein Konto aktiv.
  - „Entfernen“ löscht das Konto samt Schlüsselbund-Eintrag.
  - Die Freundesliste (F-17) und Skins (F-30) folgen dem aktiven Konto.
- **Priorität:** MVP
- **Ist-Stand:** vorhanden (`accounts.rs`; `AccountMenu.svelte`, `settings/AccountsSection.svelte`)
- **Akzeptanzkriterien:**
  - AK-02.1 Mit zwei Konten startet das Spiel mit dem Namen des aktiven Kontos.
  - AK-02.2 Nach „Entfernen“ ist der Schlüsselbund-Eintrag `msa-refresh:<uuid>` weg.
  - AK-02.3 Läuft ein Spiel, lässt sich das aktive Konto nicht entfernen. Der Knopf ist dann deaktiviert.
- **Randfälle:**
  - *Offline:* Wechsel möglich, Start scheitert erst beim Token-Refresh (siehe F-05).
  - *Abbruch:* entfällt, weil der Wechsel atomar ist.
  - *Konflikt:* Ist das aktive Konto beim Start abgelaufen (Refresh-Token ungültig), erscheint „Bitte melde dich erneut an“ mit Knopf zum Login.

### F-03 Offline-Konto (Test und LAN)

- **Nutzerziel:** Ich teste eine Instanz ohne Microsoft-Konto oder spiele im LAN.
- **Verhalten:** Ein Name wird eingegeben. Die UUID entsteht deterministisch aus `OfflinePlayer:<Name>`.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`auth.rs`)
- **Akzeptanzkriterien:**
  - AK-03.1 Das Offline-Konto ist in der Kontoliste als „Offline“ gekennzeichnet.
  - AK-03.2 Mit aktivem Offline-Konto sind „Server beitreten“ bei Events (F-13) und die Freundesliste (F-17) deaktiviert, mit Hinweis.
  - AK-03.3 Derselbe Name ergibt immer dieselbe UUID.
- **Randfälle:**
  - *Offline:* Das ist der Zweck der Funktion.
  - *Abbruch:* entfällt.
  - *Konflikt:* Name ist schon als Offline-Konto vorhanden, dann wird es nicht doppelt angelegt.

### F-04 Java-Laufzeit-Verwaltung

- **Nutzerziel:** Ich muss mich nicht um Java kümmern.
- **Verhalten:**
  - Vor dem Start bestimmt Pinecone die Java-Hauptversion aus dem Versions-JSON von Minecraft.
  - Eine passende verwaltete Laufzeit wird genutzt. Fehlt sie, wird Mojangs Java-Laufzeit heruntergeladen. Die Quelle ist ein Community-dokumentierter Endpunkt, siehe `02` §5 [ungeprüft].
  - In den Einstellungen sieht man alle gefundenen Javas. Pro Instanz lässt sich ein eigener Pfad setzen.
- **Priorität:** MVP
- **Ist-Stand:** vorhanden (`java.rs`; `settings/JavaSection.svelte`)
- **Akzeptanzkriterien:**
  - AK-04.1 Ohne installiertes Java startet eine 1.21-Instanz nach automatischem Download.
  - AK-04.2 Die heruntergeladenen Dateien werden per Hash geprüft (siehe `02` §6).
  - AK-04.3 Ein ungültiger eigener Java-Pfad führt vor dem Start zur Meldung „Java unter <Pfad> lässt sich nicht starten“, nicht zu einem Spielabsturz.
- **Randfälle:**
  - *Offline:* Fehlt die Laufzeit, kommt die Meldung „Java <n> wird benötigt und kann gerade nicht geladen werden“. Ist eine passende Laufzeit vorhanden, startet das Spiel.
  - *Abbruch:* Ein abgebrochener Download hinterlässt nur `.part`-Dateien. Die Laufzeit gilt als nicht installiert.
  - *Konflikt:* Eine System-Java hat dieselbe Hauptversion wie eine verwaltete Laufzeit. Verwaltete Laufzeiten haben Vorrang.

### F-05 Spielstart mit Startvorbereitung

- **Nutzerziel:** Ich drücke SPIELEN und das Spiel startet, egal was vorher fehlt.
- **Verhalten:** SPIELEN in der Spielleiste startet die gewählte Instanz. Die Stufen laufen sichtbar mit Fortschritt ab:
  1. Sync-Packs abgleichen
  2. Event-Integrität prüfen (F-11)
  3. Loader installieren
  4. Version herunterladen
  5. Java bereitstellen
  6. Assets laden
  7. Starten

  Während das Spiel läuft, wird SPIELEN zu „Beenden“.
- **Priorität:** MVP
- **Ist-Stand:** vorhanden (`launcher.rs` `prepare`, `src-tauri/src/game.rs`; `PlayBar.svelte`). Die Stufe „Event-Integrität prüfen“ fehlt, siehe F-11.
- **Akzeptanzkriterien:**
  - AK-05.1 Jede Stufe ist als Text in der Spielleiste oder im Downloads-Panel sichtbar.
  - AK-05.2 Eine Instanz kann nicht zweimal gleichzeitig gestartet werden.
  - AK-05.3 „Beenden“ beendet den Spielprozess innerhalb von 10 s. Erst wird normal beendet, danach hart.
  - AK-05.4 Nach Spielende wird die Spielzeit gespeichert (F-40).
- **Randfälle:**
  - *Offline:* Mit Microsoft-Konto scheitert der Token-Refresh und es erscheint „Für den Start mit Microsoft-Konto brauchst du Internet“. Mit Offline-Konto und vollständig installierter Version startet das Spiel.
  - *Abbruch:* Abbrechen in einer Download-Stufe stoppt sauber, der nächste Start setzt fort (siehe `02` §6).
  - *Konflikt:* Wird die Instanz während der Vorbereitung bearbeitet (Version geändert), bricht der Start mit Hinweis ab.

### F-06 Erststart und Intro-Animation

- **Nutzerziel:** Der erste Start fühlt sich nach Pinecone an und führt mich zum ersten Spiel.
- **Verhalten:**
  - Das Intro läuft 5 s in einem eigenen rahmenlosen Fenster: Harz-Orange → Farbwischer Nadel/Zapfen/Pergament/Nacht → farbiges Logo → „Pinecone“ tippt sich ein → „A TWOGG LAUNCHER“.
  - Danach öffnet sich das Hauptfenster. Beim allerersten Start gibt es die Willkommensansicht mit den Schritten Konto → TwoGG verknüpfen (optional) → Instanz oder Event wählen.
- **Priorität:** MVP
- **Ist-Stand:** teilweise. Das Intro ist vorhanden (`splash.rs`, `ui/splash/`). Es fehlt der geführte Erststart mit TwoGG-Verknüpfung; heute gibt es nur die Willkommensansicht mit „Instanz anlegen“.
- **Akzeptanzkriterien:**
  - AK-06.1 Das Intro dauert höchstens 5,5 s und lässt sich per Klick oder Esc überspringen.
  - AK-06.2 Mit „Animationen reduzieren“ (F-38) oder `prefers-reduced-motion` zeigt das Intro ein Standbild für höchstens 1 s.
  - AK-06.3 Mit „Intro anzeigen: aus“ öffnet sich das Hauptfenster direkt.
  - AK-06.4 Beim Erststart ohne Konto führt die Willkommensansicht in höchstens 3 Schritten zum ersten SPIELEN.
  - AK-06.5 Der Schritt „TwoGG verknüpfen“ lässt sich überspringen.
- **Randfälle:**
  - *Offline:* Das Intro läuft lokal. Der Erststart bietet „Später anmelden“ an.
  - *Abbruch:* Wird der Erststart geschlossen, kommt beim nächsten Start der zuletzt offene Schritt.
  - *Konflikt:* Das Intro-Fenster hängt (WebView-Fehler). Ein Zeitlimit öffnet das Hauptfenster trotzdem (vorhanden: Fallback-Timeout in `splash.rs`).

---

## B. TwoGG-Events

### F-07 TwoGG-Konto verknüpfen

- **Nutzerziel:** Pinecone kennt mein TwoGG-Konto, damit ich meinen Anmeldestatus sehe.
- **Verhalten:**
  1. „Mit twogg.app verbinden“ öffnet den Standardbrowser auf dem Autorisierungs-Endpunkt von twogg.app: OAuth 2.0 Authorization Code mit PKCE, Loopback-Redirect (`04` §2).
  2. Der Launcher tauscht den Code gegen Tokens und speichert den Refresh-Token im Schlüsselbund.
  3. Danach beweist Pinecone twogg.app den Besitz der Minecraft-UUID des aktiven Kontos (Mojang-Session-Beweis, `04` §2.4).
  4. Trennen widerruft den Token.
- **Priorität:** MVP
- **Ist-Stand:** teilweise. Heute gibt es zwei Wege:
  - Discord-Login in einem eingebetteten WebView mit Handoff-Token über `twogg://auth/handoff`.
  - E-Mail und Passwort direkt im Launcher.

  Beides widerspricht der festen Entscheidung „OAuth 2.0 + PKCE, twogg.app als Auth-Server“. Außerdem tritt Pinecone als Client-Typ `desktop` auf (`crates/core/src/twogg_app.rs`).
- **Akzeptanzkriterien:**
  - AK-07.1 Pinecone fragt nie nach dem twogg.app-Passwort. Es gibt kein Passwortfeld im Launcher.
  - AK-07.2 Der Login läuft im Standardbrowser, nicht in einem eingebetteten WebView.
  - AK-07.3 Der Autorisierungsantrag enthält `code_challenge_method=S256`, `state` und einen Loopback-`redirect_uri`.
  - AK-07.4 Der Refresh-Token liegt nur im Schlüsselbund. Der Access-Token liegt nur im Speicher.
  - AK-07.5 Nach der Verknüpfung zeigt Einstellungen › TwoGG-Verknüpfung den TwoGG-Namen und die verknüpfte Minecraft-UUID.
  - AK-07.6 „Trennen“ ruft den Revoke-Endpunkt auf und löscht den Schlüsselbund-Eintrag.
  - AK-07.7 Pinecone meldet sich gegenüber twogg.app als `X-Twogg-Client: pinecone` mit eigener Version, nicht als `desktop`.
- **Randfälle:**
  - *Offline:* Der Knopf ist deaktiviert und zeigt „twogg.app ist gerade nicht erreichbar“.
  - *Abbruch:* Wird der Browser geschlossen, beendet sich der Loopback-Listener nach 5 min. Es bleibt kein Token übrig.
  - *Konflikt:* Die Minecraft-UUID ist bereits mit einem anderen TwoGG-Konto verknüpft. twogg.app antwortet mit `409 mc_uuid_taken`; Pinecone zeigt „Dieses Minecraft-Konto gehört schon zu einem anderen TwoGG-Konto. Melde dich im Discord beim Orga-Team.“ Verknüpftes und aktives Minecraft-Konto weichen voneinander ab: Hinweis in der Events-Seite (F-09).

### F-08 Events-Seite: Liste und Details

- **Nutzerziel:** Ich sehe alle TwoGG-Events mit Termin, Art und Infos an einem Ort.
- **Verhalten:**
  - Der Tab „Events“ zeigt links kommende und vergangene Events, rechts das Detail.
  - Das Detail zeigt Banner, Typ, Minecraft-Version, Teamgröße, Countdown, Event-Pack, „Dein Stand“ und Server.
  - Die Daten kommen ausschließlich aus twogg.app (`GET /api/v1/events`, `04` §4).
- **Priorität:** MVP
- **Ist-Stand:** vorhanden (`EventsPage.svelte`, `twogg_app.rs`). Abweichung: Daten werden nur im Speicher gehalten, Cache fehlt (AK-08.3).
- **Akzeptanzkriterien:**
  - AK-08.1 Kommende Events stehen nach Datum aufsteigend oben, beendete gedämpft darunter.
  - AK-08.2 Ein Event ohne Termin zeigt „Termin folgt“.
  - AK-08.3 Ohne Netz zeigt die Seite den zuletzt geladenen Stand mit dem Hinweis „Stand von <Zeit>“.
  - AK-08.4 Emojis aus twogg.app-Titeln werden nicht angezeigt (Designregel „keine Emojis“).
  - AK-08.5 Die Seite funktioniert ohne verknüpftes TwoGG-Konto. Nur „Dein Stand“ fehlt dann.
- **Randfälle:**
  - *Offline:* siehe AK-08.3. Ohne Cache erscheint der Leerzustand „Events lassen sich gerade nicht laden“.
  - *Abbruch:* entfällt, weil nur gelesen wird.
  - *Konflikt:* twogg.app liefert ein Event, das der Cache noch nicht kennt, oder löscht eines. Die Serverantwort ersetzt den Cache vollständig.

### F-09 Anmelde-, Team- und Gildenstatus

- **Nutzerziel:** Ich sehe, ob ich angemeldet bin, in welchem Team und in welcher Gilde, ohne die Website zu öffnen.
- **Verhalten:**
  - Pro Event zeigt Pinecone „Dein Stand“ als Meta-Chip: Angemeldet / Vorgemerkt / Einladung offen / Bewerbung in Prüfung / Abgelehnt / Nicht angemeldet.
  - Dazu kommen der Teamname und für REVIVAL: Avalon die Gilde („Die Treue Hand“ oder „Der Phönixbund“).
  - Jede Änderung führt zu twogg.app, per Link „Auf twogg.app ändern“.
- **Priorität:** MVP
- **Ist-Stand:** teilweise. Status und Teamname sind vorhanden. Die Gilde fehlt, weil die twogg.app-API kein Gildenfeld liefert (`04` §7).
- **Akzeptanzkriterien:**
  - AK-09.1 Jeder Status aus `04` §4 Schema `Participation.status` hat einen eigenen sichtbaren Text.
  - AK-09.2 Ist eine Gilde gesetzt, steht ihr Name im Chip.
  - AK-09.3 Es gibt im Launcher keinen Knopf, der eine Anmeldung, Team- oder Gildenänderung auslöst. Nur Links nach twogg.app.
  - AK-09.4 Ändert sich der Status auf twogg.app, zeigt Pinecone ihn spätestens nach dem nächsten Abfrageintervall (`04` §5).
- **Randfälle:**
  - *Offline:* Gezeigt wird der letzte bekannte Status mit „Stand von <Zeit>“.
  - *Abbruch:* entfällt.
  - *Konflikt:* Das Token ist abgelaufen und der Refresh scheitert (`401`). Der Status verschwindet und „Verbindung zu twogg.app erneuern“ erscheint. Bei mehreren Teilnahmen am selben Event gilt die Reihenfolge Team > Angemeldet > Vorgemerkt > Einladung.

### F-10 Event-Instanz aus signiertem Manifest installieren

- **Nutzerziel:** Ich installiere das Event-Modpack mit einem Klick und kann sicher sein, dass es echt ist.
- **Verhalten:**
  - Die Events-Seite zeigt beim Event „Event-Pack installieren“.
  - Pinecone holt das Event-Pack-Manifest (`04` §6) und prüft:
    - die Ed25519-Signatur gegen den eingebauten TwoGG-Schlüssel,
    - das `eventId`,
    - die Mindest-Launcher-Version.
  - Danach lädt Pinecone das `.mrpack`, prüft SHA-512 und Größe und installiert es als Event-Instanz.
  - Das Event-Pack bekommt das Event-Symbol und erscheint in der Bibliothek unter „TwoGG Events“.
- **Priorität:** MVP
- **Ist-Stand:** teilweise. Installation, SHA-1-Prüfung und HMAC-Downloadlink sind vorhanden (`modpack.rs` `download_twogg`). Es fehlen die Signatur, SHA-512 und die Mindestversion.
- **Akzeptanzkriterien:**
  - AK-10.1 Ein Manifest mit ungültiger oder fehlender Signatur wird abgelehnt mit „Dieses Pack ist nicht von TwoGG signiert“. Es wird keine Datei geschrieben.
  - AK-10.2 Ein `.mrpack`, dessen SHA-512 nicht zum Manifest passt, wird verworfen.
  - AK-10.3 Ist die Launcher-Version kleiner als `minLauncherVersion`, erscheint „Bitte aktualisiere Pinecone“ mit Knopf zu F-34.
  - AK-10.4 Die fertige Instanz trägt im `pack.lock.json` `pack.source.type = "twogg"`, `packId`, `eventId` und die Manifestversion.
  - AK-10.5 Ist die Instanz zu einem Event schon installiert, zeigt der Knopf „Instanz öffnen“ statt „installieren“.
- **Randfälle:**
  - *Offline:* Der Knopf ist deaktiviert.
  - *Abbruch:* Ein Abbruch während der Installation entfernt den halb angelegten Instanzordner.
  - *Konflikt:* Das Manifest nennt eine Datei, die es auf Modrinth nicht mehr gibt (404). Die Installation bricht ab mit „Datei <Name> fehlt, bitte melde das dem Bauteam“.

### F-11 Pflicht-Mods sperren und Integritätsprüfung

- **Nutzerziel:** (Team) Alle spielen das Event mit identischen Pflicht-Mods. (Spieler) Ich kann meine Event-Instanz nicht aus Versehen kaputt machen.
- **Verhalten:**
  - In Event-Instanzen tragen Pflicht-Mods in der Inhaltstabelle ein Schloss statt eines Schalters und lassen sich weder abschalten noch löschen.
  - Eigene Zusatzinhalte sind erlaubt, wenn das Manifest es zulässt (`allowExtraMods`, `04` §6).
  - Vor jedem Start (F-05, Stufe 2) prüft das Backend alle Pflichtdateien gegen die Hashes des Manifests:
    - Fehlende oder veränderte Dateien werden aus dem Cache oder neu geladen.
    - Unerlaubte Zusatz-Mods werden deaktiviert (`.disabled`) und gemeldet.
- **Priorität:** MVP
- **Ist-Stand:** teilweise. Die Sperre gibt es nur in der UI (`ContentTable.svelte` `isLocked`). Backend (`content_set_enabled`, `content_remove`), CLI und Start prüfen nichts.
- **Akzeptanzkriterien:**
  - AK-11.1 `content_set_enabled` und `content_remove` lehnen Pflicht-Mods einer Event-Instanz mit Fehler ab, auch über die CLI.
  - AK-11.2 Wird eine Pflicht-Mod im Dateisystem gelöscht oder verändert, ist sie nach dem nächsten SPIELEN wieder im Originalzustand.
  - AK-11.3 Eine unerlaubte Zusatz-Mod in `mods/` ist nach SPIELEN deaktiviert, und eine Meldung nennt sie.
  - AK-11.4 Die Zeile einer Pflicht-Mod zeigt das Schloss-Icon auf `#1A140D` (Design §9.11).
  - AK-11.5 Die Prüfung dauert bei bereits passendem Zustand für ein Pack mit 200 Dateien weniger als 3 s. Die Annahme dahinter: Hash-Cache über Größe und Änderungszeit, siehe `02` §6.
- **Randfälle:**
  - *Offline:* Reparieren aus dem lokalen Pack-Cache. Fehlt eine Datei auch dort, startet das Spiel nicht: „Pflichtdatei <Name> fehlt und kann offline nicht geladen werden“.
  - *Abbruch:* Ein abgebrochener Reparatur-Download lässt den Start nicht zu.
  - *Konflikt:* Ein Sync-Pack (F-31) will eine Mod einspielen, die das Event-Pack sperrt. Die Event-Fassung gewinnt, und das Sync-Pack meldet „schon vorhanden“.

### F-12 Event-Pack aktuell halten

- **Nutzerziel:** Ich spiele immer die aktuelle Event-Version, ohne nachzudenken.
- **Verhalten:**
  - Beim Öffnen der Events-Seite und vor jedem Start einer Event-Instanz fragt Pinecone das neueste Manifest des Packs ab.
  - Eine neue Version zeigt ein Update-Banner mit Changelog.
  - Ist die Version im Manifest als `required` markiert, aktualisiert SPIELEN zuerst automatisch.
  - Das Update nutzt den Drei-Wege-Abgleich mit Snapshot und Rücksprung.
- **Priorität:** MVP
- **Ist-Stand:** teilweise. Update-Prüfung, Drei-Wege-Update, Snapshots und Rollback sind vorhanden (`modpack.rs`). Es fehlen das Pflicht-Update vor dem Start und das Feld `required`.
- **Akzeptanzkriterien:**
  - AK-12.1 Ist eine neuere Pack-Version veröffentlicht, zeigt die Instanz beim nächsten Öffnen „Update verfügbar“.
  - AK-12.2 Bei `required: true` startet SPIELEN das Spiel erst nach erfolgreichem Update.
  - AK-12.3 Eigene Änderungen an Configs, die das Pack nicht geändert hat, bleiben beim Update erhalten.
  - AK-12.4 Nach einem Update ist ein Rücksprung auf die vorige Version möglich, solange das Event ihn nicht als `required` sperrt.
- **Randfälle:**
  - *Offline:* Start mit der installierten Version. Ist ein Pflicht-Update bekannt, aber nicht geladen: „Für dieses Event brauchst du Version <v>. Verbinde dich mit dem Internet.“
  - *Abbruch:* Ein abgebrochenes Update stellt den Snapshot wieder her.
  - *Konflikt:* Hat der Nutzer eine Datei geändert, die das Update ebenfalls ändert, gewinnt die Pack-Fassung. Die Nutzerfassung landet im Snapshot, und eine Meldung nennt die Datei.

### F-13 Event-Server-Beitritt per Klick

- **Nutzerziel:** Ich komme mit einem Klick auf den Eventserver.
- **Verhalten:**
  - „Server beitreten“ im Event-Detail startet die zum Event gehörende Event-Instanz, nicht die in der Spielleiste gewählte Instanz.
  - Pinecone springt per Quick Play direkt auf den Server (`--quickPlayMultiplayer` ab 1.20, vorher `--server`/`--port`, `02` §5).
  - Die Adresse kommt von twogg.app, und zwar nur für freigegebene Teilnehmende (`GET /api/v1/events/{id}/join`).
- **Priorität:** MVP
- **Ist-Stand:** teilweise. Quick Play und die Abfrage der Adresse sind vorhanden. Es wird aber die in der Spielleiste gewählte Instanz gestartet (`EventsPage.svelte`), nicht die Event-Instanz.
- **Akzeptanzkriterien:**
  - AK-13.1 „Server beitreten“ startet die Instanz, deren `eventId` zum Event passt, und wählt sie in der Spielleiste aus.
  - AK-13.2 Fehlt die Event-Instanz, bietet der Knopf zuerst F-10 an.
  - AK-13.3 Ohne Freigabe zeigt der Knopf den Grund aus der API (z. B. „Noch nicht freigegeben“) und ist deaktiviert.
  - AK-13.4 Die Server-Adresse ist nicht im Launcher-Code fest eingetragen.
  - AK-13.5 Mit Offline-Konto ist der Knopf deaktiviert (AK-03.2).
- **Randfälle:**
  - *Offline:* Der Knopf ist deaktiviert. Eine früher erhaltene Adresse wird nicht gespeichert, weil twogg.app über den Zugang entscheidet.
  - *Abbruch:* entfällt, Abbruch läuft wie bei F-05.
  - *Konflikt:* Ist der Server voll oder nicht erreichbar, meldet Minecraft das selbst. Pinecone zeigt vorher per Server-Ping „Server antwortet nicht“, ohne den Start zu blockieren.

### F-14 Countdown und Desktop-Hinweis vor Eventstart

- **Nutzerziel:** Ich verpasse den Eventstart nicht.
- **Verhalten:**
  - Events-Seite und Startseite zeigen für angemeldete Events einen Countdown (Tage, Stunden, Minuten).
  - Läuft Pinecone, sendet es eine Desktop-Benachrichtigung zu zwei Vorlaufzeiten (Standard: 60 min und 10 min, einstellbar). Ein Klick darauf öffnet das Event-Detail.
  - Pflicht-Updates (F-12) werden beim ersten Hinweis schon im Hintergrund geladen.
- **Priorität:** MVP
- **Ist-Stand:** teilweise. Der Countdown auf der Events-Seite ist vorhanden. Es fehlen Desktop-Benachrichtigungen und Vorab-Laden, und es gibt kein Notification-Plugin.
- **Akzeptanzkriterien:**
  - AK-14.1 Für ein Event mit Status „Angemeldet“ und Termin in 60 min erscheint genau eine OS-Benachrichtigung.
  - AK-14.2 Für nicht angemeldete Events kommt keine Benachrichtigung, außer der Nutzer hat „Alle Events“ gewählt.
  - AK-14.3 Hinweise lassen sich unter Einstellungen › Allgemein abschalten.
  - AK-14.4 Ein Klick auf die Benachrichtigung bringt Pinecone in den Vordergrund mit dem Event-Detail.
  - AK-14.5 Wird der Termin auf twogg.app verschoben, erfolgt die Benachrichtigung zur neuen Zeit.
- **Randfälle:**
  - *Offline:* Der Countdown läuft mit dem letzten bekannten Termin weiter und zeigt „Stand von <Zeit>“.
  - *Abbruch:* Ist Pinecone zum Hinweiszeitpunkt geschlossen, gibt es keinen Hinweis. Ein Hintergrunddienst oder Autostart ist später (offene Frage Q-09 in `06`).
  - *Konflikt:* Bei mehreren Events zur selben Zeit kommt je Event ein Hinweis. Für die Systemzeit gilt UTC aus der API, die Anzeige erfolgt in lokaler Zeitzone.

### F-15 Startseite

- **Nutzerziel:** Beim Öffnen sehe ich sofort, was als Nächstes ansteht, und komme in mein letztes Spiel.
- **Verhalten:** Die Startseite enthält:
  - **Bühne** mit Event-Teaser des nächsten Events: Badge, Pixel-Titel, Text, Status-Chip, Link „Zur Anmeldung auf twogg.app“.
  - **Tafel „Event-Kalender“.**
  - **Neuigkeiten** vom Pinecone-Server.
  - **Leiste „Zuletzt gespielt“** mit Kacheln, die letzte ist „TwoGG Server“ (F-16).
- **Priorität:** MVP
- **Ist-Stand:** vorhanden (`PlayPage.svelte`). Abweichung: Der Knopf „Gilde wählen“ aus der Design-Referenz ist entfernt. Er widerspricht der festen Entscheidung „keine Anmeldelogik im Launcher“; die Gilde wird nur angezeigt (F-09).
- **Akzeptanzkriterien:**
  - AK-15.1 Die Bühne zeigt das nächste kommende Event. Gibt es keines, das zuletzt beendete mit „Beendet“.
  - AK-15.2 „Zuletzt gespielt“ zeigt die Instanzen absteigend nach letztem Start, höchstens 5 Kacheln.
  - AK-15.3 Neuigkeiten-HTML wird bereinigt angezeigt: keine Skripte, keine externen Iframes.
  - AK-15.4 Ohne Instanzen erscheint der Leerzustand „Leg deine erste Instanz an“ mit Knopf.
- **Randfälle:**
  - *Offline:* Bühne mit „Willkommen, twogg.app ist gerade nicht erreichbar. Deine Instanzen kannst du trotzdem starten.“ Zuletzt gespielt funktioniert lokal.
  - *Abbruch:* entfällt.
  - *Konflikt:* Eine zuletzt gespielte Instanz wurde gelöscht und verschwindet aus der Leiste.

### F-16 Community-Server-Schnellbeitritt

- **Nutzerziel:** Ich komme mit einem Klick auf den dauerhaften TwoGG-Server.
- **Verhalten:**
  - Die Kachel „TwoGG Server“ (gestrichelter Rahmen) in „Zuletzt gespielt“ zeigt Name, Ping und Spielerzahl.
  - Ein Klick startet die zuletzt gewählte passende Instanz mit Quick Play.
  - Die Adresse kommt vom Pinecone-Server (`GET /api/v1/info`, `02` §8), gepflegt im Admin. Bis dahin steht sie als `[SERVER-ADRESSE]`.
- **Priorität:** MVP
- **Ist-Stand:** vorhanden (`PlayPage.svelte`, `servers.rs`)
- **Akzeptanzkriterien:**
  - AK-16.1 Ist im Admin keine Adresse gepflegt, erscheint die Kachel nicht.
  - AK-16.2 Die Kachel zeigt den Ping in ms oder „offline“.
  - AK-16.3 Der Klick startet das Spiel und verbindet direkt (Quick Play).
- **Randfälle:**
  - *Offline:* Die Kachel ist ausgeblendet.
  - *Abbruch:* wie bei F-05.
  - *Konflikt:* Passt die Minecraft-Version des Servers zu keiner Instanz, kommt der Hinweis „Für diesen Server brauchst du Minecraft <v>“ mit Knopf zu F-19.

### F-17 Freundesanzeige

- **Nutzerziel:** Ich sehe, welche Freunde online sind und wo sie spielen, und trete ihnen bei.
- **Verhalten:**
  - In der Spielleiste rechts liegt ein Avatar-Stapel mit „<n> online“. Ein Klick öffnet das Freunde-Popup nach oben: Liste, Anfrage senden oder annehmen, Entfernen, „Beitreten“ bei Server-Aktivität.
  - Die Identität ist das Minecraft-Konto (Mojang-Session-Beweis). Die Daten liegen auf dem Pinecone-Server.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`shell/Friends.svelte`, `src-tauri/src/friends.rs`, Server `friends.rs`). Es gibt kein rechtes Freunde-Panel, wie vom Design verlangt.
- **Akzeptanzkriterien:**
  - AK-17.1 Ein Freund, der in den letzten 90 s ein Lebenszeichen gesendet hat, gilt als online.
  - AK-17.2 Anfragen lassen sich annehmen, ablehnen und zurückziehen.
  - AK-17.3 „Beitreten“ erscheint nur, wenn der Freund auf einem Server mit öffentlicher Adresse spielt.
  - AK-17.4 Unter Einstellungen › Konten lässt sich die eigene Sichtbarkeit abschalten („Für Freunde unsichtbar“).
- **Randfälle:**
  - *Offline:* Der Stapel zeigt „Freunde offline nicht verfügbar“.
  - *Abbruch:* entfällt.
  - *Konflikt:* Senden zwei Spieler einander gleichzeitig eine Anfrage, werden sie Freunde. Für Offline-Konten gibt es keine Freundesliste.

Ob die Freundesliste mit den twogg.app-Freunden zusammengeführt wird, ist offen: Q-05 in `06`.

### F-18 Verweise auf twogg.app und Discord

- **Nutzerziel:** Für Anmeldung, Teams, Bewerbungen und die Community finde ich schnell den richtigen Ort.
- **Verhalten:** Links öffnen den Standardbrowser:
  - „Zur Anmeldung auf twogg.app“ (Event-Seite auf twogg.app),
  - „Team finden auf twogg.app“ (Looking-for-Group),
  - Banner „Werde Teil des Teams – Bewerbung auf twogg.app“, nur wenn twogg.app offene Bewerbungsphasen meldet,
  - „Discord: twogg.chat“.
- **Priorität:** MVP
- **Ist-Stand:** teilweise. Die Anmelde- und Discord-Links sind vorhanden. Das Bewerbungsbanner und der LFG-Link fehlen.
- **Akzeptanzkriterien:**
  - AK-18.1 Jeder Link öffnet den Standardbrowser, nicht einen WebView.
  - AK-18.2 Das Bewerbungsbanner erscheint nur, wenn `GET /api/v1/community` `applicationsOpen: true` liefert.
  - AK-18.3 Kein Formular aus twogg.app ist im Launcher nachgebaut.
- **Randfälle:**
  - *Offline:* Links bleiben sichtbar. Das Banner erscheint nicht.
  - *Abbruch:* entfällt.
  - *Konflikt:* entfällt.

---

## C. Instanzen und Inhalte

### F-19 Instanzen anlegen und bearbeiten

- **Nutzerziel:** Ich lege eigene Instanzen mit Vanilla, Fabric, Quilt, NeoForge oder Forge an.
- **Verhalten:**
  - Der Dialog „Neue Instanz“ fragt Name, Minecraft-Version (Release, optional Snapshots), Loader und Loader-Version, Symbol bzw. Motiv und Arbeitsspeicher ab.
  - Bearbeiten erfolgt im Instanz-Tab „Einstellungen“. Löschen geht nur mit Bestätigung.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`profiles.rs`, `loader.rs`, `forge.rs`; `ProfileDialog.svelte`, `InstanceForm.svelte`)
- **Akzeptanzkriterien:**
  - AK-19.1 Für jede Kombination aus Version und Loader, die die Loader-Metadaten anbieten, startet eine leere Instanz bis ins Hauptmenü.
  - AK-19.2 Loader-Versionen werden aus den offiziellen Meta-Quellen geladen (`02` §5), nicht fest eingetragen.
  - AK-19.3 Löschen entfernt Instanzordner und Statistik. Event-Instanzen fragen zusätzlich „Event-Instanz löschen? Du kannst sie jederzeit neu installieren.“
  - AK-19.4 Zwei Instanzen mit gleichem Namen erhalten verschiedene Ordner-IDs.
- **Randfälle:**
  - *Offline:* Eine Instanz lässt sich nur mit bereits geladenen Versionen anlegen. Die Liste zeigt dann nur installierte Versionen.
  - *Abbruch:* Der abgebrochene Dialog legt nichts an.
  - *Konflikt:* Wird die Instanz bearbeitet, während sie läuft, sind die Felder gesperrt mit „Läuft gerade“.

### F-20 Version oder Loader einer Instanz wechseln

- **Nutzerziel:** Ich hebe meine Instanz auf eine neue Minecraft-Version, ohne Welten zu verlieren.
- **Verhalten:**
  - „Version wechseln“ zeigt einen Plan: welche Mods es für das Ziel gibt, welche fehlen, und ob eine Warnung bei einer älteren Zielversion nötig ist.
  - Bestätigen legt eine neue Instanz an, die alte bleibt.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`migrate.rs`; `MigrateDialog.svelte`)
- **Akzeptanzkriterien:**
  - AK-20.1 Der Plan nennt jede Mod mit „passt“, „Update“ oder „nicht verfügbar“.
  - AK-20.2 Die alte Instanz ist nach dem Wechsel unverändert.
  - AK-20.3 Für Event-Instanzen ist „Version wechseln“ deaktiviert, mit dem Hinweis, dass das Event-Pack die Version vorgibt.
- **Randfälle:**
  - *Offline:* deaktiviert.
  - *Abbruch:* Die halb angelegte Zielinstanz wird entfernt.
  - *Konflikt:* Gibt es das Modpack der Instanz selbst für das Ziel, wird stattdessen diese Pack-Version angeboten.

### F-21 Entdecken und Modrinth-Inhalte

- **Nutzerziel:** Ich finde Modpacks, Mods, Ressourcenpakete, Shader und Datenpakete und installiere sie in eine Instanz.
- **Verhalten:**
  - Der Tab „Entdecken“ hat Segment-Tabs für die Inhaltstypen, „Installieren in [Instanz]“, Suche, Filter (Loader, Version, Kategorie, Sortierung), den Streifen „TwoGG-Auswahl“ und ein Ergebnisraster.
  - „Holen“ installiert die passende Version samt Pflicht-Abhängigkeiten.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`modrinth.rs`, `content.rs`; `DiscoverPage.svelte`)
- **Akzeptanzkriterien:**
  - AK-21.1 Ist eine Instanz gewählt, zeigen die Ergebnisse nur Versionen, die zu ihrer Minecraft-Version und ihrem Loader passen.
  - AK-21.2 Pflicht-Abhängigkeiten werden mitinstalliert und in der Inhaltstabelle als solche erkennbar.
  - AK-21.3 Bereits installierte Inhalte zeigen „Installiert“ mit Haken.
  - AK-21.4 Jede Anfrage an Modrinth trägt einen User-Agent nach den Modrinth-Regeln (`02` §5, `06` H-03).
  - AK-21.5 Eine Installation in eine Event-Instanz ist nur möglich, wenn das Manifest Zusatzinhalte erlaubt (F-11).
- **Randfälle:**
  - *Offline:* Die Seite zeigt „Entdecken braucht Internet“.
  - *Abbruch:* Eine abgebrochene Installation entfernt die halb geladenen Dateien.
  - *Konflikt:* Ist dasselbe Projekt schon in anderer Version da, wird „Aktualisieren“ statt „Holen“ angeboten. Inkompatible Abhängigkeiten werden vor der Installation genannt.
  - *Rate-Limit:* Antwortet Modrinth mit HTTP 429, wartet Pinecone die gemeldete Zeit ab und zeigt „Modrinth bremst gerade, kurz warten“.

### F-22 CurseForge-Inhalte

- **Nutzerziel:** Ich nutze auch Inhalte, die es nur auf CurseForge gibt.
- **Verhalten:**
  - In Entdecken lässt sich „CurseForge“ als Quelle wählen. Die Installation läuft wie bei F-21.
  - Erlaubt ein Projekt keine Drittanbieter-Downloads, zeigt Pinecone „Diese Datei musst du selbst von CurseForge herunterladen“ mit Link und nimmt die Datei per Drag & Drop an.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`curseforge.rs`; `MissingDialog.svelte`). Die Funktion ist nur aktiv, wenn ein API-Schlüssel eingebaut ist.
- **Akzeptanzkriterien:**
  - AK-22.1 Ohne eingebauten API-Schlüssel ist die Quelle „CurseForge“ ausgeblendet, nicht fehlerhaft.
  - AK-22.2 Pinecone lädt keine Datei, deren Projekt den Drittanbieter-Download verbietet, von einer anderen Quelle nach (`06` H-02).
  - AK-22.3 Eine per Hand geladene Datei wird per Hash dem erwarteten Eintrag zugeordnet.
- **Randfälle:**
  - *Offline:* wie F-21.
  - *Abbruch:* wie F-21.
  - *Konflikt:* Gibt es ein Projekt auf Modrinth und CurseForge, zeigt die Inhaltstabelle die tatsächliche Quelle der installierten Datei.

### F-23 Inhaltsverwaltung einer Instanz

- **Nutzerziel:** Ich sehe und steuere alle Mods, Ressourcenpakete und Shader einer Instanz.
- **Verhalten:**
  - Der Instanz-Tab „Inhalt“ zeigt die Tabelle Name · Status · Quelle · Aktiv mit Filter, Suche, Update-Prüfung, „Alle aktualisieren“, Drag & Drop und Kontextmenü.
  - Unbekannte Dateien werden per Hash bei Modrinth und CurseForge erkannt.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`content.rs`; `ContentTable.svelte`, `AddContentDialog.svelte`)
- **Akzeptanzkriterien:**
  - AK-23.1 Abschalten benennt die Datei in `<name>.disabled` um, Einschalten macht das rückgängig.
  - AK-23.2 „Update verfügbar“ erscheint nur bei einer neueren Version, die zu Version und Loader passt.
  - AK-23.3 Die Quelle „TwoGG“ steht in Harz, andere Quellen in `--text-muted` (Design §9.11).
  - AK-23.4 Pflicht-Mods einer Event-Instanz verhalten sich nach F-11.
- **Randfälle:**
  - *Offline:* Tabelle und Schalter funktionieren. Update-Prüfung und Erkennung sind deaktiviert.
  - *Abbruch:* Ein abgebrochenes Update lässt die alte Datei bestehen.
  - *Konflikt:* Läuft das Spiel, sind Schalter und Entfernen gesperrt.

### F-24 `.mrpack`-Import und Modpack-Updates

- **Nutzerziel:** Ich installiere ein Modrinth-Modpack aus einer Datei oder einem Link und halte es aktuell.
- **Verhalten:**
  - Import per Dateidialog, Drag & Drop, Doppelklick auf `.mrpack` (F-39) oder Link.
  - Eine Vorschau zeigt Name, Version, Minecraft, Loader und die Anzahl der Inhalte.
  - Updates laufen mit Drei-Wege-Abgleich, Snapshot (3 behalten) und Rücksprung.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`modpack.rs`; `ImportDialog.svelte`, `PackUpdateDialog.svelte`)
- **Akzeptanzkriterien:**
  - AK-24.1 Jede Datei im Pack wird gegen den Hash aus `modrinth.index.json` geprüft.
  - AK-24.2 Download-URLs außerhalb der erlaubten Hosts werden abgelehnt (Liste und Quelle in `06` H-03).
  - AK-24.3 Pfade, die aus dem Instanzordner führen (`../`), werden abgelehnt.
  - AK-24.4 Nach einem Update lässt sich jeder der letzten 3 Snapshots wiederherstellen.
- **Randfälle:**
  - *Offline:* Ein Import ist nur möglich, wenn alle Dateien im Pack eingebettet sind (overrides). Sonst kommt eine klare Meldung.
  - *Abbruch:* Der halb angelegte Ordner wird entfernt.
  - *Konflikt:* Hat die Instanz schon ein Pack, wird nach „Als neue Instanz“ oder „Ersetzen“ gefragt.

### F-25 `.mrpack`-Export und Teilen

- **Nutzerziel:** Ich gebe meine Instanz an Freunde weiter.
- **Verhalten:**
  - Export als `.mrpack`: Plattform-Inhalte als Download-Eintrag, eigene Dateien als Override.
  - Welten, `servers.dat` und Konten sind nie enthalten.
  - Teilen-Link `pinecone://modrinth/<projekt>` oder `pinecone://twogg/<pack>`, wenn die Instanz aus einer Quelle stammt.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`modpack.rs` Export, `share.rs`; `ShareDialog.svelte`). Kurzlinks über einen Teilen-Dienst (`pinecone://pack/<code>`) fehlen und sind „später“, siehe F-39.
- **Akzeptanzkriterien:**
  - AK-25.1 Der Export enthält nie `saves/`, `servers.dat`, `accounts.json` oder Tokens.
  - AK-25.2 Ein exportiertes Pack lässt sich mit F-24 wieder importieren und startet.
  - AK-25.3 Event-Instanzen lassen sich nicht als `.mrpack` exportieren, nur als Teilen-Link auf das Event-Pack.
- **Randfälle:**
  - *Offline:* Der Export funktioniert. Plattform-Einträge brauchen bekannte Hashes aus dem Lockfile.
  - *Abbruch:* Die Zieldatei wird nicht angelegt.
  - *Konflikt:* Existiert die Zieldatei schon, kommt die Rückfrage des Speichern-Dialogs des Betriebssystems.

### F-26 CurseForge-Modpack-Import

- **Nutzerziel:** Ich importiere ein CurseForge-Modpack (ZIP).
- **Verhalten:** Das Manifest wird gelesen und die Dateien über die CurseForge-API geladen. Blockierte Dateien laufen wie bei F-22.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`modpack.rs` `read_curseforge`)
- **Akzeptanzkriterien:**
  - AK-26.1 Ein CurseForge-ZIP wird automatisch vom `.mrpack` unterschieden.
  - AK-26.2 Fehlende, nicht verteilbare Dateien stehen nach dem Import im Dialog „Fehlende Dateien“.
  - AK-26.3 Ohne API-Schlüssel kommt die Meldung „CurseForge-Import ist in diesem Build nicht verfügbar“.
- **Randfälle:**
  - *Offline:* deaktiviert.
  - *Abbruch:* wie F-24.
  - *Konflikt:* wie F-24.

### F-27 Serverliste und Server-Schnellbeitritt einer Instanz

- **Nutzerziel:** Ich sehe die Server einer Instanz mit Status und trete direkt bei.
- **Verhalten:**
  - Der Instanz-Tab „Server“ liest `servers.dat` und pingt jeden Server (Server List Ping, SRV-Auflösung).
  - „Beitreten“ startet die Instanz mit Quick Play.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`servers.rs`; `ServersTab.svelte`)
- **Akzeptanzkriterien:**
  - AK-27.1 Jeder Server zeigt MOTD, Spielerzahl und Ping oder „offline“.
  - AK-27.2 „Beitreten“ verbindet nach dem Start ohne Umweg über das Mehrspielermenü, ab Minecraft 1.20 per `--quickPlayMultiplayer` (`02` §5).
- **Randfälle:**
  - *Offline:* Alle Server sind „offline“, Beitreten ist trotzdem möglich (LAN).
  - *Abbruch:* entfällt.
  - *Konflikt:* Eine beschädigte `servers.dat` erzeugt „Serverliste nicht lesbar“, ohne die Datei zu verändern.

### F-28 Welten

- **Nutzerziel:** Ich sehe meine Welten, sichere sie und räume auf.
- **Verhalten:** Der Instanz-Tab „Welten“ zeigt Name, Spielmodus und letzte Nutzung, dazu Backup als ZIP, Ordner öffnen und Löschen mit Bestätigung. Datenpakete lassen sich in eine Welt installieren.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`worlds.rs`; `WorldsTab.svelte`)
- **Akzeptanzkriterien:**
  - AK-28.1 Das Backup erzeugt eine ZIP, die Minecraft nach dem Entpacken lädt.
  - AK-28.2 Löschen verlangt eine Bestätigung mit dem Weltnamen.
  - AK-28.3 Läuft das Spiel, sind Backup und Löschen gesperrt.
- **Randfälle:**
  - *Offline:* funktioniert.
  - *Abbruch:* Eine abgebrochene Sicherung löscht die halbe ZIP.
  - *Konflikt:* Eine Welt mit unlesbarer `level.dat` wird mit Ordnernamen und Hinweis gezeigt.

### F-29 Screenshots

- **Nutzerziel:** Ich sehe die Screenshots einer Instanz und teile sie.
- **Verhalten:** Der Instanz-Tab „Screenshots“ zeigt ein Raster mit Vorschaubildern und eine Großansicht mit „Speichern unter“, „Im Ordner zeigen“ und „Löschen“. Neue Screenshots erscheinen live.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`screenshots.rs`; `ScreenshotsTab.svelte`)
- **Akzeptanzkriterien:**
  - AK-29.1 Ein im Spiel aufgenommener Screenshot erscheint ohne Neuladen innerhalb von 5 s.
  - AK-29.2 Vorschaubilder werden im Cache abgelegt, nicht neben dem Original.
- **Randfälle:**
  - *Offline:* funktioniert.
  - *Abbruch:* entfällt.
  - *Konflikt:* Eine extern gelöschte Datei verschwindet aus dem Raster.

### F-30 Skins

- **Nutzerziel:** Ich verwalte Skins, sehe sie in 3D und setze Skin und Umhang meines Kontos.
- **Verhalten:**
  - Der Tab „Skins“ hat eine Bibliothek: Import aus PNG oder Spielername, Editor und eine 3D-Vorschau mit Animationen.
  - „Anwenden“ lädt den Skin zum Konto hoch, die Umhangwahl gilt für das Konto.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`skins.rs`, `msa.rs`; `SkinsPage.svelte`, `SkinPreview.svelte`, `SkinEditor.svelte`)
- **Akzeptanzkriterien:**
  - AK-30.1 Nur PNGs mit 64 × 64 oder 64 × 32 werden angenommen.
  - AK-30.2 Nach „Anwenden“ zeigt die Spielleiste den neuen Kopf.
  - AK-30.3 Mit „Animationen reduzieren“ bewegt sich die Vorschau nicht von selbst.
  - AK-30.4 Für Offline-Konten ist „Anwenden“ deaktiviert.
- **Randfälle:**
  - *Offline:* Die Bibliothek funktioniert, „Anwenden“ ist deaktiviert.
  - *Abbruch:* Ein abgebrochener Upload ändert nichts am Konto.
  - *Konflikt:* Liefert Mojang eine Ratenbegrenzung, kommt „Bitte versuch es in einer Minute erneut“.

### F-31 Sync-Packs und Vorlagen

- **Nutzerziel:** Meine Lieblings-Mods, Configs, Tastenbelegungen und Optionen sind in jeder Instanz da.
- **Verhalten:**
  - **Sync-Packs:**
    - Sie sammeln Projekte (ohne feste Version), eigene Dateien und `options.txt`-Werte inklusive Steuerung.
    - Eingeschaltet je Instanz, abgeglichen vor jedem Start. Ausschalten entfernt genau ihre Inhalte.
    - Optional schalten sie sich automatisch für neue Instanzen ein.
  - **Vorlagen:** werden einmal eingespielt.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`syncpacks.rs`; `SyncPackView.svelte`, `OptionsFromProfileDialog.svelte`)
- **Akzeptanzkriterien:**
  - AK-31.1 Optionen und Dateien werden beim ersten Einspielen gesetzt. Danach geänderte Werte bleiben erhalten.
  - AK-31.2 Ausschalten entfernt nur Inhalte mit Markierung `sync: <id>`.
  - AK-31.3 In Event-Instanzen tastet ein Sync-Pack keine Pflicht-Mod an (F-11).
- **Randfälle:**
  - *Offline:* Der Abgleich wird übersprungen, Start mit dem bisherigen Stand.
  - *Abbruch:* Ein fehlgeschlagenes Sync-Pack wird zur Warnung, die anderen laufen weiter.
  - *Konflikt:* Liefert ein Modpack dasselbe Projekt, gewinnt die Pack-Fassung.

### F-32 Arbeitsspeicher und JVM-Presets

- **Nutzerziel:** Ich gebe Minecraft genug Speicher und gute Java-Einstellungen, ohne Flags auswendig zu kennen.
- **Verhalten:**
  - Speicher wird je Instanz per Schieberegler eingestellt.
  - Fertige Flag-Sätze (Aikar, MeowIce ab Java 17, Generational ZGC ab Java 21) dienen als globaler Standard und lassen sich je Instanz überschreiben. Eigene Presets sind möglich.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`jvm.rs`; `JvmPresets.svelte`, `InstanceForm.svelte`)
- **Akzeptanzkriterien:**
  - AK-32.1 Ein Preset mit höherer Java-Mindestversion als die Instanz-Java entfällt beim Start, mit einem Eintrag im Log.
  - AK-32.2 `-Xmx` kann nicht in einem Preset stehen.
  - AK-32.3 Eigene JVM-Argumente der Instanz stehen nach dem Preset und haben damit Vorrang.
  - AK-32.4 Event-Packs können per Manifest einen Mindestspeicher empfehlen. Die Instanz zeigt dann „Empfohlen: <n> GB“.
- **Randfälle:**
  - *Offline:* entfällt.
  - *Abbruch:* entfällt.
  - *Konflikt:* Ist mehr Speicher eingestellt als physisch vorhanden, warnt Pinecone vor dem Start.

---

## D. Launcher, Darstellung, Betrieb

### F-33 Themes und Akzentfarben

- **Nutzerziel:** Pinecone sieht so aus, wie ich es mag.
- **Verhalten:**
  - Unter Einstellungen › Darstellung gibt es:
    - Theme-Karten Rinde, Kiefer, Winterland mit Mini-Vorschau,
    - 5 Akzent-Swatches (Harz, Nadel, Gletscher, Beere, Flechte),
    - Live-Vorschau,
    - Schalter „Zapfenschuppen-Muster“, „Pixel-Überschriften“, „Animationen reduzieren“.
  - Die Wahl gilt sofort und bleibt gespeichert.
- **Priorität:** MVP
- **Ist-Stand:** vorhanden (`settings.rs`; `AppearanceSection.svelte`, `ui/src/app.css`). Darüber hinaus gibt es eine freie Akzentfarbe sowie Theme-Export und -Import.
- **Akzeptanzkriterien:**
  - AK-33.1 Ein Theme-Wechsel ändert alle Flächen ohne Neustart.
  - AK-33.2 Text auf jedem Akzent ist `#1B1206` (Design §3).
  - AK-33.3 Jede Akzentfarbe hält 4,5:1 Kontrast zwischen Akzent und `#1B1206` (Design §13). Bei freien Farben warnt Pinecone, wenn der Kontrast nicht reicht.
  - AK-33.4 Grün erscheint nur im Theme „Kiefer“ oder als Akzent „Nadel“ als Fläche (Design §12).
- **Randfälle:**
  - *Offline:* entfällt.
  - *Abbruch:* entfällt.
  - *Konflikt:* Ist `settings.json` beschädigt, gilt der Standard Rinde/Harz. Die Datei wird nicht überschrieben, bevor der Nutzer etwas ändert.

### F-34 Launcher-Auto-Update

- **Nutzerziel:** Pinecone bleibt aktuell, ohne dass ich etwas herunterladen muss.
- **Verhalten:**
  - Beim Start und alle 6 h prüft Pinecone den gewählten Kanal (Stable oder Beta).
  - Ein Update erscheint als Benachrichtigung. „Installieren“ lädt das Paket, prüft die Updater-Signatur und startet neu.
- **Priorität:** MVP
- **Ist-Stand:** teilweise.
  - Vorhanden: Updater mit Signatur und Kanälen (`update.rs`).
  - Es fehlen:
    - eine periodische Prüfung,
    - Code-Signing der Installer (Windows Authenticode, macOS),
    - eine Sperre während laufender Spiele.
- **Akzeptanzkriterien:**
  - AK-34.1 Ein Paket mit ungültiger Updater-Signatur wird nicht installiert.
  - AK-34.2 Im Kanal Stable erscheinen keine Beta-Versionen.
  - AK-34.3 Läuft ein Spiel, wird die Installation auf nach Spielende verschoben.
  - AK-34.4 Die Windows-Installer sind mit Authenticode signiert. SmartScreen nennt einen Herausgeber statt „Unbekannt“ [ungeprüft, siehe `06` H-05].
  - AK-34.5 Ohne konfigurierten Update-Schlüssel ist die Update-Prüfung sichtbar abgeschaltet.
- **Randfälle:**
  - *Offline:* Keine Prüfung, kein Fehler-Hinweis.
  - *Abbruch:* Ein abgebrochener Download lässt die alte Version unverändert.
  - *Konflikt:* Ein Wechsel von Beta auf Stable bietet kein Downgrade an. Pinecone wartet, bis Stable die installierte Version überholt.

### F-35 Absturz- und Log-Ansicht

- **Nutzerziel:** Wenn das Spiel abstürzt, verstehe ich warum und kann Hilfe holen.
- **Verhalten:**
  - Der Instanz-Tab „Logs“ zeigt die Live-Konsole mit Filter und Suche.
  - Nach einem Absturz (Exit-Code ungleich 0 oder neuer `crash-reports/`-Eintrag) erscheint:
    - eine Benachrichtigung,
    - eine Absturzansicht mit erkannter Ursache, sofern bekannt (fehlende Abhängigkeit, falsche Java-Version, zu wenig Speicher, Mod-Konflikt),
    - die Knöpfe „Diagnose exportieren“ (F-36) und „Im Discord melden“.
- **Priorität:** MVP
- **Ist-Stand:** teilweise. Die Live-Konsole und die Benachrichtigung bei Spielfehler sind vorhanden (`game.rs`, `LogsTab.svelte`). Absturzansicht und Ursachenerkennung fehlen.
- **Akzeptanzkriterien:**
  - AK-35.1 Nach einem Absturz öffnet die Benachrichtigung die Absturzansicht der betroffenen Instanz.
  - AK-35.2 Die Absturzansicht zeigt den neuesten Crash-Report und die letzten 200 Log-Zeilen.
  - AK-35.3 Für mindestens diese 4 Ursachen gibt es einen erklärenden Satz: fehlende Abhängigkeit, falsche Java-Hauptversion, `OutOfMemoryError`, doppelte Mod-ID.
  - AK-35.4 Logs enthalten keine Access-Tokens (Ersetzung durch `<redacted>`).
- **Randfälle:**
  - *Offline:* funktioniert.
  - *Abbruch:* entfällt.
  - *Konflikt:* Bei mehreren Crash-Reports aus derselben Sitzung wird der neueste gezeigt, die anderen stehen in einer Liste.

### F-36 Diagnose-Export

- **Nutzerziel:** Ich schicke dem Team alles, was es zur Fehlersuche braucht, mit einem Klick.
- **Verhalten:** Erzeugt wird eine ZIP mit Systeminfo, Launcher-Logs (7 Tage), Spiel-Logs, den 3 neuesten Crash-Reports, der Inhaltsliste und der Instanzkonfiguration, ohne Kontodaten.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`diagnostics.rs`)
- **Akzeptanzkriterien:**
  - AK-36.1 Die ZIP enthält weder `accounts.json` noch Tokens noch die E-Mail-Adresse (Prüfung per Volltextsuche).
  - AK-36.2 Der Speicherort ist frei wählbar.
- **Randfälle:**
  - *Offline:* funktioniert.
  - *Abbruch:* Die Zieldatei wird nicht angelegt.
  - *Konflikt:* Ist eine Logdatei gesperrt, wird sie übersprungen und in `README.txt` der ZIP vermerkt.

### F-37 Benachrichtigungen

- **Nutzerziel:** Ich erfahre Wichtiges zur richtigen Zeit, ohne zugespammt zu werden.
- **Verhalten:**
  - Die Glocke in der Kopfleiste zeigt In-App-Hinweise:
    - App-Update,
    - Spielabsturz,
    - Event-Pack-Update,
    - Event-Hinweis,
    - neue Freundschaftsanfrage,
    - Statusänderung bei twogg.app („Du bist jetzt angemeldet“).
  - Ausgewählte Arten gehen zusätzlich als OS-Benachrichtigung raus.
  - Jede Art lässt sich einzeln in Einstellungen › Allgemein abschalten.
- **Priorität:** MVP
- **Ist-Stand:** teilweise. Das Glocken-Panel ist vorhanden und wird bisher nur für App-Update und Spielfehler genutzt. OS-Benachrichtigungen fehlen.
- **Akzeptanzkriterien:**
  - AK-37.1 Jede der genannten Arten erzeugt einen Eintrag im Panel.
  - AK-37.2 Ungelesene Einträge zeigen den 7-px-Harz-Punkt an der Glocke (Design §9.1).
  - AK-37.3 Dieselbe Meldung (gleiche ID) erscheint nicht zweimal.
  - AK-37.4 OS-Benachrichtigungen gibt es nur für Event-Hinweise (F-14), Absturz und Freundschaftsanfrage, jeweils abschaltbar.
- **Randfälle:**
  - *Offline:* Lokale Arten (Absturz) funktionieren.
  - *Abbruch:* entfällt.
  - *Konflikt:* Verweigert das Betriebssystem die Berechtigung für Benachrichtigungen, bleibt nur das Panel, mit einem Hinweis in den Einstellungen.

### F-38 Barrierefreiheit

- **Nutzerziel:** Ich kann Pinecone mit Tastatur, Screenreader und ohne Bewegung bedienen.
- **Verhalten:**
  - „Animationen reduzieren“ schaltet Intro, Übergänge und Skin-Animationen ab. `prefers-reduced-motion` wird zusätzlich beachtet.
  - Alle Bedienelemente sind per Tab erreichbar und haben eine sichtbare Fokusmarke.
  - Rollen und Labels folgen Design §13. Sprache Deutsch oder Englisch.
- **Priorität:** MVP
- **Ist-Stand:** teilweise.
  - Vorhanden: „Animationen reduzieren“, `:focus-visible`, `aria-*`, zwei Sprachen.
  - Es fehlen:
    - eine systematische Kontrastprüfung,
    - eine Prüfung der Tastaturbedienung aller Dialoge,
    - eine Textgrößen-Einstellung.
- **Akzeptanzkriterien:**
  - AK-38.1 Mit „Animationen reduzieren“ läuft auf keinem Screen eine Animation länger als 150 ms.
  - AK-38.2 Jeder Screen aus `03` lässt sich ohne Maus vollständig bedienen (Testliste in `03` §5).
  - AK-38.3 Jeder Text-Token erreicht auf seinen Hintergrund-Tokens in allen 3 Themes mindestens 4,5:1. Das wird automatisch im CI geprüft.
  - AK-38.4 Jeder Icon-Button hat ein `aria-label`. Das wird automatisch über `svelte-check` und Lint geprüft.
  - AK-38.5 Klickflächen messen mindestens 40 px (SPIELEN 62 px).
- **Randfälle:**
  - *Offline:* entfällt.
  - *Abbruch:* entfällt.
  - *Konflikt:* Steht das System auf reduzierte Bewegung, die App-Einstellung aber auf „aus“, gilt das System.

### F-39 Deep Links und Dateizuordnung

- **Nutzerziel:** Ein Klick auf „In Pinecone öffnen“ (Website, Discord) oder ein Doppelklick auf eine `.mrpack` öffnet Pinecone mit dem richtigen Dialog.
- **Verhalten:**
  - Das Schema `pinecone://` umfasst `modrinth/…`, `curseforge/…`, `twogg/<pack>[/<version>]` und `event/<eventId>` (neu, öffnet das Event-Detail).
  - Eine zweite Instanz übergibt an die laufende.
  - `.mrpack` ist dem Launcher zugeordnet.
- **Priorität:** v1.x
- **Ist-Stand:** teilweise. `modrinth`, `curseforge`, `twogg` und die Dateizuordnung sind vorhanden (`share.rs`, `open.rs`). `event/…` und macOS fehlen.
- **Akzeptanzkriterien:**
  - AK-39.1 Ein `pinecone://twogg/<pack>`-Link öffnet die Vorschau des Packs. Es wird nichts ohne Bestätigung installiert.
  - AK-39.2 Ein `pinecone://event/<id>`-Link öffnet das Event-Detail.
  - AK-39.3 Ein unbekanntes Format zeigt „Diesen Link kennt Pinecone nicht“.
- **Randfälle:**
  - *Offline:* Die Vorschau meldet „Keine Verbindung“.
  - *Abbruch:* entfällt.
  - *Konflikt:* Kommt der Link, während ein Dialog offen ist, wird er in die Warteschlange gestellt.

### F-40 Spielzeit und „Zuletzt gespielt“

- **Nutzerziel:** Ich sehe, wie lange und wann ich eine Instanz gespielt habe.
- **Verhalten:** Pinecone erfasst Start, Ende und Dauer je Instanz lokal. Die Instanz-Kopfzeile zeigt „<n> h gespielt“ und „zuletzt <Datum>“, dazu den letzten Server.
- **Priorität:** v1.x
- **Ist-Stand:** vorhanden (`stats.rs`)
- **Akzeptanzkriterien:**
  - AK-40.1 Nach einer Sitzung von 10 min ist die Gesamtzeit um 10 min (±1) gestiegen.
  - AK-40.2 Die Daten verlassen den Rechner nicht.
- **Randfälle:**
  - *Offline:* funktioniert.
  - *Abbruch:* Ein Absturz des Launchers während des Spiels verbucht die Sitzung beim nächsten Start nicht. Das ist akzeptiert.
  - *Konflikt:* entfällt.

### F-41 Discord Rich Presence

- **Nutzerziel:** Meine Discord-Freunde sehen, dass ich ein TwoGG-Event spiele.
- **Verhalten:** Wenn aktiviert, meldet Pinecone der lokalen Discord-App „Spielt REVIVAL: Avalon“ bzw. den Instanznamen ohne Serveradresse.
- **Priorität:** v2 (Block v2-B, vorgezogen aus „später“)
- **Ist-Stand:** vorhanden (PR #33: `tauri/discord`, Einstellung `discordPresence`). Braucht die Application-ID `PINECONE_DISCORD_CLIENT_ID`
- **Akzeptanzkriterien:**
  - AK-41.1 Rich Presence ist standardmäßig aus.
  - AK-41.2 Ohne laufende Discord-App entsteht kein Fehler.
  - AK-41.3 Die Serveradresse wird nie übertragen.
- **Randfälle:**
  - *Offline:* Discord-RPC ist lokal und funktioniert, wenn Discord läuft.
  - *Abbruch:* entfällt.
  - *Konflikt:* Bei mehreren laufenden Instanzen gilt die zuletzt gestartete.

### F-42 Import aus anderen Launchern

- **Nutzerziel:** Ich übernehme meine Instanzen aus Prism, MultiMC, der Modrinth App, der CurseForge App oder dem offiziellen Launcher.
- **Verhalten:** Pinecone sucht bekannte Datenordner, zeigt gefundene Instanzen und kopiert die gewählten als neue Pinecone-Instanzen. Die Mods werden per Hash erkannt (F-23).
- **Priorität:** später
- **Ist-Stand:** fehlt
- **Akzeptanzkriterien:**
  - AK-42.1 Die Originaldaten werden nur gelesen.
  - AK-42.2 Eine importierte Instanz startet mit derselben Version, demselben Loader und denselben Mods.
- **Randfälle:**
  - *Offline:* Der Import funktioniert. Die Erkennung von Quellen holt Pinecone später nach.
  - *Abbruch:* Die halbe Kopie wird entfernt.
  - *Konflikt:* Gleiche Instanznamen bekommen ein Suffix.

Die Ordnerformate der anderen Launcher sind nicht geprüft und gelten als [ungeprüft] (U-08 in `06`).

## E. Events und Community (Plan v2)

Die Funktionen dieses Abschnitts brauchen Daten von twogg.app, die es im Vertrag (`04` §4) noch nicht gibt (T-12 bis T-14 in `04` §7). Sie folgen auf MVP-B.

### F-43 Ergebnisse und Ranglisten

- **Nutzerziel:** Nach einem Event sehe ich im Launcher, wie mein Team und meine Gilde abgeschnitten haben.
- **Verhalten:** Die Event-Seite eines beendeten Events zeigt einen Bereich „Ergebnisse“ mit der Platzierung aller Teams bzw. Gilden. Das eigene Team ist hervorgehoben. Hat twogg.app persönliche Werte (z. B. Punkte), stehen sie darunter. Die Daten kommen von twogg.app (`GET /api/v1/events/{id}/results`, T-12).
- **Priorität:** v2
- **Ist-Stand:** fehlt
- **Akzeptanzkriterien:**
  - AK-43.1 Ergebnisse erscheinen erst, wenn twogg.app sie als veröffentlicht meldet.
  - AK-43.2 Das eigene Team bzw. die eigene Gilde ist hervorgehoben und ohne Scrollen sichtbar.
  - AK-43.3 Ohne Ergebnisse zeigt der Bereich „Noch keine Ergebnisse“ statt eines Fehlers.
- **Randfälle:**
  - *Offline:* zuletzt geladene Ergebnisse aus dem Event-Cache (F-08) mit Hinweis „Stand von …“.
  - *Abbruch:* entfällt.
  - *Konflikt:* Ändert das Orga-Team Ergebnisse nachträglich, gilt beim nächsten Abruf der neue Stand (ETag).

### F-44 Team-Lobby

- **Nutzerziel:** Vor dem Eventstart sehe ich, wer aus meinem Team schon da ist, und wir starten gemeinsam.
- **Verhalten:** Ab 30 Minuten vor Beginn zeigt die Event-Seite eine Lobby: alle Teammitglieder (aus twogg.app, T-5) mit Status „offline“, „Launcher offen“, „bereit“ oder „im Spiel“. Den Online-Status liefert die Präsenz des Pinecone-Servers (F-17), den Bereit-Status ein neuer Endpunkt am Pinecone-Server. Ein Knopf „Bereit“ setzt den eigenen Status. Öffnet der Server, bekommt jedes bereite Mitglied den Hinweis „Jetzt beitreten“ (F-13).
- **Priorität:** v2
- **Ist-Stand:** fehlt
- **Akzeptanzkriterien:**
  - AK-44.1 Teammitglieder ohne Pinecone erscheinen als „ohne Pinecone“, nicht als Fehler.
  - AK-44.2 Der Bereit-Status verfällt nach dem Eventstart bzw. nach 2 Stunden.
  - AK-44.3 Wer „Für Freunde unsichtbar“ ist (AK-17.4), erscheint im eigenen Team trotzdem mit Status; die Einstellung gilt nur für Freunde außerhalb des Teams.
  - AK-44.4 Die Lobby fragt höchstens alle 30 Sekunden ab.
- **Randfälle:**
  - *Offline:* Die Lobby zeigt nur das eigene Team aus dem Cache, ohne Status.
  - *Abbruch:* „Bereit“ lässt sich zurücknehmen.
  - *Konflikt:* Wechselt jemand das Team auf twogg.app, gilt das neue Team beim nächsten Abruf.

### F-45 Gilden-Seite

- **Nutzerziel:** Ich sehe meine Gilde mit Mitgliedern und ihren Events.
- **Verhalten:** Der Gilden-Button der Event-Seite öffnet eine eigene Seite: Name, Wappen/Farbe, Mitglieder (mit Pinecone-Präsenz, wenn befreundet), vergangene und kommende Events der Gilde und ihre Platzierungen (F-43). Daten von twogg.app (T-13).
- **Priorität:** v2
- **Ist-Stand:** fehlt
- **Akzeptanzkriterien:**
  - AK-45.1 Die Seite ist nur für Mitglieder der Gilde sichtbar, solange twogg.app nichts anderes freigibt.
  - AK-45.2 Ein Mitglied lässt sich direkt als Freund anfragen (F-17).
- **Randfälle:**
  - *Offline:* Cache-Stand mit Hinweis.
  - *Abbruch:* entfällt.
  - *Konflikt:* Hängt von Q-13 ab (Gilde als eigenes Feld oder Team-Typ).

### F-46 Event-Galerie

- **Nutzerziel:** Ich teile meine besten Screenshots eines Events mit der Community.
- **Verhalten:** In den Screenshots einer Event-Instanz (F-29) gibt es „In Event-Galerie teilen“. Pinecone lädt das Bild mit Minecraft-Name und Event-ID an den Pinecone-Server. Das Orga-Team gibt Bilder im Admin frei. Freigegebene Bilder erscheinen auf der Event-Seite im Launcher und auf der Pack-Seite des Servers.
- **Priorität:** v2
- **Ist-Stand:** fehlt
- **Akzeptanzkriterien:**
  - AK-46.1 Hochladen braucht eine Freunde-Sitzung (Minecraft-Identität geprüft) und eine Bestätigung pro Bild.
  - AK-46.2 Nicht freigegebene Bilder sieht nur das Orga-Team.
  - AK-46.3 Der Spieler kann eigene Bilder jederzeit löschen. „Meine Freunde-Daten löschen“ löscht sie mit.
  - AK-46.4 Höchstens 20 Bilder je Spieler und Event, höchstens 8 MB je Bild; Metadaten (EXIF o. Ä.) werden entfernt.
- **Randfälle:**
  - *Offline:* Der Knopf ist deaktiviert.
  - *Abbruch:* Ein abgebrochener Upload hinterlässt nichts auf dem Server.
  - *Konflikt:* Dasselbe Bild zweimal (gleicher Hash) wird nur einmal gespeichert.

### F-47 Event-Archiv

- **Nutzerziel:** Ich schaue mir vergangene Events an und spiele die Welt nach.
- **Verhalten:** Die Events-Seite hat einen Bereich „Vergangene Events“ mit Datum, Pack-Version, Ergebnissen (F-43) und Galerie (F-46). Hat das Orga-Team eine Welt veröffentlicht (Admin, Pinecone-Server), installiert „Welt herunterladen“ sie als Einzelspieler-Instanz mit derselben Pack-Version.
- **Priorität:** v2
- **Ist-Stand:** fehlt
- **Akzeptanzkriterien:**
  - AK-47.1 Die Archiv-Instanz ist eine normale Instanz ohne Pflicht-Mods (keine Event-Sperre nach F-11).
  - AK-47.2 Der Welt-Download nutzt die Download-Links des Servers (HMAC, Rate-Limit) und prüft SHA-512.
- **Randfälle:**
  - *Offline:* Archivliste aus dem Cache, Download deaktiviert.
  - *Abbruch:* Die halbe Instanz wird entfernt.
  - *Konflikt:* Existiert die Archiv-Instanz schon, fragt Pinecone „Ersetzen oder zweite Kopie?“.

## F. Launcher-Komfort (Plan v2)

Rein lokale Funktionen ohne neue Server-Abhängigkeit. Sie schließen Lücken gegenüber Prism Launcher und Modrinth App.

### F-48 Welt-Backups wiederherstellen und automatisch anlegen

- **Nutzerziel:** Geht eine Welt kaputt, hole ich mir einen älteren Stand zurück.
- **Verhalten:** Die Welten-Ansicht (F-28) zeigt je Welt die vorhandenen Backups (heute nur Anlegen über `core/worlds::backup`). „Wiederherstellen“ ersetzt die Welt durch das Backup; vorher legt Pinecone ein Backup des aktuellen Stands an. Pro Instanz lässt sich „Vor jedem Start sichern“ einschalten; Pinecone behält die letzten N Backups (Standard 5).
- **Priorität:** v2
- **Ist-Stand:** vorhanden (PR #24: `core/worlds` `restore`, `prune_auto`, `changed_since_backup`; `WorldsTab.svelte`). Offen: Prüfung des freien Speicherplatzes vor dem Wiederherstellen
- **Akzeptanzkriterien:**
  - AK-48.1 Wiederherstellen ist gesperrt, solange die Instanz läuft.
  - AK-48.2 Vor dem Wiederherstellen entsteht immer ein Sicherheits-Backup.
  - AK-48.3 Die Rotation löscht nur automatisch angelegte Backups, nie manuelle.
  - AK-48.4 Ein beschädigtes ZIP wird erkannt, bevor die aktuelle Welt angefasst wird.
- **Randfälle:**
  - *Offline:* funktioniert vollständig.
  - *Abbruch:* Das Entpacken läuft in einen Temp-Ordner; erst danach wird getauscht.
  - *Konflikt:* Zu wenig Speicherplatz → Abbruch vor Beginn mit Hinweis.

### F-49 Instanz duplizieren

- **Nutzerziel:** Ich probiere Mods aus, ohne meine laufende Instanz zu gefährden.
- **Verhalten:** Kontextmenü der Instanz → „Duplizieren“. Dialog: Name, „Welten mitkopieren“ (Standard aus), „Screenshots mitkopieren“ (Standard aus). Pack-Bindung (Modrinth/CurseForge/TwoGG) bleibt erhalten, bei Event-Instanzen wird sie entfernt (Kopie ist eine normale Instanz).
- **Priorität:** v2
- **Ist-Stand:** vorhanden (PR #25: `ProfileStore::duplicate`; `DuplicateDialog.svelte`)
- **Akzeptanzkriterien:**
  - AK-49.1 Die Kopie startet ohne erneuten Download von Mods.
  - AK-49.2 Spielzeit und „zuletzt gespielt“ beginnen bei null.
- **Randfälle:**
  - *Offline:* funktioniert.
  - *Abbruch:* Die halbe Kopie wird entfernt.
  - *Konflikt:* Gleicher Name bekommt ein Suffix „(2)“.

### F-50 Instanz-Gruppen und Sortierung

- **Nutzerziel:** Bei vielen Instanzen finde ich schnell die richtige.
- **Verhalten:** In der Bibliothek lassen sich Gruppen anlegen (z. B. „Events“, „Modded“, „Vanilla“) und Instanzen per Drag & Drop oder Kontextmenü zuordnen. Gruppen sind einklappbar. Sortierung: Name, zuletzt gespielt, Spielzeit, Erstelldatum. Event-Instanzen landen automatisch in „Events“, solange der Nutzer sie nicht verschiebt.
- **Priorität:** v2
- **Ist-Stand:** vorhanden (PR #28: Profilfeld `group`, Einstellungen `librarySort`/`collapsedGroups`; `LibraryPage.svelte`, `GroupDialog.svelte`). Sortierung nach Erstelldatum fehlt (kein Erstelldatum gespeichert)
- **Akzeptanzkriterien:**
  - AK-50.1 Gruppen und Sortierung bleiben nach Neustart erhalten.
  - AK-50.2 Drag & Drop hat eine Tastatur-Alternative (Kontextmenü „Verschieben nach“, AK-38.2).
  - AK-50.3 Löschen einer Gruppe löscht keine Instanzen.
- **Randfälle:**
  - *Offline:* entfällt.
  - *Abbruch:* entfällt.
  - *Konflikt:* Eine Instanz gehört zu genau einer Gruppe oder keiner.

### F-51 Konto pro Instanz

- **Nutzerziel:** Meine Event-Instanz startet immer mit meinem Hauptkonto, die Test-Instanz mit dem Zweitkonto.
- **Verhalten:** Instanz-Einstellungen → „Konto“: „Aktives Konto“ (Standard) oder ein bestimmtes Konto. Die Spielleiste zeigt das Konto, mit dem gestartet wird. Neues Feld `account` am `Profile` (`core/profiles`).
- **Priorität:** v2
- **Ist-Stand:** teilweise (PR #26: Profilfeld `account`, Spielleiste „als …“). AK-51.2 fehlt, folgt mit MVP-B
- **Akzeptanzkriterien:**
  - AK-51.1 Ist das gewählte Konto entfernt oder abgelaufen, fragt Pinecone vor dem Start statt still ein anderes zu nehmen.
  - AK-51.2 Event-Instanzen warnen, wenn das gewählte Konto nicht die bei twogg.app verknüpfte UUID hat.
- **Randfälle:**
  - *Offline:* Offline-Konten funktionieren wie bisher.
  - *Abbruch:* entfällt.
  - *Konflikt:* entfällt.

### F-52 Verhalten beim Spielstart

- **Nutzerziel:** Der Launcher soll mir beim Spielen nicht im Weg sein.
- **Verhalten:** Einstellungen → Allgemein → „Beim Spielstart“: geöffnet lassen (Standard), minimieren, ausblenden (in Tray bzw. Taskleiste), schließen. „Nach Spielende wieder anzeigen“ holt den Launcher zurück, außer bei „schließen“.
- **Priorität:** v2
- **Ist-Stand:** vorhanden (PR #26: Einstellungen `launchBehavior`/`showAfterExit`, umgesetzt in `App.svelte`)
- **Akzeptanzkriterien:**
  - AK-52.1 Bei „schließen“ läuft das Spiel weiter; Spielzeit wird beim nächsten Start aus dem Log nachgetragen oder entfällt mit Hinweis.
  - AK-52.2 Bei einem Absturz erscheint der Launcher immer wieder (F-35), auch bei „ausblenden“.
- **Randfälle:**
  - *Offline:* entfällt.
  - *Abbruch:* entfällt.
  - *Konflikt:* Laufen Downloads, wird nicht geschlossen, sondern ausgeblendet.

### F-53 Log teilen

- **Nutzerziel:** Ich schicke einem Helfer im Discord einen Link zu meinem Log statt einer Datei.
- **Verhalten:** In der Log-Ansicht (F-35) und im Absturz-Dialog: „Log teilen“. Pinecone entfernt Tokens, Pfade mit Benutzernamen und IP-Adressen (gleiche Redaktion wie `core/diagnostics`), zeigt den bereinigten Text zur Kontrolle und lädt ihn nach Bestätigung zu mclo.gs hoch. Der Link wird kopiert.
- **Priorität:** v2
- **Ist-Stand:** vorhanden (PR #30: `core/logshare`; `LogShareDialog.svelte` im Reiter „Logs“). Aus dem Absturz-Dialog führt die Benachrichtigung „Log“ in den Reiter
- **Akzeptanzkriterien:**
  - AK-53.1 Kein Upload ohne Bestätigung im Dialog; der Dialog nennt den Dienst (mclo.gs).
  - AK-53.2 Der Redaktions-Test von `core/diagnostics` gilt auch für den Upload.
  - AK-53.3 Logs über 10 MB werden auf die letzten 25 000 Zeilen gekürzt (Grenze des Dienstes prüfen, U-19).
- **Randfälle:**
  - *Offline:* Knopf deaktiviert, „Als Datei speichern“ bleibt.
  - *Abbruch:* entfällt.
  - *Konflikt:* Lehnt mclo.gs ab (429/5xx), zeigt Pinecone den Fehler und bietet „Als Datei speichern“ an.

### F-54 Vollständiger Instanz-Export und -Import

- **Nutzerziel:** Ich ziehe eine Instanz komplett auf einen anderen PC um, mit Welten und Einstellungen.
- **Verhalten:** Kontextmenü → „Exportieren“ → „Vollständig (.pinecone.zip)“ neben dem vorhandenen `.mrpack` (F-25). Auswahl der Ordner (Welten, Configs, Screenshots, Resourcepacks, Shaderpacks, `options.txt`). Import über Bibliothek → „Importieren“ oder Doppelklick (Dateizuordnung, F-39).
- **Priorität:** v2
- **Ist-Stand:** vorhanden (PR #29: `core/archive`; Modus „Komplett (Umzug)“ in `ShareDialog.svelte`, Import über die Modpack-Vorschau)
- **Akzeptanzkriterien:**
  - AK-54.1 Der Export enthält nie Konten, Tokens oder Pinecone-Einstellungen außerhalb der Instanz.
  - AK-54.2 Event-Instanzen lassen sich exportieren, importiert werden sie aber als normale Instanz.
  - AK-54.3 Der Import prüft Pfade im ZIP (kein `..`, keine absoluten Pfade).
- **Randfälle:**
  - *Offline:* funktioniert.
  - *Abbruch:* Halbe Datei bzw. halbe Instanz wird entfernt.
  - *Konflikt:* Gleicher Name bekommt ein Suffix.

### F-55 Start-Befehle und Wrapper

- **Nutzerziel:** Ich starte Minecraft unter Linux mit `gamemoderun` oder auf dem Laptop mit der dedizierten Grafikkarte.
- **Verhalten:** Instanz-Einstellungen → „Erweitert“: Befehl vor dem Start, Wrapper-Befehl, Befehl nach dem Ende. Platzhalter `$INST_DIR`, `$INST_NAME`, `$MC_VERSION`. Global vorbelegbar unter Einstellungen → Java.
- **Priorität:** v2
- **Ist-Stand:** teilweise (PR #27: `core/hooks`, Profilfelder `pre_launch`/`wrapper`/`post_exit`). Globale Vorbelegung unter Einstellungen → Java fehlt
- **Akzeptanzkriterien:**
  - AK-55.1 Scheitert der Befehl vor dem Start (Exit-Code ≠ 0), startet das Spiel nicht und die Ausgabe erscheint im Log.
  - AK-55.2 Befehle kommen nie aus Modpacks, Sync-Packs oder Event-Manifesten, nur aus der Eingabe des Nutzers.
  - AK-55.3 Der Wrapper wird ohne Shell ausgeführt (Argumente getrennt), damit Pfade mit Leerzeichen funktionieren.
- **Randfälle:**
  - *Offline:* entfällt.
  - *Abbruch:* „Stoppen“ beendet auch den Befehl vor dem Start.
  - *Konflikt:* entfällt.

## G. Freunde und Social (Plan v2)

Baut auf F-17 und dem Pinecone-Server auf. F-41 Discord Rich Presence wird in diesen Block vorgezogen.

### F-56 Einladungen

- **Nutzerziel:** Ich lade einen Freund mit einem Klick in meine Welt bzw. auf meinen Server ein, auch wenn er das Modpack noch nicht hat.
- **Verhalten:** Während ich spiele, zeigt die Freundesliste bei jedem Freund „Einladen“. Die Einladung enthält Server (Adresse aus der Präsenz) und Pack-Bezug (Teilen-Link nach F-25: `pinecone://twogg/<pack>` oder `pinecone://modrinth/<projekt>`). Der Empfänger bekommt eine Benachrichtigung (F-37): „Annehmen“ installiert fehlendes Pack bzw. nutzt eine passende vorhandene Instanz und tritt bei (Quick Play). Server: Tabelle `invites`, `POST /api/v1/invites`, Abruf zusammen mit der Präsenz.
- **Priorität:** v2
- **Ist-Stand:** teilweise (Server-PR #5, Launcher-PR #31). AK-56.4 (nur bei Event-Freigabe) folgt mit MVP-B
- **Akzeptanzkriterien:**
  - AK-56.1 Einladungen sind nur zwischen Freunden möglich und verfallen nach 15 Minuten.
  - AK-56.2 Höchstens 10 Einladungen je Spieler und Minute (Server-Rate-Limit).
  - AK-56.3 Einladungen in lokale Einzelspielerwelten (LAN) zeigen „nicht möglich“ statt einer falschen Adresse.
  - AK-56.4 Eine Einladung zu einem Event-Server wird nur angezeigt, wenn der Empfänger für das Event freigegeben ist (sonst Hinweis).
- **Randfälle:**
  - *Offline:* Einladen deaktiviert.
  - *Abbruch:* Abgelehnte Einladungen bekommt der Absender als „abgelehnt“ angezeigt.
  - *Konflikt:* Mehrere Einladungen: die neueste je Absender gilt.

### F-57 Freund per Code hinzufügen

- **Nutzerziel:** Ich füge einen Freund hinzu, ohne seinen Minecraft-Namen genau zu kennen.
- **Verhalten:** Jeder Spieler hat einen kurzen Freundescode (z. B. `PINE-7K3Q`) und einen Link `pinecone://friend/<code>`. Eingabe oder Klick schickt eine Anfrage. Der Code lässt sich neu erzeugen.
- **Priorität:** v2
- **Ist-Stand:** vorhanden (Server-PR #5, Launcher-PR #31)
- **Akzeptanzkriterien:**
  - AK-57.1 Codes sind nicht erratbar (≥ 40 bit Zufall) und werden bei „neu erzeugen“ sofort ungültig.
  - AK-57.2 Falsche Codes werden je IP begrenzt (gegen Durchprobieren).
- **Randfälle:**
  - *Offline:* deaktiviert.
  - *Abbruch:* entfällt.
  - *Konflikt:* Eigener Code → Hinweis „Das bist du“.

### F-58 Sync-Packs teilen

- **Nutzerziel:** Ich gebe meinen Freunden meine Tastenbelegung und Einstellungen für das Event-Pack.
- **Verhalten:** Sync-Pack (F-31) → „Teilen“ erzeugt einen Link `pinecone://sync/<code>` (Ablage am Pinecone-Server, 30 Tage). Öffnen zeigt eine Vorschau und legt das Sync-Pack an.
- **Priorität:** v2
- **Ist-Stand:** vorhanden (Server-PR #5, Launcher-PR #32)
- **Akzeptanzkriterien:**
  - AK-58.1 Geteilt werden Projekte (Modrinth/CurseForge-IDs) sowie `options.txt`-Werte und Tasten. Eigene Dateien des Sync-Packs werden nicht geteilt.
  - AK-58.2 Die Vorschau zeigt alle Werte, bevor etwas übernommen wird.
- **Randfälle:**
  - *Offline:* deaktiviert.
  - *Abbruch:* entfällt.
  - *Konflikt:* Gleichnamiges Sync-Pack bekommt ein Suffix.

## H. Admin und Server (Plan v2)

Funktionen für das Orga-Team in der Admin-Oberfläche des Pinecone-Servers (`pinecone.twogg.app/admin`).

### F-59 Nutzungsstatistik

- **Nutzerziel:** Das Orga-Team sieht, welche Packs und Launcher-Versionen genutzt werden.
- **Verhalten:** Das Admin-Dashboard zeigt (heute nur Summen aus `stats` in `src/admin.rs`) zusätzlich: Downloads je Pack und Version über die Zeit (Tageswerte), Verteilung der Launcher-Versionen (aus `X-Pinecone-Client`, Tageszähler), aktive Freunde-Profile je Tag. Gespeichert werden nur Tageszähler, keine IPs, UUIDs oder Einzelereignisse.
- **Priorität:** v2
- **Ist-Stand:** teilweise (Gesamtsummen vorhanden)
- **Akzeptanzkriterien:**
  - AK-59.1 Aus der Statistik lässt sich kein einzelner Spieler ableiten (nur Zähler je Tag).
  - AK-59.2 Tageszähler älter als 2 Jahre werden gelöscht.
- **Randfälle:**
  - *Offline:* entfällt.
  - *Abbruch:* entfällt.
  - *Konflikt:* entfällt.

### F-60 Banner und Mindestversion

- **Nutzerziel:** Das Orga-Team erreicht alle Launcher sofort, z. B. bei Server-Wartung oder einer kaputten Version.
- **Verhalten:** Im Admin unter Einstellungen: Banner (Text, Stufe Info/Warnung, Zeitraum, optional Link) und `minLauncherVersion`. `/api/v1/info` liefert beides. Der Launcher zeigt das Banner oben auf der Startseite (schließbar je Banner-ID) und bei zu alter Version eine Tafel „Update nötig“ mit Knopf (F-34).
- **Priorität:** v2
- **Ist-Stand:** fehlt
- **Akzeptanzkriterien:**
  - AK-60.1 Ein geschlossenes Banner erscheint nicht wieder, ein geändertes (neue ID) schon.
  - AK-60.2 Unter `minLauncherVersion` bleiben Bibliothek und Einzelspieler nutzbar; gesperrt werden nur Event-Beitritt und TwoGG-Packs.
- **Randfälle:**
  - *Offline:* kein Banner, keine Sperre.
  - *Abbruch:* entfällt.
  - *Konflikt:* Abgelaufene Banner liefert der Server nicht mehr aus.

### F-61 Server-Pack erzeugen

- **Nutzerziel:** Das Orga-Team bekommt aus einer Pack-Version direkt die Dateien für den Event-Server.
- **Verhalten:** Im Admin bei einer Pack-Version: „Server-Pack erzeugen“. Der Server liest das `.mrpack`, nimmt nur Dateien mit `env.server` ≠ `unsupported`, legt `overrides` und `server-overrides` dazu und fügt einen Hinweis zum Loader-Installer (Version aus dem Pack) bei. Ergebnis: ZIP zum Download im Admin.
- **Priorität:** v2
- **Ist-Stand:** fehlt
- **Akzeptanzkriterien:**
  - AK-61.1 Client-only Mods (`env.server = unsupported`) fehlen im Server-Pack.
  - AK-61.2 Jede Datei wird gegen den SHA-512 des `.mrpack` geprüft.
  - AK-61.3 Das Server-Pack ist nur für Admins abrufbar, nicht öffentlich.
- **Randfälle:**
  - *Offline:* entfällt.
  - *Abbruch:* Eine abgebrochene Erzeugung hinterlässt keine Datei.
  - *Konflikt:* Fehlt `env` im Pack, gilt die Datei als „Server: ja“ mit Warnung in der Liste.

Abgrenzung zum Nicht-Ziel „Kein Hosting oder Verwalten der Minecraft-Server“ (`00`): F-61 erzeugt nur Dateien. Hochladen, Starten und Betreiben des Eventservers bleiben außerhalb von Pinecone.

### F-62 Pack-Version zurückziehen

- **Nutzerziel:** Eine fehlerhafte Pack-Version erreicht keine weiteren Spieler, und wer sie hat, erfährt davon.
- **Verhalten:** Im Admin: Version → „Zurückziehen“ mit Grund. Die Version verschwindet aus Listen und Update-Prüfung. Launcher mit dieser Version zeigen „Diese Version wurde zurückgezogen: <Grund>“ und bieten die neueste nicht zurückgezogene Version an (Update nach F-12/F-24).
- **Priorität:** v2
- **Ist-Stand:** fehlt
- **Akzeptanzkriterien:**
  - AK-62.1 Eine zurückgezogene Version lässt sich wiederherstellen.
  - AK-62.2 Ist sie die einzige Version, warnt der Admin vor dem Zurückziehen.
- **Randfälle:**
  - *Offline:* Hinweis erst beim nächsten Abruf.
  - *Abbruch:* entfällt.
  - *Konflikt:* Ist die Version an ein laufendes Event gebunden, verlangt der Admin eine zweite Bestätigung.

### F-63 Admin-Konten, Rollen und Protokoll

- **Nutzerziel:** Mehrere Leute im Orga-Team arbeiten im Admin, und man sieht, wer was geändert hat.
- **Verhalten:** Admin-Konten mit Rollen „Admin“ (alles) und „Redakteur“ (Neuigkeiten, Empfehlungen, Galerie-Freigabe, keine Einstellungen und Konten). Ein Protokoll zeigt Zeitpunkt, Konto, Aktion und Objekt jeder Änderung.
- **Priorität:** v2
- **Ist-Stand:** fehlt (ein gemeinsamer Admin-Zugang)
- **Akzeptanzkriterien:**
  - AK-63.1 Der bisherige Zugang wird beim Update als erstes Admin-Konto übernommen.
  - AK-63.2 Das Protokoll lässt sich nicht im Admin bearbeiten oder löschen; Einträge älter als 1 Jahr werden automatisch gelöscht.
  - AK-63.3 Der letzte Admin kann sich nicht selbst herabstufen oder löschen.
- **Randfälle:**
  - *Offline:* entfällt.
  - *Abbruch:* entfällt.
  - *Konflikt:* Gleichzeitige Änderungen am selben Objekt: die spätere gewinnt, beide stehen im Protokoll.
