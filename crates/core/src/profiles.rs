//! Profile (Instanzen): je ein Ordner `instances/<id>/` mit `profile.json` und eigenem `.minecraft`.

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
    /// Geänderte Welten vor jedem Start automatisch sichern.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub backup_on_launch: bool,
    /// So viele automatische Backups je Welt behalten; ohne Angabe [`DEFAULT_BACKUP_KEEP`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_keep: Option<u32>,
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
    fn rejects_bad_names() {
        let dir = tempfile::tempdir().unwrap();
        let store = ProfileStore::new(Paths::new(dir.path()));
        assert!(store.create(Profile::new("   ", "26.3")).is_err());
        assert!(store.create(Profile::new("x".repeat(65), "26.3")).is_err());
    }
}
