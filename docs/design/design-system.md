# Pinecone – Design System

> Kontext für KI-Assistenten: Dieses Dokument beschreibt das Design System von **Pinecone**, dem Minecraft-Launcher der deutschen Gaming-Community **TwoGG**. Wenn dir Screenshots von Pinecone geschickt werden, gelten die Regeln hier. Neue Screens, Komponenten oder Änderungen sollen sich exakt an diese Tokens, Formen und Muster halten.

---

## 1. Markenkern

**Pinecone** = Kiefernzapfen. Der Launcher fühlt sich an wie ein Abend im Schwarzwald: warm, dunkel, holzig, mit Harz-Amber als Licht.

**Leitidee:** Vertraut für Minecraft-Spieler, aber kein Klon.

| Vorbild | Was übernommen wird | Was bewusst *nicht* übernommen wird |
|---|---|---|
| Original Minecraft Launcher | Großer SPIELEN-Button unten mittig, Instanzwahl links daneben, Hero-Bild | Grüner Button, Mojang-Optik |
| Modrinth | Instanz-Bibliothek, Inhaltsverwaltung | Linke Icon-Leiste, rechtes Freunde-Panel, dunkelgrünes Rundum-Theme, runde Karten |
| CurseForge | Inhaltstabelle mit Quelle/Status/Schalter | Orange, Werbeflächen |
| NoRisk Client | Pixel-Typo, wählbare Akzentfarbe | Lila/Blau-Neon, Glow-Effekte |

**Tonalität:** Deutsch, Du-Form, kurz und freundlich. Keine Emojis in der UI.

---

## 2. Design-Prinzipien

