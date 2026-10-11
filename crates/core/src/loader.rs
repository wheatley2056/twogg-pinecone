//! Mod-Loader: Versionen abfragen und Loader-Profile als Version-JSON installieren. Das Profil erbt
//! per `inheritsFrom` von der Vanilla-Version, der Rest läuft über den Installer. Fabric und Quilt
//! liefern das Profil fertig über ihre Meta-API; Forge und NeoForge kommen über ihre Installer
//! (siehe [`crate::forge`]).

use serde::{Deserialize, Serialize};

use crate::download::Downloader;
use crate::paths::Paths;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    Fabric,
    Quilt,
    Forge,
    #[serde(rename = "neoforge")]
    NeoForge,
}

impl LoaderKind {
    pub const ALL: [LoaderKind; 4] = [
        LoaderKind::Fabric,
        LoaderKind::Quilt,
        LoaderKind::Forge,
        LoaderKind::NeoForge,
    ];

    fn meta(self) -> &'static str {
        match self {
            LoaderKind::Quilt => "https://meta.quiltmc.org/v3",
            _ => "https://meta.fabricmc.net/v2",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            LoaderKind::Fabric => "Fabric",
            LoaderKind::Quilt => "Quilt",
            LoaderKind::Forge => "Forge",
            LoaderKind::NeoForge => "NeoForge",
        }
    }

    /// Schlüssel bei Modrinth und in `.mrpack`-Abhängigkeiten (ohne `-loader`).
    pub fn id(self) -> &'static str {
        match self {
            LoaderKind::Fabric => "fabric",
            LoaderKind::Quilt => "quilt",
            LoaderKind::Forge => "forge",
            LoaderKind::NeoForge => "neoforge",
        }
    }

    /// Forge und NeoForge laufen über deren Installer.
    pub fn is_forge(self) -> bool {
        matches!(self, LoaderKind::Forge | LoaderKind::NeoForge)
    }

    /// ID der installierten Version, z. B. `fabric-loader-0.19.5-26.3`.
    pub fn version_id(self, minecraft: &str, loader: &str) -> String {
        match self {
            LoaderKind::Fabric => format!("fabric-loader-{loader}-{minecraft}"),
            LoaderKind::Quilt => format!("quilt-loader-{loader}-{minecraft}"),
            LoaderKind::Forge => format!("forge-{loader}-{minecraft}"),
            LoaderKind::NeoForge => format!("neoforge-{loader}-{minecraft}"),
        }
    }
}

impl std::str::FromStr for LoaderKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "fabric" => Ok(LoaderKind::Fabric),
            "quilt" => Ok(LoaderKind::Quilt),
            "forge" => Ok(LoaderKind::Forge),
            "neoforge" => Ok(LoaderKind::NeoForge),
            other => Err(Error::Invalid(format!(
                "Unbekannter Mod-Loader {other:?} (fabric, quilt, forge, neoforge)"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoaderSpec {
    pub kind: LoaderKind,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Deserialize)]
struct MetaEntry {
    loader: MetaLoader,
}

#[derive(Debug, Deserialize)]
struct MetaLoader {
    version: String,
    /// Nur bei Fabric vorhanden.
    #[serde(default)]
    stable: Option<bool>,
}

/// Vergleicht Loader-Versionen wie `0.19.5`, `0.20.0-beta.9` (SemVer-artig):
/// Zahlenteile numerisch, eine Vorabversion steht vor der gleichnamigen stabilen Version.
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    fn split(v: &str) -> (Vec<u64>, Option<Vec<&str>>) {
        let (core, pre) = match v.split_once('-') {
            Some((c, p)) => (c, Some(p.split('.').collect())),
            None => (v, None),
        };
        (
            core.split('.').map(|n| n.parse().unwrap_or(0)).collect(),
            pre,
        )
    }
    let ((ca, pa), (cb, pb)) = (split(a), split(b));
    ca.cmp(&cb).then_with(|| match (pa, pb) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(x), Some(y)) => {
            for (l, r) in x.iter().zip(&y) {
                let ord = match (l.parse::<u64>(), r.parse::<u64>()) {
                    (Ok(l), Ok(r)) => l.cmp(&r),
                    _ => l.cmp(r),
                };
                if ord.is_ne() {
                    return ord;
                }
            }
            x.len().cmp(&y.len())
        }
    })
}

/// Quilt kennzeichnet Vorabversionen nur im Namen (`0.20.0-beta.9`).
fn looks_stable(version: &str) -> bool {
    !["-beta", "-alpha", "-pre", "-rc"]
        .iter()
        .any(|t| version.contains(t))
}

pub struct LoaderInstaller {
    paths: Paths,
    downloader: Downloader,
}

impl LoaderInstaller {
    pub fn new(paths: Paths, downloader: Downloader) -> Self {
        Self { paths, downloader }
    }

    /// Loader-Versionen für eine Minecraft-Version, neueste zuerst. Leer, wenn nicht unterstützt.
    pub async fn versions(&self, kind: LoaderKind, minecraft: &str) -> Result<Vec<LoaderVersion>> {
        if kind.is_forge() {
            return crate::forge::versions(&self.downloader, kind, minecraft).await;
        }
        let url = format!("{}/versions/loader/{minecraft}", kind.meta());
        let bytes = self.downloader.fetch_bytes(&url).await?;
        let entries: Vec<MetaEntry> = serde_json::from_slice(&bytes).map_err(Error::json(&url))?;
        let mut versions: Vec<LoaderVersion> = entries
            .into_iter()
            .map(|e| LoaderVersion {
                stable: e
                    .loader
                    .stable
                    .unwrap_or_else(|| looks_stable(&e.loader.version)),
                version: e.loader.version,
            })
            .collect();
        // Quilts Meta-API liefert die Liste unsortiert.
        versions.sort_by(|a, b| compare_versions(&b.version, &a.version));
        Ok(versions)
    }

