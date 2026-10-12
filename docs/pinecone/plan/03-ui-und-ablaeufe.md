# 03 – UI und Abläufe

Grundlage ist `docs/design/design-system.md` (im Folgenden „Design“). Jeder Screen nutzt nur die dort definierten **Tokens** (§3 und §14), **Schriften** (§4), **Formen** (§5) und **Komponenten** (§9). Verweise wie „Design §9.8“ nennen die Komponente.

## 1. Regeln für alle Screens

**Erlaubte Tokens:**

- Flächen: `--bg`, `--bar`, `--side`, `--stage`, `--surface`, `--surface-2`, `--surface-active`
- Linien: `--line`, `--line-strong`, `--line-dashed`
- Text: `--text`, `--text-soft`, `--text-muted`, `--text-dim`
- Akzent: `--harz`, `--harz-hover`, `--harz-split`, `--harz-edge`, `--harz-light`, `--ink`
- Status: `--nadel`
- Schrift: `--font-pixel`, `--font-ui`, `--font-mono`
- Form: `--cut-1`, `--cut-2`, `--edge-harz`

Dazu kommen die im Design als Hex genannten Sonderflächen:

- `#1A140D` gesperrte Zeile (§9.11)
- `#17130F` vergangenes Event (§9.8)
- `#1E160D` TwoGG-Auswahl (§10.3)
- `#3A2F22` aktives Segment in Detailseiten (§9.4)
- `#2E261D`/`#33281F` Schalter aus (§9.10)

Bei anderem Akzent steht `--harz*` für den gewählten Akzent (`02` §2).

**Zustände:** Jeder Screen definiert vier Zustände. So werden sie mit den Design-Bausteinen dargestellt:

| Zustand | Darstellung |
|---|---|
| **leer** | Fläche `--surface` mit `--cut-2`, Pixel-Titel 16–22 px (`--font-pixel`, `--text`), ein Satz in `--text-soft`, ein Primär-Button (Harz) oder gestrichelter Button (§9.3) |
| **lädt** | Platzhalterflächen in `--surface`/`--surface-2` in der Form des späteren Inhalts, ohne Animation bei „Animationen reduzieren“. Fortschritt als Balken 4 px Harz auf `--line-strong`, Prozent in Mono (§9.8) |
| **Fehler** | Tafel nach §9.13 (`--bar` 0,84–0,88, Rand `--line-strong`), Text `--text`, Detail `--text-muted`, Sekundär-Button „Erneut versuchen“. Das Design hat **keine Fehlerfarbe**; der Code nutzt `--danger` außerhalb des Designs (offene Frage Q-11) |
| **offline** | Meta-Chip (§9.9) „Offline · Stand von <Zeit>“ in `--text-soft` im Kopf des Bereichs. Netzabhängige Buttons sind deaktiviert (`--surface-2`, Text `--text-dim`), mit Tooltip |

**Verboten** (Design §12):

- linke Icon-Leiste, rechtes Freunde-Panel
- runde Karten, Pills, Verläufe, Glows, Neon
- Weiß auf Harz
- Emojis
- farbige Seitenstreifen
- Grün als Fläche (außer Theme Kiefer bzw. Akzent Nadel)

## 2. Screens

### 2.1 Shell: Kopfleiste und Spielleiste (alle Screens)

- **Zweck:** Navigation und Spielstart, auf jedem Screen gleich (Design §8, §9.1, §9.2).
- **Bereiche:**
  - **Kopfleiste** 52 px, `--bar`, Unterlinie `--line`:
    - Logo und „Pinecone“ (Pixelify 20/600).
    - Tabs Spielen · Bibliothek · Entdecken · Events · Skins (Pixelify 17/600, aktiv `--text` plus 4 px Harz-Strich im Trapez-Schnitt).
    - Rechts die Icon-Buttons Downloads, Benachrichtigungen (7-px-Harz-Punkt bei Ungelesenem) und Einstellungen.
    - Dann ein Trenner und die Fensterknöpfe.
  - **Spielleiste** 88 px, `--bar`, Oberlinie `--line`, Grid `1fr auto 1fr`:
    - Links die Instanzwahl (300 px, `--surface`, `--cut-2`, Icon 48, Name 15/700, Version Mono 12 `--text-muted`, Chevron hoch).
    - Mitte SPIELEN (290 × 62, Harz, `--ink`, Pixelify 29/700, Oktagon 12 px, Knopf-Kante).
    - Rechts der Freunde-Stapel (28-px-Quadrate, −10 px, „<n> online“), ein Trenner und das Konto (Pixel-Avatar 40, Name 14/700, Kontoart in `--text-dim`).
- **Zustände:**
  - *leer:* Keine Instanz, dann zeigt die Instanzwahl „Instanz anlegen“ (gestrichelt) und SPIELEN ist deaktiviert. Kein Konto, dann zeigt das Konto „Anmelden“.
  - *lädt:* SPIELEN zeigt die Stufe (F-05) in Mono 12 unter dem Label und den Fortschritt als 4-px-Balken am unteren Rand des Buttons. Läuft das Spiel, wird SPIELEN zu „BEENDEN“ in `--surface-2` mit Text `--text`.
  - *Fehler:* Ein Startfehler erscheint als Eintrag im Benachrichtigungs-Panel; SPIELEN kehrt in den Grundzustand zurück.
  - *offline:* Freunde-Stapel „Freunde offline nicht verfügbar“. SPIELEN bleibt aktiv (Offline-Fall in F-05).
