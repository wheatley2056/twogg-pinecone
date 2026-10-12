//! Welten einer Instanz (`saves/`): Liste mit Name, Spielmodus und letzter Änderung aus
//! `level.dat`, Sicherung als ZIP, Wiederherstellen und Löschen.
//!
//! Backups heißen `<ordner>-<millisekunden>.zip`, automatische (vor dem Start)
//! `<ordner>-<millisekunden>-auto.zip`. Nur automatische werden rotiert.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::paths::safe_relative;
use crate::{Error, Result, nbt};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    /// Ordnername in `saves/` (auch die ID für Aktionen).
    pub folder: String,
    /// Anzeigename aus `level.dat`, sonst der Ordnername.
    pub name: String,
    /// `survival`, `creative`, `adventure`, `spectator`.
    pub game_mode: Option<String>,
    pub hardcore: bool,
    /// Unix-Zeit in Millisekunden.
    pub last_played: Option<i64>,
    /// Minecraft-Version, mit der die Welt zuletzt gespeichert wurde.
    pub version: Option<String>,
    pub size: u64,
    /// `icon.png` der Welt, falls vorhanden.
    pub icon: Option<PathBuf>,
}

/// Nur einfache Ordnernamen (kein Pfad, kein `..`).
pub fn valid_folder(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\'])
        && name.len() <= 255
}

fn dir_size(dir: &Path, budget: &mut u32) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut total = 0;
    for e in entries.flatten() {
        // Riesige Welten nicht endlos durchzählen.
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        match e.file_type() {
            Ok(t) if t.is_dir() => total += dir_size(&e.path(), budget),
            Ok(t) if t.is_file() => total += e.metadata().map(|m| m.len()).unwrap_or(0),
            _ => {}
        }
    }
    total
}

fn read_level(path: &Path) -> Option<nbt::Tag> {
    let file = std::fs::File::open(path).ok()?;
    let mut data = Vec::new();
    flate2::read::GzDecoder::new(file)
        .take(32 * 1024 * 1024)
        .read_to_end(&mut data)
        .ok()?;
    nbt::read(&data).ok()
}

fn game_mode(id: i64) -> Option<String> {
    Some(
        match id {
            0 => "survival",
            1 => "creative",
            2 => "adventure",
            3 => "spectator",
            _ => return None,
        }
        .into(),
    )
}

pub fn list(game_dir: &Path) -> Result<Vec<World>> {
    let saves = game_dir.join("saves");
    let entries = match std::fs::read_dir(&saves) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(&saves)(e)),
    };
    let mut worlds = Vec::new();
    for e in entries.flatten() {
        let path = e.path();
        let folder = e.file_name().to_string_lossy().into_owned();
        if !path.is_dir() || !path.join("level.dat").is_file() || !valid_folder(&folder) {
            continue;
        }
        let level = read_level(&path.join("level.dat"));
        let data = level.as_ref().and_then(|l| l.get("Data"));
        let field = |k: &str| data.and_then(|d| d.get(k));
        let mut budget = 200_000;
        worlds.push(World {
            name: field("LevelName")
                .and_then(nbt::Tag::as_str)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(&folder)
                .to_string(),
            game_mode: field("GameType")
                .and_then(nbt::Tag::as_i64)
                .and_then(game_mode),
            hardcore: field("hardcore").and_then(nbt::Tag::as_i64) == Some(1),
            last_played: field("LastPlayed").and_then(nbt::Tag::as_i64),
            version: field("Version")
                .and_then(|v| v.get("Name"))
                .and_then(nbt::Tag::as_str)
                .map(str::to_string),
            size: dir_size(&path, &mut budget),
            icon: Some(path.join("icon.png")).filter(|p| p.is_file()),
            folder,
        });
    }
    worlds.sort_by(|a, b| b.last_played.cmp(&a.last_played).then(a.name.cmp(&b.name)));
    Ok(worlds)
}

fn world_dir(game_dir: &Path, folder: &str) -> Result<PathBuf> {
    if !valid_folder(folder) {
        return Err(Error::Invalid(format!("Ungültige Welt {folder:?}")));
    }
    let dir = game_dir.join("saves").join(folder);
    if !dir.join("level.dat").is_file() {
        return Err(Error::Invalid(format!("Welt {folder:?} nicht gefunden")));
    }
    Ok(dir)
}

