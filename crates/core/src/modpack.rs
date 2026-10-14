//! Modpacks im Modrinth-Format (`.mrpack`): importieren, aktualisieren und exportieren.
//!
//! Updates vergleichen drei Stände per SHA-1: was das Pack zuletzt geliefert hat (Basis),
//! was jetzt im Spielordner liegt (Nutzer) und was die neue Pack-Version liefert. Vom Pack
//! gelieferte, unveränderte Dateien werden ersetzt, Änderungen des Nutzers bleiben, entfernte
//! Pack-Dateien werden gelöscht, echte Konflikte entscheidet der Nutzer. Vorher wird ein
//! Snapshot angelegt, aus dem sich alles zurückholen lässt.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use crate::content::{ContentKind, ContentLock, ContentManager, DISABLED_SUFFIX, Source};
use crate::curseforge::{self, CurseForge};
use crate::download::{DownloadTask, Downloader, sha1_file};
use crate::loader::{LoaderKind, LoaderSpec};
use crate::modrinth::{self, ALLOWED_HOSTS, Modrinth};
use crate::paths::{Paths, safe_relative};
use crate::profiles::{Profile, ProfileStore};
use crate::stats::now_secs;
use crate::{Error, Result};

const INDEX: &str = "modrinth.index.json";
const MAX_INDEX: u64 = 16 * 1024 * 1024;
/// Snapshots je Profil, ältere werden aufgeräumt.
const KEEP_SNAPSHOTS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PackSource {
    Modrinth {
        project_id: String,
        version_id: String,
    },
    Curseforge {
        project_id: u64,
        file_id: u64,
    },
    /// TwoGG-Modpack vom Pinecone-Server.
    Twogg {
        pack_id: String,
        version_id: String,
    },
    /// Aus einer Datei importiert, ohne Update-Quelle.
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackInfo {
    pub name: String,
    pub version: String,
    pub source: PackSource,
    /// Was das Pack zuletzt geliefert hat: Pfad → SHA-1 (Basis für Updates).
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// Signiertes Event-Pack (F-10/F-11): Pflichtdateien und Regeln aus dem Manifest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<EventLock>,
}

/// Aus dem geprüften Manifest übernommen, damit Sperre und Integritätsprüfung offline gehen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventLock {
    pub event_id: String,
    /// Version laut Manifest.
    pub version: String,
    pub channel: String,
    pub required: bool,
    pub allow_extra_mods: bool,
    pub key_id: String,
    /// SHA-512 des `.mrpack` (für den Pack-Cache bei der Reparatur).
    pub mrpack_sha512: String,
    /// Pflichtdateien: Pfad → SHA-512.
    pub locked: BTreeMap<String, String>,
}

