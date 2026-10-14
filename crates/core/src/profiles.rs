//! Profile (Instanzen): je ein Ordner `instances/<id>/` mit `profile.json` und eigenem `.minecraft`.

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::loader::LoaderSpec;
use crate::paths::Paths;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Ordnername unter `instances/`; ergibt sich aus dem Speicherort, nicht aus der Datei.
    #[serde(skip)]
    pub id: String,
    pub name: String,
    /// Minecraft-Version, z. B. `26.3`.
    pub minecraft: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader: Option<LoaderSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jvm_args: Vec<String>,
    /// JVM-Preset (`aikar`, `none`, eigenes …); ohne Angabe gilt der globale Standard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jvm_preset: Option<String>,
    /// Fester Java-Pfad; ohne Angabe wird automatisch gewählt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<Resolution>,
    /// Farbe in der Profilliste (`#rrggbb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Kurzes Symbol (ein Emoji oder bis zu zwei Zeichen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Eigene Gruppe in der Bibliothek (F-50); ohne Angabe die Standardgruppe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// UUID des Kontos, mit dem diese Instanz startet; ohne Angabe das aktive Konto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Befehl vor dem Start (F-55); scheitert er, startet das Spiel nicht.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_launch: Option<String>,
    /// Wrapper um Java, z. B. `gamemoderun` oder `prime-run`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrapper: Option<String>,
    /// Befehl nach dem Spielende.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_exit: Option<String>,
    /// Geänderte Welten vor jedem Start automatisch sichern.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub backup_on_launch: bool,
    /// So viele automatische Backups je Welt behalten; ohne Angabe [`DEFAULT_BACKUP_KEEP`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_keep: Option<u32>,
}

/// Was beim Duplizieren zusätzlich zum Spielordner mitkopiert wird.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateOptions {
    pub worlds: bool,
    pub screenshots: bool,
}

/// Gruppennamen: getrimmt, 1–32 Zeichen; leer = keine Gruppe.
pub fn clean_group(name: Option<&str>) -> Result<Option<String>> {
    match name.map(str::trim).filter(|n| !n.is_empty()) {
        None => Ok(None),
        Some(n) if n.chars().count() > 32 => Err(Error::Invalid(
            "Gruppenname darf höchstens 32 Zeichen lang sein".into(),
        )),
        Some(n) => Ok(Some(n.to_string())),
    }
}

/// Macht aus einer Event-Instanz eine normale: Die TwoGG-Pack-Bindung im `pack.lock.json`
/// entfällt, ihre Dateien gelten als eigene. Andere Packs bleiben gebunden.
pub(crate) fn unbind_event(lock_path: &std::path::Path) -> Result<()> {
    let Ok(bytes) = std::fs::read(lock_path) else {
        return Ok(());
    };
    let mut lock: crate::content::ContentLock =
        serde_json::from_slice(&bytes).map_err(Error::json(lock_path.display().to_string()))?;
    if lock
        .pack
        .as_ref()
        .is_some_and(|p| matches!(p.source, crate::modpack::PackSource::Twogg { .. }))
    {
        lock.pack = None;
        for entry in lock.files.values_mut() {
            entry.from_pack = false;
        }
        let json = serde_json::to_vec_pretty(&lock).map_err(Error::json("pack.lock.json"))?;
        std::fs::write(lock_path, json).map_err(Error::io(lock_path))?;
    }
    Ok(())
}

/// Automatische Welt-Backups je Welt, wenn das Profil nichts anderes sagt.
pub const DEFAULT_BACKUP_KEEP: u32 = 5;

impl Profile {
    pub fn new(name: impl Into<String>, minecraft: impl Into<String>) -> Self {
        Self {
            id: String::new(),
            name: name.into(),
            minecraft: minecraft.into(),
            loader: None,
            memory_mb: None,
            jvm_args: Vec::new(),
            jvm_preset: None,
            java: None,
            resolution: None,
            color: None,
            icon: None,
            group: None,
            account: None,
            pre_launch: None,
            wrapper: None,
            post_exit: None,
            backup_on_launch: false,
            backup_keep: None,
        }
    }

    /// Version-ID, die gestartet wird (Vanilla oder Loader-Profil).
    pub fn version_id(&self) -> String {
        match &self.loader {
            Some(l) => l.kind.version_id(&self.minecraft, &l.version),
            None => self.minecraft.clone(),
        }
    }
}

