//! Java-Management: installierte Laufzeiten finden und bei Bedarf Mojangs Runtime laden.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use sha1::{Digest, Sha1};

use crate::download::{DownloadTask, Downloader, Progress};
use crate::manifest::JavaVersion;
use crate::paths::{Paths, safe_relative};
use crate::rules::Environment;
use crate::{Error, Result};

pub const JAVA_RUNTIMES_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

/// Für Versionen ohne `javaVersion` (sehr alte JSONs) gilt Java 8.
pub const DEFAULT_JAVA: JavaVersion = JavaVersion {
    component: None,
    major_version: 8,
};
const LEGACY_COMPONENT: &str = "jre-legacy";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaInstallation {
    /// Pfad zum `java`-Programm.
    pub path: PathBuf,
    /// Vollständige Version, z. B. `1.8.0_504` oder `25.0.4.1`.
    pub version: String,
    pub major: u32,
    /// Von Pinecone unter `<data>/runtimes` installiert.
    pub managed: bool,
}

#[derive(Debug, Deserialize)]
struct RuntimeEntry {
    manifest: ManifestRef,
    version: RuntimeVersion,
}

#[derive(Debug, Deserialize)]
struct ManifestRef {
    sha1: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeVersion {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeManifest {
    files: HashMap<String, RuntimeFile>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RuntimeFile {
    File {
        downloads: RuntimeDownloads,
        #[serde(default)]
        executable: bool,
    },
    Directory,
    Link {
        target: String,
    },
}

#[derive(Debug, Deserialize)]
struct RuntimeDownloads {
    raw: RawDownload,
}

#[derive(Debug, Deserialize)]
struct RawDownload {
    sha1: String,
    size: u64,
    url: String,
}

/// Plattform-Schlüssel in Mojangs Runtime-Index.
pub fn mojang_platform(env: &Environment) -> Option<&'static str> {
    Some(match (env.os_name.as_str(), env.arch.as_str()) {
        ("linux", "x86_64") => "linux",
        ("linux", "x86") => "linux-i386",
        ("windows", "x86_64") => "windows-x64",
        ("windows", "x86") => "windows-x86",
        ("windows", "arm64") => "windows-arm64",
        ("osx", "x86_64") => "mac-os",
        ("osx", "arm64") => "mac-os-arm64",
        _ => return None,
    })
}

/// Liest Version und Major-Version aus der Ausgabe von `java -version`.
pub fn parse_java_version(output: &str) -> Option<(String, u32)> {
    let start = output.find("version \"")? + "version \"".len();
    let version = &output[start..start + output[start..].find('"')?];
    let mut parts = version.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    let major = if first == 1 {
        parts.next()?.parse().ok()?
    } else {
        first
    };
    Some((version.to_string(), major))
}

fn java_exe() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

/// Mögliche Java-Programme in einem JDK/JRE-Verzeichnis.
fn binaries_in(home: &Path) -> [PathBuf; 3] {
    [
        home.join("bin").join(java_exe()),
        home.join("jre").join("bin").join(java_exe()),
        home.join("Contents")
            .join("Home")
            .join("bin")
            .join(java_exe()),
    ]
}

/// Prüft ein Java-Programm per `java -version`.
pub async fn probe(path: &Path) -> Option<JavaInstallation> {
    let mut cmd = tokio::process::Command::new(path);
    cmd.arg("-version")
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let output = tokio::time::timeout(Duration::from_secs(10), cmd.output())
        .await
        .ok()?
        .ok()?;
    // `java -version` schreibt nach stderr.
    let text = String::from_utf8_lossy(&output.stderr).into_owned()
        + &String::from_utf8_lossy(&output.stdout);
    let (version, major) = parse_java_version(&text)?;
    Some(JavaInstallation {
        path: path.to_path_buf(),
        version,
        major,
        managed: false,
    })
}

pub struct JavaManager {
    paths: Paths,
    downloader: Downloader,
    env: Environment,
}

impl JavaManager {
    pub fn new(paths: Paths, downloader: Downloader, env: Environment) -> Self {
        Self {
            paths,
            downloader,
            env,
        }
    }

    /// Ausführbares Java einer von Pinecone verwalteten Runtime.
    pub fn runtime_java(&self, component: &str) -> PathBuf {
        let dir = self.paths.runtime_dir(component);
        if self.env.os_name == "osx" {
            dir.join("jre.bundle/Contents/Home/bin/java")
        } else {
            dir.join("bin").join(java_exe())
        }
    }

    fn candidates(&self) -> Vec<PathBuf> {
        let mut homes: Vec<PathBuf> = Vec::new();
        let mut direct: Vec<PathBuf> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(self.paths.runtimes()) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                direct.push(self.runtime_java(&name));
            }
        }
        if let Some(home) = std::env::var_os("JAVA_HOME") {
            homes.push(home.into());
        }
        if let Some(path) = std::env::var_os("PATH") {
            direct.extend(std::env::split_paths(&path).map(|p| p.join(java_exe())));
        }

        let mut roots: Vec<PathBuf> = Vec::new();
        match self.env.os_name.as_str() {
            "windows" => {
                for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
                    if let Some(pf) = std::env::var_os(var) {
                        let pf = PathBuf::from(pf);
                        for vendor in [
                            "Java",
                            "Eclipse Adoptium",
                            "Microsoft",
                            "Zulu",
                            "BellSoft",
                            "Amazon Corretto",
                            "Semeru",
                        ] {
                            roots.push(pf.join(vendor));
                        }
                    }
                }
            }
            "osx" => roots.push("/Library/Java/JavaVirtualMachines".into()),
            _ => roots
                .extend(["/usr/lib/jvm", "/usr/lib64/jvm", "/usr/java", "/opt"].map(PathBuf::from)),
        }
        if let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()) {
            roots.push(home.join(".sdkman/candidates/java"));
            roots.push(home.join(".jdks"));
        }
        for root in roots {
            if let Ok(entries) = std::fs::read_dir(&root) {
                homes.extend(entries.flatten().map(|e| e.path()));
            }
        }
        direct.extend(homes.iter().flat_map(|h| binaries_in(h)));

        // Symlinks wie /usr/bin/java → /usr/lib/jvm/… nur einmal prüfen.
        let mut seen = HashSet::new();
        direct
            .into_iter()
            .filter(|p| p.is_file())
            .filter(|p| seen.insert(std::fs::canonicalize(p).unwrap_or_else(|_| p.clone())))
            .collect()
    }

    /// Alle gefundenen Java-Installationen, verwaltete zuerst.
    pub async fn installed(&self) -> Vec<JavaInstallation> {
        let runtimes = self.paths.runtimes();
        let probes = self.candidates().into_iter().map(|p| {
            let managed = p.starts_with(&runtimes);
            async move { probe(&p).await.map(|j| JavaInstallation { managed, ..j }) }
        });
        let mut found: Vec<JavaInstallation> = futures::future::join_all(probes)
            .await
            .into_iter()
            .flatten()
            .collect();
        found.sort_by_key(|j| (!j.managed, j.major));
        found
    }

    /// Passendes Java für eine Version: exakte Major-Version (verwaltet bevorzugt), sonst
    /// Mojangs Runtime laden. Gibt es für die Plattform keine, wird die nächstneuere genommen.
    pub async fn ensure(
        &self,
        wanted: &JavaVersion,
        on_progress: impl FnMut(Progress),
    ) -> Result<JavaInstallation> {
        let installed = self.installed().await;
        if let Some(java) = installed.iter().find(|j| j.major == wanted.major_version) {
            return Ok(java.clone());
        }
        let component = wanted.component.as_deref().unwrap_or(LEGACY_COMPONENT);
        match self.install_runtime(component, on_progress).await {
            Ok(java) => Ok(java),
            Err(e) => {
                let newer = installed
                    .into_iter()
                    .filter(|j| j.major > wanted.major_version && wanted.major_version > 8)
                    .min_by_key(|j| j.major);
                match newer {
                    Some(java) => {
                        tracing::warn!(
                            "Java {} nicht verfügbar ({e}), nutze Java {}",
                            wanted.major_version,
                            java.major
                        );
                        Ok(java)
                    }
                    None => Err(Error::JavaNotFound {
                        major: wanted.major_version,
                        reason: e.to_string(),
                    }),
                }
            }
        }
    }

    /// Lädt eine Mojang-Runtime (z. B. `jre-legacy`, `java-runtime-epsilon`) nach `<data>/runtimes`.
    pub async fn install_runtime(
        &self,
        component: &str,
        on_progress: impl FnMut(Progress),
    ) -> Result<JavaInstallation> {
        let platform = mojang_platform(&self.env).ok_or_else(|| {
            Error::Invalid(format!(
                "Keine Mojang-Runtime für {} {}",
                self.env.os_name, self.env.arch
            ))
        })?;
        let index_bytes = self.downloader.fetch_bytes(JAVA_RUNTIMES_URL).await?;
        let mut index: HashMap<String, HashMap<String, Vec<RuntimeEntry>>> =
            serde_json::from_slice(&index_bytes).map_err(Error::json(JAVA_RUNTIMES_URL))?;
        let entry = index
            .get_mut(platform)
            .and_then(|p| p.remove(component))
            .and_then(|mut v| (!v.is_empty()).then(|| v.remove(0)))
            .ok_or_else(|| {
                Error::Invalid(format!("Mojang bietet {component} nicht für {platform} an"))
            })?;

        let manifest_bytes = self.downloader.fetch_bytes(&entry.manifest.url).await?;
        let actual = format!("{:x}", Sha1::digest(&manifest_bytes));
        if !actual.eq_ignore_ascii_case(&entry.manifest.sha1) {
            return Err(Error::HashMismatch {
                path: entry.manifest.url.into(),
                expected: entry.manifest.sha1,
                actual,
            });
        }
        let manifest: RuntimeManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(Error::json(format!("Runtime-Manifest {component}")))?;

        let root = self.paths.runtime_dir(component);
        let mut tasks = Vec::new();
        let mut executables = Vec::new();
        let mut links = Vec::new();
        for (name, file) in manifest.files {
            let rel = safe_relative(&name)?;
            let dest = root.join(&rel);
            match file {
                RuntimeFile::Directory => {
                    std::fs::create_dir_all(&dest).map_err(Error::io(&dest))?
                }
                RuntimeFile::File {
                    downloads,
                    executable,
                } => {
                    if executable {
                        executables.push(dest.clone());
                    }
                    tasks.push(DownloadTask {
                        url: downloads.raw.url,
                        dest,
                        sha1: Some(downloads.raw.sha1),
                        size: Some(downloads.raw.size),
                    });
                }
                RuntimeFile::Link { target } => {
                    let resolved = normalize(&rel.parent().unwrap_or(Path::new("")).join(&target))
                        .ok_or_else(|| {
                            Error::Invalid(format!("Link {name} zeigt aus der Runtime heraus"))
                        })?;
                    links.push((dest, PathBuf::from(target), resolved));
                }
            }
        }

        self.downloader.download_all(tasks, on_progress).await?;
        for path in &executables {
            set_executable(path)?;
        }
        for (link, target, resolved) in links {
            create_link(&link, &target, &root.join(resolved))?;
        }
        let marker = root.join(".pinecone-version");
        std::fs::write(&marker, &entry.version.name).map_err(Error::io(&marker))?;

        let java = self.runtime_java(component);
        let mut found = probe(&java).await.ok_or_else(|| {
            Error::Invalid(format!(
                "Runtime {component} installiert, aber {} startet nicht",
                java.display()
            ))
        })?;
        found.managed = true;
        Ok(found)
    }
}

