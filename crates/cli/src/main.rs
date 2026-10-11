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
use pinecone_core::modpack::{PackSource, Resolution};
use pinecone_core::modrinth::{ProjectType, SearchQuery};
use pinecone_core::msa::{self, MsaClient};
use pinecone_core::paths::Paths;
use pinecone_core::profiles::Profile;
use pinecone_core::servers::{ServerAddress, ping as server_ping, read_servers_dat};
use pinecone_core::stats::now_secs;
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
    /// `.mrpack`-Datei oder Modrinth-Projekt (`modrinth:<slug>`) als neues Profil importieren.
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
    /// Profil als .mrpack exportieren.
    Export {
        profile: String,
        target: PathBuf,
        #[arg(long, default_value = "1.0.0")]
        version: String,
    },
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
            let (archive, src) = if let Some(slug) = source.strip_prefix("modrinth:") {
                packs.download_modrinth(slug, version.as_deref()).await?
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
        } => {
            let p = launcher.profiles().get(&profile)?;
            let r = packs.export(&p, &target, &version).await?;
            println!(
                "{} geschrieben: {} verlinkt, {} im Archiv",
                r.path, r.linked, r.included
            );
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
