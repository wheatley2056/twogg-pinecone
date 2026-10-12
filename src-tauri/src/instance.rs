//! Tabs „Welten“ und „Server“ einer Instanz.

use pinecone_core::servers::{SavedServer, read_servers_dat};
use pinecone_core::worlds::{self, World, WorldBackup};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::AppState;
use crate::commands::{CmdResult, err};

fn game_dir(state: &AppState, id: &str) -> CmdResult<std::path::PathBuf> {
    let profile = state.launcher.profiles().get(id).map_err(err)?;
    Ok(state.launcher.game_dir(&profile))
}

fn running(state: &AppState, id: &str) -> bool {
    state.running.lock().unwrap().contains_key(id)
}

#[tauri::command]
pub fn worlds_list(state: State<'_, AppState>, id: String) -> CmdResult<Vec<World>> {
    worlds::list(&game_dir(&state, &id)?).map_err(err)
}

/// Backup-Ordner der Welten einer Instanz: `<data>/backups/worlds/<instanz>/`.
pub fn backup_dir(state: &AppState, id: &str) -> std::path::PathBuf {
    state
        .launcher
        .paths()
        .root()
        .join("backups")
        .join("worlds")
        .join(id)
}

/// Sichert eine Welt nach [`backup_dir`] und gibt den Pfad zurück.
#[tauri::command]
pub async fn world_backup(
    state: State<'_, AppState>,
    id: String,
    folder: String,
) -> CmdResult<String> {
    let dir = game_dir(&state, &id)?;
    let target = backup_dir(&state, &id);
    tauri::async_runtime::spawn_blocking(move || worlds::backup(&dir, &folder, &target))
        .await
        .map_err(err)?
        .map(|p| p.display().to_string())
        .map_err(err)
}

#[tauri::command]
pub fn world_backups(state: State<'_, AppState>, id: String) -> CmdResult<Vec<WorldBackup>> {
    state.launcher.profiles().get(&id).map_err(err)?;
    worlds::list_backups(&backup_dir(&state, &id)).map_err(err)
}

/// Stellt eine Welt aus einem Backup wieder her. Gibt den Pfad der Sicherung des vorherigen
/// Stands zurück (falls die Welt noch existierte).
#[tauri::command]
pub async fn world_restore(
    state: State<'_, AppState>,
    id: String,
    file: String,
) -> CmdResult<Option<String>> {
    if running(&state, &id) {
        return Err("Die Instanz läuft gerade. Beende zuerst das Spiel.".into());
    }
    let dir = game_dir(&state, &id)?;
    let backups = backup_dir(&state, &id);
    tauri::async_runtime::spawn_blocking(move || worlds::restore(&dir, &backups, &file))
        .await
        .map_err(err)?
        .map(|p| p.map(|p| p.display().to_string()))
        .map_err(err)
}

#[tauri::command]
pub fn world_backup_delete(state: State<'_, AppState>, id: String, file: String) -> CmdResult<()> {
    state.launcher.profiles().get(&id).map_err(err)?;
    worlds::delete_backup(&backup_dir(&state, &id), &file).map_err(err)
}

/// Automatische Backups vor dem Start ein- oder ausschalten.
#[tauri::command]
pub fn set_backup_on_launch(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
    keep: u32,
) -> CmdResult<()> {
    let profiles = state.launcher.profiles();
    let mut profile = profiles.get(&id).map_err(err)?;
    profile.backup_on_launch = enabled;
    profile.backup_keep = Some(keep.clamp(1, 50));
    profiles.save(&profile).map_err(err)
}

/// Sichert vor dem Start alle seit dem letzten Backup geänderten Welten und rotiert die
/// automatischen Backups. Fehler einzelner Welten werden geloggt und halten den Start nicht auf.
pub fn backup_before_launch(state: &AppState, profile: &pinecone_core::profiles::Profile) {
    let game = state.launcher.game_dir(profile);
    let target = backup_dir(state, &profile.id);
    let keep = profile
        .backup_keep
        .unwrap_or(pinecone_core::profiles::DEFAULT_BACKUP_KEEP) as usize;
    let Ok(list) = worlds::list(&game) else {
        return;
    };
    for w in list {
        let result = worlds::changed_since_backup(&game, &w.folder, &target).and_then(|changed| {
            if changed {
                worlds::backup_auto(&game, &w.folder, &target)?;
            }
            worlds::prune_auto(&target, &w.folder, keep)
        });
        if let Err(e) = result {
            tracing::warn!("Auto-Backup von {} fehlgeschlagen: {e}", w.folder);
        }
    }
}

#[tauri::command]
pub fn world_delete(state: State<'_, AppState>, id: String, folder: String) -> CmdResult<()> {
    if running(&state, &id) {
        return Err("Die Instanz läuft gerade. Beende zuerst das Spiel.".into());
    }
    worlds::delete(&game_dir(&state, &id)?, &folder).map_err(err)
}

#[tauri::command]
pub fn world_open_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    folder: String,
) -> CmdResult<()> {
    if !worlds::valid_folder(&folder) {
        return Err("Ungültige Welt".into());
    }
    let dir = game_dir(&state, &id)?.join("saves").join(folder);
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(err)
}

#[tauri::command]
pub fn servers_list(state: State<'_, AppState>, id: String) -> CmdResult<Vec<SavedServer>> {
    read_servers_dat(&game_dir(&state, &id)?).map_err(err)
}
