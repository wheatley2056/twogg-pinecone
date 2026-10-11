//! Modpacks von außen öffnen: `pinecone://`-Links und `.mrpack`-Dateien, die das System beim
//! Start übergibt. Läuft Pinecone schon, reicht die zweite Instanz sie an die erste weiter.

use std::path::Path;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::AppState;

/// Ereignis an die Oberfläche: Link oder Dateipfad.
pub const EVENT: &str = "open-pack";

/// Links und Pack-Dateien aus den Startargumenten (ohne Programmnamen).
/// Relative Pfade gelten relativ zu `cwd`.
pub fn requests(args: &[String], cwd: &Path) -> Vec<String> {
    args.iter()
        .skip(1)
        .filter_map(|arg| {
            if arg.starts_with("pinecone://") {
                return Some(arg.clone());
            }
            let path = cwd.join(arg);
            let ext = path.extension()?.to_str()?.to_ascii_lowercase();
            (matches!(ext.as_str(), "mrpack" | "zip") && path.is_file())
                .then(|| path.display().to_string())
        })
        .collect()
}

/// Zweite Instanz gestartet: Fenster nach vorn holen und ihre Links/Dateien übernehmen.
pub fn forward(app: &AppHandle, args: Vec<String>, cwd: String) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    for request in requests(&args, Path::new(&cwd)) {
        let _ = app.emit(EVENT, request);
    }
}

/// Beim Start übergebene Links/Dateien; die Oberfläche holt sie einmal ab, sobald sie bereit ist.
#[tauri::command]
pub fn take_open_requests(state: State<'_, AppState>) -> Vec<String> {
    std::mem::take(&mut *state.open_requests.lock().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_links_and_existing_pack_files() {
        let dir = std::env::temp_dir().join(format!("pinecone-open-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pack.MRPACK"), "x").unwrap();
        let args: Vec<String> = [
            "twogg-pinecone",
            "pinecone://modrinth/abc",
            "pack.MRPACK",
            "fehlt.mrpack",
            "notiz.txt",
            "--flag",
        ]
        .map(String::from)
        .into();
        let found = requests(&args, &dir);
        assert_eq!(
            found,
            [
                "pinecone://modrinth/abc".to_string(),
                dir.join("pack.MRPACK").display().to_string()
            ]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
