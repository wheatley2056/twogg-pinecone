//! Launcher-Einstellungen in `<data>/settings.json`: Darstellung, Dashboard, Sprache.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    De,
    En,
}

/// Das, was eine Theme-Datei enthält (exportier- und importierbar).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    #[serde(default)]
    pub mode: ThemeMode,
    /// Akzentfarbe als `#rrggbb`.
    #[serde(default = "default_accent")]
    pub accent: String,
    /// Abdunklung des Hintergrundbilds in Prozent (0–90).
    #[serde(default = "default_dim")]
    pub background_dim: u8,
}

fn default_accent() -> String {
    "#4caf50".into()
}

fn default_dim() -> u8 {
    60
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            mode: ThemeMode::System,
            accent: default_accent(),
            background_dim: default_dim(),
        }
    }
}

impl Theme {
    pub fn validate(&self) -> Result<()> {
        let hex = self.accent.strip_prefix('#').unwrap_or("");
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::Invalid(format!(
                "Ungültige Akzentfarbe {:?} (erwartet #rrggbb)",
                self.accent
            )));
        }
        if self.background_dim > 90 {
            return Err(Error::Invalid(
                "Abdunklung muss zwischen 0 und 90 % liegen".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    pub visible: bool,
}

pub const WIDGETS: [&str; 5] = ["recent", "packs", "servers", "screenshots", "playtime"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub theme: Theme,
    /// Dateiname des Hintergrundbilds im Datenverzeichnis.
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default = "default_widgets")]
    pub widgets: Vec<Widget>,
    #[serde(default)]
    pub language: Language,
}

fn default_widgets() -> Vec<Widget> {
    WIDGETS
        .iter()
        .map(|id| Widget {
            id: id.to_string(),
            visible: true,
        })
        .collect()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            background: None,
            widgets: default_widgets(),
            language: Language::default(),
        }
    }
}

impl Settings {
    /// Unbekannte Widgets entfernen, fehlende hinten anhängen (z. B. nach Updates).
    fn normalize(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.widgets
            .retain(|w| WIDGETS.contains(&w.id.as_str()) && seen.insert(w.id.clone()));
        for id in WIDGETS {
            if !self.widgets.iter().any(|w| w.id == id) {
                self.widgets.push(Widget {
                    id: id.into(),
                    visible: true,
                });
            }
        }
    }
}

pub struct SettingsStore {
    root: PathBuf,
}

const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

