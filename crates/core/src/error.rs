use std::path::PathBuf;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP-Fehler bei {url}: {source}")]
    Http {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Prüfsumme falsch für {path}: erwartet {expected}, erhalten {actual}")]
    HashMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error("Ein-/Ausgabefehler bei {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Ungültiges JSON in {context}: {source}")]
    Json {
        context: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("ZIP-Fehler in {path}: {source}")]
    Zip {
        path: PathBuf,
        #[source]
        source: zip::result::ZipError,
    },

    #[error("Unbekannte Minecraft-Version: {0}")]
    UnknownVersion(String),

    #[error("Version {0} ist nicht installiert")]
    NotInstalled(String),

    #[error("Ungültiger Spielername {0:?} (1–16 Zeichen, nur A–Z, a–z, 0–9 und _)")]
    InvalidPlayerName(String),

    #[error("{0}")]
    Auth(String),

    #[error("Schlüsselbund nicht verfügbar: {0}")]
    Keyring(String),

    #[error("Kein passendes Java {major} gefunden: {reason}")]
    JavaNotFound { major: u32, reason: String },

    #[error("{0}")]
    Invalid(String),
}

impl Error {
    pub(crate) fn io(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> Error {
        let path = path.into();
        move |source| Error::Io { path, source }
    }

    pub(crate) fn json(context: impl Into<String>) -> impl FnOnce(serde_json::Error) -> Error {
        let context = context.into();
        move |source| Error::Json { context, source }
    }
}
