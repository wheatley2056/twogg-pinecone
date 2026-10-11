//! Mojang-Versionsmanifest und Version-JSON (inkl. `inheritsFrom`-Merge für Loader).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::rules::{self, Environment, Rule};

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const MOJANG_LIBRARIES_URL: &str = "https://libraries.minecraft.net/";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub sha1: String,
    pub release_time: String,
}

impl VersionManifest {
    /// Sucht eine Version; `latest` und `latest-snapshot` werden aufgelöst.
    pub fn find(&self, id: &str) -> Option<&VersionEntry> {
        let id = match id {
            "latest" | "release" => self.latest.release.as_str(),
            "latest-snapshot" | "snapshot" => self.latest.snapshot.as_str(),
            other => other,
        };
        self.versions.iter().find(|v| v.id == id)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub main_class: Option<String>,
    /// Ab 1.13.
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// Bis 1.12: alle Spielargumente in einem String.
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub downloads: Option<VersionDownloads>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub java_version: Option<JavaVersion>,
    /// Version, deren Client-JAR genutzt wird (bei Loadern die Basisversion). Wird beim Merge gesetzt.
    #[serde(skip)]
    pub jar: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional { rules: Vec<Rule>, value: ArgValue },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    One(String),
    Many(Vec<String>),
}

impl Argument {
    /// Liefert die Argumente, die in dieser Umgebung gelten.
    pub fn resolve(&self, env: &Environment) -> Vec<String> {
        match self {
            Argument::Plain(s) => vec![s.clone()],
            Argument::Conditional { rules, value } if rules::allowed(rules, env) => match value {
                ArgValue::One(s) => vec![s.clone()],
                ArgValue::Many(v) => v.clone(),
            },
            Argument::Conditional { .. } => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionDownloads {
    pub client: Option<Artifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: Option<String>,
    pub major_version: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    /// Maven-Koordinate `group:artifact:version[:classifier][@ext]`.
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// Alte Natives: OS-Name → Klassifizierer, z. B. `natives-windows-${arch}`.
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    /// Maven-Repository für Libraries ohne `downloads` (Fabric, Quilt).
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Artifact>>,
}

/// Eine Datei, die konkret heruntergeladen werden muss, mit relativem Pfad unter `libraries/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedArtifact {
    pub path: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

/// Wandelt `group:artifact:version[:classifier][@ext]` in einen Maven-Pfad um.
pub fn maven_path(coordinate: &str) -> Option<String> {
    let (coordinate, ext) = coordinate.split_once('@').unwrap_or((coordinate, "jar"));
    let parts: Vec<&str> = coordinate.split(':').collect();
    let (group, artifact, version, classifier) = match parts.as_slice() {
        [g, a, v] => (*g, *a, *v, None),
        [g, a, v, c] => (*g, *a, *v, Some(*c)),
        _ => return None,
    };
    let file = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.{ext}"),
        None => format!("{artifact}-{version}.{ext}"),
    };
    Some(format!(
        "{}/{artifact}/{version}/{file}",
        group.replace('.', "/")
    ))
}

impl Library {
    pub fn applies(&self, env: &Environment) -> bool {
        rules::allowed(&self.rules, env)
    }

    /// `group:artifact[:classifier]`, um doppelte Libraries beim Merge zu erkennen.
    fn identity(&self) -> String {
        let base = self.name.split('@').next().unwrap_or(&self.name);
        let parts: Vec<&str> = base.split(':').collect();
        match parts.as_slice() {
            [g, a, _, c] => format!("{g}:{a}:{c}"),
            [g, a, ..] => format!("{g}:{a}"),
            _ => base.to_string(),
        }
    }

    /// Neue LWJGL-Natives (ab 1.19) stecken als eigene Library mit Klassifizierer `natives-*`.
    pub fn is_natives_jar(&self) -> bool {
        self.name
            .split('@')
            .next()
            .and_then(|n| n.split(':').nth(3))
            .is_some_and(|c| c.starts_with("natives-"))
    }

    /// Die Haupt-JAR der Library (für Classpath bzw. Natives-Entpacken).
    pub fn artifact(&self) -> Option<ResolvedArtifact> {
        if let Some(downloads) = &self.downloads {
            let a = downloads.artifact.as_ref()?;
            let path = a.path.clone().or_else(|| maven_path(&self.name))?;
            return Some(ResolvedArtifact {
                path,
                url: a.url.clone(),
                sha1: a.sha1.clone(),
                size: a.size,
            });
        }
        // Reine Natives-Library alten Stils ohne Haupt-JAR.
        if self.natives.is_some() {
            return None;
        }
        let path = maven_path(&self.name)?;
        let mut base = self
            .url
            .clone()
            .unwrap_or_else(|| MOJANG_LIBRARIES_URL.into());
        if !base.ends_with('/') {
            base.push('/');
        }
        Some(ResolvedArtifact {
            url: format!("{base}{path}"),
            path,
            sha1: self.sha1.clone(),
            size: self.size,
        })
    }

    /// Natives alten Stils (bis 1.18): Klassifizierer passend zum OS.
    pub fn native_artifact(&self, env: &Environment) -> Option<ResolvedArtifact> {
        let classifier = self
            .natives
            .as_ref()?
            .get(&env.os_name)?
            .replace("${arch}", env.arch_bits());
        let a = self
            .downloads
            .as_ref()?
            .classifiers
            .as_ref()?
            .get(&classifier)?;
        let path = a
            .path
            .clone()
            .or_else(|| maven_path(&format!("{}:{classifier}", self.name)))?;
        Some(ResolvedArtifact {
            path,
            url: a.url.clone(),
            sha1: a.sha1.clone(),
            size: a.size,
        })
    }
}

impl VersionJson {
    pub fn jar_id(&self) -> &str {
        self.jar.as_deref().unwrap_or(&self.id)
    }

    /// Führt eine Loader-Version (`inheritsFrom`) mit ihrer Basisversion zusammen.
    /// Felder des Kindes gewinnen, Libraries des Kindes stehen vorne und ersetzen gleichnamige.
    pub fn merge_onto(self, parent: VersionJson) -> VersionJson {
        let jar = Some(parent.jar_id().to_string());
        let child_ids: HashSet<String> = self.libraries.iter().map(Library::identity).collect();
        let mut libraries = self.libraries;
        libraries.extend(
            parent
                .libraries
                .into_iter()
                .filter(|l| !child_ids.contains(&l.identity())),
        );

        let arguments = match (parent.arguments, self.arguments) {
            (Some(mut p), Some(c)) => {
                p.game.extend(c.game);
                p.jvm.extend(c.jvm);
                Some(p)
            }
            (p, c) => c.or(p),
        };

        VersionJson {
            id: self.id,
            inherits_from: None,
            kind: self.kind.or(parent.kind),
            main_class: self.main_class.or(parent.main_class),
            arguments,
            minecraft_arguments: self.minecraft_arguments.or(parent.minecraft_arguments),
            asset_index: self.asset_index.or(parent.asset_index),
            assets: self.assets.or(parent.assets),
            downloads: self.downloads.or(parent.downloads),
            libraries,
            java_version: self.java_version.or(parent.java_version),
            jar,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linux() -> Environment {
        Environment {
            os_name: "linux".into(),
            arch: "x86_64".into(),
            os_version: String::new(),
            features: HashMap::new(),
        }
    }

    #[test]
    fn maven_paths() {
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3").unwrap(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar"
        );
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3:natives-linux").unwrap(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar"
        );
        assert_eq!(
            maven_path("net.fabricmc:intermediary:1.21.1@zip").unwrap(),
            "net/fabricmc/intermediary/1.21.1/intermediary-1.21.1.zip"
        );
        assert_eq!(maven_path("kaputt"), None);
    }

    #[test]
    fn manifest_find_latest() {
        let m: VersionManifest = serde_json::from_str(
            r#"{"latest":{"release":"1.21.1","snapshot":"24w40a"},"versions":[
                {"id":"24w40a","type":"snapshot","url":"u1","sha1":"a","releaseTime":"t"},
                {"id":"1.21.1","type":"release","url":"u2","sha1":"b","releaseTime":"t"}]}"#,
        )
        .unwrap();
        assert_eq!(m.find("latest").unwrap().id, "1.21.1");
        assert_eq!(m.find("latest-snapshot").unwrap().id, "24w40a");
        assert_eq!(m.find("1.21.1").unwrap().url, "u2");
        assert!(m.find("0.0").is_none());
    }

