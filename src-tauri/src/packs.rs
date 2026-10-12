//! Befehle für Modpacks: installieren, importieren (mit Vorschau), Updates mit Konfliktauswahl,
//! Rollback, Export und Teilen.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pinecone_core::modpack::{
    ExportOptions, ExportReport, ImportResult, ManualDownload, PackPreview, PackSource, PackUpdate,
    Resolution, SnapshotInfo, UpdatePlan,
};
use pinecone_core::share::ShareLink;
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
    let result = packs.import(&archive, source, None).await.map_err(err)?;
    crate::syncpacks::apply_auto(&state, &result.profile).await;
    Ok(result.into())
}

#[tauri::command]
pub async fn pack_install_twogg(
    state: State<'_, AppState>,
    pack: String,
    version: Option<String>,
) -> CmdResult<ImportDto> {
    let packs = state.launcher.packs();
    let (archive, source) = packs
        .download_twogg(&pack, version.as_deref())
        .await
        .map_err(err)?;
    let result = packs.import(&archive, source, None).await.map_err(err)?;
    crate::syncpacks::apply_auto(&state, &result.profile).await;
    Ok(result.into())
}

/// TwoGG-Modpacks vom Pinecone-Server (öffentliche Liste).
#[tauri::command]
pub async fn twogg_packs(state: State<'_, AppState>) -> CmdResult<Vec<pinecone_core::twogg::Pack>> {
    let client = state
        .launcher
        .packs()
        .twogg()
        .ok_or("TwoGG-Server ist nicht eingerichtet")?;
    client.packs().await.map_err(err)
}

#[tauri::command]
pub async fn twogg_pack(
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<pinecone_core::twogg::Pack> {
    let client = state
        .launcher
        .packs()
        .twogg()
        .ok_or("TwoGG-Server ist nicht eingerichtet")?;
    client.pack(&id).await.map_err(err)
}

/// Neuigkeiten der TwoGG-Community für die Startseite.
#[tauri::command]
pub async fn twogg_news(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> CmdResult<Vec<pinecone_core::twogg::NewsItem>> {
    let client = state
        .launcher
        .packs()
        .twogg()
        .ok_or("TwoGG-Server ist nicht eingerichtet")?;
    client.news(limit.unwrap_or(5)).await.map_err(err)
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
    let result = packs.import(&archive, source, None).await.map_err(err)?;
    crate::syncpacks::apply_auto(&state, &result.profile).await;
    Ok(result.into())
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
            source: source_name(&p.source),
        }))
}

pub fn source_name(source: &PackSource) -> &'static str {
    match source {
        PackSource::Modrinth { .. } => "modrinth",
        PackSource::Curseforge { .. } => "curseforge",
        PackSource::Twogg { .. } => "twogg",
        PackSource::File => "file",
    }
}

/// Vorschau vor dem Import. Installiert wird erst mit `pack_import_pending(token)`, und nur,
/// was vorher angezeigt wurde.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDto {
    token: String,
    preview: PackPreview,
    /// `modrinth`, `curseforge` oder `file`.
    source: &'static str,
}

async fn preview(state: &AppState, archive: PathBuf, source: PackSource) -> CmdResult<PreviewDto> {
    // Vollständiger Instanz-Export (F-54) statt Modpack: eigene Vorschau, Import ohne Downloads.
    let instance = matches!(source, PackSource::File) && {
        let a = archive.clone();
        tauri::async_runtime::spawn_blocking(move || {
            pinecone_core::archive::is_instance_archive(&a)
        })
        .await
        .unwrap_or(false)
    };
    let preview = if instance {
        let a = archive.clone();
        tauri::async_runtime::spawn_blocking(move || pinecone_core::archive::preview(&a))
            .await
            .map_err(err)?
            .map_err(err)?
    } else {
        state
            .launcher
            .packs()
            .preview(&archive)
            .await
            .map_err(err)?
    };
    let token = archive.display().to_string();
    let dto = PreviewDto {
        token: token.clone(),
        preview,
        source: if instance {
            "instance"
        } else {
            source_name(&source)
        },
    };
    state
        .pending_imports
        .lock()
        .unwrap()
        .insert(token, (archive, source));
    Ok(dto)
}

#[tauri::command]
pub async fn pack_preview_file(state: State<'_, AppState>, path: String) -> CmdResult<PreviewDto> {
    preview(&state, PathBuf::from(path), PackSource::File).await
}

