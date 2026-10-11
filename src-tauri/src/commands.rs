//! Tauri-Befehle für Profile, Versionen, Accounts und Java. Fehler gehen als Text an die Oberfläche.

use std::path::PathBuf;
use std::sync::Arc;

use pinecone_core::auth::AccountKind;
use pinecone_core::loader::{LoaderKind, LoaderSpec};
use pinecone_core::profiles::Profile;
use tauri::State;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Notify;

use crate::AppState;
use crate::dto::*;

pub type CmdResult<T> = Result<T, String>;

pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        data_dir: state.launcher.paths().root().display().to_string(),
        msa_configured: state.msa.is_some(),
        curseforge_configured: state.launcher.content().curseforge().is_some(),
        updates_configured: crate::update::configured(),
    }
}

#[tauri::command]
pub async fn list_versions(
    state: State<'_, AppState>,
    snapshots: bool,
) -> CmdResult<Vec<VersionDto>> {
    let manifest = state.launcher.installer().manifest().await.map_err(err)?;
    Ok(manifest
        .versions
        .into_iter()
        .filter(|v| v.kind == "release" || (snapshots && v.kind == "snapshot"))
        .map(|v| VersionDto {
            id: v.id,
            kind: v.kind,
            release_time: v.release_time,
        })
        .collect())
}

#[tauri::command]
pub async fn loader_versions(
    state: State<'_, AppState>,
    kind: LoaderKind,
    minecraft: String,
) -> CmdResult<Vec<LoaderVersionDto>> {
    let versions = state
        .launcher
        .loaders()
        .versions(kind, &minecraft)
        .await
        .map_err(err)?;
    Ok(versions
        .into_iter()
        .map(|v| LoaderVersionDto {
            version: v.version,
            stable: v.stable,
        })
        .collect())
}

fn is_running(state: &AppState, id: &str) -> bool {
    state.running.lock().unwrap().contains_key(id)
}

#[tauri::command]
pub fn list_profiles(state: State<'_, AppState>) -> CmdResult<Vec<ProfileDto>> {
    let profiles = state.launcher.profiles().list().map_err(err)?;
    let stats = state.launcher.stats().load().unwrap_or_default();
    let packs = state.launcher.packs();
    Ok(profiles
        .into_iter()
        .map(|p| {
            let running = is_running(&state, &p.id);
            let info = packs.pack_info(&p.id).ok().flatten();
            let source = info.as_ref().map(|i| crate::packs::source_name(&i.source));
            let pack_id = match info.map(|i| i.source) {
                Some(pinecone_core::modpack::PackSource::Twogg { pack_id, .. }) => Some(pack_id),
                _ => None,
            };
            let last_played = stats.profiles.get(&p.id).and_then(|s| s.last_played);
            let mut dto = ProfileDto::new(p, running);
            dto.pack_source = source;
            dto.pack_id = pack_id;
            dto.last_played = last_played;
            dto
        })
        .collect())
}

