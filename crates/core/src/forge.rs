//! Forge und NeoForge über deren offizielle Installer-JARs.
//!
//! Ablauf wie beim Installer selbst, nur ohne dessen Oberfläche:
//!
//! 1. [`install_profile`]: Installer laden, eingebettete Bibliotheken (`maven/`) nach `libraries/`
//!    entpacken, das Version-JSON unter Pinecones Versions-ID ablegen (erbt von Vanilla).
//! 2. Danach lädt der normale [`Installer`](crate::install::Installer) Vanilla und alle Libraries.
//! 3. [`run_processors`]: Ab Minecraft 1.13 erzeugen „Prozessoren“ (kleine Java-Programme aus dem
//!    Installer) den gepatchten Client. Sie laufen einmal mit dem Java der Instanz; eine
//!    Markierungsdatei hält fest, dass alles fertig ist.
//!
//! Alte Installer (bis 1.12.2) enthalten das Version-JSON als `versionInfo` und die Forge-JAR
//! direkt; Prozessoren gibt es dort nicht.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::download::{DownloadTask, Downloader, sha1_file};
use crate::loader::{LoaderKind, LoaderVersion, compare_versions};
use crate::manifest::{Library, maven_path};
use crate::paths::{Paths, safe_relative};
use crate::{Error, Result};

const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/";
const FORGE_METADATA: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
const FORGE_PROMOTIONS: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const NEOFORGE_VERSIONS: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

const INSTALLER: &str = "installer.jar";
const PROFILE: &str = "install_profile.json";
/// Markiert, dass die Prozessoren erfolgreich gelaufen sind.
const DONE: &str = ".forge-ready";

/// Versionen für eine Minecraft-Version, neueste zuerst.
pub async fn versions(
    downloader: &Downloader,
    kind: LoaderKind,
    minecraft: &str,
) -> Result<Vec<LoaderVersion>> {
    let mut list: Vec<LoaderVersion> = match kind {
        LoaderKind::Forge => {
            let xml = downloader.fetch_bytes(FORGE_METADATA).await?;
            let xml = String::from_utf8_lossy(&xml);
            let prefix = format!("{minecraft}-");
            // Forge kennt keine Vorabversionen; welche empfohlen ist, sagt `recommended`.
            maven_versions(&xml)
                .into_iter()
                .filter_map(|v| v.strip_prefix(&prefix).map(str::to_string))
                .map(|version| LoaderVersion {
                    version,
                    stable: true,
                })
                .collect::<Vec<_>>()
        }
        LoaderKind::NeoForge => {
            let bytes = downloader.fetch_bytes(NEOFORGE_VERSIONS).await?;
            #[derive(Deserialize)]
            struct List {
                versions: Vec<String>,
            }
            let list: List =
                serde_json::from_slice(&bytes).map_err(Error::json(NEOFORGE_VERSIONS))?;
            let prefix = neoforge_prefix(minecraft);
            list.versions
                .into_iter()
                .filter(|v| v.starts_with(&prefix))
                .map(|v| LoaderVersion {
                    stable: !v.contains("beta") && !v.contains("alpha"),
                    version: v,
                })
                .collect()
        }
        _ => unreachable!("nur Forge und NeoForge"),
    };
    list.sort_by(|a, b| compare_versions(&b.version, &a.version));
    Ok(list)
}

/// Empfohlene Version: bei Forge laut `promotions_slim.json`, sonst die neueste stabile.
pub async fn recommended(
    downloader: &Downloader,
    kind: LoaderKind,
    minecraft: &str,
) -> Result<LoaderVersion> {
    if kind == LoaderKind::Forge {
        for promo in ["recommended", "latest"] {
            if let Some(version) = forge_promotion(downloader, minecraft, promo).await {
                return Ok(LoaderVersion {
                    version,
                    stable: true,
                });
            }
        }
    }
    let list = versions(downloader, kind, minecraft).await?;
    list.iter()
        .find(|v| v.stable)
        .or(list.first())
        .cloned()
        .ok_or_else(|| {
            Error::Invalid(format!(
                "{} unterstützt Minecraft {minecraft} (noch) nicht",
                kind.name()
            ))
        })
}

async fn forge_promotion(downloader: &Downloader, minecraft: &str, which: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Promotions {
        promos: HashMap<String, String>,
    }
    let bytes = downloader.fetch_bytes(FORGE_PROMOTIONS).await.ok()?;
    let p: Promotions = serde_json::from_slice(&bytes).ok()?;
    p.promos.get(&format!("{minecraft}-{which}")).cloned()
}

