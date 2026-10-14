//! Signierte Event-Pack-Manifeste (F-10, Plan `04` §6).
//!
//! Ein Manifest beschreibt eine Version eines TwoGG-Event-Packs: Event, Minecraft und Loader,
//! Prüfsumme des `.mrpack` und die Pflichtdateien (`locked`), die nach der Installation und vor
//! jedem Start unverändert sein müssen. Signiert wird mit Ed25519 über die kanonische JSON-Form
//! (RFC 8785, JCS). Die öffentlichen Schlüssel sind in den Launcher kompiliert, damit die Echtheit
//! nicht davon abhängt, dass der Pinecone-Server sicher ist.
//!
//! Schlüssel: `PINECONE_EVENTPACK_KEYS` beim Bauen, Format `keyId=<Base64 des öffentlichen
//! Schlüssels>`, mehrere durch `;` getrennt. In Debug-Builds gilt dieselbe Variable auch zur
//! Laufzeit (für Tests und Entwicklung).

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha512};

use crate::loader::LoaderSpec;
use crate::{Error, Result};

/// Formatversion; unbekannte Hauptversionen lehnt Pinecone ab.
pub const SCHEMA: &str = "pinecone-eventpack/1";

/// Meldung bei fehlender oder falscher Signatur (AK-10.1).
pub const NOT_SIGNED: &str = "Dieses Pack ist nicht von TwoGG signiert";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema: String,
    pub pack_id: String,
    pub version: String,
    pub channel: String,
    pub event_id: String,
    pub title: String,
    pub minecraft: String,
    pub loader: Option<LoaderSpec>,
    pub min_launcher_version: String,
    pub required: bool,
    pub allow_extra_mods: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_memory_mb: Option<u32>,
    pub mrpack: MrpackRef,
    pub locked: Vec<LockedFile>,
    pub published_at: String,
    pub key_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrpackRef {
    pub url: String,
    pub size: u64,
    pub sha512: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockedFile {
    pub path: String,
    pub sha512: String,
}

/// Manifest mit Signatur, wie es der Pinecone-Server ausliefert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedManifest {
    pub manifest: Value,
    pub signature: String,
}

// ---------- Kanonisches JSON (RFC 8785) ----------

/// Kanonische Form nach RFC 8785 für die Werte, die in Manifesten vorkommen: Objekte mit nach
/// UTF-16-Codeeinheiten sortierten Schlüsseln, keine Leerzeichen, ganze Zahlen. Gleitkommazahlen
/// lehnt die Funktion ab, statt sie womöglich anders als die Gegenseite zu formatieren.
pub fn canonical_json(value: &Value) -> Result<String> {
    let mut out = String::new();
    write_canonical(value, &mut out)?;
    Ok(out)
}