impl EventLock {
    pub fn from_manifest(m: &crate::eventpack::Manifest) -> Self {
        Self {
            event_id: m.event_id.clone(),
            version: m.version.clone(),
            channel: m.channel.clone(),
            required: m.required,
            allow_extra_mods: m.allow_extra_mods,
            key_id: m.key_id.clone(),
            mrpack_sha512: m.mrpack.sha512.to_ascii_lowercase(),
            locked: m
                .locked
                .iter()
                .map(|l| (l.path.clone(), l.sha512.to_ascii_lowercase()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum EnvSupport {
    Required,
    Optional,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Env {
    client: EnvSupport,
    server: EnvSupport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Hashes {
    sha1: String,
    sha512: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexFile {
    path: String,
    hashes: Hashes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    env: Option<Env>,
    downloads: Vec<String>,
    file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MrpackIndex {
    format_version: u32,
    game: String,
    version_id: String,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    files: Vec<IndexFile>,
    dependencies: BTreeMap<String, String>,
}

/// Eine Datei, die das Pack liefert.
#[derive(Debug, Clone)]
pub enum PackFile {
    Download {
        urls: Vec<String>,
        sha1: String,
        size: u64,
    },
    /// Aus `overrides/` bzw. `client-overrides/` im Archiv.
    Archive { entry: String, sha1: String },
}

impl PackFile {
    pub fn sha1(&self) -> &str {
        match self {
            PackFile::Download { sha1, .. } | PackFile::Archive { sha1, .. } => sha1,
        }
    }
}

/// Ergebnis der Integritätsprüfung einer Event-Instanz.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    /// Wiederhergestellte Pflichtdateien.
    pub repaired: Vec<String>,
    /// Abgeschaltete Zusatz-Mods.
    pub disabled: Vec<String>,
}

fn file_label(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// SHA-512 je Datei, gemerkt über Größe und Änderungszeit (AK-11.5: schnelle Prüfung ohne
/// jedes Mal alles neu zu hashen).
#[derive(Debug, Default, Serialize, Deserialize)]
struct HashCache {
    #[serde(default)]
    files: BTreeMap<String, CachedHash>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedHash {
    size: u64,
    mtime_ns: u128,
    sha512: String,
}

impl HashCache {
    fn sha512(&mut self, path: &Path) -> Option<String> {
        let meta = std::fs::metadata(path).ok()?;
        if !meta.is_file() {
            return None;
        }
        let mtime_ns = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let key = path.to_string_lossy().to_string();
        if let Some(c) = self.files.get(&key)
            && c.size == meta.len()
            && c.mtime_ns == mtime_ns
            && mtime_ns != 0
        {
            return Some(c.sha512.clone());
        }
        let sha512 = crate::eventpack::sha512_file(path).ok()?;
        self.files.insert(
            key,
            CachedHash {
                size: meta.len(),
                mtime_ns,
                sha512: sha512.clone(),
            },
        );
        Some(sha512)
    }

    fn retain(&mut self, keep: &[PathBuf]) {
        let keep: std::collections::HashSet<String> = keep
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        self.files.retain(|k, _| keep.contains(k));
    }
}

/// Gelesenes Pack-Archiv: Zielzustand des Spielordners.
#[derive(Debug, Clone)]
pub struct PackArchive {
    pub path: PathBuf,
    pub name: String,
    pub version: String,
    pub summary: Option<String>,
    pub minecraft: String,
    pub loader: Option<LoaderSpec>,
    pub files: BTreeMap<String, PackFile>,
    /// Zusätzlich erlaubte Download-Hosts (nur für Adressen aus der CurseForge-API).
    pub extra_hosts: Vec<String>,
    /// Dateien, die der Nutzer selbst herunterladen muss (CurseForge-Sperre).
    pub missing: Vec<ManualDownload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualDownload {
    pub name: String,
    /// Zielpfad im Spielordner, z. B. `mods/x.jar`.
    pub path: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub profile: Profile,
    pub missing: Vec<ManualDownload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifest {
    minecraft: CfMinecraft,
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    files: Vec<CfManifestFile>,
    #[serde(default = "default_overrides")]
    overrides: String,
}

fn default_overrides() -> String {
    "overrides".into()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMinecraft {
    version: String,
    #[serde(default)]
    mod_loaders: Vec<CfLoader>,
}

#[derive(Debug, Deserialize)]
struct CfLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Debug, Deserialize)]
struct CfManifestFile {
    #[serde(rename = "projectID")]
    project_id: u64,
    #[serde(rename = "fileID")]
    file_id: u64,
    #[serde(default = "yes")]
    required: bool,
}

fn yes() -> bool {
    true
}

/// Loader aus einem CurseForge-Manifest (`fabric-0.16.5`, `quilt-0.26.4`, …).
fn cf_loader(loaders: &[CfLoader]) -> Result<Option<LoaderSpec>> {
    let Some(l) = loaders.iter().find(|l| l.primary).or(loaders.first()) else {
        return Ok(None);
    };
    let (name, version) = l.id.split_once('-').unwrap_or((l.id.as_str(), ""));
    match name {
        "fabric" => Ok(Some(LoaderSpec {
            kind: LoaderKind::Fabric,
            version: version.into(),
        })),
        "quilt" => Ok(Some(LoaderSpec {
            kind: LoaderKind::Quilt,
            version: version.into(),
        })),
        "forge" => Ok(Some(LoaderSpec {
            kind: LoaderKind::Forge,
            version: version.into(),
        })),
        "neoforge" => Ok(Some(LoaderSpec {
            kind: LoaderKind::NeoForge,
            // Für 1.20.1 hieß NeoForge noch `1.20.1-47.1.x`; dafür gibt es keinen NeoForge-Installer.
            version: version.into(),
        })),
        other => Err(Error::Invalid(format!(
            "Unbekannter Mod-Loader {other:?} im Modpack"
        ))),
    }
}

/// Erkennt, ob ein Archiv ein Modrinth- oder CurseForge-Pack ist.
fn archive_kind(path: &Path) -> Result<&'static str> {
    let file = std::fs::File::open(path).map_err(Error::io(path))?;
    let zip = zip::ZipArchive::new(file).map_err(zip_err(path))?;
    let names: Vec<&str> = zip.file_names().collect();
    if names.contains(&INDEX) {
        Ok("modrinth")
    } else if names.contains(&"manifest.json") {
        Ok("curseforge")
    } else {
        Err(Error::Invalid(
            "Weder ein Modrinth- noch ein CurseForge-Modpack".into(),
        ))
    }
}

fn zip_err(path: &Path) -> impl Fn(zip::result::ZipError) -> Error + '_ {
    move |source| Error::Zip {
        path: path.to_path_buf(),
        source,
    }
}

fn is_sha1(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Liest und prüft ein `.mrpack`: Format, Spiel, Loader, Pfade und Hashes.
pub fn read_mrpack(path: &Path) -> Result<PackArchive> {
    let file = std::fs::File::open(path).map_err(Error::io(path))?;
    let mut zip = zip::ZipArchive::new(file).map_err(zip_err(path))?;
    let index: MrpackIndex = {
        let entry = zip.by_name(INDEX).map_err(|_| {
            Error::Invalid("Kein Modrinth-Modpack (modrinth.index.json fehlt)".into())
        })?;
        if entry.size() > MAX_INDEX {
            return Err(Error::Invalid("modrinth.index.json ist zu groß".into()));
        }
        let mut text = String::new();
        entry
            .take(MAX_INDEX)
            .read_to_string(&mut text)
            .map_err(Error::io(path))?;
        serde_json::from_str(&text).map_err(Error::json(INDEX))?
    };
    if index.format_version != 1 || index.game != "minecraft" {
        return Err(Error::Invalid(format!(
            "Nicht unterstütztes Modpack-Format {} für {}",
            index.format_version, index.game
        )));
    }
    let minecraft = index
        .dependencies
        .get("minecraft")
        .cloned()
        .ok_or_else(|| Error::Invalid("Modpack nennt keine Minecraft-Version".into()))?;
    let loader = loader_from_deps(&index.dependencies)?;

    let mut files = BTreeMap::new();
    for f in &index.files {
        if f.env
            .as_ref()
            .is_some_and(|e| e.client == EnvSupport::Unsupported)
        {
            continue;
        }
        safe_relative(&f.path)?;
        if f.downloads.is_empty() || !is_sha1(&f.hashes.sha1) {
            return Err(Error::Invalid(format!(
                "Ungültiger Dateieintrag {} im Modpack",
                f.path
            )));
        }
        files.insert(
            f.path.clone(),
            PackFile::Download {
                urls: f.downloads.clone(),
                sha1: f.hashes.sha1.to_lowercase(),
                size: f.file_size,
            },
        );
    }
    // Overrides nach den Downloads, `client-overrides` zuletzt: spätere gewinnen (Spezifikation).
    for prefix in ["overrides/", "client-overrides/"] {
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(zip_err(path))?;
            let name = entry.name().to_string();
            let Some(rel) = name.strip_prefix(prefix) else {
                continue;
            };
            if entry.is_dir() || rel.is_empty() {
                continue;
            }
            safe_relative(rel)?;
            let mut hasher = Sha1::new();
            std::io::copy(&mut entry, &mut hasher).map_err(Error::io(path))?;
            files.insert(
                rel.to_string(),
                PackFile::Archive {
                    entry: name,
                    sha1: format!("{:x}", hasher.finalize()),
                },
            );
        }
    }
    Ok(PackArchive {
        path: path.to_path_buf(),
        name: index.name,
        version: index.version_id,
        summary: index.summary,
        minecraft,
        loader,
        files,
        extra_hosts: Vec::new(),
        missing: Vec::new(),
    })
}

fn loader_from_deps(deps: &BTreeMap<String, String>) -> Result<Option<LoaderSpec>> {
    for (key, kind) in [
        ("neoforge", LoaderKind::NeoForge),
        ("forge", LoaderKind::Forge),
    ] {
        if let Some(v) = deps.get(key) {
            return Ok(Some(LoaderSpec {
                kind,
                version: v.clone(),
            }));
        }
    }
    Ok(
        match (deps.get("fabric-loader"), deps.get("quilt-loader")) {
            (_, Some(v)) => Some(LoaderSpec {
                kind: LoaderKind::Quilt,
                version: v.clone(),
            }),
            (Some(v), None) => Some(LoaderSpec {
                kind: LoaderKind::Fabric,
                version: v.clone(),
            }),
            (None, None) => None,
        },
    )
}

/// Configs und Einstellungen: bei Konflikten bleibt standardmäßig die Fassung des Nutzers.
pub fn is_user_setting(path: &str) -> bool {
    matches!(
        path,
        "options.txt" | "optionsof.txt" | "optionsshaders.txt" | "servers.dat"
    ) || path.starts_with("config/")
        || path.starts_with("defaultconfigs/")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeAction {
    Add,
    Replace,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictReason {
    /// Nutzer und Pack haben die Datei geändert.
    BothChanged,
    /// Das Pack bringt eine Datei, die der Nutzer schon (anders) angelegt hat.
    UserFileExists,
    /// Das Pack entfernt eine Datei, die der Nutzer geändert hat.
    ChangedButRemoved,
    /// Der Nutzer hat eine Pack-Datei gelöscht, die das Pack geändert hat.
    UserRemoved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Resolution {
    KeepMine,
    TakePack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedChange {
    pub path: String,
    pub action: ChangeAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub path: String,
    pub reason: ConflictReason,
    pub default: Resolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlan {
    pub pack_name: String,
    pub from_version: String,
    pub to_version: String,
    pub minecraft: String,
    pub loader: Option<LoaderSpec>,
    pub changes: Vec<PlannedChange>,
    pub conflicts: Vec<Conflict>,
    /// Dateien mit Nutzeränderungen, die unverändert bleiben (Pack hat sie nicht geändert).
    pub kept: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub id: String,
    pub created: u64,
    pub pack_version: String,
    pub files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotManifest {
    created: u64,
    pack_version: String,
    minecraft: String,
    loader: Option<LoaderSpec>,
    /// Vorher vorhandene Dateien (gesichert unter `files/`).
    saved: Vec<String>,
    /// Durch das Update neu entstandene Dateien (beim Zurückrollen gelöscht).
    created_files: Vec<String>,
    lock: ContentLock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackUpdate {
    pub current: String,
    /// Empfohlene Version; leer, wenn es keine passende gibt (nur bei `withdrawn`).
    pub latest: String,
    pub version_id: String,
    /// Die installierte Version wurde zurückgezogen (F-62): Grund des Orga-Teams.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withdrawn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub path: String,
    /// Per Download-Link referenzierte Dateien.
    pub linked: usize,
    /// Direkt ins Archiv gepackte Dateien (eigene Mods, Configs).
    pub included: usize,
}

/// Was beim Export mitkommt. Aktive Mods sind immer dabei; `servers.dat`, Welten, Logs,
/// Screenshots und Account-Daten nie.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    pub version: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default = "yes")]
    pub resourcepacks: bool,
    #[serde(default = "yes")]
    pub shaders: bool,
    /// Ordner `config/`.
    #[serde(default = "yes")]
    pub config: bool,
    /// Spieleinstellungen (`options.txt`, Shader- und OptiFine-Optionen).
    #[serde(default)]
    pub options: bool,
    /// Inhalte aus eingeschalteten Sync-Packs.
    #[serde(default = "yes")]
    pub sync_content: bool,
}

impl ExportOptions {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            summary: None,
            resourcepacks: true,
            shaders: true,
            config: true,
            options: false,
            sync_content: true,
        }
    }

    fn wants(&self, kind: ContentKind) -> bool {
        match kind {
            ContentKind::Mod => true,
            ContentKind::Resourcepack => self.resourcepacks,
            ContentKind::Shader => self.shaders,
        }
    }
}

/// Spieleinstellungen, die `ExportOptions::options` mitnimmt.
const OPTION_FILES: [&str; 3] = ["options.txt", "optionsof.txt", "optionsshaders.txt"];

/// Übersicht über ein Pack vor dem Installieren.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackPreview {
    pub name: String,
    pub version: String,
    pub summary: Option<String>,
    pub minecraft: String,
    pub loader: Option<LoaderSpec>,
    pub mods: usize,
    pub resourcepacks: usize,
    pub shaders: usize,
    /// Configs und sonstige Dateien.
    pub other_files: usize,
    /// Summe der zu ladenden Dateien in Bytes (ohne das Archiv selbst).
    pub download_size: u64,
    pub archive_size: u64,
    /// Dateien, die der Nutzer selbst laden muss (CurseForge-Sperre).
    pub missing: Vec<ManualDownload>,
    /// Download-Quellen, die Pinecone nicht lädt: Die Installation würde scheitern.
    pub blocked_hosts: Vec<String>,
}

#[derive(Clone)]
pub struct PackManager {
    paths: Paths,
    downloader: Downloader,
    content: ContentManager,
    modrinth: Modrinth,
    profiles: ProfileStore,
    allowed_hosts: Vec<String>,
    curseforge: Option<CurseForge>,
    twogg: Option<crate::twogg::TwoggClient>,
}

/// Aktueller Stand einer Pack-Datei: tatsächlicher Pfad (evtl. `.disabled`) und Hash.
async fn current(game_dir: &Path, rel: &str) -> Result<Option<(String, String)>> {
    for candidate in [rel.to_string(), format!("{rel}{DISABLED_SUFFIX}")] {
        let path = game_dir.join(&candidate);
        if path.is_file() {
            return Ok(Some((candidate, sha1_file(&path).await?)));
        }
    }
    Ok(None)
}

impl PackManager {
    pub fn new(
        paths: Paths,
        downloader: Downloader,
        content: ContentManager,
        modrinth: Modrinth,
        profiles: ProfileStore,
    ) -> Self {
        Self {
            paths,
            downloader,
            content,
            modrinth,
            profiles,
            allowed_hosts: ALLOWED_HOSTS.iter().map(|h| h.to_string()).collect(),
            curseforge: None,
            twogg: None,
        }
    }

    pub fn with_curseforge(mut self, cf: Option<CurseForge>) -> Self {
        self.curseforge = cf;
        self
    }

    pub fn with_twogg(mut self, client: Option<crate::twogg::TwoggClient>) -> Self {
        self.twogg = client;
        self
    }

    pub fn twogg(&self) -> Option<&crate::twogg::TwoggClient> {
        self.twogg.as_ref()
    }

    /// TwoGG-Modpack (bestimmte oder neueste stabile Version) über einen signierten Link laden.
    pub async fn download_twogg(
        &self,
        pack_id: &str,
        version_id: Option<&str>,
    ) -> Result<(PathBuf, PackSource)> {
        let client = self
            .twogg
            .as_ref()
            .ok_or_else(|| Error::Invalid("TwoGG-Server ist nicht eingerichtet".into()))?;
        client.require_launcher_version().await?;
        let pack = client.pack(pack_id).await?;
        let versions = pack.versions.as_deref().unwrap_or_default();
        let chosen = match version_id {
            Some(v) => versions.iter().find(|pv| pv.id == v).cloned(),
            None => Some(
                crate::twogg::best_version(versions, None)
                    .ok_or_else(|| Error::Invalid("Dieses Modpack hat noch keine Version".into()))?
                    .clone(),
            ),
        };
        let version_id = match (&chosen, version_id) {
            (Some(pv), _) => pv.id.clone(),
            (None, Some(v)) => v.to_string(),
            (None, None) => unreachable!(),
        };
        // Signiertes Event-Pack: Manifest vor dem Download prüfen (`04` §6.2, Schritte 1–4).
        let manifest = match &chosen {
            Some(pv) if pv.signed => Some(self.verified_manifest(client, &pack, pv).await?),
            _ => None,
        };
        let link = client.download_link(pack_id, &version_id).await?;
        let dest = self.cache_dir().join(format!("twogg-{version_id}.mrpack"));
        let manifest_path = dest.with_extension("manifest.json");
        let _ = std::fs::remove_file(&manifest_path);
        self.downloader
            .download(&DownloadTask {
                url: link.url,
                dest: dest.clone(),
                sha1: Some(link.sha1),
                size: Some(link.size),
            })
            .await?;
        if let Some(m) = &manifest {
            // Schritte 6 und 7: Größe, SHA-512, Minecraft und Loader (AK-10.2).
            if let Err(e) = crate::eventpack::check_mrpack(m, &dest) {
                let _ = std::fs::remove_file(&dest);
                return Err(e);
            }
            // Für finalize_lock und die spätere Reparatur: das geprüfte Manifest neben dem Pack.
            let json = serde_json::to_vec_pretty(m).map_err(Error::json("Event-Pack-Manifest"))?;
            std::fs::write(&manifest_path, json).map_err(Error::io(&manifest_path))?;
        }
        Ok((
            dest,
            PackSource::Twogg {
                pack_id: pack_id.to_string(),
                version_id,
            },
        ))
    }

    /// Holt und prüft das signierte Manifest einer TwoGG-Version: Signatur, Format, Pack, Version,
    /// Event und Mindestversion des Launchers (AK-10.1, AK-10.3).
    async fn verified_manifest(
        &self,
        client: &crate::twogg::TwoggClient,
        pack: &crate::twogg::Pack,
        version: &crate::twogg::PackVersion,
    ) -> Result<crate::eventpack::Manifest> {
        let keyring = crate::eventpack::Keyring::builtin();
        if keyring.is_empty() {
            return Err(Error::Invalid(
                "Diese Pinecone-Version kann Event-Packs nicht prüfen (keine TwoGG-Schlüssel eingebaut)"
                    .into(),
            ));
        }
        let signed = client.version_manifest(&pack.id, &version.id).await?;
        let m = keyring.verify(&signed)?;
        if m.pack_id != pack.id || m.version != version.version {
            return Err(Error::Invalid(format!(
                "{}: das Manifest gehört zu {} {}",
                crate::eventpack::NOT_SIGNED,
                m.pack_id,
                m.version
            )));
        }
        if let Some(event) = pack.event_id.as_deref().filter(|e| !e.is_empty())
            && event != m.event_id
        {
            return Err(Error::Invalid(format!(
                "Das Event-Pack gehört zu einem anderen Event ({})",
                m.event_id
            )));
        }
        if crate::twogg::version_older(crate::LAUNCHER_VERSION, &m.min_launcher_version) {
            return Err(Error::Invalid(format!(
                "Bitte aktualisiere Pinecone: Für dieses Event-Pack brauchst du mindestens Version {} (installiert: {})",
                m.min_launcher_version,
                crate::LAUNCHER_VERSION
            )));
        }
        Ok(m)
    }

    /// Geprüftes Manifest zu einer heruntergeladenen TwoGG-Version (siehe `download_twogg`).
    fn cached_manifest(&self, source: &PackSource) -> Option<crate::eventpack::Manifest> {
        let PackSource::Twogg { version_id, .. } = source else {
            return None;
        };
        let path = self
            .cache_dir()
            .join(format!("twogg-{version_id}.manifest.json"));
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    }

    /// Prüft die Pflichtdateien einer Event-Instanz vor dem Start (F-11) und repariert sie aus dem
    /// Pack-Cache bzw. neu geladen. Unerlaubte Zusatz-Mods werden abgeschaltet. `None` bei
    /// Instanzen ohne Event-Pack.
    pub async fn check_event_integrity(
        &self,
        profile: &Profile,
        offline: bool,
    ) -> Result<Option<IntegrityReport>> {
        let lock = self.content.load_lock(&profile.id)?;
        let Some(info) = lock.pack.as_ref() else {
            return Ok(None);
        };
        let Some(event) = info.event.as_ref() else {
            return Ok(None);
        };
        let game_dir = self.profiles.game_dir(&profile.id);
        let cache_path = self
            .profiles
            .instance_dir(&profile.id)
            .join("event-hashes.json");
        let mut cache: HashCache = std::fs::read(&cache_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mut report = IntegrityReport::default();

        // Abgeschaltete Pflichtdateien wieder einschalten, dann alle per SHA-512 prüfen.
        let mut broken = Vec::new();
        for (path, sha512) in &event.locked {
            let file = game_dir.join(path);
            let off = game_dir.join(format!("{path}{}", crate::content::DISABLED_SUFFIX));
            if !file.exists() && off.exists() {
                std::fs::rename(&off, &file).map_err(Error::io(&off))?;
                report.repaired.push(path.clone());
            }
            match cache.sha512(&file) {
                Some(actual) if actual.eq_ignore_ascii_case(sha512) => {}
                _ => broken.push(path.clone()),
            }
        }

        if !broken.is_empty() {
            let PackSource::Twogg {
                pack_id,
                version_id,
            } = &info.source
            else {
                return Err(Error::Invalid("Event-Instanz ohne TwoGG-Quelle".into()));
            };
            let archive = self.cache_dir().join(format!("twogg-{version_id}.mrpack"));
            let cached_ok = crate::eventpack::sha512_file(&archive)
                .is_ok_and(|h| h.eq_ignore_ascii_case(&event.mrpack_sha512));
            if !cached_ok {
                if offline {
                    return Err(Error::Invalid(format!(
                        "Pflichtdatei {} fehlt und kann offline nicht geladen werden",
                        file_label(&broken[0])
                    )));
                }
                self.download_twogg(pack_id, Some(version_id)).await?;
            }
            let pack = read_mrpack(&archive)?;
            let mut targets = Vec::new();
            for path in &broken {
                let f = pack.files.get(path).ok_or_else(|| {
                    Error::Invalid(format!(
                        "Datei {} fehlt im Event-Pack, bitte melde das dem Bauteam",
                        file_label(path)
                    ))
                })?;
                if offline && matches!(f, PackFile::Download { .. }) {
                    return Err(Error::Invalid(format!(
                        "Pflichtdatei {} fehlt und kann offline nicht geladen werden",
                        file_label(path)
                    )));
                }
                // Vorhandene, veränderte Datei weg, sonst hält der Downloader sie für fertig.
                let target = game_dir.join(path);
                if target.exists() {
                    std::fs::remove_file(&target).map_err(Error::io(&target))?;
                }
                targets.push((path.clone(), f.clone()));
            }
            self.write_files(&game_dir, &archive, &pack.extra_hosts, &targets)
                .await?;
            for path in &broken {
                let file = game_dir.join(path);
                let actual = cache.sha512(&file).unwrap_or_default();
                if !actual.eq_ignore_ascii_case(&event.locked[path]) {
                    return Err(Error::HashMismatch {
                        path: file,
                        expected: event.locked[path].clone(),
                        actual,
                    });
                }
                report.repaired.push(path.clone());
            }
        }

        // Zusatz-Mods nur, wenn das Manifest es erlaubt (AK-11.3).
        if !event.allow_extra_mods {
            let mods = game_dir.join("mods");
            if let Ok(entries) = std::fs::read_dir(&mods) {
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    let key = format!("mods/{name}");
                    if !e.path().is_file()
                        || !name.to_ascii_lowercase().ends_with(".jar")
                        || event.locked.contains_key(&key)
                    {
                        continue;
                    }
                    let off = mods.join(format!("{name}{}", crate::content::DISABLED_SUFFIX));
                    std::fs::rename(e.path(), &off).map_err(Error::io(e.path()))?;
                    report.disabled.push(key);
                }
            }
        }

        cache.retain(
            &event
                .locked
                .keys()
                .map(|p| game_dir.join(p))
                .collect::<Vec<_>>(),
        );
        if let Ok(json) = serde_json::to_vec(&cache) {
            let _ = std::fs::write(&cache_path, json);
        }
        Ok(Some(report))
    }

    /// Spielt ein als Pflicht markiertes Event-Pack-Update vor dem Start ein (F-12, AK-12.1).
    /// Gesperrte Dateien kommen immer aus dem Pack, eigene Einstellungen bleiben; was ersetzt
    /// wird, liegt im Snapshot. Liefert `(alt, neu)`, wenn aktualisiert wurde. Ist der Server
    /// nicht erreichbar, startet die Instanz mit dem bisherigen Stand.
    pub async fn required_event_update(
        &self,
        profile: &Profile,
    ) -> Result<Option<(String, String)>> {
        let lock = self.content.load_lock(&profile.id)?;
        let Some(info) = lock.pack else {
            return Ok(None);
        };
        let (
            Some(event),
            PackSource::Twogg {
                pack_id,
                version_id,
            },
        ) = (info.event, info.source)
        else {
            return Ok(None);
        };
        let Some(client) = &self.twogg else {
            return Ok(None);
        };
        let keyring = crate::eventpack::Keyring::builtin();
        let latest = async {
            let signed = client.latest_manifest(&pack_id, &event.channel).await?;
            let m = keyring.verify(&signed)?;
            let pack = client.pack(&pack_id).await?;
            Ok::<_, Error>((m, pack))
        };
        let (m, pack) = match latest.await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Event-Pack-Update für {} nicht geprüft: {e}", profile.id);
                return Ok(None);
            }
        };
        if !m.required || m.pack_id != pack_id || m.version == event.version {
            return Ok(None);
        }
        // Nur vorwärts: die Liste kommt neueste zuerst.
        let versions = pack.versions.unwrap_or_default();
        let position = |pred: &dyn Fn(&crate::twogg::PackVersion) -> bool| {
            versions.iter().position(pred).unwrap_or(usize::MAX)
        };
        let target = position(&|v| v.version == m.version);
        if target == usize::MAX || target > position(&|v| v.id == version_id) {
            return Ok(None);
        }
        let target_id = versions[target].id.clone();

        let (archive, source) = self.download_twogg(&pack_id, Some(&target_id)).await?;
        let new = self.read_archive(&archive).await?;
        let plan = self.plan_update(profile, &new).await?;
        let locked: BTreeSet<&str> = m.locked.iter().map(|l| l.path.as_str()).collect();
        let resolutions: HashMap<String, Resolution> = plan
            .conflicts
            .iter()
            .filter(|c| locked.contains(c.path.as_str()))
            .map(|c| (c.path.clone(), Resolution::TakePack))
            .collect();
        self.apply_update(profile, &archive, source, &resolutions)
            .await?;
        Ok(Some((event.version, m.version)))
    }

    /// Liest ein Modrinth- (`.mrpack`) oder CurseForge-Pack (ZIP mit `manifest.json`).
    pub async fn read_archive(&self, path: &Path) -> Result<PackArchive> {
        match archive_kind(path)? {
            "modrinth" => read_mrpack(path),
            _ => self.read_curseforge(path).await,
        }
    }

    /// Liest ein Pack und fasst zusammen, was es installieren würde, ohne etwas zu schreiben.
    pub async fn preview(&self, path: &Path) -> Result<PackPreview> {
        let pack = self.read_archive(path).await?;
        let archive_size = std::fs::metadata(path).map_err(Error::io(path))?.len();
        let count = |kind: ContentKind| {
            pack.files
                .keys()
                .chain(pack.missing.iter().map(|m| &m.path))
                .filter(|p| Self::kind_of(p) == Some(kind))
                .count()
        };
        let mut blocked_hosts = BTreeSet::new();
        let mut download_size = 0;
        for file in pack.files.values() {
            if let PackFile::Download { urls, size, .. } = file {
                download_size += size;
                if self.allowed_url(urls, &pack.extra_hosts).is_err() {
                    blocked_hosts.insert(
                        urls.first()
                            .and_then(|u| reqwest::Url::parse(u).ok())
                            .and_then(|u| u.host_str().map(str::to_string))
                            .unwrap_or_else(|| "–".into()),
                    );
                }
            }
        }
        let (mods, resourcepacks, shaders) = (
            count(ContentKind::Mod),
            count(ContentKind::Resourcepack),
            count(ContentKind::Shader),
        );
        Ok(PackPreview {
            other_files: pack.files.len() + pack.missing.len() - mods - resourcepacks - shaders,
            mods,
            resourcepacks,
            shaders,
            download_size,
            archive_size,
            blocked_hosts: blocked_hosts.into_iter().collect(),
            name: pack.name,
            version: pack.version,
            summary: pack.summary,
            minecraft: pack.minecraft,
            loader: pack.loader,
            missing: pack.missing,
        })
    }

    /// CurseForge-Pack: Dateien über die API auflösen; gesperrte kommen in `missing`.
    async fn read_curseforge(&self, path: &Path) -> Result<PackArchive> {
        let cf = self.curseforge.as_ref().ok_or_else(|| {
            Error::Invalid("CurseForge-Modpacks sind gerade nicht verfügbar".into())
        })?;
        let (manifest, overrides) = {
            let file = std::fs::File::open(path).map_err(Error::io(path))?;
            let mut zip = zip::ZipArchive::new(file).map_err(zip_err(path))?;
            let manifest: CfManifest = {
                let entry = zip.by_name("manifest.json").map_err(zip_err(path))?;
                let mut text = String::new();
                entry
                    .take(MAX_INDEX)
                    .read_to_string(&mut text)
                    .map_err(Error::io(path))?;
                serde_json::from_str(&text).map_err(Error::json("manifest.json"))?
            };
            let prefix = format!("{}/", manifest.overrides.trim_matches('/'));
            let mut overrides = BTreeMap::new();
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i).map_err(zip_err(path))?;
                let name = entry.name().to_string();
                let Some(rel) = name.strip_prefix(&prefix) else {
                    continue;
                };
                if entry.is_dir() || rel.is_empty() {
                    continue;
                }
                safe_relative(rel)?;
                let mut hasher = Sha1::new();
                std::io::copy(&mut entry, &mut hasher).map_err(Error::io(path))?;
                overrides.insert(
                    rel.to_string(),
                    PackFile::Archive {
                        entry: name,
                        sha1: format!("{:x}", hasher.finalize()),
                    },
                );
            }
            (manifest, overrides)
        };
        let loader = cf_loader(&manifest.minecraft.mod_loaders)?;
        let wanted: Vec<&CfManifestFile> = manifest.files.iter().filter(|f| f.required).collect();
        let file_ids: Vec<u64> = wanted.iter().map(|f| f.file_id).collect();
        let mod_ids: Vec<u64> = wanted.iter().map(|f| f.project_id).collect();
        let files: HashMap<u64, curseforge::File> = cf
            .files_by_ids(&file_ids)
            .await?
            .into_iter()
            .map(|f| (f.id, f))
            .collect();
        let mods: HashMap<u64, curseforge::Mod> = cf
            .mods_by_ids(&mod_ids)
            .await?
            .into_iter()
            .map(|m| (m.id, m))
            .collect();

        let mut pack_files = BTreeMap::new();
        let mut missing = Vec::new();
        for w in wanted {
            let (Some(file), Some(info)) = (files.get(&w.file_id), mods.get(&w.project_id)) else {
                return Err(Error::Invalid(format!(
                    "CurseForge kennt Datei {} von Projekt {} nicht",
                    w.file_id, w.project_id
                )));
            };
            let folder = match info.class_id.and_then(curseforge::ClassKind::from_class_id) {
                Some(curseforge::ClassKind::Resourcepack) => "resourcepacks",
                Some(curseforge::ClassKind::Shader) => "shaderpacks",
                _ => "mods",
            };
            let rel = format!("{folder}/{}", file.file_name);
            safe_relative(&rel)?;
            match (&file.download_url, file.sha1()) {
                (Some(url), Some(sha1)) => {
                    pack_files.insert(
                        rel,
                        PackFile::Download {
                            urls: vec![url.clone()],
                            sha1: sha1.to_lowercase(),
                            size: file.file_length,
                        },
                    );
                }
                _ => missing.push(ManualDownload {
                    name: info.name.clone(),
                    path: rel,
                    url: curseforge::manual_download_url(info, file.id),
                }),
            }
        }
        // Overrides nach den Downloads: sie gewinnen bei gleichen Pfaden.
        pack_files.extend(overrides);
        Ok(PackArchive {
            path: path.to_path_buf(),
            name: manifest.name,
            version: manifest.version,
            summary: None,
            minecraft: manifest.minecraft.version,
            loader,
            files: pack_files,
            extra_hosts: curseforge::DOWNLOAD_HOSTS
                .iter()
                .map(|h| h.to_string())
                .collect(),
            missing,
        })
    }

    /// Lädt eine CurseForge-Modpack-Datei (bestimmt oder passend) in den Cache.
    pub async fn download_curseforge(
        &self,
        project_id: u64,
        file_id: Option<u64>,
        minecraft: Option<&str>,
    ) -> Result<(PathBuf, PackSource)> {
        let cf = self
            .curseforge
            .as_ref()
            .ok_or_else(|| Error::Invalid("CurseForge ist nicht eingerichtet".into()))?;
        let file = match file_id {
            Some(id) => cf.files_by_ids(&[id]).await?.into_iter().next(),
            None => curseforge::best_file(&cf.files(project_id, minecraft, None).await?).cloned(),
        }
        .ok_or_else(|| Error::Invalid("Keine passende Modpack-Datei bei CurseForge".into()))?;
        let url = file.download_url.clone().ok_or_else(|| {
            Error::Invalid("Dieses Modpack darf nur über die CurseForge-App geladen werden".into())
        })?;
        let dest = self.cache_dir().join(format!("cf-{}.zip", file.id));
        self.downloader
            .download(&DownloadTask {
                url,
                dest: dest.clone(),
                sha1: file.sha1().map(str::to_string),
                size: Some(file.file_length),
            })
            .await?;
        Ok((
            dest,
            PackSource::Curseforge {
                project_id,
                file_id: file.id,
            },
        ))
    }

    /// Nur für Tests: weitere Download-Hosts erlauben (z. B. einen lokalen Mock-Server).
    pub fn allow_host(mut self, host: &str) -> Self {
        self.allowed_hosts.push(host.to_string());
        self
    }

    fn cache_dir(&self) -> PathBuf {
        self.paths.root().join("cache").join("packs")
    }

    fn snapshots_dir(&self, profile_id: &str) -> PathBuf {
        self.paths.instance_dir(profile_id).join("snapshots")
    }

    fn allowed_url<'a>(&self, urls: &'a [String], extra_hosts: &[String]) -> Result<&'a str> {
        urls.iter()
            .find(|u| {
                reqwest::Url::parse(u).is_ok_and(|url| {
                    matches!(url.scheme(), "https" | "http")
                        && url.host_str().is_some_and(|h| {
                            self.allowed_hosts.iter().chain(extra_hosts).any(|a| a == h)
                        })
                })
            })
            .map(String::as_str)
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "Download-Quelle nicht erlaubt: {}",
                    urls.first().map(String::as_str).unwrap_or("–")
                ))
            })
    }

    /// Lädt eine Modpack-Version von Modrinth (passend oder bestimmt) in den Cache.
    pub async fn download_modrinth(
        &self,
        project: &str,
        version_id: Option<&str>,
    ) -> Result<(PathBuf, PackSource)> {
        let version = match version_id {
            Some(id) => self.modrinth.version(id).await?,
            None => {
                let versions = self.modrinth.versions(project, &[], &[]).await?;
                modrinth::best_version(&versions)
                    .cloned()
                    .ok_or_else(|| Error::Invalid("Modpack hat keine Version".into()))?
            }
        };
        let file = version
            .files
            .iter()
            .find(|f| f.primary && f.filename.ends_with(".mrpack"))
            .or_else(|| {
                version
                    .files
                    .iter()
                    .find(|f| f.filename.ends_with(".mrpack"))
            })
            .ok_or_else(|| Error::Invalid("Diese Version enthält kein .mrpack".into()))?;
        let dest = self.cache_dir().join(format!("{}.mrpack", version.id));
        self.downloader
            .download(&DownloadTask {
                url: file.url.clone(),
                dest: dest.clone(),
                sha1: Some(file.hashes.sha1.clone()),
                size: Some(file.size),
            })
            .await?;
        Ok((
            dest,
            PackSource::Modrinth {
                project_id: version.project_id,
                version_id: version.id,
            },
        ))
    }

    /// Schreibt Pack-Dateien in den Spielordner. `targets`: (Zielpfad relativ, Datei).
    async fn write_files(
        &self,
        game_dir: &Path,
        archive: &Path,
        extra_hosts: &[String],
        targets: &[(String, PackFile)],
    ) -> Result<()> {
        let mut tasks = Vec::new();
        for (dest, file) in targets {
            safe_relative(dest)?;
            if let PackFile::Download { urls, sha1, size } = file {
                tasks.push(DownloadTask {
                    url: self.allowed_url(urls, extra_hosts)?.to_string(),
                    dest: game_dir.join(dest),
                    sha1: Some(sha1.clone()),
                    size: Some(*size),
                });
            }
        }
        self.downloader.download_all(tasks, |_| {}).await?;

        let archive = archive.to_path_buf();
        let game_dir = game_dir.to_path_buf();
        let extract: Vec<(String, String, String)> = targets
            .iter()
            .filter_map(|(dest, f)| match f {
                PackFile::Archive { entry, sha1 } => {
                    Some((dest.clone(), entry.clone(), sha1.clone()))
                }
                PackFile::Download { .. } => None,
            })
            .collect();
        tokio::task::spawn_blocking(move || -> Result<()> {
            if extract.is_empty() {
                return Ok(());
            }
            let file = std::fs::File::open(&archive).map_err(Error::io(&archive))?;
            let mut zip = zip::ZipArchive::new(file).map_err(zip_err(&archive))?;
            for (dest, entry, sha1) in extract {
                let target = game_dir.join(&dest);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
                }
                let mut data = Vec::new();
                zip.by_name(&entry)
                    .map_err(zip_err(&archive))?
                    .read_to_end(&mut data)
                    .map_err(Error::io(&archive))?;
                let actual = format!("{:x}", Sha1::digest(&data));
                if actual != sha1 {
                    return Err(Error::HashMismatch {
                        path: target,
                        expected: sha1,
                        actual,
                    });
                }
                std::fs::write(&target, data).map_err(Error::io(&target))?;
            }
            Ok(())
        })
        .await
        .map_err(|e| Error::Invalid(format!("Entpacken abgebrochen: {e}")))?
    }

    /// Hält Pack-Herkunft und Basis fest und markiert Pack-Inhalte.
    async fn finalize_lock(
        &self,
        profile_id: &str,
        game_dir: &Path,
        pack: &PackArchive,
        source: PackSource,
    ) -> Result<()> {
        self.content.identify(profile_id, game_dir).await?;
        let mut lock = self.content.load_lock(profile_id)?;
        for (key, entry) in lock.files.iter_mut() {
            entry.from_pack = pack.files.contains_key(key);
        }
        let event = self
            .cached_manifest(&source)
            .map(|m| EventLock::from_manifest(&m));
        // Schritt 9: jede Pflichtdatei muss nach der Installation stimmen.
        if let Some(event) = &event {
            for (path, sha512) in &event.locked {
                let file = game_dir.join(path);
                let actual = crate::eventpack::sha512_file(&file).map_err(|_| {
                    Error::Invalid(format!(
                        "Pflichtdatei {path} fehlt nach der Installation, bitte melde das dem Bauteam"
                    ))
                })?;
                if !actual.eq_ignore_ascii_case(sha512) {
                    return Err(Error::HashMismatch {
                        path: file,
                        expected: sha512.clone(),
                        actual,
                    });
                }
            }
        }
        lock.pack = Some(PackInfo {
            name: pack.name.clone(),
            version: pack.version.clone(),
            source,
            files: pack
                .files
                .iter()
                .map(|(k, f)| (k.clone(), f.sha1().to_string()))
                .collect(),
            event,
        });
        self.content.save_lock(profile_id, &lock)
    }

    /// Legt aus einem Pack ein neues Profil an. Schlägt etwas fehl, wird es wieder entfernt.
    pub async fn import(
        &self,
        archive: &Path,
        source: PackSource,
        name: Option<String>,
    ) -> Result<ImportResult> {
        let pack = self.read_archive(archive).await?;
        let base = name.unwrap_or_else(|| pack.name.clone());
        let mut name = base.clone();
        let mut n = 2;
        while self.profiles.find(&name)?.is_some() {
            name = format!("{base} ({n})");
            n += 1;
        }
        let mut profile = Profile::new(name, pack.minecraft.clone());
        profile.loader = pack.loader.clone();
        let profile = self.profiles.create(profile)?;
        let game_dir = self.profiles.game_dir(&profile.id);
        let icon_source = source.clone();

        let result = async {
            let targets: Vec<(String, PackFile)> = pack
                .files
                .iter()
                .map(|(k, f)| (k.clone(), f.clone()))
                .collect();
            self.write_files(&game_dir, archive, &pack.extra_hosts, &targets)
                .await?;
            self.finalize_lock(&profile.id, &game_dir, &pack, source)
                .await
        }
        .await;
        if let Err(e) = result {
            let _ = self.profiles.delete(&profile.id);
            return Err(e);
        }
        // Logo der Quelle als Instanz-Symbol; fehlt es oder klappt der Abruf nicht, bleiben die
        // Anfangsbuchstaben.
        let mut profile = profile;
        match self.source_icon(&icon_source).await {
            Ok(Some(icon)) => {
                profile.icon = Some(icon);
                if let Err(e) = self.profiles.save(&profile) {
                    tracing::warn!("Instanz-Symbol nicht gespeichert: {e}");
                }
            }
            Ok(None) => {}
            Err(e) => tracing::warn!("Pack-Logo nicht geladen: {e}"),
        }
        Ok(ImportResult {
            profile,
            missing: pack.missing,
        })
    }

    /// Pack-Instanzen ohne Symbol (vor dem Logo-Import angelegt) bekommen das Logo ihrer Quelle.
    /// Liefert, wie viele Instanzen ein Symbol bekommen haben.
    pub async fn fill_missing_icons(&self) -> Result<usize> {
        let mut filled = 0;
        for mut profile in self.profiles.list()? {
            if profile.icon.is_some() {
                continue;
            }
            let Some(info) = self.content.load_lock(&profile.id)?.pack else {
                continue;
            };
            match self.source_icon(&info.source).await {
                Ok(Some(icon)) => {
                    profile.icon = Some(icon);
                    self.profiles.save(&profile)?;
                    filled += 1;
                }
                Ok(None) => {}
                Err(e) => tracing::warn!("Pack-Logo für {} nicht geladen: {e}", profile.id),
            }
        }
        Ok(filled)
    }

    /// Logo eines Packs an seiner Quelle (Modrinth, CurseForge, TwoGG-Server) als kleines PNG
    /// (`data:`-URL), passend für das Instanz-Symbol.
    pub async fn source_icon(&self, source: &PackSource) -> Result<Option<String>> {
        let url = match source {
            PackSource::Modrinth { project_id, .. } => {
                self.modrinth.project(project_id).await?.icon_url
            }
            PackSource::Curseforge { project_id, .. } => match &self.curseforge {
                Some(cf) => cf
                    .mods_by_ids(&[*project_id])
                    .await?
                    .into_iter()
                    .next()
                    .and_then(|m| m.logo)
                    .and_then(|l| l.thumbnail_url),
                None => None,
            },
            PackSource::Twogg { pack_id, .. } => match &self.twogg {
                Some(client) => client.pack(pack_id).await?.icon_url,
                None => None,
            },
            PackSource::File => None,
        };
        let Some(url) = url.filter(|u| u.starts_with("https://") || u.starts_with("http://"))
        else {
            return Ok(None);
        };
        let bytes = self.downloader.fetch_bytes(&url).await?;
        icon_data_url(&bytes).map(Some)
    }

    /// Neuere Pack-Version für dieselbe Minecraft-Version und denselben Loader?
    /// Ein Wechsel der Minecraft-Version ist kein Update, sondern ein neues Profil.
    pub async fn check_update(&self, profile: &Profile) -> Result<Option<PackUpdate>> {
        let lock = self.content.load_lock(&profile.id)?;
        let Some(info) = lock.pack else {
            return Ok(None);
        };
        match info.source {
            PackSource::Modrinth {
                project_id,
                version_id,
            } => {
                let loaders: Vec<String> = profile
                    .loader
                    .as_ref()
                    .map(|l| vec![l.kind.id().to_string()])
                    .unwrap_or_default();
                let versions = self
                    .modrinth
                    .versions(
                        &project_id,
                        &loaders,
                        std::slice::from_ref(&profile.minecraft),
                    )
                    .await?;
                let Some(best) = modrinth::best_version(&versions) else {
                    return Ok(None);
                };
                // Liste kommt neueste zuerst: nur anbieten, wenn die Kandidatin vor der installierten liegt.
                let position = |id: &str| {
                    versions
                        .iter()
                        .position(|v| v.id == id)
                        .unwrap_or(usize::MAX)
                };
                if best.id == version_id || position(&best.id) > position(&version_id) {
                    return Ok(None);
                }
                Ok(Some(PackUpdate {
                    current: info.version,
                    latest: best.version_number.clone(),
                    version_id: best.id.clone(),
                    withdrawn: None,
                }))
            }
            PackSource::Curseforge {
                project_id,
                file_id,
            } => {
                let Some(cf) = &self.curseforge else {
                    return Ok(None);
                };
                let files = cf.files(project_id, Some(&profile.minecraft), None).await?;
                let current_date = files
                    .iter()
                    .find(|f| f.id == file_id)
                    .map(|f| f.file_date.clone());
                Ok(curseforge::best_file(&files)
                    .filter(|f| {
                        f.id != file_id && current_date.as_ref().is_none_or(|d| f.file_date > *d)
                    })
                    .map(|f| PackUpdate {
                        current: info.version,
                        latest: f.display_name.clone(),
                        version_id: f.id.to_string(),
                        withdrawn: None,
                    }))
            }
            PackSource::Twogg {
                pack_id,
                version_id,
            } => {
                let Some(client) = &self.twogg else {
                    return Ok(None);
                };
                let pack = client.pack(&pack_id).await?;
                // Zurückgezogen (F-62): Grund melden und die beste verfügbare Version anbieten,
                // auch wenn sie älter ist.
                let withdrawn = pack
                    .withdrawn
                    .iter()
                    .find(|w| w.id == version_id)
                    .map(|w| w.reason.clone());
                let versions = pack.versions.unwrap_or_default();
                let best = crate::twogg::best_version(&versions, Some(&profile.minecraft));
                if let Some(reason) = withdrawn {
                    return Ok(Some(PackUpdate {
                        current: info.version,
                        latest: best.map(|b| b.version.clone()).unwrap_or_default(),
                        version_id: best.map(|b| b.id.clone()).unwrap_or_default(),
                        withdrawn: Some(reason),
                    }));
                }
                let Some(best) = best else {
                    return Ok(None);
                };
                // Liste kommt neueste zuerst: nur anbieten, wenn die Kandidatin neuer ist.
                let position = |id: &str| {
                    versions
                        .iter()
                        .position(|v| v.id == id)
                        .unwrap_or(usize::MAX)
                };
                if best.id == version_id || position(&best.id) > position(&version_id) {
                    return Ok(None);
                }
                Ok(Some(PackUpdate {
                    current: info.version,
                    latest: best.version.clone(),
                    version_id: best.id.clone(),
                    withdrawn: None,
                }))
            }
            PackSource::File => Ok(None),
        }
    }

    /// Plant ein Update gegen das neue Pack, ohne etwas zu ändern.
    pub async fn plan_update(&self, profile: &Profile, new: &PackArchive) -> Result<UpdatePlan> {
        let lock = self.content.load_lock(&profile.id)?;
        let info = lock.pack.ok_or_else(|| {
            Error::Invalid(format!("{:?} stammt aus keinem Modpack", profile.name))
        })?;
        let game_dir = self.profiles.game_dir(&profile.id);
        let base = &info.files;
        let paths: BTreeSet<&String> = base.keys().chain(new.files.keys()).collect();

        let (mut changes, mut conflicts, mut kept) = (Vec::new(), Vec::new(), Vec::new());
        for path in paths {
            let b = base.get(path).map(String::as_str);
            let n = new.files.get(path).map(PackFile::sha1);
            let c = current(&game_dir, path).await?;
            let c_hash = c.as_ref().map(|(_, h)| h.as_str());
            let conflict = |reason| Conflict {
                path: path.clone(),
                reason,
                default: if is_user_setting(path)
                    || matches!(
                        reason,
                        ConflictReason::ChangedButRemoved | ConflictReason::UserRemoved
                    ) {
                    Resolution::KeepMine
                } else {
                    Resolution::TakePack
                },
            };
            match (b, n, c_hash) {
                (_, Some(n), Some(c)) if c == n => {}
                (Some(b), Some(_), Some(c)) if c == b => changes.push(PlannedChange {
                    path: path.clone(),
                    action: ChangeAction::Replace,
                }),
                (Some(b), Some(n), Some(_)) if n == b => kept.push(path.clone()),
                (Some(_), Some(_), Some(_)) => {
                    conflicts.push(conflict(ConflictReason::BothChanged))
                }
                (Some(b), Some(n), None) if n == b => {}
                (Some(_), Some(_), None) => conflicts.push(conflict(ConflictReason::UserRemoved)),
                (None, Some(_), None) => changes.push(PlannedChange {
                    path: path.clone(),
                    action: ChangeAction::Add,
                }),
                (None, Some(_), Some(_)) => {
                    conflicts.push(conflict(ConflictReason::UserFileExists))
                }
                (Some(b), None, Some(c)) if c == b => changes.push(PlannedChange {
                    path: path.clone(),
                    action: ChangeAction::Remove,
                }),
                (Some(_), None, Some(_)) => {
                    conflicts.push(conflict(ConflictReason::ChangedButRemoved))
                }
                (_, None, None) | (None, None, Some(_)) => {}
            }
        }
        Ok(UpdatePlan {
            pack_name: new.name.clone(),
            from_version: info.version,
            to_version: new.version.clone(),
            minecraft: new.minecraft.clone(),
            loader: new.loader.clone(),
            changes,
            conflicts,
            kept,
        })
    }

    /// Wendet ein Update an (Plan wird neu berechnet, nicht aus der Oberfläche übernommen).
    /// `resolutions`: Entscheidung je Konfliktpfad, sonst gilt der Vorschlag.
    pub async fn apply_update(
        &self,
        profile: &Profile,
        archive: &Path,
        source: PackSource,
        resolutions: &HashMap<String, Resolution>,
    ) -> Result<SnapshotInfo> {
        let new = self.read_archive(archive).await?;
        let plan = self.plan_update(profile, &new).await?;
        let game_dir = self.profiles.game_dir(&profile.id);
        let lock_before = self.content.load_lock(&profile.id)?;

        // Was geschrieben bzw. gelöscht wird (Ziel jeweils mit evtl. vorhandenem `.disabled`).
        let mut writes: Vec<(String, PackFile)> = Vec::new();
        let mut removes: Vec<String> = Vec::new();
        let take = |path: &str,
                    remove: bool,
                    writes: &mut Vec<(String, PackFile)>,
                    removes: &mut Vec<String>| {
            if remove {
                removes.push(path.to_string());
            } else if let Some(f) = new.files.get(path) {
                writes.push((path.to_string(), f.clone()));
            }
        };
        for c in &plan.changes {
            take(
                &c.path,
                c.action == ChangeAction::Remove,
                &mut writes,
                &mut removes,
            );
        }
        for c in &plan.conflicts {
            if resolutions.get(&c.path).copied().unwrap_or(c.default) == Resolution::TakePack {
                take(
                    &c.path,
                    c.reason == ConflictReason::ChangedButRemoved,
                    &mut writes,
                    &mut removes,
                );
            }
        }

        // Snapshot: betroffene Dateien sichern, neue merken.
        let id = format!("{}", now_secs());
        let snap_dir = self.snapshots_dir(&profile.id).join(&id);
        let mut saved = Vec::new();
        let mut created_files = Vec::new();
        let mut resolved_writes = Vec::new();
        for (path, file) in &writes {
            let actual = match current(&game_dir, path).await? {
                Some((actual, _)) => {
                    backup(&game_dir, &snap_dir, &actual)?;
                    saved.push(actual.clone());
                    actual
                }
                None => {
                    created_files.push(path.clone());
                    path.clone()
                }
            };
            resolved_writes.push((actual, file.clone()));
        }
        let mut resolved_removes = Vec::new();
        for path in &removes {
            if let Some((actual, _)) = current(&game_dir, path).await? {
                backup(&game_dir, &snap_dir, &actual)?;
                saved.push(actual.clone());
                resolved_removes.push(actual);
            }
        }
        let manifest = SnapshotManifest {
            created: now_secs(),
            pack_version: plan.from_version.clone(),
            minecraft: profile.minecraft.clone(),
            loader: profile.loader.clone(),
            saved,
            created_files,
            lock: lock_before,
        };
        std::fs::create_dir_all(&snap_dir).map_err(Error::io(&snap_dir))?;
        let manifest_path = snap_dir.join("snapshot.json");
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).map_err(Error::json("snapshot.json"))?,
        )
        .map_err(Error::io(&manifest_path))?;

