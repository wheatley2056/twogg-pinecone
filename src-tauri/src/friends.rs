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

pub(crate) fn key(uuid: &str) -> String {
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

/// Eigener Freundescode (F-57); `fresh` erzeugt einen neuen.
#[tauri::command]
pub async fn friends_code(state: State<'_, AppState>, fresh: bool) -> CmdResult<String> {
    let c = client(&state)?;
    with_session!(&state, |token| c.friend_code(&token, fresh))
}

#[derive(Serialize)]
pub struct CodeRequestDto {
    status: String,
    name: String,
}

/// Anfrage per Freundescode oder `pinecone://friend/<code>`.
#[tauri::command]
pub async fn friends_request_code(
    state: State<'_, AppState>,
    code: String,
) -> CmdResult<CodeRequestDto> {
    let c = client(&state)?;
    let (status, name) = with_session!(&state, |token| c.friend_request_code(&token, code.trim()))?;
    Ok(CodeRequestDto { status, name })
}

/// Freund auf den eigenen Server einladen (F-56). `pack`: Teilen-Link der Instanz, falls vorhanden.
#[tauri::command]
pub async fn friends_invite(
    state: State<'_, AppState>,
    to: String,
    server: String,
    pack: Option<String>,
    instance: Option<String>,
) -> CmdResult<()> {
    let c = client(&state)?;
    with_session!(&state, |token| c.invite(
        &token,
        &to,
        &server,
        pack.as_deref(),
        instance.as_deref()
    ))
}

#[tauri::command]
pub async fn friends_invite_answer(
    state: State<'_, AppState>,
    id: String,
    accept: bool,
) -> CmdResult<()> {
    let c = client(&state)?;
    with_session!(&state, |token| c.invite_answer(&token, &id, accept))
}

/// Sync-Pack teilen (F-58): ohne eigene Dateien, Link `pinecone://sync/<code>` für 30 Tage.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncShareDto {
    link: String,
    /// Eigene Dateien, die nicht mitgeteilt werden.
    skipped_files: usize,
}

#[tauri::command]
pub async fn syncpack_share(state: State<'_, AppState>, id: String) -> CmdResult<SyncShareDto> {
    use pinecone_core::syncpacks::SharedPack;
    let pack = state.launcher.syncpacks().get(&id).map_err(err)?;
    let data = serde_json::to_value(SharedPack::from_pack(&pack)).map_err(err)?;
    let c = client(&state)?;
    let code = with_session!(&state, |token| c.sync_share(&token, &data))?;
    Ok(SyncShareDto {
        link: format!("pinecone://sync/{code}"),
        skipped_files: SharedPack::skipped_files(&pack),
    })
}

#[derive(Serialize)]
pub struct SharedSyncDto {
    owner: String,
    pack: pinecone_core::syncpacks::SharedPack,
}

fn share_code(input: &str) -> String {
    input
        .trim()
        .trim_start_matches("pinecone://sync/")
        .trim_end_matches('/')
        .to_ascii_uppercase()
}

/// Vorschau eines geteilten Sync-Packs: alle Werte, bevor etwas übernommen wird (AK-58.2).
#[tauri::command]
pub async fn syncpack_share_preview(
    state: State<'_, AppState>,
    code: String,
) -> CmdResult<SharedSyncDto> {
    let c = client(&state)?;
    let code = share_code(&code);
    let shared = with_session!(&state, |token| c.sync_share_get(&token, &code))?;
    Ok(SharedSyncDto {
        owner: shared.owner,
        pack: pinecone_core::syncpacks::SharedPack::parse(&shared.data).map_err(err)?,
    })
}

#[tauri::command]
pub async fn syncpack_share_import(
    state: State<'_, AppState>,
    code: String,
) -> CmdResult<pinecone_core::syncpacks::SyncPack> {
    let preview = syncpack_share_preview(state.clone(), code).await?;
    state
        .launcher
        .syncpacks()
        .import_shared(&preview.pack)
        .map_err(err)
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
