//! Spielstart aus der Oberfläche: Vorbereitung mit Fortschritt, Start mit Log-Datei, Log mitlesen,
//! Spiel beenden. Das Spiel läuft unabhängig vom Launcher weiter, wenn dieser geschlossen wird.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use pinecone_core::download::Progress;
use pinecone_core::launcher::{PrepareEvent, Stage};
use pinecone_core::servers::ServerAddress;
use pinecone_core::stats::now_secs;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Notify;

use crate::AppState;
use crate::commands::{CmdResult, err, kind_label};
use crate::dto::LaunchEvent;

/// Konsolenausgabe des Spiels (stdout/stderr), neben Minecrafts eigenem `logs/latest.log`.
pub fn console_log(game_dir: &Path) -> PathBuf {
    game_dir.join("logs").join("pinecone-console.log")
}

/// Entfernt den Eintrag beim Verlassen, auch bei Fehlern während der Vorbereitung.
struct RunningGuard {
    app: AppHandle,
    id: String,
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.app
            .state::<AppState>()
            .running
            .lock()
            .unwrap()
            .remove(&self.id);
    }
}

fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Sync => "sync",
        Stage::Loader => "loader",
        Stage::Version => "version",
        Stage::Java => "java",
        Stage::Assets => "assets",
    }
}

#[tauri::command]
pub async fn launch(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    server: Option<String>,
    on_event: Channel<LaunchEvent>,
) -> CmdResult<()> {
    let quick_play = server
        .as_deref()
        .map(ServerAddress::parse)
        .transpose()
        .map_err(err)?;
    let stop = Arc::new(Notify::new());
    {
        let mut running = state.running.lock().unwrap();
        if running.contains_key(&id) {
            return Err("Dieses Profil läuft bereits".into());
        }
        running.insert(id.clone(), stop.clone());
    }
    let guard = RunningGuard {
        app: app.clone(),
        id: id.clone(),
    };

    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    if profile.backup_on_launch {
        let _ = on_event.send(LaunchEvent::Stage { stage: "backup" });
        let app = app.clone();
        let p = profile.clone();
        tauri::async_runtime::spawn_blocking(move || {
            crate::instance::backup_before_launch(&app.state::<AppState>(), &p)
        })
        .await
        .map_err(err)?;
    }
    let account = state
        .accounts
        .session(None, state.msa.as_ref())
        .await
        .map_err(err)?;
    tracing::info!(
        "Starte {} als {} ({})",
        profile.name,
        account.name,
        kind_label(account.kind)
    );

    // Fortschritt höchstens alle 100 ms senden, sonst flutet der Asset-Download den Channel.
    let mut last = Instant::now() - Duration::from_secs(1);
    let mut on_prepare = |e: PrepareEvent| {
        let event = match e {
            PrepareEvent::Stage(s) => LaunchEvent::Stage {
                stage: stage_name(s),
            },
            PrepareEvent::Java { version, path } => LaunchEvent::Java {
                version,
                path: path.display().to_string(),
            },
            PrepareEvent::Download(Progress::Started { files, bytes }) => LaunchEvent::Progress {
                done: 0,
                files,
                bytes_done: 0,
                bytes,
            },
            PrepareEvent::Download(Progress::FileDone {
                done,
                files,
                bytes_done,
                ..
            }) => {
                if done < files && last.elapsed() < Duration::from_millis(100) {
                    return;
                }
                last = Instant::now();
                LaunchEvent::Progress {
                    done,
                    files,
                    bytes_done,
                    bytes: 0,
                }
            }
        };
        let _ = on_event.send(event);
    };
    let command = state
        .launcher
        .prepare(&profile, account, false, quick_play, &mut on_prepare)
        .await
        .map_err(err)?;

    let log = console_log(&state.launcher.game_dir(&profile));
    let mut child = command.spawn_logged(&log).await.map_err(err)?;
    let started = now_secs();
    let _ = on_event.send(LaunchEvent::Started { pid: child.id() });

    tauri::async_runtime::spawn(async move {
        let exited = Arc::new(AtomicBool::new(false));
        let tail =
            tauri::async_runtime::spawn(tail_log(log.clone(), on_event.clone(), exited.clone()));
        let status = tokio::select! {
            status = child.wait() => status,
            _ = stop.notified() => {
                let _ = child.kill().await;
                child.wait().await
            }
        };
        exited.store(true, Ordering::SeqCst);
        let _ = tail.await;
        // Spielzeit und letzten Server verbuchen, auch nach Abstürzen.
        let output = tokio::fs::read_to_string(&log).await.unwrap_or_default();
        let launcher = &guard.app.state::<AppState>().launcher;
        if let Err(e) = launcher.finish_session(&guard.id, started, &output) {
            tracing::warn!("Spielzeit nicht gespeichert: {e}");
        }
        let (code, success) = match status {
            Ok(s) => (s.code(), s.success()),
            Err(_) => (None, false),
        };
        let _ = on_event.send(LaunchEvent::Exited { code, success });
        drop(guard);
    });
    Ok(())
}

/// Liest neue Zeilen der Log-Datei und schickt sie gebündelt, bis das Spiel beendet ist.
/// Eine noch unvollständige letzte Zeile bleibt im Puffer, bis ihr Zeilenumbruch kommt.
async fn tail_log(path: PathBuf, channel: Channel<LaunchEvent>, exited: Arc<AtomicBool>) {
    let file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("Log {} nicht lesbar: {e}", path.display());
            return;
        }
    };
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    loop {
        // Vor dem Lesen merken: Was danach noch kommt, ist sicher vor dem Spielende geschrieben.
        let done = exited.load(Ordering::SeqCst);
        let mut batch = Vec::new();
        loop {
            match reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) if line.ends_with('\n') => {
                    batch.push(line.trim_end().to_string());
                    line.clear();
                    if batch.len() >= 500 {
                        break;
                    }
                }
                Ok(_) => break,
            }
        }
        if done && !line.is_empty() {
            batch.push(std::mem::take(&mut line).trim_end().to_string());
        }
        let full = batch.len() >= 500;
        if !batch.is_empty() {
            let _ = channel.send(LaunchEvent::Log { lines: batch });
        }
        if full {
            continue;
        }
        if done {
            return;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}

#[tauri::command]
pub fn stop_game(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    match state.running.lock().unwrap().get(&id) {
        Some(stop) => {
            stop.notify_one();
            Ok(())
        }
        None => Err("Dieses Profil läuft nicht".into()),
    }
}