        // Änderungen ausführen. Bei Fehlern bleibt der Snapshot für ein Zurückrollen erhalten.
        self.write_files(&game_dir, archive, &new.extra_hosts, &resolved_writes)
            .await?;
        for actual in &resolved_removes {
            let p = game_dir.join(actual);
            std::fs::remove_file(&p).map_err(Error::io(&p))?;
        }
        if profile.minecraft != new.minecraft || profile.loader != new.loader {
            let mut updated = profile.clone();
            updated.minecraft = new.minecraft.clone();
            updated.loader = new.loader.clone();
            self.profiles.save(&updated)?;
        }
        self.finalize_lock(&profile.id, &game_dir, &new, source)
            .await?;
        self.prune_snapshots(&profile.id);
        Ok(SnapshotInfo {
            id,
            created: manifest.created,
            pack_version: manifest.pack_version,
            files: manifest.saved.len() + manifest.created_files.len(),
        })
    }

    pub fn snapshots(&self, profile_id: &str) -> Vec<SnapshotInfo> {
        let Ok(entries) = std::fs::read_dir(self.snapshots_dir(profile_id)) else {
            return Vec::new();
        };
        let mut list: Vec<SnapshotInfo> = entries
            .flatten()
            .filter_map(|e| {
                let bytes = std::fs::read(e.path().join("snapshot.json")).ok()?;
                let m: SnapshotManifest = serde_json::from_slice(&bytes).ok()?;
                Some(SnapshotInfo {
                    id: e.file_name().to_string_lossy().into_owned(),
                    created: m.created,
                    pack_version: m.pack_version,
                    files: m.saved.len() + m.created_files.len(),
                })
            })
            .collect();
        list.sort_by_key(|s| std::cmp::Reverse(s.created));
        list
    }

    fn prune_snapshots(&self, profile_id: &str) {
        for old in self.snapshots(profile_id).into_iter().skip(KEEP_SNAPSHOTS) {
            let _ = std::fs::remove_dir_all(self.snapshots_dir(profile_id).join(old.id));
        }
    }

    /// Stellt den Stand vor einem Update wieder her und entfernt den Snapshot.
    pub fn rollback(&self, profile: &Profile, snapshot_id: &str) -> Result<()> {
        if snapshot_id.is_empty() || !snapshot_id.chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::Invalid(format!(
                "Ungültiger Snapshot {snapshot_id:?}"
            )));
        }
        let snap_dir = self.snapshots_dir(&profile.id).join(snapshot_id);
        let manifest_path = snap_dir.join("snapshot.json");
        let bytes = std::fs::read(&manifest_path).map_err(Error::io(&manifest_path))?;
        let m: SnapshotManifest =
            serde_json::from_slice(&bytes).map_err(Error::json("snapshot.json"))?;
        let game_dir = self.profiles.game_dir(&profile.id);
        for rel in &m.created_files {
            safe_relative(rel)?;
            for p in [
                game_dir.join(rel),
                game_dir.join(format!("{rel}{DISABLED_SUFFIX}")),
            ] {
                if p.is_file() {
                    std::fs::remove_file(&p).map_err(Error::io(&p))?;
                }
            }
        }
        for rel in &m.saved {
            safe_relative(rel)?;
            let target = game_dir.join(rel);
            // Ein inzwischen umbenanntes Gegenstück (aktiv ↔ deaktiviert) entfernen.
            let twin = match rel.strip_suffix(DISABLED_SUFFIX) {
                Some(base) => game_dir.join(base),
                None => game_dir.join(format!("{rel}{DISABLED_SUFFIX}")),
            };
            let _ = std::fs::remove_file(&twin);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
            }
            std::fs::copy(snap_dir.join("files").join(rel), &target).map_err(Error::io(&target))?;
        }
        let mut restored = profile.clone();
        restored.minecraft = m.minecraft;
        restored.loader = m.loader;
        self.profiles.save(&restored)?;
        self.content.save_lock(&profile.id, &m.lock)?;
        std::fs::remove_dir_all(&snap_dir).map_err(Error::io(&snap_dir))
    }

    /// Exportiert ein Profil als `.mrpack`: Modrinth-Inhalte als Links, eigene Dateien und
    /// gewählte Ordner direkt im Archiv. Deaktivierte Inhalte bleiben draußen.
    pub async fn export(
        &self,
        profile: &Profile,
        target: &Path,
        opts: &ExportOptions,
    ) -> Result<ExportReport> {
        let version_label = opts.version.trim();
        if version_label.is_empty() {
            return Err(Error::Invalid("Bitte eine Versionsnummer angeben".into()));
        }
        let game_dir = self.profiles.game_dir(&profile.id);
        let installed = self.content.identify(&profile.id, &game_dir).await?;
        let lock = self.content.load_lock(&profile.id)?;
        let installed: Vec<_> = installed
            .into_iter()
            .filter(|c| c.enabled && opts.wants(c.kind) && (opts.sync_content || c.sync.is_none()))
            .collect();
        let version_ids: Vec<String> = installed
            .iter()
            .filter_map(|c| match &c.source {
                Source::Modrinth { version_id, .. } => Some(version_id.clone()),
                _ => None,
            })
            .collect();
        let versions: HashMap<String, modrinth::Version> = self
            .modrinth
            .versions_by_ids(&version_ids)
            .await?
            .into_iter()
            .map(|v| (v.id.clone(), v))
            .collect();

        let mut files = Vec::new();
        let mut include: Vec<String> = Vec::new();
        for c in &installed {
            let sha1 = lock
                .files
                .get(&c.key)
                .map(|e| e.sha1.as_str())
                .unwrap_or_default();
            let linked = match &c.source {
                Source::Modrinth { version_id, .. } => versions
                    .get(version_id)
                    .and_then(|v| v.files.iter().find(|f| f.hashes.sha1 == sha1))
                    .and_then(|f| {
                        Some(IndexFile {
                            path: c.key.clone(),
                            hashes: Hashes {
                                sha1: f.hashes.sha1.clone(),
                                sha512: f.hashes.sha512.clone()?,
                            },
                            env: None,
                            downloads: vec![f.url.clone()],
                            file_size: f.size,
                        })
                    }),
                _ => None,
            };
            match linked {
                Some(f) => files.push(f),
                // Unbekannt oder verändert: Datei selbst mitnehmen.
                None => include.push(c.key.clone()),
            }
        }
        if opts.config {
            include.extend(walk_files(&game_dir, "config"));
        }
        if opts.options {
            include.extend(
                OPTION_FILES
                    .into_iter()
                    .filter(|f| game_dir.join(f).is_file())
                    .map(str::to_string),
            );
        }

        let mut dependencies =
            BTreeMap::from([("minecraft".to_string(), profile.minecraft.clone())]);
        if let Some(l) = &profile.loader {
            let key = match l.kind {
                LoaderKind::Fabric => "fabric-loader",
                LoaderKind::Quilt => "quilt-loader",
                LoaderKind::Forge => "forge",
                LoaderKind::NeoForge => "neoforge",
            };
            dependencies.insert(key.into(), l.version.clone());
        }
        let index = MrpackIndex {
            format_version: 1,
            game: "minecraft".into(),
            version_id: version_label.to_string(),
            name: profile.name.clone(),
            summary: opts
                .summary
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            files,
            dependencies,
        };
        let report = ExportReport {
            path: target.display().to_string(),
            linked: index.files.len(),
            included: include.len(),
        };

        let target = target.to_path_buf();
        tokio::task::spawn_blocking(move || -> Result<()> {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
            }
            let out = std::fs::File::create(&target).map_err(Error::io(&target))?;
            let mut zip = zip::ZipWriter::new(out);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file(INDEX, opts).map_err(zip_err(&target))?;
            zip.write_all(&serde_json::to_vec_pretty(&index).map_err(Error::json(INDEX))?)
                .map_err(Error::io(&target))?;
            for rel in include {
                let data =
                    std::fs::read(game_dir.join(&rel)).map_err(Error::io(game_dir.join(&rel)))?;
                zip.start_file(format!("overrides/{rel}"), opts)
                    .map_err(zip_err(&target))?;
                zip.write_all(&data).map_err(Error::io(&target))?;
            }
            zip.finish().map_err(zip_err(&target))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::Invalid(format!("Export abgebrochen: {e}")))??;
        Ok(report)
    }

    pub fn pack_info(&self, profile_id: &str) -> Result<Option<PackInfo>> {
        Ok(self.content.load_lock(profile_id)?.pack)
    }

    /// Inhaltsart einer Pack-Datei (für die Anzeige), falls es eine ist.
    pub fn kind_of(path: &str) -> Option<ContentKind> {
        ContentKind::ALL
            .into_iter()
            .find(|k| path.starts_with(&format!("{}/", k.folder())))
    }
}

