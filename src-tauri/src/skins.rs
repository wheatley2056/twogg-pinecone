//! Befehle für Skins: Bibliothek, Editor, Skin und Cape des Microsoft-Accounts.

use std::path::Path;

use pinecone_core::auth::AccountKind;
use pinecone_core::msa::Profile;
use pinecone_core::skins::{SavedSkin, SkinModel, decode_data_url, encode_data_url};
use serde::Serialize;
use tauri::State;

use crate::AppState;
use crate::commands::{CmdResult, err};

#[tauri::command]
pub fn skins_list(state: State<'_, AppState>) -> CmdResult<Vec<SavedSkin>> {
    state.launcher.skins().list().map_err(err)
}

/// PNG eines Skins als `data:`-URL (für Vorschau und Editor).
#[tauri::command]
pub fn skin_data(state: State<'_, AppState>, id: String) -> CmdResult<String> {
    Ok(encode_data_url(
        &state.launcher.skins().png(&id).map_err(err)?,
    ))
}

#[tauri::command]
pub fn skin_import_file(
    state: State<'_, AppState>,
    path: String,
    model: SkinModel,
) -> CmdResult<SavedSkin> {
    state
        .launcher
        .skins()
        .import_file(Path::new(&path), None, model)
        .map_err(err)
}

#[tauri::command]
pub async fn skin_import_player(
    state: State<'_, AppState>,
    player: String,
) -> CmdResult<SavedSkin> {
    state
        .launcher
        .skins()
        .import_player(&player)
        .await
        .map_err(err)
}

/// Aus dem Editor speichern: mit `id` überschreiben, sonst neu anlegen.
#[tauri::command]
pub fn skin_save(
    state: State<'_, AppState>,
    id: Option<String>,
    name: String,
    model: SkinModel,
    data_url: String,
) -> CmdResult<SavedSkin> {
    let png = decode_data_url(&data_url).map_err(err)?;
    let lib = state.launcher.skins();
    match id {
        Some(id) => {
            lib.update(&id, model, &png).map_err(err)?;
            lib.rename(&id, &name).map_err(err)
        }
        None => lib.add(&name, model, &png).map_err(err),
    }
}

#[tauri::command]
pub fn skin_rename(state: State<'_, AppState>, id: String, name: String) -> CmdResult<SavedSkin> {
    state.launcher.skins().rename(&id, &name).map_err(err)
}

#[tauri::command]
pub fn skin_set_model(
    state: State<'_, AppState>,
    id: String,
    model: SkinModel,
) -> CmdResult<SavedSkin> {
    state.launcher.skins().set_model(&id, model).map_err(err)
}

#[tauri::command]
pub fn skin_delete(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state.launcher.skins().delete(&id).map_err(err)
}

/// Skin als PNG-Datei speichern.
#[tauri::command]
pub fn skin_export(state: State<'_, AppState>, id: String, path: String) -> CmdResult<()> {
    let png = state.launcher.skins().png(&id).map_err(err)?;
    std::fs::write(&path, png).map_err(err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapeDto {
    id: String,
    alias: Option<String>,
    active: bool,
    data_url: Option<String>,
}

/// Skin und Capes des aktiven Microsoft-Accounts (Texturen als `data:`-URL).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSkinDto {
    name: String,
    skin_url: Option<String>,
    skin_data_url: Option<String>,
    model: SkinModel,
    capes: Vec<CapeDto>,
}

async fn token(state: &AppState) -> CmdResult<String> {
    let active = state
        .accounts
        .load()
        .map_err(err)?
        .active_account()
        .cloned()
        .ok_or("Kein aktiver Account")?;
    if active.kind != AccountKind::Microsoft {
        return Err("Skins und Capes lassen sich nur mit einem Microsoft-Account ändern. Offline-Accounts nutzen Skins nur lokal.".into());
    }
    Ok(state
        .accounts
        .session(None, state.msa.as_ref())
        .await
        .map_err(err)?
        .access_token)
}

fn msa(state: &AppState) -> CmdResult<&pinecone_core::msa::MsaClient> {
    state
        .msa
        .as_ref()
        .ok_or_else(|| "Microsoft-Login ist nicht eingerichtet".to_string())
}

async fn account_dto(state: &AppState, profile: Profile) -> AccountSkinDto {
    let lib = state.launcher.skins();
    let active = profile.active_skin().cloned();
    let skin_data_url = match &active {
        Some(s) => lib.texture_data_url(&s.url).await.ok(),
        None => None,
    };
    let mut capes = Vec::new();
    for c in profile.capes {
        capes.push(CapeDto {
            data_url: lib.texture_data_url(&c.url).await.ok(),
            active: c.state == "ACTIVE",
            alias: c.alias,
            id: c.id,
        });
    }
    AccountSkinDto {
        name: profile.name,
        model: match active.as_ref().and_then(|s| s.variant.as_deref()) {
            Some(v) if v.eq_ignore_ascii_case("slim") => SkinModel::Slim,
            _ => SkinModel::Classic,
        },
        skin_url: active.map(|s| s.url),
        skin_data_url,
        capes,
    }
}

#[tauri::command]
pub async fn skin_account(state: State<'_, AppState>) -> CmdResult<AccountSkinDto> {
    let msa = msa(&state)?;
    let token = token(&state).await?;
    let profile = msa.profile(&token).await.map_err(err)?;
    Ok(account_dto(&state, profile).await)
}

/// Skin aus der Bibliothek als Skin des aktiven Accounts setzen.
#[tauri::command]
pub async fn skin_apply(state: State<'_, AppState>, id: String) -> CmdResult<AccountSkinDto> {
    let msa = msa(&state)?;
    let lib = state.launcher.skins();
    let skin = lib
        .list()
        .map_err(err)?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or("Skin nicht gefunden")?;
    let png = lib.png(&id).map_err(err)?;
    let token = token(&state).await?;
    let profile = msa
        .upload_skin(&token, skin.model, png)
        .await
        .map_err(err)?;
    Ok(account_dto(&state, profile).await)
}

#[tauri::command]
pub async fn skin_set_cape(
    state: State<'_, AppState>,
    cape_id: Option<String>,
) -> CmdResult<AccountSkinDto> {
    let msa = msa(&state)?;
    let token = token(&state).await?;
    let profile = msa
        .set_cape(&token, cape_id.as_deref())
        .await
        .map_err(err)?;
    Ok(account_dto(&state, profile).await)
}

/// Aktiven Skin des Accounts in die Bibliothek übernehmen.
#[tauri::command]
pub async fn skin_import_account(state: State<'_, AppState>) -> CmdResult<SavedSkin> {
    let msa = msa(&state)?;
    let token = token(&state).await?;
    let profile = msa.profile(&token).await.map_err(err)?;
    let skin = profile
        .active_skin()
        .ok_or("Der Account nutzt den Standard-Skin")?;
    let model = match skin.variant.as_deref() {
        Some(v) if v.eq_ignore_ascii_case("slim") => SkinModel::Slim,
        _ => SkinModel::Classic,
    };
    state
        .launcher
        .skins()
        .import_texture(&skin.url, &profile.name, model)
        .await
        .map_err(err)
}
