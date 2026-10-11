# TwoGG Pinecone

Minecraft-Launcher der TwoGG-Community: Rust-Kern, Desktop-App mit Tauri 2 und Svelte 5.
Windows und Linux, Fabric, Quilt, Forge und NeoForge, Modrinth und CurseForge, TwoGG-Events und -Modpacks.
Architektur und Phasen stehen in [PLAN.md](PLAN.md), Änderungen in [CHANGELOG.md](CHANGELOG.md).

Website mit Download: https://wheatley2056.github.io/twogg-pinecone/ (Quelle in `site/`).

> **Beta.** Pinecone ist in der Beta-Phase. Fehler bitte mit dem Diagnose-ZIP
> (Einstellungen → Allgemein → „Hilfe & Diagnose“) melden.

Der Server hinter `https://pinecone.twogg.app` (Neuigkeiten, TwoGG-Modpacks, Freunde, Admin-Seite)
liegt im eigenen Repo [twogg-pinecone-server](https://github.com/wheatley2056/twogg-pinecone-server).

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

Aufbau der App (nach dem Pinecone-Design-System, `docs/design/`): Kopfleiste mit den Bereichen
**Spielen · Bibliothek · Entdecken · Events · Skins**, rechts Downloads, Benachrichtigungen und
Einstellungen; unten auf jedem Screen die Spielleiste mit Instanzwahl, SPIELEN, Freunden und Konto.

- **Spielen**: Event-Teaser, Event-Kalender, zuletzt gespielte Instanzen und der Community-Server
  mit Live-Status und Schnellbeitritt.
- **Bibliothek**: Instanzen in Gruppen (TwoGG Events, eigene Instanzen, Sync-Packs). Jede Instanz hat
  die Tabs Inhalt, Welten, Server, Screenshots, Logs und Einstellungen. Herkunft und Hash jeder Datei
  stehen in `instances/<id>/pack.lock.json`.
- **Entdecken**: Modpacks, Mods, Ressourcenpakete, Shader und Datenpakete von Modrinth, CurseForge
  und TwoGG, mit der TwoGG-Auswahl oben.
- **Events**: Events von twogg.app mit Anmeldung, Event-Pack und Server-Beitritt.
- **Einstellungen**: Allgemein, Darstellung (Themes Rinde, Kiefer, Winterland, Akzentfarbe),
  Java & Arbeitsspeicher, Konten, Downloads & Quellen, TwoGG-Verknüpfung. Gespeichert wird in
  `<data-dir>/settings.json`, die Spielzeit in `<data-dir>/stats.json`.

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
Loader-Version gewählt. Forge (empfohlene Version laut Forge) und NeoForge kommen über ihre
offiziellen Installer: Pinecone lädt die Installer-JAR nach `versions/<id>/`, entpackt die
mitgelieferten Bibliotheken und führt beim ersten Start die Installer-Prozessoren mit dem Java der
Instanz aus (moderne Versionen ab 1.13; ältere Installer bis 1.12.2 brauchen das nicht). Spontane
Starts ohne Profil nutzen `instances/_<version>/`.

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

„Version wechseln …“ im Menü einer Instanz (bzw. `profile migrate`) zieht eine Instanz auf eine andere
Minecraft-Version oder einen anderen Loader (Fabric, Quilt, Forge, NeoForge, Vanilla) um. Pinecone legt dafür ein
neues Profil an und lässt das bisherige unverändert. Für jeden Inhalt wird die passende Version
gesucht: Modrinth-Inhalte per Hash-Abgleich, CurseForge-Inhalte per Projekt; der Kanal (Release/Beta)
wird nicht instabiler. Bekannte Gegenstücke werden ersetzt (z. B. Quilted Fabric API → Fabric API
beim Wechsel zu Fabric). Mods, die es für das Ziel nicht gibt, kommen deaktiviert mit oder werden
weggelassen; Ressourcenpakete, Shader und Configs werden übernommen. Welten lassen sich mitkopieren,
bei einem Wechsel auf eine ältere Version warnt Pinecone. Gibt es das Modpack des Profils selbst für
das Ziel, wird stattdessen diese Pack-Version angeboten.

## Sync-Packs und Vorlagen

In der Bibliothek unter „Sync-Packs“ legt man eigene Sammlungen an: Mods, Ressourcenpakete und Shader (als
Modrinth-/CurseForge-Projekt ohne feste Version), eigene Dateien wie Configs und Standardwerte für
`options.txt`. Im Tab Inhalt einer Instanz schaltet man sie per Klick an und aus.

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

Der Tab Screenshots einer Instanz zeigt ihre Screenshots, nach Tagen gruppiert. Vorschaubilder
entstehen bei Bedarf in `<data-dir>/cache/thumbs/`. Neue Screenshots erscheinen automatisch
(Datei-Watcher auf den Screenshot-Ordnern). In der Großansicht: blättern mit ←/→, kopieren,
speichern unter, im Ordner zeigen, öffnen, löschen.

## Modpacks teilen

„Teilen …“ im Menü einer Instanz bietet zwei Wege:

- **Link** (nur für Packs von Modrinth oder CurseForge): `pinecone://modrinth/<projekt>/<version>` bzw.
  `pinecone://curseforge/<projekt>/<datei>`. Der Link verweist auf genau die installierte Version bei
  der Originalquelle; eigene Änderungen sind nicht enthalten. Freunde fügen ihn unter
  Bibliothek → „Modpack importieren“ ein, oder klicken ihn an, falls Pinecone installiert ist.
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

## TwoGG: Server, Events und Freunde

Neuigkeiten, TwoGG-Modpacks, die TwoGG-Auswahl, der Community-Server und die Pinecone-Freundesliste
kommen vom [Pinecone-Server](https://github.com/wheatley2056/twogg-pinecone-server)
(`https://pinecone.twogg.app`, abweichend über `PINECONE_SERVER_URL` zur Laufzeit oder beim Bauen).
TwoGG-Packs: `pinecone pack import twogg:<pack>` bzw. `pinecone://twogg/<pack>[/<version>]`; sie
bekommen Updates wie Modrinth-Packs.

- **Freunde**: Anmeldung mit dem Minecraft-Konto über den Mojang-Session-Server, Anfragen per
  Spielername, Status sehen nur Freunde. Offline-Konten nehmen nicht teil.
- **twogg.app**: Events, Anmeldungen und Freunde holt der Launcher direkt von `https://api.twogg.app`
  (Anmeldung mit Discord im Login-Fenster oder per E-Mail; das Refresh-Token liegt im
  Schlüsselbund). CLI: `pinecone events` listet die kommenden Events.

## Logs, Diagnose und Updates

Die App schreibt ihre Logs nach `<data-dir>/logs/` (eine Datei pro Tag, die letzten sieben bleiben).
Unter Einstellungen → Allgemein → „Hilfe & Diagnose“ (oder im Logs-Tab einer Instanz, bzw.
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

## Lizenz

MIT oder Apache-2.0, nach Wahl ([LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)).
Eingebundene Fremdkomponenten (skinview3d, MC Skin Editor, Schriften unter OFL) behalten ihre Lizenzen.
