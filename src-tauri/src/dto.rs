//! Datentypen für die Oberfläche (camelCase wie im TypeScript-Code).

use pinecone_core::accounts::StoredAccount;
use pinecone_core::auth::AccountKind;
use pinecone_core::loader::LoaderSpec;
use pinecone_core::profiles::Profile;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: &'static str,
    pub data_dir: String,
    pub msa_configured: bool,
    pub curseforge_configured: bool,
    pub updates_configured: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDto {
    pub id: String,
    pub kind: String,
    pub release_time: String,
}

#[derive(Serialize)]
pub struct LoaderVersionDto {
    pub version: String,
    pub stable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDto {
    pub id: String,
    pub name: String,
    pub minecraft: String,
    pub loader: Option<LoaderSpec>,
    pub memory_mb: Option<u32>,
    pub jvm_args: Vec<String>,
    pub java: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub running: bool,
    /// Herkunft, falls aus einem Modpack (`twogg`, `modrinth`, `curseforge`, `file`); nur in der Liste.
    pub pack_source: Option<&'static str>,
    /// Pack-ID bei TwoGG-Packs (für die Zuordnung zu Events); nur in der Liste.
    pub pack_id: Option<String>,
    /// Unix-Zeit des letzten Spielendes; nur in der Liste.
    pub last_played: Option<u64>,
}

impl ProfileDto {
    pub fn new(p: Profile, running: bool) -> Self {
        Self {
            id: p.id,
            name: p.name,
            minecraft: p.minecraft,
            loader: p.loader,
            memory_mb: p.memory_mb,
            jvm_args: p.jvm_args,
            java: p.java.map(|j| j.display().to_string()),
            color: p.color,
            icon: p.icon,
            running,
            pack_source: None,
            pack_id: None,
            last_played: None,
        }
    }
}

/// Eingaben aus dem Profil-Dialog. `loader.version` leer = empfohlene Version wählen.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    pub name: String,
    pub minecraft: String,
    pub loader: Option<LoaderSpec>,
    pub memory_mb: Option<u32>,
    #[serde(default)]
    pub jvm_args: Vec<String>,
    pub java: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDto {
    pub uuid: String,
    pub name: String,
    pub kind: AccountKind,
    pub active: bool,
}

impl AccountDto {
    pub fn new(a: &StoredAccount, active: bool) -> Self {
        Self {
            uuid: a.uuid.to_string(),
            name: a.name.clone(),
            kind: a.kind,
            active,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStart {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaDto {
    pub path: String,
    pub version: String,
    pub major: u32,
    pub managed: bool,
}

/// Ereignisse während Vorbereitung und Spiel, über einen Tauri-Channel an die Oberfläche.
#[derive(Clone, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LaunchEvent {
    Stage {
        stage: &'static str,
    },
    Progress {
        done: usize,
        files: usize,
        bytes_done: u64,
        bytes: u64,
    },
    Java {
        version: String,
        path: String,
    },
    Started {
        pid: Option<u32>,
    },
    Log {
        lines: Vec<String>,
    },
    Exited {
        code: Option<i32>,
        success: bool,
    },
}
