//! Instanzen als abonnierbares Pack mit Freunden teilen (`pinecone://pack/<id>`). Braucht die
//! Freunde-Anmeldung mit dem Microsoft-Konto; der Pinecone-Server speichert die Versionen.

use pinecone_core::auth::AccountKind;
use pinecone_core::sharedpacks::{self, PublishRequest, PublishStore, Published};
use pinecone_core::twogg::SharedPack;
use serde::Serialize;
use tauri::State;

use crate::AppState;
use crate::commands::{CmdResult, err};
use crate::friends::with_session;

fn store(state: &AppState) -> PublishStore {
    PublishStore::new(state.launcher.paths().root())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedStatusDto {
    /// `false` ohne Microsoft-Konto: Teilen geht nur mit Freunde-Anmeldung.
    available: bool,
    /// Diese Instanz ist schon veröffentlicht.
    published: Option<Published>,
    /// Stand auf dem Server (Versionen, Ablauf); `None`, wenn nicht abrufbar oder gelöscht.
    remote: Option<SharedPack>,
    suggested_version: String,
}

#[tauri::command]
pub async fn shared_pack_status(
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<SharedStatusDto> {
    let available = state
        .accounts
        .load()
        .map_err(err)?
        .active_account()
        .is_some_and(|a| a.kind == AccountKind::Microsoft);
    let published = store(&state).get(&id);
    let remote = match (&published, available) {
        (Some(p), true) => {
            let r: CmdResult<Option<SharedPack>> = async {
                let packs = state.launcher.packs();
                let client = packs.twogg().ok_or("TwoGG-Server ist nicht eingerichtet")?;
                with_session!(&state, |token| client.shared_pack(&token, &p.pack_id))
            }
            .await;
            r.unwrap_or_else(|e| {
                tracing::warn!("Geteiltes Pack {} nicht abrufbar: {e}", p.pack_id);
                None
            })
        }
        _ => None,
    };
    // Vorschlag: nach der zuletzt veröffentlichten Version, sonst 1.0.
    let suggested_version =
        sharedpacks::next_version(published.as_ref().map(|p| p.version.as_str()));
    Ok(SharedStatusDto {
        available,
        published,
        remote,
        suggested_version,
    })
}

#[tauri::command]
pub async fn shared_pack_publish(
    state: State<'_, AppState>,
    id: String,
    request: PublishRequest,
) -> CmdResult<SharedPack> {
    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    let packs = state.launcher.packs();
    let store = store(&state);
    let cache = state.launcher.paths().root().join("cache").join("publish");
    with_session!(&state, |token| sharedpacks::publish(
        packs, &store, &cache, &token, &profile, &request
    ))
    .map_err(|e| {
        if e == "OFFLINE" {
            "Zum Teilen mit Freunden brauchst du ein Microsoft-Konto.".to_string()
        } else {
            e
        }
    })
}

/// Pack auf dem Server löschen. Freunde behalten ihre Instanzen ohne weitere Updates.
#[tauri::command]
pub async fn shared_pack_unpublish(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let packs = state.launcher.packs();
    let store = store(&state);
    with_session!(&state, |token| sharedpacks::unpublish(
        packs, &store, &token, &id
    ))
}
