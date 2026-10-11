//! Spielzeit und zuletzt gespielt je Profil, gespeichert in `<data>/stats.json`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::servers::ServerAddress;
use crate::{Error, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileStats {
    pub playtime_secs: u64,
    pub sessions: u32,
    /// Unix-Zeit (Sekunden) des letzten Spielendes.
    pub last_played: Option<u64>,
    pub last_server: Option<ServerAddress>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    #[serde(default)]
    pub profiles: HashMap<String, ProfileStats>,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub struct StatsStore {
    path: PathBuf,
    /// Spiele können gleichzeitig enden; Lesen-Ändern-Schreiben nacheinander.
    lock: Mutex<()>,
}

impl StatsStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn load(&self) -> Result<Stats> {
        match std::fs::read(&self.path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                tracing::warn!("stats.json unlesbar, beginne neu: {e}");
                Stats::default()
            })),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Stats::default()),
            Err(e) => Err(Error::io(&self.path)(e)),
        }
    }

    /// Verbucht eine beendete Spielsitzung.
    pub fn record_session(
        &self,
        profile_id: &str,
        started: u64,
        ended: u64,
        server: Option<ServerAddress>,
    ) -> Result<ProfileStats> {
        let _guard = self.lock.lock().unwrap();
        let mut stats = self.load()?;
        let entry = stats.profiles.entry(profile_id.to_string()).or_default();
        entry.playtime_secs += ended.saturating_sub(started);
        entry.sessions += 1;
        entry.last_played = Some(ended);
        if server.is_some() {
            entry.last_server = server;
        }
        let result = entry.clone();
        let json = serde_json::to_vec_pretty(&stats).map_err(Error::json("stats.json"))?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(Error::io(&tmp))?;
        std::fs::rename(&tmp, &self.path).map_err(Error::io(&self.path))?;
        Ok(result)
    }

    /// Statistik eines gelöschten Profils entfernen.
    pub fn forget(&self, profile_id: &str) -> Result<()> {
        let _guard = self.lock.lock().unwrap();
        let mut stats = self.load()?;
        if stats.profiles.remove(profile_id).is_some() {
            let json = serde_json::to_vec_pretty(&stats).map_err(Error::json("stats.json"))?;
            std::fs::write(&self.path, json).map_err(Error::io(&self.path))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let store = StatsStore::new(dir.path().join("stats.json"));
        assert!(store.load().unwrap().profiles.is_empty());

        store.record_session("fabric", 1000, 1600, None).unwrap();
        let server = ServerAddress {
            host: "mc.twogg.de".into(),
            port: None,
        };
        let s = store
            .record_session("fabric", 2000, 2300, Some(server.clone()))
            .unwrap();
        assert_eq!(
            s,
            ProfileStats {
                playtime_secs: 900,
                sessions: 2,
                last_played: Some(2300),
                last_server: Some(server.clone())
            }
        );

        // Sitzung ohne Server überschreibt den letzten Server nicht.
        let s = store.record_session("fabric", 3000, 3010, None).unwrap();
        assert_eq!(s.last_server, Some(server));

        store.forget("fabric").unwrap();
        assert!(store.load().unwrap().profiles.is_empty());
    }

    #[test]
    fn broken_file_starts_fresh() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("stats.json"), "{kaputt").unwrap();
        let store = StatsStore::new(dir.path().join("stats.json"));
        assert!(store.load().unwrap().profiles.is_empty());
        assert_eq!(
            store
                .record_session("a", 0, 60, None)
                .unwrap()
                .playtime_secs,
            60
        );
    }
}