    /// Neueste stabile Version, sonst die neueste überhaupt (z. B. Quilt-Betas für neue Versionen).
    pub async fn recommended(&self, kind: LoaderKind, minecraft: &str) -> Result<LoaderVersion> {
        if kind.is_forge() {
            return crate::forge::recommended(&self.downloader, kind, minecraft).await;
        }
        let versions = self.versions(kind, minecraft).await?;
        let pick = versions
            .iter()
            .find(|v| v.stable)
            .or(versions.first())
            .cloned();
        pick.ok_or_else(|| {
            Error::Invalid(format!(
                "{} unterstützt Minecraft {minecraft} (noch) nicht",
                kind.name()
            ))
        })
    }

    /// Legt das Loader-Profil als `versions/<id>/<id>.json` ab und liefert die Version-ID.
    /// Die Basisversion und alle Libraries lädt danach `Installer::install(id)`.
    pub async fn install(&self, minecraft: &str, spec: &LoaderSpec) -> Result<String> {
        let id = spec.kind.version_id(minecraft, &spec.version);
        let path = self.paths.version_json(&id);
        if path.exists() {
            return Ok(id);
        }
        if spec.kind.is_forge() {
            crate::forge::install_profile(
                &self.paths,
                &self.downloader,
                spec.kind,
                minecraft,
                &spec.version,
                &id,
            )
            .await?;
            return Ok(id);
        }
        let url = format!(
            "{}/versions/loader/{minecraft}/{}/profile/json",
            spec.kind.meta(),
            spec.version
        );
        let bytes = match self.downloader.fetch_bytes(&url).await {
            Ok(b) => b,
            Err(Error::Http { source, .. })
                if source
                    .status()
                    .is_some_and(|s| s.as_u16() == 400 || s.as_u16() == 404) =>
            {
                return Err(Error::Invalid(format!(
                    "{} {} gibt es nicht für Minecraft {minecraft}",
                    spec.kind.name(),
                    spec.version
                )));
            }
            Err(e) => return Err(e),
        };
        // Prüfen, bevor die Datei abgelegt wird: kaputtes JSON soll nicht als „installiert“ gelten.
        let json: serde_json::Value = serde_json::from_slice(&bytes).map_err(Error::json(&url))?;
        if json.get("id").and_then(|v| v.as_str()) != Some(id.as_str()) {
            return Err(Error::Invalid(format!("Unerwartete Loader-ID von {url}")));
        }
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(Error::io(parent))?;
        }
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(Error::io(&path))?;
        Ok(id)
    }
}

impl LoaderInstaller {
    /// Forge/NeoForge: Prozessoren nach dem Laden der Libraries ausführen (einmalig).
    pub async fn finish(
        &self,
        spec: &LoaderSpec,
        id: &str,
        minecraft: &str,
        java: &std::path::Path,
    ) -> Result<()> {
        if !spec.kind.is_forge() {
            return Ok(());
        }
        crate::forge::run_processors(&self.paths, &self.downloader, id, minecraft, java).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_parsing() {
        assert_eq!(
            LoaderKind::Fabric.version_id("26.3", "0.19.5"),
            "fabric-loader-0.19.5-26.3"
        );
        assert_eq!(
            LoaderKind::Quilt.version_id("1.21.1", "0.26.4"),
            "quilt-loader-0.26.4-1.21.1"
        );
        assert_eq!("Fabric".parse::<LoaderKind>().unwrap(), LoaderKind::Fabric);
        assert_eq!(
            "NeoForge".parse::<LoaderKind>().unwrap(),
            LoaderKind::NeoForge
        );
        assert!("rift".parse::<LoaderKind>().is_err());
        assert_eq!(
            serde_json::to_string(&LoaderKind::NeoForge).unwrap(),
            "\"neoforge\""
        );
    }

    #[test]
    fn version_order() {
        let mut v = vec![
            "0.20.0-beta.9",
            "0.20.0-beta.10",
            "0.24.0",
            "0.20.0",
            "0.16.1-beta.3",
            "0.29.1",
            "0.29.1-rc.1",
            "0.9.2",
        ];
        v.sort_by(|a, b| compare_versions(b, a));
        assert_eq!(
            v,
            [
                "0.29.1",
                "0.29.1-rc.1",
                "0.24.0",
                "0.20.0",
                "0.20.0-beta.10",
                "0.20.0-beta.9",
                "0.16.1-beta.3",
                "0.9.2"
            ]
        );
    }

    #[test]
    fn stability() {
        assert!(looks_stable("0.26.4"));
        assert!(!looks_stable("0.20.0-beta.9"));
        let fabric: Vec<MetaEntry> =
            serde_json::from_str(r#"[{"loader":{"version":"0.19.5","stable":true}},{"loader":{"version":"0.19.4","stable":false}}]"#).unwrap();
        assert_eq!(fabric[1].loader.stable, Some(false));
        let quilt: Vec<MetaEntry> =
            serde_json::from_str(r#"[{"loader":{"version":"0.20.0-beta.9","build":9}}]"#).unwrap();
        assert_eq!(quilt[0].loader.stable, None);
    }
}
