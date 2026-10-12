//! Inhalte eines Profils: Mods, Ressourcenpakete und Shader.
//! Herkunft und Hash jeder Datei stehen in `instances/<id>/pack.lock.json`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::curseforge::{self, CurseForge};
use crate::download::{DownloadTask, Downloader, sha1_file};
use crate::loader::LoaderKind;
use crate::modrinth::{self, DependencyType, Modrinth, ProjectType, Version, VersionType};
use crate::paths::{Paths, safe_relative};
use crate::profiles::Profile;
use crate::{Error, Result};

pub const DISABLED_SUFFIX: &str = ".disabled";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    Mod,
    Resourcepack,
    Shader,
}

impl ContentKind {
    pub const ALL: [ContentKind; 3] = [
        ContentKind::Mod,
        ContentKind::Resourcepack,
        ContentKind::Shader,
    ];

    pub fn folder(self) -> &'static str {
        match self {
            ContentKind::Mod => "mods",
            ContentKind::Resourcepack => "resourcepacks",
            ContentKind::Shader => "shaderpacks",
        }
    }

    fn extensions(self) -> &'static [&'static str] {
        match self {
            ContentKind::Mod => &["jar"],
            ContentKind::Resourcepack | ContentKind::Shader => &["zip"],
        }
    }

    pub fn from_project_type(t: ProjectType) -> Option<Self> {
        match t {
            ProjectType::Mod => Some(ContentKind::Mod),
            ProjectType::Resourcepack => Some(ContentKind::Resourcepack),
            ProjectType::Shader => Some(ContentKind::Shader),
            ProjectType::Modpack | ProjectType::Datapack => None,
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        let folder = key.split('/').next()?;
        Self::ALL.into_iter().find(|k| k.folder() == folder)
    }

    /// Art einer hineingezogenen Datei: `.jar` ist eine Mod, bei `.zip` entscheidet der Inhalt
    /// (`shaders/` → Shader, `assets/` oder `pack.mcmeta` → Ressourcenpaket).
    pub fn detect(path: &Path) -> Result<Self> {
        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .map(str::to_ascii_lowercase);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        match ext.as_deref() {
            Some("jar") => return Ok(ContentKind::Mod),
            Some("zip") => {}
            _ => {
                return Err(Error::Invalid(format!(
                    "{name}: nur .jar (Mods) und .zip (Ressourcenpakete, Shader)"
                )));
            }
        }
        let file = std::fs::File::open(path).map_err(Error::io(path))?;
        let zip = zip::ZipArchive::new(file)
            .map_err(|e| Error::Invalid(format!("{name} ist kein ZIP-Archiv: {e}")))?;
        let names: Vec<&str> = zip.file_names().collect();
        let has = |prefix: &str| {
            names.iter().any(|n| {
                n.starts_with(prefix)
                    || n.split_once('/')
                        .is_some_and(|(_, r)| r.starts_with(prefix))
            })
        };
        if has("shaders/") {
            Ok(ContentKind::Shader)
        } else if has("assets/") {
            Ok(ContentKind::Resourcepack)
        } else if has("data/") {
            Err(Error::Invalid(format!(
                "{name} ist ein Datenpaket; installiere es im Tab Welten in eine Welt"
            )))
        } else if has("pack.mcmeta") {
            Ok(ContentKind::Resourcepack)
        } else {
            Err(Error::Invalid(format!(
                "{name}: weder Ressourcenpaket noch Shader erkannt"
            )))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Source {
    Modrinth {
        project_id: String,
        version_id: String,
    },
    Curseforge {
        project_id: u64,
        file_id: u64,
    },
    /// Eigene Datei oder bei keiner Plattform gefunden.
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockEntry {
    pub kind: ContentKind,
    pub source: Source,
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    pub sha1: String,
    pub size: u64,
    #[serde(default)]
    pub icon_url: Option<String>,
    /// Gehört zu einem installierten Modpack (für Pack-Updates).
    #[serde(default)]
    pub from_pack: bool,
    /// Kanal der installierten Version; Updates werden nicht instabiler (Release → Alpha).
    #[serde(default)]
    pub channel: Option<VersionType>,
    /// Kommt aus diesem Sync-Pack (wird beim Ausschalten wieder entfernt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentLock {
    /// Schlüssel: Pfad relativ zum Spielordner ohne `.disabled`, z. B. `mods/sodium.jar`.
    #[serde(default)]
    pub files: BTreeMap<String, LockEntry>,
    /// Installiertes Modpack, falls das Profil aus einem stammt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack: Option<crate::modpack::PackInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledContent {
    pub key: String,
    pub file_name: String,
    pub kind: ContentKind,
    pub enabled: bool,
    pub name: String,
    pub version: Option<String>,
    pub source: Source,
    pub icon_url: Option<String>,
    pub from_pack: bool,
    /// ID des Sync-Packs, aus dem der Inhalt stammt.
    pub sync: Option<String>,
    pub size: u64,
    /// Noch nicht per Hash geprüft (`identify` aufrufen).
    pub unidentified: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    /// Neu installiert oder aktualisiert (Projektname, Version).
    pub installed: Vec<(String, String)>,
    /// Abhängigkeiten, die schon vorhanden waren.
    pub already_present: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub key: String,
    pub name: String,
    pub current: Option<String>,
    pub latest: String,
    pub project_id: String,
    pub version_id: String,
    pub kind: ContentKind,
    /// `modrinth` oder `curseforge`.
    #[serde(default = "default_platform")]
    pub platform: String,
}

fn default_platform() -> String {
    "modrinth".into()
}

/// Modrinth-Loader-Filter je Inhaltsart und Profil. `None`: Inhaltsart passt nicht zum Profil.
pub fn loaders_for(profile: &Profile, kind: ContentKind) -> Option<Vec<String>> {
    match kind {
        ContentKind::Mod => match profile.loader.as_ref().map(|l| l.kind) {
            Some(LoaderKind::Fabric) => Some(vec!["fabric".into()]),
            // Quilt lädt auch Fabric-Mods.
            Some(LoaderKind::Quilt) => Some(vec!["quilt".into(), "fabric".into()]),
            Some(LoaderKind::Forge) => Some(vec!["forge".into()]),
            // NeoForge lädt viele Forge-Mods (bis 1.20.x) weiterhin.
            Some(LoaderKind::NeoForge) => Some(vec!["neoforge".into()]),
            None => None,
        },
        ContentKind::Resourcepack => Some(vec!["minecraft".into()]),
        // Shader-Loader (Iris, OptiFine, …) sind unterschiedlich getaggt; nur nach Version filtern.
        ContentKind::Shader => Some(Vec::new()),
    }
}

fn not_moddable(profile: &Profile) -> Error {
    Error::Invalid(format!(
        "{:?} ist ein Vanilla-Profil. Für Mods bitte Fabric oder Quilt wählen.",
        profile.name
    ))
}

#[derive(Clone)]
pub struct ContentManager {
    paths: Paths,
    downloader: Downloader,
    modrinth: Modrinth,
    curseforge: Option<CurseForge>,
}

fn curseforge_missing() -> Error {
    Error::Invalid("CurseForge ist nicht verfügbar".into())
}

/// CurseForge-Freigabestufe (1 Release, 2 Beta, 3 Alpha) als Kanal.
fn cf_channel(release_type: u32) -> VersionType {
    match release_type {
        1 => VersionType::Release,
        2 => VersionType::Beta,
        _ => VersionType::Alpha,
    }
}

impl ContentManager {
    pub fn new(paths: Paths, downloader: Downloader, modrinth: Modrinth) -> Self {
        Self {
            paths,
            downloader,
            modrinth,
            curseforge: None,
        }
    }

    pub fn with_curseforge(mut self, cf: Option<CurseForge>) -> Self {
        self.curseforge = cf;
        self
    }

    pub fn modrinth(&self) -> &Modrinth {
        &self.modrinth
    }

    /// Lädt ein Datenpaket von Modrinth in `saves/<welt>/datapacks` und gibt den Dateinamen zurück.
    /// Ohne `version` die neueste stabile Version für die Minecraft-Version der Instanz.
    pub async fn install_datapack(
        &self,
        game_dir: &Path,
        world: &str,
        project: &str,
        version: Option<&str>,
        minecraft: &str,
    ) -> Result<String> {
        if !crate::worlds::valid_folder(world)
            || !game_dir
                .join("saves")
                .join(world)
                .join("level.dat")
                .is_file()
        {
            return Err(Error::Invalid(format!("Welt {world:?} nicht gefunden")));
        }
        let chosen = match version {
            Some(id) => self.modrinth.version(id).await?,
            None => {
                let versions = self
                    .modrinth
                    .versions(project, &["datapack".into()], &[minecraft.into()])
                    .await?;
                crate::modrinth::best_version(&versions)
                    .cloned()
                    .ok_or_else(|| {
                        Error::Invalid(format!("Kein Datenpaket für Minecraft {minecraft}"))
                    })?
            }
        };
        let file = chosen.primary_file().ok_or_else(|| {
            Error::Invalid(format!("Version {} hat keine Datei", chosen.version_number))
        })?;
        let name = file.filename.replace(['/', '\\'], "_");
        if !name.ends_with(".zip") {
            return Err(Error::Invalid(format!(
                "{name} ist kein Datenpaket (ZIP erwartet)"
            )));
        }
        let dest = game_dir
            .join("saves")
            .join(world)
            .join("datapacks")
            .join(&name);
        self.downloader
            .download(&crate::download::DownloadTask {
                url: file.url.clone(),
                dest,
                sha1: Some(file.hashes.sha1.clone()),
                size: Some(file.size),
            })
            .await?;
        Ok(name)
    }

    pub fn curseforge(&self) -> Option<&CurseForge> {
        self.curseforge.as_ref()
    }

    fn lock_path(&self, profile_id: &str) -> PathBuf {
        self.paths.instance_dir(profile_id).join("pack.lock.json")
    }

    pub fn load_lock(&self, profile_id: &str) -> Result<ContentLock> {
        let path = self.lock_path(profile_id);
        match std::fs::read(&path) {
            Ok(b) => serde_json::from_slice(&b).map_err(Error::json(path.display().to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ContentLock::default()),
            Err(e) => Err(Error::io(&path)(e)),
        }
    }

    pub fn save_lock(&self, profile_id: &str, lock: &ContentLock) -> Result<()> {
        let path = self.lock_path(profile_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
        }
        let json = serde_json::to_vec_pretty(lock).map_err(Error::json("pack.lock.json"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(Error::io(&tmp))?;
        std::fs::rename(&tmp, &path).map_err(Error::io(&path))
    }

    /// Dateien im Spielordner: (Schlüssel, Dateiname, Art, aktiv, Größe).
    fn scan(game_dir: &Path) -> Vec<(String, String, ContentKind, bool, u64)> {
        let mut found = Vec::new();
        for kind in ContentKind::ALL {
            let Ok(entries) = std::fs::read_dir(game_dir.join(kind.folder())) else {
                continue;
            };
            for e in entries.flatten() {
                let Ok(meta) = e.metadata() else { continue };
                if !meta.is_file() {
                    continue;
                }
                let file_name = e.file_name().to_string_lossy().into_owned();
                let (base, enabled) = match file_name.strip_suffix(DISABLED_SUFFIX) {
                    Some(b) => (b.to_string(), false),
                    None => (file_name.clone(), true),
                };
                let ext_ok = Path::new(&base)
                    .extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| kind.extensions().iter().any(|e| e.eq_ignore_ascii_case(x)));
                if ext_ok {
                    found.push((
                        format!("{}/{base}", kind.folder()),
                        file_name,
                        kind,
                        enabled,
                        meta.len(),
                    ));
                }
            }
        }
        found.sort_by_key(|f| f.0.to_lowercase());
        found
    }

    /// Installierte Inhalte, ohne Netzwerk. Unbekannte Dateien sind als `unidentified` markiert.
    pub fn list(&self, profile_id: &str, game_dir: &Path) -> Result<Vec<InstalledContent>> {
        let lock = self.load_lock(profile_id)?;
        Ok(Self::scan(game_dir)
            .into_iter()
            .map(|(key, file_name, kind, enabled, size)| {
                match lock.files.get(&key).filter(|e| e.size == size) {
                    Some(e) => InstalledContent {
                        key,
                        file_name,
                        kind,
                        enabled,
                        name: e.name.clone(),
                        version: e.version.clone(),
                        source: e.source.clone(),
                        icon_url: e.icon_url.clone(),
                        from_pack: e.from_pack,
                        sync: e.sync.clone(),
                        size,
                        unidentified: false,
                    },
                    None => InstalledContent {
                        name: file_stem(&key),
                        key,
                        file_name,
                        kind,
                        enabled,
                        version: None,
                        source: Source::Local,
                        icon_url: None,
                        from_pack: false,
                        sync: None,
                        size,
                        unidentified: true,
                    },
                }
            })
            .collect())
    }

    /// Ordnet unbekannte Dateien per SHA-1 Modrinth zu und räumt verwaiste Einträge auf.
    pub async fn identify(
        &self,
        profile_id: &str,
        game_dir: &Path,
    ) -> Result<Vec<InstalledContent>> {
        let mut lock = self.load_lock(profile_id)?;
        let present = Self::scan(game_dir);
        let keys: HashSet<&str> = present.iter().map(|p| p.0.as_str()).collect();
        lock.files.retain(|k, _| keys.contains(k.as_str()));

        let mut unknown: Vec<(String, String, ContentKind, u64, String)> = Vec::new();
        for (key, file_name, kind, _, size) in &present {
            if lock.files.get(key).is_some_and(|e| e.size == *size) {
                continue;
            }
            let path = game_dir.join(kind.folder()).join(file_name);
            unknown.push((
                key.clone(),
                file_name.clone(),
                *kind,
                *size,
                sha1_file(&path).await?,
            ));
        }
        if !unknown.is_empty() {
            let hashes: Vec<String> = unknown.iter().map(|u| u.4.clone()).collect();
            let found = self
                .modrinth
                .versions_by_hash(&hashes)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!("Modrinth-Abgleich fehlgeschlagen: {e}");
                    HashMap::new()
                });
            let ids: Vec<String> = found
                .values()
                .map(|v| v.project_id.clone())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            let projects: HashMap<String, modrinth::Project> = self
                .modrinth
                .projects(&ids)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|p| (p.id.clone(), p))
                .collect();
            for (key, _, kind, size, sha1) in unknown {
                let entry = match found.get(&sha1) {
                    Some(v) => LockEntry {
                        kind,
                        source: Source::Modrinth {
                            project_id: v.project_id.clone(),
                            version_id: v.id.clone(),
                        },
                        name: projects
                            .get(&v.project_id)
                            .map(|p| p.title.clone())
                            .unwrap_or_else(|| v.name.clone()),
                        version: Some(v.version_number.clone()),
                        sha1,
                        size,
                        icon_url: projects.get(&v.project_id).and_then(|p| p.icon_url.clone()),
                        from_pack: false,
                        channel: Some(v.version_type),
                        sync: None,
                    },
                    None => LockEntry {
                        kind,
                        source: Source::Local,
                        name: file_stem(&key),
                        version: None,
                        sha1,
                        size,
                        icon_url: None,
                        from_pack: false,
                        channel: None,
                        sync: None,
                    },
                };
                lock.files.insert(key, entry);
            }
        }
        self.save_lock(profile_id, &lock)?;
        self.list(profile_id, game_dir)
    }

    /// Prüft einen Schlüssel aus der Oberfläche: muss in einem Inhaltsordner liegen.
    fn checked_key(key: &str) -> Result<ContentKind> {
        safe_relative(key)?;
        let kind = ContentKind::from_key(key)
            .ok_or_else(|| Error::Invalid(format!("Kein Inhaltspfad: {key}")))?;
        if key.matches('/').count() != 1 {
            return Err(Error::Invalid(format!("Kein Inhaltspfad: {key}")));
        }
        Ok(kind)
    }

    pub fn set_enabled(&self, game_dir: &Path, key: &str, enabled: bool) -> Result<()> {
        Self::checked_key(key)?;
        let on = game_dir.join(key);
        let off = game_dir.join(format!("{key}{DISABLED_SUFFIX}"));
        let (from, to) = if enabled { (off, on) } else { (on, off) };
        if from.exists() {
            std::fs::rename(&from, &to).map_err(Error::io(&from))?;
        }
        Ok(())
    }

    pub fn remove(&self, profile_id: &str, game_dir: &Path, key: &str) -> Result<()> {
        Self::checked_key(key)?;
        for path in [
            game_dir.join(key),
            game_dir.join(format!("{key}{DISABLED_SUFFIX}")),
        ] {
            if path.exists() {
                std::fs::remove_file(&path).map_err(Error::io(&path))?;
            }
        }
        let mut lock = self.load_lock(profile_id)?;
        lock.files.remove(key);
        self.save_lock(profile_id, &lock)
    }

    /// Eigene Datei (z. B. eine selbst gebaute Mod) ins Profil kopieren.
    pub async fn add_local(
        &self,
        profile_id: &str,
        game_dir: &Path,
        kind: ContentKind,
        source: &Path,
    ) -> Result<String> {
        let file_name = source
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::Invalid("Ungültiger Dateiname".into()))?
            .to_string();
        let ext_ok = Path::new(&file_name)
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| kind.extensions().iter().any(|e| e.eq_ignore_ascii_case(x)));
        if !ext_ok {
            return Err(Error::Invalid(format!(
                "{file_name} ist keine {}-Datei",
                kind.extensions().join("/")
            )));
        }
        let key = format!("{}/{file_name}", kind.folder());
        Self::checked_key(&key)?;
        let target = game_dir.join(&key);
        std::fs::create_dir_all(target.parent().unwrap()).map_err(Error::io(&target))?;
        std::fs::copy(source, &target).map_err(Error::io(&target))?;
        // Herkunft per Hash nachschlagen, z. B. wenn die Datei von Modrinth stammt.
        self.identify(profile_id, game_dir).await?;
        Ok(key)
    }

    /// Passende Version eines Projekts für das Profil.
    async fn resolve_version(
        &self,
        profile: &Profile,
        kind: ContentKind,
        project: &str,
    ) -> Result<Option<Version>> {
        let loaders = loaders_for(profile, kind).ok_or_else(|| not_moddable(profile))?;
        let versions = self
            .modrinth
            .versions(project, &loaders, std::slice::from_ref(&profile.minecraft))
            .await?;
        Ok(modrinth::best_version(&versions).cloned())
    }

    /// Lädt eine Datei in den Inhaltsordner und ersetzt eine ältere Version desselben Projekts
    /// (`existing`). War die alte deaktiviert, wird die neue ebenfalls deaktiviert abgelegt.
    #[allow(clippy::too_many_arguments)]
    async fn place_file(
        &self,
        lock: &mut ContentLock,
        game_dir: &Path,
        kind: ContentKind,
        file_name: &str,
        url: &str,
        sha1: Option<&str>,
        size: u64,
        existing: Option<String>,
    ) -> Result<String> {
        let safe_name = safe_relative(file_name)?
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| *n == file_name)
            .ok_or_else(|| Error::Invalid(format!("Unsicherer Dateiname {file_name}")))?
            .to_string();
        let key = format!("{}/{safe_name}", kind.folder());
        let was_disabled = existing
            .as_ref()
            .is_some_and(|k| game_dir.join(format!("{k}{DISABLED_SUFFIX}")).exists());
        let dest = if was_disabled {
            game_dir.join(format!("{key}{DISABLED_SUFFIX}"))
        } else {
            game_dir.join(&key)
        };
        self.downloader
            .download(&DownloadTask {
                url: url.to_string(),
                dest,
                sha1: sha1.map(str::to_string),
                size: Some(size),
            })
            .await?;
        if let Some(old) = existing.filter(|old| *old != key) {
            for p in [
                game_dir.join(&old),
                game_dir.join(format!("{old}{DISABLED_SUFFIX}")),
            ] {
                let _ = std::fs::remove_file(p);
            }
            lock.files.remove(&old);
        }
        Ok(key)
    }

    /// Wie `install`, aber von CurseForge. Gesperrte Downloads werden mit Link gemeldet.
    pub async fn install_curseforge(
        &self,
        profile: &Profile,
        game_dir: &Path,
        kind: ContentKind,
        mod_id: u64,
        file_id: Option<u64>,
    ) -> Result<InstallReport> {
        let cf = self.curseforge.as_ref().ok_or_else(curseforge_missing)?;
        if loaders_for(profile, kind).is_none() {
            return Err(not_moddable(profile));
        }
        let mut lock = self.load_lock(&profile.id)?;
        let present: HashSet<String> = Self::scan(game_dir).into_iter().map(|p| p.0).collect();
        let mut by_project: HashMap<u64, String> = lock
            .files
            .iter()
            .filter(|(k, _)| present.contains(*k))
            .filter_map(|(k, e)| match &e.source {
                Source::Curseforge { project_id, .. } => Some((*project_id, k.clone())),
                _ => None,
            })
            .collect();
        let mut report = InstallReport::default();
        let mut queue = vec![(mod_id, file_id, kind, true)];
        let mut done = HashSet::new();
        let mut new_ids = Vec::new();

        while let Some((mod_id, file_id, kind, root)) = queue.pop() {
            if !done.insert(mod_id) {
                continue;
            }
            let loader = (kind == ContentKind::Mod)
                .then(|| profile.loader.as_ref().map(|l| l.kind))
                .flatten();
            let file = match file_id {
                Some(id) => cf.files_by_ids(&[id]).await?.into_iter().next(),
                None => {
                    let files = cf.files(mod_id, Some(&profile.minecraft), loader).await?;
                    curseforge::best_file(&files).cloned()
                }
            };
            let Some(file) = file else {
                if root {
                    return Err(Error::Invalid(format!(
                        "Keine Datei dieses CurseForge-Projekts passt zu Minecraft {}",
                        profile.minecraft
                    )));
                }
                report.warnings.push(format!(
                    "CurseForge-Abhängigkeit {mod_id} gibt es nicht für Minecraft {}",
                    profile.minecraft
                ));
                continue;
            };
            let existing = by_project.get(&mod_id).cloned();
            if let Some(key) = &existing {
                let same = matches!(&lock.files[key].source, Source::Curseforge { file_id, .. } if *file_id == file.id);
                if !root || same {
                    report.already_present.push(lock.files[key].name.clone());
                    continue;
                }
            }
            let Some(url) = file.download_url.clone() else {
                let info = cf.mods_by_ids(&[mod_id]).await?.into_iter().next();
                let link = info
                    .as_ref()
                    .map(|m| curseforge::manual_download_url(m, file.id))
                    .unwrap_or_default();
                let name = info
                    .map(|m| m.name)
                    .unwrap_or_else(|| file.display_name.clone());
                let msg = format!(
                    "{name}: Der Autor erlaubt keine Downloads über andere Launcher. Bitte manuell herunterladen: {link}"
                );
                if root {
                    return Err(Error::Invalid(msg));
                }
                report.warnings.push(msg);
                continue;
            };
            // Herkunft der ersetzten Datei (Pack, Sync-Pack) bleibt erhalten.
            let (from_pack, sync) = previous(&lock, existing.as_deref());
            let key = self
                .place_file(
                    &mut lock,
                    game_dir,
                    kind,
                    &file.file_name,
                    &url,
                    file.sha1(),
                    file.file_length,
                    existing,
                )
                .await?;
            lock.files.insert(
                key.clone(),
                LockEntry {
                    kind,
                    source: Source::Curseforge {
                        project_id: mod_id,
                        file_id: file.id,
                    },
                    name: file.display_name.clone(),
                    version: Some(file.display_name.clone()),
                    sha1: match file.sha1() {
                        Some(h) => h.to_string(),
                        None => sha1_file(&game_dir.join(&key)).await?,
                    },
                    size: file.file_length,
                    icon_url: None,
                    from_pack,
                    channel: Some(cf_channel(file.release_type)),
                    sync,
                },
            );
            by_project.insert(mod_id, key);
            new_ids.push(mod_id);
            report
                .installed
                .push((mod_id.to_string(), file.display_name.clone()));
            for dep in &file.dependencies {
                match dep.relation_type {
                    3 => queue.push((dep.mod_id, None, ContentKind::Mod, false)),
                    5 if by_project.contains_key(&dep.mod_id) => {
                        let k = &by_project[&dep.mod_id];
                        report.warnings.push(format!(
                            "{} ist laut CurseForge inkompatibel mit {}",
                            lock.files[k].name, file.display_name
                        ));
                    }
                    _ => {}
                }
            }
        }

        let mods = cf.mods_by_ids(&new_ids).await.unwrap_or_default();
        let titles: HashMap<u64, &curseforge::Mod> = mods.iter().map(|m| (m.id, m)).collect();
        for entry in lock.files.values_mut() {
            if let Source::Curseforge { project_id, .. } = &entry.source
                && let Some(m) = titles.get(project_id)
            {
                entry.name = m.name.clone();
                entry.icon_url = m.logo.as_ref().and_then(|l| l.thumbnail_url.clone());
            }
        }
        for (project, _) in report.installed.iter_mut() {
            if let Some(m) = project.parse::<u64>().ok().and_then(|id| titles.get(&id)) {
                *project = m.name.clone();
            }
        }
        self.save_lock(&profile.id, &lock)?;
        Ok(report)
    }

    /// Installiert ein Projekt (bestimmte oder passende Version) samt Pflicht-Abhängigkeiten.
    /// Eine ältere Version desselben Projekts wird ersetzt, ihr Aktiv-Status bleibt erhalten.
    pub async fn install(
        &self,
        profile: &Profile,
        game_dir: &Path,
        kind: ContentKind,
        project: &str,
        version_id: Option<&str>,
    ) -> Result<InstallReport> {
        if loaders_for(profile, kind).is_none() {
            return Err(not_moddable(profile));
        }
        let mut lock = self.load_lock(&profile.id)?;
        let present: HashSet<String> = Self::scan(game_dir).into_iter().map(|p| p.0).collect();
        // Projekt → vorhandener Schlüssel (nur Dateien, die es noch gibt).
        let mut by_project: HashMap<String, String> = lock
            .files
            .iter()
            .filter(|(k, _)| present.contains(*k))
            .filter_map(|(k, e)| match &e.source {
                Source::Modrinth { project_id, .. } => Some((project_id.clone(), k.clone())),
                _ => None,
            })
            .collect();

        let mut report = InstallReport::default();
        let mut queue: Vec<(String, Option<String>, ContentKind, bool)> = vec![(
            project.to_string(),
            version_id.map(str::to_string),
            kind,
            true,
        )];
        let mut done: HashSet<String> = HashSet::new();
        let mut new_projects = Vec::new();

        while let Some((project, version_id, kind, root)) = queue.pop() {
            let version = match version_id {
                Some(id) => Some(self.modrinth.version(&id).await?),
                None => self.resolve_version(profile, kind, &project).await?,
            };
            let Some(version) = version else {
                if root {
                    return Err(Error::Invalid(format!(
                        "Keine Version von {project} passt zu Minecraft {} mit diesem Profil",
                        profile.minecraft
                    )));
                }
                report.warnings.push(format!(
                    "Abhängigkeit {project} gibt es nicht für Minecraft {}",
                    profile.minecraft
                ));
                continue;
            };
            if !done.insert(version.project_id.clone()) {
                continue;
            }
            let existing = by_project.get(&version.project_id).cloned();
            if let Some(key) = &existing {
                let same = matches!(&lock.files[key].source, Source::Modrinth { version_id, .. } if *version_id == version.id);
                if !root || same {
                    report.already_present.push(lock.files[key].name.clone());
                    continue;
                }
            }

            let file = version.primary_file().ok_or_else(|| {
                Error::Invalid(format!(
                    "Version {} hat keine Datei",
                    version.version_number
                ))
            })?;
            let (from_pack, sync) = previous(&lock, existing.as_deref());
            let key = self
                .place_file(
                    &mut lock,
                    game_dir,
                    kind,
                    &file.filename,
                    &file.url,
                    Some(&file.hashes.sha1),
                    file.size,
                    existing,
                )
                .await?;
            lock.files.insert(
                key.clone(),
                LockEntry {
                    kind,
                    source: Source::Modrinth {
                        project_id: version.project_id.clone(),
                        version_id: version.id.clone(),
                    },
                    name: version.name.clone(),
                    version: Some(version.version_number.clone()),
                    sha1: file.hashes.sha1.clone(),
                    size: file.size,
                    icon_url: None,
                    from_pack,
                    channel: Some(version.version_type),
                    sync,
                },
            );
            by_project.insert(version.project_id.clone(), key);
            new_projects.push(version.project_id.clone());
            report
                .installed
                .push((version.project_id.clone(), version.version_number.clone()));

            for dep in &version.dependencies {
                match dep.dependency_type {
                    DependencyType::Required => {
                        let dep_project = match (&dep.project_id, &dep.version_id) {
                            (Some(p), v) => Some((p.clone(), v.clone())),
                            (None, Some(v)) => {
                                Some((self.modrinth.version(v).await?.project_id, Some(v.clone())))
                            }
                            (None, None) => None,
                        };
                        if let Some((p, v)) = dep_project {
                            // Abhängigkeiten sind Mods (Shader/Ressourcenpakete haben praktisch keine).
                            queue.push((p, v, ContentKind::Mod, false));
                        }
                    }
                    DependencyType::Incompatible => {
                        if let Some(p) = &dep.project_id
                            && let Some(k) = by_project.get(p)
                        {
                            report.warnings.push(format!(
                                "{} ist laut Modrinth inkompatibel mit {}",
                                lock.files[k].name, version.name
                            ));
                        }
                    }
                    DependencyType::Optional | DependencyType::Embedded => {}
                }
            }
        }

        // Projektnamen und Icons nachtragen (eine Anfrage für alle).
        let projects = self
            .modrinth
            .projects(&new_projects)
            .await
            .unwrap_or_default();
        let titles: HashMap<&str, &modrinth::Project> =
            projects.iter().map(|p| (p.id.as_str(), p)).collect();
        for entry in lock.files.values_mut() {
            if let Source::Modrinth { project_id, .. } = &entry.source
                && let Some(p) = titles.get(project_id.as_str())
            {
                entry.name = p.title.clone();
                entry.icon_url = p.icon_url.clone();
            }
        }
        for (project, _) in report.installed.iter_mut() {
            if let Some(p) = titles.get(project.as_str()) {
                *project = p.title.clone();
            }
        }
        self.save_lock(&profile.id, &lock)?;
        Ok(report)
    }

    /// Verfügbare Updates für Modrinth-Inhalte (eine Anfrage je Inhaltsart).
    pub async fn check_updates(
        &self,
        profile: &Profile,
        game_dir: &Path,
    ) -> Result<Vec<UpdateInfo>> {
        let installed = self.list(&profile.id, game_dir)?;
        let lock = self.load_lock(&profile.id)?;
        let mut updates = Vec::new();
        for kind in ContentKind::ALL {
            let Some(loaders) = loaders_for(profile, kind) else {
                continue;
            };
            let entries: Vec<(&InstalledContent, &LockEntry)> = installed
                .iter()
                .filter(|c| c.kind == kind && matches!(c.source, Source::Modrinth { .. }))
                .filter_map(|c| lock.files.get(&c.key).map(|e| (c, e)))
                .collect();
            if entries.is_empty() {
                continue;
            }
            let hashes: Vec<String> = entries.iter().map(|(_, e)| e.sha1.clone()).collect();
            let latest = self
                .modrinth
                .latest_by_hash(&hashes, &loaders, std::slice::from_ref(&profile.minecraft))
                .await?;
            for (c, e) in entries {
                let (Some(v), Source::Modrinth { version_id, .. }) =
                    (latest.get(&e.sha1), &e.source)
                else {
                    continue;
                };
                // Unbekannter Kanal gilt als Release: dann nur stabile Updates anbieten.
                let channel = e.channel.unwrap_or(VersionType::Release);
                if v.id != *version_id && v.version_type >= channel {
                    updates.push(UpdateInfo {
                        key: c.key.clone(),
                        name: c.name.clone(),
                        current: c.version.clone(),
                        latest: v.version_number.clone(),
                        project_id: v.project_id.clone(),
                        version_id: v.id.clone(),
                        kind,
                        platform: default_platform(),
                    });
                }
            }
        }

        // CurseForge: eine Anfrage je Projekt (die API kennt keinen Sammelabgleich per Hash).
        if let Some(cf) = &self.curseforge {
            for c in &installed {
                let (
                    Source::Curseforge {
                        project_id,
                        file_id,
                    },
                    Some(e),
                ) = (&c.source, lock.files.get(&c.key))
                else {
                    continue;
                };
                let loader = (c.kind == ContentKind::Mod)
                    .then(|| profile.loader.as_ref().map(|l| l.kind))
                    .flatten();
                let files = match cf
                    .files(*project_id, Some(&profile.minecraft), loader)
                    .await
                {
                    Ok(f) => f,
                    Err(err) => {
                        tracing::warn!("CurseForge-Update für {} nicht prüfbar: {err}", c.name);
                        continue;
                    }
                };
                let channel = e.channel.unwrap_or(VersionType::Release);
                let candidates: Vec<curseforge::File> = files
                    .into_iter()
                    .filter(|f| cf_channel(f.release_type) >= channel)
                    .collect();
                if let Some(best) = curseforge::best_file(&candidates)
                    && best.id != *file_id
                    && best.download_url.is_some()
                {
                    updates.push(UpdateInfo {
                        key: c.key.clone(),
                        name: c.name.clone(),
                        current: c.version.clone(),
                        latest: best.display_name.clone(),
                        project_id: project_id.to_string(),
                        version_id: best.id.to_string(),
                        kind: c.kind,
                        platform: "curseforge".into(),
                    });
                }
            }
        }
        Ok(updates)
    }

    pub async fn apply_update(
        &self,
        profile: &Profile,
        game_dir: &Path,
        update: &UpdateInfo,
    ) -> Result<InstallReport> {
        if update.platform == "curseforge" {
            let parse = |s: &str| {
                s.parse::<u64>()
                    .map_err(|_| Error::Invalid(format!("Ungültige CurseForge-ID {s}")))
            };
            return self
                .install_curseforge(
                    profile,
                    game_dir,
                    update.kind,
                    parse(&update.project_id)?,
                    Some(parse(&update.version_id)?),
                )
                .await;
        }
        self.install(
            profile,
            game_dir,
            update.kind,
            &update.project_id,
            Some(&update.version_id),
        )
        .await
    }
}

