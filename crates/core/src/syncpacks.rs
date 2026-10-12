//! Sync-Packs und Vorlagen: eigene Sammlungen von Inhalten, die man Profilen zuschaltet.
//!
//! Ein Sync-Pack nennt Projekte (Modrinth/CurseForge) ohne feste Version; für jedes Profil wird
//! die zur Minecraft-Version und zum Loader passende Datei gesucht. Dazu kommen eigene Dateien
//! (z. B. Configs) und Standardwerte für `options.txt`.
//!
//! - **Sync-Pack**: bleibt verbunden. Vor jedem Start wird abgeglichen; Ausschalten entfernt genau
//!   die Inhalte, die es gebracht hat (im `pack.lock.json` mit `sync: <id>` markiert).
//! - **Vorlage**: wird einmal eingespielt und ist danach normaler Profil-Inhalt.
//!
//! Bringt ein Modpack oder der Nutzer dasselbe Projekt schon mit, gewinnt deren Fassung.
//! Eigene Dateien und Optionen (inklusive Steuerung, `key_…`) werden beim ersten Einspielen
//! gesetzt; danach nur noch, solange der Nutzer sie in der Instanz nicht geändert hat.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::content::{
    ContentKind, ContentLock, ContentManager, DISABLED_SUFFIX, Source, loaders_for,
};
use crate::download::sha1_file;
use crate::loader::LoaderKind;
use crate::paths::{Paths, safe_relative};
use crate::profiles::{Profile, ProfileStore, slugify};
use crate::{Error, Result};

const PACK_FILE: &str = "syncpack.json";
const STATE_FILE: &str = "syncpacks.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncMode {
    #[default]
    Sync,
    Template,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SyncEntry {
    #[serde(rename_all = "camelCase")]
    Modrinth {
        project_id: String,
        kind: ContentKind,
        name: String,
    },
    #[serde(rename_all = "camelCase")]
    Curseforge {
        project_id: u64,
        kind: ContentKind,
        name: String,
    },
    /// Eigene Datei, liegt im Sync-Pack unter `files/<path>`; `path` ist das Ziel im Spielordner.
    #[serde(rename_all = "camelCase")]
    File { path: String, sha1: String },
}

impl SyncEntry {
    pub fn label(&self) -> &str {
        match self {
            SyncEntry::Modrinth { name, .. } | SyncEntry::Curseforge { name, .. } => name,
            SyncEntry::File { path, .. } => path,
        }
    }
}

/// Wann ein Sync-Pack bei neuen Profilen automatisch eingeschaltet wird.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoRule {
    pub enabled: bool,
    /// Nur bei diesen Loadern (`None` im Vektor = Vanilla); leer = bei allen.
    #[serde(default)]
    pub loaders: Vec<Option<LoaderKind>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPack {
    /// Ordnername unter `syncpacks/`.
    #[serde(skip_deserializing, default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub mode: SyncMode,
    #[serde(default)]
    pub entries: Vec<SyncEntry>,
    /// Standardwerte für `options.txt`, z. B. `fov` → `0.25`, `key_key.zoom` → `key.keyboard.c`.
    #[serde(default)]
    pub options: BTreeMap<String, String>,
    #[serde(default)]
    pub auto: AutoRule,
}

/// Geteiltes Sync-Pack (F-58): Projekte, Optionen und Tasten, nie eigene Dateien (die könnten
/// beliebige Inhalte in den Spielordner bringen).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedPack {
    pub name: String,
    #[serde(default)]
    pub mode: SyncMode,
    #[serde(default)]
    pub entries: Vec<SyncEntry>,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

impl SharedPack {
    /// Was von einem Sync-Pack geteilt wird.
    pub fn from_pack(pack: &SyncPack) -> Self {
        Self {
            name: pack.name.clone(),
            mode: pack.mode,
            entries: pack
                .entries
                .iter()
                .filter(|e| !matches!(e, SyncEntry::File { .. }))
                .cloned()
                .collect(),
            options: pack.options.clone(),
        }
    }

