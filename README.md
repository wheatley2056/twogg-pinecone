# TwoGG Pinecone

Minecraft-Launcher der TwoGG-Community, geschrieben in Rust (später mit Tauri-GUI).
Architektur und Phasen stehen in [PLAN.md](PLAN.md).

## Aufbau

```
crates/
  core/     # pinecone-core: Manifeste, Downloads, Java, Loader, Profile, Accounts, Launch
  cli/      # pinecone: schlanke CLI über core für Entwicklung, Tests und Debugging
src-tauri/  # Desktop-App (Tauri 2): Befehle und Spielstart über core
ui/         # Oberfläche (Svelte 5 + TypeScript, Vite)
assets/     # Quell-Grafiken (Pixel-Art-Tannenzapfen fürs App-Icon)
```

## Desktop-App

Voraussetzungen: Rust, Node.js mit npm und unter Linux WebKitGTK 4.1
(`webkit2gtk-4.1`, unter Arch/CachyOS `pacman -S webkit2gtk-4.1`).

```sh
npm install
npm run tauri dev       # App mit Hot-Reload starten
npm run tauri build     # Release-Build mit Installern
npm run dev             # nur die Oberfläche im Browser, mit Beispieldaten (Mock-Modus)
npm run check           # Svelte/TypeScript prüfen
```

Aufbau der App (angelehnt an Modrinth App, CurseForge und NoRisk): links eine Navigationsleiste
mit Start, Bibliothek, Entdecken und Einstellungen. Die Bibliothek zeigt die Profile als Kacheln,
die Profilseite hat einen großen Spielen-Button und die Tabs „Inhalte“ (Mods, Ressourcenpakete,
Shader: aktivieren/deaktivieren, aktualisieren, entfernen, eigene Dateien) und „Spielausgabe“.
„Entdecken“ durchsucht Modrinth; aus einem Profil heraus sind Version und Loader vorbelegt.
Herkunft und Hash jeder Datei stehen in `instances/<id>/pack.lock.json`.

Die Startseite zeigt eine Übersicht (zuletzt gespielt, Server mit Live-Status und Quick-Join,
neueste Screenshots, Spielzeit). Unter Einstellungen lassen sich Sprache (de/en), Hell/Dunkel/System,
Akzentfarbe, Hintergrundbild und die Anordnung der Übersicht ändern; das Theme ist als Datei
exportier- und importierbar. Gespeichert wird in `<data-dir>/settings.json`, die Spielzeit in
`<data-dir>/stats.json`.

Das Spiel schreibt seine Konsolenausgabe nach `<profil>/.minecraft/logs/pinecone-console.log`;
die App liest sie live mit. Wird der Launcher geschlossen, läuft das Spiel weiter.
Unter Linux setzt die App `WEBKIT_DISABLE_DMABUF_RENDERER=1`, weil WebKitGTK sonst mit
NVIDIA-Treibern unter Wayland abstürzt (eigener Wert der Variable hat Vorrang).

## CLI