/// Herkunft (Pack, Sync-Pack) eines Eintrags, der gerade ersetzt wird.
fn previous(lock: &ContentLock, key: Option<&str>) -> (bool, Option<String>) {
    key.and_then(|k| lock.files.get(k))
        .map(|e| (e.from_pack, e.sync.clone()))
        .unwrap_or_default()
}

fn file_stem(key: &str) -> String {
    let name = key.rsplit('/').next().unwrap_or(key);
    name.rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(name)
        .to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn detects_dropped_files() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let make = |name: &str, entries: &[&str]| {
            let path = dir.path().join(name);
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            for e in entries {
                zip.start_file(*e, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(b"x").unwrap();
            }
            zip.finish().unwrap();
            path
        };
        assert_eq!(
            ContentKind::detect(Path::new("x/sodium.jar")).unwrap(),
            ContentKind::Mod
        );
        let shader = make("bsl.zip", &["BSL/shaders/final.fsh"]);
        assert_eq!(ContentKind::detect(&shader).unwrap(), ContentKind::Shader);
        let pack = make("faithful.zip", &["pack.mcmeta", "assets/minecraft/x.png"]);
        assert_eq!(
            ContentKind::detect(&pack).unwrap(),
            ContentKind::Resourcepack
        );
        let data = make("dp.zip", &["pack.mcmeta", "data/x/function/a.mcfunction"]);
        assert!(ContentKind::detect(&data).is_err());
        assert!(ContentKind::detect(Path::new("a.exe")).is_err());
    }

    use super::*;

    #[test]
    fn keys_and_stems() {
        assert_eq!(
            ContentManager::checked_key("mods/sodium.jar").unwrap(),
            ContentKind::Mod
        );
        assert_eq!(
            ContentManager::checked_key("shaderpacks/bsl.zip").unwrap(),
            ContentKind::Shader
        );
        assert!(ContentManager::checked_key("../mods/x.jar").is_err());
        assert!(ContentManager::checked_key("saves/welt/level.dat").is_err());
        assert!(ContentManager::checked_key("mods/sub/x.jar").is_err());
        assert_eq!(
            file_stem("mods/fabric-api-0.162.0+26.3.jar"),
            "fabric-api-0.162.0+26.3"
        );
    }

    #[test]
    fn loader_filters() {
        let mut p = Profile::new("x", "1.21.1");
        assert_eq!(loaders_for(&p, ContentKind::Mod), None);
        assert_eq!(
            loaders_for(&p, ContentKind::Resourcepack),
            Some(vec!["minecraft".to_string()])
        );
        p.loader = Some(crate::loader::LoaderSpec {
            kind: LoaderKind::Quilt,
            version: "0.30.1".into(),
        });
        assert_eq!(
            loaders_for(&p, ContentKind::Mod),
            Some(vec!["quilt".to_string(), "fabric".to_string()])
        );
    }

    #[test]
    fn scans_enabled_and_disabled() {
        let dir = tempfile::tempdir().unwrap();
        for f in [
            "mods/a.jar",
            "mods/b.jar.disabled",
            "mods/notiz.txt",
            "resourcepacks/faithful.zip",
            "shaderpacks/bsl.zip.disabled",
        ] {
            let p = dir.path().join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        std::fs::create_dir_all(dir.path().join("mods/ordner")).unwrap();
        let found: Vec<(String, bool)> = ContentManager::scan(dir.path())
            .into_iter()
            .map(|f| (f.0, f.3))
            .collect();
        assert_eq!(
            found,
            [
                ("mods/a.jar".to_string(), true),
                ("mods/b.jar".to_string(), false),
                ("resourcepacks/faithful.zip".to_string(), true),
                ("shaderpacks/bsl.zip".to_string(), false),
            ]
        );
    }
}