/// Ordnername aus einem Profilnamen: klein, ASCII, Bindestriche statt Leerzeichen.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        match c {
            'a'..='z' | '0'..='9' => out.push(c),
            'ä' => out.push_str("ae"),
            'ö' => out.push_str("oe"),
            'ü' => out.push_str("ue"),
            'ß' => out.push_str("ss"),
            _ if !out.ends_with('-') => out.push('-'),
            _ => {}
        }
    }
    let slug: String = out.trim_matches('-').chars().take(48).collect();
    let slug = slug.trim_end_matches('-').to_string();
    if slug.is_empty() {
        "profil".into()
    } else {
        slug
    }
}

#[derive(Clone)]
pub struct ProfileStore {
    paths: Paths,
}

impl ProfileStore {
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    fn profile_file(&self, id: &str) -> PathBuf {
        self.paths.instance_dir(id).join("profile.json")
    }

    /// Instanzordner eines Profils (`instances/<id>/`, enthält `profile.json` und `.minecraft`).
    pub fn instance_dir(&self, id: &str) -> PathBuf {
        self.paths.instance_dir(id)
    }

    /// Spielordner eines Profils.
    pub fn game_dir(&self, id: &str) -> PathBuf {
        self.paths.instance_dir(id).join(".minecraft")
    }

    fn read(&self, id: &str) -> Result<Profile> {
        let path = self.profile_file(id);
        let bytes = std::fs::read(&path).map_err(Error::io(&path))?;
        let mut profile: Profile =
            serde_json::from_slice(&bytes).map_err(Error::json(path.display().to_string()))?;
        profile.id = id.to_string();
        Ok(profile)
    }

