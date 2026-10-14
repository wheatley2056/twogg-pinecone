//! Absturz erkennen und erklären (F-35): neuester Crash-Report der Sitzung, letzte Log-Zeilen und
//! eine Ursache nach festen Regeln (fehlende Abhängigkeit, Java-Version, Speicher, doppelte Mod).

use std::path::Path;
use std::sync::LazyLock;
use std::time::UNIX_EPOCH;

use regex::Regex;
use serde::Serialize;

use crate::diagnostics::redact;

/// So viele Log-Zeilen zeigt die Absturztafel (AK-35.2).
pub const LOG_TAIL: usize = 200;
/// Crash-Reports werden nur bis zu dieser Größe gelesen und angezeigt.
const REPORT_MAX: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CrashCause {
    /// Eine Mod braucht eine andere, die fehlt.
    #[serde(rename_all = "camelCase")]
    MissingDependency {
        r#mod: String,
        dependency: String,
    },
    /// Mod oder Spiel braucht eine neuere Java-Hauptversion.
    #[serde(rename_all = "camelCase")]
    JavaVersion {
        required: Option<u32>,
        actual: Option<u32>,
    },
    OutOfMemory,
    /// Dieselbe Mod-ID liegt mehrfach im Mods-Ordner.
    #[serde(rename_all = "camelCase")]
    DuplicateMod {
        id: String,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashInfo {
    pub cause: CrashCause,
    /// Die Zeile, an der die Ursache erkannt wurde.
    pub trigger: Option<String>,
    /// Dateiname des neuesten Crash-Reports dieser Sitzung.
    pub report: Option<String>,
    /// Inhalt des Reports (gekürzt, ohne Tokens).
    pub report_text: Option<String>,
    /// Weitere Crash-Reports derselben Sitzung, neueste zuerst.
    pub others: Vec<String>,
    /// Letzte Log-Zeilen, ohne Tokens.
    pub log_tail: Vec<String>,
}

/// Crash-Reports in `crash-reports/`, die seit `since` (Unix-Sekunden) entstanden sind,
/// neueste zuerst.
pub fn session_reports(game_dir: &Path, since: u64) -> Vec<String> {
    let mut found: Vec<(u64, String)> = std::fs::read_dir(game_dir.join("crash-reports"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.ends_with(".txt") {
                return None;
            }
            let modified = e
                .metadata()
                .ok()?
                .modified()
                .ok()?
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_secs();
            // Etwas Spielraum für grobe Zeitstempel einiger Dateisysteme.
            (modified + 2 >= since).then_some((modified, name))
        })
        .collect();
    found.sort_by(|a, b| b.cmp(a));
    found.into_iter().map(|(_, n)| n).collect()
}

/// Wertet eine beendete Sitzung aus. `log` ist die Spielausgabe, `since` der Startzeitpunkt.
pub fn analyze(game_dir: &Path, since: u64, log: &str) -> CrashInfo {
    let mut reports = session_reports(game_dir, since);
    let report = (!reports.is_empty()).then(|| reports.remove(0));
    let report_text = report.as_ref().and_then(|name| {
        let bytes = std::fs::read(game_dir.join("crash-reports").join(name)).ok()?;
        let text = String::from_utf8_lossy(&bytes[..bytes.len().min(REPORT_MAX)]).to_string();
        Some(redact(&text))
    });
    // Ursache zuerst im Report, dann im Log suchen.
    let (cause, trigger) = report_text
        .as_deref()
        .and_then(detect)
        .or_else(|| detect(log))
        .map_or((CrashCause::Unknown, None), |(c, t)| (c, Some(t)));
    let lines: Vec<&str> = log.lines().collect();
    let log_tail = lines[lines.len().saturating_sub(LOG_TAIL)..]
        .iter()
        .map(|l| redact(l))
        .collect();
    CrashInfo {
        cause,
        trigger: trigger.map(|t| redact(t.trim())),
        report,
        report_text,
        others: reports,
        log_tail,
    }
}

static FABRIC_MISSING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"Mod '([^']+)' \(([^)]+)\) \S+ requires (?:any version|version [^,]+?) of (?:mod )?'?([^',!]+?)'?(?: \([^)]*\))?, which is missing",
    )
    .unwrap()
});
static FORGE_MISSING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Mod ID: '([^']+)', Requested by: '([^']+)'").unwrap());
static CLASS_VERSION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"class file version (\d+)(?:\.\d+)?\).*?up to (\d+)(?:\.\d+)?").unwrap()
});
static JAVA_REQUIRED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\brequires? (?:at least )?java (\d+)|java (\d+) or (?:newer|above|later|higher)",
    )
    .unwrap()
});
static DUPLICATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)duplicate mod").unwrap());
static MOD_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:Mod ID:? ?|mod id |\bmod )'?([a-z0-9_.\-]+)'?").unwrap());

