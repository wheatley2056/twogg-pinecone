//! JVM-Voreinstellungen („Presets“): bewährte Flag-Sätze wie Aikar's Flags, eigene Sätze und
//! Anpassungen. Die Einstellungen speichern nur Abweichungen von den eingebauten Sätzen und eigene
//! Presets; ein Profil wählt ein Preset oder folgt dem globalen Standard.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Kein Preset: nur Arbeitsspeicher und eigene Argumente.
pub const NONE: &str = "none";

/// Ein Flag-Satz, wie ihn Einstellungen und Instanz-Dialog anzeigen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JvmPreset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub args: Vec<String>,
    /// Mindestens nötige Java-Hauptversion; darunter startet das Spiel ohne das Preset.
    #[serde(default = "default_min_java")]
    pub min_java: u32,
    /// Eingebaut (zurücksetzbar) oder selbst angelegt (löschbar).
    #[serde(default)]
    pub builtin: bool,
    /// Eingebautes Preset mit geänderten Argumenten.
    #[serde(default)]
    pub modified: bool,
}

fn default_min_java() -> u32 {
    8
}

/// Gespeichert in den Einstellungen: eigene Presets und geänderte eingebaute (gleiche ID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredPreset {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub args: Vec<String>,
    #[serde(default = "default_min_java")]
    pub min_java: u32,
}

/// JVM-Einstellungen: globaler Standard und gespeicherte Presets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JvmSettings {
    /// Preset für Instanzen ohne eigene Wahl.
    #[serde(default = "default_preset")]
    pub default_preset: String,
    #[serde(default)]
    pub presets: Vec<StoredPreset>,
}

fn default_preset() -> String {
    NONE.into()
}

impl Default for JvmSettings {
    fn default() -> Self {
        Self {
            default_preset: default_preset(),
            presets: Vec::new(),
        }
    }
}

const AIKAR: &[&str] = &[
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:MaxGCPauseMillis=200",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+DisableExplicitGC",
    "-XX:+AlwaysPreTouch",
    "-XX:G1NewSizePercent=30",
    "-XX:G1MaxNewSizePercent=40",
    "-XX:G1HeapRegionSize=8M",
    "-XX:G1ReservePercent=20",
    "-XX:G1HeapWastePercent=5",
    "-XX:G1MixedGCCountTarget=4",
    "-XX:InitiatingHeapOccupancyPercent=15",
    "-XX:G1MixedGCLiveThresholdPercent=90",
    "-XX:G1RSetUpdatingPauseTimePercent=5",
    "-XX:SurvivorRatio=32",
    "-XX:+PerfDisableSharedMem",
    "-XX:MaxTenuringThreshold=1",
];

/// MeowIce's Flags (G1-Variante). Einige Optionen gibt es nicht in jeder Java-Version, daher
/// `IgnoreUnrecognizedVMOptions` vorneweg.
const MEOWICE: &[&str] = &[
    "-XX:+IgnoreUnrecognizedVMOptions",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+UnlockDiagnosticVMOptions",
    "-XX:+AlwaysActAsServerClassMachine",
    "-XX:+AlwaysPreTouch",
    "-XX:+DisableExplicitGC",
    "-XX:+UseNUMA",
    "-XX:NmethodSweepActivity=1",
    "-XX:ReservedCodeCacheSize=400M",
    "-XX:NonNMethodCodeHeapSize=12M",
    "-XX:ProfiledCodeHeapSize=194M",
    "-XX:NonProfiledCodeHeapSize=194M",
    "-XX:-DontCompileHugeMethods",
    "-XX:MaxNodeLimit=240000",
    "-XX:NodeLimitFudgeFactor=8000",
    "-XX:+UseVectorCmov",
    "-XX:+PerfDisableSharedMem",
    "-XX:+UseFastUnorderedTimeStamps",
    "-XX:+UseCriticalJavaThreadPriority",
    "-XX:ThreadPriorityPolicy=1",
    "-XX:AllocatePrefetchStyle=3",
    "-XX:+UseG1GC",
    "-XX:MaxGCPauseMillis=37",
    "-XX:G1HeapRegionSize=16M",
    "-XX:G1NewSizePercent=23",
    "-XX:G1ReservePercent=20",
    "-XX:SurvivorRatio=32",
    "-XX:G1MixedGCCountTarget=3",
    "-XX:G1HeapWastePercent=20",
    "-XX:InitiatingHeapOccupancyPercent=10",
    "-XX:G1RSetUpdatingPauseTimePercent=0",
    "-XX:MaxTenuringThreshold=1",
    "-XX:G1SATBBufferEnqueueingThresholdPercent=30",
    "-XX:G1ConcMarkStepDurationMillis=5.0",
    "-XX:G1ConcRSHotCardLimit=16",
    "-XX:G1ConcRefinementServiceIntervalMillis=150",
    "-XX:GCTimeRatio=99",
];