    /// Liest geteilte Daten; eigene Dateien werden verworfen, auch wenn jemand sie ergänzt hat.
    pub fn parse(data: &serde_json::Value) -> Result<Self> {
        let mut shared: SharedPack = serde_json::from_value(data.clone())
            .map_err(|e| Error::Invalid(format!("Geteiltes Sync-Pack ist ungültig: {e}")))?;
        shared
            .entries
            .retain(|e| !matches!(e, SyncEntry::File { .. }));
        shared.name = shared
            .name
            .chars()
            .filter(|c| !c.is_control())
            .take(48)
            .collect();
        Ok(shared)
    }

    /// Anzahl der Dateien, die beim Teilen weggelassen werden.
    pub fn skipped_files(pack: &SyncPack) -> usize {
        pack.entries
            .iter()
            .filter(|e| matches!(e, SyncEntry::File { .. }))
            .count()
    }
}

/// Was ein Sync-Pack in einem Profil eingespielt hat.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Applied {
    /// Eigene Dateien: Pfad → Hash beim Einspielen.
    #[serde(default)]
    files: BTreeMap<String, String>,
    /// Optionen: Schlüssel → eingespielter Wert.
    #[serde(default)]
    options: BTreeMap<String, String>,
    /// Projekte, die das Sync-Pack beim letzten Abgleich enthielt.
    #[serde(default)]
    projects: BTreeSet<String>,
}

/// `instances/<id>/syncpacks.json`
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProfileState {
    /// Eingeschaltete Sync-Packs (Vorlagen stehen hier nie).
    #[serde(default)]
    enabled: BTreeSet<String>,
    #[serde(default)]
    applied: BTreeMap<String, Applied>,
}

/// Sync-Pack aus Sicht eines Profils.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSyncPack {
    pub pack: SyncPack,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    /// Neu installiert (Name).
    pub installed: Vec<String>,
    /// Entfernt, weil abgeschaltet, aus dem Sync-Pack gelöscht oder vom Modpack geliefert.
    pub removed: Vec<String>,
    /// Übersprungen: schon vorhanden (z. B. aus dem Modpack).
    pub present: Vec<String>,
    /// Gibt es nicht für Version/Loader des Profils.
    pub unavailable: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone)]
pub struct SyncManager {
    paths: Paths,
    content: ContentManager,
    profiles: ProfileStore,
}

impl SyncManager {
    pub fn new(paths: Paths, content: ContentManager, profiles: ProfileStore) -> Self {
        Self {
            paths,
            content,
            profiles,
        }
    }

    fn root(&self) -> PathBuf {
        self.paths.root().join("syncpacks")
    }

    fn dir(&self, id: &str) -> Result<PathBuf> {
        if id.is_empty() || id != slugify(id) {
            return Err(Error::Invalid(format!("Ungültiges Sync-Pack {id:?}")));
        }
        Ok(self.root().join(id))
    }

    pub fn list(&self) -> Result<Vec<SyncPack>> {
        let Ok(entries) = std::fs::read_dir(self.root()) else {
            return Ok(Vec::new());
        };
        let mut packs: Vec<SyncPack> = entries
            .flatten()
            .filter_map(|e| self.get(&e.file_name().to_string_lossy()).ok())
            .collect();
        packs.sort_by_key(|p| p.name.to_lowercase());
        Ok(packs)
    }