/// Packt die Welt als ZIP nach `target_dir/<ordner>-<zeit>.zip` und gibt den Pfad zurück.
pub fn backup(game_dir: &Path, folder: &str, target_dir: &Path) -> Result<PathBuf> {
    write_backup(game_dir, folder, target_dir, false)
}

/// Wie [`backup`], aber als automatisches Backup markiert (wird von [`prune_auto`] rotiert).
pub fn backup_auto(game_dir: &Path, folder: &str, target_dir: &Path) -> Result<PathBuf> {
    write_backup(game_dir, folder, target_dir, true)
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn write_backup(game_dir: &Path, folder: &str, target_dir: &Path, auto: bool) -> Result<PathBuf> {
    let dir = world_dir(game_dir, folder)?;
    std::fs::create_dir_all(target_dir).map_err(Error::io(target_dir))?;
    let suffix = if auto { "-auto" } else { "" };
    let mut stamp = now_millis();
    let mut target = target_dir.join(format!("{folder}-{stamp}{suffix}.zip"));
    while target.exists() {
        stamp += 1;
        target = target_dir.join(format!("{folder}-{stamp}{suffix}.zip"));
    }
    // Erst unter Zwischennamen schreiben: Ein abgebrochenes Backup taucht nie in der Liste auf.
    let partial = target.with_extension("zip.part");
    let result = zip_world(&dir, folder, &partial)
        .and_then(|()| std::fs::rename(&partial, &target).map_err(Error::io(&target)));
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result.map(|()| target)
}

fn zip_world(dir: &Path, folder: &str, target: &Path) -> Result<()> {
    let out = std::fs::File::create(target).map_err(Error::io(target))?;
    let mut zip = zip::ZipWriter::new(out);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let zip_err = |e: zip::result::ZipError| Error::Zip {
        path: target.to_path_buf(),
        source: e,
    };
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for e in std::fs::read_dir(&current)
            .map_err(Error::io(&current))?
            .flatten()
        {
            let path = e.path();
            let rel = path.strip_prefix(dir).unwrap_or(&path);
            let name = format!("{folder}/{}", rel.to_string_lossy().replace('\\', "/"));
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(path),
                Ok(t) if t.is_file() => {
                    // `session.lock` ist während des Spiels gesperrt und wird nicht gebraucht.
                    if e.file_name() == "session.lock" {
                        continue;
                    }
                    zip.start_file(name, opts).map_err(zip_err)?;
                    let data = std::fs::read(&path).map_err(Error::io(&path))?;
                    zip.write_all(&data).map_err(Error::io(target))?;
                }
                _ => {}
            }
        }
    }
    zip.finish().map_err(zip_err)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldBackup {
    /// Dateiname im Backup-Ordner (auch die ID für Aktionen).
    pub file: String,
    /// Ordnername der gesicherten Welt.
    pub folder: String,
    /// Unix-Zeit in Millisekunden.
    pub created: u64,
    pub size: u64,
    /// Automatisch vor einem Start angelegt.
    pub auto: bool,
}

/// Zerlegt `<ordner>-<zeit>[-auto].zip`. Ältere Backups tragen Sekunden statt Millisekunden.
fn parse_backup_name(file: &str) -> Option<(String, u64, bool)> {
    let stem = file.strip_suffix(".zip")?;
    let (stem, auto) = match stem.strip_suffix("-auto") {
        Some(s) => (s, true),
        None => (stem, false),
    };
    let (folder, stamp) = stem.rsplit_once('-')?;
    if stamp.is_empty() || !stamp.bytes().all(|b| b.is_ascii_digit()) || !valid_folder(folder) {
        return None;
    }
    let stamp: u64 = stamp.parse().ok()?;
    let created = if stamp < 100_000_000_000 {
        stamp * 1000
    } else {
        stamp
    };
    Some((folder.to_string(), created, auto))
}

