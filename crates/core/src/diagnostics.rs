//! Diagnose-Export: ein ZIP mit allem, was man für eine Fehlersuche braucht.
//!
//! Enthalten sind Systeminfos, die Logs des Launchers und je Profil Spiel-Log, Konsolenausgabe,
//! die neuesten Crash-Reports, die Inhaltsliste und die Profil-Konfiguration. Account-Daten
//! (`accounts.json`, Schlüsselbund) kommen nie mit; Tokens in Logs werden geschwärzt.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::launcher::Launcher;
use crate::profiles::Profile;
use crate::{Error, LAUNCHER_VERSION, Result};

/// Höchstens so viele Bytes je Logdatei (vom Ende her), damit das ZIP klein bleibt.
const MAX_LOG: u64 = 4 * 1024 * 1024;
const CRASH_REPORTS: usize = 3;
const LAUNCHER_LOGS: usize = 3;

/// Muster, die Zugangsdaten enthalten können.
static SECRETS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        // JWTs (Minecraft- und Xbox-Tokens)
        r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}",
        // Startargumente und Schlüssel=Wert-Paare
        r"(?i)(--accessToken\s+)\S+",
        r#"(?i)((?:access|refresh)_?token["']?\s*[:=]\s*["']?)[^\s"',}]+"#,
        // Microsoft-Refresh-Tokens (M.C… bzw. M.R3_…)
        r"\bM\.[A-Z0-9]{1,4}_[A-Za-z0-9!*$._-]{20,}",
    ]
    .iter()
    .map(|p| Regex::new(p).expect("gültiges Muster"))
    .collect()
});

/// Ersetzt Tokens durch `<entfernt>`.
pub fn redact(text: &str) -> String {
    let mut out = text.to_string();
    for re in SECRETS.iter() {
        out = re
            .replace_all(&out, |c: &regex::Captures| match c.get(1) {
                Some(prefix) => format!("{}<entfernt>", prefix.as_str()),
                None => "<entfernt>".into(),
            })
            .into_owned();
    }
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    pub path: String,
    pub files: usize,
    pub bytes: u64,
}

