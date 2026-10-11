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
cargo run -p pinecone-cli -- pack export Fabric fabric.mrpack --version 1.0.0
cargo run -p pinecone-cli -- account login           # Microsoft-Login (siehe unten)
cargo run -p pinecone-cli -- account offline Tobi     # Offline-Account für Tests/LAN
cargo run -p pinecone-cli -- account list             # Accounts, * = aktiv
cargo run -p pinecone-cli -- run 1.21.1               # Version ohne Profil starten
cargo run -p pinecone-cli -- run 1.8.9 --offline Tobi # einmalig offline, ohne zu speichern
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

## CurseForge

Suche und Installation über CurseForge brauchen einen API-Key von https://console.curseforge.com,
gesetzt als `PINECONE_CURSEFORGE_API_KEY` (beim Bauen gesetzt, wird er einkompiliert). Dateien, deren
Autoren Downloads über andere Launcher gesperrt haben, meldet Pinecone mit Link zum manuellen
Herunterladen.

## Entwicklung

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo test -p pinecone-core --test msa -- --ignored keyring   # echter OS-Schlüsselbund
```