/// Lädt das Pack hinter einem `pinecone://`- oder Modrinth-Link und zeigt die Vorschau.
#[tauri::command]
pub async fn pack_preview_link(state: State<'_, AppState>, link: String) -> CmdResult<PreviewDto> {
    let packs = state.launcher.packs();
    let (archive, source) = match ShareLink::parse(&link).map_err(err)? {
        ShareLink::Modrinth { project, version } => packs
            .download_modrinth(&project, version.as_deref())
            .await
            .map_err(err)?,
        ShareLink::Curseforge { project, file } => packs
            .download_curseforge(project, file, None)
            .await
            .map_err(err)?,
        ShareLink::Twogg { pack, version } => packs
            .download_twogg(&pack, version.as_deref())
            .await
            .map_err(err)?,
        ShareLink::Code(_) => {
            return Err("Share-Codes funktionieren erst mit dem Share-Dienst. Bitte das Pack als Datei schicken lassen.".into());
        }
    };
    preview(&state, archive, source).await
}

#[tauri::command]
pub async fn pack_import_pending(
    state: State<'_, AppState>,
    token: String,
    name: Option<String>,
) -> CmdResult<ImportDto> {
    let (archive, source) = state
        .pending_imports
        .lock()
        .unwrap()
        .remove(&token)
        .ok_or("Keine Vorschau zu diesem Modpack. Bitte erneut öffnen.")?;
    let name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    if matches!(source, PackSource::File) && pinecone_core::archive::is_instance_archive(&archive) {
        let profiles = state.launcher.profiles().clone();
        let profile = tauri::async_runtime::spawn_blocking(move || {
            pinecone_core::archive::import(&profiles, &archive, name)
        })
        .await
        .map_err(err)?
        .map_err(err)?;
        return Ok(ImportDto {
            profile: ProfileDto::new(profile, false),
            missing: Vec::new(),
        });
    }
    let result = state
        .launcher
        .packs()
        .import(&archive, source, name)
        .await
        .map_err(err)?;
    crate::syncpacks::apply_auto(&state, &result.profile).await;
    Ok(result.into())
}

/// Vorschau geschlossen, ohne zu installieren.
#[tauri::command]
pub fn pack_preview_cancel(state: State<'_, AppState>, token: String) {
    state.pending_imports.lock().unwrap().remove(&token);
}

/// Link auf genau die installierte Pack-Version, falls es eine Online-Quelle gibt.
#[tauri::command]
pub fn pack_share_link(state: State<'_, AppState>, id: String) -> CmdResult<Option<String>> {
    Ok(state
        .launcher
        .packs()
        .pack_info(&id)
        .map_err(err)?
        .and_then(|p| ShareLink::for_source(&p.source))
        .map(|l| l.to_string()))
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

/// Pack-Logos für ältere Pack-Instanzen nachladen; Anzahl der neuen Symbole.
#[tauri::command]
pub async fn pack_fill_icons(state: State<'_, AppState>) -> CmdResult<usize> {
    state
        .launcher
        .packs()
        .fill_missing_icons()
        .await
        .map_err(err)
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
        (None, PackSource::Twogg { pack_id, .. }) => {
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
                .download_twogg(pack_id, Some(&target))
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

/// Vollständiger Instanz-Export als `.pinecone.zip` (F-54).
#[tauri::command]
pub async fn instance_export(
    state: State<'_, AppState>,
    id: String,
    path: String,
    options: pinecone_core::archive::ExportOptions,
) -> CmdResult<()> {
    let profiles = state.launcher.profiles().clone();
    tauri::async_runtime::spawn_blocking(move || {
        pinecone_core::archive::export(&profiles, &id, Path::new(&path), options)
    })
    .await
    .map_err(err)?
    .map_err(err)
}

/// Ungefähre Größe eines vollständigen Exports in Bytes.
#[tauri::command]
pub async fn instance_export_size(
    state: State<'_, AppState>,
    id: String,
    options: pinecone_core::archive::ExportOptions,
) -> CmdResult<u64> {
    let profiles = state.launcher.profiles().clone();
    tauri::async_runtime::spawn_blocking(move || {
        pinecone_core::archive::export_size(&profiles, &id, options)
    })
    .await
    .map_err(err)?
    .map_err(err)
}

#[tauri::command]
pub async fn pack_export(
    state: State<'_, AppState>,
    id: String,
    path: String,
    options: ExportOptions,
) -> CmdResult<ExportReport> {
    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    state
        .launcher
        .packs()
        .export(&profile, Path::new(&path), &options)
        .await
        .map_err(err)
}