fn write_canonical(value: &Value, out: &mut String) -> Result<()> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if n.is_f64() {
                return Err(Error::Invalid(
                    "Manifest enthält eine Kommazahl; erlaubt sind nur ganze Zahlen".into(),
                ));
            }
            out.push_str(&n.to_string());
        }
        // serde_json maskiert wie RFC 8785: nur `"`, `\` und Steuerzeichen (als \uXXXX
        // klein geschrieben, bzw. \b \f \n \r \t), alles andere als UTF-8.
        Value::String(s) => {
            out.push_str(&serde_json::to_string(s).map_err(Error::json("Manifest"))?)
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            out.push('{');
            for (i, key) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).map_err(Error::json("Manifest"))?);
                out.push(':');
                write_canonical(&map[key], out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

// ---------- Schlüssel ----------

/// Öffentliche Schlüssel, gegen die Manifeste geprüft werden.
#[derive(Debug, Clone, Default)]
pub struct Keyring {
    keys: BTreeMap<String, VerifyingKey>,
}

impl Keyring {
    /// Liest `keyId=<Base64>` (mehrere durch `;` getrennt). Ungültige Einträge sind ein Fehler.
    pub fn parse(spec: &str) -> Result<Self> {
        let mut keys = BTreeMap::new();
        for part in spec.split(';').map(str::trim).filter(|p| !p.is_empty()) {
            let (id, b64) = part.split_once('=').ok_or_else(|| {
                Error::Invalid(format!(
                    "Schlüssel ohne keyId: {part:?} (erwartet keyId=Base64)"
                ))
            })?;
            keys.insert(id.trim().to_string(), verifying_key(b64.trim())?);
        }
        Ok(Self { keys })
    }

    /// Die eingebauten Schlüssel (siehe Moduldoku).
    pub fn builtin() -> Self {
        let mut spec = option_env!("PINECONE_EVENTPACK_KEYS")
            .unwrap_or_default()
            .to_string();
        if cfg!(debug_assertions)
            && let Ok(extra) = std::env::var("PINECONE_EVENTPACK_KEYS")
        {
            spec.push(';');
            spec.push_str(&extra);
        }
        Self::parse(&spec).unwrap_or_else(|e| {
            tracing::error!("PINECONE_EVENTPACK_KEYS ungültig: {e}");
            Self::default()
        })
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Prüft Signatur und Schema und liefert das Manifest (Schritte 1 und 2 aus `04` §6.2).
    pub fn verify(&self, signed: &SignedManifest) -> Result<Manifest> {
        let key_id = signed
            .manifest
            .get("keyId")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Invalid(NOT_SIGNED.into()))?;
        let key = self.keys.get(key_id).ok_or_else(|| {
            Error::Invalid(format!("{NOT_SIGNED} (unbekannter Schlüssel {key_id})"))
        })?;
        let bytes = B64
            .decode(signed.signature.trim())
            .map_err(|_| Error::Invalid(NOT_SIGNED.into()))?;
        let signature =
            Signature::from_slice(&bytes).map_err(|_| Error::Invalid(NOT_SIGNED.into()))?;
        let canonical = canonical_json(&signed.manifest)?;
        key.verify_strict(canonical.as_bytes(), &signature)
            .map_err(|_| Error::Invalid(NOT_SIGNED.into()))?;
        let schema = signed
            .manifest
            .get("schema")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if schema.split('/').next() != SCHEMA.split('/').next()
            || schema.rsplit('/').next() != SCHEMA.rsplit('/').next()
        {
            return Err(Error::Invalid(format!(
                "Unbekanntes Event-Pack-Format {schema:?}; bitte aktualisiere Pinecone"
            )));
        }
        serde_json::from_value(signed.manifest.clone()).map_err(Error::json("Event-Pack-Manifest"))
    }
}

fn verifying_key(b64: &str) -> Result<VerifyingKey> {
    let bytes: [u8; 32] = B64
        .decode(b64)
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| {
            Error::Invalid("Öffentlicher Schlüssel muss 32 Byte in Base64 sein".into())
        })?;
    VerifyingKey::from_bytes(&bytes)
        .map_err(|e| Error::Invalid(format!("Öffentlicher Schlüssel: {e}")))
}

/// Neues Schlüsselpaar: (privater Schlüssel, öffentlicher Schlüssel), beide Base64.
pub fn generate_key() -> (String, String) {
    let key = SigningKey::generate(&mut rand_core::OsRng);
    (
        B64.encode(key.to_bytes()),
        B64.encode(key.verifying_key().to_bytes()),
    )
}

/// Liest einen privaten Schlüssel (32 Byte Base64, Leerzeichen und Zeilenumbrüche erlaubt).
pub fn signing_key(b64: &str) -> Result<SigningKey> {
    let bytes: [u8; 32] = B64
        .decode(b64.trim())
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| Error::Invalid("Privater Schlüssel muss 32 Byte in Base64 sein".into()))?;
    Ok(SigningKey::from_bytes(&bytes))
}

/// Öffentlicher Schlüssel zu einem privaten, Base64.
pub fn public_key(key: &SigningKey) -> String {
    B64.encode(key.verifying_key().to_bytes())
}

/// Signiert ein Manifest; liefert es zusammen mit der Signatur.
pub fn sign(manifest: &Manifest, key: &SigningKey) -> Result<SignedManifest> {
    let value = serde_json::to_value(manifest).map_err(Error::json("Event-Pack-Manifest"))?;
    let canonical = canonical_json(&value)?;
    let signature = key.sign(canonical.as_bytes());
    Ok(SignedManifest {
        manifest: value,
        signature: B64.encode(signature.to_bytes()),
    })
}

// ---------- Prüfsummen und Erzeugen ----------

pub fn sha512_hex(data: &[u8]) -> String {
    hex(&Sha512::digest(data))
}

/// SHA-512 einer Datei, gestreamt.
pub fn sha512_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path).map_err(Error::io(path))?;
    let mut hasher = Sha512::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(Error::io(path))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Angaben für ein neues Manifest, die nicht im `.mrpack` stehen.
