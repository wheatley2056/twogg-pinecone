# Änderungen

## Unveröffentlicht

- **Welt-Backups wiederherstellen.** Im Reiter „Welten“ zeigt „Backups (n)“ die Sicherungen einer
  Welt; „Wiederherstellen“ setzt sie zurück und sichert vorher den aktuellen Stand. Backups
  gelöschter Welten stehen unten und lassen sich ebenfalls zurückholen. Ein beschädigtes Backup
  wird erkannt, bevor die Welt angefasst wird.
- **Automatisch vor jedem Start sichern:** pro Instanz einschaltbar. Gesichert werden nur Welten,
  die sich seit dem letzten Backup geändert haben; Pinecone behält die letzten N automatischen
  Backups (Standard 5), manuelle bleiben immer.
- **Instanz duplizieren:** Rechtsklick auf eine Instanz oder „…“ → „Duplizieren“. Mods, Configs und
  Einstellungen werden ohne neuen Download kopiert, Welten und Screenshots auf Wunsch. Die Kopie
  einer Event-Instanz ist eine normale Instanz ohne Pack-Bindung.
- **Konto pro Instanz:** Unter Einstellungen der Instanz → Start → Konto startet eine Instanz immer
  mit einem bestimmten Konto. Die Spielleiste zeigt dann „als <Name>“. Ist das Konto entfernt,
  startet Pinecone nicht still mit einem anderen, sondern sagt Bescheid.
- **Beim Spielstart:** Einstellungen → Allgemein: Launcher geöffnet lassen, minimieren, ausblenden
  oder schließen. Ausgeblendet kommt er nach Spielende zurück, minimiert nach Wahl, nach einem
  Absturz immer. „Schließen“ blendet nur aus, solange noch Downloads laufen.
- **Start-Befehle und Wrapper:** Einstellungen der Instanz → „Start-Befehle und Wrapper“: Befehl vor
  dem Start, Wrapper um Java (z. B. `gamemoderun`, `prime-run`) und Befehl nach dem Ende, mit
  Platzhaltern wie `$INST_DIR`. Sie laufen ohne Shell; scheitert der Befehl vor dem Start, startet
  das Spiel nicht und die Ausgabe steht im Log.
- **Gruppen in der Bibliothek:** Eigene Gruppen wie „Modded“ oder „Vanilla“ anlegen, Instanzen mit
  der Maus hineinziehen oder per Rechtsklick → „In Gruppe verschieben …“. Gruppen lassen sich
  einklappen, umbenennen und auflösen; Event-Instanzen stehen automatisch unter „TwoGG Events“.
  Sortierung nach zuletzt gespielt, Name oder Spielzeit. Alles bleibt nach einem Neustart erhalten.
- **Komplette Instanz exportieren:** „Exportieren“ → „Komplett (Umzug)“ packt Mods, Configs und
  Einstellungen und nach Wahl Welten, Screenshots, Ressourcen- und Shaderpakete in eine
  `.pinecone.zip`. Auf dem anderen Rechner öffnet „Modpack importieren“ sie als neue Instanz.
  Konten und Start-Befehle sind nie enthalten.

## 0.9.7

- **Entfernen repariert.** Konto entfernen (Einstellungen → Konten und Kontomenü), Inhalt aus einer
  Instanz entfernen und Welt löschen brachen mit „Cannot read properties of null (reading 'uuid')“
  ab und taten nichts. Sie funktionieren wieder.

## 0.9.6

- **CurseForge über den Pinecone-Server.** Der Launcher enthält keinen CurseForge-Schlüssel mehr,
  Suche und Installation laufen über den Pinecone-Server. Ist dort CurseForge nicht eingerichtet,
  blendet Pinecone die Quelle aus.
- **Über Pinecone:** Unter Einstellungen → Allgemein stehen Version, Lizenz, Impressum,
  Datenschutz, Kontakt und der Hinweis, dass Pinecone kein offizielles Minecraft-Produkt ist.