/// `<version>…</version>` aus einer `maven-metadata.xml`.
fn maven_versions(xml: &str) -> Vec<String> {
    xml.split("<version>")
        .skip(1)
        .filter_map(|s| {
            s.split_once("</version>")
                .map(|(v, _)| v.trim().to_string())
        })
        .collect()
}

/// NeoForge zählt nach der Minecraft-Version: 1.21.1 → `21.1.`, 1.20.4 → `20.4.`, 1.21 → `21.0.`,
/// ab den Jahresversionen (26.1) → `26.1.`.
fn neoforge_prefix(minecraft: &str) -> String {
    match minecraft.strip_prefix("1.") {
        Some(rest) if rest.contains('.') => format!("{rest}."),
        Some(rest) => format!("{rest}.0."),
        None => format!("{minecraft}."),
    }
}

fn installer_url(kind: LoaderKind, minecraft: &str, version: &str) -> String {
    match kind {
        LoaderKind::NeoForge => {
            format!(
                "{NEOFORGE_MAVEN}net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar"
            )
        }
        _ => {
            // Alte Versionen tragen die Minecraft-Version schon im Namen (`10.13.4.1614-1.7.10`).
            let full = format!("{minecraft}-{version}");
            format!("{FORGE_MAVEN}net/minecraftforge/forge/{full}/forge-{full}-installer.jar")
        }
    }
}

#[derive(Debug, Deserialize)]
struct Profile {
    /// Pfad des Version-JSON im Installer, z. B. `/version.json`.
    #[serde(default)]
    json: Option<String>,
    #[serde(default)]
    data: HashMap<String, SidedData>,
    #[serde(default)]
    processors: Vec<Processor>,
    #[serde(default)]
    libraries: Vec<Library>,
    /// Alte Installer: Version-JSON direkt im Profil …
    #[serde(default, rename = "versionInfo")]
    version_info: Option<serde_json::Value>,
    /// … und wo die Forge-JAR liegt.
    #[serde(default)]
    install: Option<LegacyInstall>,
}

#[derive(Debug, Deserialize)]
struct SidedData {
    client: String,
}

#[derive(Debug, Deserialize)]
struct Processor {
    #[serde(default)]
    sides: Option<Vec<String>>,
    jar: String,
    #[serde(default)]
    classpath: Vec<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    outputs: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyInstall {
    /// Maven-Koordinate der Forge-JAR.
    path: String,
    /// Dateiname im Installer.
    file_path: String,
}

/// Schritt 1: Installer laden und das Version-JSON als `versions/<id>/<id>.json` ablegen.
pub async fn install_profile(
    paths: &Paths,
    downloader: &Downloader,
    kind: LoaderKind,
    minecraft: &str,
    version: &str,
    id: &str,
) -> Result<()> {
    let dir = paths.version_dir(id);
    let installer = dir.join(INSTALLER);
    let url = installer_url(kind, minecraft, version);
    match downloader
        .download(&DownloadTask {
            url: url.clone(),
            dest: installer.clone(),
            sha1: None,
            size: None,
        })
        .await
    {
        Ok(_) => {}
        Err(Error::Http { source, .. }) if source.status().is_some_and(|s| s.as_u16() == 404) => {
            return Err(Error::Invalid(format!(
                "{} {version} gibt es nicht für Minecraft {minecraft}",
                kind.name()
            )));
        }
        Err(e) => return Err(e),
    }

    let libraries = paths.libraries();
    let (id, minecraft, dir) = (id.to_string(), minecraft.to_string(), dir.clone());
    tokio::task::spawn_blocking(move || {
        let mut zip = open_zip(&installer)?;
        let profile_bytes = read_entry(&mut zip, &installer, PROFILE)?;
        let profile: Profile =
            serde_json::from_slice(&profile_bytes).map_err(Error::json(PROFILE))?;

        let mut version: serde_json::Value = match (&profile.version_info, &profile.json) {
            (Some(info), _) => {
                // Alter Installer: Forge-JAR an ihren Maven-Platz legen.
                let legacy = profile.install.as_ref().ok_or_else(|| {
                    Error::Invalid("Forge-Installer ohne Angaben zur Forge-JAR".into())
                })?;
                let target = libraries.join(maven_path(&legacy.path).ok_or_else(|| {
                    Error::Invalid(format!("Ungültige Koordinate {}", legacy.path))
                })?);
                let bytes = read_entry(&mut zip, &installer, &legacy.file_path)?;
                write(&target, &bytes)?;
                legacy_fix_libraries(info.clone())
            }
            (None, Some(json)) => {
                let bytes = read_entry(&mut zip, &installer, json.trim_start_matches('/'))?;
                serde_json::from_slice(&bytes).map_err(Error::json(json.as_str()))?
            }
            (None, None) => {
                return Err(Error::Invalid("Unbekanntes Forge-Installer-Format".into()));
            }
        };

        // Mitgelieferte Bibliotheken (z. B. die Forge-JAR selbst) bereitstellen.
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(zip_err(&installer))?;
            let Some(rel) = entry.name().strip_prefix("maven/").map(str::to_string) else {
                continue;
            };
            if entry.is_dir() || rel.is_empty() {
                continue;
            }
            let target = libraries.join(safe_relative(&rel)?);
            if target.exists() {
                continue;
            }
            let mut bytes = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut bytes).map_err(Error::io(&target))?;
            write(&target, &bytes)?;
        }

        // Unter Pinecones ID ablegen; erbt von der Vanilla-Version.
        version["id"] = serde_json::Value::String(id.clone());
        if version.get("inheritsFrom").is_none() {
            version["inheritsFrom"] = serde_json::Value::String(minecraft.clone());
        }
        // Moderne Forge-Versionen filtern den Client über `${version_name}.jar` vom Classpath
        // (`-DignoreList`); deshalb liegt der Vanilla-Client unter der Forge-ID.
        if version.get("jar").is_none() {
            version["jar"] = serde_json::Value::String(id.clone());
        }
        let json = serde_json::to_vec_pretty(&version).map_err(Error::json("version.json"))?;
        write(&dir.join(PROFILE), &profile_bytes)?;
        if profile.version_info.is_some() || profile.processors.is_empty() {
            write(&dir.join(DONE), b"")?;
        }
        // Zuletzt: Erst ein vollständiges Profil gilt als installiert.
        write(&dir.join(format!("{id}.json")), &json)?;
        Ok(())
    })
    .await
    .map_err(|e| Error::Invalid(format!("Forge-Installation abgebrochen: {e}")))?
}

