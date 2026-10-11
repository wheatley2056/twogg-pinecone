//! Selbst-Update über das Tauri-Updater-Plugin.
//!
//! Updates sind signiert. Öffentlicher Schlüssel und Update-Adresse kommen aus
//! `PINECONE_UPDATE_PUBKEY` und `PINECONE_UPDATE_URL` (beim Bauen gesetzt werden sie
//! einkompiliert). Ohne beide ist das Update abgeschaltet.

use std::sync::Mutex;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::commands::{CmdResult, err};

fn setting(name: &str, built_in: Option<&'static str>) -> Option<String> {
    std::env::var(name)
        .ok()
        .or_else(|| built_in.map(str::to_string))
        .filter(|v| !v.trim().is_empty())
}

pub fn pubkey() -> Option<String> {
    setting(
        "PINECONE_UPDATE_PUBKEY",
        option_env!("PINECONE_UPDATE_PUBKEY"),
    )
}

/// z. B. `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`
pub fn endpoint() -> Option<String> {
    setting("PINECONE_UPDATE_URL", option_env!("PINECONE_UPDATE_URL"))
}

pub fn configured() -> bool {
    pubkey().is_some() && endpoint().is_some()
}

/// Gefundenes, noch nicht installiertes Update.
#[derive(Default)]
pub struct PendingUpdate(pub Mutex<Option<Update>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    current_version: String,
    /// Unix-Zeit der Veröffentlichung.
    date: Option<i64>,
    notes: Option<String>,
}

#[tauri::command]
pub async fn update_check(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> CmdResult<Option<UpdateInfo>> {
    let (Some(_), Some(url)) = (pubkey(), endpoint()) else {
        return Err("Updates sind in dieser Version nicht eingerichtet".into());
    };
    let url = url.parse().map_err(err)?;
    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(err)?
        .build()
        .map_err(err)?;
    let update = updater.check().await.map_err(err)?;
    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        date: u.date.map(|d| d.unix_timestamp()),
        notes: u.body.clone(),
    });
    *pending.0.lock().unwrap() = update;
    Ok(info)
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UpdateEvent {
    Progress { done: u64, total: Option<u64> },
    Installing,
}

/// Lädt das zuvor gefundene Update, prüft die Signatur, installiert es und startet neu.
/// Laufende Spiele laufen weiter (sie hängen nicht am Launcher-Prozess).
#[tauri::command]
pub async fn update_install(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
    on_event: Channel<UpdateEvent>,
) -> CmdResult<()> {
    let update = pending
        .0
        .lock()
        .unwrap()
        .take()
        .ok_or("Kein Update gefunden. Bitte erneut prüfen.")?;
    tracing::info!(
        "Update von {} auf {} wird installiert",
        update.current_version,
        update.version
    );
    let mut done = 0u64;
    update
        .download_and_install(
            |chunk, total| {
                done += chunk as u64;
                let _ = on_event.send(UpdateEvent::Progress { done, total });
            },
            || {
                let _ = on_event.send(UpdateEvent::Installing);
            },
        )
        .await
        .map_err(err)?;
    app.restart()
}