#[derive(Debug, Clone)]
pub struct ManifestOptions {
    pub pack_id: String,
    pub event_id: String,
    pub title: String,
    pub channel: String,
    pub min_launcher_version: String,
    pub required: bool,
    pub allow_extra_mods: bool,
    pub recommended_memory_mb: Option<u32>,
    /// Basis des Pinecone-Servers, z. B. `https://pinecone.twogg.app`.
    pub server_url: String,
    pub key_id: String,
    pub published_at: String,
}

#[derive(Deserialize)]
struct IndexFile {
    path: String,
    hashes: IndexHashes,
    #[serde(default)]
    env: Option<IndexEnv>,
}

#[derive(Deserialize)]
struct IndexHashes {
    #[serde(default)]
    sha512: Option<String>,
}

#[derive(Deserialize)]
struct IndexEnv {
    #[serde(default)]
    client: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Index {
    #[serde(default)]
    files: Vec<IndexFile>,
}

/// Erzeugt das Manifest zu einem `.mrpack`. Pflichtdateien (`locked`) sind alle Dateien aus
/// `modrinth.index.json`, die der Client braucht, und alle Mods aus `overrides/mods/` bzw.
/// `client-overrides/mods/`. Configs und `options.txt` bleiben bewusst frei, damit Spieler ihre
/// Einstellungen ändern können (AK-12.3).
pub fn build_manifest(mrpack: &Path, opts: &ManifestOptions) -> Result<Manifest> {
    let pack = crate::modpack::read_mrpack(mrpack)?;
    let size = std::fs::metadata(mrpack).map_err(Error::io(mrpack))?.len();
    let sha512 = sha512_file(mrpack)?;

    let file = std::fs::File::open(mrpack).map_err(Error::io(mrpack))?;
    let zip_err = |e: zip::result::ZipError| Error::Zip {
        path: mrpack.to_path_buf(),
        source: e,
    };
    let mut zip = zip::ZipArchive::new(file).map_err(zip_err)?;
    let index: Index = {
        let mut text = String::new();
        zip.by_name("modrinth.index.json")
            .map_err(zip_err)?
            .read_to_string(&mut text)
            .map_err(Error::io(mrpack))?;
        serde_json::from_str(&text).map_err(Error::json("modrinth.index.json"))?
    };
    let mut locked = BTreeMap::new();
    for f in index.files {
        if f.env.as_ref().is_some_and(|e| e.client == "unsupported") {
            continue;
        }
        let sha = f.hashes.sha512.filter(|s| s.len() == 128).ok_or_else(|| {
            Error::Invalid(format!("{}: SHA-512 fehlt in modrinth.index.json", f.path))
        })?;
        locked.insert(f.path, sha.to_ascii_lowercase());
    }
    // client-overrides gewinnen wie bei der Installation gegen overrides.
    for prefix in ["overrides/", "client-overrides/"] {
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(zip_err)?;
            let Some(rel) = entry.name().strip_prefix(prefix).map(str::to_string) else {
                continue;
            };
            if entry.is_dir() || !rel.starts_with("mods/") {
                continue;
            }
            let mut data = Vec::new();
            entry.read_to_end(&mut data).map_err(Error::io(mrpack))?;
            locked.insert(rel, sha512_hex(&data));
        }
    }

    Ok(Manifest {
        schema: SCHEMA.into(),
        pack_id: opts.pack_id.clone(),
        version: pack.version.clone(),
        channel: opts.channel.clone(),
        event_id: opts.event_id.clone(),
        title: opts.title.clone(),
        minecraft: pack.minecraft.clone(),
        loader: pack.loader.clone(),
        min_launcher_version: opts.min_launcher_version.clone(),
        required: opts.required,
        allow_extra_mods: opts.allow_extra_mods,
        recommended_memory_mb: opts.recommended_memory_mb,
        mrpack: MrpackRef {
            url: format!(
                "{}/api/v1/packs/{}/versions/{}/download",
                opts.server_url.trim_end_matches('/'),
                opts.pack_id,
                pack.version
            ),
            size,
            sha512,
        },
        locked: locked
            .into_iter()
            .map(|(path, sha512)| LockedFile { path, sha512 })
            .collect(),
        published_at: opts.published_at.clone(),
        key_id: opts.key_id.clone(),
    })
}

