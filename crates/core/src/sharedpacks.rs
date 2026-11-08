//! Instanzen als abonnierbares Pack mit Freunden teilen (`pinecone://pack/<id>`).
//!
//! Der Ersteller veröffentlicht seine Instanz als `.mrpack` (wie beim Datei-Export) auf dem
//! Pinecone-Server. Wer den Link öffnet, installiert das Pack und bekommt neue Versionen als
//! „Update verfügbar“ über den normalen Update-Ablauf (Snapshot, eigene Einstellungen bleiben).
//! Welche eigene Instanz zu welchem geteilten Pack gehört, steht in `<data-dir>/shared-packs.json`.
//! Die Zuordnung liegt bewusst nicht im Profil, damit eine duplizierte Instanz nicht in
//! dasselbe Pack veröffentlicht.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::modpack::{ExportOptions, PackManager};
use crate::profiles::Profile;
use crate::twogg::SharedPack;
use crate::{Error, Result};

/// Eigene Instanz, die als geteiltes Pack veröffentlicht ist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Published {
    pub pack_id: String,
    pub name: String,
    /// Zuletzt veröffentlichte Versionsnummer.
    pub version: String,
    pub link: String,
    pub published_at: i64,
}

/// Was beim Veröffentlichen mitkommt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishRequest {
    /// Name des Packs, wie ihn Freunde sehen.
    pub name: String,
    #[serde(default)]
    pub changelog: String,
    /// Versionsnummer und Inhalte wie beim `.mrpack`-Export.
    pub options: ExportOptions,
}

pub struct PublishStore {
    path: PathBuf,
}

impl PublishStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join("shared-packs.json"),
        }
    }

    pub fn load(&self) -> BTreeMap<String, Published> {
        std::fs::read(&self.path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn get(&self, profile_id: &str) -> Option<Published> {
        self.load().remove(profile_id)
    }

    fn save(&self, all: &BTreeMap<String, Published>) -> Result<()> {
        let json = serde_json::to_vec_pretty(all).map_err(Error::json("shared-packs.json"))?;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(Error::io(dir))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(Error::io(&tmp))?;
        std::fs::rename(&tmp, &self.path).map_err(Error::io(&self.path))
    }

    pub fn set(&self, profile_id: &str, published: Published) -> Result<()> {
        let mut all = self.load();
        all.insert(profile_id.to_string(), published);
        self.save(&all)
    }

    pub fn remove(&self, profile_id: &str) -> Result<()> {
        let mut all = self.load();
        if all.remove(profile_id).is_some() {
            self.save(&all)?;
        }
        Ok(())
    }
}

/// Veröffentlicht die Instanz: beim ersten Mal als neues Pack, danach als neue Version desselben
/// Packs. Wurde das Pack auf dem Server gelöscht oder ist es abgelaufen, entsteht ein neues
/// (mit neuem Link).
pub async fn publish(
    packs: &PackManager,
    store: &PublishStore,
    cache_dir: &Path,
    token: &str,
    profile: &Profile,
    req: &PublishRequest,
) -> Result<SharedPack> {
    let client = packs
        .twogg()
        .ok_or_else(|| Error::Invalid("TwoGG-Server ist nicht eingerichtet".into()))?;
    let name = req.name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("Bitte gib dem Pack einen Namen".into()));
    }
    std::fs::create_dir_all(cache_dir).map_err(Error::io(cache_dir))?;
    let file = cache_dir.join(format!("publish-{}.mrpack", profile.id));
    let exported = packs.export(profile, &file, &req.options).await;
    let bytes = exported.and_then(|_| std::fs::read(&file).map_err(Error::io(&file)));
    let _ = std::fs::remove_file(&file);
    let bytes = bytes?;

    let existing = match store.get(&profile.id) {
        Some(p) => client
            .shared_pack(token, &p.pack_id)
            .await?
            .filter(|s| s.mine),
        None => None,
    };
    let pack = match existing {
        Some(s) => {
            let rename = (s.name != name).then_some(name);
            client
                .shared_pack_publish(token, &s.id, rename, &req.changelog, bytes)
                .await?
        }
        None => {
            client
                .shared_pack_create(token, name, &req.changelog, bytes)
                .await?
        }
    };
    store.set(
        &profile.id,
        Published {
            pack_id: pack.id.clone(),
            name: pack.name.clone(),
            version: req.options.version.trim().to_string(),
            link: pack.link.clone(),
            published_at: crate::stats::now_secs() as i64,
        },
    )?;
    Ok(pack)
}

/// Löscht das geteilte Pack auf dem Server. Freunde behalten ihre Instanzen, bekommen aber
/// keine Updates mehr.
pub async fn unpublish(
    packs: &PackManager,
    store: &PublishStore,
    token: &str,
    profile_id: &str,
) -> Result<()> {
    let Some(published) = store.get(profile_id) else {
        return Ok(());
    };
    let client = packs
        .twogg()
        .ok_or_else(|| Error::Invalid("TwoGG-Server ist nicht eingerichtet".into()))?;
    client.shared_pack_delete(token, &published.pack_id).await?;
    store.remove(profile_id)
}

/// Vorschlag für die nächste Versionsnummer: letzte Zahl um eins erhöht (`1.2` → `1.3`,
/// `v2` → `v3`), sonst `.1` angehängt; ohne Vorgänger `1.0`.
pub fn next_version(last: Option<&str>) -> String {
    let Some(last) = last.map(str::trim).filter(|l| !l.is_empty()) else {
        return "1.0".into();
    };
    let end = last.len() - last.bytes().rev().take_while(u8::is_ascii_digit).count();
    let (head, digits) = last.split_at(end);
    match digits.parse::<u64>() {
        Ok(n) => format!("{head}{}", n + 1),
        Err(_) => format!("{last}.1"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_next_version() {
        assert_eq!(next_version(None), "1.0");
        assert_eq!(next_version(Some("  ")), "1.0");
        assert_eq!(next_version(Some("1.0")), "1.1");
        assert_eq!(next_version(Some("1.9")), "1.10");
        assert_eq!(next_version(Some("v2")), "v3");
        assert_eq!(next_version(Some("2.0-beta")), "2.0-beta.1");
    }

    #[test]
    fn store_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = PublishStore::new(dir.path());
        assert_eq!(store.get("a"), None);
        let p = Published {
            pack_id: "AB23CD45EF".into(),
            name: "Pack".into(),
            version: "1.0".into(),
            link: "pinecone://pack/AB23CD45EF".into(),
            published_at: 1,
        };
        store.set("a", p.clone()).unwrap();
        assert_eq!(store.get("a"), Some(p));
        store.remove("a").unwrap();
        store.remove("a").unwrap();
        assert_eq!(store.get("a"), None);
    }
}