    pub fn get(&self, id: &str) -> Result<SyncPack> {
        let path = self.dir(id)?.join(PACK_FILE);
        let bytes = std::fs::read(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Error::Invalid(format!("Kein Sync-Pack {id:?}")),
            _ => Error::io(&path)(e),
        })?;
        let mut pack: SyncPack =
            serde_json::from_slice(&bytes).map_err(Error::json(path.display().to_string()))?;
        pack.id = id.to_string();
        Ok(pack)
    }

    /// Legt ein leeres Sync-Pack an; die ID ergibt sich aus dem Namen.
    pub fn create(&self, name: &str, mode: SyncMode) -> Result<SyncPack> {
        let name = name.trim();
        let base = slugify(name);
        if base.is_empty() {
            return Err(Error::Invalid("Bitte einen Namen angeben".into()));
        }
        let mut id = base.clone();
        let mut n = 2;
        while self.root().join(&id).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        let pack = SyncPack {
            id,
            name: name.to_string(),
            mode,
            entries: Vec::new(),
            options: BTreeMap::new(),
            auto: AutoRule::default(),
        };
        self.save(&pack)?;
        Ok(pack)
    }

    /// Legt aus einem geteilten Sync-Pack ein eigenes an (ohne Automatik; der Nutzer schaltet es
    /// selbst ein). Optionen werden wie beim Speichern geprüft.
    pub fn import_shared(&self, shared: &SharedPack) -> Result<SyncPack> {
        let mut pack = self.create(&shared.name, shared.mode)?;
        pack.entries = shared.entries.clone();
        pack.options = shared.options.clone();
        if let Err(e) = self.save(&pack) {
            let _ = self.delete(&pack.id);
            return Err(e);
        }
        Ok(pack)
    }

    /// Speichert Name, Art, Einträge, Optionen und Auto-Regel. Dateien von entfernten
    /// `File`-Einträgen werden gelöscht.
    pub fn save(&self, pack: &SyncPack) -> Result<()> {
        let dir = self.dir(&pack.id)?;
        if pack.name.trim().is_empty() {
            return Err(Error::Invalid("Bitte einen Namen angeben".into()));
        }
        let mut seen = HashSet::new();
        for e in &pack.entries {
            if let SyncEntry::File { path, .. } = e {
                safe_relative(path)?;
                if !seen.insert(path.as_str()) {
                    return Err(Error::Invalid(format!("Datei {path} doppelt")));
                }
            }
        }
        for (key, value) in &pack.options {
            if key.is_empty() || key == "version" || key.contains([':', '\n', '\r']) {
                return Err(Error::Invalid(format!("Ungültige Option {key:?}")));
            }
            if value.contains(['\n', '\r']) {
                return Err(Error::Invalid(format!("Ungültiger Wert für {key}")));
            }
        }
        std::fs::create_dir_all(&dir).map_err(Error::io(&dir))?;
        let path = dir.join(PACK_FILE);
        let json = serde_json::to_vec_pretty(pack).map_err(Error::json(PACK_FILE))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(Error::io(&tmp))?;
        std::fs::rename(&tmp, &path).map_err(Error::io(&path))?;

        // Verwaiste eigene Dateien aufräumen.
        let files = dir.join("files");
        for rel in walk(&files) {
            if !seen.contains(rel.as_str()) {
                let _ = std::fs::remove_file(files.join(&rel));
            }
        }
        Ok(())
    }

    /// Löscht ein Sync-Pack. In Profilen eingespielte Inhalte bleiben, verlieren aber die Markierung.
    pub fn delete(&self, id: &str) -> Result<()> {
        let dir = self.dir(id)?;
        for profile in self.profiles.list()? {
            let mut state = self.state(&profile.id)?;
            if state.enabled.remove(id) | state.applied.remove(id).is_some() {
                self.save_state(&profile.id, &state)?;
            }
            let mut lock = self.content.load_lock(&profile.id)?;
            let mut changed = false;
            for e in lock.files.values_mut() {
                if e.sync.as_deref() == Some(id) {
                    e.sync = None;
                    changed = true;
                }
            }
            if changed {
                self.content.save_lock(&profile.id, &lock)?;
            }
        }
        std::fs::remove_dir_all(&dir).map_err(Error::io(&dir))
    }

    /// Eigene Datei ins Sync-Pack kopieren; `target` ist das Ziel im Spielordner
    /// (z. B. `config/sodium-options.json` oder `resourcepacks/x.zip`).
    pub async fn add_file(&self, id: &str, source: &Path, target: &str) -> Result<SyncPack> {
        let target = target.trim().trim_start_matches('/').replace('\\', "/");
        safe_relative(&target)?;
        if target == "options.txt" || target == "servers.dat" {
            return Err(Error::Invalid(format!(
                "{target} lässt sich nicht als Datei teilen (für options.txt einzelne Optionen verwenden)"
            )));
        }
        let mut pack = self.get(id)?;
        let dest = self.dir(id)?.join("files").join(&target);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
        }
        std::fs::copy(source, &dest).map_err(Error::io(&dest))?;
        let sha1 = sha1_file(&dest).await?;
        pack.entries
            .retain(|e| !matches!(e, SyncEntry::File { path, .. } if *path == target));
        pack.entries.push(SyncEntry::File { path: target, sha1 });
        self.save(&pack)?;
        Ok(pack)
    }

    /// Inhalte eines Profils (Schlüssel wie im Inhalte-Tab) ins Sync-Pack übernehmen:
    /// Plattform-Inhalte als Projekt, eigene Dateien als Kopie.
    pub async fn add_from_profile(
        &self,
        id: &str,
        profile: &Profile,
        keys: &[String],
    ) -> Result<SyncPack> {
        let game_dir = self.profiles.game_dir(&profile.id);
        let installed = self.content.identify(&profile.id, &game_dir).await?;
        let mut pack = self.get(id)?;
        for key in keys {
            let c = installed
                .iter()
                .find(|c| c.key == *key)
                .ok_or_else(|| Error::Invalid(format!("{key} gibt es im Profil nicht")))?;
            let entry = match &c.source {
                Source::Modrinth { project_id, .. } => SyncEntry::Modrinth {
                    project_id: project_id.clone(),
                    kind: c.kind,
                    name: c.name.clone(),
                },
                Source::Curseforge { project_id, .. } => SyncEntry::Curseforge {
                    project_id: *project_id,
                    kind: c.kind,
                    name: c.name.clone(),
                },
                Source::Local => {
                    let path = game_dir.join(c.kind.folder()).join(&c.file_name);
                    pack = self.add_file(id, &path, key).await?;
                    continue;
                }
            };
            if !pack.entries.iter().any(|e| same_project(e, &entry)) {
                pack.entries.push(entry);
            }
        }
        self.save(&pack)?;
        Ok(pack)
    }

    fn state_path(&self, profile_id: &str) -> PathBuf {
        self.paths.instance_dir(profile_id).join(STATE_FILE)
    }

    fn state(&self, profile_id: &str) -> Result<ProfileState> {
        let path = self.state_path(profile_id);
        match std::fs::read(&path) {
            Ok(b) => serde_json::from_slice(&b).map_err(Error::json(path.display().to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ProfileState::default()),
            Err(e) => Err(Error::io(&path)(e)),
        }
    }

    fn save_state(&self, profile_id: &str, state: &ProfileState) -> Result<()> {
        let path = self.state_path(profile_id);
        let json = serde_json::to_vec_pretty(state).map_err(Error::json(STATE_FILE))?;
        std::fs::write(&path, json).map_err(Error::io(&path))
    }

    /// Profile, in denen das Sync-Pack eingeschaltet ist.
    pub fn enabled_in(&self, id: &str) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for profile in self.profiles.list()? {
            if self.state(&profile.id)?.enabled.contains(id) {
                out.push(profile.id);
            }
        }
        Ok(out)
    }

    /// Optionen und Steuerung aus der `options.txt` eines Profils, zum Übernehmen ins Sync-Pack.
    pub fn profile_options(&self, profile_id: &str) -> BTreeMap<String, String> {
        read_options(&self.profiles.game_dir(profile_id))
    }

    /// Alle Sync-Packs mit ihrem Schalter für dieses Profil.
    pub fn for_profile(&self, profile_id: &str) -> Result<Vec<ProfileSyncPack>> {
        let state = self.state(profile_id)?;
        Ok(self
            .list()?
            .into_iter()
            .map(|pack| ProfileSyncPack {
                enabled: state.enabled.contains(&pack.id),
                pack,
            })
            .collect())
    }

    /// Schaltet ein Sync-Pack im Profil ein (und spielt es ein) oder aus (und entfernt, was es
    /// gebracht hat). Vorlagen werden nur eingespielt.
    pub async fn set_enabled(
        &self,
        profile: &Profile,
        id: &str,
        enabled: bool,
    ) -> Result<SyncReport> {
        let pack = self.get(id)?;
        let mut state = self.state(&profile.id)?;
        let mut report = SyncReport::default();
        if !enabled {
            state.enabled.remove(id);
            let applied = state.applied.remove(id).unwrap_or_default();
            self.save_state(&profile.id, &state)?;
            self.remove_applied(profile, &pack, &applied, &mut report)
                .await?;
            return Ok(report);
        }
        self.apply(profile, &pack, &mut state, &mut report).await?;
        if pack.mode == SyncMode::Sync {
            state.enabled.insert(id.to_string());
        } else {
            state.applied.remove(id);
        }
        self.save_state(&profile.id, &state)?;
        Ok(report)
    }

    /// Gleicht alle eingeschalteten Sync-Packs ab (vor dem Start, nach Änderungen am Sync-Pack).
    pub async fn sync(&self, profile: &Profile) -> Result<SyncReport> {
        let mut state = self.state(&profile.id)?;
        let mut report = SyncReport::default();
        for id in state.enabled.clone() {
            match self.get(&id) {
                // Ein fehlerhaftes Sync-Pack hält die anderen nicht auf.
                Ok(pack) => {
                    if let Err(e) = self.apply(profile, &pack, &mut state, &mut report).await {
                        report.warnings.push(format!("{}: {e}", pack.name));
                    }
                }
                Err(_) => {
                    state.enabled.remove(&id);
                    state.applied.remove(&id);
                }
            }
        }
        self.save_state(&profile.id, &state)?;
        Ok(report)
    }

    /// Neues Profil: Sync-Packs mit passender Auto-Regel einschalten.
    pub async fn apply_auto(&self, profile: &Profile) -> Result<SyncReport> {
        let loader = profile.loader.as_ref().map(|l| l.kind);
        let mut report = SyncReport::default();
        for pack in self.list()? {
            let rule = &pack.auto;
            if rule.enabled && (rule.loaders.is_empty() || rule.loaders.contains(&loader)) {
                let r = self.set_enabled(profile, &pack.id, true).await?;
                merge(&mut report, r);
            }
        }
        Ok(report)
    }

    /// Spielt ein Sync-Pack ein: fehlende Projekte installieren, eigene Dateien und Optionen
    /// setzen, nicht mehr enthaltene Inhalte entfernen.
    async fn apply(
        &self,
        profile: &Profile,
        pack: &SyncPack,
        state: &mut ProfileState,
        report: &mut SyncReport,
    ) -> Result<()> {
        let game_dir = self.profiles.game_dir(&profile.id);
        std::fs::create_dir_all(&game_dir).map_err(Error::io(&game_dir))?;
        self.content.identify(&profile.id, &game_dir).await?;
        let tag = (pack.mode == SyncMode::Sync).then(|| pack.id.clone());

        // Projekte
        let mut wanted_projects = HashSet::new();
        for entry in &pack.entries {
            let (project, kind, name) = match entry {
                SyncEntry::Modrinth {
                    project_id,
                    kind,
                    name,
                } => (project_id.clone(), *kind, name),
                SyncEntry::Curseforge {
                    project_id,
                    kind,
                    name,
                } => (project_id.to_string(), *kind, name),
                SyncEntry::File { .. } => continue,
            };
            wanted_projects.insert(project.clone());
            let lock = self.content.load_lock(&profile.id)?;
            if let Some(owner) = owner_of(&lock, entry) {
                // Schon da: aus dem Pack, vom Nutzer oder bereits von uns.
                if owner.as_deref() != tag.as_deref() {
                    report.present.push(name.clone());
                }
                continue;
            }
            if loaders_for(profile, kind).is_none() {
                report.unavailable.push(name.clone());
                continue;
            }
            let before: HashSet<String> = lock.files.keys().cloned().collect();
            let result = match entry {
                SyncEntry::Modrinth { project_id, .. } => {
                    self.content
                        .install(profile, &game_dir, kind, project_id, None)
                        .await
                }
                SyncEntry::Curseforge { project_id, .. } => {
                    self.content
                        .install_curseforge(profile, &game_dir, kind, *project_id, None)
                        .await
                }
                SyncEntry::File { .. } => unreachable!(),
            };
            match result {
                Ok(r) => {
                    report.warnings.extend(r.warnings);
                    let mut lock = self.content.load_lock(&profile.id)?;
                    for (key, e) in lock.files.iter_mut() {
                        if !before.contains(key) {
                            e.sync = tag.clone();
                        }
                    }
                    self.content.save_lock(&profile.id, &lock)?;
                    report.installed.push(name.clone());
                }
                // „Keine Version/Datei … passt“: gibt es nicht für dieses Profil.
                Err(Error::Invalid(msg)) if msg.starts_with("Keine") => {
                    report.unavailable.push(name.clone());
                }
                Err(e) => report.warnings.push(format!("{name}: {e}")),
            }
        }

        // Projekte, die das Sync-Pack nicht mehr enthält oder die jetzt jemand anderes liefert.
        if let Some(tag) = &tag {
            let mut lock = self.content.load_lock(&profile.id)?;
            let ours: Vec<(String, Option<String>, String)> = lock
                .files
                .iter()
                .filter(|(_, e)| e.sync.as_deref() == Some(tag))
                .map(|(k, e)| (k.clone(), project_of(&e.source), e.name.clone()))
                .collect();
            for (key, project, name) in ours {
                let Some(project) = project else { continue };
                let others = lock.files.iter().any(|(k, e)| {
                    *k != key
                        && e.sync.as_deref() != Some(tag)
                        && project_of(&e.source).as_deref() == Some(&project)
                });
                // Abhängigkeiten stehen nicht im Sync-Pack; sie gehen erst mit dem Ausschalten.
                let listed_before = state
                    .applied
                    .get(tag)
                    .is_some_and(|a| a.projects.contains(&project));
                let dropped = listed_before && !wanted_projects.contains(&project);
                if others || dropped {
                    remove_file_pair(&game_dir, &key);
                    lock.files.remove(&key);
                    report.removed.push(name);
                }
            }
            self.content.save_lock(&profile.id, &lock)?;
        }

        // Eigene Dateien und Optionen
        let applied = state.applied.entry(pack.id.clone()).or_default();
        let files_dir = self.dir(&pack.id)?.join("files");
        let mut wanted_files = HashSet::new();
        for entry in &pack.entries {
            let SyncEntry::File { path, sha1 } = entry else {
                continue;
            };
            safe_relative(path)?;
            wanted_files.insert(path.clone());
            let target = game_dir.join(path);
            let current = if target.is_file() {
                Some(sha1_file(&target).await?)
            } else {
                None
            };
            let ours = applied.files.get(path);
            // Schreiben beim ersten Einspielen, wenn die Datei fehlt oder noch unserem letzten
            // Stand entspricht. Eine Config, die die Mod schon angelegt hat, wird also ersetzt.
            if ours.is_none() || current.is_none() || current.as_ref() == ours {
                if current.as_ref() != Some(sha1) {
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
                    }
                    std::fs::copy(files_dir.join(path), &target).map_err(Error::io(&target))?;
                    report.installed.push(path.clone());
                }
                applied.files.insert(path.clone(), sha1.clone());
            } else if current.as_ref() != Some(sha1) {
                report.present.push(path.clone());
            }
        }
        // Aus dem Sync-Pack entfernte Dateien löschen, wenn unverändert.
        for (path, hash) in applied.files.clone() {
            if wanted_files.contains(&path) {
                continue;
            }
            let target = game_dir.join(&path);
            if target.is_file() && sha1_file(&target).await? == hash {
                std::fs::remove_file(&target).map_err(Error::io(&target))?;
                report.removed.push(path.clone());
            }
            applied.files.remove(&path);
        }
        // Gelistete Projekte merken (für „aus dem Sync-Pack entfernt“).
        applied.projects = wanted_projects.into_iter().collect();

        apply_options(&game_dir, &pack.options, &mut applied.options)?;
        Ok(())
    }

    /// Entfernt, was ein Sync-Pack eingespielt hat (Dateien nur, wenn unverändert).
    async fn remove_applied(
        &self,
        profile: &Profile,
        pack: &SyncPack,
        applied: &Applied,
        report: &mut SyncReport,
    ) -> Result<()> {
        let game_dir = self.profiles.game_dir(&profile.id);
        let mut lock = self.content.load_lock(&profile.id)?;
        let ours: Vec<String> = lock
            .files
            .iter()
            .filter(|(_, e)| e.sync.as_deref() == Some(&pack.id))
            .map(|(k, _)| k.clone())
            .collect();
        for key in ours {
            remove_file_pair(&game_dir, &key);
            if let Some(e) = lock.files.remove(&key) {
                report.removed.push(e.name);
            }
        }
        self.content.save_lock(&profile.id, &lock)?;
        for (path, hash) in &applied.files {
            let target = game_dir.join(path);
            if target.is_file() && sha1_file(&target).await? == *hash {
                std::fs::remove_file(&target).map_err(Error::io(&target))?;
                report.removed.push(path.clone());
            }
        }
        // Optionen bleiben (sonst wären Einstellungen plötzlich weg); nur nicht mehr verwaltet.
        Ok(())
    }
}

