//! twogg.app: Events, Konto-Verknüpfung (Discord über eigenes Fenster oder E-Mail), Anmeldungen,
//! Server-Adressen und Freunde.

use pinecone_core::twogg_app::{
    AppEvent, AppFriend, AppUser, JoinInfo, Participation, handoff_error, handoff_token,
};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::AppState;
use crate::commands::{CmdResult, err};

const LOGIN_WINDOW: &str = "twogg-login";

#[tauri::command]
pub async fn twogg_app_events(state: State<'_, AppState>) -> CmdResult<Vec<AppEvent>> {
    state.twogg_app.events().await.map_err(err)
}

/// Verknüpftes Konto, sonst `null` (auch wenn die Anmeldung abgelaufen ist).
#[tauri::command]
pub async fn twogg_app_me(state: State<'_, AppState>) -> CmdResult<Option<AppUser>> {
    if !state.twogg_app.linked() {
        return Ok(None);
    }
    match state.twogg_app.me().await {
        Ok(u) => Ok(Some(u)),
        Err(pinecone_core::Error::Auth(_)) => Ok(None),
        Err(e) => Err(err(e)),
    }
}

/// Öffnet die Discord-Anmeldung von twogg.app in einem eigenen Fenster. Die Weiterleitung auf
/// `twogg://auth/handoff` wird abgefangen; das Ergebnis kommt als Event `twogg-app-login`
/// (`{ user }` oder `{ error }`).
#[tauri::command]
pub async fn twogg_app_login_discord(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    if let Some(w) = app.get_webview_window(LOGIN_WINDOW) {
        let _ = w.set_focus();
        return Ok(());
    }
    let url = state.twogg_app.discord_login_url();
    let url = url.parse().map_err(err)?;
    let handle = app.clone();
    WebviewWindowBuilder::new(&app, LOGIN_WINDOW, WebviewUrl::External(url))
        .title("Mit twogg.app verbinden")
        .inner_size(520.0, 760.0)
        .on_navigation(move |url| {
            if url.scheme() != "twogg" {
                // Nur https-Seiten (Discord, twogg.app) zulassen.
                return url.scheme() == "https";
            }
            let link = url.to_string();
            let app = handle.clone();
            tauri::async_runtime::spawn(async move {
                let result = match (handoff_token(&link), handoff_error(&link)) {
                    (Some(token), _) => {
                        let state = app.state::<AppState>();
                        state.twogg_app.redeem_handoff(&token).await.map_err(err)
                    }
                    (None, Some(e)) => Err(format!("Anmeldung abgebrochen ({e})")),
                    (None, None) => Err("Unerwartete Antwort von twogg.app".into()),
                };
                let payload = match result {
                    Ok(user) => serde_json::json!({ "user": user }),
                    Err(e) => serde_json::json!({ "error": e }),
                };
                let _ = app.emit("twogg-app-login", payload);
                if let Some(w) = app.get_webview_window(LOGIN_WINDOW) {
                    let _ = w.close();
                }
            });
            false
        })
        .build()
        .map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn twogg_app_login_password(
    state: State<'_, AppState>,
    email: String,
    password: String,
) -> CmdResult<AppUser> {
    state
        .twogg_app
        .login_password(email.trim(), &password)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn twogg_app_logout(state: State<'_, AppState>) -> CmdResult<()> {
    state.twogg_app.logout().await.map_err(err)
}

#[tauri::command]
pub async fn twogg_app_participations(state: State<'_, AppState>) -> CmdResult<Vec<Participation>> {
    state.twogg_app.participations().await.map_err(err)
}

#[tauri::command]
pub async fn twogg_app_join(state: State<'_, AppState>, event_id: String) -> CmdResult<JoinInfo> {
    // Event-Beitritt nur ab der Mindestversion des Pinecone-Servers (F-60, AK-60.2).
    if let Some(client) = state.launcher.packs().twogg() {
        client.require_launcher_version().await.map_err(err)?;
    }
    state.twogg_app.join_info(&event_id).await.map_err(err)
}

#[tauri::command]
pub async fn twogg_app_friends(state: State<'_, AppState>) -> CmdResult<Vec<AppFriend>> {
    state.twogg_app.friends().await.map_err(err)
}
