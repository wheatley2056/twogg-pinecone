//! Startanimation: Das Hauptfenster startet unsichtbar und lädt im Hintergrund, während ein
//! randloses Fenster das Logo animiert (`ui/splash.html`). Ist die Animation durch, meldet die
//! Seite `splash_finished`; dann geht der Launcher auf. Spätestens nach `FALLBACK` erscheint er
//! auch so, falls die Seite hängt.

use std::time::Duration;

use pinecone_core::settings::Settings;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "splash";
const FALLBACK: Duration = Duration::from_secs(10);

/// Beim Start aufrufen; ohne Einstellungen gilt die Voreinstellung (Animation an).
pub fn start(app: &AppHandle, settings: Option<&Settings>) {
    if !settings.is_none_or(|s| s.splash) {
        show_main(app);
        return;
    }
    let reduce = settings.is_some_and(|s| s.theme.reduce_motion);
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("splash.html".into()))
        .title("TwoGG Pinecone")
        .inner_size(640.0, 400.0)
        .resizable(false)
        .maximizable(false)
        .decorations(false)
        .center()
        .focused(true)
        .initialization_script(if reduce {
            "window.__PINECONE_REDUCE_MOTION__ = true;"
        } else {
            ""
        })
        .build();
    match built {
        Ok(_) => {
            let app = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(FALLBACK);
                finish(&app);
            });
        }
        Err(e) => {
            tracing::warn!("Startanimation nicht möglich: {e}");
            show_main(app);
        }
    }
}

fn show_main(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}

fn finish(app: &AppHandle) {
    show_main(app);
    if let Some(splash) = app.get_webview_window(LABEL) {
        let _ = splash.close();
    }
}

/// Von der Splash-Seite, sobald die Animation fertig ist.
#[tauri::command]
pub fn splash_finished(app: AppHandle) {
    finish(&app);
}
