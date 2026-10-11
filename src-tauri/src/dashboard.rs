//! Dashboard (zuletzt gespielt, Server, Screenshots, Spielzeit) und Personalisierung.

use std::path::{Path, PathBuf};

use pinecone_core::servers::{self, ServerAddress, ServerStatus};
use pinecone_core::settings::Settings;
use pinecone_core::stats::ProfileStats;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::AppState;
use crate::commands::{CmdResult, err};
use crate::dto::ProfileDto;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentDto {
    pub profile: ProfileDto,
    pub stats: ProfileStats,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEntry {
    pub profile_id: String,
    pub profile_name: String,
    pub name: String,
    pub address: String,
    pub icon: Option<String>,
    /// Zuletzt in diesem Profil gespielt.
    pub last: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotDto {
    pub profile_id: String,
    pub path: String,
    pub modified: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDto {
    /// Alle Profile mit Statistik, zuletzt gespielte zuerst.
    pub recent: Vec<RecentDto>,
    pub servers: Vec<ServerEntry>,
    pub screenshots: Vec<ScreenshotDto>,
    pub total_playtime_secs: u64,
}

#[tauri::command]
pub fn dashboard(state: State<'_, AppState>) -> CmdResult<DashboardDto> {
    let launcher = &state.launcher;
    let profiles = launcher.profiles().list().map_err(err)?;
    let stats = launcher.stats().load().map_err(err)?;

    let mut servers = Vec::new();
    for p in &profiles {
        let last = stats
            .profiles
            .get(&p.id)
            .and_then(|s| s.last_server.clone());
        let saved = servers::read_servers_dat(&launcher.game_dir(p)).unwrap_or_else(|e| {
            tracing::warn!("servers.dat von {} unlesbar: {e}", p.id);
            Vec::new()
        });
        let same = |a: &str, b: &ServerAddress| {
            ServerAddress::parse(a).is_ok_and(|a| {
                a.host.eq_ignore_ascii_case(&b.host) && a.port_or_default() == b.port_or_default()
            })
        };
        // Zuletzt gespielter Server zuerst, auch wenn er nicht in der Serverliste steht.
        if let Some(addr) = &last
            && !saved.iter().any(|s| same(&s.address, addr))
        {
            servers.push(ServerEntry {
                profile_id: p.id.clone(),
                profile_name: p.name.clone(),
                name: addr.display(),
                address: addr.display(),
                icon: None,
                last: true,
            });
        }
        for s in saved {
            servers.push(ServerEntry {
                profile_id: p.id.clone(),
                profile_name: p.name.clone(),
                last: last.as_ref().is_some_and(|l| same(&s.address, l)),
                name: s.name,
                address: s.address,
                icon: s.icon,
            });
        }
    }
    servers.sort_by_key(|s| !s.last);

    let dirs: Vec<(String, PathBuf)> = profiles
        .iter()
        .map(|p| (p.id.clone(), launcher.game_dir(p)))
        .collect();
    let screenshots =
        pinecone_core::screenshots::latest(dirs.iter().map(|(id, d)| (id.as_str(), d.clone())), 8)
            .into_iter()
            .map(|s| ScreenshotDto {
                profile_id: s.profile_id,
                path: s.path.display().to_string(),
                modified: s.modified,
            })
            .collect();

    let total_playtime_secs = stats.profiles.values().map(|s| s.playtime_secs).sum();
    let mut recent: Vec<RecentDto> = profiles
        .into_iter()
        .map(|p| {
            let s = stats.profiles.get(&p.id).cloned().unwrap_or_default();
            let running = state.running.lock().unwrap().contains_key(&p.id);
            RecentDto {
                profile: ProfileDto::new(p, running),
                stats: s,
            }
        })
        .collect();
    recent.sort_by_key(|r| std::cmp::Reverse(r.stats.last_played));
    Ok(DashboardDto {
        recent,
        servers,
        screenshots,
        total_playtime_secs,
    })
}

#[tauri::command]
pub async fn ping_server(address: String) -> CmdResult<ServerStatus> {
    let addr = ServerAddress::parse(&address).map_err(err)?;
    servers::ping(&addr).await.map_err(err)
}

/// Öffnet einen Screenshot im Bildbetrachter – nur Dateien aus Profilordnern.
#[tauri::command]
pub fn open_screenshot(app: AppHandle, state: State<'_, AppState>, path: String) -> CmdResult<()> {
    let instances = state.launcher.paths().root().join("instances");
    let path =
        pinecone_core::screenshots::checked(&PathBuf::from(path), &instances).map_err(err)?;
    app.opener()
        .open_path(path.display().to_string(), None::<&str>)
        .map_err(err)
}

/// Gibt der Oberfläche Lesezugriff auf Screenshots (Profilordner).
pub fn allow_assets(app: &AppHandle) {
    let state = app.state::<AppState>();
    let root = state.launcher.paths().root().to_path_buf();
    let scope = app.asset_protocol_scope();
    if let Err(e) = scope.allow_directory(root.join("instances"), true) {
        tracing::warn!("Asset-Freigabe fehlgeschlagen: {e}");
    }
    // Vorschaubilder der Screenshot-Galerie.
    let thumbs = crate::screenshots::thumbs_dir(&state);
    let _ = std::fs::create_dir_all(&thumbs);
    if let Err(e) = scope.allow_directory(thumbs, false) {
        tracing::warn!("Asset-Freigabe fehlgeschlagen: {e}");
    }
}

/// Eigene Kopfleiste oder Rahmen des Systems (Einstellung `systemFrame`).
pub fn apply_frame(app: &AppHandle, settings: &Settings) {
    if let Some(window) = app.get_webview_window("main")
        && let Err(e) = window.set_decorations(settings.system_frame)
    {
        tracing::warn!("Fensterrahmen nicht umschaltbar: {e}");
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> CmdResult<Settings> {
    state.launcher.settings().load().map_err(err)
}

/// Eingebaute JVM-Presets (Aikar's Flags usw.) im Originalzustand, zum Vergleichen und Zurücksetzen.
#[tauri::command]
pub fn jvm_builtin_presets() -> Vec<pinecone_core::jvm::JvmPreset> {
    pinecone_core::jvm::builtin()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> CmdResult<Settings> {
    let saved = state.launcher.settings().save(&settings).map_err(err)?;
    apply_frame(&app, &saved);
    Ok(saved)
}

#[tauri::command]
pub fn export_theme(state: State<'_, AppState>, path: String) -> CmdResult<()> {
    state
        .launcher
        .settings()
        .export_theme(Path::new(&path))
        .map_err(err)
}

#[tauri::command]
pub fn import_theme(state: State<'_, AppState>, path: String) -> CmdResult<Settings> {
    state
        .launcher
        .settings()
        .import_theme(Path::new(&path))
        .map_err(err)
}