/// Alte Profile verweisen noch auf `http://files.minecraftforge.net/maven/`.
fn legacy_fix_libraries(mut info: serde_json::Value) -> serde_json::Value {
    if let Some(libs) = info.get_mut("libraries").and_then(|l| l.as_array_mut()) {
        for lib in libs {
            if let Some(url) = lib.get("url").and_then(|u| u.as_str()) {
                let fixed = url
                    .replace("http://files.minecraftforge.net/maven/", FORGE_MAVEN)
                    .replace("https://files.minecraftforge.net/maven/", FORGE_MAVEN)
                    .replace("http://", "https://");
                lib["url"] = serde_json::Value::String(fixed);
            }
        }
    }
    info
}

/// Ob die Prozessoren für diese Version schon gelaufen sind.
pub fn is_ready(paths: &Paths, id: &str) -> bool {
    paths.version_dir(id).join(DONE).exists()
}

/// Schritt 3: Prozessoren ausführen (nach dem Laden von Vanilla und den Libraries).
pub async fn run_processors(
    paths: &Paths,
    downloader: &Downloader,
    id: &str,
    minecraft: &str,
    java: &Path,
) -> Result<()> {
    if is_ready(paths, id) {
        return Ok(());
    }
    let dir = paths.version_dir(id);
    let profile_path = dir.join(PROFILE);
    let bytes = tokio::fs::read(&profile_path)
        .await
        .map_err(Error::io(&profile_path))?;
    let profile: Profile = serde_json::from_slice(&bytes).map_err(Error::json(PROFILE))?;
    let libraries = paths.libraries();

    // Bibliotheken der Prozessoren (ohne Adresse: liegen schon aus `maven/` bereit).
    let tasks: Vec<DownloadTask> = profile
        .libraries
        .iter()
        .filter_map(Library::artifact)
        .filter(|a| !a.url.is_empty())
        .map(|a| DownloadTask {
            dest: libraries.join(&a.path),
            url: a.url,
            sha1: a.sha1,
            size: a.size,
        })
        .collect();
    downloader.download_all(tasks, |_| {}).await?;

    let installer = dir.join(INSTALLER);
    let data_dir = dir.join("installer-data");
    let mut data = HashMap::from([
        ("SIDE".to_string(), "client".to_string()),
        ("MINECRAFT_VERSION".into(), minecraft.to_string()),
        (
            // Vanilla-Client, unter der Forge-ID abgelegt (siehe `install_profile`).
            "MINECRAFT_JAR".into(),
            display(&paths.client_jar(id)),
        ),
        ("ROOT".into(), display(paths.root())),
        ("INSTALLER".into(), display(&installer)),
        ("LIBRARY_DIR".into(), display(&libraries)),
    ]);
    {
        let installer = installer.clone();
        let data_dir = data_dir.clone();
        let libraries = libraries.clone();
        let entries: Vec<(String, String)> = profile
            .data
            .iter()
            .map(|(k, v)| (k.clone(), v.client.clone()))
            .collect();
        let resolved = tokio::task::spawn_blocking(move || -> Result<Vec<(String, String)>> {
            let mut zip = open_zip(&installer)?;
            entries
                .into_iter()
                .map(|(key, value)| {
                    let value = if let Some(coord) = bracketed(&value) {
                        display(&libraries.join(coord_path(coord)?))
                    } else if let Some(literal) =
                        value.strip_prefix('\'').and_then(|v| v.strip_suffix('\''))
                    {
                        literal.to_string()
                    } else if let Some(entry) = value.strip_prefix('/') {
                        // Datei aus dem Installer, z. B. `/data/client.lzma`.
                        let target = data_dir.join(safe_relative(entry)?);
                        let bytes = read_entry(&mut zip, &installer, entry)?;
                        write(&target, &bytes)?;
                        display(&target)
                    } else {
                        value
                    };
                    Ok((key, value))
                })
                .collect()
        })
        .await
        .map_err(|e| Error::Invalid(format!("Forge-Installation abgebrochen: {e}")))??;
        data.extend(resolved);
    }

    let separator = if cfg!(windows) { ";" } else { ":" };
    for (n, p) in profile.processors.iter().enumerate() {
        if p.sides
            .as_ref()
            .is_some_and(|s| !s.iter().any(|x| x == "client"))
        {
            continue;
        }
        let outputs: Vec<(PathBuf, String)> = p
            .outputs
            .iter()
            .map(|(k, v)| {
                Ok((
                    PathBuf::from(substitute(k, &data, &libraries)?),
                    substitute(v, &data, &libraries)?,
                ))
            })
            .collect::<Result<_>>()?;
        if !outputs.is_empty() && outputs_ok(&outputs).await {
            continue;
        }

        let jar = libraries.join(coord_path(&p.jar)?);
        let main_class = main_class(&jar).await?;
        let mut classpath = vec![display(&jar)];
        for c in &p.classpath {
            classpath.push(display(&libraries.join(coord_path(c)?)));
        }
        let args = p
            .args
            .iter()
            .map(|a| substitute(a, &data, &libraries))
            .collect::<Result<Vec<_>>>()?;

        tracing::info!(
            "Forge-Prozessor {}/{}: {}",
            n + 1,
            profile.processors.len(),
            p.jar
        );
        let mut cmd = tokio::process::Command::new(java);
        cmd.arg("-cp")
            .arg(classpath.join(separator))
            .arg(&main_class)
            .args(&args)
            .current_dir(paths.root())
            .kill_on_drop(true);
        #[cfg(windows)]
        {
            // Kein Konsolenfenster pro Prozessor.
            cmd.creation_flags(0x0800_0000);
        }
        let out = cmd.output().await.map_err(|e| {
            Error::Invalid(format!("Java für den Forge-Installer nicht startbar: {e}"))
        })?;
        if !out.status.success() {
            let log = String::from_utf8_lossy(&out.stderr);
            let log = if log.trim().is_empty() {
                String::from_utf8_lossy(&out.stdout).to_string()
            } else {
                log.to_string()
            };
            let tail: Vec<&str> = log.lines().rev().take(8).collect();
            tracing::warn!("Forge-Prozessor {} fehlgeschlagen:\n{log}", p.jar);
            return Err(Error::Invalid(format!(
                "Forge-Installation fehlgeschlagen ({}): {}",
                p.jar,
                tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
            )));
        }
        if !outputs.is_empty() && !outputs_ok(&outputs).await {
            return Err(Error::Invalid(format!(
                "Forge-Installation: Ergebnis von {} stimmt nicht",
                p.jar
            )));
        }
    }

    let _ = tokio::fs::remove_dir_all(&data_dir).await;
    write(&dir.join(DONE), b"")
}