    #[test]
    fn modern_arguments() {
        let v: VersionJson =
            serde_json::from_str(include_str!("../tests/fixtures/modern.json")).unwrap();
        let args = v.arguments.as_ref().unwrap();
        let game: Vec<String> = args.game.iter().flat_map(|a| a.resolve(&linux())).collect();
        assert_eq!(game[..2], ["--username", "${auth_player_name}"]);
        assert!(!game.contains(&"--demo".to_string()));
        let jvm: Vec<String> = args.jvm.iter().flat_map(|a| a.resolve(&linux())).collect();
        assert!(jvm.contains(&"-Djava.library.path=${natives_directory}".to_string()));
        assert!(!jvm.iter().any(|a| a.starts_with("-XstartOnFirstThread")));
        assert_eq!(v.java_version.unwrap().major_version, 21);
    }

    #[test]
    fn modern_natives_libraries() {
        let v: VersionJson =
            serde_json::from_str(include_str!("../tests/fixtures/modern.json")).unwrap();
        let env = linux();
        let applicable: Vec<&Library> = v.libraries.iter().filter(|l| l.applies(&env)).collect();
        let names: Vec<&str> = applicable.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"org.lwjgl:lwjgl:3.3.3"));
        assert!(names.contains(&"org.lwjgl:lwjgl:3.3.3:natives-linux"));
        assert!(!names.contains(&"org.lwjgl:lwjgl:3.3.3:natives-windows"));
        assert!(applicable.iter().any(|l| l.is_natives_jar()));
    }

    #[test]
    fn legacy_natives_classifier() {
        let v: VersionJson =
            serde_json::from_str(include_str!("../tests/fixtures/legacy.json")).unwrap();
        assert!(v.arguments.is_none());
        assert!(
            v.minecraft_arguments
                .as_deref()
                .unwrap()
                .contains("${auth_player_name}")
        );

        let platform = v
            .libraries
            .iter()
            .find(|l| l.name.contains("lwjgl-platform"))
            .unwrap();
        assert!(platform.artifact().is_none());
        let native = platform.native_artifact(&linux()).unwrap();
        assert!(native.path.ends_with("natives-linux.jar"));

        let mut win = linux();
        win.os_name = "windows".into();
        let native = platform.native_artifact(&win).unwrap();
        assert!(
            native.path.ends_with("natives-windows-64.jar"),
            "{}",
            native.path
        );
    }

    #[test]
    fn fabric_merge() {
        let parent: VersionJson =
            serde_json::from_str(include_str!("../tests/fixtures/modern.json")).unwrap();
        let child: VersionJson =
            serde_json::from_str(include_str!("../tests/fixtures/fabric.json")).unwrap();
        assert_eq!(child.inherits_from.as_deref(), Some("1.21.1"));

        let merged = child.merge_onto(parent);
        assert_eq!(merged.id, "fabric-loader-0.16.5-1.21.1");
        assert_eq!(merged.jar_id(), "1.21.1");
        assert_eq!(
            merged.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );
        assert_eq!(merged.asset_index.unwrap().id, "17");
        assert!(merged.downloads.unwrap().client.is_some());
        assert_eq!(merged.libraries[0].name, "org.ow2.asm:asm:9.7.1");
        // Die ältere ASM-Version der Basis wird ersetzt.
        assert_eq!(
            merged
                .libraries
                .iter()
                .filter(|l| l.name.starts_with("org.ow2.asm:asm:"))
                .count(),
            1
        );
        let jvm = merged.arguments.unwrap().jvm;
        assert!(
            matches!(jvm.last(), Some(Argument::Plain(a)) if a == "-DFabricMcEmu= net.minecraft.client.main.Main ")
        );

        let fabric_lib = merged
            .libraries
            .iter()
            .find(|l| l.name.starts_with("net.fabricmc:fabric-loader"))
            .unwrap();
        let art = fabric_lib.artifact().unwrap();
        assert_eq!(
            art.url,
            "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.16.5/fabric-loader-0.16.5.jar"
        );
    }
}