- **Funktionen:** F-02, F-05, F-17, F-37

### 2.2 Spielen (Start)

- **Zweck:** nächstes Event, letzte Instanzen, Neuigkeiten (F-15).
- **Bereiche (Design §10.1):**
  - **Bühne** `--stage` mit Wald-Illustration rechts und Schuppenmuster. Links der Event-Teaser:
    - Event-Badge (§9.9) z. B. `TWOGG EVENT · 03.10.` oder `TERMIN FOLGT`
    - Pixel-Titel 64–66 px
    - Text 16 px `--text-soft`
    - Status-Chip „Angemeldet · <Team> · <Gilde>“ (F-09)
    - Primär-Button „Zum Event“ (öffnet Events-Detail) und Link-Button „Zur Anmeldung auf twogg.app“ (F-18)
  - **Tafel „Event-Kalender“** rechts oben (300 px, Overlay §9.13): Event-Einträge (§9.8) mit Countdown in Mono, Link „Discord: twogg.chat“.
  - **Neuigkeiten:** Liste aus Event-Einträgen (§9.8) mit „NEU“-Mini-Badge. Ein Klick öffnet die Neuigkeit in einer Tafel.
  - **Leiste „Zuletzt gespielt“** (138 px): 4 Instanz-Kacheln (§9.8) plus Kachel „TwoGG Server“ mit gestricheltem Rahmen, Ping in Mono (F-16).
  - **Bewerbungsbanner** (F-18), nur bei offener Phase: Overlay-Leiste am unteren Bühnenrand mit Link-Button.
- **Abweichung von der Referenz:**
  - Der Button „Gilde wählen“ aus `referenz/01-spielen.html` entfällt. Er widerspricht der festen Entscheidung, dass es keine Anmeldelogik im Launcher gibt.
  - An seine Stelle tritt „Zum Event“.
  - Die Gilde erscheint nur im Status-Chip.
- **Zustände:**
  - *leer:* Keine Instanzen, dann zeigt die Bühne „Willkommen bei Pinecone“ und „Leg deine erste Instanz an“ (Primär). Ohne Events zeigt der Kalender „Gerade ist kein Event geplant“.
  - *lädt:* Platzhalter für Bühnentext und Kalendereinträge. „Zuletzt gespielt“ kommt sofort aus lokalen Daten.
  - *Fehler:* twogg.app antwortet mit 5xx. Die Bühne zeigt „twogg.app antwortet gerade nicht. Deine Instanzen kannst du trotzdem starten.“, der Kalender ist ausgeblendet.
  - *offline:* Bühne und Kalender aus dem Cache mit Chip „Offline · Stand von <Zeit>“. Ohne Cache gilt der Fehlertext.

### 2.3 Bibliothek und Instanz

- **Zweck:** Instanzen und Sync-Packs verwalten (F-11, F-19 bis F-31, F-35, F-40).
- **Bereiche (Design §10.2):**
  - **Bibliotheksliste** links 280 px auf `--side`:
    - Filterfeld (§9.6)
    - Gruppen `TWOGG EVENTS`, `EIGENE INSTANZEN`, `SYNC-PACKS` (Gruppenlabels §9.12)
    - Listenelemente (§9.12, aktiv `--surface-active` mit `--cut-1` 7 px, Name `--harz-hover`)
    - gestrichelter Button „Modpack importieren“
  - **Header** mit Schuppenmuster rechts:
    - Icon 96 px mit 16-px-Schnitt
    - Pixel-Titel 40 px
    - Meta-Chips (§9.9): Version in Mono, Loader, „<n> h gespielt“, bei Event-Instanzen „Synchron mit Event-Pack“ in `--nadel`
    - Aktionen als Icon- bzw. Sekundär-Buttons: Ordner, Exportieren/Teilen, Version wechseln, Löschen im Kontextmenü
    - **Kein SPIELEN-Button** (Design §10.2)
  - **Unter-Tabs** (§9.5): Inhalt · Welten · Server · Screenshots · Logs · Einstellungen, jeweils mit Zähler in Mono 11.
  - **Inhalt:**
    - Werkzeugzeile mit Suche, Segment-Tabs Alle/Mods/Ressourcen/Shader (§9.4), „Updates prüfen“ bzw. „Alle aktualisieren“ (Primär) und „Hinzufügen“ (gestrichelt).
    - Sync-Pack-Schalter (§9.10).
    - Inhaltstabelle (§9.11) mit gesperrten Pflicht-Mod-Zeilen auf `#1A140D` und Schloss-Icon.
  - **Logs:** Live-Konsole in Mono 12 auf `--surface`, Filter als Segment-Tabs, Suche, „Diagnose exportieren“. Bei Absturz steht oben die **Absturztafel** (§2.9).
  - **Einstellungen der Instanz:** Formular aus Eingabefeldern (§9.6), Dropdowns (§9.7) und Schaltern.
  - **Sync-Pack-Ansicht** (Bestand 0.9.5): gleiche Header-Struktur mit Bereichen Art, Instanzen, Inhalte, Steuerung und Spieleinstellungen.