/// Generational ZGC: sehr kurze Pausen, braucht etwas mehr Arbeitsspeicher.
const ZGC: &[&str] = &[
    "-XX:+IgnoreUnrecognizedVMOptions",
    "-XX:+UseZGC",
    "-XX:+ZGenerational",
    "-XX:+AlwaysPreTouch",
    "-XX:+DisableExplicitGC",
    "-XX:+PerfDisableSharedMem",
];

/// Eingebaute Presets in Anzeige-Reihenfolge (ohne „Keine“).
pub fn builtin() -> Vec<JvmPreset> {
    let preset = |id: &str, name: &str, description: &str, args: &[&str], min_java| JvmPreset {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        args: args.iter().map(|a| a.to_string()).collect(),
        min_java,
        builtin: true,
        modified: false,
    };
    vec![
        preset(
            "aikar",
            "Aikar's Flags",
            "Bewährte G1-Einstellungen für gleichmäßige Bildraten und kurze Ruckler. Guter Standard für die meisten Instanzen.",
            AIKAR,
            8,
        ),
        preset(
            "meowice",
            "MeowIce's Flags",
            "Aggressiver abgestimmtes G1 mit JIT-Optimierungen. Für große Modpacks und starke Rechner, ab Java 17.",
            MEOWICE,
            17,
        ),
        preset(
            "zgc",
            "Generational ZGC",
            "Kaum spürbare GC-Pausen, braucht etwas mehr Arbeitsspeicher. Ab Java 21 (Minecraft 1.20.5 und neuer).",
            ZGC,
            21,
        ),
    ]
}

impl JvmSettings {
    /// Alle Presets: eingebaute (ggf. angepasst), dann eigene.
    pub fn presets(&self) -> Vec<JvmPreset> {
        let mut out: Vec<JvmPreset> = builtin()
            .into_iter()
            .map(|mut p| {
                if let Some(s) = self.presets.iter().find(|s| s.id == p.id) {
                    p.modified = s.args != p.args;
                    p.args = s.args.clone();
                }
                p
            })
            .collect();
        let builtin_ids: Vec<String> = out.iter().map(|p| p.id.clone()).collect();
        out.extend(
            self.presets
                .iter()
                .filter(|s| !builtin_ids.contains(&s.id))
                .map(|s| JvmPreset {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    description: String::new(),
                    args: s.args.clone(),
                    min_java: s.min_java,
                    builtin: false,
                    modified: false,
                }),
        );
        out
    }

    pub fn find(&self, id: &str) -> Option<JvmPreset> {
        self.presets().into_iter().find(|p| p.id == id)
    }

    /// Prüft gespeicherte Presets: IDs, Namen und Argumente.
    pub fn validate(&self) -> Result<()> {
        let builtin_ids: Vec<String> = builtin().into_iter().map(|p| p.id).collect();
        let mut seen = std::collections::HashSet::new();
        for p in &self.presets {
            let id_ok = !p.id.is_empty()
                && p.id.len() <= 48
                && p.id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !id_ok || p.id == NONE || !seen.insert(p.id.clone()) {
                return Err(Error::Invalid(format!("Ungültige Preset-ID {:?}", p.id)));
            }
            if !builtin_ids.contains(&p.id) && p.name.trim().is_empty() {
                return Err(Error::Invalid(
                    "Ein eigenes Preset braucht einen Namen".into(),
                ));
            }
            check_args(&p.args)?;
        }
        if self.default_preset != NONE && self.find(&self.default_preset).is_none() {
            return Err(Error::Invalid(format!(
                "Unbekanntes Standard-Preset {:?}",
                self.default_preset
            )));
        }
        Ok(())
    }

    /// Eingebaute Presets, die gleich wie das Original sind, nicht speichern.
    pub(crate) fn normalize(&mut self) {
        let builtin = builtin();
        self.presets.retain(|s| {
            builtin
                .iter()
                .find(|b| b.id == s.id)
                .is_none_or(|b| b.args != s.args)
        });
        for s in &mut self.presets {
            if let Some(b) = builtin.iter().find(|b| b.id == s.id) {
                s.name = b.name.clone();
                s.min_java = b.min_java;
            }
        }
        if self.default_preset != NONE && self.find(&self.default_preset).is_none() {
            self.default_preset = NONE.into();
        }
    }
}