fn bracketed(s: &str) -> Option<&str> {
    s.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
}

fn coord_path(coord: &str) -> Result<PathBuf> {
    let rel =
        maven_path(coord).ok_or_else(|| Error::Invalid(format!("Ungültige Koordinate {coord}")))?;
    safe_relative(&rel)
}

/// `{KEY}` durch Daten ersetzen, `[koordinate]` durch den Bibliothekspfad.
fn substitute(arg: &str, data: &HashMap<String, String>, libraries: &Path) -> Result<String> {
    if let Some(coord) = bracketed(arg) {
        return Ok(display(&libraries.join(coord_path(coord)?)));
    }
    let mut out = String::new();
    let mut rest = arg;
    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start..].find('}') else {
            break;
        };
        let key = &rest[start + 1..start + len];
        let value = data.get(key).ok_or_else(|| {
            Error::Invalid(format!("Forge-Installer: unbekannter Wert {{{key}}}"))
        })?;
        out.push_str(&rest[..start]);
        out.push_str(value);
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

async fn outputs_ok(outputs: &[(PathBuf, String)]) -> bool {
    for (path, sha1) in outputs {
        let expected = sha1.trim_matches('\'');
        match sha1_file(path).await {
            Ok(actual) if actual.eq_ignore_ascii_case(expected) => {}
            _ => return false,
        }
    }
    true
}

