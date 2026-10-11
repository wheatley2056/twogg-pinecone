//! Skins: lokale Bibliothek (`<data-dir>/skins/`), Import von Spielernamen und Dateien sowie
//! Hochladen und Capes für Microsoft-Accounts (siehe `MsaClient::upload_skin`/`set_cape`).
//!
//! Minecraft akzeptiert nur Skins mit 64×64 oder 64×32 Pixeln; das wird beim Speichern geprüft.

use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::paths::Paths;
use crate::stats::now_secs;
use crate::{Error, LAUNCHER_NAME, LAUNCHER_VERSION, Result};

const INDEX: &str = "skins.json";
/// Größer ist kein Skin (64×64-PNGs haben wenige KB).
const MAX_PNG: usize = 512 * 1024;
/// Hosts, von denen Skin-Texturen geladen werden dürfen.
const TEXTURE_HOSTS: [&str; 1] = ["textures.minecraft.net"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkinModel {
    /// Steve: 4 Pixel breite Arme.
    #[default]
    Classic,
    /// Alex: 3 Pixel breite Arme.
    Slim,
}

impl SkinModel {
    /// Wert für die Minecraft-API (`variant`).
    pub fn variant(self) -> &'static str {
        match self {
            SkinModel::Classic => "classic",
            SkinModel::Slim => "slim",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSkin {
    pub id: String,
    pub name: String,
    pub model: SkinModel,
    pub created: u64,
    pub updated: u64,
}

/// Breite und Höhe aus dem PNG-Kopf; prüft Signatur und Größe für Minecraft.
pub fn check_skin_png(bytes: &[u8]) -> Result<(u32, u32)> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() > MAX_PNG {
        return Err(Error::Invalid(
            "Die Datei ist zu groß für einen Skin".into(),
        ));
    }
    if bytes.len() < 24 || bytes[..8] != SIGNATURE || &bytes[12..16] != b"IHDR" {
        return Err(Error::Invalid("Keine PNG-Datei".into()));
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    match (width, height) {
        (64, 64) | (64, 32) => Ok((width, height)),
        _ => Err(Error::Invalid(format!(
            "Minecraft akzeptiert nur Skins mit 64×64 oder 64×32 Pixeln (diese Datei: {width}×{height})"
        ))),
    }
}

/// `data:image/png;base64,…` → Bytes (für Skins aus dem Editor).
pub fn decode_data_url(data_url: &str) -> Result<Vec<u8>> {
    let b64 = data_url
        .strip_prefix("data:image/png;base64,")
        .ok_or_else(|| Error::Invalid("Erwartet ein PNG als data:-URL".into()))?;
    base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| Error::Invalid(format!("Ungültige PNG-Daten: {e}")))
}

pub fn encode_data_url(bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

#[derive(Debug, Clone)]
pub struct MojangEndpoints {
    /// Name → UUID, z. B. `https://api.mojang.com`.
    pub api: String,
    /// UUID → Profil mit Texturen, z. B. `https://sessionserver.mojang.com`.
    pub session: String,
}

impl Default for MojangEndpoints {
    fn default() -> Self {
        Self {
            api: "https://api.mojang.com".into(),
            session: "https://sessionserver.mojang.com".into(),
        }
    }
}

#[derive(Clone)]
pub struct SkinLibrary {
    root: PathBuf,
    http: reqwest::Client,
    mojang: MojangEndpoints,
    /// Zusätzliche Textur-Hosts (nur für Tests).
    extra_hosts: Vec<String>,
}

impl SkinLibrary {
    pub fn new(paths: &Paths) -> Result<Self> {
        Self::with_endpoints(paths, MojangEndpoints::default())
    }

    pub fn with_endpoints(paths: &Paths, mojang: MojangEndpoints) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(format!("{LAUNCHER_NAME}/{LAUNCHER_VERSION}"))
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|source| Error::Http {
                url: String::new(),
                source,
            })?;
        Ok(Self {
            root: paths.root().join("skins"),
            http,
            mojang,
            extra_hosts: Vec::new(),
        })
    }

    /// Nur für Tests: weiteren Textur-Host erlauben (z. B. einen lokalen Mock-Server).
    pub fn allow_texture_host(mut self, host: &str) -> Self {
        self.extra_hosts.push(host.to_string());
        self
    }

    fn index_path(&self) -> PathBuf {
        self.root.join(INDEX)
    }

    pub fn list(&self) -> Result<Vec<SavedSkin>> {
        let path = self.index_path();
        let mut list: Vec<SavedSkin> = match std::fs::read(&path) {
            Ok(b) => serde_json::from_slice(&b).map_err(Error::json(path.display().to_string()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(Error::io(&path)(e)),
        };
        list.sort_by_key(|s| std::cmp::Reverse(s.updated));
        Ok(list)
    }

    fn save_index(&self, list: &[SavedSkin]) -> Result<()> {
        std::fs::create_dir_all(&self.root).map_err(Error::io(&self.root))?;
        let path = self.index_path();
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(list).map_err(Error::json(INDEX))?;
        std::fs::write(&tmp, json).map_err(Error::io(&tmp))?;
        std::fs::rename(&tmp, &path).map_err(Error::io(&path))
    }

    fn get(&self, id: &str) -> Result<SavedSkin> {
        self.list()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| Error::Invalid(format!("Kein Skin {id:?} in der Bibliothek")))
    }

    /// PNG-Datei eines Skins (IDs sind Ziffern, siehe `add`).
    pub fn path(&self, id: &str) -> Result<PathBuf> {
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
            return Err(Error::Invalid(format!("Ungültige Skin-ID {id:?}")));
        }
        Ok(self.root.join(format!("{id}.png")))
    }

    pub fn png(&self, id: &str) -> Result<Vec<u8>> {
        self.get(id)?;
        let path = self.path(id)?;
        std::fs::read(&path).map_err(Error::io(&path))
    }

    pub fn add(&self, name: &str, model: SkinModel, png: &[u8]) -> Result<SavedSkin> {
        check_skin_png(png)?;
        let name = clean_name(name)?;
        let mut list = self.list()?;
        let now = now_secs();
        let base = format!("{}", now_millis());
        let mut id = base.clone();
        let mut n = 2;
        while list.iter().any(|s| s.id == id) || self.path(&id)?.exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        std::fs::create_dir_all(&self.root).map_err(Error::io(&self.root))?;
        let path = self.path(&id)?;
        std::fs::write(&path, png).map_err(Error::io(&path))?;
        let skin = SavedSkin {
            id,
            name,
            model,
            created: now,
            updated: now,
        };
        list.push(skin.clone());
        self.save_index(&list)?;
        Ok(skin)
    }

    /// Bild und Modell eines Skins ersetzen (z. B. nach dem Bearbeiten im Editor).
    pub fn update(&self, id: &str, model: SkinModel, png: &[u8]) -> Result<SavedSkin> {
        check_skin_png(png)?;
        let path = self.path(id)?;
        self.modify(
            id,
            |s| s.model = model,
            || std::fs::write(&path, png).map_err(Error::io(&path)),
        )
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<SavedSkin> {
        let name = clean_name(name)?;
        self.modify(id, |s| s.name = name, || Ok(()))
    }

    pub fn set_model(&self, id: &str, model: SkinModel) -> Result<SavedSkin> {
        self.modify(id, |s| s.model = model, || Ok(()))
    }

    fn modify(
        &self,
        id: &str,
        change: impl FnOnce(&mut SavedSkin),
        write: impl FnOnce() -> Result<()>,
    ) -> Result<SavedSkin> {
        let mut list = self.list()?;
        let skin = list
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| Error::Invalid(format!("Kein Skin {id:?} in der Bibliothek")))?;
        write()?;
        change(skin);
        skin.updated = now_secs();
        let skin = skin.clone();
        self.save_index(&list)?;
        Ok(skin)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let path = self.path(id)?;
        let mut list = self.list()?;
        let before = list.len();
        list.retain(|s| s.id != id);
        if list.len() == before {
            return Err(Error::Invalid(format!(
                "Kein Skin {id:?} in der Bibliothek"
            )));
        }
        self.save_index(&list)?;
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    /// Skin aus einer PNG-Datei; Name aus dem Dateinamen, falls keiner angegeben ist.
    pub fn import_file(
        &self,
        file: &Path,
        name: Option<&str>,
        model: SkinModel,
    ) -> Result<SavedSkin> {
        let png = std::fs::read(file).map_err(Error::io(file))?;
        let fallback = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Skin")
            .to_string();
        self.add(name.unwrap_or(&fallback), model, &png)
    }

    /// Aktuellen Skin eines Spielers über die öffentliche Mojang-API übernehmen.
    pub async fn import_player(&self, player: &str) -> Result<SavedSkin> {
        let player = player.trim();
        if !crate::auth::valid_player_name(player) {
            return Err(Error::Invalid(format!("Ungültiger Spielername {player:?}")));
        }
        #[derive(Deserialize)]
        struct Uuid {
            id: String,
            name: String,
        }
        let url = format!("{}/users/profiles/minecraft/{player}", self.mojang.api);
        let resp = self.http.get(&url).send().await.map_err(http(&url))?;
        if matches!(resp.status().as_u16(), 204 | 404) {
            return Err(Error::Invalid(format!("Spieler {player:?} gibt es nicht")));
        }
        let who: Uuid = resp
            .error_for_status()
            .map_err(http(&url))?
            .json()
            .await
            .map_err(http(&url))?;
        let (url, model) = self.texture_of(&who.id).await?;
        let png = self.fetch_texture(&url).await?;
        self.add(&who.name, model, &png)
    }

    /// Skin-Textur von `textures.minecraft.net` übernehmen (z. B. den aktiven Skin des Accounts).
    pub async fn import_texture(
        &self,
        url: &str,
        name: &str,
        model: SkinModel,
    ) -> Result<SavedSkin> {
        let png = self.fetch_texture(url).await?;
        self.add(name, model, &png)
    }

    async fn texture_of(&self, uuid: &str) -> Result<(String, SkinModel)> {
        #[derive(Deserialize)]
        struct Profile {
            properties: Vec<Property>,
        }
        #[derive(Deserialize)]
        struct Property {
            name: String,
            value: String,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "UPPERCASE")]
        struct Textures {
            #[serde(default)]
            skin: Option<Texture>,
        }
        #[derive(Deserialize)]
        struct Texture {
            url: String,
            #[serde(default)]
            metadata: Option<Meta>,
        }
        #[derive(Deserialize)]
        struct Meta {
            #[serde(default)]
            model: Option<String>,
        }
        #[derive(Deserialize)]
        struct Payload {
            textures: Textures,
        }
        let url = format!("{}/session/minecraft/profile/{uuid}", self.mojang.session);
        let profile: Profile = self
            .http
            .get(&url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(http(&url))?
            .json()
            .await
            .map_err(http(&url))?;
        let value = profile
            .properties
            .into_iter()
            .find(|p| p.name == "textures")
            .ok_or_else(|| Error::Invalid("Profil ohne Texturen".into()))?
            .value;
        let json = base64::engine::general_purpose::STANDARD
            .decode(value)
            .map_err(|e| Error::Invalid(format!("Ungültige Texturdaten: {e}")))?;
        let payload: Payload = serde_json::from_slice(&json).map_err(Error::json("textures"))?;
        let skin = payload.textures.skin.ok_or_else(|| {
            Error::Invalid("Dieser Spieler nutzt den Standard-Skin (Steve/Alex)".into())
        })?;
        let slim = skin
            .metadata
            .and_then(|m| m.model)
            .is_some_and(|m| m == "slim");
        Ok((
            skin.url,
            if slim {
                SkinModel::Slim
            } else {
                SkinModel::Classic
            },
        ))
    }

    /// Skin- oder Cape-Textur von Mojang als `data:`-URL (für die Anzeige in der App).
    pub async fn texture_data_url(&self, url: &str) -> Result<String> {
        let bytes = self.fetch_raw(url).await?;
        if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") || bytes.len() > MAX_PNG {
            return Err(Error::Invalid("Textur ist kein gültiges PNG".into()));
        }
        Ok(encode_data_url(&bytes))
    }

    async fn fetch_texture(&self, url: &str) -> Result<Vec<u8>> {
        let bytes = self.fetch_raw(url).await?;
        check_skin_png(&bytes)?;
        Ok(bytes)
    }

    async fn fetch_raw(&self, url: &str) -> Result<Vec<u8>> {
        let mut parsed = reqwest::Url::parse(url)
            .map_err(|_| Error::Invalid(format!("Ungültige Adresse {url}")))?;
        // Mojang nennt Texturen mit http://; der Server kann auch HTTPS.
        if parsed.scheme() == "http"
            && parsed
                .host_str()
                .is_some_and(|h| TEXTURE_HOSTS.contains(&h))
        {
            let _ = parsed.set_scheme("https");
        }
        let allowed = parsed
            .host_str()
            .is_some_and(|h| TEXTURE_HOSTS.contains(&h) || self.extra_hosts.iter().any(|x| x == h));
        if !allowed || !matches!(parsed.scheme(), "https" | "http") {
            return Err(Error::Invalid(format!("Skin-Quelle nicht erlaubt: {url}")));
        }
        let bytes = self
            .http
            .get(parsed)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(http(url))?
            .bytes()
            .await
            .map_err(http(url))?;
        Ok(bytes.to_vec())
    }
}

fn http(url: &str) -> impl Fn(reqwest::Error) -> Error + '_ {
    move |source| Error::Http {
        url: url.to_string(),
        source,
    }
}

