// Im Release unter Windows kein Konsolenfenster neben der App.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod content;
mod dashboard;
mod dto;
mod friends;
mod game;
mod instance;
mod migrate;
mod open;
mod packs;
mod screenshots;
mod skins;
mod splash;
mod syncpacks;
mod twogg_app;
mod update;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use pinecone_core::accounts::{AccountStore, KeyringStore};
use pinecone_core::download::Downloader;
use pinecone_core::launcher::Launcher;
use pinecone_core::msa::{self, DeviceCode, MsaClient};
use pinecone_core::paths::Paths;
use tokio::sync::Notify;

pub struct AppState {
    pub launcher: Launcher,
    pub accounts: AccountStore<KeyringStore>,
    pub msa: Option<MsaClient>,
    /// Laufende Spiele: Profil-ID → Signal zum Beenden.
    pub running: Mutex<HashMap<String, Arc<Notify>>>,
    /// Laufender Microsoft-Login: Code und Abbruchsignal.
    pub login: Mutex<Option<(DeviceCode, Arc<Notify>)>>,
    /// Geplante Modpack-Updates: Profil-ID → (Archiv, Quelle). Die Oberfläche schickt nur Entscheidungen.
    pub pending_updates:
        Mutex<HashMap<String, (std::path::PathBuf, pinecone_core::modpack::PackSource)>>,
    /// Angezeigte Import-Vorschauen: Token → (Archiv, Quelle).
    pub pending_imports:
        Mutex<HashMap<String, (std::path::PathBuf, pinecone_core::modpack::PackSource)>>,
    /// Beim Start übergebene Links/Dateien, bis die Oberfläche sie abholt.
    pub open_requests: Mutex<Vec<String>>,
    /// twogg.app: Events, Konto, Anmeldungen.
    pub twogg_app: Arc<pinecone_core::twogg_app::TwoggApp>,
}

/// Logs in die Konsole (Warnungen) und nach `<data-dir>/logs/pinecone.log.<datum>` (Infos),
/// eine Datei pro Tag, die letzten sieben bleiben. `RUST_LOG` überschreibt beide Stufen.
fn init_logging(paths: &Paths) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{EnvFilter, Layer, fmt};

    let filter =
        |default: &str| EnvFilter::try_from_default_env().unwrap_or_else(|_| default.into());
    let console = fmt::layer().with_filter(filter("warn"));
    let file = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("pinecone")
        .filename_suffix("log")
        .max_log_files(7)
        .build(logs_dir(paths))
        .ok();
    match file {
        Some(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            let file_layer = fmt::layer()
                .with_ansi(false)
                .with_writer(writer)
                .with_filter(filter("info,tao=warn,wry=warn"));
            tracing_subscriber::registry()
                .with(console)
                .with(file_layer)
                .init();
            Some(guard)
        }
        None => {
            tracing_subscriber::registry().with(console).init();
            None
        }
    }
}

pub fn logs_dir(paths: &Paths) -> std::path::PathBuf {
    paths.root().join("logs")
}

