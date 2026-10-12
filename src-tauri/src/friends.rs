//! Pinecone-Freundesliste. Angemeldet wird mit dem aktiven Microsoft-Account (Mojang-Sitzung);
//! den Sitzungsschlüssel des Pinecone-Servers merkt sich der Schlüsselbund je Account.

use pinecone_core::Error;
use pinecone_core::accounts::{KeyringStore, SecretStore};
use pinecone_core::auth::AccountKind;
use pinecone_core::twogg::{Activity, FriendList, ServerInfo, TwoggClient};
use serde::Serialize;
use tauri::State;

use crate::AppState;
use crate::commands::{CmdResult, err};

fn client(state: &AppState) -> CmdResult<&TwoggClient> {
    state
        .launcher
        .packs()
        .twogg()
        .ok_or_else(|| "TwoGG-Server ist nicht eingerichtet".to_string())
}

fn key(uuid: &str) -> String {
    format!("pinecone-friends-{uuid}")
}

/// Sitzung am Pinecone-Server für den aktiven Account; `fresh` erzwingt eine neue Anmeldung.
async fn session(state: &AppState, fresh: bool) -> CmdResult<String> {
    let active = state
        .accounts
        .load()
        .map_err(err)?
        .active_account()
        .cloned()
        .ok_or("Kein aktiver Account")?;
    if active.kind != AccountKind::Microsoft {
        return Err("OFFLINE".into());
    }
    let uuid = active.uuid.simple().to_string();
    if !fresh && let Ok(Some(token)) = KeyringStore.get(&key(&uuid)) {
        return Ok(token);
    }
    let account = state
        .accounts
        .session(None, state.msa.as_ref())
        .await
        .map_err(err)?;
    let session = client(state)?
        .player_login(&account.access_token, &uuid, &account.name)
        .await
        .map_err(err)?;
    KeyringStore.set(&key(&uuid), &session.token).map_err(err)?;
    Ok(session.token)
}

/// Führt eine Anfrage aus; ist die Sitzung abgelaufen, einmal neu anmelden.
macro_rules! with_session {
    ($state:expr, |$token:ident| $body:expr) => {{
        let $token = session($state, false).await?;
        match $body.await {
            Err(Error::Auth(_)) => {
                let $token = session($state, true).await?;
                $body.await.map_err(err)
            }
            other => other.map_err(err),
        }
    }};
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendsDto {
    /// `false` bei Offline-Accounts: Freunde gibt es nur mit Microsoft-Account.
    pub available: bool,
    #[serde(flatten)]
    pub list: Option<FriendList>,
}

#[tauri::command]
pub async fn friends_list(state: State<'_, AppState>) -> CmdResult<FriendsDto> {
    let c = client(&state)?;
    match session(&state, false).await {
        Err(e) if e == "OFFLINE" => {
            return Ok(FriendsDto {
                available: false,
                list: None,
            });
        }
        Err(e) => return Err(e),
        Ok(_) => {}
    }
    let list = with_session!(&state, |token| c.friends(&token))?;
    Ok(FriendsDto {
        available: true,
        list: Some(list),
    })
}

#[tauri::command]
pub async fn friends_presence(state: State<'_, AppState>, activity: Activity) -> CmdResult<()> {
    let c = client(&state)?;
    with_session!(&state, |token| c.presence(&token, &activity))
}

#[tauri::command]
pub async fn friends_request(state: State<'_, AppState>, name: String) -> CmdResult<String> {
    let c = client(&state)?;
    with_session!(&state, |token| c.friend_request(&token, name.trim()))
}

#[tauri::command]
pub async fn friends_accept(state: State<'_, AppState>, uuid: String) -> CmdResult<()> {
    let c = client(&state)?;
    with_session!(&state, |token| c.friend_accept(&token, &uuid))
}

#[tauri::command]
pub async fn friends_decline(state: State<'_, AppState>, uuid: String) -> CmdResult<()> {
    let c = client(&state)?;
    with_session!(&state, |token| c.friend_decline(&token, &uuid))
}

#[tauri::command]
pub async fn friends_remove(state: State<'_, AppState>, uuid: String) -> CmdResult<()> {
    let c = client(&state)?;
    with_session!(&state, |token| c.friend_remove(&token, &uuid))
}

/// Eigenes Spielerprofil auf dem Pinecone-Server löschen (Freunde, Anfragen, Status) und die
/// Sitzung vergessen. Beim nächsten Öffnen der Freundesliste entsteht ein neues, leeres Profil.
#[tauri::command]
pub async fn friends_delete_data(state: State<'_, AppState>) -> CmdResult<()> {
    let c = client(&state)?;
    with_session!(&state, |token| c.delete_player(&token))?;
    if let Some(active) = state.accounts.load().map_err(err)?.active_account() {
        KeyringStore
            .delete(&key(&active.uuid.simple().to_string()))
            .map_err(err)?;
    }
    Ok(())
}

/// TwoGG-Auswahl für eine Inhaltsart (leer, wenn der Server nicht erreichbar ist).
#[tauri::command]
pub async fn twogg_recommendations(
    state: State<'_, AppState>,
    project_type: String,
) -> CmdResult<Vec<pinecone_core::twogg::Recommendation>> {
    client(&state)?
        .recommendations(&project_type)
        .await
        .map_err(err)
}

/// Angaben des Pinecone-Servers (z. B. Community-Server für den Schnellbeitritt).
#[tauri::command]
pub async fn twogg_server_info(state: State<'_, AppState>) -> CmdResult<ServerInfo> {
    client(&state)?.info().await.map_err(err)
}
