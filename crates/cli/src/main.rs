//! `pinecone`: schlanke Kommandozeile über `pinecone-core` für Entwicklung, Tests und Debugging.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use pinecone_core::accounts::{AccountStore, KeyringStore};
use pinecone_core::auth::{Account, AccountKind};
use pinecone_core::content::{ContentKind, Source, loaders_for};
use pinecone_core::download::{Downloader, Progress};
use pinecone_core::install::Installer;
use pinecone_core::launch;
use pinecone_core::launcher::{Launcher, PrepareEvent, Stage};
use pinecone_core::loader::{LoaderKind, LoaderSpec};
use pinecone_core::manifest::VersionJson;
use pinecone_core::migrate::{ItemStatus, MigrateOptions, MigrationTarget, MissingChoice};
use pinecone_core::modpack::{ExportOptions, PackSource, Resolution};
use pinecone_core::modrinth::{ProjectType, SearchQuery};
use pinecone_core::msa::{self, MsaClient};
use pinecone_core::paths::Paths;
use pinecone_core::profiles::Profile;
use pinecone_core::servers::{ServerAddress, ping as server_ping, read_servers_dat};
use pinecone_core::share::ShareLink;
use pinecone_core::skins::SkinModel;
use pinecone_core::stats::now_secs;
use pinecone_core::syncpacks::{SyncEntry, SyncMode, SyncPack, SyncReport};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Parser)]
#[command(
    name = "pinecone",
    version,
    about = "TwoGG Pinecone – Minecraft Launcher (CLI)"
)]
struct Cli {
    /// Datenverzeichnis (Versionen, Libraries, Assets, Instanzen).
    #[arg(long, global = true, env = "PINECONE_HOME")]
    data_dir: Option<PathBuf>,

    /// Vorhandene Dateien per SHA-1 prüfen statt nur per Größe.
    #[arg(long, global = true)]
    verify: bool,

    /// Client-ID der Azure-App für den Microsoft-Login.
    #[arg(
        long,
        global = true,
        env = "PINECONE_MSA_CLIENT_ID",
        hide_env_values = true
    )]
    client_id: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Verfügbare Minecraft-Versionen anzeigen.
    Versions {
        /// Auch Snapshots, Betas und Alphas anzeigen.
        #[arg(long)]
        all: bool,
        /// Höchstens so viele Einträge anzeigen.
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Version installieren (`latest` und `latest-snapshot` sind erlaubt).
    Install { version: String },
    /// Profil oder Version starten; fehlende Dateien werden vorher geladen.
    Run {
        /// Profilname bzw. -ID, sonst eine Minecraft-Version (ohne Profil).
        target: String,
        /// Gespeicherter Account (Name oder UUID); Standard: aktiver Account.
        #[arg(long, value_name = "NAME", conflicts_with = "offline")]
        account: Option<String>,
        /// Einmalig offline mit diesem Spielernamen spielen, ohne ihn zu speichern.
        #[arg(long, value_name = "NAME")]
        offline: Option<String>,
        /// Java-Programm (überschreibt Profil und automatische Auswahl).
        #[arg(long)]
        java: Option<PathBuf>,
        /// Maximaler Arbeitsspeicher in MB (überschreibt das Profil).
        #[arg(long)]
        memory: Option<u32>,
        /// Nur den Startbefehl ausgeben, nicht starten.
        #[arg(long)]
        print_command: bool,
        /// Ohne Netzwerk starten (Version muss installiert sein).
        #[arg(long)]
        no_update: bool,
        /// Direkt auf diesen Server verbinden, z. B. `mc.twogg.de` oder `host:25570`.
        #[arg(long, value_name = "ADRESSE")]
        server: Option<String>,
    },
    /// Server abfragen.
    #[command(subcommand)]
    Server(ServerCommand),
    /// Spielzeit je Profil anzeigen.
    Stats,
    /// Mods, Ressourcenpakete und Shader eines Profils verwalten.
    #[command(subcommand)]
    Content(ContentCommand),
    /// Modpacks importieren, aktualisieren, zurückrollen und exportieren.
    #[command(subcommand)]
    Pack(PackCommand),
    /// Profile (Instanzen) verwalten.
    #[command(subcommand)]
    Profile(ProfileCommand),
    /// Sync-Packs und Vorlagen: Inhalte, die man Profilen zuschaltet.
    #[command(subcommand)]
    Sync(SyncCommand),
    /// Skin-Bibliothek, Skin hochladen, Capes.
    #[command(subcommand)]
    Skin(SkinCommand),
    /// Diagnose-ZIP für die Fehlersuche (Logs, Crash-Reports, Konfiguration; ohne Account-Daten).
    Diag {
        target: PathBuf,
        /// Nur dieses Profil statt aller.
        #[arg(long)]
        profile: Option<String>,
    },
    /// Mod-Loader abfragen.
    #[command(subcommand)]
    Loader(LoaderCommand),
    /// Java-Laufzeiten verwalten.
    #[command(subcommand)]
    Java(JavaCommand),
    /// Accounts verwalten.
    #[command(subcommand)]
    Account(AccountCommand),
}

#[derive(Subcommand)]
enum ProfileCommand {
    /// Neues Profil anlegen.
    Create {
        name: String,
        /// Minecraft-Version (`latest` erlaubt).
        #[arg(long)]
        version: String,
        /// Mod-Loader: fabric oder quilt.
        #[arg(long)]
        loader: Option<LoaderKind>,
        /// Loader-Version; Standard: neueste stabile.
        #[arg(long, requires = "loader")]
        loader_version: Option<String>,
        /// Arbeitsspeicher in MB (Standard 4096).
        #[arg(long)]
        memory: Option<u32>,
    },
    /// Profile anzeigen.
    List,
    /// Profil ändern.
    Edit {
        name: String,
        /// Neuer Anzeigename (der Ordner bleibt).
        #[arg(long)]
        rename: Option<String>,
        #[arg(long)]
        version: Option<String>,
        /// fabric, quilt oder none.
        #[arg(long)]
        loader: Option<String>,
        #[arg(long)]
        loader_version: Option<String>,
        #[arg(long)]
        memory: Option<u32>,
        /// Java-Pfad oder `auto`.
        #[arg(long)]
        java: Option<String>,
        /// Zusätzliche JVM-Argumente, durch Leerzeichen getrennt (leer = entfernen).
        #[arg(long, allow_hyphen_values = true)]
        jvm_args: Option<String>,
    },
    /// Profil samt Spielordner (Welten, Mods, Einstellungen) löschen.
    Delete {
        name: String,
        /// Löschen bestätigen.
        #[arg(long)]
        yes: bool,
    },
    /// Auf andere Minecraft-Version/anderen Loader umziehen (als neues Profil).
    /// Ohne --apply nur den Plan anzeigen.
    Migrate {
        name: String,
        /// Ziel-Minecraft-Version.
        #[arg(long)]
        minecraft: String,
        /// Ziel-Loader: fabric, quilt oder none (Standard: wie bisher).
        #[arg(long)]
        loader: Option<String>,
        /// Name des neuen Profils.
        #[arg(long = "as")]
        new_name: Option<String>,
        /// Welten nicht mitkopieren.
        #[arg(long)]
        no_worlds: bool,
        /// Nicht verfügbare Mods weglassen statt deaktiviert mitnehmen.
        #[arg(long)]
        drop_missing: bool,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Subcommand)]
