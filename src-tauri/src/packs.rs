//! Befehle für Modpacks: installieren, importieren, Updates mit Konfliktauswahl, Rollback, Export.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pinecone_core::modpack::{
    ExportReport, ImportResult, ManualDownload, PackSource, PackUpdate, Resolution, SnapshotInfo,
    UpdatePlan,
};
use serde::Serialize;
use tauri::State;

use crate::AppState;
use crate::commands::{CmdResult, err};
use crate::dto::ProfileDto;

/// Neues Profil aus einem Modpack und Dateien, die der Nutzer selbst laden muss.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDto {
    profile: ProfileDto,
    missing: Vec<ManualDownload>,
}

impl From<ImportResult> for ImportDto {
    fn from(r: ImportResult) -> Self {
        Self {
            profile: ProfileDto::new(r.profile, false),
            missing: r.missing,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInfoDto {
    name: String,
    version: String,
    /// `modrinth`, `curseforge` oder `file` (dann keine automatischen Updates).
    source: &'static str,
}

#[tauri::command]
pub async fn pack_install_modrinth(
    state: State<'_, AppState>,
    project: String,
    version: Option<String>,
) -> CmdResult<ImportDto> {
    let packs = state.launcher.packs();
    let (archive, source) = packs
        .download_modrinth(&project, version.as_deref())
        .await
        .map_err(err)?;
    Ok(packs
        .import(&archive, source, None)
        .await
        .map_err(err)?
        .into())
}

#[tauri::command]
pub async fn pack_install_curseforge(
    state: State<'_, AppState>,
    project: u64,
    file: Option<u64>,
) -> CmdResult<ImportDto> {
    let packs = state.launcher.packs();
    let (archive, source) = packs
        .download_curseforge(project, file, None)
        .await
        .map_err(err)?;
    Ok(packs
        .import(&archive, source, None)
        .await
        .map_err(err)?
        .into())
}

#[tauri::command]
pub async fn pack_import_file(state: State<'_, AppState>, path: String) -> CmdResult<ImportDto> {
    Ok(state
        .launcher
        .packs()
        .import(Path::new(&path), PackSource::File, None)
        .await
        .map_err(err)?
        .into())
}

#[tauri::command]
pub fn pack_info(state: State<'_, AppState>, id: String) -> CmdResult<Option<PackInfoDto>> {
    Ok(state
        .launcher
        .packs()
        .pack_info(&id)
        .map_err(err)?
        .map(|p| PackInfoDto {
            name: p.name,
            version: p.version,
            source: match p.source {
                PackSource::Modrinth { .. } => "modrinth",
                PackSource::Curseforge { .. } => "curseforge",
                PackSource::File => "file",
            },
        }))
}

#[tauri::command]
pub async fn pack_check_update(
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<Option<PackUpdate>> {
    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    state
        .launcher
        .packs()
        .check_update(&profile)
        .await
        .map_err(err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePackUpdate {
    profile: ProfileDto,
    update: PackUpdate,
}

/// Für das Übersichts-Widget: alle Profile mit verfügbarem Modpack-Update.
#[tauri::command]
pub async fn pack_updates_all(state: State<'_, AppState>) -> CmdResult<Vec<ProfilePackUpdate>> {
    let mut found = Vec::new();
    for profile in state.launcher.profiles().list().map_err(err)? {
        match state.launcher.packs().check_update(&profile).await {
            Ok(Some(update)) => found.push(ProfilePackUpdate {
                profile: ProfileDto::new(profile, false),
                update,
            }),
            Ok(None) => {}
            Err(e) => tracing::warn!("Pack-Update für {} nicht prüfbar: {e}", profile.id),
        }
    }
    Ok(found)
}

/// Lädt die Zielversion (Modrinth: neueste passende bzw. bestimmte; sonst Datei) und plant.
#[tauri::command]
pub async fn pack_plan_update(
    state: State<'_, AppState>,
    id: String,
    version: Option<String>,
    file: Option<String>,
) -> CmdResult<UpdatePlan> {
    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    let packs = state.launcher.packs();
    let info = packs
        .pack_info(&id)
        .map_err(err)?
        .ok_or("Dieses Profil stammt aus keinem Modpack")?;
    let (archive, source): (PathBuf, PackSource) = match (file, &info.source) {
        (Some(f), _) => (PathBuf::from(f), PackSource::File),
        (None, PackSource::Modrinth { project_id, .. }) => {
            let target = match version {
                Some(v) => v,
                None => {
                    packs
                        .check_update(&profile)
                        .await
                        .map_err(err)?
                        .ok_or("Das Modpack ist aktuell")?
                        .version_id
                }
            };
            packs
                .download_modrinth(project_id, Some(&target))
                .await
                .map_err(err)?
        }
        (None, PackSource::Curseforge { project_id, .. }) => {
            let target: u64 = match version {
                Some(v) => v.parse().map_err(err)?,
                None => packs
                    .check_update(&profile)
                    .await
                    .map_err(err)?
                    .ok_or("Das Modpack ist aktuell")?
                    .version_id
                    .parse()
                    .map_err(err)?,
            };
            packs
                .download_curseforge(*project_id, Some(target), None)
                .await
                .map_err(err)?
        }
        (None, PackSource::File) => {
            return Err("Für dieses Modpack bitte die neue Version als Datei wählen".into());
        }
    };
    let new = packs.read_archive(&archive).await.map_err(err)?;
    let plan = packs.plan_update(&profile, &new).await.map_err(err)?;
    state
        .pending_updates
        .lock()
        .unwrap()
        .insert(id, (archive, source));
    Ok(plan)
}

#[tauri::command]
pub async fn pack_apply_update(
    state: State<'_, AppState>,
    id: String,
    resolutions: HashMap<String, Resolution>,
) -> CmdResult<SnapshotInfo> {
    if state.running.lock().unwrap().contains_key(&id) {
        return Err("Das Profil läuft gerade. Bitte erst das Spiel beenden.".into());
    }
    let (archive, source) = state
        .pending_updates
        .lock()
        .unwrap()
        .remove(&id)
        .ok_or("Kein geplantes Update. Bitte erneut prüfen.")?;
    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    state
        .launcher
        .packs()
        .apply_update(&profile, &archive, source, &resolutions)
        .await
        .map_err(err)
}

#[tauri::command]
pub fn pack_snapshots(state: State<'_, AppState>, id: String) -> Vec<SnapshotInfo> {
    state.launcher.packs().snapshots(&id)
}

#[tauri::command]
pub fn pack_rollback(state: State<'_, AppState>, id: String, snapshot: String) -> CmdResult<()> {
    if state.running.lock().unwrap().contains_key(&id) {
        return Err("Das Profil läuft gerade. Bitte erst das Spiel beenden.".into());
    }
    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    state
        .launcher
        .packs()
        .rollback(&profile, &snapshot)
        .map_err(err)
}

#[tauri::command]
pub async fn pack_export(
    state: State<'_, AppState>,
    id: String,
    path: String,
    version: String,
) -> CmdResult<ExportReport> {
    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    state
        .launcher
        .packs()
        .export(&profile, Path::new(&path), version.trim())
        .await
        .map_err(err)
}
