//! Screenshot-Galerie über alle Profile: Liste, Vorschaubilder, Aktionen und ein Watcher,
//! der die Oberfläche über neue oder gelöschte Screenshots informiert.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use pinecone_core::screenshots;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::AppState;
use crate::commands::{CmdResult, err};

/// Ereignis an die Oberfläche, wenn sich Screenshots geändert haben.
pub const EVENT: &str = "screenshots-changed";

pub fn thumbs_dir(state: &AppState) -> PathBuf {
    state.launcher.paths().root().join("cache").join("thumbs")
}

fn instances(state: &AppState) -> PathBuf {
    state.launcher.paths().root().join("instances")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotDto {
    profile_id: String,
    profile_name: String,
    path: String,
    modified: u64,
    size: u64,
}

#[tauri::command]
pub fn screenshots_list(state: State<'_, AppState>) -> CmdResult<Vec<ScreenshotDto>> {
    let launcher = &state.launcher;
    let profiles = launcher.profiles().list().map_err(err)?;
    let dirs: Vec<(String, PathBuf)> = profiles
        .iter()
        .map(|p| (p.id.clone(), launcher.game_dir(p)))
        .collect();
    let all = screenshots::latest(
        dirs.iter().map(|(id, d)| (id.as_str(), d.clone())),
        usize::MAX,
    );
    Ok(all
        .into_iter()
        .map(|s| ScreenshotDto {
            profile_name: profiles
                .iter()
                .find(|p| p.id == s.profile_id)
                .map(|p| p.name.clone())
                .unwrap_or_default(),
            profile_id: s.profile_id,
            path: s.path.display().to_string(),
            modified: s.modified,
            size: s.size,
        })
        .collect())
}

/// Pfad des Vorschaubilds (wird bei Bedarf erzeugt).
#[tauri::command]
pub async fn screenshot_thumb(state: State<'_, AppState>, path: String) -> CmdResult<String> {
    let real = screenshots::checked(Path::new(&path), &instances(&state)).map_err(err)?;
    let cache = thumbs_dir(&state);
    tauri::async_runtime::spawn_blocking(move || screenshots::thumbnail(&real, &cache))
        .await
        .map_err(err)?
        .map(|p| p.display().to_string())
        .map_err(err)
}

#[tauri::command]
pub fn screenshot_delete(state: State<'_, AppState>, path: String) -> CmdResult<()> {
    screenshots::delete(Path::new(&path), &instances(&state)).map_err(err)
}

/// Im Dateimanager zeigen.
#[tauri::command]
pub fn screenshot_reveal(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CmdResult<()> {
    let real = screenshots::checked(Path::new(&path), &instances(&state)).map_err(err)?;
    app.opener().reveal_item_in_dir(real).map_err(err)
}

/// Kopie an einem gewählten Ort speichern (zum Teilen).
#[tauri::command]
pub fn screenshot_save_as(
    state: State<'_, AppState>,
    path: String,
    target: String,
) -> CmdResult<()> {
    let real = screenshots::checked(Path::new(&path), &instances(&state)).map_err(err)?;
    std::fs::copy(real, target).map(|_| ()).map_err(err)
}

/// Startet den Watcher: beobachtet die Screenshot-Ordner aller Profile (nicht rekursiv) und
/// prüft alle paar Sekunden, ob neue Ordner dazugekommen sind (neue Profile, erster Screenshot).
pub fn start_watcher(app: &AppHandle) {
    let state = app.state::<AppState>();
    let root = instances(&state);
    let cache = thumbs_dir(&state);
    // Alte Vorschaubilder aufräumen (werden bei Bedarf neu erzeugt).
    screenshots::prune_thumbnails(&cache, 60 * 24 * 3600);
    let app = app.clone();
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("Screenshot-Watcher nicht gestartet: {e}");
                return;
            }
        };
        let mut watched: HashSet<PathBuf> = HashSet::new();
        loop {
            // Neue Screenshot-Ordner aufnehmen, verschwundene vergessen.
            let current: HashSet<PathBuf> = std::fs::read_dir(&root)
                .map(|d| {
                    d.flatten()
                        .map(|e| e.path().join(".minecraft").join("screenshots"))
                        .filter(|p| p.is_dir())
                        .collect()
                })
                .unwrap_or_default();
            for dir in current.difference(&watched) {
                if let Err(e) = watcher.watch(dir, RecursiveMode::NonRecursive) {
                    tracing::warn!("{} nicht beobachtet: {e}", dir.display());
                }
            }
            for dir in watched.difference(&current) {
                let _ = watcher.unwatch(dir);
            }
            let added = current.difference(&watched).count();
            watched = current;
            if added > 0 && !watched.is_empty() {
                // Ein neuer Ordner heißt meist: erster Screenshot in einem Profil.
                let _ = app.emit(EVENT, ());
            }

            // Ereignisse sammeln, kurz entprellen (Minecraft schreibt in mehreren Schritten).
            let mut changed = false;
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
                match rx.recv_timeout(left) {
                    Ok(Ok(event)) => {
                        changed |= event
                            .paths
                            .iter()
                            .any(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")));
                        if changed {
                            std::thread::sleep(Duration::from_millis(400));
                            while rx.try_recv().is_ok() {}
                            let _ = app.emit(EVENT, ());
                            changed = false;
                        }
                    }
                    Ok(Err(e)) => tracing::warn!("Screenshot-Watcher: {e}"),
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        }
    });
}