fn project_of(source: &Source) -> Option<String> {
    match source {
        Source::Modrinth { project_id, .. } => Some(project_id.clone()),
        Source::Curseforge { project_id, .. } => Some(project_id.to_string()),
        Source::Local => None,
    }
}

fn same_project(a: &SyncEntry, b: &SyncEntry) -> bool {
    match (a, b) {
        (SyncEntry::Modrinth { project_id: x, .. }, SyncEntry::Modrinth { project_id: y, .. }) => {
            x == y
        }
        (
            SyncEntry::Curseforge { project_id: x, .. },
            SyncEntry::Curseforge { project_id: y, .. },
        ) => x == y,
        _ => false,
    }
}

/// Wer liefert das Projekt schon? `Some(None)`: Pack/Nutzer, `Some(Some(id))`: Sync-Pack `id`.
fn owner_of(lock: &ContentLock, entry: &SyncEntry) -> Option<Option<String>> {
    lock.files.values().find_map(|e| {
        let hit = match (entry, &e.source) {
            (SyncEntry::Modrinth { project_id: a, .. }, Source::Modrinth { project_id: b, .. }) => {
                a == b
            }
            (
                SyncEntry::Curseforge { project_id: a, .. },
                Source::Curseforge { project_id: b, .. },
            ) => a == b,
            _ => false,
        };
        hit.then(|| e.sync.clone())
    })
}