/// Alle Backups im Ordner, neueste zuerst.
pub fn list_backups(backup_dir: &Path) -> Result<Vec<WorldBackup>> {
    let entries = match std::fs::read_dir(backup_dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(backup_dir)(e)),
    };
    let mut backups: Vec<WorldBackup> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().into_owned();
            let (folder, created, auto) = parse_backup_name(&file)?;
            Some(WorldBackup {
                size: e.metadata().map(|m| m.len()).unwrap_or(0),
                file,
                folder,
                created,
                auto,
            })
        })
        .collect();
    backups.sort_by(|a, b| b.created.cmp(&a.created).then(a.file.cmp(&b.file)));
    Ok(backups)
}

fn backup_file(backup_dir: &Path, file: &str) -> Result<(PathBuf, String)> {
    let (folder, _, _) = parse_backup_name(file)
        .filter(|_| valid_folder(file))
        .ok_or_else(|| Error::Invalid(format!("Ungültiges Backup {file:?}")))?;
    let path = backup_dir.join(file);
    if !path.is_file() {
        return Err(Error::Invalid(format!("Backup {file:?} nicht gefunden")));
    }
    Ok((path, folder))
}

/// Löscht automatische Backups einer Welt bis auf die neuesten `keep`. Manuelle bleiben.
pub fn prune_auto(backup_dir: &Path, folder: &str, keep: usize) -> Result<()> {
    let autos = list_backups(backup_dir)?
        .into_iter()
        .filter(|b| b.auto && b.folder == folder);
    for b in autos.skip(keep) {
        let path = backup_dir.join(&b.file);
        std::fs::remove_file(&path).map_err(Error::io(&path))?;
    }
    Ok(())
}

pub fn delete_backup(backup_dir: &Path, file: &str) -> Result<()> {
    let (path, _) = backup_file(backup_dir, file)?;
    std::fs::remove_file(&path).map_err(Error::io(&path))
}

/// Ob seit dem letzten Backup der Welt gespielt wurde (Änderungszeit von `level.dat`).
pub fn changed_since_backup(game_dir: &Path, folder: &str, backup_dir: &Path) -> Result<bool> {
    let level = world_dir(game_dir, folder)?.join("level.dat");
    let modified = std::fs::metadata(&level)
        .and_then(|m| m.modified())
        .map_err(Error::io(&level))?
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(u64::MAX);
    let newest = list_backups(backup_dir)?
        .into_iter()
        .find(|b| b.folder == folder)
        .map(|b| b.created);
    Ok(newest.is_none_or(|created| created < modified))
}

/// Ersetzt die Welt durch den Stand aus dem Backup. Das ZIP wird zuerst vollständig in einen
/// Zwischenordner entpackt und geprüft; erst danach wird die aktuelle Welt gesichert (manuelles
/// Backup, das nicht rotiert wird) und getauscht. Gibt den Pfad dieser Sicherung zurück, falls
/// es die Welt noch gab.
pub fn restore(game_dir: &Path, backup_dir: &Path, file: &str) -> Result<Option<PathBuf>> {
    let (zip_path, folder) = backup_file(backup_dir, file)?;
    let saves = game_dir.join("saves");
    std::fs::create_dir_all(&saves).map_err(Error::io(&saves))?;
    let staging = saves.join(format!(".pinecone-restore-{folder}"));
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(Error::io(&staging))?;
    }
    if let Err(e) = unpack(&zip_path, &folder, &staging) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }

    let target = saves.join(&folder);
    let safety = if target.join("level.dat").is_file() {
        match backup(game_dir, &folder, backup_dir) {
            Ok(p) => Some(p),
            Err(e) => {
                let _ = std::fs::remove_dir_all(&staging);
                return Err(e);
            }
        }
    } else {
        None
    };
    let old = saves.join(format!(".pinecone-old-{folder}"));
    if old.exists() {
        std::fs::remove_dir_all(&old).map_err(Error::io(&old))?;
    }
    if target.exists() {
        std::fs::rename(&target, &old).map_err(Error::io(&target))?;
    }
    if let Err(e) = std::fs::rename(&staging, &target) {
        // Alten Stand zurücklegen, damit die Welt nicht verschwindet.
        if old.exists() {
            let _ = std::fs::rename(&old, &target);
        }
        let _ = std::fs::remove_dir_all(&staging);
        return Err(Error::io(&target)(e));
    }
    if old.exists() {
        std::fs::remove_dir_all(&old).map_err(Error::io(&old))?;
    }
    Ok(safety)
}

