# Änderungen

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
- Forge und NeoForge werden noch nicht unterstützt.
- Die Gildenwahl läuft vorerst auf twogg.app.
