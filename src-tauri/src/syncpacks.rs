//! Befehle für Sync-Packs und Vorlagen.

use std::path::Path;

use pinecone_core::profiles::Profile;
use pinecone_core::syncpacks::{ProfileSyncPack, SyncMode, SyncPack, SyncReport};
use tauri::State;

use crate::AppState;
use crate::commands::{CmdResult, err};

/// Neues Profil: Sync-Packs mit passender Auto-Regel einschalten. Fehler blockieren nichts.
pub async fn apply_auto(state: &AppState, profile: &Profile) {
    match state.launcher.syncpacks().apply_auto(profile).await {
        Ok(r) => {
            for w in r.warnings {
                tracing::warn!("Sync-Pack: {w}");
            }
        }
        Err(e) => tracing::warn!("Sync-Packs für {} nicht eingespielt: {e}", profile.id),
    }
}

fn not_running(state: &AppState, profile_id: &str) -> CmdResult<()> {
    if state.running.lock().unwrap().contains_key(profile_id) {
        return Err("Das Profil läuft gerade. Bitte erst das Spiel beenden.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn syncpack_list(state: State<'_, AppState>) -> CmdResult<Vec<SyncPack>> {
    state.launcher.syncpacks().list().map_err(err)
}

#[tauri::command]
pub fn syncpack_create(
    state: State<'_, AppState>,
    name: String,
    mode: SyncMode,
) -> CmdResult<SyncPack> {
    state.launcher.syncpacks().create(&name, mode).map_err(err)
}

/// Speichert Name, Art, Einträge, Optionen und Auto-Regel.
#[tauri::command]
pub fn syncpack_save(
    state: State<'_, AppState>,
    id: String,
    mut pack: SyncPack,
) -> CmdResult<SyncPack> {
    let sync = state.launcher.syncpacks();
    sync.get(&id).map_err(err)?;
    pack.id = id;
    sync.save(&pack).map_err(err)?;
    Ok(pack)
}

#[tauri::command]
pub fn syncpack_delete(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state.launcher.syncpacks().delete(&id).map_err(err)
}

#[tauri::command]
pub async fn syncpack_add_file(
    state: State<'_, AppState>,
    id: String,
    path: String,
    target: String,
) -> CmdResult<SyncPack> {
    state
        .launcher
        .syncpacks()
        .add_file(&id, Path::new(&path), &target)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn syncpack_add_from_profile(
    state: State<'_, AppState>,
    id: String,
    profile_id: String,
    keys: Vec<String>,
) -> CmdResult<SyncPack> {
    let profile = state.launcher.profiles().get(&profile_id).map_err(err)?;
    state
        .launcher
        .syncpacks()
        .add_from_profile(&id, &profile, &keys)
        .await
        .map_err(err)
}

#[tauri::command]
pub fn syncpack_for_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> CmdResult<Vec<ProfileSyncPack>> {
    state
        .launcher
        .syncpacks()
        .for_profile(&profile_id)
        .map_err(err)
}

#[tauri::command]
pub async fn syncpack_set_enabled(
    state: State<'_, AppState>,
    profile_id: String,
    id: String,
    enabled: bool,
) -> CmdResult<SyncReport> {
    not_running(&state, &profile_id)?;
    let profile = state.launcher.profiles().get(&profile_id).map_err(err)?;
    state
        .launcher
        .syncpacks()
        .set_enabled(&profile, &id, enabled)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn syncpack_sync(
    state: State<'_, AppState>,
    profile_id: String,
) -> CmdResult<SyncReport> {
    not_running(&state, &profile_id)?;
    let profile = state.launcher.profiles().get(&profile_id).map_err(err)?;
    state.launcher.syncpacks().sync(&profile).await.map_err(err)
}