async fn main_class(jar: &Path) -> Result<String> {
    let jar = jar.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut zip = open_zip(&jar)?;
        let manifest = read_entry(&mut zip, &jar, "META-INF/MANIFEST.MF")?;
        String::from_utf8_lossy(&manifest)
            .lines()
            .find_map(|l| l.strip_prefix("Main-Class:").map(|c| c.trim().to_string()))
            .ok_or_else(|| Error::Invalid(format!("{} hat keine Main-Class", jar.display())))
    })
    .await
    .map_err(|e| Error::Invalid(format!("Forge-Installation abgebrochen: {e}")))?
}

fn display(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn zip_err(path: &Path) -> impl Fn(zip::result::ZipError) -> Error + '_ {
    move |source| Error::Zip {
        path: path.to_path_buf(),
        source,
    }
}

fn open_zip(path: &Path) -> Result<zip::ZipArchive<std::fs::File>> {
    let file = std::fs::File::open(path).map_err(Error::io(path))?;
    zip::ZipArchive::new(file).map_err(zip_err(path))
}

fn read_entry(
    zip: &mut zip::ZipArchive<std::fs::File>,
    path: &Path,
    name: &str,
) -> Result<Vec<u8>> {
    let mut entry = zip
        .by_name(name)
        .map_err(|_| Error::Invalid(format!("{name} fehlt im Forge-Installer")))?;
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut bytes).map_err(Error::io(path))?;
    Ok(bytes)
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
    }
    std::fs::write(path, bytes).map_err(Error::io(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neoforge_prefixes() {
        assert_eq!(neoforge_prefix("1.21.1"), "21.1.");
        assert_eq!(neoforge_prefix("1.20.4"), "20.4.");
        assert_eq!(neoforge_prefix("1.21"), "21.0.");
        assert_eq!(neoforge_prefix("26.1"), "26.1.");
    }

    #[test]
    fn metadata_and_urls() {
        let xml = "<metadata><versioning><versions><version>1.20.1-47.3.0</version>\n<version>1.12.2-14.23.5.2860</version></versions></versioning></metadata>";
        assert_eq!(
            maven_versions(xml),
            ["1.20.1-47.3.0", "1.12.2-14.23.5.2860"]
        );
        assert_eq!(
            installer_url(LoaderKind::Forge, "1.20.1", "47.3.0"),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.3.0/forge-1.20.1-47.3.0-installer.jar"
        );
        assert_eq!(
            installer_url(LoaderKind::NeoForge, "1.21.1", "21.1.77"),
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.77/neoforge-21.1.77-installer.jar"
        );
    }

    #[test]
    fn substitutes_arguments() {
        let libs = Path::new("/l");
        let data = HashMap::from([
            ("SIDE".to_string(), "client".to_string()),
            ("MAPPINGS".to_string(), "/x/m.txt".to_string()),
        ]);
        assert_eq!(substitute("{SIDE}", &data, libs).unwrap(), "client");
        assert_eq!(
            substitute("--side={SIDE}", &data, libs).unwrap(),
            "--side=client"
        );
        assert_eq!(
            substitute("[de.oceanlabs.mcp:mcp_config:1.20.1@zip]", &data, libs).unwrap(),
            display(&libs.join("de/oceanlabs/mcp/mcp_config/1.20.1/mcp_config-1.20.1.zip"))
        );
        assert!(substitute("{NOPE}", &data, libs).is_err());
    }

    #[test]
    fn legacy_urls() {
        let info = serde_json::json!({"libraries":[{"name":"a:b:1","url":"http://files.minecraftforge.net/maven/"},{"name":"c:d:1"}]});
        let fixed = legacy_fix_libraries(info);
        assert_eq!(fixed["libraries"][0]["url"], FORGE_MAVEN);
        assert!(fixed["libraries"][1].get("url").is_none());
    }
}