impl SettingsStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn load(&self) -> Result<Settings> {
        let path = self.path();
        let mut settings = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                tracing::warn!("settings.json unlesbar, nutze Standardwerte: {e}");
                Settings::default()
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(e) => return Err(Error::io(&path)(e)),
        };
        settings.normalize();
        Ok(settings)
    }

    pub fn save(&self, settings: &Settings) -> Result<Settings> {
        settings.theme.validate()?;
        let mut settings = settings.clone();
        settings.normalize();
        // Nur Hintergründe zulassen, die Pinecone selbst abgelegt hat.
        if let Some(bg) = &settings.background
            && (bg.contains(['/', '\\']) || !self.root.join(bg).is_file())
        {
            settings.background = None;
        }
        std::fs::create_dir_all(&self.root).map_err(Error::io(&self.root))?;
        let path = self.path();
        let json = serde_json::to_vec_pretty(&settings).map_err(Error::json("settings.json"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(Error::io(&tmp))?;
        std::fs::rename(&tmp, &path).map_err(Error::io(&path))?;
        Ok(settings)
    }

    /// Kopiert ein Bild als `background.<ext>` ins Datenverzeichnis und setzt es als Hintergrund.
    pub fn set_background(&self, source: &Path) -> Result<Settings> {
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .filter(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
            .ok_or_else(|| Error::Invalid("Hintergrundbild muss PNG, JPG oder WebP sein".into()))?;
        let meta = std::fs::metadata(source).map_err(Error::io(source))?;
        if meta.len() > 20 * 1024 * 1024 {
            return Err(Error::Invalid(
                "Hintergrundbild ist größer als 20 MB".into(),
            ));
        }
        let mut settings = self.load()?;
        self.remove_background_files()?;
        // Neuer Name je Bild, damit die Oberfläche kein altes Bild aus dem Cache zeigt.
        let name = format!("background-{}.{ext}", crate::stats::now_secs());
        std::fs::create_dir_all(&self.root).map_err(Error::io(&self.root))?;
        let target = self.root.join(&name);
        std::fs::copy(source, &target).map_err(Error::io(&target))?;
        settings.background = Some(name);
        self.save(&settings)
    }

    pub fn clear_background(&self) -> Result<Settings> {
        let mut settings = self.load()?;
        self.remove_background_files()?;
        settings.background = None;
        self.save(&settings)
    }

    fn remove_background_files(&self) -> Result<()> {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return Ok(());
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with("background") && IMAGE_EXTENSIONS.iter().any(|x| name.ends_with(x))
            {
                std::fs::remove_file(e.path()).map_err(Error::io(e.path()))?;
            }
        }
        Ok(())
    }

    pub fn export_theme(&self, target: &Path) -> Result<()> {
        let theme = self.load()?.theme;
        let json =
            serde_json::to_vec_pretty(&serde_json::json!({ "pineconeTheme": 1, "theme": theme }))
                .map_err(Error::json("Theme"))?;
        std::fs::write(target, json).map_err(Error::io(target))
    }

    pub fn import_theme(&self, source: &Path) -> Result<Settings> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ThemeFile {
            pinecone_theme: u32,
            theme: Theme,
        }
        let bytes = std::fs::read(source).map_err(Error::io(source))?;
        if bytes.len() > 64 * 1024 {
            return Err(Error::Invalid("Theme-Datei ist zu groß".into()));
        }
        let file: ThemeFile = serde_json::from_slice(&bytes)
            .map_err(|_| Error::Invalid("Keine gültige Pinecone-Theme-Datei".into()))?;
        if file.pinecone_theme != 1 {
            return Err(Error::Invalid(
                "Theme-Datei aus einer neueren Pinecone-Version".into(),
            ));
        }
        file.theme.validate()?;
        let mut settings = self.load()?;
        settings.theme = file.theme;
        self.save(&settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path());
        let s = store.load().unwrap();
        assert_eq!(s, Settings::default());
        assert_eq!(s.widgets.len(), WIDGETS.len());

        let mut s = s;
        s.theme.mode = ThemeMode::Dark;
        s.theme.accent = "#e91e63".into();
        s.language = Language::En;
        s.widgets.reverse();
        s.widgets[0].visible = false;
        store.save(&s).unwrap();
        assert_eq!(store.load().unwrap(), s);
    }

    #[test]
    fn normalizes_widgets_and_rejects_bad_theme() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{"widgets":[{"id":"servers","visible":false},{"id":"weg","visible":true},{"id":"servers","visible":true}]}"#,
        )
        .unwrap();
        let store = SettingsStore::new(dir.path());
        let s = store.load().unwrap();
        let ids: Vec<&str> = s.widgets.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(
            ids,
            ["servers", "recent", "packs", "screenshots", "playtime"]
        );
        assert!(!s.widgets[0].visible);

        let mut bad = s.clone();
        bad.theme.accent = "rot".into();
        assert!(store.save(&bad).is_err());
        bad.theme.accent = "#123456".into();
        bad.theme.background_dim = 99;
        assert!(store.save(&bad).is_err());
        // Fremde Pfade als Hintergrund werden verworfen.
        let mut s2 = s;
        s2.background = Some("../../etc/passwd".into());
        assert_eq!(store.save(&s2).unwrap().background, None);
    }

    #[test]
    fn background_and_theme_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path().join("data"));
        let img = dir.path().join("bild.PNG");
        std::fs::write(&img, b"\x89PNG").unwrap();
        let s = store.set_background(&img).unwrap();
        let name = s.background.clone().unwrap();
        assert!(name.starts_with("background-") && name.ends_with(".png"));
        assert!(dir.path().join("data").join(&name).is_file());
        assert!(store.set_background(&dir.path().join("x.exe")).is_err());
        assert_eq!(store.clear_background().unwrap().background, None);
        assert!(!dir.path().join("data").join(&name).exists());

        let mut s = store.load().unwrap();
        s.theme.accent = "#ff9800".into();
        store.save(&s).unwrap();
        let file = dir.path().join("mein.pinecone-theme.json");
        store.export_theme(&file).unwrap();
        s.theme.accent = "#000000".into();
        store.save(&s).unwrap();
        assert_eq!(store.import_theme(&file).unwrap().theme.accent, "#ff9800");

        std::fs::write(&file, r#"{"irgendwas": true}"#).unwrap();
        assert!(store.import_theme(&file).is_err());
    }
}
