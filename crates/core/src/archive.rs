//! Vollständiger Instanz-Export als `.pinecone.zip` (F-54): Profil, Mods, Configs und nach Wahl
//! Welten, Screenshots, Ressourcen- und Shaderpakete. Für den Umzug auf einen anderen Rechner.
//!
//! Aufbau: `pinecone-instance.json` (Format und Profil) und darunter `instance/…` mit dem Inhalt
//! des Instanzordners. Konten, Start-Befehle (F-55) und Pinecone-Einstellungen sind nie dabei;
//! beim Import werden sie zusätzlich verworfen, falls jemand sie von Hand ergänzt hat.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::modpack::PackPreview;
use crate::paths::safe_relative;
use crate::profiles::{Profile, ProfileStore};
use crate::{Error, Result};

pub const MANIFEST: &str = "pinecone-instance.json";
const PREFIX: &str = "instance/";
const FORMAT: u32 = 1;

/// Was zusätzlich zu Mods, Configs und Einstellungen exportiert wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    pub worlds: bool,
    pub screenshots: bool,
    pub resourcepacks: bool,
    pub shaderpacks: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            worlds: true,
            screenshots: false,
            resourcepacks: true,
            shaderpacks: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    format: u32,
    profile: Profile,
}

/// Entfernt, was nie zwischen Rechnern wandern darf: festes Konto und Start-Befehle.
fn sanitize(profile: &mut Profile) {
    profile.account = None;
    profile.pre_launch = None;
    profile.wrapper = None;
    profile.post_exit = None;
}

/// Relativer Pfad im Instanzordner → mitnehmen?
fn included(rel: &Path, opts: ExportOptions) -> bool {
    let parts: Vec<&str> = rel.iter().filter_map(|c| c.to_str()).collect();
    match parts.as_slice() {
        ["profile.json"] | ["snapshots", ..] => false,
        [".minecraft", top, ..] => match *top {
            "logs" | "crash-reports" => false,
            "saves" => opts.worlds && parts.last() != Some(&"session.lock"),
            "screenshots" => opts.screenshots,
            "resourcepacks" => opts.resourcepacks,
            "shaderpacks" => opts.shaderpacks,
            _ => true,
        },
        _ => true,
    }
}

