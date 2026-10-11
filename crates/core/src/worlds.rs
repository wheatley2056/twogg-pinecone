//! Welten einer Instanz (`saves/`): Liste mit Name, Spielmodus und letzter Änderung aus
//! `level.dat`, Sicherung als ZIP und Löschen.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

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
    let dir = world_dir(game_dir, folder)?;
    std::fs::create_dir_all(target_dir).map_err(Error::io(target_dir))?;
    let target = target_dir.join(format!("{folder}-{}.zip", crate::stats::now_secs()));
    let out = std::fs::File::create(&target).map_err(Error::io(&target))?;
    let mut zip = zip::ZipWriter::new(out);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let zip_err = |e: zip::result::ZipError| Error::Zip {
        path: target.clone(),
        source: e,
    };
    let mut stack = vec![dir.clone()];
    while let Some(current) = stack.pop() {
        for e in std::fs::read_dir(&current)
            .map_err(Error::io(&current))?
            .flatten()
        {
            let path = e.path();
            let rel = path.strip_prefix(&dir).unwrap_or(&path);
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
                    zip.write_all(&data).map_err(Error::io(&target))?;
                }
                _ => {}
            }
        }
    }
    zip.finish().map_err(zip_err)?;
    Ok(target)
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
}