- **Freunde-Daten löschen:** Unter Einstellungen → Konten entfernt „Meine Freunde-Daten löschen“
  Profil, Freunde und Anfragen vom Pinecone-Server.

## 0.9.5

- **Sync-Packs: Steuerung und Optionen.** Tastenbelegungen und Spieleinstellungen lassen sich aus
  einer Instanz übernehmen („Aus Instanz übernehmen …“) und in andere Instanzen synchronisieren;
  Tasten werden per Klick und Tastendruck gesetzt. Im Sync-Pack selbst schaltet man es jetzt für
  jede Instanz direkt ein oder aus, Vorlagen spielt man dort ein.
- **Sync-Packs: Abgleich repariert.** Optionen und eigene Dateien (z. B. Configs) kamen in schon
  gespielten Instanzen nicht an, weil Minecraft und Mods ihre Standardwerte bereits geschrieben
  hatten; beim ersten Einspielen werden sie jetzt gesetzt. Eine von Pinecone neu angelegte
  `options.txt` bekommt eine Versionszeile, sonst verwarf Minecraft wegen der Tastennamen alle
  Einstellungen. Ein fehlerhaftes Sync-Pack hält beim Start die anderen nicht mehr auf, und nach dem
  Ein- und Ausschalten zeigt die Instanz, was installiert, entfernt oder nicht verfügbar ist.

## 0.9.4

- **JVM-Optimierungen:** Fertige Flag-Sätze für Java unter Einstellungen → Java & Arbeitsspeicher:
  Aikar's Flags, MeowIce's Flags (ab Java 17) und Generational ZGC (ab Java 21). Einer gilt als
  Standard für alle Instanzen, jede Instanz kann im Instanz-Dialog einen anderen oder keinen wählen.
  Die Sätze lassen sich anpassen und zurücksetzen, eigene Presets kommen dazu. Ist das Java einer
  Instanz zu alt für einen Satz, startet das Spiel ohne ihn; eigene JVM-Argumente der Instanz
  gelten zusätzlich und haben Vorrang.
- **TwoGG-Modpack-Seiten:** Jedes TwoGG-Pack hat eine eigene Seite auf dem Pinecone-Server
  (`pinecone.twogg.app/packs/<id>`), aufgebaut wie bei CurseForge mit Beschreibung, Versionen und
  „In Pinecone installieren“. In „Entdecken“ und auf der Event-Seite öffnet der Pack-Name sie.
- **Pack-Logos in der Bibliothek:** Beim Installieren eines Modpacks (TwoGG, Modrinth, CurseForge)
  wird sein Logo zum Instanz-Symbol. Ältere Pack-Instanzen ohne Symbol bekommen es beim nächsten
  Start; im Instanz-Dialog lässt es sich gegen ein Motiv tauschen.
- **Installationen unter „Downloads“:** Modpacks und Inhalte aus „Entdecken“, Event-Packs,
  geöffnete Pack-Links und Migrationen erscheinen beim Installieren im Downloads-Panel.
- **Skin-Vorschau mit eigenen Animationen:** Ruhe, Gehen, Laufen, Winken, Jubeln und Tanzen mit
  weichen Übergängen; der Kopf folgt dem Mauszeiger, Drehen lässt sich zuschalten. Mit
  „Animationen reduzieren“ bewegt sich die Figur nur sanft.

## 0.9.3

- **Forge und NeoForge:** Instanzen mit Forge (ab 1.13 und ältere bis 1.12.2) und NeoForge,
  installiert über die offiziellen Installer. Modpacks mit Forge/NeoForge von Modrinth und
  CurseForge lassen sich importieren, die Suche filtert passend.

## 0.9.2

- **Update-Kanal** wählbar (Einstellungen → Allgemein): „Release“ bekommt nur stabile Versionen,
  „Beta“ auch Vorabversionen (auf GitHub als Pre-release veröffentlicht). Pinecone sucht dafür in
  den GitHub-Releases die neueste passende Version.