- **Zustände:**
  - *leer:* Ohne Instanzen zeigt die Liste „Noch keine Instanzen“ und den Primär-Button „Instanz anlegen“. Eine leere Inhaltstabelle zeigt „Noch keine Inhalte. Zieh Dateien hierher oder such in Entdecken.“
  - *lädt:* Tabellenzeilen als Platzhalter. Die Erkennung unbekannter Dateien zeigt den Status „wird erkannt …“ in `--text-dim`.
  - *Fehler:* Instanzordner nicht lesbar führt zur Fehlertafel mit „Ordner öffnen“.
  - *offline:* Chip im Header. „Updates prüfen“ und „Hinzufügen aus Entdecken“ sind deaktiviert; lokale Schalter funktionieren.

### 2.4 Entdecken

- **Zweck:** Inhalte suchen und installieren (F-21, F-22, F-24).
- **Bereiche (Design §10.3):**
  - Titel (Pixelify 28–30), Segment-Tabs Modpacks · Mods · Ressourcenpakete · Shader · Datenpakete, Split-Button „Installieren in [Instanz] ▾“ (§9.3).
  - Suchfeld und Filter-Dropdowns „Loader“, „Version“, „Kategorie“, „Quelle“, „Sortierung“ (§9.7).
  - Streifen „TwoGG-Auswahl“ auf `#1E160D` mit Schuppenmuster links und 3 Ergebnis-Karten.
  - Ergebnisraster mit 3 Spalten (§9.8), Button „Holen“, Statusvarianten „Installiert“ (`--nadel` mit Haken) und Fortschrittsbalken.
- **Zustände:**
  - *leer:* Suche ohne Treffer zeigt „Nichts gefunden. Versuch einen anderen Begriff oder entferne Filter.“
  - *lädt:* 6 Karten-Platzhalter.
  - *Fehler:* Modrinth oder CurseForge nicht erreichbar zeigt eine Tafel mit Quelle in Mono. Bei Rate-Limit: „Modrinth bremst gerade, kurz warten“.
  - *offline:* Leerzustand „Entdecken braucht Internet“, Karten der TwoGG-Auswahl ausgeblendet.

### 2.5 Events (neu gestaltet, siehe §3.1)

- **Zweck:** Event-Kalender, Status, Event-Pack, Countdown, Beitritt (F-08 bis F-14, F-18).

### 2.6 Skins (neu gestaltet, siehe §3.2)

- **Zweck:** Skins und Umhang (F-30).

### 2.7 Einstellungen

- **Zweck:** alle Einstellungen (F-01 bis F-04, F-07, F-14, F-22, F-32 bis F-34, F-36 bis F-38).
- **Bereiche (Design §10.4):**
  - Nav links 230 px auf `--side`: Allgemein · Darstellung · Java & Arbeitsspeicher · Konten · Downloads & Quellen · TwoGG-Verknüpfung (Listenelemente §9.12).
  - **Allgemein:** Sprache (Dropdown), Intro anzeigen, Update-Kanal (Segment Stable/Beta), „Nach Updates suchen“, Benachrichtigungen je Art (Schalter), Event-Hinweise mit Vorlaufzeiten (zwei Dropdowns, Standard 60/10 min), „Diagnose exportieren“, „Logs öffnen“.
  - **Darstellung:** Theme-Karten Rinde/Kiefer/Winterland mit Mini-Vorschau (Kopfleiste, Inhalt, Spielleiste mit Akzentbalken), Akzent-Swatches 44 px mit `--cut-2` (Auswahl als Inset-Rand), Live-Vorschau, Schalter „Zapfenschuppen-Muster“, „Pixel-Überschriften“, „Animationen reduzieren“.
  - **Java & Arbeitsspeicher:** gefundene Javas (Liste mit Mono-Pfaden), Standard-Speicher, JVM-Presets.
  - **Konten:** Microsoft-Konten, Offline-Konto, „Für Freunde unsichtbar“.
  - **Downloads & Quellen:** Modrinth und CurseForge (Status „aktiv“ bzw. „in diesem Build nicht verfügbar“), gleichzeitige Downloads.
  - **TwoGG-Verknüpfung:**
    - Nicht verknüpft: Primär-Button „Mit twogg.app verbinden“ und ein erklärender Text, dass sich der Browser öffnet und Pinecone dein Passwort nie sieht.
    - Verknüpft: TwoGG-Name, verknüpfte Minecraft-UUID in Mono, Hinweis bei abweichendem aktivem Konto, Sekundär-Button „Trennen“.
- **Zustände:**
  - *leer:* entfällt, Einstellungen haben immer Standardwerte.
  - *lädt:* Java-Liste und Update-Prüfung zeigen „wird gesucht …“.
  - *Fehler:* Speichern fehlgeschlagen zeigt eine Tafel „Einstellungen konnten nicht gespeichert werden“ mit Pfad in Mono.
  - *offline:* „Nach Updates suchen“ und „Mit twogg.app verbinden“ sind deaktiviert.

### 2.8 Panels der Kopfleiste (Downloads, Benachrichtigungen)

- **Zweck:** laufende Vorgänge und Hinweise (F-05, F-10, F-24, F-37).
- **Bereiche:**
  - Tafel nach §9.13, rechtsbündig unter dem jeweiligen Icon-Button (kein Seitenpanel über die ganze Höhe).
  - **Downloads:** Liste mit Name, Fortschrittsbalken, Prozent in Mono und Abbrechen (Icon-Button).
  - **Benachrichtigungen:** Einträge mit Titel 14/700, Text 13 `--text-soft`, Zeit in Mono 11 `--text-dim` und optionalem Aktions-Link in Harz. „Alle gelesen“ ist ein Sekundär-Button.
