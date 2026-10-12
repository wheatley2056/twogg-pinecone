# Anhang – Entwurf Datenschutzerklärung „TwoGG Pinecone“

> **Entwurf, keine Rechtsberatung.** Dieser Text beschreibt, was der Code von Launcher 0.9.5 und Pinecone-Server tatsächlich verarbeitet (Stand 12.10.2026, inkl. Launcher-PR #22 und Server-PR #4). Er muss vor der Veröffentlichung von der verantwortlichen Stelle bzw. einer fachkundigen Person geprüft werden.
>
> Platzhalter in eckigen Klammern sind auszufüllen. Stellen mit **[PRÜFEN]** brauchen eine rechtliche Bewertung.
>
> Nach der Veröffentlichung trägst du die Adresse im Admin unter Einstellungen › Rechtliches ein. Sie erscheint dann im Launcher und auf den Pack-Seiten.

---

## Datenschutzerklärung für TwoGG Pinecone

### 1. Verantwortlicher

[NAME / VEREIN / PERSON], [ANSCHRIFT], E-Mail: [KONTAKT-E-MAIL]

Impressum: [URL IMPRESSUM]

### 2. Worum es geht

TwoGG Pinecone besteht aus zwei Teilen:

- dem **Launcher**, einem Programm auf deinem Computer, mit dem du Minecraft startest, Modpacks installierst und TwoGG-Events spielst,
- dem **Pinecone-Server** unter `pinecone.twogg.app`, der TwoGG-Modpacks, Neuigkeiten und die Freundesliste bereitstellt.

Pinecone ist kein offizielles Minecraft-Produkt und nicht von Mojang oder Microsoft genehmigt oder mit ihnen verbunden.

**Pinecone enthält keine Werbung, kein Tracking und keine Analyse-Dienste.**

### 3. Daten, die nur auf deinem Computer bleiben

Diese Daten speichert der Launcher im Datenordner auf deinem Gerät und überträgt sie nicht an uns:

- Instanzen, Mods, Welten, Screenshots, Einstellungen,
- Spielzeit je Instanz,
- Liste deiner Minecraft-Konten (Name, UUID),
- Protokolldateien (7 Tage).

Zugangsschlüssel (Tokens) für dein Microsoft-Konto, deine twogg.app-Verknüpfung und die Freundesliste liegen im Schlüsselbund deines Betriebssystems.

Du löschst diese Daten so:

- Konten entfernst du unter Einstellungen › Konten.
- Den Rest löschst du, indem du den Datenordner löschst (Pfad unter Einstellungen › Allgemein) oder Pinecone deinstallierst.

Eine Diagnose-ZIP (Einstellungen › Allgemein) erstellst du selbst und gibst sie selbst weiter. Sie enthält keine Kontodaten.

### 4. Daten auf dem Pinecone-Server

**a) Modpacks, Neuigkeiten, TwoGG-Auswahl**

- Beim Abruf verarbeitet der Server technisch deine IP-Adresse.
- Für die Begrenzung von Download-Links und CurseForge-Anfragen hält er sie kurzzeitig im Arbeitsspeicher, höchstens für das jeweilige Zeitfenster (10 bzw. 1 Minute). Er speichert sie nicht in einer Datenbank.
- Der vorgeschaltete Reverse-Proxy schreibt Zugriffsprotokolle mit IP-Adresse. Sie werden nach [FRIST, Vorschlag 14 Tage] gelöscht. **[PRÜFEN: tatsächliche Einstellung von Nginx Proxy Manager auf core01]**
- Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO (sicherer Betrieb, Schutz vor Missbrauch). **[PRÜFEN]**

**b) Freundesliste** (nur wenn du sie nutzt)

- Gespeichert werden:
  - dein Minecraft-Name und deine UUID,
  - deine Freunde und offenen Anfragen,
  - der Zeitpunkt deiner letzten Aktivität und was du gerade spielst (Instanzname, Version, Server).
- Die Anmeldung läuft über die Sitzungsprüfung von Mojang; dein Passwort sehen wir nie. Der Sitzungsschlüssel wird nur gehasht gespeichert und läuft nach 30 Tagen ab.
- Deine Freunde sehen deinen Online-Status und deine Aktivität. Andere sehen sie nicht.
- **Löschen:**
  - Einstellungen › Konten › „Meine Freunde-Daten löschen“ entfernt Profil, Freunde und Anfragen sofort.
  - Profile ohne Aktivität seit 365 Tagen löscht der Server automatisch.
- Rechtsgrundlage: Art. 6 Abs. 1 lit. b DSGVO (Bereitstellung der von dir genutzten Funktion). **[PRÜFEN]**

**c) CurseForge-Inhalte**

- Suchst oder installierst du Inhalte von CurseForge, leitet der Pinecone-Server deine Anfrage an CurseForge (Overwolf) weiter. Das betrifft Suchbegriff, Filter und Projekt- bzw. Datei-IDs. Weitergeleitet wird ohne deine IP-Adresse und ohne Kontodaten.
- Der Server speichert die Anfragen nicht.
- Die Dateien selbst lädt der Launcher direkt von den Servern von CurseForge.