/// Schreibt das Diagnose-ZIP. `profiles`: nur diese Profile (IDs), sonst alle.
/// `launcher_logs`: Ordner mit den Logdateien der App bzw. CLI.
pub async fn export(
    launcher: &Launcher,
    target: &Path,
    profiles: Option<&[String]>,
    launcher_logs: Option<&Path>,
) -> Result<DiagnosticsReport> {
    let all = launcher.profiles().list()?;
    let chosen: Vec<Profile> = match profiles {
        Some(ids) => all.into_iter().filter(|p| ids.contains(&p.id)).collect(),
        None => all,
    };
    let javas = launcher.java().installed().await;

    let mut info = String::new();
    info.push_str(&format!("TwoGG Pinecone {LAUNCHER_VERSION}\n"));
    info.push_str(&format!(
        "System: {} {} ({})\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY
    ));
    info.push_str(&format!(
        "Erstellt: {} (Unix-Zeit)\n",
        crate::stats::now_secs()
    ));
    info.push_str(&format!(
        "Datenordner: {}\n",
        launcher.paths().root().display()
    ));
    info.push_str("\nJava:\n");
    for j in &javas {
        info.push_str(&format!(
            "  {} (Java {}{}) {}\n",
            j.version,
            j.major,
            if j.managed { ", von Pinecone" } else { "" },
            j.path.display()
        ));
    }
    info.push_str(&format!("\nProfile im Export: {}\n", chosen.len()));
    for p in &chosen {
        let loader = p
            .loader
            .as_ref()
            .map(|l| format!("{} {}", l.kind.name(), l.version))
            .unwrap_or_else(|| "Vanilla".into());
        info.push_str(&format!(
            "  {} ({}): Minecraft {}, {loader}, RAM {} MB, Java {}\n",
            p.name,
            p.id,
            p.minecraft,
            p.memory_mb
                .map(|m| m.to_string())
                .unwrap_or_else(|| "Standard".into()),
            p.java
                .as_ref()
                .map(|j| j.display().to_string())
                .unwrap_or_else(|| "automatisch".into())
        ));
    }

    // Dateien sammeln: (Pfad im ZIP, Quelle, Text schwärzen?)
    let mut files: Vec<(String, PathBuf, bool)> = Vec::new();
    if let Some(dir) = launcher_logs {
        for f in newest(dir, |n| n.contains(".log"), LAUNCHER_LOGS) {
            let name = file_name(&f);
            files.push((format!("launcher/{name}"), f, true));
        }
    }
    let settings = launcher.paths().root().join("settings.json");
    if settings.is_file() {
        files.push(("launcher/settings.json".into(), settings, false));
    }
    let mut extra: Vec<(String, String)> = vec![("info.txt".into(), info)];
    for p in &chosen {
        let base = format!("profiles/{}", p.id);
        let instance = launcher.paths().instance_dir(&p.id);
        let game = launcher.game_dir(p);
        for (name, src, text) in [
            ("profile.json", instance.join("profile.json"), false),
            ("pack.lock.json", instance.join("pack.lock.json"), false),
            ("syncpacks.json", instance.join("syncpacks.json"), false),
            ("latest.log", game.join("logs/latest.log"), true),
            (
                "pinecone-console.log",
                game.join("logs/pinecone-console.log"),
                true,
            ),
        ] {
            if src.is_file() {
                files.push((format!("{base}/{name}"), src, text));
            }
        }
        for f in newest(
            &game.join("crash-reports"),
            |n| n.ends_with(".txt"),
            CRASH_REPORTS,
        ) {
            let name = file_name(&f);
            files.push((format!("{base}/crash-reports/{name}"), f, true));
        }
        // Inhalte als Liste (die Dateien selbst wären zu groß).
        let mut listing = String::new();
        for c in launcher.content().list(&p.id, &game)? {
            listing.push_str(&format!(
                "{}{}\t{}\t{}\n",
                if c.enabled { "" } else { "[aus] " },
                c.key,
                c.version.unwrap_or_default(),
                match c.source {
                    crate::content::Source::Modrinth { .. } => "Modrinth",
                    crate::content::Source::Curseforge { .. } => "CurseForge",
                    crate::content::Source::Local => "lokal",
                }
            ));
        }
        extra.push((format!("{base}/inhalte.txt"), listing));
    }

    let target_buf = target.to_path_buf();
    let count = files.len() + extra.len();
    tokio::task::spawn_blocking(move || -> Result<()> {
        if let Some(parent) = target_buf.parent() {
            std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
        }
        let out = std::fs::File::create(&target_buf).map_err(Error::io(&target_buf))?;
        let mut zip = zip::ZipWriter::new(out);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let zerr = |source| Error::Zip {
            path: target_buf.clone(),
            source,
        };
        for (name, text) in extra {
            zip.start_file(name, opts).map_err(zerr)?;
            zip.write_all(redact(&text).as_bytes())
                .map_err(Error::io(&target_buf))?;
        }
        for (name, src, text) in files {
            let data = read_tail(&src, MAX_LOG)?;
            zip.start_file(name, opts).map_err(zerr)?;
            let data = if text {
                redact(&String::from_utf8_lossy(&data)).into_bytes()
            } else {
                data
            };
            zip.write_all(&data).map_err(Error::io(&target_buf))?;
        }
        zip.finish().map_err(zerr)?;
        Ok(())
    })
    .await
    .map_err(|e| Error::Invalid(format!("Export abgebrochen: {e}")))??;

    let bytes = std::fs::metadata(target).map(|m| m.len()).unwrap_or(0);
    Ok(DiagnosticsReport {
        path: target.display().to_string(),
        files: count,
        bytes,
    })
}

/// Die neuesten `limit` Dateien eines Ordners, deren Name `filter` erfüllt.
fn newest(dir: &Path, filter: impl Fn(&str) -> bool, limit: usize) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter(|e| filter(&e.file_name().to_string_lossy()))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    found.sort_by_key(|f| std::cmp::Reverse(f.0));
    found.into_iter().take(limit).map(|(_, p)| p).collect()
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Liest höchstens `max` Bytes vom Ende einer Datei (Logs werden am Ende interessant).
fn read_tail(path: &Path, max: u64) -> Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).map_err(Error::io(path))?;
    let len = f.metadata().map_err(Error::io(path))?.len();
    if len > max {
        f.seek(SeekFrom::Start(len - max))
            .map_err(Error::io(path))?;
    }
    let mut buf = Vec::new();
    f.take(max).read_to_end(&mut buf).map_err(Error::io(path))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_tokens() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let text = format!(
            "--username Tobi --accessToken {jwt} --version 1.21\n\"access_token\": \"geheim123\"\nrefresh_token=M.C507_BAY.0.U.-abcdefghijklmnopqrstuvwxyz\nSetting user: Tobi"
        );
        let out = redact(&text);
        assert!(!out.contains("eyJhbGci"), "{out}");
        assert!(!out.contains("geheim123"), "{out}");
        assert!(!out.contains("abcdefghijklmnop"), "{out}");
        assert!(out.contains("--accessToken <entfernt>"), "{out}");
        assert!(out.contains("Setting user: Tobi"));
        assert!(out.contains("--version 1.21"));
    }

    #[test]
    fn tail_keeps_the_end() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("x.log");
        std::fs::write(&f, "anfang\nmitte\nende\n").unwrap();
        assert_eq!(read_tail(&f, 5).unwrap(), b"ende\n");
        assert_eq!(read_tail(&f, 100).unwrap().len(), 18);
    }
}