fn clean_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 48 {
        return Err(Error::Invalid(
            "Bitte einen Namen (1–48 Zeichen) angeben".into(),
        ));
    }
    Ok(name.to_string())
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimaler PNG-Kopf (Signatur + IHDR) mit Breite und Höhe.
    fn png_header(w: u32, h: u32) -> Vec<u8> {
        let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
        b.extend_from_slice(b"IHDR");
        b.extend_from_slice(&w.to_be_bytes());
        b.extend_from_slice(&h.to_be_bytes());
        b.extend_from_slice(&[8, 6, 0, 0, 0]);
        b
    }

    #[test]
    fn accepts_only_minecraft_sizes() {
        assert_eq!(check_skin_png(&png_header(64, 64)).unwrap(), (64, 64));
        assert_eq!(check_skin_png(&png_header(64, 32)).unwrap(), (64, 32));
        let err = check_skin_png(&png_header(128, 128)).unwrap_err();
        assert!(err.to_string().contains("128×128"), "{err}");
        assert!(check_skin_png(b"GIF89a....................").is_err());
        assert!(check_skin_png(&vec![0; MAX_PNG + 1]).is_err());
    }

    #[test]
    fn data_urls_round_trip() {
        let png = png_header(64, 64);
        assert_eq!(decode_data_url(&encode_data_url(&png)).unwrap(), png);
        assert!(decode_data_url("data:image/jpeg;base64,AAAA").is_err());
    }

    #[test]
    fn library_add_update_rename_delete() {
        let dir = tempfile::tempdir().unwrap();
        let lib = SkinLibrary::new(&Paths::new(dir.path())).unwrap();
        let a = lib
            .add("  Mein Skin ", SkinModel::Slim, &png_header(64, 64))
            .unwrap();
        assert_eq!((a.name.as_str(), a.model), ("Mein Skin", SkinModel::Slim));
        let b = lib
            .add("Zweiter", SkinModel::Classic, &png_header(64, 32))
            .unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(lib.list().unwrap().len(), 2);
        assert!(
            lib.add("HD", SkinModel::Classic, &png_header(128, 128))
                .is_err()
        );

        let mut png = png_header(64, 64);
        png.push(1);
        lib.update(&a.id, SkinModel::Classic, &png).unwrap();
        assert_eq!(lib.png(&a.id).unwrap(), png);
        assert_eq!(lib.rename(&a.id, "Neu").unwrap().name, "Neu");
        assert!(lib.rename(&a.id, "  ").is_err());

        lib.delete(&a.id).unwrap();
        assert!(lib.png(&a.id).is_err());
        assert!(lib.delete(&a.id).is_err());
        assert!(lib.path("../x").is_err());
    }
}
