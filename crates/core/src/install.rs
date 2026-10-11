//! Installation einer Version: Version-JSON, Client, Libraries, Natives und Assets.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::download::{DownloadTask, Downloader, Progress};
use crate::manifest::{VERSION_MANIFEST_URL, VersionJson, VersionManifest};
use crate::paths::{Paths, safe_relative};
use crate::rules::Environment;
use crate::{Error, Result};

pub const RESOURCES_URL: &str = "https://resources.download.minecraft.net/";

#[derive(Debug, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
    /// 1.6–1.7.2 (`legacy`): Assets unter `assets/virtual/<index>/<name>` erwartet.
    #[serde(default, rename = "virtual")]
    pub is_virtual: bool,
    /// Vor 1.6 (`pre-1.6`): Assets unter `<spielordner>/resources/<name>` erwartet.
    #[serde(default)]
    pub map_to_resources: bool,
}

#[derive(Debug, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

pub struct Installer {
    paths: Paths,
    downloader: Downloader,
    env: Environment,
}

impl Installer {
    pub fn new(paths: Paths, downloader: Downloader) -> Self {
        Self {
            paths,
            downloader,
            env: Environment::current(),
        }
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn environment(&self) -> &Environment {
        &self.env
    }

    /// Lädt das Versionsmanifest. Ohne Netz wird die zuletzt gespeicherte Kopie genutzt.
    pub async fn manifest(&self) -> Result<VersionManifest> {
        let cache = self.paths.version_manifest();
        match self.downloader.fetch_bytes(VERSION_MANIFEST_URL).await {
            Ok(bytes) => {
                let manifest =
                    serde_json::from_slice(&bytes).map_err(Error::json(VERSION_MANIFEST_URL))?;
                write_file(&cache, &bytes).await?;
                Ok(manifest)
            }
            Err(e) => {
                tracing::warn!("Manifest nicht erreichbar, nutze Cache: {e}");
                let bytes = tokio::fs::read(&cache).await.map_err(|_| e)?;
                serde_json::from_slice(&bytes).map_err(Error::json(cache.display().to_string()))
            }
        }
    }

    /// Installiert eine Version vollständig und liefert das (gemergte) Version-JSON.
    pub async fn install(
        &self,
        id: &str,
        on_progress: impl FnMut(Progress),
    ) -> Result<VersionJson> {
        let id = self.resolve_alias(id).await?;
        let version = self.ensure_version_json(&id).await?;
        let tasks = self.collect_tasks(&version).await?;
        self.downloader.download_all(tasks, on_progress).await?;
        self.extract_natives(&version).await?;
        Ok(version)
    }

    /// Liest eine bereits installierte Version, ohne Netzwerk.
    pub async fn load_installed(&self, id: &str) -> Result<VersionJson> {
        let path = self.paths.version_json(id);
        let bytes = match tokio::fs::read(&path).await {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::NotInstalled(id.into()));
            }
            Err(e) => return Err(Error::io(&path)(e)),
        };
        let version: VersionJson =
            serde_json::from_slice(&bytes).map_err(Error::json(path.display().to_string()))?;
        match version.inherits_from.clone() {
            Some(parent) => Ok(version.merge_onto(Box::pin(self.load_installed(&parent)).await?)),
            None => Ok(version),
        }
    }

    async fn resolve_alias(&self, id: &str) -> Result<String> {
        if matches!(id, "latest" | "release" | "latest-snapshot" | "snapshot") {
            let manifest = self.manifest().await?;
            return manifest
                .find(id)
                .map(|v| v.id.clone())
                .ok_or_else(|| Error::UnknownVersion(id.into()));
        }
        Ok(id.to_string())
    }

    async fn ensure_version_json(&self, id: &str) -> Result<VersionJson> {
        let path = self.paths.version_json(id);
        if !path.exists() {
            let manifest = self.manifest().await?;
            let entry = manifest
                .find(id)
                .ok_or_else(|| Error::UnknownVersion(id.into()))?;
            self.downloader
                .download(&DownloadTask {
                    url: entry.url.clone(),
                    dest: path.clone(),
                    sha1: Some(entry.sha1.clone()),
                    size: None,
                })
                .await?;
        }
        let bytes = tokio::fs::read(&path).await.map_err(Error::io(&path))?;
        let version: VersionJson =
            serde_json::from_slice(&bytes).map_err(Error::json(path.display().to_string()))?;
        match version.inherits_from.clone() {
            Some(parent) => {
                Ok(version.merge_onto(Box::pin(self.ensure_version_json(&parent)).await?))
            }
            None => Ok(version),
        }
    }

    async fn collect_tasks(&self, version: &VersionJson) -> Result<Vec<DownloadTask>> {
        let mut tasks = Vec::new();

        if let Some(client) = version.downloads.as_ref().and_then(|d| d.client.as_ref()) {
            tasks.push(DownloadTask {
                url: client.url.clone(),
                dest: self.paths.client_jar(version.jar_id()),
                sha1: client.sha1.clone(),
                size: client.size,
            });
        }

        let libraries = self.paths.libraries();
        for lib in version.libraries.iter().filter(|l| l.applies(&self.env)) {
            for artifact in [lib.artifact(), lib.native_artifact(&self.env)]
                .into_iter()
                .flatten()
            {
                tasks.push(DownloadTask {
                    dest: libraries.join(&artifact.path),
                    url: artifact.url,
                    sha1: artifact.sha1,
                    size: artifact.size,
                });
            }
        }

        if let Some(index_ref) = &version.asset_index {
            let index_path = self.paths.asset_index(&index_ref.id);
            self.downloader
                .download(&DownloadTask {
                    url: index_ref.url.clone(),
                    dest: index_path.clone(),
                    sha1: Some(index_ref.sha1.clone()),
                    size: Some(index_ref.size),
                })
                .await?;
            let bytes = tokio::fs::read(&index_path)
                .await
                .map_err(Error::io(&index_path))?;
            let index: AssetIndex = serde_json::from_slice(&bytes)
                .map_err(Error::json(index_path.display().to_string()))?;
            for object in index.objects.into_values() {
                tasks.push(DownloadTask {
                    url: format!("{RESOURCES_URL}{}/{}", &object.hash[..2], object.hash),
                    dest: self.paths.asset_object(&object.hash),
                    sha1: Some(object.hash),
                    size: Some(object.size),
                });
            }
        }

        Ok(tasks)
    }

    /// Legt für alte Asset-Formate die erwarteten Klardateien an und liefert `${game_assets}`.
    pub async fn prepare_assets(&self, version: &VersionJson, game_dir: &Path) -> Result<PathBuf> {
        let Some(index_ref) = &version.asset_index else {
            return Ok(self.paths.assets());
        };
        let index_path = self.paths.asset_index(&index_ref.id);
        let bytes = tokio::fs::read(&index_path)
            .await
            .map_err(Error::io(&index_path))?;
        let index: AssetIndex = serde_json::from_slice(&bytes)
            .map_err(Error::json(index_path.display().to_string()))?;

        let target = if index.map_to_resources {
            game_dir.join("resources")
        } else if index.is_virtual {
            self.paths.virtual_assets(&index_ref.id)
        } else {
            return Ok(self.paths.assets());
        };
        let copies: Vec<(PathBuf, PathBuf, u64)> = index
            .objects
            .into_iter()
            .map(|(name, obj)| {
                Ok((
                    self.paths.asset_object(&obj.hash),
                    target.join(safe_relative(&name)?),
                    obj.size,
                ))
            })
            .collect::<Result<_>>()?;
        let result = target.clone();
        tokio::task::spawn_blocking(move || {
            for (src, dest, size) in copies {
                if std::fs::metadata(&dest).is_ok_and(|m| m.len() == size) {
                    continue;
                }
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
                }
                // Hardlink spart Platz; über Dateisystemgrenzen hinweg wird kopiert.
                let _ = std::fs::remove_file(&dest);
                if std::fs::hard_link(&src, &dest).is_err() {
                    std::fs::copy(&src, &dest).map_err(Error::io(&dest))?;
                }
            }
            Ok::<_, Error>(())
        })
        .await
        .map_err(|e| Error::Invalid(format!("Asset-Task abgebrochen: {e}")))??;
        Ok(result)
    }

    /// Entpackt Natives alten Stils (bis 1.18, `natives`-Map) flach in `versions/<id>/natives`.
    /// Neuere `natives-*`-JARs bleiben auf dem Classpath, LWJGL entpackt sie selbst passend zur
    /// Architektur (`-Dorg.lwjgl.system.SharedLibraryExtractPath`). Flach entpackt würden sich
    /// x64- und arm64-Varianten gegenseitig überschreiben.
    pub async fn extract_natives(&self, version: &VersionJson) -> Result<()> {
        let libraries = self.paths.libraries();
        let jars: Vec<_> = version
            .libraries
            .iter()
            .filter(|l| l.applies(&self.env))
            .filter_map(|l| l.native_artifact(&self.env))
            .map(|a| libraries.join(a.path))
            .collect();
        let target = self.paths.natives_dir(&version.id);
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&target).map_err(Error::io(&target))?;
            for jar in jars {
                extract_native_libs(&jar, &target)?;
            }
            Ok(())
        })
        .await
        .map_err(|e| Error::Invalid(format!("Natives-Task abgebrochen: {e}")))?
    }
}

fn extract_native_libs(jar: &Path, target: &Path) -> Result<()> {
    let zip_err = |source| Error::Zip {
        path: jar.to_path_buf(),
        source,
    };
    let file = std::fs::File::open(jar).map_err(Error::io(jar))?;
    let mut archive = zip::ZipArchive::new(file).map_err(zip_err)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(zip_err)?;
        let name = entry.name().to_string();
        if entry.is_dir() || name.starts_with("META-INF/") {
            continue;
        }
        let is_native = [".so", ".dll", ".dylib", ".jnilib"]
            .iter()
            .any(|ext| name.ends_with(ext));
        // Nur den Dateinamen verwenden: flach entpacken und Path-Traversal ausschließen.
        let Some(file_name) = Path::new(&name).file_name() else {
            continue;
        };
        if !is_native {
            continue;
        }
        let dest = target.join(file_name);
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf).map_err(Error::io(&dest))?;
        std::fs::write(&dest, buf).map_err(Error::io(&dest))?;
    }
    Ok(())
}

async fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(Error::io(parent))?;
    }
    tokio::fs::write(path, bytes).await.map_err(Error::io(path))
}
