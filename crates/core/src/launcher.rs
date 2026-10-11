//! Kompletter Startablauf für ein Profil: Loader, Version, Java, Assets, Startbefehl.
//! Wird von CLI und GUI gleichermaßen genutzt.

use std::path::PathBuf;

use crate::Result;
use crate::auth::Account;
use crate::content::ContentManager;
use crate::curseforge::{self, CurseForge};
use crate::download::{Downloader, Progress};
use crate::install::Installer;
use crate::java::{DEFAULT_JAVA, JavaManager};
use crate::launch::{self, LaunchCommand, LaunchOptions};
use crate::loader::LoaderInstaller;
use crate::modpack::PackManager;
use crate::modrinth::Modrinth;
use crate::paths::Paths;
use crate::profiles::{Profile, ProfileStore};
use crate::servers::{ServerAddress, last_server_in_log};
use crate::settings::SettingsStore;
use crate::skins::SkinLibrary;
use crate::stats::{ProfileStats, StatsStore, now_secs};
use crate::syncpacks::SyncManager;

pub const DEFAULT_MEMORY_MB: u32 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Eingeschaltete Sync-Packs abgleichen.
    Sync,
    Loader,
    Version,
    Java,
    Assets,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepareEvent {
    Stage(Stage),
    Download(Progress),
    /// Gewähltes Java (Version, Pfad).
    Java {
        version: String,
        path: PathBuf,
    },
}

pub struct Launcher {
    paths: Paths,
    installer: Installer,
    java: JavaManager,
    loaders: LoaderInstaller,
    profiles: ProfileStore,
    stats: StatsStore,
    settings: SettingsStore,
    content: ContentManager,
    packs: PackManager,
    sync: SyncManager,
    skins: SkinLibrary,
}

impl Launcher {
    pub fn new(paths: Paths, downloader: Downloader) -> Result<Self> {
        let curseforge = curseforge::api_key_from_env()
            .map(CurseForge::new)
            .transpose()?;
        Self::with_services(paths, downloader, Modrinth::new()?, curseforge)
    }

    /// Mit eigenen API-Clients (z. B. gegen einen Mock-Server in Tests).
    pub fn with_services(
        paths: Paths,
        downloader: Downloader,
        modrinth: Modrinth,
        curseforge: Option<CurseForge>,
    ) -> Result<Self> {
        let installer = Installer::new(paths.clone(), downloader.clone());
        let content = ContentManager::new(paths.clone(), downloader.clone(), modrinth.clone())
            .with_curseforge(curseforge.clone());
        let packs = PackManager::new(
            paths.clone(),
            downloader.clone(),
            content.clone(),
            modrinth,
            ProfileStore::new(paths.clone()),
        )
        .with_curseforge(curseforge)
        .with_twogg(crate::twogg::TwoggClient::new().ok());
        let java = JavaManager::new(
            paths.clone(),
            downloader.clone(),
            installer.environment().clone(),
        );
        Ok(Self {
            skins: SkinLibrary::new(&paths)?,
            sync: SyncManager::new(
                paths.clone(),
                content.clone(),
                ProfileStore::new(paths.clone()),
            ),
            content,
            packs,
            loaders: LoaderInstaller::new(paths.clone(), downloader),
            profiles: ProfileStore::new(paths.clone()),
            stats: StatsStore::new(paths.stats_file()),
            settings: SettingsStore::new(paths.root()),
            paths,
            installer,
            java,
        })
    }

    pub fn content(&self) -> &ContentManager {
        &self.content
    }

    pub fn packs(&self) -> &PackManager {
        &self.packs
    }

    pub fn syncpacks(&self) -> &SyncManager {
        &self.sync
    }