/// Löst `..` lexikalisch auf; `None`, wenn der Pfad das Wurzelverzeichnis verlässt.
fn normalize(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(out)
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)
        .map_err(Error::io(path))?
        .permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms).map_err(Error::io(path))
}

#[cfg(not(unix))]
fn set_executable(_: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn create_link(link: &Path, target: &Path, _resolved: &Path) -> Result<()> {
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
    }
    let _ = std::fs::remove_file(link);
    std::os::unix::fs::symlink(target, link).map_err(Error::io(link))
}

/// Windows: Symlinks brauchen Rechte, daher Kopie des Ziels (in Mojangs Windows-Runtimes selten).
#[cfg(not(unix))]
fn create_link(link: &Path, _target: &Path, resolved: &Path) -> Result<()> {
    if resolved.is_file() {
        std::fs::copy(resolved, link).map_err(Error::io(link))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_java_versions() {
        let j8 = "openjdk version \"1.8.0_504\"\nOpenJDK Runtime Environment (build 1.8.0_504-b01)";
        assert_eq!(parse_java_version(j8), Some(("1.8.0_504".into(), 8)));
        let j25 = "openjdk version \"25.0.4.1\" 2026-08-18\nOpenJDK Runtime Environment";
        assert_eq!(parse_java_version(j25), Some(("25.0.4.1".into(), 25)));
        let j21 = "java version \"21\" 2023-09-19 LTS";
        assert_eq!(parse_java_version(j21), Some(("21".into(), 21)));
        let ea = "openjdk version \"26-ea\" 2026-03-17";
        assert_eq!(parse_java_version(ea), Some(("26-ea".into(), 26)));
        assert_eq!(parse_java_version("kein java"), None);
    }

    #[test]
    fn platforms() {
        let env = |os: &str, arch: &str| Environment {
            os_name: os.into(),
            arch: arch.into(),
            os_version: String::new(),
            features: HashMap::new(),
        };
        assert_eq!(mojang_platform(&env("linux", "x86_64")), Some("linux"));
        assert_eq!(
            mojang_platform(&env("windows", "x86_64")),
            Some("windows-x64")
        );
        assert_eq!(mojang_platform(&env("osx", "arm64")), Some("mac-os-arm64"));
        assert_eq!(mojang_platform(&env("linux", "arm64")), None);
    }

    #[test]
    fn normalizes_link_targets() {
        assert_eq!(
            normalize(Path::new("lib/amd64/server/../libjsig.so")),
            Some("lib/amd64/libjsig.so".into())
        );
        assert_eq!(normalize(Path::new("bin/../../draussen")), None);
    }

    #[test]
    fn manifest_entries() {
        let m: RuntimeManifest = serde_json::from_str(
            r#"{"files":{
                "bin":{"type":"directory"},
                "bin/java":{"type":"file","executable":true,"downloads":{"raw":{"sha1":"ab","size":1,"url":"u"}}},
                "bin/ControlPanel":{"type":"link","target":"jcontrol"}}}"#,
        )
        .unwrap();
        assert!(matches!(
            m.files["bin/java"],
            RuntimeFile::File {
                executable: true,
                ..
            }
        ));
        assert!(
            matches!(&m.files["bin/ControlPanel"], RuntimeFile::Link { target } if target == "jcontrol")
        );
        assert!(matches!(m.files["bin"], RuntimeFile::Directory));
    }
}
