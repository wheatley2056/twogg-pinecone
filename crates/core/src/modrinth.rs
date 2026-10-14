//! Modrinth-API v2: Suche, Projekte, Versionen und Zuordnung von Dateien per SHA-1.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Error, Result};

pub const API: &str = "https://api.modrinth.com/v2";

/// Download-Hosts, die Modrinth für Modpacks erlaubt (Spezifikation `.mrpack`).
pub const ALLOWED_HOSTS: [&str; 4] = [
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "gitlab.com",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Mod,
    Modpack,
    Resourcepack,
    Shader,
    /// Datenpakete gehören in eine Welt (`saves/<welt>/datapacks`), nicht in die Instanz.
    Datapack,
}

impl ProjectType {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectType::Mod => "mod",
            ProjectType::Modpack => "modpack",
            ProjectType::Resourcepack => "resourcepack",
            ProjectType::Shader => "shader",
            ProjectType::Datapack => "datapack",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortIndex {
    #[default]
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub query: String,
    pub project_type: Option<ProjectType>,
    /// Alternativen (ODER), z. B. `["quilt", "fabric"]`.
    pub loaders: Vec<String>,
    pub game_version: Option<String>,
    /// Alle müssen passen (UND).
    pub categories: Vec<String>,
    pub sort: SortIndex,
    pub offset: u32,
    pub limit: u32,
}

impl SearchQuery {
    /// Facetten im Modrinth-Format: äußere Liste UND, innere Listen ODER.
    pub fn facets(&self) -> String {
        let mut facets: Vec<Vec<String>> = Vec::new();
        if let Some(t) = self.project_type {
            facets.push(vec![format!("project_type:{}", t.as_str())]);
        }
        if !self.loaders.is_empty() {
            facets.push(
                self.loaders
                    .iter()
                    .map(|l| format!("categories:{l}"))
                    .collect(),
            );
        }
        if let Some(v) = &self.game_version {
            facets.push(vec![format!("versions:{v}")]);
        }
        for c in &self.categories {
            facets.push(vec![format!("categories:{c}")]);
        }
        serde_json::to_string(&facets).unwrap_or_else(|_| "[]".into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub project_type: String,
    pub title: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub downloads: u64,
    #[serde(default)]
    pub follows: u64,
    #[serde(default)]
    pub display_categories: Vec<String>,
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default)]
    pub date_modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub project_type: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub downloads: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHashes {
    pub sha1: String,
    #[serde(default)]
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    pub hashes: FileHashes,
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionType {
    Alpha,
    Beta,
    Release,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub version_type: VersionType,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    pub date_published: String,
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

impl Version {
    pub fn primary_file(&self) -> Option<&VersionFile> {
        self.files.iter().find(|f| f.primary).or(self.files.first())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct Category {
    pub name: String,
    pub project_type: String,
    /// Gruppe, z. B. `categories`, `features`, `resolutions`, `performance impact`.
    pub header: String,
}

/// Beste Version: die neueste stabile, sonst die neueste überhaupt (Liste kommt neueste zuerst).
pub fn best_version(versions: &[Version]) -> Option<&Version> {
    versions
        .iter()
        .find(|v| v.version_type == VersionType::Release)
        .or_else(|| versions.first())
}

#[derive(Clone)]
pub struct Modrinth {
    http: reqwest::Client,
    base: String,
}

impl Modrinth {
    pub fn new() -> Result<Self> {
        Self::with_base(API)
    }

    pub fn with_base(base: impl Into<String>) -> Result<Self> {
        // Modrinth verlangt einen aussagekräftigen User-Agent mit Kontakt.
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
        })
    }

    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let url = format!("{}{path}", self.base);
        let http = |source| Error::Http {
            url: url.clone(),
            source,
        };
        self.http
            .get(&url)
            .query(query)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(http)?
            .json()
            .await
            .map_err(http)
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<T> {
        let url = format!("{}{path}", self.base);
        let http = |source| Error::Http {
            url: url.clone(),
            source,
        };
        self.http
            .post(&url)
            .json(&body)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(http)?
            .json()
            .await
            .map_err(http)
    }

    pub async fn search(&self, q: &SearchQuery) -> Result<SearchResult> {
        let index = match q.sort {
            SortIndex::Relevance => "relevance",
            SortIndex::Downloads => "downloads",
            SortIndex::Follows => "follows",
            SortIndex::Newest => "newest",
            SortIndex::Updated => "updated",
        };
        self.get(
            "/search",
            &[
                ("query", q.query.clone()),
                ("facets", q.facets()),
                ("index", index.into()),
                ("offset", q.offset.to_string()),
                ("limit", q.limit.clamp(1, 100).to_string()),
            ],
        )
        .await
    }

    pub async fn project(&self, id_or_slug: &str) -> Result<Project> {
        self.get(&format!("/project/{}", encode_segment(id_or_slug)), &[])
            .await
    }

    pub async fn projects(&self, ids: &[String]) -> Result<Vec<Project>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = serde_json::to_string(ids).unwrap_or_default();
        self.get("/projects", &[("ids", ids)]).await
    }

    /// Versionen eines Projekts, neueste zuerst; leere Filter = alle.
    pub async fn versions(
        &self,
        project: &str,
        loaders: &[String],
        game_versions: &[String],
    ) -> Result<Vec<Version>> {
        let mut query = Vec::new();
        if !loaders.is_empty() {
            query.push((
                "loaders",
                serde_json::to_string(loaders).unwrap_or_default(),
            ));
        }
        if !game_versions.is_empty() {
            query.push((
                "game_versions",
                serde_json::to_string(game_versions).unwrap_or_default(),
            ));
        }
        self.get(
            &format!("/project/{}/version", encode_segment(project)),
            &query,
        )
        .await
    }

    /// Kategorien aller Projektarten (Icons werden bewusst nicht übernommen: es sind rohe SVG-Texte).
    pub async fn categories(&self) -> Result<Vec<Category>> {
        self.get("/tag/category", &[]).await
    }

    /// Mehrere Versionen auf einmal (Reihenfolge nicht garantiert).
    pub async fn versions_by_ids(&self, ids: &[String]) -> Result<Vec<Version>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = serde_json::to_string(ids).unwrap_or_default();
        self.get("/versions", &[("ids", ids)]).await
    }

    pub async fn version(&self, id: &str) -> Result<Version> {
        self.get(&format!("/version/{}", encode_segment(id)), &[])
            .await
    }

    /// Ordnet Dateien per SHA-1 ihrer Modrinth-Version zu; unbekannte fehlen in der Antwort.
    pub async fn versions_by_hash(&self, sha1: &[String]) -> Result<HashMap<String, Version>> {
        if sha1.is_empty() {
            return Ok(HashMap::new());
        }
        self.post(
            "/version_files",
            json!({ "hashes": sha1, "algorithm": "sha1" }),
        )
        .await
    }

    /// Neueste passende Version je Datei-Hash (für Updates).
    pub async fn latest_by_hash(
        &self,
        sha1: &[String],
        loaders: &[String],
        game_versions: &[String],
    ) -> Result<HashMap<String, Version>> {
        if sha1.is_empty() {
            return Ok(HashMap::new());
        }
        let mut body = json!({ "hashes": sha1, "algorithm": "sha1" });
        if !loaders.is_empty() {
            body["loaders"] = json!(loaders);
        }
        if !game_versions.is_empty() {
            body["game_versions"] = json!(game_versions);
        }
        self.post("/version_files/update", body).await
    }
}

/// IDs und Slugs bestehen aus [A-Za-z0-9_-]; alles andere wird kodiert statt in den Pfad übernommen.
fn encode_segment(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facets_format() {
        let q = SearchQuery {
            project_type: Some(ProjectType::Mod),
            loaders: vec!["quilt".into(), "fabric".into()],
            game_version: Some("1.21.1".into()),
            categories: vec!["optimization".into()],
            ..Default::default()
        };
        assert_eq!(
            q.facets(),
            r#"[["project_type:mod"],["categories:quilt","categories:fabric"],["versions:1.21.1"],["categories:optimization"]]"#
        );
        assert_eq!(SearchQuery::default().facets(), "[]");
    }

    #[test]
    fn picks_release_over_newer_beta() {
        let v = |id: &str, t: VersionType| Version {
            id: id.into(),
            project_id: "p".into(),
            name: id.into(),
            version_number: id.into(),
            version_type: t,
            loaders: vec![],
            game_versions: vec![],
            date_published: String::new(),
            files: vec![],
            dependencies: vec![],
        };
        let list = [
            v("3-beta", VersionType::Beta),
            v("2", VersionType::Release),
            v("1", VersionType::Release),
        ];
        assert_eq!(best_version(&list).unwrap().id, "2");
        let only_alpha = [v("a", VersionType::Alpha)];
        assert_eq!(best_version(&only_alpha).unwrap().id, "a");
        assert!(best_version(&[]).is_none());
    }

    #[test]
    fn encodes_path_segments() {
        assert_eq!(encode_segment("fabric-api"), "fabric-api");
        assert_eq!(encode_segment("../x?y"), "%2E%2E%2Fx%3Fy");
    }
}