- **Zustände:**
  - *leer:* „Gerade läuft nichts“ bzw. „Keine neuen Hinweise“.
  - *lädt:* entfällt, weil das Panel live ist.
  - *Fehler:* Ein fehlgeschlagener Download bleibt mit „Erneut versuchen“ in der Liste.
  - *offline:* Laufende Downloads pausieren mit „Wartet auf Verbindung“.

### 2.9 Dialoge und Tafeln

Alle Dialoge sind Tafeln nach §9.13 über abgedunkeltem Inhalt (`--bar` 0,84). Titel in Pixelify 22, Aktionen rechts unten (Primär rechts, Sekundär links daneben).

| Dialog | Funktionen | Besonderheit |
|---|---|---|
| Microsoft-Login | F-01 | Code in Mono 28 (`--harz-hover`), „Code kopieren“, „Seite öffnen“, Wartezustand, Abbrechen |
| Neue Instanz / Instanz bearbeiten | F-19 | Version und Loader als Dropdowns (§9.7), Motivwahl als Kacheln |
| Import / Vorschau | F-24, F-26, F-39 | Pack-Kopf wie Instanz-Header in klein, Meta-Chips |
| Pack-Update | F-12, F-24 | Changelog, Liste „ändert sich / bleibt / neu“, Primär „Aktualisieren“ |
| Fehlende Dateien (CurseForge) | F-22, F-26 | Liste mit Link-Buttons zu CurseForge, Drop-Fläche gestrichelt |
| Teilen / Exportieren | F-25 | Link in Mono mit „Kopieren“ |
| Version wechseln | F-20 | Planliste mit Status-Chips |
| **Absturztafel** (neu) | F-35, F-36 | siehe unten |
| Bestätigen (Löschen) | F-19, F-28 | Sekundär „Abbrechen“, Primär mit Aktion (keine Fehlerfarbe, siehe Q-11) |

Die **Absturztafel** steht oben im Logs-Tab:

- Badge `ABSTURZ` (Event-Badge-Stil §9.9)
- Titel 16 px „Minecraft ist abgestürzt“
- erkannte Ursache als Satz in `--text-soft`, darunter der Auslöser in Mono 12
- Buttons „Diagnose exportieren“ (Primär), „Crash-Report öffnen“ (Sekundär), „Im Discord melden“ (Link-Button)
- Liste weiterer Crash-Reports als Tags (§9.9)

### 2.10 Intro und Erststart

- **Intro:**
  - eigenes rahmenloses Fenster, 5 s
  - Ablauf: Harz-Orange-Fläche → Farbwischer in `--nadel`, Zapfenbraun `#5A3416`, `--text` (Pergament), `--bar` (Nacht) → farbiges Logo (Design §6) → „Pinecone“ tippt sich in Pixelify ein → Claim „A TWOGG LAUNCHER“ in Mono, Laufweite 2 px
  - Quelle: `docs/design/intro/Intro.dc.html`
  - Mit „Animationen reduzieren“ erscheint ein Standbild (Logo und Wortmarke) für höchstens 1 s.
- **Erststart (Willkommensansicht im Inhaltsbereich, Shell sichtbar):** drei Schritt-Karten (`--surface`, `--cut-2`, Nummer in Pixelify):
  1. „Melde dich mit Microsoft an“
  2. „Verbinde dein TwoGG-Konto (optional)“
  3. „Wähle ein Event oder leg eine Instanz an“

  Erledigte Schritte zeigen einen Haken in `--nadel`.
- **Zustände:**
  - *leer:* ist der Ausgangszustand.
  - *lädt:* entfällt.
  - *Fehler:* Ein Fehler in einem Schritt (z. B. Login) erscheint in der Schrittkarte, der Rest bleibt bedienbar.
  - *offline:* Schritte 1 und 2 sind deaktiviert mit „Später anmelden“. Schritt 3 bietet nur „Instanz anlegen“ mit installierten Versionen.

## 3. Neu zu gestaltende Screens

Beide Screens bestehen nur aus Bausteinen aus Design §9 und Tokens aus §14. Sie ersetzen bzw. bestätigen die in 0.9.x gebauten Fassungen; Abweichungen im Code sind Aufgaben in `05`.

### 3.1 Events