- Verständliche Meldung statt „Could not fetch a valid release JSON“, wenn es im Kanal noch keine
  veröffentlichte Version gibt.

## 0.9.1

- **Startanimation:** Beim Öffnen erscheint das Pinecone-Logo als Splash-Screen, während der
  Launcher im Hintergrund lädt. Abschaltbar unter Einstellungen → Allgemein; mit „Animationen
  reduzieren“ nur als Standbild.
- **Startseite neu:** aktuelles TwoGG-Event mit Banner von twogg.app, „Zuletzt gespielt“ mit
  Spielen-Knopf und letzter Spielzeit, Neuigkeiten (zum Lesen anklicken), Community-Server mit
  Spielerzahl und Beitreten, Freunde online.
- **Inhalte hinzufügen** direkt in der Instanz: suchen, mit einem Klick hinzufügen, Dateien wählen
  oder `.jar`/`.zip` ins Fenster ziehen (Mod, Ressourcenpaket oder Shader wird erkannt).
- **Rechtsklick-Menü** für Instanzen, Inhalte, Welten, Screenshots, Events und Textfelder.
- **Skin-Editor** im Pinecone-Design (Farben, Schriften, Ecken, Linien-Icons).
- Bilder von twogg.app (Event-Banner, Logos, Profilbilder) werden angezeigt, auch von Discord.
- Einstellungen-Symbol ist jetzt ein Zahnrad.
- Behoben: Neue Instanz mit Motiv schlug mit „Symbol ist zu lang“ fehl.
- „Gilde wählen“ entfällt; die Anmeldung läuft über den Link zu twogg.app.

## 0.9.0 – erste Beta

Erste öffentliche Beta von TwoGG Pinecone für Windows und Linux.

**Spielen und Instanzen**
- Vanilla, Fabric und Quilt, automatisches Java (passende Mojang-Runtime), Microsoft-Login und
  Offline-Konten für Tests.
- Instanzen mit eigenem Spielordner, Inhalten (Mods, Ressourcenpakete, Shader), Welten (Sichern,
  Löschen), Servern mit Schnellbeitritt, Screenshots, Logs und Einstellungen.
- Version wechseln: Instanz auf eine andere Minecraft-Version oder einen anderen Loader umziehen,
  Mods werden passend gesucht.
- Sync-Packs und Vorlagen für eigene Sammlungen über mehrere Instanzen.

**Inhalte**
- Entdecken: Modpacks, Mods, Ressourcenpakete, Shader und Datenpakete von Modrinth, CurseForge und
  TwoGG, mit der TwoGG-Auswahl.
- Modpack-Import (`.mrpack`, CurseForge-ZIP, `pinecone://`-Links), Updates mit Snapshot und Erhalt
  eigener Änderungen, Export und Teilen.

**TwoGG**
- Events von twogg.app mit Anmeldung (Discord oder E-Mail), Event-Packs und Server-Beitritt.
- TwoGG-Modpacks, Neuigkeiten und Community-Server vom Pinecone-Server.
- Pinecone-Freundesliste mit dem Minecraft-Konto: Anfragen per Spielername, sehen, wer online ist
  und was gespielt wird.

**Oberfläche**
- Neues Design nach dem Pinecone-Design-System: Themes Rinde, Kiefer und Winterland, Akzentfarben,
  Pixel-Überschriften, eigener Fensterrahmen (unter Linux abschaltbar), Deutsch und Englisch.
- Skins: Bibliothek, 3D-Vorschau, Editor, Skin und Cape des Kontos setzen.

**Sonstiges**
- Signierte Selbst-Updates, Diagnose-ZIP für Fehlermeldungen, Logs in `<data-dir>/logs/`.

**Bekannt**
- Die Gildenwahl läuft vorerst auf twogg.app.