/// Entpackt alle Einträge unter `<folder>/` nach `target`; alles andere ist ein Fehler.
fn unpack(zip_path: &Path, folder: &str, target: &Path) -> Result<()> {
    let zip_err = |e: zip::result::ZipError| Error::Zip {
        path: zip_path.to_path_buf(),
        source: e,
    };
    let file = std::fs::File::open(zip_path).map_err(Error::io(zip_path))?;
    let mut zip = zip::ZipArchive::new(file).map_err(zip_err)?;
    std::fs::create_dir_all(target).map_err(Error::io(target))?;
    let prefix = format!("{folder}/");
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(zip_err)?;
        let name = entry.name().replace('\\', "/");
        let Some(rel) = name.strip_prefix(&prefix) else {
            return Err(Error::Invalid(format!(
                "Das Backup enthält fremde Dateien ({name})"
            )));
        };
        if rel.is_empty() {
            continue;
        }
        let rel = rel.trim_end_matches('/');
        let path = target.join(safe_relative(rel)?);
        if entry.is_dir() {
            std::fs::create_dir_all(&path).map_err(Error::io(&path))?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
        }
        let mut out = std::fs::File::create(&path).map_err(Error::io(&path))?;
        std::io::copy(&mut entry, &mut out).map_err(Error::io(&path))?;
    }
    if !target.join("level.dat").is_file() {
        return Err(Error::Invalid("Im Backup fehlt level.dat".into()));
    }
    Ok(())
}

