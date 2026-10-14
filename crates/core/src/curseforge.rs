//! CurseForge-API v1. Release-Builds fragen über den Proxy des Pinecone-Servers, der den
//! API-Schlüssel hält (die CurseForge-API-Bedingungen verbieten, ihn weiterzugeben).

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::loader::LoaderKind;
use crate::{Error, LAUNCHER_VERSION, Result};

pub const API: &str = "https://api.curseforge.com";
pub const MINECRAFT: u32 = 432;
/// Hosts, von denen CurseForge Dateien ausliefert.
pub const DOWNLOAD_HOSTS: [&str; 2] = ["edge.forgecdn.net", "mediafilez.forgecdn.net"];

/// Eigener API-Schlüssel aus `PINECONE_CURSEFORGE_API_KEY` (nur Laufzeit).
pub fn api_key_from_env() -> Option<String> {
    // Nur zur Laufzeit (Entwicklung, eigene Builds): Release-Builds enthalten keinen Schlüssel,
    // denn die CurseForge-API-Bedingungen verbieten die Weitergabe. Sie fragen über den
    // Pinecone-Server (`CurseForge::via_server`).
    std::env::var("PINECONE_CURSEFORGE_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClassKind {
    Mod,
    Resourcepack,
    Shader,
    Modpack,
}

impl ClassKind {
    pub fn class_id(self) -> u32 {
        match self {
            ClassKind::Mod => 6,
            ClassKind::Resourcepack => 12,
            ClassKind::Shader => 6552,
            ClassKind::Modpack => 4471,
        }
    }

    pub fn from_class_id(id: u32) -> Option<Self> {
        [
            ClassKind::Mod,
            ClassKind::Resourcepack,
            ClassKind::Shader,
            ClassKind::Modpack,
        ]
        .into_iter()
        .find(|k| k.class_id() == id)
    }
}

pub fn loader_type(kind: LoaderKind) -> u32 {
    match kind {
        LoaderKind::Forge => 1,
        LoaderKind::Fabric => 4,
        LoaderKind::Quilt => 5,
        LoaderKind::NeoForge => 6,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Logo {
    #[serde(default)]
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    #[serde(default)]
    pub website_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mod {
    pub id: u64,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub download_count: f64,
    #[serde(default)]
    pub logo: Option<Logo>,
    #[serde(default)]
    pub authors: Vec<Author>,
    #[serde(default)]
    pub categories: Vec<Category>,
    #[serde(default)]
    pub date_modified: String,
    #[serde(default)]
    pub class_id: Option<u32>,
    #[serde(default)]
    pub links: Option<Links>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHash {
    pub value: String,
    /// 1 = SHA-1, 2 = MD5.
    pub algo: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDependency {
    pub mod_id: u64,
    /// 3 = Pflicht, 5 = inkompatibel.
    pub relation_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: u64,
    pub mod_id: u64,
    pub display_name: String,
    pub file_name: String,
    /// 1 = Release, 2 = Beta, 3 = Alpha.
    pub release_type: u32,
    pub file_date: String,
    pub file_length: u64,
    /// Fehlt, wenn der Autor Downloads über Drittanbieter gesperrt hat.
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub hashes: Vec<FileHash>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<FileDependency>,
}

impl File {
    pub fn sha1(&self) -> Option<&str> {
        self.hashes
            .iter()
            .find(|h| h.algo == 1)
            .map(|h| h.value.as_str())
    }
}

/// Beste Datei: neueste Release, sonst neueste überhaupt.
pub fn best_file(files: &[File]) -> Option<&File> {
    let newest = |f: &&File| f.file_date.clone();
    files
        .iter()
        .filter(|f| f.release_type == 1)
        .max_by_key(newest)
        .or_else(|| files.iter().max_by_key(newest))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub index: u32,
    pub page_size: u32,
    pub total_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct Data<T> {
    data: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchPage {
    pub data: Vec<Mod>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Default)]
pub struct CfSearch {
    pub query: String,
    pub class: Option<ClassKind>,
    pub game_version: Option<String>,
    pub loader: Option<LoaderKind>,
    /// 2 = Beliebtheit, 3 = zuletzt aktualisiert, 6 = Downloads.
    pub sort_field: u32,
    pub index: u32,
    pub page_size: u32,
}

#[derive(Clone)]
pub struct CurseForge {
    http: reqwest::Client,
    base: String,
    /// `None`: über den Proxy des Pinecone-Servers, der den Schlüssel hält.
    key: Option<String>,
}

impl CurseForge {
    pub fn new(key: impl Into<String>) -> Result<Self> {
        Self::with_base(API, key)
    }

    /// Über den Pinecone-Server (`<server>/api/v1/curseforge`), ohne eigenen Schlüssel.
    pub fn via_server(server: &str) -> Result<Self> {
        let mut cf = Self::with_base(
            format!("{}/api/v1/curseforge", server.trim_end_matches('/')),
            "",
        )?;
        cf.key = None;
        Ok(cf)
    }

    /// Spricht CurseForge direkt an (eigener Schlüssel) statt über den Pinecone-Server.
    pub fn direct(&self) -> bool {
        self.key.is_some()
    }

    pub fn with_base(base: impl Into<String>, key: impl Into<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(crate::user_agent())
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|source| Error::Http {
                url: String::new(),
                source,
            })?;
        Ok(Self {
            http,
            base: base.into(),
            key: Some(key.into()),
        })
    }

    async fn send<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
        url: &str,
    ) -> Result<T> {
        let req = match &self.key {
            Some(key) => req.header("x-api-key", key),
            None => req.header("x-pinecone-client", format!("pinecone/{LAUNCHER_VERSION}")),
        };
        let resp = req
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|source| Error::Http {
                url: url.into(),
                source,
            })?;
        if resp.status() == reqwest::StatusCode::FORBIDDEN && self.key.is_some() {
            return Err(Error::Invalid(
                "CurseForge lehnt den API-Key ab (403)".into(),
            ));
        }
        if self.key.is_none() {
            match resp.status() {
                reqwest::StatusCode::SERVICE_UNAVAILABLE | reqwest::StatusCode::BAD_GATEWAY => {
                    return Err(Error::Invalid(
                        "CurseForge ist gerade nicht verfügbar (Pinecone-Server)".into(),
                    ));
                }
                reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    return Err(Error::Invalid(
                        "Zu viele CurseForge-Anfragen, bitte kurz warten".into(),
                    ));
                }
                _ => {}
            }
        }
        resp.error_for_status()
            .map_err(|source| Error::Http {
                url: url.into(),
                source,
            })?
            .json()
            .await
            .map_err(|source| Error::Http {
                url: url.into(),
                source,
            })
    }

    pub async fn search(&self, q: &CfSearch) -> Result<SearchPage> {
        let url = format!("{}/v1/mods/search", self.base);
        let mut query: Vec<(&str, String)> = vec![
            ("gameId", MINECRAFT.to_string()),
            ("searchFilter", q.query.clone()),
            ("sortField", q.sort_field.max(1).to_string()),
            ("sortOrder", "desc".into()),
            ("index", q.index.to_string()),
            // CurseForge erlaubt höchstens Index + Seitengröße ≤ 10 000.
            ("pageSize", q.page_size.clamp(1, 50).to_string()),
        ];
        if let Some(c) = q.class {
            query.push(("classId", c.class_id().to_string()));
        }
        if let Some(v) = &q.game_version {
            query.push(("gameVersion", v.clone()));
        }
        if let Some(l) = q.loader {
            query.push(("modLoaderType", loader_type(l).to_string()));
        }
        self.send(self.http.get(&url).query(&query), &url).await
    }

    /// Dateien eines Projekts, optional gefiltert.
    pub async fn files(
        &self,
        mod_id: u64,
        game_version: Option<&str>,
        loader: Option<LoaderKind>,
    ) -> Result<Vec<File>> {
        let url = format!("{}/v1/mods/{mod_id}/files", self.base);
        let mut query = vec![("pageSize", "50".to_string())];
        if let Some(v) = game_version {
            query.push(("gameVersion", v.to_string()));
        }
        if let Some(l) = loader {
            query.push(("modLoaderType", loader_type(l).to_string()));
        }
        let data: Data<Vec<File>> = self.send(self.http.get(&url).query(&query), &url).await?;
        Ok(data.data)
    }

    pub async fn files_by_ids(&self, ids: &[u64]) -> Result<Vec<File>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!("{}/v1/mods/files", self.base);
        let data: Data<Vec<File>> = self
            .send(self.http.post(&url).json(&json!({ "fileIds": ids })), &url)
            .await?;
        Ok(data.data)
    }

    pub async fn mods_by_ids(&self, ids: &[u64]) -> Result<Vec<Mod>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!("{}/v1/mods", self.base);
        let data: Data<Vec<Mod>> = self
            .send(self.http.post(&url).json(&json!({ "modIds": ids })), &url)
            .await?;
        Ok(data.data)
    }
}

/// Seite, auf der man eine gesperrte Datei von Hand herunterladen kann.
pub fn manual_download_url(m: &Mod, file_id: u64) -> String {
    match m.links.as_ref().and_then(|l| l.website_url.clone()) {
        Some(site) => format!("{}/files/{file_id}", site.trim_end_matches('/')),
        None => format!(
            "https://www.curseforge.com/minecraft/mc-mods/{}/files/{file_id}",
            m.slug
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(id: u64, release: u32, date: &str) -> File {
        File {
            id,
            mod_id: 1,
            display_name: id.to_string(),
            file_name: format!("{id}.jar"),
            release_type: release,
            file_date: date.into(),
            file_length: 1,
            download_url: None,
            hashes: vec![
                FileHash {
                    value: "abc".into(),
                    algo: 1,
                },
                FileHash {
                    value: "md5".into(),
                    algo: 2,
                },
            ],
            game_versions: vec![],
            dependencies: vec![],
        }
    }

    #[test]
    fn picks_newest_release() {
        let files = [
            file(1, 1, "2026-01-01"),
            file(2, 3, "2026-05-01"),
            file(3, 1, "2026-03-01"),
        ];
        assert_eq!(best_file(&files).unwrap().id, 3);
        assert_eq!(best_file(&[file(4, 2, "2026-01-01")]).unwrap().id, 4);
        assert_eq!(files[0].sha1(), Some("abc"));
    }

    #[test]
    fn class_ids() {
        assert_eq!(ClassKind::from_class_id(6552), Some(ClassKind::Shader));
        assert_eq!(ClassKind::Modpack.class_id(), 4471);
        assert_eq!(ClassKind::from_class_id(1), None);
    }
}