/// Übernimmt die Dialogeingaben; eine leere Loader-Version wird durch die empfohlene ersetzt.
async fn apply_input(
    state: &AppState,
    profile: &mut Profile,
    input: ProfileInput,
) -> CmdResult<()> {
    profile.name = input.name.trim().to_string();
    profile.minecraft = input.minecraft;
    profile.loader = match input.loader {
        Some(LoaderSpec { kind, version }) if version.trim().is_empty() => {
            let rec = state
                .launcher
                .loaders()
                .recommended(kind, &profile.minecraft)
                .await
                .map_err(err)?;
            Some(LoaderSpec {
                kind,
                version: rec.version,
            })
        }
        other => other,
    };
    profile.memory_mb = input.memory_mb;
    profile.jvm_args = input
        .jvm_args
        .into_iter()
        .filter(|a| !a.trim().is_empty())
        .collect();
    profile.java = input
        .java
        .filter(|j| !j.trim().is_empty())
        .map(PathBuf::from);
    profile.color = match input.color.filter(|c| !c.is_empty()) {
        Some(c)
            if c.len() == 7
                && c.starts_with('#')
                && c[1..].chars().all(|x| x.is_ascii_hexdigit()) =>
        {
            Some(c)
        }
        Some(c) => return Err(format!("Ungültige Farbe {c:?}")),
        None => None,
    };
    profile.icon = match input
        .icon
        .map(|i| i.trim().to_string())
        .filter(|i| !i.is_empty())
    {
        // Motiv aus dem Instanz-Dialog, z. B. `motif:snowflake`.
        Some(i)
            if i.strip_prefix("motif:")
                .is_some_and(|m| MOTIFS.contains(&m)) =>
        {
            Some(i)
        }
        // Ein Emoji kann aus mehreren Codepoints bestehen (z. B. mit Hautfarbe).
        Some(i) if i.chars().count() <= 8 && i.len() <= 32 => Some(i),
        Some(_) => return Err("Symbol ist zu lang (ein Emoji oder bis zu zwei Zeichen)".into()),
        None => None,
    };
    Ok(())
}

/// Instanz-Motive, wie in `ui/src/components/ProfileIcon.svelte`.
const MOTIFS: [&str; 8] = [
    "shield",
    "block",
    "house",
    "sliders",
    "waves",
    "snowflake",
    "server",
    "bolt",
];

#[tauri::command]
pub async fn create_profile(
    state: State<'_, AppState>,
    input: ProfileInput,
) -> CmdResult<ProfileDto> {
    let mut profile = Profile::new("", "");
    apply_input(&state, &mut profile, input).await?;
    let profile = state.launcher.profiles().create(profile).map_err(err)?;
    crate::syncpacks::apply_auto(&state, &profile).await;
    Ok(ProfileDto::new(profile, false))
}

#[tauri::command]
pub async fn update_profile(
    state: State<'_, AppState>,
    id: String,
    input: ProfileInput,
) -> CmdResult<ProfileDto> {
    let profiles = state.launcher.profiles();
    let mut profile = profiles.get(&id).map_err(err)?;
    if let Some(other) = profiles.find(input.name.trim()).map_err(err)?
        && other.id != profile.id
    {
        return Err(format!("Es gibt schon ein Profil {:?}", other.name));
    }
    apply_input(&state, &mut profile, input).await?;
    profiles.save(&profile).map_err(err)?;
    let running = is_running(&state, &id);
    Ok(ProfileDto::new(profile, running))
}

#[tauri::command]
pub fn delete_profile(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    if is_running(&state, &id) {
        return Err("Das Profil läuft gerade. Bitte erst das Spiel beenden.".into());
    }
    // Löscht auch die Spielzeit-Statistik des Profils.
    state.launcher.delete_profile(&id).map(|_| ()).map_err(err)
}

#[tauri::command]
pub fn open_profile_folder(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<()> {
    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    let dir = state.launcher.game_dir(&profile);
    std::fs::create_dir_all(&dir).map_err(err)?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(err)
}

#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> CmdResult<Vec<AccountDto>> {
    let accounts = state.accounts.load().map_err(err)?;
    Ok(accounts
        .accounts
        .iter()
        .map(|a| AccountDto::new(a, accounts.active == Some(a.uuid)))
        .collect())
}

#[tauri::command]
pub fn add_offline_account(state: State<'_, AppState>, name: String) -> CmdResult<AccountDto> {
    let account = state.accounts.add_offline(name.trim()).map_err(err)?;
    Ok(AccountDto::new(&account, true))
}

#[tauri::command]
pub fn set_active_account(state: State<'_, AppState>, uuid: String) -> CmdResult<()> {
    state.accounts.set_active(&uuid).map(|_| ()).map_err(err)
}

#[tauri::command]
pub fn remove_account(state: State<'_, AppState>, uuid: String) -> CmdResult<()> {
    state.accounts.remove(&uuid).map(|_| ()).map_err(err)
}