fn backup(game_dir: &Path, snap_dir: &Path, rel: &str) -> Result<()> {
    let target = snap_dir.join("files").join(rel);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
    }
    std::fs::copy(game_dir.join(rel), &target).map_err(Error::io(&target))?;
    Ok(())
}

/// Alle Dateien unter `<root>/<dir>` als relative Pfade (mit `/`), ohne Symlinks.
fn walk_files(root: &Path, dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_string()];
    while let Some(rel) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(root.join(&rel)) else {
            continue;
        };
        for e in entries.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            let child = format!("{rel}/{}", e.file_name().to_string_lossy());
            if ft.is_dir() {
                stack.push(child);
            } else if ft.is_file() {
                out.push(child);
            }
        }
    }
    out.sort();
    out
}

/// Größte Kantenlänge gespeicherter Instanz-Symbole (Pixel).
const ICON_SIZE: u32 = 96;

/// Bild (PNG, JPEG, WebP, GIF) auf höchstens [`ICON_SIZE`] verkleinern und als PNG-`data:`-URL
/// liefern. Kleine Pixel-Logos werden nicht weichgezeichnet.
pub fn icon_data_url(bytes: &[u8]) -> Result<String> {
    use base64::Engine;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(Error::Invalid("Logo ist zu groß".into()));
    }
    let img = image::load_from_memory(bytes)
        .map_err(|e| Error::Invalid(format!("Logo nicht lesbar: {e}")))?;
    let img = if img.width() > ICON_SIZE || img.height() > ICON_SIZE {
        img.resize(ICON_SIZE, ICON_SIZE, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let mut png = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| Error::Invalid(format!("Logo nicht umwandelbar: {e}")))?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_is_shrunk_to_png_data_url() {
        let img = image::RgbaImage::from_pixel(400, 200, image::Rgba([200, 120, 40, 255]));
        let mut jpeg = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .to_rgb8()
            .write_to(
                &mut std::io::Cursor::new(&mut jpeg),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        let url = icon_data_url(&jpeg).unwrap();
        use base64::Engine;
        let png = base64::engine::general_purpose::STANDARD
            .decode(url.strip_prefix("data:image/png;base64,").unwrap())
            .unwrap();
        let back = image::load_from_memory(&png).unwrap();
        assert_eq!((back.width(), back.height()), (ICON_SIZE, ICON_SIZE / 2));
        assert!(icon_data_url(b"kein Bild").is_err());
    }

    #[test]
    fn settings_classification() {
        assert!(is_user_setting("options.txt"));
        assert!(is_user_setting("config/sodium-options.json"));
        assert!(!is_user_setting("mods/sodium.jar"));
        assert!(!is_user_setting("resourcepacks/x.zip"));
    }

    #[test]
    fn loader_dependencies() {
        let deps = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        };
        assert_eq!(
            loader_from_deps(&deps(&[
                ("minecraft", "1.21.1"),
                ("fabric-loader", "0.19.3")
            ]))
            .unwrap(),
            Some(LoaderSpec {
                kind: LoaderKind::Fabric,
                version: "0.19.3".into()
            })
        );
        assert_eq!(
            loader_from_deps(&deps(&[("minecraft", "1.21.1")])).unwrap(),
            None
        );
        assert_eq!(
            loader_from_deps(&deps(&[("neoforge", "21.1.1")])).unwrap(),
            Some(LoaderSpec {
                kind: LoaderKind::NeoForge,
                version: "21.1.1".into()
            })
        );
    }
}