```
┌ Kopfleiste ─────────────────────────────────────────────────────────────────┐
├──────────────┬──────────────────────────────────────────────────────────────┤
│ EVENTS       │  [Banner/Bühne --stage, Schuppenmuster rechte Hälfte]        │
│ ─ KOMMEND ─  │  [Event-Badge: TWOGG EVENT · ROLEPLAY]                       │
│ ▣ REVIVAL:   │  REVIVAL: AVALON          (Pixelify 40)                      │
│   Avalon     │  Mittelalter-RP-Sandbox im Land Avalon. (--text-soft 16)     │
│   Termin folgt│ [Chip Fabric 1.21.11] [Chip Teams 1] [Chip Mit Bewerbung]   │
│ ▣ WINTERLAND │ ┌ COUNTDOWN ───────┐ ┌ DEIN STAND ───────────────────────┐   │
│ ─ BEENDET ─  │ │ 03 T 14 H 22 M   │ │ Angemeldet · Die Treue Hand        │   │
│ ▢ LATEST 2026│ │ (Pixelify 30)    │ │ [Auf twogg.app ändern ↗]           │   │
│ ▢ TALES      │ └──────────────────┘ └────────────────────────────────────┘   │
│              │ ┌ EVENT-PACK ──────────────────────────────────────────────┐ │
│              │ │ Avalon-Pack  1.4.0 (Mono)  [Synchron ✓] | [Update 1.5.0] │ │
│              │ │ [Instanz öffnen]                     [■ SERVER BEITRETEN]│ │
│              │ └──────────────────────────────────────────────────────────┘ │
│              │ [Discord: twogg.chat ↗]  [Team finden auf twogg.app ↗]       │
├──────────────┴──────────────────────────────────────────────────────────────┤
│ Spielleiste                                                                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Bereich | Bausteine und Tokens |
|---|---|
| Eventliste links (280 px, `--side`) | Gruppenlabels `KOMMEND` / `BEENDET` (Mono 11, `--text-dim`). Event-Einträge (§9.8): `--surface`, `--cut-1` 8 px, Icon 38 nach Event-Farbcode (§3 Instanz-/Event-Farbcodes), Name 14/700, Untertitel 12 `--text-muted`, Mini-Badge „NEU“. Beendete auf `#17130F`, Text `--text-dim`. Aktiv: `--surface-active` + Name `--harz-hover` (§9.12) |
| Detail-Kopf | `--stage` mit Banner aus twogg.app (falls vorhanden), Overlay-Tafel §9.13 für Text. Schuppenmuster rechte Hälfte, Deckkraft 0,08–0,12. Event-Badge §9.9. Titel Pixelify 38–40. Meta-Chips §9.9 (Version in Mono) |
| Countdown | Karte `--surface`, `--cut-2`. Label `COUNTDOWN` (Mono 11, `--text-dim`). Zahlen Pixelify 30 `--text`, Einheiten Mono 12 `--text-muted`. Ohne Termin: „Termin folgt“. Bei laufendem Event: Meta-Chip „Läuft gerade“ in `--nadel` |
| Dein Stand | Karte `--surface`, `--cut-2`. Label `DEIN STAND`. Meta-Chip mit Status, Team und Gilde. Link-Button „Auf twogg.app ändern“. Nicht verknüpft: Text „Verbinde dein TwoGG-Konto, um deinen Stand zu sehen“ + Sekundär-Button „Verbinden“ (führt zu F-07) |
| Event-Pack | Karte `--surface`, `--cut-1`. Name 15/700, Version Mono 12. Status-Chip „Synchron mit Event-Pack“ (`--nadel`-Text) oder „Update <v>“ (Harz-Text mit Pfeil-hoch-Icon, wie §9.11). Buttons: „Event-Pack installieren“ (Primär) bzw. „Instanz öffnen“ (Sekundär) |
| Server beitreten | Primär-Button Harz mit Knopf-Kante, `--cut-2`, Server-Stapel-Icon. Deaktiviert mit Grund als Text darunter (`--text-dim`): „Noch nicht freigegeben“ / „Melde dich zuerst auf twogg.app an“ / „Mit Offline-Konto nicht möglich“ |
| Links | Link-Buttons §9.3 (Harz-Text + externes Pfeil-Icon) |
| Bewerbungsbanner (F-18) | Overlay-Leiste §9.13 über dem Detailende, Link-Button. Nur bei offener Phase |

**Zustände:**

- *leer:* twogg.app liefert keine Events. Die Liste zeigt „Gerade sind keine Events geplant“, das Detail zeigt Logo und Link „Events auf twogg.app“.
- *lädt:* 4 Listen-Platzhalter und ein Detail-Platzhalter. Ist ein Cache vorhanden, wird er sofort gezeigt und im Hintergrund aktualisiert.
- *Fehler:*
  - twogg.app 5xx: Tafel „Events lassen sich gerade nicht laden“ und „Erneut versuchen“.
  - `401`: In „Dein Stand“ erscheint „Verbindung zu twogg.app erneuern“.
- *offline:* Cache mit Chip „Offline · Stand von <Zeit>“. „Server beitreten“ und „Event-Pack installieren“ sind deaktiviert. „Instanz öffnen“ funktioniert. Ohne Cache gilt der Fehlerzustand.

### 3.2 Skins

