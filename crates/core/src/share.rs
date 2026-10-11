//! Modpacks teilen: Links der Form `pinecone://…` erzeugen und lesen.
//!
//! Ein Link verweist nur auf die Original-Quelle (Modrinth- oder CurseForge-Version), es werden
//! keine Dateien übertragen. Eigene Änderungen am Profil teilt man als Datei (`.mrpack`-Export).
//! `pinecone://pack/<code>` ist für den späteren Share-Dienst reserviert.

use std::fmt;

use crate::modpack::PackSource;
use crate::{Error, Result};

pub const SCHEME: &str = "pinecone";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareLink {
    /// Projekt-ID oder Slug, optional eine bestimmte Version.
    Modrinth {
        project: String,
        version: Option<String>,
    },
    Curseforge {
        project: u64,
        file: Option<u64>,
    },
    /// TwoGG-Modpack vom Pinecone-Server.
    Twogg {
        pack: String,
        version: Option<String>,
    },
    /// Share-Code des (noch nicht vorhandenen) Share-Dienstes.
    Code(String),
}

impl ShareLink {
    /// Link auf genau die installierte Pack-Version; aus Dateien importierte Packs haben keinen.
    pub fn for_source(source: &PackSource) -> Option<Self> {
        match source {
            PackSource::Modrinth {
                project_id,
                version_id,
            } => Some(ShareLink::Modrinth {
                project: project_id.clone(),
                version: Some(version_id.clone()),
            }),
            PackSource::Curseforge {
                project_id,
                file_id,
            } => Some(ShareLink::Curseforge {
                project: *project_id,
                file: Some(*file_id),
            }),
            PackSource::Twogg {
                pack_id,
                version_id,
            } => Some(ShareLink::Twogg {
                pack: pack_id.clone(),
                version: Some(version_id.clone()),
            }),
            PackSource::File => None,
        }
    }

    /// Liest einen Pinecone-Link oder eine Modrinth-Modpack-Adresse
    /// (`https://modrinth.com/modpack/<slug>[/version/<id>]`).
    pub fn parse(input: &str) -> Result<Self> {
        let invalid = || Error::Invalid(format!("Kein gültiger Modpack-Link: {input}"));
        let input = input.trim();
        let url = reqwest::Url::parse(input).map_err(|_| invalid())?;
        let segments: Vec<&str> = url
            .path_segments()
            .map(|s| s.filter(|s| !s.is_empty()).collect())
            .unwrap_or_default();

        match (url.scheme(), url.host_str()) {
            (SCHEME, Some(host)) => match (host, segments.as_slice()) {
                ("modrinth", [project]) => Ok(ShareLink::Modrinth {
                    project: id(project).ok_or_else(invalid)?,
                    version: None,
                }),
                ("modrinth", [project, version]) => Ok(ShareLink::Modrinth {
                    project: id(project).ok_or_else(invalid)?,
                    version: Some(id(version).ok_or_else(invalid)?),
                }),
                ("curseforge", [project]) => Ok(ShareLink::Curseforge {
                    project: project.parse().map_err(|_| invalid())?,
                    file: None,
                }),
                ("curseforge", [project, file]) => Ok(ShareLink::Curseforge {
                    project: project.parse().map_err(|_| invalid())?,
                    file: Some(file.parse().map_err(|_| invalid())?),
                }),
                ("twogg", [pack]) => Ok(ShareLink::Twogg {
                    pack: id(pack).ok_or_else(invalid)?,
                    version: None,
                }),
                ("twogg", [pack, version]) => Ok(ShareLink::Twogg {
                    pack: id(pack).ok_or_else(invalid)?,
                    version: Some(id(version).ok_or_else(invalid)?),
                }),
                ("pack", [code]) => Ok(ShareLink::Code(id(code).ok_or_else(invalid)?)),
                _ => Err(invalid()),
            },
            ("https", Some("modrinth.com" | "www.modrinth.com")) => match segments.as_slice() {
                ["modpack", project] => Ok(ShareLink::Modrinth {
                    project: id(project).ok_or_else(invalid)?,
                    version: None,
                }),
                ["modpack", project, "version", version] => Ok(ShareLink::Modrinth {
                    project: id(project).ok_or_else(invalid)?,
                    version: Some(id(version).ok_or_else(invalid)?),
                }),
                _ => Err(invalid()),
            },
            _ => Err(invalid()),
        }
    }
}

impl fmt::Display for ShareLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShareLink::Modrinth { project, version } => {
                write!(f, "{SCHEME}://modrinth/{project}")?;
                if let Some(v) = version {
                    write!(f, "/{v}")?;
                }
                Ok(())
            }
            ShareLink::Curseforge { project, file } => {
                write!(f, "{SCHEME}://curseforge/{project}")?;
                if let Some(id) = file {
                    write!(f, "/{id}")?;
                }
                Ok(())
            }
            ShareLink::Twogg { pack, version } => {
                write!(f, "{SCHEME}://twogg/{pack}")?;
                if let Some(v) = version {
                    write!(f, "/{v}")?;
                }
                Ok(())
            }
            ShareLink::Code(code) => write!(f, "{SCHEME}://pack/{code}"),
        }
    }
}

/// Projekt-, Versions-IDs und Slugs: nur ASCII-Buchstaben, Ziffern, `-`, `_` und `.`.
fn id(s: &str) -> Option<String> {
    (!s.is_empty()
        && s.len() <= 64
        && s != "."
        && s != ".."
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')))
    .then(|| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for link in [
            "pinecone://modrinth/fabulously-optimized",
            "pinecone://modrinth/1KVo5zza/hY9wBqQs",
            "pinecone://curseforge/394468",
            "pinecone://curseforge/394468/5512345",
            "pinecone://pack/ab12cd",
            "pinecone://twogg/horizons-unbound",
            "pinecone://twogg/horizons-unbound/af1508aa3b19432f",
        ] {
            assert_eq!(ShareLink::parse(link).unwrap().to_string(), link);
        }
    }

    #[test]
    fn tolerates_trailing_slash_and_whitespace() {
        assert_eq!(
            ShareLink::parse("  pinecone://modrinth/sodium-pack/ \n").unwrap(),
            ShareLink::Modrinth {
                project: "sodium-pack".into(),
                version: None
            }
        );
    }

    #[test]
    fn modrinth_web_links() {
        assert_eq!(
            ShareLink::parse("https://modrinth.com/modpack/fabulously-optimized/version/6.0.0")
                .unwrap(),
            ShareLink::Modrinth {
                project: "fabulously-optimized".into(),
                version: Some("6.0.0".into())
            }
        );
        assert!(ShareLink::parse("https://modrinth.com/mod/sodium").is_err());
    }

    #[test]
    fn rejects_garbage() {
        for bad in [
            "",
            "modrinth:sodium",
            "pinecone://modrinth/",
            "pinecone://modrinth/a/b/c",
            "pinecone://modrinth/..",
            "pinecone://modrinth/a%2Fb",
            "pinecone://curseforge/abc",
            "pinecone://unbekannt/x",
            "http://modrinth.com/modpack/x",
            "https://example.com/modpack/x",
        ] {
            assert!(
                ShareLink::parse(bad).is_err(),
                "{bad:?} sollte ungültig sein"
            );
        }
    }

    #[test]
    fn link_for_source() {
        let src = PackSource::Curseforge {
            project_id: 1,
            file_id: 2,
        };
        assert_eq!(
            ShareLink::for_source(&src).unwrap().to_string(),
            "pinecone://curseforge/1/2"
        );
        assert_eq!(ShareLink::for_source(&PackSource::File), None);
    }
}