/// Schritt 1 des Microsoft-Logins: Code holen, den die Oberfläche anzeigt.
#[tauri::command]
pub async fn login_start(state: State<'_, AppState>) -> CmdResult<LoginStart> {
    let msa = state
        .msa
        .as_ref()
        .ok_or("Microsoft-Login ist nicht konfiguriert (PINECONE_MSA_CLIENT_ID fehlt)")?;
    let code = msa.request_device_code().await.map_err(err)?;
    let start = LoginStart {
        user_code: code.user_code.clone(),
        verification_uri: code.verification_uri.clone(),
        expires_in: code.expires_in,
    };
    *state.login.lock().unwrap() = Some((code, Arc::new(Notify::new())));
    Ok(start)
}

/// Schritt 2: warten, bis der Code bestätigt ist, dann Konto speichern.
#[tauri::command]
pub async fn login_finish(state: State<'_, AppState>) -> CmdResult<AccountDto> {
    let msa = state
        .msa
        .as_ref()
        .ok_or("Microsoft-Login ist nicht konfiguriert")?;
    let (code, cancel) = state
        .login
        .lock()
        .unwrap()
        .clone()
        .ok_or("Kein Login gestartet")?;
    let result = tokio::select! {
        r = async {
            let tokens = msa.poll_device_code(&code).await?;
            msa.login_minecraft(tokens).await
        } => r.map_err(err),
        _ = cancel.notified() => Err("Anmeldung abgebrochen".to_string()),
    };
    *state.login.lock().unwrap() = None;
    let account = state.accounts.add_microsoft(&result?).map_err(err)?;
    Ok(AccountDto::new(&account, true))
}

#[tauri::command]
pub fn login_cancel(state: State<'_, AppState>) {
    if let Some((_, cancel)) = state.login.lock().unwrap().take() {
        cancel.notify_one();
    }
}

#[tauri::command]
pub async fn java_list(state: State<'_, AppState>) -> CmdResult<Vec<JavaDto>> {
    Ok(state
        .launcher
        .java()
        .installed()
        .await
        .into_iter()
        .map(|j| JavaDto {
            path: j.path.display().to_string(),
            version: j.version,
            major: j.major,
            managed: j.managed,
        })
        .collect())
}

/// Letzte Zeilen der Spielausgabe eines Profils (z. B. nach Neustart des Launchers).
#[tauri::command]
pub fn last_log(
    state: State<'_, AppState>,
    id: String,
    max_lines: usize,
) -> CmdResult<Vec<String>> {
    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    let path = crate::game::console_log(&state.launcher.game_dir(&profile));
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(err(e)),
    };
    let lines: Vec<&str> = text.lines().collect();
    Ok(lines[lines.len().saturating_sub(max_lines)..]
        .iter()
        .map(|l| l.to_string())
        .collect())
}

/// Für Offline-Accounts kein Hinweis nötig, Microsoft-Accounts melden sich beim Start neu an.
pub fn kind_label(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::Microsoft => "Microsoft",
        AccountKind::Offline => "offline",
    }
}

/// Diagnose-ZIP (Logs, Crash-Reports, Konfiguration ohne Account-Daten) schreiben.
#[tauri::command]
pub async fn diagnostics_export(
    state: State<'_, AppState>,
    path: String,
    profile_id: Option<String>,
) -> CmdResult<pinecone_core::diagnostics::DiagnosticsReport> {
    let logs = crate::logs_dir(state.launcher.paths());
    let only = profile_id.map(|id| vec![id]);
    pinecone_core::diagnostics::export(
        &state.launcher,
        std::path::Path::new(&path),
        only.as_deref(),
        Some(&logs),
    )
    .await
    .map_err(err)
}

/// Ordner mit den Logdateien des Launchers öffnen.
#[tauri::command]
pub fn open_logs_folder(app: tauri::AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    let dir = crate::logs_dir(state.launcher.paths());
    std::fs::create_dir_all(&dir).map_err(err)?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(err)
}