fn remove_file_pair(game_dir: &Path, key: &str) {
    for p in [
        game_dir.join(key),
        game_dir.join(format!("{key}{DISABLED_SUFFIX}")),
    ] {
        let _ = std::fs::remove_file(p);
    }
}

fn merge(into: &mut SyncReport, r: SyncReport) {
    into.installed.extend(r.installed);
    into.removed.extend(r.removed);
    into.present.extend(r.present);
    into.unavailable.extend(r.unavailable);
    into.warnings.extend(r.warnings);
}

/// Datenversion für eine `options.txt` ohne `version:`-Zeile. Fehlt sie, hält Minecraft die Datei
/// für uralt und schickt sie durch alle Umwandlungen; die für Tasten erwartet Zahlen und bricht bei
/// `key.keyboard.c` ab, worauf das Spiel *alle* Optionen verwirft. Eine Version über der des Spiels
/// überspringt die Umwandlungen; beim nächsten Speichern schreibt Minecraft seine eigene.
const OPTIONS_VERSION: &str = "version:2147483647";

/// Optionen aus der `options.txt` eines Spielordners (ohne `version`).
pub fn read_options(game_dir: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(game_dir.join("options.txt"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once(':'))
        .filter(|(k, _)| !k.is_empty() && *k != "version")
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Setzt Optionen in `options.txt`: beim ersten Einspielen immer, danach nur dort, wo der Nutzer
/// nichts anderes eingestellt hat (Wert fehlt oder entspricht dem zuletzt von uns gesetzten).
fn apply_options(
    game_dir: &Path,
    wanted: &BTreeMap<String, String>,
    applied: &mut BTreeMap<String, String>,
) -> Result<()> {
    applied.retain(|k, _| wanted.contains_key(k));
    if wanted.is_empty() {
        return Ok(());
    }
    let path = game_dir.join("options.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut changed = false;
    for (key, value) in wanted {
        let pos = lines
            .iter()
            .position(|l| l.split_once(':').is_some_and(|(k, _)| k == key));
        let current = pos.and_then(|i| lines[i].split_once(':').map(|(_, v)| v.to_string()));
        let ours = applied.get(key);
        if ours.is_none() || current.is_none() || current.as_ref() == ours {
            if current.as_ref() != Some(value) {
                let line = format!("{key}:{value}");
                match pos {
                    Some(i) => lines[i] = line,
                    None => lines.push(line),
                }
                changed = true;
            }
            applied.insert(key.clone(), value.clone());
        }
    }
    // Auch nachrüsten, wenn eine frühere Fassung die Datei ohne Version angelegt hat.
    if !lines.iter().any(|l| l.starts_with("version:")) {
        lines.insert(0, OPTIONS_VERSION.to_string());
        changed = true;
    }
    if changed {
        let mut out = lines.join("\n");
        out.push('\n');
        std::fs::write(&path, out).map_err(Error::io(&path))?;
    }
    Ok(())
}

/// Alle Dateien unter `root` als relative Pfade mit `/`.
fn walk(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let rel = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            match e.file_type() {
                Ok(ft) if ft.is_dir() => stack.push((e.path(), rel)),
                Ok(ft) if ft.is_file() => out.push(rel),
                _ => {}
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_first_apply_then_respect_user_changes() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("options.txt");
        std::fs::write(&file, "version:3955\nfov:0.0\nguiScale:2\n").unwrap();
        let wanted = BTreeMap::from([
            ("fov".to_string(), "0.25".to_string()),
            ("key_key.zoom".to_string(), "key.keyboard.c".to_string()),
        ]);
        let mut applied = BTreeMap::new();
        // Erstes Einspielen: setzt auch Werte, die Minecraft schon geschrieben hat.
        apply_options(dir.path(), &wanted, &mut applied).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert_eq!(
            text,
            "version:3955\nfov:0.25\nguiScale:2\nkey_key.zoom:key.keyboard.c\n"
        );
        assert_eq!(applied, wanted);

        // Neuer Wert im Sync-Pack ersetzt unseren alten, solange der Nutzer ihn nicht geändert hat.
        let wanted2 = BTreeMap::from([("key_key.zoom".to_string(), "key.keyboard.v".to_string())]);
        apply_options(dir.path(), &wanted2, &mut applied).unwrap();
        assert!(
            std::fs::read_to_string(&file)
                .unwrap()
                .contains("key_key.zoom:key.keyboard.v")
        );

        // Nutzer ändert die Taste im Spiel → bleibt bei seiner Wahl.
        std::fs::write(&file, "version:3955\nkey_key.zoom:key.keyboard.b\n").unwrap();
        apply_options(dir.path(), &wanted2, &mut applied).unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "version:3955\nkey_key.zoom:key.keyboard.b\n"
        );
    }

    #[test]
    fn new_options_file_gets_a_version() {
        // Ohne `version:` würde Minecraft die Tastennamen als Zahlen lesen und alles verwerfen.
        let dir = tempfile::tempdir().unwrap();
        let wanted = BTreeMap::from([("key_key.zoom".to_string(), "key.keyboard.c".to_string())]);
        apply_options(dir.path(), &wanted, &mut BTreeMap::new()).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            format!("{OPTIONS_VERSION}\nkey_key.zoom:key.keyboard.c\n")
        );
        assert_eq!(read_options(dir.path()), wanted);
    }

    #[test]
    fn entries_serialize_tagged() {
        let e = SyncEntry::Modrinth {
            project_id: "AANobbMI".into(),
            kind: ContentKind::Mod,
            name: "Sodium".into(),
        };
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "modrinth", "projectId": "AANobbMI", "kind": "mod", "name": "Sodium" })
        );
    }
}