1. **Eckig statt rund.** Kaum `border-radius`. Stattdessen **Schuppen-Schnitt** (abgeschrägte Ecken per `clip-path`). Maximal 2–4 px Radius bei Kleinteilen (Schalter).
2. **Warm statt grün.** Grundton ist Rinden-Braun. Grün ist **nur** Statusfarbe („online", „aktuell", „installiert").
3. **Harz ist Aktion.** Amber `#E9A23B` markiert genau das, was man klicken soll: SPIELEN, Installieren, aktiver Tab.
4. **Pixel für Titel, klar für Inhalt.** Pixel-Schrift nur für Überschriften, Tabs, Buttons. Fließtext bleibt gut lesbar.
5. **Kanten statt Schatten.** Tiefe entsteht durch eine dunklere Unterkante (`inset box-shadow`), wie ein Minecraft-Knopf. Keine weichen Drop-Shadows, keine Glows, keine Verläufe.
6. **TwoGG ist eingebaut.** Events, Server und kuratierte Inhalte der Community sind fester Bestandteil.

---

## 3. Farben

### Grundtöne (Theme „Rinde", Standard)

| Token | Hex | Verwendung |
|---|---|---|
| `--bg` | `#12100D` | App-Hintergrund |
| `--bar` | `#0D0B09` | Kopfleiste, Spielleiste, Overlays (auch als `rgba(13,11,9,0.84–0.88)`) |
| `--side` | `#0F0D0B` | Seitenspalten (Bibliotheksliste, Einstellungs-Nav) |
| `--stage` | `#17120D` | Hero-/Banner-Hintergrund |
| `--surface` | `#1C1814` | Karten, Eingabefelder, Kacheln |
| `--surface-2` | `#241F19` | Chips, Sekundär-Buttons, Icon-Flächen |
| `--surface-active` | `#2A2219` | Ausgewähltes Listenelement |
| `--line` | `#2A231B` | Trennlinien in Leisten |
| `--line-strong` | `#33281F` | Rahmen, Fensterrand |
| `--line-dashed` | `#4A3D2E` | Gestrichelte Rahmen („importieren", „hinzufügen") |

### Text

| Token | Hex | Verwendung |
|---|---|---|
| `--text` | `#F1E8DA` | Primärtext („Pergament") |
| `--text-soft` | `#D2C4AC` | Beschreibungen, Fließtext auf dunklem Grund |
| `--text-muted` | `#B5A790` | Sekundärtext, inaktive Tabs, Meta |
| `--text-dim` | `#8F8270` | Labels, Abschnittsüberschriften, Hinweise |

### Akzente

| Token | Hex | Verwendung |
|---|---|---|
| `--harz` | `#E9A23B` | Primäre Aktion, aktiver Tab-Strich, Badges, Links |
| `--harz-hover` | `#F3BC64` | Hover, hervorgehobener Text auf Dunkel |
| `--harz-split` | `#D48F2C` | Zweiter Teil von Split-Buttons |
| `--harz-edge` | `#B8742A` | Unterkante (Tiefe) von Harz-Buttons |
| `--harz-light` | `#F6C77A` | Obere Lichtkante von Harz-Buttons |
| `--ink` | `#1B1206` | Text **auf** Harz (nie Weiß auf Amber) |
| `--nadel` | `#8CCB7E` | Nur Status: online, aktuell, installiert, synchron |

### Instanz-/Event-Farbcodes (Icon-Hintergründe)

| Thema | Hintergrund | Icon-Farbe |
|---|---|---|
| Event / Avalon (Wappen) | `#2A1F14` | `#E9A23B` |
| Survival (Block) | `#1E2A1C` | Gras `#8CCB7E`, Erde `#7A5230` / `#5E3E22` |
| Bauen (Haus) | `#1B2232` | `#9FC3E8` |
| QoL (Regler) | `#2B2817` | `#E6D27A` |
| LATEST / Atlantis (Wellen) | `#142329` | `#7FB8C9` |
| WINTERLAND (Schneeflocke) | `#1B2232` | `#9FC3E8` |

### Weitere Themes (Einstellungen › Darstellung)

| Theme | `--bg` | `--bar` | `--surface` | `--text` | `--text-muted` |
|---|---|---|---|---|---|
| **Rinde** (Standard) | `#12100D` | `#0D0B09` | `#241F19` | `#F1E8DA` | `#B5A790` |
| **Kiefer** | `#0E1512` | `#0A0F0D` | `#1C2822` | `#E9EFE9` | `#9DB0A4` |
| **Winterland** (hell) | `#EDF1F0` | `#D5DEDC` | `#FFFFFF` | `#14211C` | `#4B5F56` |

**Wählbare Akzentfarben:** Harz `#E9A23B` (Standard) · Nadel `#8CCB7E` · Gletscher `#7FB8E0` · Beere `#E58A98` · Flechte `#CFC870`. Text auf jedem Akzent bleibt `#1B1206`.

---

## 4. Typografie

Alle drei Schriften über Google Fonts:
`https://fonts.googleapis.com/css2?family=Figtree:wght@400;500;600;700&family=Pixelify+Sans:wght@500;600;700&family=JetBrains+Mono:wght@400;500&display=swap`

| Rolle | Schrift | Gewicht | Größen |
|---|---|---|---|
| Logo, Titel, Tabs, SPIELEN | **Pixelify Sans** | 600–700 | Tabs 17 · Abschnitt 16–22 · Seitentitel 28–30 · Instanz-Titel 38–40 · Hero 64–66 · SPIELEN 29 |
| UI & Fließtext | **Figtree** | 400–700 | Meta 12 · Standard 13–14 · Karten-Titel 14–15 · Hero-Text 16 |
| Versionen, Loader, Labels, Adressen, Zahlen | **JetBrains Mono** | 400–500 | 10–13 |

**Regeln**
- Abschnitts-Labels: JetBrains Mono, 11 px, `letter-spacing: 1–1.5px`, GROSSBUCHSTABEN, Farbe `--text-dim` (z. B. `TWOGG EVENTS`, `THEMA`).
- Versionsangaben immer in Mono: `Fabric 1.21.11`, `Vanilla 26.2`.
- Fließtext `line-height: 1.5–1.55`. Pixel-Titel `line-height: 0.95–1`.
- Kein Inter, Roboto oder Arial.

---

## 5. Formen

### Schuppen-Schnitt (Signatur-Form)

Abgeschrägte Ecke(n) per `clip-path`. Das ist die wichtigste Form im System.

```css
/* Eine Ecke (oben links) – Karten, Badges, Listenelemente */
clip-path: polygon(8px 0, 100% 0, 100% 100%, 0 100%, 0 8px);

/* Zwei Ecken (oben links + unten rechts) – Buttons, Kacheln, Instanzwahl */
clip-path: polygon(8px 0, 100% 0, 100% calc(100% - 8px), calc(100% - 8px) 100%, 0 100%, 0 8px);

/* Alle vier Ecken (Oktagon) – NUR der große SPIELEN-Button */
clip-path: polygon(12px 0, calc(100% - 12px) 0, 100% 12px, 100% calc(100% - 12px),
                   calc(100% - 12px) 100%, 12px 100%, 0 calc(100% - 12px), 0 12px);
```

**Schnittgrößen:** Badge 6–7 px · Karte/Liste 7–10 px · Button 8–9 px · SPIELEN 12 px · großes Instanz-Icon 16 px · Marken-Panel 20 px.

**Achtung:** `border` folgt dem `clip-path` nicht. Geschnittene Elemente bekommen keinen Rahmen, nur Flächenfarbe.

### Tiefe („Knopf-Kante")

```css
/* Harz-Button */
box-shadow: inset 0 -5px 0 #B8742A, inset 0 2px 0 #F6C77A;  /* groß */
box-shadow: inset 0 -3px 0 #B8742A;                          /* klein */
/* Dunkler Sekundär-Button */
box-shadow: inset 0 -3px 0 #0D0B09;
```

### Radien
- Standard: **0**.
- Schalter: Track 3 px, Knopf 2 px.
- Fenster: 10 px (nur äußerer Rahmen).

### Zapfenschuppen-Muster
Dezentes Schuppenmuster hinter Bannern und Heros, immer nur auf einer Teilfläche (meist rechte Hälfte).

```html
<pattern id="pc-scales" width="28" height="20" patternUnits="userSpaceOnUse">
  <path d="M0 10 Q7 0 14 10 Q21 0 28 10 M-7 20 Q0 10 7 20 Q14 10 21 20 Q28 10 35 20"
        fill="none" stroke="#E9A23B" stroke-width="1.2" stroke-opacity="0.10"/>
</pattern>
```

Deckkraft 0.08–0.12. Auf dem Marken-Board als Musterfläche bis 0.45.

---

## 6. Logo

Pixeliger Kiefernzapfen auf einem 9 × 13 Raster mit zwei grünen Nadeln oben. Immer mit `shape-rendering="crispEdges"` rendern.

```html
<svg width="18" height="26" viewBox="0 0 9 13" shape-rendering="crispEdges">
  <path fill="#8CCB7E" d="M3 0h1v1H3z M5 0h1v1H5z M4 1h1v1H4z"/>
  <path fill="#5A3416" d="M3 2h3v1H3z M2 3h1v1H2z M6 3h1v1H6z M1 4h1v1H1z M7 4h1v1H7z M0 6h1v2H0z M8 6h1v2H8z M1 9h1v1H1z M7 9h1v1H7z M2 10h1v1H2z M6 10h1v1H6z M3 11h1v1H3z M5 11h1v1H5z M4 12h1v1H4z"/>
  <path fill="#E9A23B" d="M3 3h3v1H3z M2 4h5v1H2z M1 5h7v4H1z M2 9h5v1H2z M3 10h3v1H3z M4 11h1v1H4z"/>
  <path fill="#A8641F" d="M4 3h1v1H4z M3 4h1v1H3z M5 4h1v1H5z M2 5h1v1H2z M6 5h1v1H6z M3 6h1v1H3z M5 6h1v1H5z M2 7h1v1H2z M6 7h1v1H6z M3 8h1v1H3z M5 8h1v1H5z M4 9h1v1H4z M4 10h1v1H4z"/>
  <path fill="#F6C77A" d="M4 5h1v1H4z M4 7h1v1H4z"/>
</svg>
```

**Farben im Logo:**
- Nadeln `#8CCB7E`
- Kontur „Zapfen" `#5A3416`
- Schuppen `#E9A23B`
- Schuppen-Schatten `#A8641F`
- Glanzpunkte `#F6C77A`

**Varianten**
- **Wortmarke:** Logo + „Pinecone" in Pixelify Sans 600, 10 px Abstand.
- **Hell:** auf `#E9EFEE`, Nadeln `#3E7A4A`, Text `#14211C`.
- **App-Symbol:** Logo auf quadratischer Fläche `#2A1F14`.
- **Einfarbig:** Silhouette `#1B1206` auf Harz.
- **Claim:** „A TWOGG LAUNCHER" (JetBrains Mono, letter-spacing 2px).

---

## 7. Icons

- Linien-Icons, 24 × 24 viewBox, `stroke="currentColor"`, `stroke-width` 1.8 (klein 2–2.6), `stroke-linecap="round"`, `fill="none"`.
- Ausnahmen gefüllt: Play-Dreieck `M6 4l14 8-14 8z` sowie Instanz-Icons (Wappen, Block).
- Standardgrößen: 14–19 px in Leisten, 20–26 px in Kacheln, bis 54 px im Instanz-Header.
- Keine Emojis.

**Wiederkehrende Motive:**

| Icon | Bedeutung |
|---|---|
| Wappen/Schild | Avalon/Event |
| Isometrischer Block | Vanilla/Survival |
| Haus | Bauwelt |
| Regler | QoL |
| Wellen + Pfeil | LATEST/Atlantis |
| Schneeflocke | WINTERLAND |
| Server-Stapel | Server |
| Blitz | Performance |

---

## 8. Layout & Fensterstruktur

**Fenster:** 1280 × 800 px, Rahmen `1px solid #33281F`, Radius 10 px.

```
┌────────────────────────────────────────────────────────────────┐
│ KOPFLEISTE 52px  [Logo Pinecone] [Spielen|Bibliothek|Entdecken| │
│                  Events|Skins]          [🔔][⚙] │ [– □ ×]      │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│                 INHALT (658px, je nach Screen)                 │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│ SPIELLEISTE 88px  [Instanzwahl]   [ ▶ SPIELEN ]   [Freunde|Konto]│
└────────────────────────────────────────────────────────────────┘
```

- **CSS-Grid:** `grid-template-rows: 52px 1fr 88px`.
- **Kopfleiste und Spielleiste** sind auf **jedem** Screen identisch. Die Spielleiste ist immer sichtbar.
- **Seitenspalten:** Bibliothek 280 px, Einstellungs-Nav 230 px. Beide auf `--side` mit `border-right: 1px solid #2A231B`.
- **Innenabstände:** Inhalt 20–32 px seitlich. Abstände zwischen Abschnitten 14–24 px, innerhalb von Gruppen 6–12 px.
- **Gaps** in Rastern: 12–14 px.

---

## 9. Komponenten

### 9.1 Kopfleiste
- Hintergrund `#0D0B09`, `border-bottom: 1px solid #2A231B`, Höhe 52 px, links 20 px Innenabstand.
- **Logo:** 18 × 26 px, daneben „Pinecone" in Pixelify 20 px / 600. Danach 30 px Abstand.
- **Haupt-Tabs:** Pixelify Sans 17 px / 600, Padding `0 16px`, volle Leistenhöhe.
  - Inaktiv: `#B5A790`.
  - Aktiv: `#F1E8DA` + 4 px Harz-Strich unten, links/rechts 10 px eingerückt, mit Trapez-Schnitt `clip-path: polygon(4px 0, calc(100% - 4px) 0, 100% 100%, 0 100%)`.
- **Rechts:** Icon-Buttons 40 × 40 (Downloads, Benachrichtigungen mit 7 px Harz-Quadrat als Punkt, Einstellungen). Dann eine 1 px Trennlinie, dann die Fenster-Buttons (je 46 px breit).

### 9.2 Spielleiste (Signatur-Komponente)
- Hintergrund `#0D0B09`, `border-top: 1px solid #2A231B`, Höhe 88 px.
- Grid `1fr auto 1fr`, Gap 20 px, Padding 0 20 px.
- **Links – Instanzwahl:**
  - 300 px breit, `#1C1814`, Zwei-Ecken-Schnitt 8 px.
  - Instanz-Icon 48 × 48, Name 15 px / 700, darunter Version in Mono 12 px `#B5A790`.
  - Chevron nach **oben** (öffnet nach oben).
- **Mitte – SPIELEN:**
  - 290 × 62 px, Hintergrund `#E9A23B`, Text `#1B1206`.
  - Pixelify Sans 29 px / 700, `letter-spacing: 3px`.
  - Gefülltes Play-Dreieck 22 px davor.
  - **Oktagon-Schnitt 12 px** plus Knopf-Kante `inset 0 -5px 0 #B8742A, inset 0 2px 0 #F6C77A`.
- **Rechts:**
  - **Freunde:** Avatar-Stapel aus 28 px Quadraten, −10 px Überlappung, 2 px Rand `#0D0B09`, letztes Feld „+14". Daneben „2 online" / „Freunde".
  - 1 px Trenner.
  - **Konto:** 8 × 8 Pixel-Avatar mit 40 px, Name 14 px / 700, „Microsoft-Konto" `#8F8270`, Chevron hoch.

### 9.3 Buttons

| Typ | Stil |
|---|---|
| **Primär (Harz)** | `#E9A23B`, Text `#1B1206` 700, Knopf-Kante 3 px, optional Zwei-Ecken-Schnitt. Höhe 32–46 px. |
| **Split** | Primär + rechts 34–44 px breiter Teil in `#D48F2C` mit Chevron, Trennlinie `#B8742A`. |
| **Sekundär** | `#241F19`, Text `#F1E8DA` 600–700, kein Rahmen. Optional Kante `inset 0 -3px 0 #0D0B09`. |
| **Link-Button** | Text in Harz 700, mit externem Pfeil-Icon. |
| **Gestrichelt** | transparent, `1px dashed #4A3D2E`, Text `#B5A790` (Import, Hinzufügen). |
| **Icon-Button** | 40–46 px quadratisch, `#241F19` oder transparent, Icon `#D2C4AC` / `#B5A790`. |

### 9.4 Segment-Tabs (Inhaltstypen)
- Container `#1C1814`, Padding 3 px, Gap 2 px.
- Segment 28–30 px hoch, 13 px.
- Aktiv: Harz-Fläche mit `#1B1206`-Text **oder** `#3A2F22` mit hellem Text (innerhalb von Detailseiten).
- Inaktiv: transparent, `#B5A790`.

### 9.5 Unter-Tabs (Instanz-Detail)
- Figtree 14 px, Höhe 44 px.
- Aktiv: 700 `#F1E8DA` + 3 px Harz-Strich unten.
- Zähler in Mono 11 px `#B5A790` (z. B. „Inhalt 42").

### 9.6 Eingabefelder
- Hintergrund `#1C1814`, **kein Rahmen**, Höhe 34–42 px, Padding 0 10–14 px.
- Lupen-Icon `#8F8270`, Platzhalter `#8F8270`, Text `#F1E8DA`.

### 9.7 Filter-Dropdowns
Button `#241F19`, Höhe 40 px. Inhalt: Label in `#8F8270` + Wert in `#F1E8DA` + Chevron, z. B. „Loader Fabric ▾".

### 9.8 Kacheln & Karten
- **Instanz-Kachel („Zuletzt gespielt"):**
  - `#1C1814`, Zwei-Ecken-Schnitt 8 px, Padding 10 px.
  - Icon 44 × 44, Name 14 px / 700, Version Mono 11 px.
  - **Ausgewählt:** `#2A2219` + Harz-Unterkante `inset 0 -3px 0 #E9A23B`.
- **Ergebnis-Karte (Entdecken):**
  - `#1C1814`, Ein-Ecken-Schnitt 10 px, Padding 14 px.
  - Icon 50 px (Initialen in Pixelify, wenn kein Bild), Titel 15 px / 700, „Autor · Quelle" (Quelle in Mono).
  - Beschreibung einzeilig mit Ellipse, Tags, rechts der Button „Holen".
  - Statusvarianten: „Installiert" (grün mit Haken) und Fortschrittsbalken (4 px, Harz auf `#33281F`, Prozent in Mono).
- **Event-Eintrag:**
  - `#1C1814`, Ein-Ecken-Schnitt 8 px, Icon 38 × 38.
  - Name 14 px / 700, Untertitel 12 px `#B5A790`.
  - Optional Badge „NEU".
  - Vergangene Events auf `#17130F` mit gedämpftem Text.

### 9.9 Badges & Chips
- **Event-Badge:**
  - Harz-Fläche, Mono 11 px / 500, `letter-spacing: 1.5px`, GROSS.
  - Ein-Ecken-Schnitt 6–7 px, Padding `6px 12px 6px 14px`.
  - Beispiel: `TWOGG EVENT · TERMIN FOLGT`.
- **Mini-Badge:** „NEU" in Mono 10 px auf Harz, ohne Schnitt.
- **Meta-Chip:** Mono 12 px, `#241F19`, Text `#D2C4AC`, eckig. Grüne Textvariante für „Synchron mit Event-Pack".
- **Tag:** 11 px, `#241F19`, Text `#B5A790`, eckig.

### 9.10 Schalter
- Track 40–44 × 22–24 px, Radius 3 px. Knopf 16 px, Radius 2 px.
- **An:** Track = Akzentfarbe, Knopf `#1B1206` rechts.
- **Aus:** Track `#2E261D` / `#33281F`, Knopf `#8F8270` links.
- Als `<button aria-pressed>` umsetzen.

### 9.11 Inhaltstabelle (Instanz)
- Spalten: `1fr 170px 120px 60px` = Name · Status · Quelle · Aktiv.
- **Kopfzeile:** Mono 11 px `#8F8270`, GROSS, `letter-spacing 1px`, Unterlinie `#2A231B`.
- **Zeilen:** Padding 9 px 14 px, Trenner `#221C16`, Icon 34 px.
- **Status:**
  - „Aktuell" in `#8CCB7E`.
  - „Update verfügbar" in Harz mit Pfeil-hoch-Icon.
- **Quelle:** in Mono. „TwoGG" in Harz, sonst `#B5A790`.
- **Vom Event vorgegebene Mods:** Zeile auf `#1A140D`, Schloss-Icon statt Schalter.

### 9.12 Listen (Seitenspalten)
- Gruppenlabel in Mono 11 px GROSS `#8F8270`.
- Element: Icon 36 px + Name 14 px + Version Mono 11 px, Padding 8 px.
- **Aktiv:** `#2A2219` mit Ein-Ecken-Schnitt 7 px, Name in `#F3BC64`.
- Kein farbiger Seitenstreifen.

### 9.13 Overlays über Bildern
- Halbtransparente Leisten/Tafeln `rgba(13,11,9,0.84–0.88)` mit `1px solid #33281F` (bzw. `border-top #2A231B`).
- Keine Verlaufs-Abdunkelung.

---

## 10. Screens (Referenz)

1. **Spielen (Start):** Vollflächige Bühne `#17120D` mit Wald-Illustration rechts. Die Illustration besteht aus gestuften Tannen-Silhouetten in `#2E2318` (hinten) und `#0E0B08` (vorne), Mond `#F3D9A4`, Hügel `#211910` und Schuppenmuster.
   - Links: Event-Teaser (Badge, Pixel-Titel 66 px, Text, Buttons „Gilde wählen" + Link „Zur Anmeldung auf twogg.app").
   - Rechts oben: Tafel „Event-Kalender" (300 px) mit Events und Discord-Link.
   - Unten: Leiste „Zuletzt gespielt" (138 px) mit 5 Kacheln, die letzte ist der Schnellbeitritt „TwoGG Server" mit gestricheltem Rahmen.
2. **Bibliothek / Instanz:**
   - Links die Bibliotheksliste (Gruppen „TwoGG Events" und „Eigene Instanzen", Button „Modpack importieren").
   - Rechts der Header mit Schuppenmuster: Icon 96 px mit 16 px Schnitt, Pixel-Titel 40 px, Meta-Chips, Aktionen (Ordner, Exportieren, …).
   - Darunter Unter-Tabs, Werkzeugzeile und Inhaltstabelle.
   - **Kein eigener SPIELEN-Button** – das macht die Spielleiste.
3. **Entdecken:**
   - Titel + Segment-Tabs (Modpacks, Mods, Ressourcenpakete, Shader, Datenpakete) + „Installieren in [Instanz]".
   - Darunter Suche + Filter-Dropdowns.
   - Dann der Streifen „TwoGG-Auswahl" (`#1E160D`, Schuppenmuster links, 3 Empfehlungen) und ein Ergebnisraster mit 3 Spalten.
4. **Einstellungen › Darstellung:**
   - Links die Nav (Allgemein, Darstellung, Java & Arbeitsspeicher, Konten, Downloads & Quellen, TwoGG-Verknüpfung).
   - Rechts: Theme-Karten mit Mini-Vorschau (Kopfleiste, Inhalt, Spielleiste mit Akzentbalken), Akzent-Swatches (44 px, Zwei-Ecken-Schnitt, Auswahl = inset-Rand), Live-Vorschau und Schalter.
   - Schalter: „Zapfenschuppen-Muster", „Pixel-Überschriften", „Animationen reduzieren".
5. **Marke:** Logo-Panel, Logo-Varianten, Farbpalette, Schriften, Formen.

**Geplant, noch nicht gestaltet:** Events-Seite (Anmeldung, Gildenwahl, Countdown) und Skins-Seite.

---

## 11. TwoGG-Inhalte (echte Daten, nicht erfinden)

- **Community:** TwoGG, deutsche Gaming-Community.
  - Event-Anmeldung über **twogg.app**.
  - Discord über **twogg.chat**.
- **Events:**
  - **REVIVAL: Avalon:** Mittelalter-RP-Sandbox im Land Avalon. Zwei Gilden: **Die Treue Hand** und **Der Phönixbund**. Termin folgt.
  - **WINTERLAND:** Winter-Event, Termin folgt.
  - **LATEST 2026:** PvP-Event, Thema „Rise Of Atlantis", 03.10., beendet.
  - **TALES:** weitere Event-Reihe.
- **Platzhalter** in eckigen Klammern für Unbekanntes: `[SERVER-ADRESSE]`, `[VERSION]`. Keine Zahlen erfinden.

---

## 12. Do & Don't

**Do**
- Schuppen-Schnitt statt Rundung.
- Harz nur für die wichtigste Aktion je Bereich.
- Pixel-Schrift für Titel, Tabs, SPIELEN.
- Versionen und technische Werte in Mono.
- Spielleiste unten auf jedem Screen.
- Deutsche UI-Texte in Du-Form.

**Don't**
- Kein linker Icon-Rail und kein rechtes Freunde-Panel. Das ist Modrinth.
- Keine runden Karten oder Pill-Buttons.
- Kein Grün als Flächen- oder Aktionsfarbe (Ausnahme: Theme „Kiefer").
- Keine Verläufe, Glows, weichen Drop-Shadows oder Neon.
- Kein weißer Text auf Harz.
- Keine Emojis, kein Inter/Roboto/Arial.
- Keine farbigen Seitenstreifen an Karten.

---

## 13. Barrierefreiheit

- Kontrast mindestens 4.5:1 für Text. Die Text-Tokens oben erfüllen das auf `--bg`, `--surface` und `--bar`.
- Echte `<button>`, `<a>`, `<input>` mit `<label>`.
- Icon-Buttons mit `aria-label`.
- Schalter als `aria-pressed`, Tabs als `role="tablist"` / `role="tab"` / `aria-selected`.
- Dekorative SVGs mit `aria-hidden="true"`.
- Klickflächen mindestens 40 px (SPIELEN 62 px).
- Option „Animationen reduzieren" respektieren.

---

## 14. CSS-Tokens (Kopiervorlage)

```css
:root {
  /* Flächen */
  --bg: #12100D;  --bar: #0D0B09;  --side: #0F0D0B;  --stage: #17120D;
  --surface: #1C1814;  --surface-2: #241F19;  --surface-active: #2A2219;
  --line: #2A231B;  --line-strong: #33281F;  --line-dashed: #4A3D2E;
  /* Text */
  --text: #F1E8DA;  --text-soft: #D2C4AC;  --text-muted: #B5A790;  --text-dim: #8F8270;
  /* Akzent */
  --harz: #E9A23B;  --harz-hover: #F3BC64;  --harz-split: #D48F2C;
  --harz-edge: #B8742A;  --harz-light: #F6C77A;  --ink: #1B1206;
  --nadel: #8CCB7E;
  /* Schrift */
  --font-pixel: 'Pixelify Sans', sans-serif;
  --font-ui: 'Figtree', system-ui, sans-serif;
  --font-mono: 'JetBrains Mono', monospace;
  /* Formen */
  --cut-1: polygon(8px 0, 100% 0, 100% 100%, 0 100%, 0 8px);
  --cut-2: polygon(8px 0, 100% 0, 100% calc(100% - 8px), calc(100% - 8px) 100%, 0 100%, 0 8px);
  --edge-harz: inset 0 -3px 0 #B8742A;
}
```
