//! Tabs „Welten“ und „Server“ einer Instanz.

use pinecone_core::servers::{SavedServer, read_servers_dat};
use pinecone_core::worlds::{self, World};
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

/// Sichert eine Welt nach `<data>/backups/worlds/<instanz>/` und gibt den Pfad zurück.
#[tauri::command]
pub async fn world_backup(
    state: State<'_, AppState>,
    id: String,
    folder: String,
) -> CmdResult<String> {
    let dir = game_dir(&state, &id)?;
    let target = state
        .launcher
        .paths()
        .root()
        .join("backups")
        .join("worlds")
        .join(&id);
    tauri::async_runtime::spawn_blocking(move || worlds::backup(&dir, &folder, &target))
        .await
        .map_err(err)?
        .map(|p| p.display().to_string())
        .map_err(err)
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