```
┌ Kopfleiste ─────────────────────────────────────────────────────────────────┐
├───────────────────────────────┬─────────────────────────────────────────────┤
│ [Stage --stage, Schuppen]     │ SKINS (Pixelify 28)   [+ Importieren ▾]     │
│        3D-Vorschau            │ [Suche]  [Segment: Alle | Klassisch | Schmal]│
│      (Spielfigur, Animation)  │ ┌Karte┐ ┌Karte┐ ┌Karte┐ ┌Karte┐             │
│ [Segment: Ruhe|Gehen|Winken…] │ │Kopf │ │Kopf │ │Kopf │ │ + ┆ │  gestrichelt│
│ AKTIV: Wanderer (Mono)        │ └─────┘ └─────┘ └─────┘ └─────┘             │
│ [■ ANWENDEN] [Bearbeiten]     │ UMHANG                                      │
│                               │ [Kachel][Kachel][Kachel: Kein Umhang]       │
├───────────────────────────────┴─────────────────────────────────────────────┤
│ Spielleiste                                                                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Bereich | Bausteine und Tokens |
|---|---|
| Vorschau links (≈ 40 %) | `--stage`, Schuppenmuster. 3D-Figur. Segment-Tabs (§9.4) für Animationen; mit „Animationen reduzieren“ nur „Ruhe“. Label `AKTIV` (Mono 11) + Skinname in Mono 12 |
| Aktionen | Primär „Anwenden“ (Harz, `--cut-2`), Sekundär „Bearbeiten“ (öffnet Skin-Editor), Icon-Buttons Exportieren/Löschen mit `aria-label` |
| Bibliothek rechts | Titel Pixelify 28. Split-Button „Importieren ▾“ (Datei / Spielername / Von meinem Konto). Suche §9.6, Segment-Tabs Modell. Raster aus Kacheln (§9.8 Instanz-Kachel): Kopf-Render 44 px, Name 14/700, Modell in Mono 11. Ausgewählt: `--surface-active` + Harz-Unterkante. Letzte Kachel gestrichelt „Skin hinzufügen“ |
| Umhang | Gruppenlabel `UMHANG`. Kacheln wie oben. „Kein Umhang“ als gestrichelte Kachel |

**Zustände:**

- *leer:* Keine Skins in der Bibliothek, dann zeigt die Vorschau den aktuellen Konto-Skin und das Raster „Importiere deinen ersten Skin“ (gestrichelter Button).
- *lädt:* Kachel-Platzhalter. Die Vorschau zeigt die Silhouette in `--surface-2`.
- *Fehler:* „Anwenden“ scheitert, dann erscheint eine Tafel mit dem Grund (Ratenbegrenzung oder Netzwerk).
- *offline:* „Anwenden“, „Von Spielername“ und „Von meinem Konto“ sind deaktiviert. Die Bibliothek funktioniert.

### 3.3 Plan v2: neue Bereiche in bestehenden Screens

Die v2-Funktionen bekommen keine eigenen Hauptseiten, sondern Bereiche in vorhandenen Screens. Es gelten die Regeln aus §1 und die Bausteine aus §9 des Design-Systems.

| Funktion | Ort | Bausteine und Verhalten |
|---|---|---|
| F-43 Ergebnisse | §3.1 Events, Detail eines beendeten Events, unter dem Hero | Tabelle Platz/Team/Punkte, eigene Zeile `--surface-active` + Harz-Kante. Zustände: *leer* „Noch keine Ergebnisse“, *offline* Stand-Hinweis |
| F-44 Team-Lobby | §3.1 Events, ersetzt ab T−30 min den Countdown-Block | Liste der Teammitglieder mit Kopf-Render und Status-Punkt (wie Freundesliste), Primärknopf „Bereit“ / „Nicht mehr bereit“. Bei Serveröffnung Tafel „Jetzt beitreten“ |
| F-45 Gilden-Seite | Unterseite von §3.1 (Gilden-Button) | Kopf mit Gildenname und -farbe, Reiter Mitglieder / Events. Mitglied-Kontextmenü „Als Freund anfragen“ |
| F-46 Galerie | §2.3 Instanz → Screenshots (Kontextmenü „In Event-Galerie teilen“), §3.1 Event-Detail (Raster) | Bestätigungsdialog mit Vorschau, Event-Name und Hinweis „wird vom Orga-Team geprüft“. Eigene Bilder mit Status „wartet“/„freigegeben“ und „Löschen“ |
| F-47 Archiv | §3.1 Events, Segment „Kommend | Vergangen“ | Karte wie Event-Karte, gedämpft. Knopf „Welt herunterladen“ nur, wenn veröffentlicht |
| F-48 Welt-Backups | §2.3 Instanz → Welten, Zeile aufklappbar | Backup-Liste mit Datum, Größe, „automatisch“-Chip. „Wiederherstellen“ mit Bestätigungsdialog. Instanz-Einstellungen: Schalter „Vor jedem Start sichern“, Zahl „Behalten“ |
| F-49 Duplizieren | §2.3 Kontextmenü der Instanz | Dialog Name + zwei Schalter (Welten, Screenshots), Fortschritt im Downloads-Panel (§2.8) |
| F-50 Gruppen | §2.3 Bibliotheksliste | Einklappbare Gruppenköpfe (Label-Stil Mono 11, Anzahl), Sortier-Dropdown rechts neben der Suche, Kontextmenü „Verschieben nach“ |
| F-51 Konto pro Instanz | §2.3 Instanz → Einstellungen; §2.1 Spielleiste | Dropdown „Konto“ mit „Aktives Konto“ zuerst. Spielleiste zeigt Kopf des Startkontos |
| F-52 Startverhalten | §2.7 Einstellungen → Allgemein | Segment „Geöffnet lassen | Minimieren | Ausblenden | Schließen“, Schalter „Nach Spielende wieder anzeigen“ |
| F-53 Log teilen | §2.3 Instanz → Logs; Absturz-Dialog (§4.7) | Sekundärknopf „Log teilen“ → Dialog mit bereinigtem Text (Mono 12, scrollbar), Hinweis auf mclo.gs, Primärknopf „Hochladen und Link kopieren“ |
| F-54 Voll-Export | §2.3 Kontextmenü „Exportieren ▸ Vollständig“ | Dialog mit Ordner-Checkliste und geschätzter Größe |
| F-55 Start-Befehle | §2.3 Instanz → Einstellungen → „Erweitert“ (eingeklappt) | drei Textfelder Mono 12, Platzhalter-Hilfe darunter |
| F-56 Einladungen | §2.1 Freundesliste (Popover), §2.8 Benachrichtigungen | Knopf „Einladen“ je Freund während des Spiels; Benachrichtigung mit „Annehmen“/„Ablehnen“ und Pack-Name |
| F-57 Freundescode | §2.1 Freundesliste, Kopf | eigener Code in Mono mit „Kopieren“ und „Neu erzeugen“; Eingabefeld akzeptiert Name oder Code |
| F-58 Sync-Pack teilen | Sync-Pack-Ansicht | Knopf „Teilen“ → Link-Dialog wie `ShareDialog.svelte`; Öffnen eines Links → Vorschau-Tafel (§2.9) |
| F-60 Banner | §2.2 Spielen, oberhalb der Neuigkeiten | Hinweisleiste Info/Warnung, schließbar. „Update nötig“ als Tafel (§2.9) |
| F-59, F-61 bis F-63 | Admin-Oberfläche des Pinecone-Servers (React-Admin) | Dashboard-Diagramme (F-59), Aktion „Server-Pack erzeugen“ und „Zurückziehen“ an der Version (F-61, F-62), Ressourcen „Konten“ und „Protokoll“ (F-63), Galerie-Freigabe (F-46) |

## 4. Kernabläufe

Jeder Ablauf nennt die betroffenen Funktionen. „System“ meint Pinecone.

### 4.1 Erststart mit Intro (F-06, F-01, F-07, F-10, F-19)

1. Nutzer startet Pinecone zum ersten Mal.
2. System zeigt das Intro (5 s), mit „Animationen reduzieren“ ein Standbild. Klick oder Esc überspringt.
3. System öffnet das Hauptfenster auf „Spielen“ mit der Willkommensansicht (§2.10).
4. Schritt 1: Nutzer wählt „Mit Microsoft anmelden“ und es folgt Ablauf §4.2. Alternativ wählt er „Später“.
5. Schritt 2: Nutzer wählt „TwoGG verbinden“ und es folgt Ablauf §4.3. Alternativ wählt er „Überspringen“.
6. Schritt 3: Gibt es ein kommendes Event, mit dem der Nutzer verknüpft ist, schlägt das System „Event-Pack für <Event> installieren“ vor. Sonst bietet es „Instanz anlegen“ an.
7. Nach Abschluss eines Schritts 3 verschwindet die Willkommensansicht dauerhaft (Merker in `settings.json`).
8. Abbruch: Beim nächsten Start erscheint die Ansicht mit dem ersten offenen Schritt.

### 4.2 Microsoft-Login (F-01, F-02)

1. Nutzer klickt in der Spielleiste auf „Anmelden“ oder unter Einstellungen › Konten auf „Microsoft-Konto hinzufügen“.
2. System fordert einen Device Code an und zeigt den Dialog mit Code und „Seite öffnen“.
3. Nutzer öffnet die Microsoft-Seite im Browser, gibt den Code ein und meldet sich an.
4. System fragt im vorgegebenen Intervall ab, bis die Anmeldung bestätigt ist (höchstens 15 min).
5. System durchläuft Xbox Live → XSTS → Minecraft Services, prüft den Besitz und lädt das Profil.
6. System speichert den Refresh-Token im Schlüsselbund und das Konto in `accounts.json` und macht es aktiv.
7. Die Spielleiste zeigt Kopf und Namen. Ist ein TwoGG-Konto verknüpft und weicht dessen Minecraft-UUID ab, erscheint ein Hinweis (F-07).
8. Fehler:
   - XSTS-`XErr` führt zu einem verständlichen Text (`02` §7.1).
   - Keine Lizenz: AK-01.4.
   - Ablauf des Codes: „Der Code ist abgelaufen“ mit „Neu versuchen“.

### 4.3 TwoGG-Konto verknüpfen (F-07)

1. Nutzer klickt „Mit twogg.app verbinden“.
2. System erzeugt `code_verifier`, `code_challenge` (S256) und `state` und öffnet einen Loopback-Listener auf `127.0.0.1:<zufälliger Port>`.
3. System öffnet den Standardbrowser mit `GET /oauth/authorize?...` bei twogg.app (`04` §2).
4. Nutzer meldet sich auf twogg.app an (Discord oder E-Mail, wie dort üblich) und bestätigt „Pinecone darf deinen Event-Status lesen“.
5. twogg.app leitet auf `http://127.0.0.1:<port>/callback?code=…&state=…` um. Der Browser zeigt eine Seite „Du kannst dieses Fenster schließen“.
6. System prüft `state`, tauscht den Code mit `code_verifier` gegen Tokens (`POST /oauth/token`) und speichert den Refresh-Token im Schlüsselbund.
7. System verknüpft die Minecraft-UUID:
   1. Es holt `POST /api/v1/me/minecraft/challenge`.
   2. Es führt den Mojang-Session-Join mit dem aktiven Minecraft-Konto aus.
   3. Es sendet `POST /api/v1/me/minecraft/verify` (`04` §2.4).