```sh
cargo run -p pinecone-cli -- versions                 # Releases anzeigen (--all für Snapshots)
cargo run -p pinecone-cli -- install latest           # Version, Libraries, Assets laden
cargo run -p pinecone-cli -- profile create "Fabric" --version latest --loader fabric --memory 6144
cargo run -p pinecone-cli -- profile list             # Profile (Instanzen)
cargo run -p pinecone-cli -- profile edit Fabric --java auto --jvm-args "-XX:+UseZGC"
cargo run -p pinecone-cli -- run Fabric               # Profil starten
cargo run -p pinecone-cli -- loader versions quilt 1.21.1
cargo run -p pinecone-cli -- run Fabric --server mc.twogg.de   # Quick-Join
cargo run -p pinecone-cli -- server ping mc.twogg.de  # MOTD, Spieler, Ping (inkl. SRV-Eintrag)
cargo run -p pinecone-cli -- server list Fabric       # Server aus servers.dat
cargo run -p pinecone-cli -- stats                    # Spielzeit je Profil
cargo run -p pinecone-cli -- content search sodium --profile Fabric
cargo run -p pinecone-cli -- content install Fabric iris            # inkl. Abhängigkeiten (Sodium)
cargo run -p pinecone-cli -- content install Fabric complementary-reimagined --type shader
cargo run -p pinecone-cli -- content list Fabric       # erkennt auch von Hand abgelegte Dateien
cargo run -p pinecone-cli -- content updates Fabric --apply
cargo run -p pinecone-cli -- pack import modrinth:fabulously-optimized   # Modpack als neues Profil
cargo run -p pinecone-cli -- pack import ~/Downloads/pack.mrpack         # oder CurseForge-ZIP
cargo run -p pinecone-cli -- pack update "Fabulously Optimized"          # Plan anzeigen
cargo run -p pinecone-cli -- pack update "Fabulously Optimized" --apply  # einspielen (mit Snapshot)
cargo run -p pinecone-cli -- pack rollback "Fabulously Optimized" --to <snapshot>
cargo run -p pinecone-cli -- pack export Fabric fabric.mrpack --version 1.0.0 --with-options
cargo run -p pinecone-cli -- pack share "Fabulously Optimized"    # pinecone://modrinth/<projekt>/<version>
cargo run -p pinecone-cli -- pack preview pinecone://modrinth/fabulously-optimized   # oder Datei
cargo run -p pinecone-cli -- pack import pinecone://modrinth/1KVo5zza/ssWn7YI0
cargo run -p pinecone-cli -- account login           # Microsoft-Login (siehe unten)
cargo run -p pinecone-cli -- account offline Tobi     # Offline-Account für Tests/LAN
cargo run -p pinecone-cli -- account list             # Accounts, * = aktiv
cargo run -p pinecone-cli -- run 1.21.1               # Version ohne Profil starten
cargo run -p pinecone-cli -- run 1.8.9 --offline Tobi # einmalig offline, ohne zu speichern
cargo run -p pinecone-cli -- profile migrate Fabric --minecraft 1.21.1            # Plan anzeigen
cargo run -p pinecone-cli -- profile migrate Fabric --minecraft 1.21.1 --loader quilt --apply
cargo run -p pinecone-cli -- java list                # gefundene Java-Installationen
cargo run -p pinecone-cli -- java install jre-legacy  # Mojang-Runtime laden (hier Java 8)
```

Profile liegen unter `<data-dir>/instances/<id>/` mit `profile.json` (Version, Loader, RAM,
JVM-Argumente, Java) und eigenem `.minecraft`-Ordner für Welten, Mods und Einstellungen.
Fabric und Quilt werden über ihre Meta-APIs installiert; ohne Angabe wird die neueste stabile
Loader-Version gewählt. Spontane Starts ohne Profil nutzen `instances/_<version>/`.

Java wird beim Start automatisch gewählt: zuerst eine installierte Java-Version mit genau der
Major-Version aus dem Version-JSON (von Pinecone verwaltete Runtimes bevorzugt), sonst wird Mojangs
passende Runtime nach `<data-dir>/runtimes/` geladen.

Nützliche Optionen:

- `--data-dir <pfad>` bzw. `PINECONE_HOME`: Datenverzeichnis (Standard: `~/.local/share/twogg-pinecone`,
  unter Windows `%APPDATA%\TwoGG\Pinecone`)
- `--verify`: vorhandene Dateien per SHA-1 prüfen statt nur per Größe
- `run --print-command`: nur den Java-Startbefehl ausgeben
- `run --no-update`: ohne Netzwerk starten
- `run --java <pfad>`: bestimmtes Java-Programm statt automatischer Auswahl

## Microsoft-Login

Pinecone nutzt den Device-Code-Flow (Code im Browser bestätigen) und speichert nur den
Refresh-Token, und zwar im Schlüsselbund des Systems (Windows Credential Manager, macOS
Keychain, unter Linux Secret Service/KWallet/GNOME Keyring). `accounts.json` enthält keine Tokens.

Dafür braucht Pinecone eine eigene Azure-App:

1. Im [Azure-Portal](https://portal.azure.com) unter *Microsoft Entra ID → App-Registrierungen*
   eine neue App anlegen, Kontotyp **„Nur persönliche Microsoft-Konten“**.
2. Unter *Authentifizierung* **„Öffentliche Clientflows zulassen“** aktivieren (nötig für den Device-Code-Flow).
3. Die Anwendungs-ID (Client-ID) als `PINECONE_MSA_CLIENT_ID` setzen oder per `--client-id` übergeben.
   Beim Bauen gesetzt, wird sie fest einkompiliert.
4. Neue Apps müssen von Mojang für die Minecraft-API freigeschaltet werden. Bis dahin antwortet
   `login_with_xbox` mit 403 (Pinecone meldet das so); Infos unter https://aka.ms/AppRegInfo.
   Bis zur Freischaltung mit Offline-Accounts testen.

## Modpacks

Modrinth-Packs (`.mrpack`) und CurseForge-Packs (ZIP mit `manifest.json`) werden als neues Profil
installiert. Updates bleiben bei der Minecraft-Version und dem Loader des Profils und vergleichen
drei Stände per Hash: was das Pack zuletzt geliefert hat, was jetzt im Spielordner liegt und was die
neue Version liefert. Unveränderte Pack-Dateien werden ersetzt, eigene Änderungen und eigene Mods
bleiben, vom Pack entfernte Dateien werden gelöscht; bei echten Konflikten wählt man selbst
(Vorschlag: Configs und `options.txt` behalten, Mods aus dem Pack nehmen). Vorher wird ein Snapshot
unter `instances/<id>/snapshots/` angelegt (die letzten drei bleiben), aus dem man zurückrollen kann.
`.mrpack`-Downloads sind auf die von Modrinth erlaubten Hosts beschränkt.

## Version wechseln

„Version wechseln …“ im Profilmenü (bzw. `profile migrate`) zieht ein Profil auf eine andere
Minecraft-Version oder einen anderen Loader (Fabric, Quilt, Vanilla) um. Pinecone legt dafür ein
neues Profil an und lässt das bisherige unverändert. Für jeden Inhalt wird die passende Version
gesucht: Modrinth-Inhalte per Hash-Abgleich, CurseForge-Inhalte per Projekt; der Kanal (Release/Beta)
wird nicht instabiler. Bekannte Gegenstücke werden ersetzt (z. B. Quilted Fabric API → Fabric API
beim Wechsel zu Fabric). Mods, die es für das Ziel nicht gibt, kommen deaktiviert mit oder werden
weggelassen; Ressourcenpakete, Shader und Configs werden übernommen. Welten lassen sich mitkopieren,
bei einem Wechsel auf eine ältere Version warnt Pinecone. Gibt es das Modpack des Profils selbst für
das Ziel, wird stattdessen diese Pack-Version angeboten.

## Sync-Packs und Vorlagen

Unter „Sync-Packs“ legt man eigene Sammlungen an: Mods, Ressourcenpakete und Shader (als
Modrinth-/CurseForge-Projekt ohne feste Version), eigene Dateien wie Configs und Standardwerte für
`options.txt`. Im Inhalte-Tab eines Profils schaltet man sie per Klick an und aus.

- **Sync-Pack**: bleibt verbunden und wird vor jedem Start abgeglichen. Für jedes Profil wird die
  zur Minecraft-Version und zum Loader passende Datei installiert (samt Abhängigkeiten). Ausschalten
  entfernt genau die Inhalte, die das Sync-Pack gebracht hat (im `pack.lock.json` mit `sync` markiert).
- **Vorlage**: wird einmal eingespielt und ist danach normaler Profil-Inhalt.
- Bringt das Modpack oder der Nutzer ein Projekt schon mit, gewinnt deren Fassung. Eigene Dateien und
  Optionen werden nur gesetzt, solange man sie im Profil nicht selbst geändert hat.
- Optional automatisch bei neuen Profilen und Modpack-Imports, auf Wunsch nur für bestimmte Loader.
- Gespeichert unter `<data-dir>/syncpacks/<id>/`, der Zustand je Profil in `instances/<id>/syncpacks.json`.

```sh
cargo run -p pinecone-cli -- sync create Basics
cargo run -p pinecone-cli -- sync add basics zoomify
cargo run -p pinecone-cli -- sync add basics fresh-animations --type resourcepack
cargo run -p pinecone-cli -- sync option basics fov 0.25
cargo run -p pinecone-cli -- sync enable Fabric basics      # bzw. disable, run, list --profile Fabric
```

## Skins

Unter „Skins“ liegt eine eigene Skin-Bibliothek (`<data-dir>/skins/`): Skins per Spielername von
Mojang übernehmen, PNG importieren, umbenennen, Modell Classic/Slim wählen, als PNG speichern. Die
3D-Vorschau (mit aktivem Cape) nutzt [skinview3d](https://github.com/bs-community/skinview3d) (MIT).
Mit Microsoft-Account lässt sich ein Skin als Account-Skin setzen, das Cape wechseln oder ausblenden
und der aktuelle Skin in die Bibliothek übernehmen. Minecraft akzeptiert nur 64×64 und 64×32.

Der Skin-Editor ist der [MC Skin Editor](https://github.com/eheikes/mc-skin-editor) von Eric Heikes
(Apache-2.0), unverändert eingebunden unter `ui/vendor/mc-skin-editor` und als eigene Seite im iframe
geladen; Details und Aktualisieren siehe `ui/vendor/mc-skin-editor/VENDORED.md`.

```sh
cargo run -p pinecone-cli -- skin import jeb_          # oder eine PNG-Datei (--slim)
cargo run -p pinecone-cli -- skin list
cargo run -p pinecone-cli -- skin upload <id>          # als Skin des aktiven Microsoft-Accounts
cargo run -p pinecone-cli -- skin cape                 # Capes anzeigen; mit <id> bzw. none wechseln
```

## Screenshots

„Screenshots“ zeigt alle Screenshots aller Profile, nach Tagen gruppiert und nach Profil filterbar.
Vorschaubilder entstehen bei Bedarf in `<data-dir>/cache/thumbs/`. Neue Screenshots erscheinen
automatisch (Datei-Watcher auf den Screenshot-Ordnern). In der Großansicht: blättern mit ←/→,
kopieren, speichern unter, im Ordner zeigen, öffnen, löschen.

## Modpacks teilen

„Teilen …“ im Profilmenü bietet zwei Wege:

- **Link** (nur für Packs von Modrinth oder CurseForge): `pinecone://modrinth/<projekt>/<version>` bzw.
  `pinecone://curseforge/<projekt>/<datei>`. Der Link verweist auf genau die installierte Version bei
  der Originalquelle; eigene Änderungen sind nicht enthalten. Freunde fügen ihn unter
  Bibliothek → „Link öffnen“ ein, oder klicken ihn an, falls Pinecone installiert ist.
  Modrinth-Adressen (`https://modrinth.com/modpack/<slug>`) funktionieren dort auch.
- **Datei** (`.mrpack`): für eigene Zusammenstellungen. Aktive Mods sind immer dabei,
  Ressourcenpakete, Shader, `config/` und `options.txt` sind wählbar. Welten, `servers.dat`,
  Screenshots, Logs und Accounts werden nie exportiert. Mods von Modrinth werden nur verlinkt.

Vor jeder Installation aus Datei oder Link zeigt Pinecone eine Vorschau (Version, Loader, Anzahl der
Inhalte, Download-Größe) und warnt bei Dateien von Dritten sowie bei Download-Quellen, die Pinecone
nicht lädt. Die App registriert sich für `pinecone://`-Links und `.mrpack`-Dateien; läuft sie schon,
übernimmt das offene Fenster den Link. `pinecone://pack/<code>` ist für einen späteren Share-Dienst
reserviert.

## CurseForge

Suche und Installation über CurseForge brauchen einen API-Key von https://console.curseforge.com,
gesetzt als `PINECONE_CURSEFORGE_API_KEY` (beim Bauen gesetzt, wird er einkompiliert). Dateien, deren
Autoren Downloads über andere Launcher gesperrt haben, meldet Pinecone mit Link zum manuellen
Herunterladen.

## TwoGG-Server (Neuigkeiten und TwoGG-Modpacks)

`crates/server` ist der Server hinter `https://pinecone.twogg.app`: Er liefert dem Launcher
Neuigkeiten (Startseite, Widget „Neuigkeiten“) und TwoGG-Modpacks (Entdecken → TwoGG). Die
Modpack-Dateien gibt es nur über kurzlebige, signierte Links, die der Launcher anfordert. Verwaltet
wird alles unter `/admin` (React-Admin, Quellcode in `admin/`): Modpacks anlegen, Versionen als
`.mrpack` hochladen (Minecraft-Version und Loader liest der Server aus der Datei), Neuigkeiten mit
Markdown und Bild, geplante Veröffentlichung.

```sh
cd admin && npm ci && npm run build && cd ..
PINECONE_ADMIN_PASSWORD=ein-langes-passwort PINECONE_ADMIN_DIR=admin/dist cargo run -p pinecone-server
# → http://localhost:8080/admin ; der Launcher nutzt ihn mit PINECONE_SERVER_URL=http://localhost:8080
```

Launcher-Seite: `pinecone pack import twogg:<pack>` bzw. `pinecone://twogg/<pack>[/<version>]`;
TwoGG-Packs bekommen Updates wie Modrinth-Packs. Die Server-Adresse ist `https://pinecone.twogg.app`,
abweichend über `PINECONE_SERVER_URL` (Laufzeit oder beim Bauen). Deploy auf core01 (Docker-Stack,
Nginx Proxy Manager, Backup): [deploy/pinecone/README.md](deploy/pinecone/README.md).

Weitere Bereiche des Servers:

- **Pinecone-Freunde** (`/api/v1/auth/challenge`, `/auth/verify`, `/me`, `/presence`, `/friends`,
  `/friends/requests`): Anmeldung mit dem Minecraft-Konto über den Mojang-Session-Server
  (`session/minecraft/join` im Launcher, `hasJoined` im Server, abweichend über
  `PINECONE_SESSION_SERVER`). Sitzungen gelten 30 Tage, Status sehen nur Freunde, „online“ heißt
  Präsenz in den letzten 90 Sekunden. Offline-Konten nehmen nicht teil.
- **Community-Server** (`/admin` → Einstellungen): Name und Adresse des TwoGG-Servers für die
  Spielen-Seite, ausgeliefert in `/api/v1/info`.
- **TwoGG-Auswahl** (`/api/v1/recommendations?type=modpack|mod|resourcepack|shader|datapack`):
  kuratierte Projekte (TwoGG-Pack, Modrinth oder CurseForge) für den Streifen in „Entdecken“.
- **Event-Packs**: Ein Modpack kann einer twogg.app-Event-ID zugeordnet werden; die Events-Seite
  bietet es dann zum Installieren an.

Events, Anmeldungen und Freunde von twogg.app holt der Launcher direkt von `https://api.twogg.app`
(Anmeldung mit Discord im Login-Fenster oder E-Mail; das Refresh-Token liegt im Schlüsselbund).
CLI: `pinecone events` listet die kommenden Events.

## Logs, Diagnose und Updates

Die App schreibt ihre Logs nach `<data-dir>/logs/` (eine Datei pro Tag, die letzten sieben bleiben).
Unter Einstellungen → „Hilfe & Diagnose“ (oder im Spielausgabe-Tab eines Profils, bzw.
`pinecone diag <datei.zip>`) entsteht ein ZIP mit Systeminfos, Launcher- und Spiel-Logs, den neuesten
Crash-Reports, der Mod-Liste und der Profil-Konfiguration; Account-Daten fehlen, Tokens werden geschwärzt.

Release-Builds prüfen beim Start auf Updates (signiert, Tauri-Updater) und installieren sie auf Wunsch.
Installer gibt es für Windows (NSIS, MSI) und Linux (AppImage, deb, rpm); wie ein Release entsteht und
welche Schlüssel nötig sind, steht in [RELEASING.md](RELEASING.md).

## Entwicklung

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo test -p pinecone-core --test msa -- --ignored keyring   # echter OS-Schlüsselbund
```
