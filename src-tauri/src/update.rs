//! Selbst-Update über das Tauri-Updater-Plugin.
//!
//! Updates sind signiert (`PINECONE_UPDATE_PUBKEY`). Woher sie kommen, legt eins von beiden fest
//! (beim Bauen gesetzt, werden sie einkompiliert):
//!
//! - `PINECONE_UPDATE_REPO` (`owner/repo`): Pinecone sucht in den GitHub-Releases die neueste
//!   veröffentlichte Version des gewählten Kanals (Release: ohne „Pre-release“, Beta: alle) und
//!   nimmt deren `latest.json`.
//! - `PINECONE_UPDATE_URL`: feste Adresse einer `latest.json`; `{channel}` wird durch `release`
//!   bzw. `beta` ersetzt.
//!
//! Ohne Schlüssel oder Quelle ist das Update abgeschaltet.

use std::sync::Mutex;

use pinecone_core::settings::UpdateChannel;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::AppState;
use crate::commands::{CmdResult, err};

const MANIFEST: &str = "latest.json";

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

/// z. B. `https://example.org/pinecone/{channel}.json`
fn endpoint() -> Option<String> {
    setting("PINECONE_UPDATE_URL", option_env!("PINECONE_UPDATE_URL"))
}

/// GitHub-Repo mit den Releases, z. B. `wheatley2056/twogg-pinecone`.
fn repo() -> Option<String> {
    setting("PINECONE_UPDATE_REPO", option_env!("PINECONE_UPDATE_REPO"))
}

pub fn configured() -> bool {
    pubkey().is_some() && (repo().is_some() || endpoint().is_some())
}

fn channel_name(channel: UpdateChannel) -> &'static str {
    match channel {
        UpdateChannel::Release => "release",
        UpdateChannel::Beta => "beta",
    }
}

#[derive(Deserialize)]
struct GhRelease {
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// `latest.json` der neuesten passenden Version. GitHub liefert die Releases neueste zuerst;
/// Entwürfe zählen nie, Pre-releases nur im Beta-Kanal.
fn pick_manifest(releases: &[GhRelease], channel: UpdateChannel) -> Option<&str> {
    releases
        .iter()
        .filter(|r| !r.draft && (channel == UpdateChannel::Beta || !r.prerelease))
        .find_map(|r| r.assets.iter().find(|a| a.name == MANIFEST))
        .map(|a| a.browser_download_url.as_str())
}

/// Adresse der `latest.json` für den Kanal.
async fn manifest_url(channel: UpdateChannel) -> Result<String, String> {
    if let Some(repo) = repo() {
        let url = format!("https://api.github.com/repos/{repo}/releases?per_page=30");
        let releases: Vec<GhRelease> = reqwest::Client::new()
            .get(&url)
            .header(
                "User-Agent",
                concat!("pinecone/", env!("CARGO_PKG_VERSION")),
            )
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("Update-Server nicht erreichbar: {e}"))?
            .json()
            .await
            .map_err(|e| format!("Antwort des Update-Servers unlesbar: {e}"))?;
        return pick_manifest(&releases, channel)
            .map(str::to_string)
            .ok_or_else(|| no_release(channel));
    }
    endpoint()
        .map(|u| u.replace("{channel}", channel_name(channel)))
        .ok_or_else(|| "Updates sind in dieser Version nicht eingerichtet".to_string())
}

fn no_release(channel: UpdateChannel) -> String {
    match channel {
        UpdateChannel::Release => {
            "Noch keine stabile Version veröffentlicht. Für Vorabversionen den Update-Kanal Beta wählen."
                .into()
        }
        UpdateChannel::Beta => "Noch keine Version veröffentlicht.".into(),
    }
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
    state: State<'_, AppState>,
    pending: State<'_, PendingUpdate>,
) -> CmdResult<Option<UpdateInfo>> {
    if pubkey().is_none() {
        return Err("Updates sind in dieser Version nicht eingerichtet".into());
    }
    let channel = state
        .launcher
        .settings()
        .load()
        .map(|s| s.update_channel)
        .unwrap_or_default();
    let url = manifest_url(channel).await?.parse().map_err(err)?;
    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(err)?
        .build()
        .map_err(err)?;
    let update = updater.check().await.map_err(|e| match e {
        // 404 auf latest.json: Es gibt noch kein veröffentlichtes Release (nur Entwürfe).
        tauri_plugin_updater::Error::ReleaseNotFound => no_release(channel),
        e => err(e),
    })?;
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
    state: State<'_, crate::AppState>,
    pending: State<'_, PendingUpdate>,
    on_event: Channel<UpdateEvent>,
) -> CmdResult<()> {
    // Nie während eines Spiels neu starten (AK-34.3); die Oberfläche holt es nach Spielende nach.
    if !state.running.lock().unwrap().is_empty() {
        return Err("Ein Spiel läuft gerade. Das Update wird nach Spielende installiert.".into());
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn release(draft: bool, prerelease: bool, tag: &str) -> GhRelease {
        GhRelease {
            draft,
            prerelease,
            assets: vec![
                GhAsset {
                    name: "setup.exe".into(),
                    browser_download_url: format!("https://x/{tag}/setup.exe"),
                },
                GhAsset {
                    name: MANIFEST.into(),
                    browser_download_url: format!("https://x/{tag}/latest.json"),
                },
            ],
        }
    }

    #[test]
    fn picks_by_channel() {
        let releases = vec![
            release(true, false, "v0.9.4"),
            release(false, true, "v0.9.3"),
            release(false, false, "v0.9.2"),
        ];
        assert_eq!(
            pick_manifest(&releases, UpdateChannel::Beta),
            Some("https://x/v0.9.3/latest.json")
        );
        assert_eq!(
            pick_manifest(&releases, UpdateChannel::Release),
            Some("https://x/v0.9.2/latest.json")
        );
        assert_eq!(pick_manifest(&releases[..2], UpdateChannel::Release), None);
    }
}