/// Prüft, ob ein `.mrpack` zum Manifest passt (Schritte 6 und 7 aus `04` §6.2).
pub fn check_mrpack(manifest: &Manifest, mrpack: &Path) -> Result<()> {
    let size = std::fs::metadata(mrpack).map_err(Error::io(mrpack))?.len();
    let sha512 = sha512_file(mrpack)?;
    if size != manifest.mrpack.size || !sha512.eq_ignore_ascii_case(&manifest.mrpack.sha512) {
        return Err(Error::HashMismatch {
            path: mrpack.to_path_buf(),
            expected: manifest.mrpack.sha512.clone(),
            actual: sha512,
        });
    }
    let pack = crate::modpack::read_mrpack(mrpack)?;
    if pack.minecraft != manifest.minecraft || pack.loader != manifest.loader {
        return Err(Error::Invalid(format!(
            "Das Modpack passt nicht zum Manifest (Minecraft {} statt {})",
            pack.minecraft, manifest.minecraft
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_form_follows_rfc8785() {
        let v = json!({ "b": [1, "x\n", true, null], "a": { "z": 1, "é": 2, "A": 3 }, "\u{20ac}": "€" });
        assert_eq!(
            canonical_json(&v).unwrap(),
            "{\"a\":{\"A\":3,\"z\":1,\"é\":2},\"b\":[1,\"x\\n\",true,null],\"€\":\"€\"}"
        );
        assert!(canonical_json(&json!({ "x": 1.5 })).is_err());
        // Sortierung nach UTF-16: U+1F600 (Surrogatpaar D83D…) vor U+FFFD.
        let v = json!({ "\u{fffd}": 1, "\u{1f600}": 2 });
        assert_eq!(
            canonical_json(&v).unwrap(),
            "{\"\u{1f600}\":2,\"\u{fffd}\":1}"
        );
    }

    fn manifest() -> Manifest {
        Manifest {
            schema: SCHEMA.into(),
            pack_id: "avalon".into(),
            version: "1.4.0".into(),
            channel: "release".into(),
            event_id: "evt-1".into(),
            title: "REVIVAL: Avalon".into(),
            minecraft: "1.21.11".into(),
            loader: None,
            min_launcher_version: "0.10.0".into(),
            required: true,
            allow_extra_mods: false,
            recommended_memory_mb: Some(6144),
            mrpack: MrpackRef {
                url: "https://pinecone.test/x".into(),
                size: 1,
                sha512: "00".repeat(64),
            },
            locked: vec![LockedFile {
                path: "mods/a.jar".into(),
                sha512: "11".repeat(64),
            }],
            published_at: "2026-10-14T00:00:00Z".into(),
            key_id: "test".into(),
        }
    }

    #[test]
    fn sign_and_verify() {
        let (private, public) = generate_key();
        let key = signing_key(&private).unwrap();
        assert_eq!(public_key(&key), public);
        let keyring = Keyring::parse(&format!("test={public}")).unwrap();
        let signed = sign(&manifest(), &key).unwrap();
        assert_eq!(keyring.verify(&signed).unwrap(), manifest());

        // Jede Änderung am Inhalt bricht die Signatur, die Reihenfolge der Schlüssel nicht.
        let mut changed = signed.clone();
        changed.manifest["allowExtraMods"] = json!(true);
        assert!(
            keyring
                .verify(&changed)
                .unwrap_err()
                .to_string()
                .contains(NOT_SIGNED)
        );
        let reordered: Value =
            serde_json::from_str(&canonical_json(&signed.manifest).unwrap()).unwrap();
        assert!(
            keyring
                .verify(&SignedManifest {
                    manifest: reordered,
                    signature: signed.signature.clone()
                })
                .is_ok()
        );

        // Fremder Schlüssel, unbekannte keyId, kaputte Signatur.
        let (_, other) = generate_key();
        let wrong = Keyring::parse(&format!("test={other}")).unwrap();
        assert!(wrong.verify(&signed).is_err());
        assert!(Keyring::default().verify(&signed).is_err());
        let broken = SignedManifest {
            manifest: signed.manifest.clone(),
            signature: "AAAA".into(),
        };
        assert!(keyring.verify(&broken).is_err());

        // Neue Hauptversion des Formats wird abgelehnt, auch wenn richtig signiert.
        let mut future = manifest();
        future.schema = "pinecone-eventpack/2".into();
        let err = keyring.verify(&sign(&future, &key).unwrap()).unwrap_err();
        assert!(err.to_string().contains("aktualisiere Pinecone"), "{err}");
    }

    #[test]
    fn keyring_parsing() {
        let (_, a) = generate_key();
        let (_, b) = generate_key();
        let k = Keyring::parse(&format!(" twogg-2026={a} ; twogg-2027={b};")).unwrap();
        assert_eq!(k.keys.len(), 2);
        assert!(Keyring::parse("ohne-gleich").is_err());
        assert!(Keyring::parse("x=kaputt").is_err());
        assert!(Keyring::parse("").unwrap().is_empty());
    }
}
