//! Launcher-Einstellungen in `<data>/settings.json`: Darstellung, Dashboard, Sprache.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Farbwelt nach dem Pinecone-Design-System (`docs/design/design-system.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    /// Warmes Dunkel, Standard.
    #[default]
    Rinde,
    /// Tiefes Waldgrün.
    Kiefer,
    /// Hell.
    Winterland,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    De,
    En,
}

/// Welche Launcher-Versionen das Selbst-Update anbietet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    /// Nur stabile Versionen.
    #[default]
    Release,
    /// Auch Vorabversionen (auf GitHub als „Pre-release“ veröffentlicht).
    Beta,
}

/// Standard-Akzent „Harz“.
pub const DEFAULT_ACCENT: &str = "#e9a23b";

/// Das, was eine Theme-Datei enthält (exportier- und importierbar).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", from = "RawTheme")]
pub struct Theme {
    pub name: ThemeName,
    /// Akzentfarbe als `#rrggbb`.
    pub accent: String,
    /// Zapfenschuppen-Muster hinter Bannern.
    pub pattern: bool,
    /// Pixelify Sans für Titel, Tabs und Buttons.
    pub pixel_titles: bool,
    pub reduce_motion: bool,
}

impl Default for Theme {
    fn default() -> Self {
        RawTheme::default().into()
    }
}

/// Liest auch Dateien älterer Versionen (`mode` hell/dunkel/System, grüner Standard-Akzent).
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTheme {
    name: Option<ThemeName>,
    mode: Option<String>,
    accent: Option<String>,
    pattern: Option<bool>,
    pixel_titles: Option<bool>,
    reduce_motion: Option<bool>,
}

impl From<RawTheme> for Theme {
    fn from(raw: RawTheme) -> Self {
        let name = raw.name.unwrap_or(match raw.mode.as_deref() {
            Some("light") => ThemeName::Winterland,
            _ => ThemeName::Rinde,
        });
        let accent = match raw.accent {
            // Grün war der alte Standard; im neuen Design ist Grün nur Statusfarbe.
            Some(a) if !a.eq_ignore_ascii_case("#4caf50") => a,
            _ => DEFAULT_ACCENT.to_string(),
        };
        Self {
            name,
            accent,
            pattern: raw.pattern.unwrap_or(true),
            pixel_titles: raw.pixel_titles.unwrap_or(true),
            reduce_motion: raw.reduce_motion.unwrap_or(false),
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
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    pub visible: bool,
}

pub const WIDGETS: [&str; 6] = [
    "recent",
    "news",
    "packs",
    "servers",
    "screenshots",
    "playtime",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub theme: Theme,
    /// Fensterrahmen des Systems statt der eigenen Kopfleiste (hilft unter manchen Linux-Fenstermanagern).
    #[serde(default)]
    pub system_frame: bool,
    /// Startanimation (Splash-Screen) beim Öffnen des Launchers.
    #[serde(default = "default_splash")]
    pub splash: bool,
    #[serde(default)]
    pub update_channel: UpdateChannel,
    #[serde(default = "default_widgets")]
    pub widgets: Vec<Widget>,
    #[serde(default)]
    pub language: Language,
    /// JVM-Presets (Aikar's Flags usw.) und globaler Standard.
    #[serde(default)]
    pub jvm: crate::jvm::JvmSettings,
}

fn default_splash() -> bool {
    true
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
            system_frame: false,
            splash: true,
            update_channel: UpdateChannel::Release,
            widgets: default_widgets(),
            language: Language::default(),
            jvm: crate::jvm::JvmSettings::default(),
        }
    }
}

impl Settings {
    /// Unbekannte Widgets entfernen, fehlende hinten anhängen (z. B. nach Updates).
    fn normalize(&mut self) {
        self.jvm.normalize();
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
        settings.jvm.validate()?;
        let mut settings = settings.clone();
        settings.normalize();
        std::fs::create_dir_all(&self.root).map_err(Error::io(&self.root))?;
        let path = self.path();
        let json = serde_json::to_vec_pretty(&settings).map_err(Error::json("settings.json"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(Error::io(&tmp))?;
        std::fs::rename(&tmp, &path).map_err(Error::io(&path))?;
        Ok(settings)
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
        s.theme.name = ThemeName::Kiefer;
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
            [
                "servers",
                "recent",
                "news",
                "packs",
                "screenshots",
                "playtime"
            ]
        );
        assert!(!s.widgets[0].visible);

        let mut bad = s.clone();
        bad.theme.accent = "rot".into();
        assert!(store.save(&bad).is_err());
    }

    #[test]
    fn migrates_old_theme() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.json"),
            r##"{"theme":{"mode":"light","accent":"#4caf50","backgroundDim":60},"background":"background-1.png"}"##,
        )
        .unwrap();
        let s = SettingsStore::new(dir.path()).load().unwrap();
        assert_eq!(s.theme.name, ThemeName::Winterland);
        assert_eq!(s.theme.accent, DEFAULT_ACCENT);
        assert!(s.theme.pattern && s.theme.pixel_titles && !s.theme.reduce_motion);

        let custom: Theme =
            serde_json::from_str(r##"{"mode":"dark","accent":"#7fb8e0"}"##).unwrap();
        assert_eq!(custom.name, ThemeName::Rinde);
        assert_eq!(custom.accent, "#7fb8e0");
    }

    #[test]
    fn theme_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path().join("data"));
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