/// Argumente, die Pinecone selbst setzt oder die den Start verhindern würden, sind nicht erlaubt.
pub fn check_args(args: &[String]) -> Result<()> {
    if args.len() > 200 {
        return Err(Error::Invalid("Zu viele JVM-Argumente".into()));
    }
    for a in args {
        if !a.starts_with('-') || a.len() > 300 || a.chars().any(char::is_whitespace) {
            return Err(Error::Invalid(format!("Ungültiges JVM-Argument {a:?}")));
        }
        if a.starts_with("-Xmx") || a == "-cp" || a == "-classpath" || a.starts_with("-jar") {
            return Err(Error::Invalid(format!(
                "{a} setzt Pinecone selbst (Arbeitsspeicher in der Instanz einstellen)"
            )));
        }
    }
    Ok(())
}

/// Text aus einem Eingabefeld in Argumente zerlegen (Leerzeichen und Zeilenumbrüche).
pub fn parse_args(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

/// Argumente, die ein Preset beim Start beiträgt; leer, wenn die Java-Version zu alt ist.
/// `java_major` = `None`, wenn die Version unbekannt ist (eigener Java-Pfad): dann gilt das Preset.
pub fn launch_args(preset: &JvmPreset, java_major: Option<u32>) -> Vec<String> {
    match java_major {
        Some(major) if major < preset.min_java => Vec::new(),
        _ => preset.args.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_presets_can_be_customized_and_reset() {
        let mut s = JvmSettings::default();
        assert_eq!(s.presets().len(), 3);
        assert!(s.presets().iter().all(|p| p.builtin && !p.modified));

        // Aikar anpassen, eigenes Preset anlegen.
        let mut aikar = s.find("aikar").unwrap().args;
        aikar.push("-XX:+UseStringDeduplication".into());
        s.presets.push(StoredPreset {
            id: "aikar".into(),
            name: String::new(),
            args: aikar,
            min_java: 8,
        });
        s.presets.push(StoredPreset {
            id: "mein-preset".into(),
            name: "Mein Preset".into(),
            args: vec!["-XX:+UseG1GC".into()],
            min_java: 17,
        });
        s.default_preset = "mein-preset".into();
        s.validate().unwrap();
        s.normalize();
        let all = s.presets();
        assert_eq!(all.len(), 4);
        let a = all.iter().find(|p| p.id == "aikar").unwrap();
        assert!(a.modified);
        assert_eq!(a.name, "Aikar's Flags");
        assert_eq!(all[3].name, "Mein Preset");
        assert!(!all[3].builtin);

        // Zurücksetzen = gleiche Argumente wie eingebaut → wird nicht mehr gespeichert.
        s.presets[0].args = builtin()[0].args.clone();
        s.normalize();
        assert_eq!(s.presets.len(), 1);

        // Gelöschtes Standard-Preset fällt auf „Keine“ zurück.
        s.presets.clear();
        s.normalize();
        assert_eq!(s.default_preset, NONE);
    }

    #[test]
    fn rejects_bad_presets() {
        let bad = |id: &str, name: &str, args: &[&str]| JvmSettings {
            default_preset: NONE.into(),
            presets: vec![StoredPreset {
                id: id.into(),
                name: name.into(),
                args: args.iter().map(|a| a.to_string()).collect(),
                min_java: 8,
            }],
        };
        assert!(bad("ok", "Ok", &["-XX:+UseG1GC"]).validate().is_ok());
        assert!(bad("Groß", "x", &[]).validate().is_err());
        assert!(bad("none", "x", &[]).validate().is_err());
        assert!(bad("ohne-name", " ", &[]).validate().is_err());
        assert!(bad("x", "x", &["-Xmx8G"]).validate().is_err());
        assert!(bad("x", "x", &["kein-flag"]).validate().is_err());
        assert!(bad("x", "x", &["-cp"]).validate().is_err());
        let unknown_default = JvmSettings {
            default_preset: "gibt-es-nicht".into(),
            presets: vec![],
        };
        assert!(unknown_default.validate().is_err());
    }

    #[test]
    fn presets_need_a_new_enough_java() {
        let zgc = builtin().into_iter().find(|p| p.id == "zgc").unwrap();
        assert!(launch_args(&zgc, Some(17)).is_empty());
        assert_eq!(launch_args(&zgc, Some(21)), zgc.args);
        assert_eq!(launch_args(&zgc, None), zgc.args);
        assert_eq!(parse_args("-XX:+A\n  -XX:B=1 "), vec!["-XX:+A", "-XX:B=1"]);
    }
}