/// Sucht die erste erkennbare Ursache; liefert sie mit der auslösenden Zeile.
pub fn detect(text: &str) -> Option<(CrashCause, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let line_of = |needle: &str| {
        lines
            .iter()
            .find(|l| l.contains(needle))
            .map(|l| l.to_string())
            .unwrap_or_else(|| needle.to_string())
    };

    if let Some(c) = FABRIC_MISSING.captures(text) {
        return Some((
            CrashCause::MissingDependency {
                r#mod: c[1].to_string(),
                dependency: c[3].trim().to_string(),
            },
            line_of(&c[0]),
        ));
    }
    if text.contains("Missing or unsupported mandatory dependencies")
        && let Some(c) = FORGE_MISSING.captures(text)
    {
        return Some((
            CrashCause::MissingDependency {
                r#mod: c[2].to_string(),
                dependency: c[1].to_string(),
            },
            line_of(&c[0]),
        ));
    }
    if text.contains("UnsupportedClassVersionError") {
        let (required, actual) = CLASS_VERSION
            .captures(text)
            .map(|c| (class_to_java(&c[1]), class_to_java(&c[2])))
            .unwrap_or((None, None));
        return Some((
            CrashCause::JavaVersion { required, actual },
            line_of("UnsupportedClassVersionError"),
        ));
    }
    if let Some(i) = lines.iter().position(|l| JAVA_REQUIRED.is_match(l)) {
        let c = JAVA_REQUIRED.captures(lines[i]).unwrap();
        let required = c.get(1).or(c.get(2)).and_then(|m| m.as_str().parse().ok());
        return Some((
            CrashCause::JavaVersion {
                required,
                actual: None,
            },
            lines[i].to_string(),
        ));
    }
    if text.contains("java.lang.OutOfMemoryError") {
        return Some((
            CrashCause::OutOfMemory,
            line_of("java.lang.OutOfMemoryError"),
        ));
    }
    if let Some(i) = lines.iter().position(|l| DUPLICATE.is_match(l)) {
        // Die ID steht in derselben oder einer der nächsten Zeilen.
        let id = lines[i..lines.len().min(i + 4)]
            .iter()
            .find_map(|l| MOD_ID.captures(l).map(|c| c[1].to_string()))?;
        return Some((CrashCause::DuplicateMod { id }, lines[i].to_string()));
    }
    None
}

