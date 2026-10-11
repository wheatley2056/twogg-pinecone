//! Screenshots der Profile (`<spielordner>/screenshots/*.png`): Liste, Vorschaubilder, Löschen.

use std::path::{Path, PathBuf};

use serde::Serialize;
use sha1::{Digest, Sha1};

use crate::{Error, Result};

/// Kantenlänge der Vorschaubilder (längere Seite) in Pixeln.
pub const THUMB_SIZE: u32 = 360;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub profile_id: String,
    pub path: PathBuf,
    /// Unix-Zeit (Sekunden) der letzten Änderung.
    pub modified: u64,
    pub size: u64,
}

/// Screenshots eines Spielordners.
pub fn list(profile_id: &str, game_dir: &Path) -> Vec<Screenshot> {
    let Ok(entries) = std::fs::read_dir(game_dir.join("screenshots")) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("png"))
        })
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let modified = meta
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            Some(Screenshot {
                profile_id: profile_id.to_string(),
                path: e.path(),
                modified,
                size: meta.len(),
            })
        })
        .collect()
}

/// Die neuesten `limit` Screenshots über alle Spielordner.
pub fn latest<'a>(
    dirs: impl IntoIterator<Item = (&'a str, PathBuf)>,
    limit: usize,
) -> Vec<Screenshot> {
    let mut all: Vec<Screenshot> = dirs
        .into_iter()
        .flat_map(|(id, dir)| list(id, &dir))
        .collect();
    all.sort_by_key(|s| std::cmp::Reverse(s.modified));
    all.truncate(limit);
    all
}

/// Liegt `path` als PNG in `<instances>/<id>/.minecraft/screenshots/`? Prüft den echten Pfad
/// (Symlinks aufgelöst), damit die Oberfläche keine anderen Dateien öffnen oder löschen kann.
pub fn checked(path: &Path, instances: &Path) -> Result<PathBuf> {
    let deny = || Error::Invalid("Nur Screenshots aus Profilen sind erlaubt".into());
    let real = std::fs::canonicalize(path).map_err(|_| deny())?;
    let root = std::fs::canonicalize(instances).map_err(|_| deny())?;
    let rel = real.strip_prefix(&root).map_err(|_| deny())?;
    let parts: Vec<_> = rel.components().map(|c| c.as_os_str()).collect();
    let ok = parts.len() == 4
        && parts[1] == ".minecraft"
        && parts[2] == "screenshots"
        && real
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("png"))
        && real.is_file();
    if ok { Ok(real) } else { Err(deny()) }
}

/// Vorschaubild (JPEG) im Cache; wird neu erzeugt, wenn sich der Screenshot ändert.
pub fn thumbnail(path: &Path, cache: &Path) -> Result<PathBuf> {
    let meta = std::fs::metadata(path).map_err(Error::io(path))?;
    let stamp = meta
        .modified()
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let key = format!(
        "{:x}",
        Sha1::digest(format!("{}|{stamp}|{}", path.display(), meta.len()))
    );
    let target = cache.join(format!("{key}.jpg"));
    if target.is_file() {
        return Ok(target);
    }
    let img = image::open(path)
        .map_err(|e| Error::Invalid(format!("{} ist kein lesbares Bild: {e}", path.display())))?;
    let thumb = img.thumbnail(THUMB_SIZE, THUMB_SIZE).to_rgb8();
    std::fs::create_dir_all(cache).map_err(Error::io(cache))?;
    let tmp = target.with_extension("jpg.tmp");
    let file = std::fs::File::create(&tmp).map_err(Error::io(&tmp))?;
    image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), 82)
        .encode_image(&thumb)
        .map_err(|e| Error::Invalid(format!("Vorschaubild nicht erzeugt: {e}")))?;
    std::fs::rename(&tmp, &target).map_err(Error::io(&target))?;
    Ok(target)
}

/// Löscht einen Screenshot (nur, was `checked` erlaubt).
pub fn delete(path: &Path, instances: &Path) -> Result<()> {
    let real = checked(path, instances)?;
    std::fs::remove_file(&real).map_err(Error::io(&real))
}

/// Entfernt Vorschaubilder, die älter als `max_age_secs` sind (ihr Screenshot ist evtl. weg).
pub fn prune_thumbnails(cache: &Path, max_age_secs: u64) {
    let Ok(entries) = std::fs::read_dir(cache) else {
        return;
    };
    let now = std::time::SystemTime::now();
    for e in entries.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .is_some_and(|age| age.as_secs() > max_age_secs);
        if old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_latest_pngs() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        for (d, name, age) in [
            (&a, "alt.png", 300),
            (&a, "neu.png", 10),
            (&b, "mittel.PNG", 100),
            (&b, "notiz.txt", 0),
        ] {
            std::fs::create_dir_all(d.join("screenshots")).unwrap();
            let f = d.join("screenshots").join(name);
            std::fs::write(&f, b"x").unwrap();
            let t = std::time::SystemTime::now() - std::time::Duration::from_secs(age);
            std::fs::File::options()
                .write(true)
                .open(&f)
                .unwrap()
                .set_modified(t)
                .unwrap();
        }
        let shots = latest(
            [
                ("a", a.clone()),
                ("b", b.clone()),
                ("c", dir.path().join("fehlt")),
            ],
            2,
        );
        let names: Vec<String> = shots
            .iter()
            .map(|s| s.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["neu.png", "mittel.PNG"]);
        assert_eq!(shots[1].profile_id, "b");
    }

    #[test]
    fn checks_paths_and_makes_thumbnails() {
        let dir = tempfile::tempdir().unwrap();
        let instances = dir.path().join("instances");
        let shots = instances.join("p1/.minecraft/screenshots");
        std::fs::create_dir_all(&shots).unwrap();
        let shot = shots.join("2026-10-11_12.00.00.png");
        image::RgbImage::from_pixel(800, 400, image::Rgb([10, 200, 30]))
            .save(&shot)
            .unwrap();
        let other = instances.join("p1/.minecraft/options.png");
        std::fs::write(&other, "x").unwrap();

        assert!(checked(&shot, &instances).is_ok());
        assert!(checked(&other, &instances).is_err());
        assert!(checked(&shots.join("../options.png"), &instances).is_err());
        assert!(checked(&dir.path().join("fremd.png"), &instances).is_err());

        let cache = dir.path().join("thumbs");
        let thumb = thumbnail(&shot, &cache).unwrap();
        let img = image::open(&thumb).unwrap();
        assert_eq!((img.width(), img.height()), (THUMB_SIZE, THUMB_SIZE / 2));
        // Zweiter Aufruf nutzt den Cache.
        assert_eq!(thumbnail(&shot, &cache).unwrap(), thumb);

        delete(&shot, &instances).unwrap();
        assert!(!shot.exists());
        assert!(delete(&other, &instances).is_err());
        assert!(other.exists());
    }
}