fn main() {
    // WebKitGTK stürzt mit NVIDIA unter Wayland ab („Error 71 (Protokollfehler) dispatching to
    // Wayland display“). Ohne DMA-BUF-Renderer läuft es stabil; eine eigene Einstellung gewinnt.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: Ganz am Anfang von main, bevor weitere Threads existieren.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }

    let root = std::env::var_os("PINECONE_HOME")
        .map(Into::into)
        .map(Ok)
        .unwrap_or_else(Paths::default_root)
        .expect("Kein Datenverzeichnis");
    let paths = Paths::new(root);
    // Der Guard muss bis zum Programmende leben, sonst gehen die letzten Logzeilen verloren.
    let _log_guard = init_logging(&paths);
    let downloader = Downloader::new().expect("HTTP-Client");
    let msa = msa::client_id_from_env()
        .map(MsaClient::new)
        .transpose()
        .expect("HTTP-Client");
    let state = AppState {
        accounts: AccountStore::new(paths.accounts_file(), KeyringStore),
        launcher: Launcher::new(paths, downloader).expect("HTTP-Client"),
        msa,
        running: Mutex::default(),
        login: Mutex::default(),
        pending_updates: Mutex::default(),
        pending_imports: Mutex::default(),
        open_requests: Mutex::new(open::requests(
            &std::env::args().collect::<Vec<_>>(),
            &std::env::current_dir().unwrap_or_default(),
        )),
        twogg_app: Arc::new(
            pinecone_core::twogg_app::TwoggApp::new(Arc::new(KeyringStore)).expect("HTTP-Client"),
        ),
    };

    tracing::info!("TwoGG Pinecone {} gestartet", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        // Muss als erstes Plugin kommen: Eine zweite Instanz (Link oder Datei geöffnet)
        // beendet sich sofort und übergibt ihre Argumente an die laufende.
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            open::forward(app, args, cwd)
        }))
        .plugin(tauri_plugin_deep_link::init())
        // Ohne eingebauten Schlüssel bleibt der Updater registriert, prüft aber nie (siehe update.rs).
        .plugin(
            tauri_plugin_updater::Builder::new()
                .pubkey(update::pubkey().unwrap_or_default())
                .build(),
        )
        .manage(update::PendingUpdate::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(|app| {
            dashboard::allow_assets(app.handle());
            let settings = tauri::Manager::state::<AppState>(app)
                .launcher
                .settings()
                .load()
                .ok();
            if let Some(settings) = &settings {
                dashboard::apply_frame(app.handle(), settings);
            }
            splash::start(app.handle(), settings.as_ref());
            screenshots::start_watcher(app.handle());
            // Installer tragen `pinecone://` selbst ein; beim Entwickeln und für AppImages
            // registriert sich die App hier (Windows: Registry des Nutzers, Linux: .desktop-Datei).
            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Err(e) = app.deep_link().register_all() {
                    tracing::warn!("pinecone://-Links nicht registriert: {e}");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::list_versions,
            commands::loader_versions,
            commands::list_profiles,
            commands::create_profile,
            commands::update_profile,
            commands::delete_profile,
            commands::duplicate_profile,
            commands::set_profile_account,
            commands::set_profile_group,
            commands::set_profile_hooks,
            commands::open_profile_folder,
            commands::list_accounts,
            commands::add_offline_account,
            commands::set_active_account,
            commands::remove_account,
            commands::login_start,
            commands::login_finish,
            commands::login_cancel,
            commands::java_list,
            commands::last_log,
            splash::splash_finished,
            dashboard::dashboard,
            dashboard::ping_server,
            dashboard::open_screenshot,
            dashboard::get_settings,
            dashboard::jvm_builtin_presets,
            dashboard::save_settings,
            dashboard::export_theme,
            content::datapack_install,
            instance::worlds_list,
            instance::world_backup,
            instance::world_delete,
            instance::world_backups,
            instance::world_restore,
            instance::world_backup_delete,
            instance::set_backup_on_launch,
            instance::world_open_folder,
            instance::servers_list,
            friends::friends_list,
            friends::friends_presence,
            friends::friends_request,
            friends::friends_accept,
            friends::friends_decline,
            friends::friends_remove,
            friends::friends_delete_data,
            friends::twogg_server_info,
            friends::twogg_recommendations,
            twogg_app::twogg_app_events,
            twogg_app::twogg_app_me,
            twogg_app::twogg_app_login_discord,
            twogg_app::twogg_app_login_password,
            twogg_app::twogg_app_logout,
            twogg_app::twogg_app_participations,
            twogg_app::twogg_app_join,
            twogg_app::twogg_app_friends,
            dashboard::import_theme,
            content::content_list,
            content::content_identify,
            content::content_set_enabled,
            content::content_remove,
            content::content_add_local,
            content::content_add_dropped,
            content::content_install,
            content::content_updates,
            content::content_update,
            content::modrinth_search,
            content::modrinth_versions,
            content::modrinth_categories,
            content::curseforge_search,
            content::curseforge_files,
            content::content_install_curseforge,
            packs::pack_install_modrinth,
            packs::pack_install_curseforge,
            packs::pack_install_twogg,
            packs::twogg_packs,
            packs::twogg_pack,
            packs::twogg_news,
            packs::pack_preview_file,
            packs::pack_preview_link,
            packs::pack_preview_cancel,
            packs::pack_import_pending,
            packs::pack_share_link,
            packs::pack_info,
            packs::pack_check_update,
            packs::pack_updates_all,
            packs::pack_fill_icons,
            packs::pack_plan_update,
            packs::pack_apply_update,
            packs::pack_snapshots,
            packs::pack_rollback,
            packs::pack_export,
            open::take_open_requests,
            migrate::migrate_plan,
            migrate::migrate_apply,
            syncpacks::syncpack_list,
            syncpacks::syncpack_create,
            syncpacks::syncpack_save,
            syncpacks::syncpack_delete,
            syncpacks::syncpack_add_file,
            syncpacks::syncpack_add_from_profile,
            syncpacks::syncpack_for_profile,
            syncpacks::syncpack_set_enabled,
            syncpacks::syncpack_sync,
            syncpacks::syncpack_profiles,
            syncpacks::syncpack_profile_options,
            update::update_check,
            update::update_install,
            commands::diagnostics_export,
            commands::open_logs_folder,
            screenshots::screenshots_list,
            screenshots::screenshot_thumb,
            screenshots::screenshot_delete,
            screenshots::screenshot_reveal,
            screenshots::screenshot_save_as,
            skins::skins_list,
            skins::skin_data,
            skins::skin_import_file,
            skins::skin_import_player,
            skins::skin_save,
            skins::skin_rename,
            skins::skin_set_model,
            skins::skin_delete,
            skins::skin_export,
            skins::skin_account,
            skins::skin_apply,
            skins::skin_set_cape,
            skins::skin_import_account,
            game::launch,
            game::stop_game,
        ])
        .run(tauri::generate_context!())
        .expect("TwoGG Pinecone konnte nicht starten");
}