/// Class-Datei-Version → Java-Hauptversion (52 = Java 8, 65 = Java 21).
fn class_to_java(v: &str) -> Option<u32> {
    v.parse::<u32>().ok().filter(|v| *v > 44).map(|v| v - 44)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fabric_missing_dependency() {
        let log = "[main/ERROR]: Incompatible mods found!\n\
            net.fabricmc.loader.impl.FormattedException: Some of your mods are incompatible with the game or each other!\n\
            A potential solution has been determined, this may resolve your problem:\n\
            \t - Install fabric-api, any version.\n\
            More details:\n\
            \t - Mod 'Sodium Extra' (sodium-extra) 0.5.1 requires any version of fabric-api, which is missing!";
        let (cause, trigger) = detect(log).unwrap();
        assert_eq!(
            cause,
            CrashCause::MissingDependency {
                r#mod: "Sodium Extra".into(),
                dependency: "fabric-api".into()
            }
        );
        assert!(trigger.contains("which is missing"), "{trigger}");

        let versioned = "\t - Mod 'Iris' (iris) 1.7.0 requires version 0.5.0 or later of mod 'Sodium' (sodium), which is missing!";
        assert_eq!(
            detect(versioned).unwrap().0,
            CrashCause::MissingDependency {
                r#mod: "Iris".into(),
                dependency: "Sodium".into()
            }
        );
    }

    #[test]
    fn forge_missing_dependency() {
        let log = "Missing or unsupported mandatory dependencies:\n\
            \tMod ID: 'jei', Requested by: 'justenoughprofessions', Expected range: '[19.0,)', Actual version: '[MISSING]'";
        assert_eq!(
            detect(log).unwrap().0,
            CrashCause::MissingDependency {
                r#mod: "justenoughprofessions".into(),
                dependency: "jei".into()
            }
        );
    }

    #[test]
    fn java_version() {
        let log = "Exception in thread \"main\" java.lang.UnsupportedClassVersionError: net/minecraft/client/main/Main has been compiled by a more recent version of the Java Runtime (class file version 65.0), this version of the Java Runtime only recognizes class file versions up to 61.0";
        assert_eq!(
            detect(log).unwrap().0,
            CrashCause::JavaVersion {
                required: Some(21),
                actual: Some(17)
            }
        );
        let fabric = "net.fabricmc.loader.impl.FormattedException: Mod 'Minecraft' (minecraft) 1.20.5 requires Java 21 or later";
        assert!(matches!(
            detect(fabric).unwrap().0,
            CrashCause::JavaVersion {
                required: Some(21),
                ..
            }
        ));
    }

    #[test]
    fn out_of_memory() {
        let log =
            "[Render thread/ERROR]: oops\njava.lang.OutOfMemoryError: Java heap space\n\tat foo";
        let (cause, trigger) = detect(log).unwrap();
        assert_eq!(cause, CrashCause::OutOfMemory);
        assert_eq!(trigger, "java.lang.OutOfMemoryError: Java heap space");
    }

    #[test]
    fn duplicate_mod() {
        let forge = "net.minecraftforge.fml.loading.moddiscovery.InvalidModFileException: Found duplicate mods:\n\
            \tMod ID: 'create' from mod files: create-1.20.1-0.5.1.jar, create-1.20.1-0.5.1f.jar";
        assert_eq!(
            detect(forge).unwrap().0,
            CrashCause::DuplicateMod {
                id: "create".into()
            }
        );
        let fabric = "net.fabricmc.loader.impl.discovery.ModResolutionException: Duplicate mod id 'sodium' found in sodium-a.jar and sodium-b.jar";
        assert_eq!(
            detect(fabric).unwrap().0,
            CrashCause::DuplicateMod {
                id: "sodium".into()
            }
        );
    }

    #[test]
    fn unknown_without_pattern() {
        assert_eq!(detect("[main/INFO]: alles gut\nNullPointerException"), None);
    }

    #[test]
    fn analyze_reads_session_reports_and_redacts() {
        let dir = tempfile::tempdir().unwrap();
        let reports = dir.path().join("crash-reports");
        std::fs::create_dir_all(&reports).unwrap();
        std::fs::write(
            reports.join("crash-2026-10-14_10.00.00-client.txt"),
            "älter",
        )
        .unwrap();
        std::fs::write(
            reports.join("crash-2026-10-14_10.00.05-client.txt"),
            "---- Minecraft Crash Report ----\njava.lang.OutOfMemoryError: Java heap space",
        )
        .unwrap();
        let log: String = (0..300)
            .map(|i| format!("Zeile {i}\n"))
            .chain(["--accessToken eyJabc.def.ghi\n".to_string()])
            .collect();

        let info = analyze(dir.path(), 0, &log);
        assert_eq!(info.log_tail.len(), LOG_TAIL);
        assert!(
            info.log_tail.iter().all(|l| !l.contains("eyJabc")),
            "Token entfernt"
        );
        // Gleiche Änderungszeit: der Name entscheidet, neueste zuerst.
        assert_eq!(
            info.report.as_deref(),
            Some("crash-2026-10-14_10.00.05-client.txt")
        );
        assert_eq!(info.others, vec!["crash-2026-10-14_10.00.00-client.txt"]);
        assert_eq!(info.cause, CrashCause::OutOfMemory);

        // Reports aus früheren Sitzungen zählen nicht.
        let later = analyze(dir.path(), u64::MAX / 2, "");
        assert_eq!((later.report, later.cause), (None, CrashCause::Unknown));
    }
}
