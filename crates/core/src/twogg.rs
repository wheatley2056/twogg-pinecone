//! Client für den TwoGG-Pinecone-Server (`crates/server`): Neuigkeiten und TwoGG-Modpacks.
//! Modpack-Dateien gibt es nur über kurzlebige, signierte Links, die der Server Pinecone ausstellt.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{Error, LAUNCHER_VERSION, Result};

pub const DEFAULT_SERVER: &str = "https://pinecone.twogg.app";

/// Server-Adresse: `PINECONE_SERVER_URL` (Laufzeit oder beim Bauen), sonst die TwoGG-Instanz.
pub fn server_url() -> String {
    std::env::var("PINECONE_SERVER_URL")
        .ok()
        .or_else(|| option_env!("PINECONE_SERVER_URL").map(str::to_string))
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SERVER.to_string())
        .trim_end_matches('/')
        .to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    /// Vom Server aus Markdown erzeugt (ohne rohes HTML, nur http(s)-Links).
    pub body_html: String,
    pub image_url: Option<String>,
    pub link: Option<String>,
    pub pinned: bool,
    pub published_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackVersion {
    pub id: String,
    pub version: String,
    pub minecraft: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub channel: String,
    pub changelog_html: String,
    pub size: u64,
    pub sha1: String,
    pub published_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pack {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub icon_url: Option<String>,
    pub featured: bool,
    pub updated_at: i64,
    pub downloads: i64,
    pub latest: Option<PackVersion>,
    #[serde(default)]
    pub description_html: Option<String>,
    /// Nur in der Detailansicht; neueste zuerst.
    #[serde(default)]
    pub versions: Option<Vec<PackVersion>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadLink {
    pub url: String,
    pub size: u64,
    pub sha1: String,
    pub file_name: String,
}

#[derive(Clone)]
pub struct TwoggClient {
    http: reqwest::Client,
    base: String,
}

impl TwoggClient {
    pub fn new() -> Result<Self> {
        Self::with_base(server_url())
    }

    pub fn with_base(base: impl Into<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(format!("pinecone/{LAUNCHER_VERSION}"))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|source| Error::Http {
                url: String::new(),
                source,
            })?;
        Ok(Self {
            http,
            base: base.into().trim_end_matches('/').to_string(),
        })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    async fn send<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
        url: &str,
    ) -> Result<T> {
        let resp = req
            .header("X-Pinecone-Client", format!("pinecone/{LAUNCHER_VERSION}"))
            .send()
            .await
            .map_err(|source| Error::Http {
                url: url.to_string(),
                source,
            })?;
        let status = resp.status();
        if !status.is_success() {
            // Der Server erklärt Fehler im Feld `error`.
            let text = resp.text().await.unwrap_or_default();
            let message = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v["error"].as_str().map(str::to_string))
                .unwrap_or_else(|| format!("HTTP {status}"));
            return Err(Error::Invalid(format!("TwoGG-Server: {message}")));
        }
        resp.json().await.map_err(|source| Error::Http {
            url: url.to_string(),
            source,
        })
    }

    pub async fn news(&self, limit: u32) -> Result<Vec<NewsItem>> {
        let url = format!("{}/api/v1/news?limit={limit}", self.base);
        self.send(self.http.get(&url), &url).await
    }

    pub async fn packs(&self) -> Result<Vec<Pack>> {
        let url = format!("{}/api/v1/packs", self.base);
        self.send(self.http.get(&url), &url).await
    }

    pub async fn pack(&self, id: &str) -> Result<Pack> {
        if !valid_id(id) {
            return Err(Error::Invalid(format!("Ungültige Pack-ID {id:?}")));
        }
        let url = format!("{}/api/v1/packs/{id}", self.base);
        self.send(self.http.get(&url), &url).await
    }

    pub async fn download_link(&self, pack: &str, version: &str) -> Result<DownloadLink> {
        if !valid_id(pack) || !valid_id(version) {
            return Err(Error::Invalid("Ungültige Pack- oder Versions-ID".into()));
        }
        let url = format!(
            "{}/api/v1/packs/{pack}/versions/{version}/download",
            self.base
        );
        self.send(self.http.post(&url), &url).await
    }
}

/// IDs aus der Server-Antwort landen in URLs: nur unbedenkliche Zeichen.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Neueste Version auf dem gleichen Kanal oder stabiler (Liste: neueste zuerst).
pub fn best_version<'a>(
    versions: &'a [PackVersion],
    minecraft: Option<&str>,
) -> Option<&'a PackVersion> {
    let fits = |v: &&PackVersion| minecraft.is_none_or(|mc| v.minecraft == mc);
    versions
        .iter()
        .filter(fits)
        .find(|v| v.channel == "release")
        .or_else(|| versions.iter().find(fits))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(id: &str, mc: &str, channel: &str) -> PackVersion {
        PackVersion {
            id: id.into(),
            version: id.into(),
            minecraft: mc.into(),
            loader: None,
            loader_version: None,
            channel: channel.into(),
            changelog_html: String::new(),
            size: 0,
            sha1: String::new(),
            published_at: 0,
        }
    }

    #[test]
    fn picks_release_for_minecraft_version() {
        let list = [
            v("b3", "1.21.1", "beta"),
            v("r2", "1.21.1", "release"),
            v("r1", "1.20.1", "release"),
        ];
        assert_eq!(best_version(&list, None).unwrap().id, "r2");
        assert_eq!(best_version(&list, Some("1.20.1")).unwrap().id, "r1");
        assert_eq!(best_version(&list[..1], None).unwrap().id, "b3");
        assert!(best_version(&list, Some("26.1")).is_none());
    }

    #[test]
    fn ids_are_checked() {
        assert!(valid_id("horizons-unbound"));
        assert!(valid_id("af1508aa3b19432f910d4776bad3cc74"));
        assert!(!valid_id("../admin"));
        assert!(!valid_id("a/b"));
    }
}