/// Alle Dateien, die exportiert würden: (relativer Pfad, absoluter Pfad, Größe).
fn files(dir: &Path, opts: ExportOptions) -> Result<Vec<(String, PathBuf, u64)>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = std::fs::read_dir(&current).map_err(Error::io(&current))?;
        for e in entries.flatten() {
            let path = e.path();
            let rel = path.strip_prefix(dir).unwrap_or(&path).to_path_buf();
            if !included(&rel, opts) {
                continue;
            }
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(path),
                Ok(t) if t.is_file() => {
                    let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                    let name = rel.to_string_lossy().replace('\\', "/");
                    out.push((name, path, size));
                }
                _ => {}
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Ungefähre Größe des Exports (unkomprimiert), für die Anzeige im Dialog.
pub fn export_size(store: &ProfileStore, id: &str, opts: ExportOptions) -> Result<u64> {
    let profile = store.get(id)?;
    Ok(files(&store.instance_dir(&profile.id), opts)?
        .iter()
        .map(|(_, _, s)| s)
        .sum())
}

/// Schreibt den Export nach `target`. Eine halbe Datei wird bei Fehlern entfernt.
pub fn export(store: &ProfileStore, id: &str, target: &Path, opts: ExportOptions) -> Result<()> {
    let mut profile = store.get(id)?;
    sanitize(&mut profile);
    let dir = store.instance_dir(&profile.id);
    let partial = target.with_extension("zip.part");
    let result = write_zip(&dir, &profile, &partial, opts)
        .and_then(|()| std::fs::rename(&partial, target).map_err(Error::io(target)));
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result
}

fn write_zip(dir: &Path, profile: &Profile, target: &Path, opts: ExportOptions) -> Result<()> {
    let zip_err = |e: zip::result::ZipError| Error::Zip {
        path: target.to_path_buf(),
        source: e,
    };
    let out = std::fs::File::create(target).map_err(Error::io(target))?;
    let mut zip = zip::ZipWriter::new(out);
    let deflate = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(true);
    let manifest = Manifest {
        format: FORMAT,
        profile: profile.clone(),
    };
    zip.start_file(MANIFEST, deflate).map_err(zip_err)?;
    let json = serde_json::to_vec_pretty(&manifest).map_err(Error::json(MANIFEST))?;
    zip.write_all(&json).map_err(Error::io(target))?;
    for (name, path, _) in files(dir, opts)? {
        zip.start_file(format!("{PREFIX}{name}"), deflate)
            .map_err(zip_err)?;
        let mut file = std::fs::File::open(&path).map_err(Error::io(&path))?;
        std::io::copy(&mut file, &mut zip).map_err(Error::io(target))?;
    }
    zip.finish().map_err(zip_err)?;
    Ok(())
}

fn open(path: &Path) -> Result<zip::ZipArchive<std::fs::File>> {
    let file = std::fs::File::open(path).map_err(Error::io(path))?;
    zip::ZipArchive::new(file).map_err(|e| Error::Zip {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Ist die Datei ein Pinecone-Instanz-Export (und kein Modpack)?
pub fn is_instance_archive(path: &Path) -> bool {
    open(path).is_ok_and(|zip| zip.index_for_name(MANIFEST).is_some())
}

fn read_manifest(zip: &mut zip::ZipArchive<std::fs::File>, path: &Path) -> Result<Manifest> {
    let mut entry = zip.by_name(MANIFEST).map_err(|e| Error::Zip {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut text = String::new();
    entry
        .by_ref()
        .take(1024 * 1024)
        .read_to_string(&mut text)
        .map_err(Error::io(path))?;
    let manifest: Manifest = serde_json::from_str(&text).map_err(Error::json(MANIFEST))?;
    if manifest.format > FORMAT {
        return Err(Error::Invalid(
            "Dieser Export stammt aus einer neueren Pinecone-Version. Bitte Pinecone aktualisieren."
                .into(),
        ));
    }
    Ok(manifest)
}

/// Vorschau für den Import-Dialog (gleiche Form wie bei Modpacks).
pub fn preview(path: &Path) -> Result<PackPreview> {
    let mut zip = open(path)?;
    let manifest = read_manifest(&mut zip, path)?;
    let (mut mods, mut resourcepacks, mut shaders, mut other) = (0, 0, 0, 0);
    for name in zip.file_names() {
        let Some(rel) = name.strip_prefix(PREFIX) else {
            continue;
        };
        if rel.ends_with('/') {
            continue;
        }
        match rel
            .strip_prefix(".minecraft/")
            .and_then(|r| r.split('/').next())
        {
            Some("mods") => mods += 1,
            Some("resourcepacks") => resourcepacks += 1,
            Some("shaderpacks") => shaders += 1,
            _ => other += 1,
        }
    }
    let p = manifest.profile;
    Ok(PackPreview {
        name: p.name,
        version: String::new(),
        summary: None,
        minecraft: p.minecraft,
        loader: p.loader,
        mods,
        resourcepacks,
        shaders,
        other_files: other,
        download_size: 0,
        archive_size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        missing: Vec::new(),
        blocked_hosts: Vec::new(),
    })
}

/// Legt aus dem Export eine neue Instanz an. Gibt es den Namen schon, bekommt er ein Suffix.
/// Eine Event-Instanz wird zur normalen Instanz. Bei Fehlern wird die halbe Instanz entfernt.
pub fn import(store: &ProfileStore, path: &Path, name: Option<String>) -> Result<Profile> {
    let mut zip = open(path)?;
    let manifest = read_manifest(&mut zip, path)?;
    let mut profile = manifest.profile;
    sanitize(&mut profile);
    let base = name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| profile.name.clone());
    profile.name = base.clone();
    let mut n = 2;
    while store.find(&profile.name)?.is_some() {
        profile.name = format!("{base} ({n})");
        n += 1;
    }
    let created = store.create(profile)?;
    let dir = store.instance_dir(&created.id);
    let result = unpack(&mut zip, path, &dir)
        .and_then(|()| crate::profiles::unbind_event(&dir.join("pack.lock.json")));
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }
    Ok(created)
}

fn unpack(zip: &mut zip::ZipArchive<std::fs::File>, path: &Path, dir: &Path) -> Result<()> {
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| Error::Zip {
            path: path.to_path_buf(),
            source: e,
        })?;
        let name = entry.name().replace('\\', "/");
        if name == MANIFEST {
            continue;
        }
        let Some(rel) = name.strip_prefix(PREFIX) else {
            return Err(Error::Invalid(format!(
                "Unbekannter Eintrag im Export: {name}"
            )));
        };
        let rel = rel.trim_end_matches('/');
        if rel.is_empty() || rel == "profile.json" {
            continue;
        }
        let target = dir.join(safe_relative(rel)?);
        if entry.is_dir() {
            std::fs::create_dir_all(&target).map_err(Error::io(&target))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
        }
        let mut out = std::fs::File::create(&target).map_err(Error::io(&target))?;
        std::io::copy(&mut entry, &mut out).map_err(Error::io(&target))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Paths;

    fn setup() -> (tempfile::TempDir, ProfileStore, Profile) {
        let dir = tempfile::tempdir().unwrap();
        let store = ProfileStore::new(Paths::new(dir.path().join("data")));
        let mut p = Profile::new("Avalon", "1.21.11");
        p.memory_mb = Some(6144);
        p.account = Some("1a70aea0-11fc-356b-8f09-325a0158a7d0".into());
        p.pre_launch = Some("rm -rf ~".into());
        p.wrapper = Some("gamemoderun".into());
        let p = store.create(p).unwrap();
        let game = store.game_dir(&p.id);
        for (rel, data) in [
            ("mods/a.jar", "jar"),
            ("config/c.toml", "cfg"),
            ("options.txt", "fov:90"),
            ("saves/Welt/level.dat", "lvl"),
            ("saves/Welt/session.lock", "lock"),
            ("screenshots/s.png", "png"),
            ("resourcepacks/r.zip", "rp"),
            ("logs/latest.log", "log"),
        ] {
            let path = game.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, data).unwrap();
        }
        std::fs::create_dir_all(store.instance_dir(&p.id).join("snapshots/1")).unwrap();
        std::fs::write(store.instance_dir(&p.id).join("snapshots/1/x"), "s").unwrap();
        (dir, store, p)
    }

    #[test]
    fn exports_and_imports_on_another_machine() {
        let (dir, store, p) = setup();
        let target = dir.path().join("avalon.pinecone.zip");
        let opts = ExportOptions::default();
        assert!(export_size(&store, &p.id, opts).unwrap() > 0);
        export(&store, &p.id, &target, opts).unwrap();
        assert!(is_instance_archive(&target));

        let names: Vec<String> = open(&target)
            .unwrap()
            .file_names()
            .map(String::from)
            .collect();
        assert!(names.contains(&"instance/.minecraft/mods/a.jar".to_string()));
        assert!(names.contains(&"instance/.minecraft/saves/Welt/level.dat".to_string()));
        assert!(!names.iter().any(|n| n.contains("session.lock")));
        assert!(!names.iter().any(|n| n.contains("logs/")));
        assert!(
            !names.iter().any(|n| n.contains("screenshots/")),
            "Screenshots nur auf Wunsch"
        );
        assert!(!names.iter().any(|n| n.contains("snapshots/")));
        assert!(!names.iter().any(|n| n.ends_with("profile.json")));
        let manifest = read_manifest(&mut open(&target).unwrap(), &target).unwrap();
        assert!(
            manifest.profile.account.is_none(),
            "kein Konto im Export (AK-54.1)"
        );
        assert!(manifest.profile.pre_launch.is_none() && manifest.profile.wrapper.is_none());

        let preview = preview(&target).unwrap();
        assert_eq!(preview.name, "Avalon");
        assert_eq!((preview.mods, preview.resourcepacks), (1, 1));

        // „Anderer Rechner“: neuer Datenordner.
        let other = ProfileStore::new(Paths::new(dir.path().join("other")));
        let imported = import(&other, &target, None).unwrap();
        assert_eq!(imported.name, "Avalon");
        assert_eq!(imported.memory_mb, Some(6144));
        assert!(imported.account.is_none());
        let game = other.game_dir(&imported.id);
        assert_eq!(
            std::fs::read_to_string(game.join("mods/a.jar")).unwrap(),
            "jar"
        );
        assert!(game.join("saves/Welt/level.dat").is_file());
        // Zweiter Import: Name mit Suffix.
        assert_eq!(import(&other, &target, None).unwrap().name, "Avalon (2)");
    }

    #[test]
    fn rejects_path_escapes_and_cleans_up() {
        let (dir, store, _) = setup();
        let evil = dir.path().join("evil.pinecone.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&evil).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        zip.start_file(MANIFEST, o).unwrap();
        let mut profile = Profile::new("Böse", "1.21.11");
        profile.pre_launch = Some("evil".into());
        zip.write_all(&serde_json::to_vec(&Manifest { format: 1, profile }).unwrap())
            .unwrap();
        zip.start_file("instance/.minecraft/ok.txt", o).unwrap();
        zip.write_all(b"ok").unwrap();
        zip.start_file("instance/../../ausbruch.txt", o).unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
        assert!(import(&store, &evil, None).is_err());
        assert!(
            store.find("Böse").unwrap().is_none(),
            "halbe Instanz entfernt"
        );
        assert!(!dir.path().join("ausbruch.txt").exists());
        // Ein Modpack (ohne Manifest) ist kein Instanz-Export.
        let mrpack = dir.path().join("x.mrpack");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&mrpack).unwrap());
        zip.start_file("modrinth.index.json", o).unwrap();
        zip.finish().unwrap();
        assert!(!is_instance_archive(&mrpack));
    }
}