8. Einstellungen › TwoGG-Verknüpfung zeigt TwoGG-Name und UUID. Events und „Dein Stand“ laden neu.
9. Fehler:
   - Browser geschlossen: Der Listener endet nach 5 min.
   - `state` falsch: Abbruch ohne Tokens.
   - `409 mc_uuid_taken`: siehe F-07.

### 4.4 Event beitreten (F-08 bis F-13, F-05)

1. Nutzer öffnet „Events“ und wählt ein Event.
2. System zeigt das Detail mit Countdown, „Dein Stand“ und dem Event-Pack-Status (`event_pack_status`).
3. Steht „Dein Stand“ auf „Nicht angemeldet“, führt „Zur Anmeldung auf twogg.app“ zur Website. Pinecone meldet nicht selbst an.
4. Fehlt die Event-Instanz, klickt der Nutzer „Event-Pack installieren“. Das System:
   1. lädt das Manifest und prüft die Signatur,
   2. prüft die Mindestversion,
   3. lädt das `.mrpack` und prüft SHA-512,
   4. installiert,
   5. bindet Sync-Packs ein,
   6. zeigt den Fortschritt im Downloads-Panel.
5. Zum Eventstart klickt der Nutzer „Server beitreten“.
6. System ruft `GET /api/v1/events/{id}/join` ab:
   - Ohne Freigabe zeigt es den Grund, Ende.
   - Mit Freigabe geht es weiter.
