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
use crate::stats::{ProfileStats, StatsStore, now_secs};

pub const DEFAULT_MEMORY_MB: u32 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
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
}

impl Launcher {
    pub fn new(paths: Paths, downloader: Downloader) -> Result<Self> {
        let installer = Installer::new(paths.clone(), downloader.clone());
        let modrinth = Modrinth::new()?;
        let curseforge = curseforge::api_key_from_env()
            .map(CurseForge::new)
            .transpose()?;
        let content = ContentManager::new(paths.clone(), downloader.clone(), modrinth.clone())
            .with_curseforge(curseforge.clone());
        let packs = PackManager::new(
            paths.clone(),
            downloader.clone(),
            content.clone(),
            modrinth,
            ProfileStore::new(paths.clone()),
        )
        .with_curseforge(curseforge);
        let java = JavaManager::new(
            paths.clone(),
            downloader.clone(),
            installer.environment().clone(),
        );
        Ok(Self {
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
        let java = match &profile.java {
            Some(path) => path.clone(),
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
                found.path
            }
        };

        on_event(PrepareEvent::Stage(Stage::Assets));
        let game_dir = self.game_dir(profile);
        let game_assets = self.installer.prepare_assets(&version, &game_dir).await?;

        let opts = LaunchOptions {
            java,
            account,
            game_dir,
            game_assets: Some(game_assets),
            memory_mb: Some(profile.memory_mb.unwrap_or(DEFAULT_MEMORY_MB)),
            extra_jvm_args: profile.jvm_args.clone(),
            resolution: profile.resolution.map(|r| (r.width, r.height)),
            quick_play,
        };
        launch::build_command(&self.paths, &version, &opts, self.installer.environment())
    }
}