    pub fn skins(&self) -> &SkinLibrary {
        &self.skins
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn installer(&self) -> &Installer {
        &self.installer
    }

    pub fn java(&self) -> &JavaManager {
        &self.java
    }

    pub fn loaders(&self) -> &LoaderInstaller {
        &self.loaders
    }

    pub fn profiles(&self) -> &ProfileStore {
        &self.profiles
    }

    pub fn stats(&self) -> &StatsStore {
        &self.stats
    }

    /// JVM-Argumente einer Instanz: Preset (eigenes oder globaler Standard), danach die eigenen
    /// Argumente der Instanz, damit diese gewinnen. Ist Java zu alt für das Preset, entfällt es.
    pub fn jvm_args(&self, profile: &Profile, java_major: Option<u32>) -> Vec<String> {
        let jvm = self.settings.load().map(|s| s.jvm).unwrap_or_default();
        let id = profile
            .jvm_preset
            .clone()
            .unwrap_or_else(|| jvm.default_preset.clone());
        let mut args = match jvm.find(&id) {
            Some(preset) => {
                let args = crate::jvm::launch_args(&preset, java_major);
                if args.is_empty() && !preset.args.is_empty() {
                    tracing::warn!(
                        "JVM-Preset {} braucht Java {}+, gestartet wird ohne",
                        preset.name,
                        preset.min_java
                    );
                }
                args
            }
            None => Vec::new(),
        };
        args.extend(profile.jvm_args.iter().cloned());
        args
    }

    pub fn settings(&self) -> &SettingsStore {
        &self.settings
    }

    /// Verbucht eine beendete Sitzung; der letzte Server kommt aus der Spielausgabe.
    pub fn finish_session(
        &self,
        profile_id: &str,
        started: u64,
        output: &str,
    ) -> Result<ProfileStats> {
        let server = last_server_in_log(output);
        self.stats
            .record_session(profile_id, started, now_secs(), server)
    }

    /// Löscht ein Profil samt Spielordner und Statistik.
    pub fn delete_profile(&self, selector: &str) -> Result<Profile> {
        let profile = self.profiles.delete(selector)?;
        if let Err(e) = self.stats.forget(&profile.id) {
            tracing::warn!("Statistik von {} nicht entfernt: {e}", profile.id);
        }
        Ok(profile)
    }

    /// Spielordner: bei gespeicherten Profilen `instances/<id>/.minecraft`,
    /// bei spontanen Starts ohne Profil `instances/_<version>`.
    pub fn game_dir(&self, profile: &Profile) -> PathBuf {
        if profile.id.is_empty() {
            self.paths
                .instance_dir(&format!("_{}", profile.version_id()))
        } else {
            self.profiles.game_dir(&profile.id)
        }
    }

    /// Lädt alles Nötige (außer mit `offline`) und baut den Startbefehl.
    pub async fn prepare(
        &self,
        profile: &Profile,
        account: Account,
        offline: bool,
        quick_play: Option<ServerAddress>,
        on_event: &mut (dyn FnMut(PrepareEvent) + Send),
    ) -> Result<LaunchCommand> {
        // Sync-Packs abgleichen; ohne Netz oder bei Fehlern mit dem bisherigen Stand starten.
        if !offline && !profile.id.is_empty() {
            on_event(PrepareEvent::Stage(Stage::Sync));
            match self.sync.sync(profile).await {
                Ok(r) => {
                    for w in r.warnings {
                        tracing::warn!("Sync-Pack: {w}");
                    }
                }
                Err(e) => tracing::warn!("Sync-Packs nicht abgeglichen: {e}"),
            }
        }
        let id = match (&profile.loader, offline) {
            (Some(spec), false) => {
                on_event(PrepareEvent::Stage(Stage::Loader));
                self.loaders.install(&profile.minecraft, spec).await?
            }
            _ => profile.version_id(),
        };

        on_event(PrepareEvent::Stage(Stage::Version));
        let version = if offline {
            self.installer.load_installed(&id).await?
        } else {
            self.installer
                .install(&id, |p| on_event(PrepareEvent::Download(p)))
                .await?
        };

        on_event(PrepareEvent::Stage(Stage::Java));
        let (java, java_major) = match &profile.java {
            Some(path) => (
                path.clone(),
                crate::java::probe(path).await.map(|j| j.major),
            ),
            None => {
                let wanted = version.java_version.clone().unwrap_or(DEFAULT_JAVA);
                let found = self
                    .java
                    .ensure(&wanted, |p| on_event(PrepareEvent::Download(p)))
                    .await?;
                on_event(PrepareEvent::Java {
                    version: found.version.clone(),
                    path: found.path.clone(),
                });
                (found.path, Some(found.major))
            }
        };

        // Forge/NeoForge: gepatchten Client einmalig mit den Installer-Prozessoren erzeugen.
        if let (Some(spec), false) = (&profile.loader, offline) {
            on_event(PrepareEvent::Stage(Stage::Loader));
            self.loaders
                .finish(spec, &id, &profile.minecraft, &java)
                .await?;
        }

        on_event(PrepareEvent::Stage(Stage::Assets));
        let game_dir = self.game_dir(profile);
        let game_assets = self.installer.prepare_assets(&version, &game_dir).await?;

        let opts = LaunchOptions {
            java,
            account,
            game_dir,
            game_assets: Some(game_assets),
            memory_mb: Some(profile.memory_mb.unwrap_or(DEFAULT_MEMORY_MB)),
            extra_jvm_args: self.jvm_args(profile, java_major),
            resolution: profile.resolution.map(|r| (r.width, r.height)),
            quick_play,
        };
        launch::build_command(&self.paths, &version, &opts, self.installer.environment())
    }
}