7. System wählt die Event-Instanz in der Spielleiste und prüft auf Pflicht-Update (F-12):
   - `required` und neuer: zuerst Ablauf §4.5.
8. System startet nach §4.6 mit Server-Adresse (Quick Play).
9. Minecraft verbindet direkt mit dem Eventserver.

### 4.5 Modpack-Update (F-12, F-24)

1. System erkennt beim Öffnen der Events-Seite, der Instanz oder vor dem Start eine neuere Pack-Version.
2. Nicht `required`: Die Instanz zeigt „Update verfügbar“, und der Nutzer öffnet den Update-Dialog.
3. System zeigt Changelog und Plan (ändert sich / bleibt / neu / entfällt).
4. Nutzer bestätigt. Bei `required` vor dem Start entfällt die Bestätigung, und es kommt der Hinweis „Das Event verlangt Version <v>, wird aktualisiert“.
5. System legt einen Snapshot an, lädt und prüft Manifest und Pack und wendet den Drei-Wege-Abgleich an.
6. System aktualisiert `pack.lock.json` inklusive `manifest.locked` und schließt mit der Integritätsprüfung ab.
7. Fehler oder Abbruch: Das System stellt den Snapshot wieder her und meldet den Grund.

### 4.6 Spielstart (F-05, F-11, F-31, F-04)

1. Nutzer drückt SPIELEN (oder „Beitreten“ in Events, Freunden oder der Serverliste).
2. System prüft, ob die Instanz schon läuft. Wenn ja, Abbruch mit Hinweis.
3. System gleicht die Sync-Packs ab (ohne Netz übersprungen).
4. Event-Instanz: Das System prüft die Integrität (F-11), repariert sie und deaktiviert unerlaubte Zusatz-Mods.
5. System installiert den Loader und die Version (Bibliotheken, Client).
6. System stellt Java bereit.
7. System lädt die Assets.
8. System erneuert die Kontotokens. Ohne Netz und mit Microsoft-Konto bricht es mit Hinweis ab.
9. System baut den Startbefehl (JVM-Preset, Speicher, Quick Play) und startet den Prozess.
10. Die Spielleiste zeigt „BEENDEN“, der Logs-Tab läuft live mit.
11. Spielende:
    1. Spielzeit verbuchen.
    2. Exit-Code prüfen.
    3. Bei Absturz geht es weiter mit Ablauf §4.7.

### 4.7 Absturz (F-35, F-36, F-37, F-53)

1. Das Spiel endet mit einem Exit-Code ungleich 0, oder ein neuer Eintrag erscheint in `crash-reports/`.
2. System analysiert Crash-Report und Log (`crash_info`): fehlende Abhängigkeit, Java-Version, Speicher, doppelte Mod-ID oder „unbekannt“.
3. System erzeugt eine Benachrichtigung „<Instanz> ist abgestürzt“ im Panel und als OS-Hinweis, falls aktiviert.
4. Klick auf den Hinweis: Die Bibliothek öffnet die Instanz im Tab „Logs“ mit der Absturztafel.
5. Nutzer wählt eine der drei Aktionen:
   - „Diagnose exportieren“ (ZIP, F-36)
   - „Crash-Report öffnen“
   - „Im Discord melden“ (Link twogg.chat)
6. Bei Event-Instanzen nennt die Tafel zusätzlich „Event-Pack prüfen“, eine manuelle Integritätsprüfung.

## 5. Prüfliste Tastatur und Screenreader (AK-38.2)

Für jeden Screen aus §2 und §3 gilt:

1. Mit Tab sind alle Bedienelemente der Reihe nach erreichbar: Kopfleiste → Inhalt → Spielleiste.
2. Der Fokus ist sichtbar (`:focus-visible`, 2 px in Akzentfarbe).
3. Die Kopfleisten-Tabs nutzen `role="tablist"` / `role="tab"` / `aria-selected`.
4. Unter-Tabs und Segment-Tabs sind mit den Pfeiltasten bedienbar.
5. Schalter sind `<button aria-pressed>`.
6. Dialoge fangen den Fokus. Esc schließt, der Fokus kehrt zum Auslöser zurück.
7. SPIELEN ist per Leertaste und Eingabetaste auslösbar.
8. Kontextmenüs sind auch per Tastatur erreichbar (Menütaste oder Shift+F10).
9. Fortschritt wird über `aria-live="polite"` angesagt (Startstufen, Downloads).
10. Dekorative SVGs (Wald, Schuppen) tragen `aria-hidden="true"`.