pub fn delete(game_dir: &Path, folder: &str) -> Result<()> {
    let dir = world_dir(game_dir, folder)?;
    std::fs::remove_dir_all(&dir).map_err(Error::io(&dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nbt::tests::Writer;

    fn level_dat(name: &str) -> Vec<u8> {
        // Kompound "" { Data { LevelName, GameType, LastPlayed, Version { Name } } }
        let mut w = Writer(Vec::new());
        w.named(10, "").named(10, "Data");
        w.named(8, "LevelName").str(name);
        w.named(3, "GameType").int(1);
        w.named(4, "LastPlayed");
        w.0.extend_from_slice(&1_700_000_000_000i64.to_be_bytes());
        w.named(10, "Version")
            .named(8, "Name")
            .str("1.21.11")
            .byte(0);
        w.byte(0).byte(0);
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&w.0).unwrap();
        gz.finish().unwrap()
    }

    #[test]
    fn lists_backs_up_and_deletes() {
        let dir = tempfile::tempdir().unwrap();
        let world = dir.path().join("saves/Neue Welt");
        std::fs::create_dir_all(world.join("region")).unwrap();
        std::fs::write(world.join("level.dat"), level_dat("Avalon-Burg")).unwrap();
        std::fs::write(world.join("region/r.0.0.mca"), vec![0u8; 4096]).unwrap();
        std::fs::create_dir_all(dir.path().join("saves/kaputt")).unwrap();

        let worlds = list(dir.path()).unwrap();
        assert_eq!(worlds.len(), 1);
        let w = &worlds[0];
        assert_eq!(w.name, "Avalon-Burg");
        assert_eq!(w.folder, "Neue Welt");
        assert_eq!(w.game_mode.as_deref(), Some("creative"));
        assert_eq!(w.last_played, Some(1_700_000_000_000));
        assert_eq!(w.version.as_deref(), Some("1.21.11"));
        assert!(w.size >= 4096);

        let zip = backup(dir.path(), "Neue Welt", &dir.path().join("backups")).unwrap();
        let archive = zip::ZipArchive::new(std::fs::File::open(&zip).unwrap()).unwrap();
        assert!(
            archive
                .file_names()
                .any(|n| n == "Neue Welt/region/r.0.0.mca")
        );

        assert!(delete(dir.path(), "../saves").is_err());
        delete(dir.path(), "Neue Welt").unwrap();
        assert!(list(dir.path()).unwrap().is_empty());
    }

    fn make_world(game: &Path, folder: &str, name: &str) {
        let world = game.join("saves").join(folder);
        std::fs::create_dir_all(world.join("region")).unwrap();
        std::fs::write(world.join("level.dat"), level_dat(name)).unwrap();
        std::fs::write(world.join("region/r.0.0.mca"), b"alt").unwrap();
    }

    #[test]
    fn parses_backup_names() {
        assert_eq!(
            parse_backup_name("Neue Welt-1700000000.zip"),
            Some(("Neue Welt".into(), 1_700_000_000_000, false))
        );
        assert_eq!(
            parse_backup_name("Welt-2-1700000000123-auto.zip"),
            Some(("Welt-2".into(), 1_700_000_000_123, true))
        );
        assert_eq!(parse_backup_name("Welt.zip"), None);
        assert_eq!(parse_backup_name("Welt-12a.zip"), None);
        assert_eq!(parse_backup_name("Welt-1700000000.zip.part"), None);
    }

    #[test]
    fn restores_and_keeps_safety_backup() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let backups = dir.path().join("backups");
        make_world(&game, "Welt", "Burg");
        let zip = backup(&game, "Welt", &backups).unwrap();
        let file = zip.file_name().unwrap().to_string_lossy().into_owned();

        // Welt nach dem Backup verändern, dann zurückholen.
        std::fs::write(game.join("saves/Welt/region/r.0.0.mca"), b"neu").unwrap();
        std::fs::write(game.join("saves/Welt/extra.dat"), b"x").unwrap();
        let safety = restore(&game, &backups, &file).unwrap().unwrap();
        assert_eq!(
            std::fs::read(game.join("saves/Welt/region/r.0.0.mca")).unwrap(),
            b"alt"
        );
        assert!(!game.join("saves/Welt/extra.dat").exists());
        assert!(
            safety.is_file(),
            "Stand vor dem Wiederherstellen ist gesichert"
        );
        let names: Vec<_> = std::fs::read_dir(game.join("saves"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["Welt"], "keine Zwischenordner übrig");

        // Gelöschte Welt lässt sich ebenfalls wiederherstellen, ohne Sicherung.
        delete(&game, "Welt").unwrap();
        assert!(restore(&game, &backups, &file).unwrap().is_none());
        assert_eq!(list(&game).unwrap()[0].name, "Burg");
    }

    #[test]
    fn broken_backup_leaves_world_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let backups = dir.path().join("backups");
        make_world(&game, "Welt", "Burg");
        std::fs::create_dir_all(&backups).unwrap();
        std::fs::write(backups.join("Welt-1700000000000.zip"), b"kein zip").unwrap();
        assert!(restore(&game, &backups, "Welt-1700000000000.zip").is_err());
        // Fremder Inhalt (anderer Ordner, Pfad-Ausbruch) wird abgelehnt.
        let evil = backups.join("Welt-1700000000001.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&evil).unwrap());
        zip.start_file("Andere/level.dat", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
        assert!(restore(&game, &backups, "Welt-1700000000001.zip").is_err());
        assert!(restore(&game, &backups, "../Welt-1700000000001.zip").is_err());

        assert_eq!(
            std::fs::read(game.join("saves/Welt/region/r.0.0.mca")).unwrap(),
            b"alt"
        );
        assert_eq!(
            list_backups(&backups).unwrap().len(),
            2,
            "keine Sicherung bei Fehlschlag"
        );
        let names: Vec<_> = std::fs::read_dir(game.join("saves"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["Welt"]);
    }

    #[test]
    fn prunes_only_auto_backups() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let backups = dir.path().join("backups");
        make_world(&game, "Welt", "Burg");
        make_world(&game, "Andere", "Andere");
        let manual = backup(&game, "Welt", &backups).unwrap();
        for _ in 0..4 {
            backup_auto(&game, "Welt", &backups).unwrap();
        }
        backup_auto(&game, "Andere", &backups).unwrap();
        prune_auto(&backups, "Welt", 2).unwrap();
        let all = list_backups(&backups).unwrap();
        assert_eq!(
            all.iter().filter(|b| b.folder == "Welt" && b.auto).count(),
            2
        );
        assert!(manual.is_file());
        assert_eq!(all.iter().filter(|b| b.folder == "Andere").count(), 1);
        // Neueste zuerst.
        assert!(all.windows(2).all(|w| w[0].created >= w[1].created));

        // Nach dem Backup unverändert → kein neues nötig.
        assert!(!changed_since_backup(&game, "Welt", &backups).unwrap());
        delete_backup(&backups, &manual.file_name().unwrap().to_string_lossy()).unwrap();
        assert!(!manual.exists());
    }
}