    /// Alle Profile, nach Name sortiert. Kaputte `profile.json` werden übersprungen und geloggt.
    pub fn list(&self) -> Result<Vec<Profile>> {
        let dir = self.paths.root().join("instances");
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(Error::io(&dir)(e)),
        };
        let mut profiles: Vec<Profile> = entries
            .flatten()
            .filter_map(|e| {
                let id = e.file_name().to_string_lossy().into_owned();
                if !self.profile_file(&id).is_file() {
                    return None;
                }
                self.read(&id)
                    .inspect_err(|err| tracing::warn!("Profil {id} übersprungen: {err}"))
                    .ok()
            })
            .collect();
        profiles.sort_by_key(|p| p.name.to_lowercase());
        Ok(profiles)
    }

    /// Sucht per ID oder Name (ohne Groß-/Kleinschreibung).
    pub fn find(&self, selector: &str) -> Result<Option<Profile>> {
        Ok(self
            .list()?
            .into_iter()
            .find(|p| p.id == selector || p.name.eq_ignore_ascii_case(selector)))
    }

    pub fn get(&self, selector: &str) -> Result<Profile> {
        self.find(selector)?
            .ok_or_else(|| Error::Invalid(format!("Kein Profil {selector:?} vorhanden")))
    }

    /// Legt ein neues Profil an. Die ID wird aus dem Namen gebildet und bei Bedarf durchnummeriert.
    pub fn create(&self, mut profile: Profile) -> Result<Profile> {
        profile.name = profile.name.trim().to_string();
        if profile.name.is_empty() || profile.name.chars().count() > 64 {
            return Err(Error::Invalid(
                "Profilname muss 1–64 Zeichen lang sein".into(),
            ));
        }
        if self.find(&profile.name)?.is_some() {
            return Err(Error::Invalid(format!(
                "Es gibt schon ein Profil {:?}",
                profile.name
            )));
        }
        let base = slugify(&profile.name);
        let mut id = base.clone();
        let mut n = 2;
        while self.paths.instance_dir(&id).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        profile.id = id;
        let game_dir = self.game_dir(&profile.id);
        std::fs::create_dir_all(&game_dir).map_err(Error::io(&game_dir))?;
        self.save(&profile)?;
        Ok(profile)
    }

    pub fn save(&self, profile: &Profile) -> Result<()> {
        if profile.id.is_empty() {
            return Err(Error::Invalid(
                "Profil ohne ID kann nicht gespeichert werden".into(),
            ));
        }
        let path = self.profile_file(&profile.id);
        let json = serde_json::to_vec_pretty(profile).map_err(Error::json("profile.json"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(Error::io(&tmp))?;
        std::fs::rename(&tmp, &path).map_err(Error::io(&path))
    }

    /// Kopiert ein Profil samt Mods, Configs und Einstellungen unter neuem Namen. Logs,
    /// Absturzberichte und Pack-Snapshots bleiben weg, Welten und Screenshots nach Wahl.
    /// Eine TwoGG-Event-Instanz wird zur normalen Instanz: Die Pack-Bindung entfällt, ihre
    /// Dateien gelten als eigene. Bei einem Fehler wird die halbe Kopie entfernt.
    pub fn duplicate(&self, source: &str, name: &str, opts: DuplicateOptions) -> Result<Profile> {
        let original = self.get(source)?;
        let mut copy = original.clone();
        copy.name = name.to_string();
        let copy = self.create(copy)?;
        let result = self.copy_instance(&original.id, &copy.id, opts);
        if let Err(e) = result {
            let _ = std::fs::remove_dir_all(self.paths.instance_dir(&copy.id));
            return Err(e);
        }
        Ok(copy)
    }

    fn copy_instance(&self, from: &str, to: &str, opts: DuplicateOptions) -> Result<()> {
        // Instanzordner: alles außer profile.json (schon angelegt), Spielordner und Snapshots.
        let src = self.paths.instance_dir(from);
        let dst = self.paths.instance_dir(to);
        let top = HashSet::from(["profile.json", ".minecraft", "snapshots"]);
        crate::migrate::copy_dir(&src, &dst, &top, true)?;

        let mut skip = HashSet::from(["logs", "crash-reports", "session.lock"]);
        if !opts.worlds {
            skip.insert("saves");
        }
        if !opts.screenshots {
            skip.insert("screenshots");
        }
        crate::migrate::copy_dir(&self.game_dir(from), &self.game_dir(to), &skip, true)?;

        unbind_event(&dst.join("pack.lock.json"))
    }

    /// Löscht das Profil samt Spielordner (Welten, Mods, Einstellungen!).
    pub fn delete(&self, selector: &str) -> Result<Profile> {
        let profile = self.get(selector)?;
        let dir = self.paths.instance_dir(&profile.id);
        // Nur echte Profilordner löschen: ID ohne Pfadtrenner, mit profile.json.
        if profile.id.is_empty() || profile.id.contains(['/', '\\']) || profile.id.starts_with('.')
        {
            return Err(Error::Invalid(format!(
                "Ungültige Profil-ID {:?}",
                profile.id
            )));
        }
        std::fs::remove_dir_all(&dir).map_err(Error::io(&dir))?;
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::LoaderKind;

    #[test]
    fn slugs() {
        assert_eq!(slugify("Mein Fabric-Profil"), "mein-fabric-profil");
        assert_eq!(slugify("  Größte Welt!! "), "groesste-welt");
        assert_eq!(slugify("1.21.1"), "1-21-1");
        assert_eq!(slugify("../../etc"), "etc");
        assert_eq!(slugify("★★★"), "profil");
    }

    #[test]
    fn create_list_find_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = ProfileStore::new(Paths::new(dir.path()));
        assert!(store.list().unwrap().is_empty());

        let mut p = Profile::new("Fabric Test", "26.3");
        p.loader = Some(LoaderSpec {
            kind: LoaderKind::Fabric,
            version: "0.19.5".into(),
        });
        p.memory_mb = Some(6144);
        let created = store.create(p).unwrap();
        assert_eq!(created.id, "fabric-test");
        assert!(store.game_dir("fabric-test").is_dir());
        assert_eq!(created.version_id(), "fabric-loader-0.19.5-26.3");

        // Gleicher Name verboten, gleicher Slug bekommt eine Nummer.
        assert!(store.create(Profile::new("fabric test", "26.3")).is_err());
        let second = store.create(Profile::new("Fabric-Test", "1.8.9")).unwrap();
        assert_eq!(second.id, "fabric-test-2");

        let names: Vec<String> = store.list().unwrap().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["Fabric Test", "Fabric-Test"]);
        let loaded = store.get("FABRIC TEST").unwrap();
        assert_eq!(loaded.memory_mb, Some(6144));
        assert_eq!(loaded, created);

        let json =
            std::fs::read_to_string(dir.path().join("instances/fabric-test/profile.json")).unwrap();
        assert!(!json.contains("\"id\""), "ID steckt im Ordnernamen: {json}");
        assert!(json.contains("\"kind\": \"fabric\""));

        store.delete("fabric-test-2").unwrap();
        assert!(!dir.path().join("instances/fabric-test-2").exists());
        assert_eq!(store.list().unwrap().len(), 1);
        assert!(store.delete("gibt-es-nicht").is_err());
    }

    #[test]
    fn cleans_group_names() {
        assert_eq!(clean_group(None).unwrap(), None);
        assert_eq!(clean_group(Some("   ")).unwrap(), None);
        assert_eq!(
            clean_group(Some(" Modded ")).unwrap().as_deref(),
            Some("Modded")
        );
        assert!(clean_group(Some(&"x".repeat(33))).is_err());
    }

    #[test]
    fn rejects_bad_names() {
        let dir = tempfile::tempdir().unwrap();
        let store = ProfileStore::new(Paths::new(dir.path()));
        assert!(store.create(Profile::new("   ", "26.3")).is_err());
        assert!(store.create(Profile::new("x".repeat(65), "26.3")).is_err());
    }

    #[test]
    fn duplicates_without_worlds_and_event_binding() {
        use crate::content::{ContentKind, ContentLock, LockEntry, Source};
        use crate::modpack::{PackInfo, PackSource};
        let dir = tempfile::tempdir().unwrap();
        let store = ProfileStore::new(Paths::new(dir.path()));
        let mut p = Profile::new("Avalon", "1.21.11");
        p.memory_mb = Some(6144);
        let original = store.create(p).unwrap();
        let game = store.game_dir(&original.id);
        for (rel, data) in [
            ("mods/a.jar", "jar"),
            ("config/c.toml", "cfg"),
            ("options.txt", "fov:90"),
            ("saves/Welt/level.dat", "lvl"),
            ("screenshots/s.png", "png"),
            ("logs/latest.log", "log"),
        ] {
            let path = game.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, data).unwrap();
        }
        let inst = dir.path().join("instances").join(&original.id);
        std::fs::create_dir_all(inst.join("snapshots/1")).unwrap();
        std::fs::write(inst.join("icon.png"), "icon").unwrap();
        let mut lock = ContentLock::default();
        lock.files.insert(
            "mods/a.jar".into(),
            LockEntry {
                kind: ContentKind::Mod,
                source: Source::Local,
                name: "A".into(),
                version: None,
                sha1: "x".into(),
                size: 3,
                icon_url: None,
                from_pack: true,
                channel: None,
                sync: None,
            },
        );
        lock.pack = Some(PackInfo {
            name: "Avalon".into(),
            version: "6.4.0".into(),
            source: PackSource::Twogg {
                pack_id: "avalon".into(),
                version_id: "v1".into(),
            },
            files: Default::default(),
            event: None,
        });
        std::fs::write(
            inst.join("pack.lock.json"),
            serde_json::to_vec(&lock).unwrap(),
        )
        .unwrap();

        let copy = store
            .duplicate("Avalon", "Avalon (Kopie)", DuplicateOptions::default())
            .unwrap();
        assert_eq!(copy.id, "avalon-kopie");
        assert_eq!(copy.memory_mb, Some(6144));
        let copy_game = store.game_dir(&copy.id);
        assert!(copy_game.join("mods/a.jar").is_file());
        assert!(copy_game.join("config/c.toml").is_file());
        assert!(copy_game.join("options.txt").is_file());
        assert!(!copy_game.join("saves").exists(), "Welten nur auf Wunsch");
        assert!(!copy_game.join("screenshots").exists());
        assert!(!copy_game.join("logs").exists());
        let copy_inst = dir.path().join("instances").join(&copy.id);
        assert!(copy_inst.join("icon.png").is_file());
        assert!(!copy_inst.join("snapshots").exists());
        let lock: ContentLock =
            serde_json::from_slice(&std::fs::read(copy_inst.join("pack.lock.json")).unwrap())
                .unwrap();
        assert!(lock.pack.is_none(), "Event-Bindung entfällt");
        assert!(!lock.files["mods/a.jar"].from_pack);
        // Das Original bleibt unverändert.
        let orig: ContentLock =
            serde_json::from_slice(&std::fs::read(inst.join("pack.lock.json")).unwrap()).unwrap();
        assert!(orig.pack.is_some());

        let with_worlds = store
            .duplicate(
                "avalon",
                "Avalon 2",
                DuplicateOptions {
                    worlds: true,
                    screenshots: true,
                },
            )
            .unwrap();
        let g = store.game_dir(&with_worlds.id);
        assert!(g.join("saves/Welt/level.dat").is_file());
        assert!(g.join("screenshots/s.png").is_file());

        // Gleicher Name ist verboten und hinterlässt nichts.
        assert!(
            store
                .duplicate("avalon", "Avalon 2", DuplicateOptions::default())
                .is_err()
        );
        assert_eq!(store.list().unwrap().len(), 3);
    }
}
