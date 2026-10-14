//! Verzeichnisstruktur des Launchers. Alles liegt unter einem Datenverzeichnis pro OS.

use std::path::{Component, Path, PathBuf};

use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Standard-Datenverzeichnis: `~/.local/share/twogg-pinecone` (Linux),
    /// `%APPDATA%\TwoGG\Pinecone` (Windows), `~/Library/Application Support/TwoGG Pinecone` (macOS).
    pub fn default_root() -> Result<PathBuf> {
        let base = directories::BaseDirs::new()
            .ok_or_else(|| Error::Invalid("Kein Home-Verzeichnis gefunden".into()))?;
        let data = base.data_dir();
        Ok(if cfg!(windows) {
            data.join("TwoGG").join("Pinecone")
        } else if cfg!(target_os = "macos") {
            data.join("TwoGG Pinecone")
        } else {
            data.join("twogg-pinecone")
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Markierung für „Alle lokalen Daten löschen“: Beim nächsten Start wird das Datenverzeichnis
    /// entfernt, bevor Logs oder Caches wieder Dateien öffnen.
    pub fn wipe_marker(&self) -> PathBuf {
        self.root.join(".delete-on-start")
    }

    /// Löscht das Datenverzeichnis, falls die Markierung gesetzt ist. `true`, wenn gelöscht wurde.
    /// Ohne Markierung passiert nichts, so kann nie ein fremder Ordner gelöscht werden.
    pub fn finish_pending_wipe(&self) -> std::io::Result<bool> {
        if !self.wipe_marker().is_file() {
            return Ok(false);
        }
        std::fs::remove_dir_all(&self.root)?;
        Ok(true)
    }

    pub fn version_manifest(&self) -> PathBuf {
        self.root.join("versions").join("version_manifest_v2.json")
    }

    pub fn version_dir(&self, id: &str) -> PathBuf {
        self.root.join("versions").join(id)
    }

    pub fn version_json(&self, id: &str) -> PathBuf {
        self.version_dir(id).join(format!("{id}.json"))
    }

    pub fn client_jar(&self, id: &str) -> PathBuf {
        self.version_dir(id).join(format!("{id}.jar"))
    }

    pub fn natives_dir(&self, id: &str) -> PathBuf {
        self.version_dir(id).join("natives")
    }

    pub fn libraries(&self) -> PathBuf {
        self.root.join("libraries")
    }

    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn asset_index(&self, id: &str) -> PathBuf {
        self.assets().join("indexes").join(format!("{id}.json"))
    }

    pub fn asset_object(&self, hash: &str) -> PathBuf {
        self.assets().join("objects").join(&hash[..2]).join(hash)
    }

    pub fn accounts_file(&self) -> PathBuf {
        self.root.join("accounts.json")
    }

    pub fn stats_file(&self) -> PathBuf {
        self.root.join("stats.json")
    }

    pub fn runtimes(&self) -> PathBuf {
        self.root.join("runtimes")
    }

    pub fn runtime_dir(&self, component: &str) -> PathBuf {
        self.runtimes().join(component)
    }

    pub fn virtual_assets(&self, index: &str) -> PathBuf {
        self.assets().join("virtual").join(index)
    }

    pub fn instance_dir(&self, name: &str) -> PathBuf {
        self.root.join("instances").join(name)
    }
}

/// Prüft einen relativen Pfad aus fremden Daten (Manifeste, Asset-Indizes, Archive):
/// absolute Pfade, `..` und leere Namen werden abgelehnt (Schutz vor Path-Traversal).
pub fn safe_relative(name: &str) -> Result<PathBuf> {
    let path = Path::new(name);
    if !name.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_))) {
        Ok(path.to_path_buf())
    } else {
        Err(Error::Invalid(format!("Unsicherer Pfad: {name}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_paths() {
        assert!(safe_relative("bin/java").is_ok());
        assert!(safe_relative("minecraft/sounds/a.ogg").is_ok());
        assert!(safe_relative("../evil").is_err());
        assert!(safe_relative("/etc/passwd").is_err());
        assert!(safe_relative("lib/../../evil").is_err());
        assert!(safe_relative("").is_err());
    }

    #[test]
    fn wipe_only_with_marker() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("data");
        std::fs::create_dir_all(root.join("instances/a")).unwrap();
        let paths = Paths::new(&root);
        assert!(
            !paths.finish_pending_wipe().unwrap(),
            "ohne Markierung bleibt alles"
        );
        assert!(root.join("instances/a").is_dir());
        std::fs::write(paths.wipe_marker(), "").unwrap();
        assert!(paths.finish_pending_wipe().unwrap());
        assert!(!root.exists());
    }
}