enum LoaderCommand {
    /// Loader-Versionen für eine Minecraft-Version anzeigen.
    Versions {
        kind: LoaderKind,
        minecraft: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum AccountCommand {
    /// Mit Microsoft anmelden (Code im Browser bestätigen).
    Login,
    /// Offline-Account anlegen (nur für Tests und LAN).
    Offline { name: String },
    /// Gespeicherte Accounts anzeigen.
    List,
    /// Account als aktiv setzen.
    Use { name: String },
    /// Account entfernen (inkl. gespeichertem Login).
    Remove { name: String },
}

#[derive(Subcommand)]
enum ContentCommand {
    /// Installierte Inhalte anzeigen (unbekannte Dateien werden per Hash erkannt).
    List { profile: String },
    /// Auf Modrinth suchen; mit --profile nur passende Inhalte.
    Search {
        query: String,
        #[arg(long, value_enum, default_value = "mod")]
        r#type: KindArg,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Projekt (Slug oder ID) samt Abhängigkeiten installieren.
    Install {
        profile: String,
        project: String,
        #[arg(long, value_enum, default_value = "mod")]
        r#type: KindArg,
        /// Bestimmte Modrinth-Versions-ID statt der passenden neuesten.
        #[arg(long)]
        version: Option<String>,
    },
    /// Verfügbare Updates anzeigen bzw. mit --apply einspielen.
    Updates {
        profile: String,
        #[arg(long)]
        apply: bool,
    },
    /// Inhalt entfernen (Schlüssel wie in `list`, z. B. mods/sodium.jar).
    Remove { profile: String, key: String },
}

#[derive(Subcommand)]
enum PackCommand {
    /// Als neues Profil importieren: `.mrpack`-Datei, `modrinth:<slug>`, `curseforge:<id>`,
    /// `twogg:<pack>` oder ein `pinecone://`-Link.
    Import {
        source: String,
        /// Bestimmte Modrinth-Versions-ID.
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        name: Option<String>,
    },
    /// Update planen und mit --apply einspielen (Konflikte: Vorschlag oder --take-pack).
    Update {
        profile: String,
        /// Neue Version als Datei statt von Modrinth.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Bestimmte Modrinth-Versions-ID statt der neuesten.
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        apply: bool,
        /// Bei diesen Konfliktpfaden die Pack-Fassung nehmen.
        #[arg(long = "take-pack", value_name = "PFAD")]
        take_pack: Vec<String>,
        /// Bei diesen Konfliktpfaden die eigene Fassung behalten.
        #[arg(long = "keep-mine", value_name = "PFAD")]
        keep_mine: Vec<String>,
    },
    /// Snapshots anzeigen bzw. mit --to zurückrollen.
    Rollback {
        profile: String,
        #[arg(long)]
        to: Option<String>,
    },
    /// Profil als .mrpack exportieren (Mods immer; servers.dat, Welten und Accounts nie).
    Export {
        profile: String,
        target: PathBuf,
        #[arg(long, default_value = "1.0.0")]
        version: String,
        /// Kurzbeschreibung im Pack.
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        no_resourcepacks: bool,
        #[arg(long)]
        no_shaders: bool,
        /// Ordner config/ weglassen.
        #[arg(long)]
        no_config: bool,
        /// options.txt (Spieleinstellungen) mitnehmen.
        #[arg(long)]
        with_options: bool,
    },
    /// Link zum Teilen eines Modrinth- oder CurseForge-Packs (pinecone://…) ausgeben.
    Share { profile: String },
    /// Inhalt eines Packs anzeigen, ohne es zu installieren (Datei oder Link).
    Preview { source: String },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum KindArg {
    Mod,
    Resourcepack,
    Shader,
}

impl From<KindArg> for ContentKind {
    fn from(k: KindArg) -> Self {
        match k {
            KindArg::Mod => ContentKind::Mod,
            KindArg::Resourcepack => ContentKind::Resourcepack,
            KindArg::Shader => ContentKind::Shader,
        }
    }
}

#[derive(Subcommand)]
enum SkinCommand {
    /// Skins der Bibliothek anzeigen.
    List,
    /// Skin aus PNG-Datei oder vom Spielernamen in die Bibliothek übernehmen.
    Import {
        /// Pfad zu einer PNG-Datei oder ein Spielername.
        source: String,
        #[arg(long)]
        name: Option<String>,
        /// Schmale Arme (Alex); nur für Dateien, bei Spielern kommt das Modell von Mojang.
        #[arg(long)]
        slim: bool,
    },
    /// Skin aus der Bibliothek löschen.
    Delete { id: String },
    /// Skin aus der Bibliothek als Skin des aktiven Microsoft-Accounts setzen.
    Upload { id: String },
    /// Capes des aktiven Microsoft-Accounts anzeigen bzw. eins aktivieren (`none` blendet aus).
    Cape { id: Option<String> },
}

#[derive(Subcommand)]
enum SyncCommand {
    /// Sync-Packs anzeigen (mit --profile: an/aus für dieses Profil).
    List {
        #[arg(long)]
        profile: Option<String>,
    },
    /// Neues Sync-Pack (oder mit --template eine Vorlage) anlegen.
    Create {
        name: String,
        #[arg(long)]
        template: bool,
    },
    /// Modrinth-Projekt hinzufügen (Slug oder ID).
    Add {
        pack: String,
        project: String,
        #[arg(long = "type", value_enum, default_value = "mod")]
        kind: KindArg,
    },
    /// Eigene Datei hinzufügen; Ziel relativ zum Spielordner, z. B. config/x.json.
    AddFile {
        pack: String,
        file: PathBuf,
        target: String,
    },
    /// Option für options.txt setzen, z. B. fov 0.25.
    Option {
        pack: String,
        key: String,
        value: String,
    },
    /// Im Profil einschalten (Vorlagen: einmal einspielen).
    Enable { profile: String, pack: String },
    /// Im Profil ausschalten und eingespielte Inhalte entfernen.
    Disable { profile: String, pack: String },
    /// Eingeschaltete Sync-Packs eines Profils abgleichen.
    Run { profile: String },
    /// Sync-Pack löschen (eingespielte Inhalte bleiben).
    Delete { pack: String },
}

#[derive(Subcommand)]
enum ServerCommand {
    /// Status eines Servers abfragen (MOTD, Spieler, Ping).
    Ping { address: String },
    /// Gespeicherte Server eines Profils (servers.dat) anzeigen.
    List { profile: String },
}

#[derive(Subcommand)]
enum JavaCommand {
    /// Gefundene Java-Installationen anzeigen.
    List,
    /// Mojang-Runtime laden, z. B. `jre-legacy` (Java 8) oder `java-runtime-epsilon` (Java 25).
    Install { component: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let root = match cli.data_dir {
        Some(dir) => dir,
        None => Paths::default_root()?,
    };
    let paths = Paths::new(root);
    let launcher = Launcher::new(
        paths.clone(),
        Downloader::new()?.verify_existing(cli.verify),
    )?;
    let accounts = AccountStore::new(paths.accounts_file(), KeyringStore);
    let msa = cli
        .client_id
        .or_else(msa::client_id_from_env)
        .map(MsaClient::new)
        .transpose()?;

    match cli.command {
        Command::Versions { all, limit } => versions(launcher.installer(), all, limit).await,
        Command::Install { version } => {
            let v = install(launcher.installer(), &version).await?;
            println!("{} installiert in {}", v.id, paths.root().display());
            Ok(())
        }
        Command::Profile(cmd) => profile_command(cmd, &launcher).await,
        Command::Loader(LoaderCommand::Versions {
            kind,
            minecraft,
            limit,
        }) => {
            let versions = launcher.loaders().versions(kind, &minecraft).await?;
            if versions.is_empty() {
                println!("{} unterstützt Minecraft {minecraft} nicht.", kind.name());
            }
            for v in versions.iter().take(limit) {
                println!(
                    "  {:<20} {}",
                    v.version,
                    if v.stable { "stabil" } else { "Vorabversion" }
                );
            }
            Ok(())
        }
        Command::Java(JavaCommand::List) => {
            for j in launcher.java().installed().await {
                let tag = if j.managed { "verwaltet" } else { "System" };
                println!(
                    "Java {:<3} {:<14} {:<10} {}",
                    j.major,
                    j.version,
                    tag,
                    j.path.display()
                );
            }
            Ok(())
        }
        Command::Java(JavaCommand::Install { component }) => {
            let bar = progress_bar();
            let j = launcher
                .java()
                .install_runtime(&component, progress(&bar))
                .await?;
            bar.finish_and_clear();
            println!("Java {} installiert: {}", j.version, j.path.display());
            Ok(())
        }
        Command::Account(cmd) => account_command(cmd, &accounts, msa.as_ref()).await,
        Command::Run {
            target,
            account,
            offline,
            java,
            memory,
            print_command,
            no_update,
            server,
        } => {
            let quick_play = server.as_deref().map(ServerAddress::parse).transpose()?;
            let mut profile = match launcher.profiles().find(&target)? {
                Some(p) => p,
                // Ohne Profil: Version spontan starten (`latest` wird aufgelöst).
                None => Profile::new(
                    &target,
                    resolve_version(launcher.installer(), &target, no_update).await?,
                ),
            };
            if java.is_some() {
                profile.java = java;
            }
            if memory.is_some() {
                profile.memory_mb = memory;
            }

            // Erst anmelden, damit ein abgelaufener Login nicht erst nach dem Download auffällt.
            let session = match offline {
                Some(name) => Account::offline(&name)?,
                None => accounts.session(account.as_deref(), msa.as_ref()).await?,
            };
            eprintln!(
                "Spiele {} als {} ({})",
                profile.name,
                session.name,
                kind_label(session.kind)
            );

            let bar = progress_bar();
            let mut on_progress = progress(&bar);
            let mut on_event = |e: PrepareEvent| match e {
                PrepareEvent::Stage(stage) => {
                    bar.set_prefix(match stage {
                        Stage::Sync => "Sync-Packs",
                        Stage::Loader => "Mod-Loader",
                        Stage::Version => "Spieldateien",
                        Stage::Java => "Java",
                        Stage::Assets => "Assets",
                    });
                }
                PrepareEvent::Download(p) => on_progress(p),
                PrepareEvent::Java { version, path } => {
                    bar.suspend(|| eprintln!("Java {version} ({})", path.display()));
                }
            };
            let cmd = launcher
                .prepare(
                    &profile,
                    session.clone(),
                    no_update,
                    quick_play,
                    &mut on_event,
                )
                .await?;
            bar.finish_and_clear();

            if print_command {
                println!("{}", cmd.program.display());
                // Echte Zugriffstoken nicht im Terminal oder in Logs landen lassen.
                let token = &session.access_token;
                for arg in &cmd.args {
                    match session.kind {
                        AccountKind::Microsoft => {
                            println!("  {}", arg.replace(token.as_str(), "<token>"))
                        }
                        AccountKind::Offline => println!("  {arg}"),
                    }
                }
                return Ok(());
            }
            let started = now_secs();
            let (connections, status) = run(cmd).await?;
            // Nur gespeicherte Profile bekommen eine Statistik, auch nach Abstürzen.
            if !profile.id.is_empty() {
                match launcher.finish_session(&profile.id, started, &connections) {
                    Ok(s) => eprintln!(
                        "Spielzeit {}: {}",
                        profile.name,
                        format_duration(s.playtime_secs)
                    ),
                    Err(e) => eprintln!("Spielzeit nicht gespeichert: {e}"),
                }
            }
            anyhow::ensure!(status.success(), "Minecraft wurde mit {status} beendet");
            Ok(())
        }
        Command::Server(ServerCommand::Ping { address }) => {
            let addr = ServerAddress::parse(&address)?;
            let s = server_ping(&addr).await?;
            println!("{}", addr.display());
            println!("  {}", s.motd.replace('\n', "\n  "));
            println!(
                "  {} · {}/{} Spieler · {} ms",
                s.version, s.online, s.max, s.ping_ms
            );
            Ok(())
        }
        Command::Server(ServerCommand::List { profile }) => {
            let profile = launcher.profiles().get(&profile)?;
            let servers = read_servers_dat(&launcher.game_dir(&profile))?;
            if servers.is_empty() {
                println!("Keine gespeicherten Server in {:?}.", profile.name);
            }
            for s in servers {
                println!("  {:<28} {}", s.name, s.address);
            }
            Ok(())
        }
        Command::Content(cmd) => content_command(cmd, &launcher).await,
        Command::Pack(cmd) => pack_command(cmd, &launcher).await,
        Command::Sync(cmd) => sync_command(cmd, &launcher).await,
        Command::Diag { target, profile } => {
            let only = match profile {
                Some(p) => Some(vec![launcher.profiles().get(&p)?.id]),
                None => None,
            };
            let r = pinecone_core::diagnostics::export(&launcher, &target, only.as_deref(), None)
                .await?;
            println!(
                "{} geschrieben: {} Dateien, {} KB",
                r.path,
                r.files,
                r.bytes / 1024
            );
            Ok(())
        }
        Command::Skin(cmd) => skin_command(cmd, &launcher, &accounts, msa.as_ref()).await,
        Command::Stats => {
            let stats = launcher.stats().load()?;
            let profiles = launcher.profiles().list()?;
            if profiles.is_empty() {
                println!("Keine Profile.");
            }
            for p in &profiles {
                let s = stats.profiles.get(&p.id).cloned().unwrap_or_default();
                let server = s
                    .last_server
                    .map(|a| format!(" · zuletzt auf {}", a.display()))
                    .unwrap_or_default();
                println!(
                    "  {:<24} {:>8} in {} Sitzungen{server}",
                    p.name,
                    format_duration(s.playtime_secs),
                    s.sessions
                );
            }
            Ok(())
        }
    }
}

async fn content_command(cmd: ContentCommand, launcher: &Launcher) -> Result<()> {
    let content = launcher.content();
    let profile_of = |name: &str| -> Result<(Profile, PathBuf)> {
        let p = launcher.profiles().get(name)?;
        let dir = launcher.game_dir(&p);
        Ok((p, dir))
    };
    match cmd {
        ContentCommand::List { profile } => {
            let (p, dir) = profile_of(&profile)?;
            let list = content.identify(&p.id, &dir).await?;
            if list.is_empty() {
                println!("Keine Inhalte in {:?}.", p.name);
            }
            for c in list {
                let source = match c.source {
                    Source::Modrinth { .. } => "Modrinth",
                    Source::Curseforge { .. } => "CurseForge",
                    Source::Local => "lokal",
                };
                let state = if c.enabled { "" } else { " (deaktiviert)" };
                println!(
                    "  {:<28} {:<22} {:<10} {}{state}",
                    c.name,
                    c.version.unwrap_or_default(),
                    source,
                    c.key
                );
            }
        }
        ContentCommand::Search {
            query,
            r#type,
            profile,
            limit,
        } => {
            let kind: ContentKind = r#type.into();
            let mut q = SearchQuery {
                query,
                project_type: Some(match kind {
                    ContentKind::Mod => ProjectType::Mod,
                    ContentKind::Resourcepack => ProjectType::Resourcepack,
                    ContentKind::Shader => ProjectType::Shader,
                }),
                limit,
                ..Default::default()
            };
            if let Some(name) = profile {
                let (p, _) = profile_of(&name)?;
                q.loaders = loaders_for(&p, kind).context("Vanilla-Profile haben keine Mods")?;
                q.game_version = Some(p.minecraft);
            }
            let result = content.modrinth().search(&q).await?;
            println!("{} Treffer", result.total_hits);
            for h in result.hits {
                println!(
                    "  {:<24} {:<26} {:>11} Downloads · {}",
                    h.slug, h.title, h.downloads, h.author
                );
            }
        }
        ContentCommand::Install {
            profile,
            project,
            r#type,
            version,
        } => {
            let (p, dir) = profile_of(&profile)?;
            let report = content
                .install(&p, &dir, r#type.into(), &project, version.as_deref())
                .await?;
            for (name, v) in &report.installed {
                println!("  installiert: {name} {v}");
            }
            for name in &report.already_present {
                println!("  schon vorhanden: {name}");
            }
            for w in &report.warnings {
                println!("  Hinweis: {w}");
            }
        }
        ContentCommand::Updates { profile, apply } => {
            let (p, dir) = profile_of(&profile)?;
            content.identify(&p.id, &dir).await?;
            let updates = content.check_updates(&p, &dir).await?;
            if updates.is_empty() {
                println!("Alles aktuell.");
            }
            for u in &updates {
                println!(
                    "  {:<28} {} → {}",
                    u.name,
                    u.current.clone().unwrap_or_default(),
                    u.latest
                );
                if apply {
                    content.apply_update(&p, &dir, u).await?;
                }
            }
        }
        ContentCommand::Remove { profile, key } => {
            let (p, dir) = profile_of(&profile)?;
            content.remove(&p.id, &dir, &key)?;
            println!("{key} entfernt");
        }
    }
    Ok(())
}

async fn pack_command(cmd: PackCommand, launcher: &Launcher) -> Result<()> {
    let packs = launcher.packs();
    match cmd {
        PackCommand::Import {
            source,
            version,
            name,
        } => {
            let (archive, src) = if source.contains("://") {
                resolve_link(packs, &source).await?
            } else if let Some(slug) = source.strip_prefix("modrinth:") {
                packs.download_modrinth(slug, version.as_deref()).await?
            } else if let Some(id) = source.strip_prefix("twogg:") {
                packs.download_twogg(id, version.as_deref()).await?
            } else if let Some(id) = source.strip_prefix("curseforge:") {
                let file = version
                    .as_deref()
                    .map(str::parse)
                    .transpose()
                    .context("CurseForge-Datei-ID ist eine Zahl")?;
                let id = id.parse().context("CurseForge-Projekt-ID ist eine Zahl")?;
                packs.download_curseforge(id, file, None).await?
            } else {
                (PathBuf::from(&source), PackSource::File)
            };
            let result = packs.import(&archive, src, name).await?;
            let p = result.profile;
            apply_auto_sync(launcher, &p).await;
            let count = launcher
                .content()
                .list(&p.id, &launcher.game_dir(&p))?
                .len();
            println!(
                "Profil {:?} angelegt: Minecraft {}, {count} Inhalte",
                p.name, p.minecraft
            );
            for m in result.missing {
                println!(
                    "  Bitte manuell herunterladen und nach {} legen: {} ({})",
                    m.path, m.name, m.url
                );
            }
        }
        PackCommand::Update {
            profile,
            file,
            version,
            apply,
            take_pack,
            keep_mine,
        } => {
            let p = launcher.profiles().get(&profile)?;
            let info = packs
                .pack_info(&p.id)?
                .context("Profil stammt aus keinem Modpack")?;
            let (archive, src) = match (file, &info.source) {
                (Some(f), _) => (f, PackSource::File),
                (None, PackSource::Modrinth { project_id, .. }) => {
                    let target = match version {
                        Some(v) => v,
                        None => match packs.check_update(&p).await? {
                            Some(u) => {
                                println!("Update verfügbar: {} → {}", u.current, u.latest);
                                u.version_id
                            }
                            None => {
                                println!(
                                    "{} {} ist aktuell (Minecraft {}).",
                                    info.name, info.version, p.minecraft
                                );
                                return Ok(());
                            }
                        },
                    };
                    packs.download_modrinth(project_id, Some(&target)).await?
                }
                (None, PackSource::Curseforge { project_id, .. }) => {
                    let target: u64 = match version {
                        Some(v) => v.parse().context("CurseForge-Datei-ID ist eine Zahl")?,
                        None => match packs.check_update(&p).await? {
                            Some(u) => {
                                println!("Update verfügbar: {} → {}", u.current, u.latest);
                                u.version_id.parse()?
                            }
                            None => {
                                println!(
                                    "{} {} ist aktuell (Minecraft {}).",
                                    info.name, info.version, p.minecraft
                                );
                                return Ok(());
                            }
                        },
                    };
                    packs
                        .download_curseforge(*project_id, Some(target), None)
                        .await?
                }
                (None, PackSource::Twogg { pack_id, .. }) => {
                    let target = match version {
                        Some(v) => v,
                        None => match packs.check_update(&p).await? {
                            Some(u) => u.version_id,
                            None => {
                                println!("{} ist aktuell", info.name);
                                return Ok(());
                            }
                        },
                    };
                    packs.download_twogg(pack_id, Some(&target)).await?
                }
                (None, PackSource::File) => {
                    bail!("Pack ohne Online-Quelle: neue Version mit --file angeben")
                }
            };
            let plan = packs
                .plan_update(&p, &packs.read_archive(&archive).await?)
                .await?;
            println!(
                "{} {} → {} (Minecraft {})",
                plan.pack_name, plan.from_version, plan.to_version, plan.minecraft
            );
            for c in &plan.changes {
                println!("  {:<8} {}", format!("{:?}", c.action), c.path);
            }
            for k in &plan.kept {
                println!("  behalten {k} (eigene Änderung)");
            }
            for c in &plan.conflicts {
                println!(
                    "  KONFLIKT {} ({:?}, Vorschlag {:?})",
                    c.path, c.reason, c.default
                );
            }
            if apply {
                let mut choices = HashMap::new();
                choices.extend(take_pack.into_iter().map(|p| (p, Resolution::TakePack)));
                choices.extend(keep_mine.into_iter().map(|p| (p, Resolution::KeepMine)));
                let snap = packs.apply_update(&p, &archive, src, &choices).await?;
                println!(
                    "Aktualisiert. Snapshot {} ({} Dateien gesichert/neu)",
                    snap.id, snap.files
                );
            }
        }
        PackCommand::Rollback { profile, to } => {
            let p = launcher.profiles().get(&profile)?;
            match to {
                Some(id) => {
                    packs.rollback(&p, &id)?;
                    println!("Zurückgerollt auf den Stand vor Snapshot {id}");
                }
                None => {
                    for s in packs.snapshots(&p.id) {
                        println!(
                            "  {}  vorher Version {}  {} Dateien",
                            s.id, s.pack_version, s.files
                        );
                    }
                }
            }
        }
        PackCommand::Export {
            profile,
            target,
            version,
            summary,
            no_resourcepacks,
            no_shaders,
            no_config,
            with_options,
        } => {
            let p = launcher.profiles().get(&profile)?;
            let opts = ExportOptions {
                summary,
                resourcepacks: !no_resourcepacks,
                shaders: !no_shaders,
                config: !no_config,
                options: with_options,
                ..ExportOptions::new(version)
            };
            let r = packs.export(&p, &target, &opts).await?;
            println!(
                "{} geschrieben: {} verlinkt, {} im Archiv",
                r.path, r.linked, r.included
            );
        }
        PackCommand::Share { profile } => {
            let p = launcher.profiles().get(&profile)?;
            let info = packs
                .pack_info(&p.id)?
                .context("Profil stammt aus keinem Modpack; zum Teilen mit `pack export` als Datei speichern")?;
            let link = ShareLink::for_source(&info.source).context(
                "Pack wurde aus einer Datei importiert; zum Teilen mit `pack export` als Datei speichern",
            )?;
            println!("{link}");
        }
        PackCommand::Preview { source } => {
            let archive = if source.contains("://") {
                resolve_link(packs, &source).await?.0
            } else {
                PathBuf::from(&source)
            };
            let p = packs.preview(&archive).await?;
            println!("{} {}", p.name, p.version);
            if let Some(s) = &p.summary {
                println!("  {s}");
            }
            let loader = p
                .loader
                .map(|l| format!(" mit {:?} {}", l.kind, l.version))
                .unwrap_or_default();
            println!("  Minecraft {}{loader}", p.minecraft);
            println!(
                "  {} Mods, {} Ressourcenpakete, {} Shader, {} weitere Dateien",
                p.mods, p.resourcepacks, p.shaders, p.other_files
            );
            println!(
                "  Download {:.1} MB, Archiv {:.1} MB",
                p.download_size as f64 / 1e6,
                p.archive_size as f64 / 1e6
            );
            for m in &p.missing {
                println!("  Manuell herunterladen: {} ({})", m.name, m.url);
            }
            if !p.blocked_hosts.is_empty() {
                println!(
                    "  Achtung: Downloads von nicht erlaubten Quellen ({}), Installation schlägt fehl",
                    p.blocked_hosts.join(", ")
                );
            }
        }
    }
    Ok(())
}

/// Lädt das Pack hinter einem `pinecone://`- oder Modrinth-Link in den Cache.
async fn resolve_link(
    packs: &pinecone_core::modpack::PackManager,
    link: &str,
) -> Result<(PathBuf, PackSource)> {
    Ok(match ShareLink::parse(link)? {
        ShareLink::Modrinth { project, version } => {
            packs
                .download_modrinth(&project, version.as_deref())
                .await?
        }
        ShareLink::Curseforge { project, file } => {
            packs.download_curseforge(project, file, None).await?
        }
        ShareLink::Twogg { pack, version } => {
            packs.download_twogg(&pack, version.as_deref()).await?
        }
        ShareLink::Code(_) => bail!("Share-Codes gibt es erst mit dem Share-Dienst"),
    })
}

async fn skin_command(
    cmd: SkinCommand,
    launcher: &Launcher,
    accounts: &AccountStore,
    msa: Option<&MsaClient>,
) -> Result<()> {
    let lib = launcher.skins();
    let model_label = |m: SkinModel| match m {
        SkinModel::Classic => "classic",
        SkinModel::Slim => "slim",
    };
    match cmd {
        SkinCommand::List => {
            let list = lib.list()?;
            if list.is_empty() {
                println!("Keine Skins. Übernehmen mit `pinecone skin import <spieler|datei.png>`.");
            }
            for s in list {
                println!("  {:<20} {:<28} {}", s.id, s.name, model_label(s.model));
            }
        }
        SkinCommand::Import { source, name, slim } => {
            let path = PathBuf::from(&source);
            let skin = if path.is_file() {
                let model = if slim {
                    SkinModel::Slim
                } else {
                    SkinModel::Classic
                };
                lib.import_file(&path, name.as_deref(), model)?
            } else {
                let mut s = lib.import_player(&source).await?;
                if let Some(n) = name {
                    s = lib.rename(&s.id, &n)?;
                }
                s
            };
            println!(
                "Skin {:?} ({}) übernommen, ID {}",
                skin.name,
                model_label(skin.model),
                skin.id
            );
        }
        SkinCommand::Delete { id } => {
            lib.delete(&id)?;
            println!("Skin {id} gelöscht");
        }
        SkinCommand::Upload { id } => {
            let msa =
                msa.context("Microsoft-Login nicht konfiguriert (PINECONE_MSA_CLIENT_ID fehlt)")?;
            let skin = lib
                .list()?
                .into_iter()
                .find(|s| s.id == id)
                .context("Kein Skin mit dieser ID")?;
            let account = accounts.session(None, Some(msa)).await?;
            let profile = msa
                .upload_skin(&account.access_token, skin.model, lib.png(&id)?)
                .await?;
            println!("Skin {:?} für {} gesetzt", skin.name, profile.name);
        }
        SkinCommand::Cape { id } => {
            let msa =
                msa.context("Microsoft-Login nicht konfiguriert (PINECONE_MSA_CLIENT_ID fehlt)")?;
            let account = accounts.session(None, Some(msa)).await?;
            let profile = match id.as_deref() {
                None => msa.profile(&account.access_token).await?,
                Some("none") => msa.set_cape(&account.access_token, None).await?,
                Some(cape) => msa.set_cape(&account.access_token, Some(cape)).await?,
            };
            if profile.capes.is_empty() {
                println!("{} hat keine Capes.", profile.name);
            }
            for c in profile.capes {
                let mark = if c.state == "ACTIVE" { "*" } else { " " };
                println!("{mark} {:<38} {}", c.id, c.alias.unwrap_or_default());
            }
        }
    }
    Ok(())
}

/// Neues Profil: Sync-Packs mit Auto-Regel einschalten (Fehler nur melden).
async fn apply_auto_sync(launcher: &Launcher, profile: &Profile) {
    match launcher.syncpacks().apply_auto(profile).await {
        Ok(r) if !r.installed.is_empty() => {
            println!("Sync-Packs eingespielt: {}", r.installed.join(", "))
        }
        Ok(_) => {}
        Err(e) => eprintln!("Sync-Packs nicht eingespielt: {e}"),
    }
}

async fn sync_command(cmd: SyncCommand, launcher: &Launcher) -> Result<()> {
    let sync = launcher.syncpacks();
    let print = |r: SyncReport| {
        for (label, list) in [
            ("installiert", &r.installed),
            ("entfernt", &r.removed),
            ("schon vorhanden", &r.present),
            ("nicht verfügbar", &r.unavailable),
        ] {
            if !list.is_empty() {
                println!("  {label}: {}", list.join(", "));
            }
        }
        for w in &r.warnings {
            println!("  Hinweis: {w}");
        }
    };
    match cmd {
        SyncCommand::List { profile } => {
            let packs: Vec<(SyncPack, Option<bool>)> = match profile {
                Some(p) => {
                    let p = launcher.profiles().get(&p)?;
                    sync.for_profile(&p.id)?
                        .into_iter()
                        .map(|x| (x.pack, Some(x.enabled)))
                        .collect()
                }
                None => sync.list()?.into_iter().map(|p| (p, None)).collect(),
            };
            if packs.is_empty() {
                println!("Keine Sync-Packs. Anlegen mit `pinecone sync create <name>`.");
            }
            for (p, on) in packs {
                let mark = match on {
                    Some(true) => "[x] ",
                    Some(false) => "[ ] ",
                    None => "",
                };
                let mode = match p.mode {
                    SyncMode::Sync => "Sync-Pack",
                    SyncMode::Template => "Vorlage",
                };
                println!(
                    "{mark}{} ({}, {mode}): {} Einträge",
                    p.name,
                    p.id,
                    p.entries.len()
                );
                for e in &p.entries {
                    println!("    {}", e.label());
                }
                for (k, v) in &p.options {
                    println!("    options.txt: {k}:{v}");
                }
            }
        }
        SyncCommand::Create { name, template } => {
            let mode = if template {
                SyncMode::Template
            } else {
                SyncMode::Sync
            };
            let p = sync.create(&name, mode)?;
            println!("{:?} angelegt (ID {})", p.name, p.id);
        }
        SyncCommand::Add {
            pack,
            project,
            kind,
        } => {
            let mut p = sync.get(&pack)?;
            let info = launcher.content().modrinth().project(&project).await?;
            p.entries.push(SyncEntry::Modrinth {
                project_id: info.id,
                kind: kind.into(),
                name: info.title.clone(),
            });
            sync.save(&p)?;
            println!("{} zu {:?} hinzugefügt", info.title, p.name);
        }
        SyncCommand::AddFile { pack, file, target } => {
            let p = sync.add_file(&pack, &file, &target).await?;
            println!("{target} zu {:?} hinzugefügt", p.name);
        }
        SyncCommand::Option { pack, key, value } => {
            let mut p = sync.get(&pack)?;
            p.options.insert(key, value);
            sync.save(&p)?;
        }
        SyncCommand::Enable { profile, pack } => {
            let p = launcher.profiles().get(&profile)?;
            print(sync.set_enabled(&p, &pack, true).await?);
        }
        SyncCommand::Disable { profile, pack } => {
            let p = launcher.profiles().get(&profile)?;
            print(sync.set_enabled(&p, &pack, false).await?);
        }
        SyncCommand::Run { profile } => {
            let p = launcher.profiles().get(&profile)?;
            print(sync.sync(&p).await?);
        }
        SyncCommand::Delete { pack } => {
            sync.delete(&pack)?;
            println!("Sync-Pack {pack} gelöscht");
        }
    }
    Ok(())
}

fn format_duration(secs: u64) -> String {
    match secs {
        s if s < 60 => format!("{s} s"),
        s if s < 3600 => format!("{} min", s / 60),
        s => format!("{} h {} min", s / 3600, (s % 3600) / 60),
    }
}

/// Löst `latest` usw. über das Manifest auf und prüft, dass es die Version gibt.
async fn resolve_version(installer: &Installer, version: &str, offline: bool) -> Result<String> {
    if offline {
        return Ok(version.to_string());
    }
    let manifest = installer.manifest().await?;
    match manifest.find(version) {
        Some(v) => Ok(v.id.clone()),
        None => bail!("Weder ein Profil noch eine Minecraft-Version heißt {version:?}"),
    }
}

/// Loader-Version wählen: angegeben oder empfohlen, mit Hinweis bei Vorabversionen.
async fn pick_loader(
    launcher: &Launcher,
    kind: LoaderKind,
    minecraft: &str,
    version: Option<String>,
) -> Result<LoaderSpec> {
    let version = match version {
        Some(v) => v,
        None => {
            let rec = launcher.loaders().recommended(kind, minecraft).await?;
            if !rec.stable {
                eprintln!(
                    "Hinweis: Für Minecraft {minecraft} gibt es nur Vorabversionen von {}, nutze {}",
                    kind.name(),
                    rec.version
                );
            }
            rec.version
        }
    };
    Ok(LoaderSpec { kind, version })
}

async fn profile_command(cmd: ProfileCommand, launcher: &Launcher) -> Result<()> {
    let profiles = launcher.profiles();
    match cmd {
        ProfileCommand::Create {
            name,
            version,
            loader,
            loader_version,
            memory,
        } => {
            let minecraft = resolve_version(launcher.installer(), &version, false).await?;
            let mut profile = Profile::new(name, &minecraft);
            if let Some(kind) = loader {
                profile.loader =
                    Some(pick_loader(launcher, kind, &minecraft, loader_version).await?);
            }
            profile.memory_mb = memory;
            let profile = profiles.create(profile)?;
            apply_auto_sync(launcher, &profile).await;
            println!(
                "Profil {:?} angelegt ({}): {}",
                profile.name,
                profile.id,
                describe(&profile)
            );
        }
        ProfileCommand::List => {
            let list = profiles.list()?;
            if list.is_empty() {
                println!("Keine Profile. Anlegen mit `profile create <name> --version <version>`.");
            }
            for p in &list {
                println!("  {:<24} {:<20} {}", p.name, p.id, describe(p));
            }
        }
        ProfileCommand::Edit {
            name,
            rename,
            version,
            loader,
            loader_version,
            memory,
            java,
            jvm_args,
        } => {
            let mut profile = profiles.get(&name)?;
            if let Some(new_name) = rename {
                if profiles
                    .find(&new_name)?
                    .is_some_and(|p| p.id != profile.id)
                {
                    bail!("Es gibt schon ein Profil {new_name:?}");
                }
                profile.name = new_name;
            }
            let version_changed = version.is_some();
            if let Some(v) = version {
                profile.minecraft = resolve_version(launcher.installer(), &v, false).await?;
            }
            let kind = match loader.as_deref() {
                Some("none") => None,
                Some(k) => Some(k.parse::<LoaderKind>()?),
                None => profile.loader.as_ref().map(|l| l.kind),
            };
            let loader_changed = loader.is_some() || loader_version.is_some();
            profile.loader = match kind {
                None => None,
                // Loader-Version nur neu wählen, wenn sich etwas daran geändert hat.
                Some(kind) if version_changed || loader_changed || profile.loader.is_none() => {
                    Some(pick_loader(launcher, kind, &profile.minecraft, loader_version).await?)
                }
                Some(_) => profile.loader.clone(),
            };
            if memory.is_some() {
                profile.memory_mb = memory;
            }
            if let Some(j) = java {
                profile.java = (j != "auto").then(|| PathBuf::from(j));
            }
            if let Some(args) = jvm_args {
                profile.jvm_args = args.split_whitespace().map(str::to_string).collect();
            }
            profiles.save(&profile)?;
            println!(
                "Profil {:?} gespeichert: {}",
                profile.name,
                describe(&profile)
            );
        }
        ProfileCommand::Delete { name, yes } => {
            let profile = profiles.get(&name)?;
            if !yes {
                bail!(
                    "Das löscht {} mit allen Welten, Mods und Einstellungen. Zum Bestätigen `--yes` anhängen.",
                    launcher.paths().instance_dir(&profile.id).display()
                );
            }
            launcher.delete_profile(&profile.id)?;
            println!("Profil {:?} gelöscht", profile.name);
        }
        ProfileCommand::Migrate {
            name,
            minecraft,
            loader,
            new_name,
            no_worlds,
            drop_missing,
            apply,
        } => {
            let profile = profiles.get(&name)?;
            let loader = match loader.as_deref() {
                None => profile.loader.as_ref().map(|l| l.kind),
                Some("none" | "vanilla") => None,
                Some(other) => Some(other.parse()?),
            };
            let target = MigrationTarget { minecraft, loader };
            let plan = launcher.plan_migration(&profile, &target).await?;
            let loader_label = |l: &Option<LoaderSpec>| match l {
                Some(l) => format!("{} {}", l.kind.name(), l.version),
                None => "Vanilla".into(),
            };
            println!(
                "{} {} → {} {}",
                plan.from_minecraft,
                loader_label(&plan.from_loader),
                plan.minecraft,
                loader_label(&plan.loader)
            );
            if let Some(o) = &plan.pack_offer {
                println!(
                    "  Tipp: {} gibt es in Version {} für das Ziel: pinecone pack import {}:{} --version {}",
                    o.name, o.version, o.platform, o.project_id, o.version_id
                );
            }
            for item in &plan.items {
                let (mark, detail) = match item.status {
                    ItemStatus::Compatible => ("✓", "passt".to_string()),
                    ItemStatus::Update => (
                        "✓",
                        format!(
                            "{} → {}",
                            item.from_version.as_deref().unwrap_or("?"),
                            item.to_version.as_deref().unwrap_or("?")
                        ),
                    ),
                    ItemStatus::Substitute => (
                        "↺",
                        format!(
                            "ersetzt durch {}",
                            item.substitute.as_deref().unwrap_or("?")
                        ),
                    ),
                    ItemStatus::Kept => ("·", "bleibt unverändert".to_string()),
                    ItemStatus::Missing => (
                        "✗",
                        if drop_missing {
                            "nicht verfügbar, wird weggelassen"
                        } else {
                            "nicht verfügbar, wird deaktiviert"
                        }
                        .to_string(),
                    ),
                };
                println!("  {mark} {} ({}): {detail}", item.name, item.key);
            }
            if plan.downgrade && plan.worlds > 0 && !no_worlds {
                println!(
                    "  Achtung: {} Welt(en) werden in eine ältere Version kopiert und können dort kaputtgehen (Original bleibt erhalten).",
                    plan.worlds
                );
            }
            if !apply {
                println!("Mit --apply als neues Profil anlegen.");
                return Ok(());
            }
            let missing = plan
                .items
                .iter()
                .filter(|i| i.status == ItemStatus::Missing)
                .map(|i| {
                    let choice = if drop_missing {
                        MissingChoice::Drop
                    } else {
                        MissingChoice::Disable
                    };
                    (i.key.clone(), choice)
                })
                .collect();
            let result = launcher
                .migrate(
                    &profile,
                    &MigrateOptions {
                        target,
                        name: new_name,
                        include_worlds: !no_worlds,
                        missing,
                    },
                )
                .await?;
            println!(
                "Profil {:?} angelegt: {}",
                result.profile.name,
                describe(&result.profile)
            );
            for w in result.report.warnings {
                println!("  Hinweis: {w}");
            }
        }
    }
    Ok(())
}

fn describe(p: &Profile) -> String {
    let loader = match &p.loader {
        Some(l) => format!("{} {}", l.kind.name(), l.version),
        None => "Vanilla".into(),
    };
    let memory = p.memory_mb.map(|m| format!(", {m} MB")).unwrap_or_default();
    format!("Minecraft {} · {loader}{memory}", p.minecraft)
}

fn kind_label(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::Microsoft => "Microsoft",
        AccountKind::Offline => "offline",
    }
}

async fn account_command(
    cmd: AccountCommand,
    accounts: &AccountStore,
    msa: Option<&MsaClient>,
) -> Result<()> {
    match cmd {
        AccountCommand::Login => {
            let msa = msa.context(
                "Für den Microsoft-Login wird die Client-ID einer Azure-App benötigt \
                 (--client-id oder PINECONE_MSA_CLIENT_ID)",
            )?;
            let code = msa.request_device_code().await?;
            println!(
                "Öffne {} und gib den Code {} ein.",
                code.verification_uri, code.user_code
            );
            println!("Warte auf Bestätigung …");
            let tokens = msa.poll_device_code(&code).await?;
            let login = msa.login_minecraft(tokens).await?;
            let account = accounts.add_microsoft(&login)?;
            println!("Angemeldet als {} ({})", account.name, account.uuid);
        }
        AccountCommand::Offline { name } => {
            let account = accounts.add_offline(&name)?;
            println!("Offline-Account {} angelegt und aktiv", account.name);
        }
        AccountCommand::List => {
            let list = accounts.load()?;
            if list.accounts.is_empty() {
                println!("Keine Accounts gespeichert.");
            }
            for a in &list.accounts {
                let marker = if list.active == Some(a.uuid) {
                    "*"
                } else {
                    " "
                };
                println!(
                    "{marker} {:<16} {:<9} {}",
                    a.name,
                    kind_label(a.kind),
                    a.uuid
                );
            }
        }
        AccountCommand::Use { name } => {
            let account = accounts.set_active(&name)?;
            println!("Aktiv: {}", account.name);
        }
        AccountCommand::Remove { name } => {
            let account = accounts.remove(&name)?;
            println!("{} entfernt", account.name);
        }
    }
    Ok(())
}

async fn versions(installer: &Installer, all: bool, limit: usize) -> Result<()> {
    let manifest = installer.manifest().await?;
    println!(
        "Neueste: {} (Snapshot: {})",
        manifest.latest.release, manifest.latest.snapshot
    );
    for v in manifest
        .versions
        .iter()
        .filter(|v| all || v.kind == "release")
        .take(limit)
    {
        println!(
            "  {:<24} {:<10} {}",
            v.id,
            v.kind,
            &v.release_time[..10.min(v.release_time.len())]
        );
    }
    Ok(())
}

async fn install(installer: &Installer, version: &str) -> Result<VersionJson> {
    let bar = progress_bar();
    let v = installer
        .install(version, progress(&bar))
        .await
        .with_context(|| format!("Installation von {version} fehlgeschlagen"))?;
    bar.finish_and_clear();
    Ok(v)
}

fn progress_bar() -> ProgressBar {
    ProgressBar::new(0).with_style(
        ProgressStyle::with_template(
            "{spinner} {prefix:12} [{bar:40}] {pos}/{len} Dateien · {msg}",
        )
        .unwrap()
        .progress_chars("=> "),
    )
}

/// Überträgt Download-Fortschritt auf einen Fortschrittsbalken.
fn progress(bar: &ProgressBar) -> impl FnMut(Progress) + '_ {
    let mut total_bytes = 0;
    move |p| match p {
        Progress::Started { files, bytes } => {
            bar.set_length(files as u64);
            total_bytes = bytes;
        }
        Progress::FileDone {
            done, bytes_done, ..
        } => {
            bar.set_position(done as u64);
            bar.set_message(format!("{} / {} MB", bytes_done >> 20, total_bytes >> 20));
        }
    }
}

/// Startet das Spiel, gibt seine Ausgabe weiter und liefert die Zeilen zu Serververbindungen
/// (für „zuletzt gespielter Server“) sowie den Exit-Status, auch wenn das Spiel abstürzt.
async fn run(cmd: launch::LaunchCommand) -> Result<(String, std::process::ExitStatus)> {
    let mut child = cmd.spawn().await.with_context(|| {
        format!(
            "Java konnte nicht gestartet werden ({})",
            cmd.program.display()
        )
    })?;
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout als Pipe")).lines();
    let mut stderr = BufReader::new(child.stderr.take().expect("stderr als Pipe")).lines();
    let (mut out_open, mut err_open) = (true, true);
    let mut connections = String::new();
    while out_open || err_open {
        tokio::select! {
            line = stdout.next_line(), if out_open => match line? {
                Some(l) => {
                    if l.contains("Connecting to ") {
                        connections.push_str(&l);
                        connections.push('\n');
                    }
                    println!("{l}");
                }
                None => out_open = false,
            },
            line = stderr.next_line(), if err_open => match line? {
                Some(l) => eprintln!("{l}"),
                None => err_open = false,
            },
        }
    }
    let status = child.wait().await?;
    Ok((connections, status))
}