**d) Geplante Funktionen (Plan v2, noch nicht umgesetzt – erst aufnehmen, wenn sie veröffentlicht werden)**

- *Team-Lobby, Einladungen, Freundescode:* Bereit-Status (höchstens 2 Stunden), Einladungen (15 Minuten) und dein Freundescode werden auf dem Pinecone-Server gespeichert und mit „Meine Freunde-Daten löschen“ entfernt. **[PRÜFEN]**
- *Event-Galerie:* Lädst du einen Screenshot hoch, speichern wir das Bild (ohne Metadaten) mit deinem Minecraft-Namen und dem Event. Das Orga-Team prüft es vor der Veröffentlichung. Du kannst es jederzeit löschen. Rechtsgrundlage: Einwilligung (Art. 6 Abs. 1 lit. a DSGVO). **[PRÜFEN]**
- *Log teilen:* Nur auf deinen Klick wird ein bereinigtes Log an mclo.gs (Aternos GmbH) übertragen. **[PRÜFEN: Anbieter, Speicherdauer, Datenschutzerklärung von mclo.gs]**
- *Geteilte Sync-Packs:* Teilst du ein Sync-Pack, speichern wir es 30 Tage.
- *Statistik:* Der Server zählt Downloads und Launcher-Versionen als Tageszähler ohne IP-Adresse oder Spieler-ID.

### 5. Dienste, die der Launcher direkt anspricht

Bei diesen Diensten sieht der jeweilige Anbieter deine IP-Adresse. Es gelten deren Datenschutzerklärungen. **[PRÜFEN: Drittlandübermittlung, insbesondere USA]**

| Dienst | Wofür | Anbieter |
|---|---|---|
| Microsoft / Xbox Live / Minecraft Services | Anmeldung mit deinem Microsoft-Konto, Skins | Microsoft, Mojang |
| Mojang-Server | Spieldateien, Java, Assets | Mojang |
| Fabric, Quilt, Forge, NeoForge | Mod-Loader | jeweilige Projekte |
| Modrinth | Suche und Download von Inhalten | Rinth, Inc. |
| CurseForge-CDN | Download von Dateien | Overwolf |
| GitHub | Prüfung auf Launcher-Updates | GitHub (Microsoft) |
| Discord (lokal, nur wenn eingeschaltet, Plan v2) | Anzeige „Spielt …“ in deinem Discord-Profil | Discord Inc. – die Übertragung läuft über die Discord-App auf deinem Gerät |
| twogg.app | Events, dein Anmeldestatus (nur wenn du dein TwoGG-Konto verknüpfst) | TwoGG, siehe Datenschutzerklärung von twogg.app [URL] |

Schriften sind im Launcher enthalten. Es entsteht keine Verbindung zu Google Fonts.

### 6. Verknüpfung mit twogg.app

- Verknüpfst du dein TwoGG-Konto, liest Pinecone von twogg.app:
  - dein Profil,
  - deine Event-Anmeldungen, Teams und Gilde,
  - die Freigabe für Eventserver.
- Der Launcher speichert nur einen Zugangsschlüssel im Schlüsselbund und einen Zwischenstand der Events auf deinem Gerät.
- Trennen löscht beides.
- Was twogg.app speichert, regelt die Datenschutzerklärung von twogg.app.

### 7. Kinder und Jugendliche

Minecraft wird auch von Minderjährigen gespielt. **[PRÜFEN: Hinweis zur Einwilligung der Erziehungsberechtigten bzw. Altersgrenzen der genutzten Dienste]**

### 8. Deine Rechte

Du hast das Recht auf:

- Auskunft (Art. 15 DSGVO),
- Berichtigung (Art. 16),
- Löschung (Art. 17),
- Einschränkung (Art. 18),
- Datenübertragbarkeit (Art. 20),
- Widerspruch (Art. 21).

Schreib dafür an [KONTAKT-E-MAIL].

Du kannst dich bei einer Datenschutz-Aufsichtsbehörde beschweren, z. B. bei [ZUSTÄNDIGE AUFSICHTSBEHÖRDE].

### 9. Stand

[DATUM]

---

## Hinweise für die verantwortliche Stelle (nicht veröffentlichen)

- Quellen zu Pflichtinhalten:
  - DSGVO Art. 13 (Informationspflicht), Art. 17 (Löschung).
  - DSK-Kurzpapiere Nr. 10 und 11 (datenschutzkonferenz-online.de, als „in Überarbeitung“ markiert).
- Die beschriebenen Verarbeitungen entsprechen dem Code. Wenn sich etwas ändert, z. B. neue Server-Endpunkte, Telemetrie oder Logs, muss dieser Text nachgezogen werden. Zuständig: [VERANTWORTLICHE PERSON].
- Offene Punkte stehen in `06-risiken-und-offene-fragen.md`, Q-08.
