//! Startbefehl bauen (JVM- und Spielargumente mit Platzhaltern) und Prozess starten.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::auth::Account;
use crate::manifest::{Argument, VersionJson};
use crate::paths::Paths;
use crate::rules::Environment;
use crate::servers::ServerAddress;
use crate::{Error, LAUNCHER_NAME, LAUNCHER_VERSION, Result};

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub java: PathBuf,
    pub account: Account,
    pub game_dir: PathBuf,
    /// Wert für `${game_assets}` (alte Asset-Formate), Standard: `assets/`.
    pub game_assets: Option<PathBuf>,
    pub memory_mb: Option<u32>,
    pub extra_jvm_args: Vec<String>,
    pub resolution: Option<(u32, u32)>,
    /// Direkt auf diesen Server verbinden (Quick-Join).
    pub quick_play: Option<ServerAddress>,
}

#[derive(Debug, Clone)]
pub struct LaunchCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

/// Standard-JVM-Argumente für Versionen vor 1.13 (nur `minecraftArguments`).
const LEGACY_JVM_ARGS: &[&str] = &[
    "-Djava.library.path=${natives_directory}",
    "-Dminecraft.launcher.brand=${launcher_name}",
    "-Dminecraft.launcher.version=${launcher_version}",
    "-cp",
    "${classpath}",
];

pub fn build_command(
    paths: &Paths,
    version: &VersionJson,
    opts: &LaunchOptions,
    env: &Environment,
) -> Result<LaunchCommand> {
    let main_class = version
        .main_class
        .clone()
        .ok_or_else(|| Error::Invalid(format!("Version {} hat keine mainClass", version.id)))?;

    let mut env = env.clone();
    env.features
        .insert("has_custom_resolution".into(), opts.resolution.is_some());
    // Ab 1.20 per Feature-Rule (`--quickPlayMultiplayer`), davor per `--server`/`--port`.
    let quick_play_feature = supports_quick_play(version);
    env.features.insert(
        "is_quick_play_multiplayer".into(),
        opts.quick_play.is_some() && quick_play_feature,
    );

    let separator = if env.os_name == "windows" { ";" } else { ":" };
    let classpath = classpath(paths, version, &env).join(separator);
    let vars = placeholders(paths, version, opts, classpath, separator);

    let resolve = |args: &[Argument]| -> Vec<String> {
        args.iter()
            .flat_map(|a| a.resolve(&env))
            .map(|a| substitute(&a, &vars))
            .collect()
    };

    let mut args = Vec::new();
    if let Some(mb) = opts.memory_mb {
        args.push(format!("-Xmx{mb}M"));
    }
    args.extend(opts.extra_jvm_args.iter().cloned());

    match &version.arguments {
        Some(a) => args.extend(resolve(&a.jvm)),
        None => args.extend(LEGACY_JVM_ARGS.iter().map(|a| substitute(a, &vars))),
    }
    args.push(main_class);
    match (&version.arguments, &version.minecraft_arguments) {
        (_, Some(legacy)) => args.extend(legacy.split_whitespace().map(|a| substitute(a, &vars))),
        (Some(a), None) => args.extend(resolve(&a.game)),
        (None, None) => {}
    }
    // Alte Versionen kennen die Auflösungs-Feature-Rules nicht.
    if let (Some((w, h)), None) = (opts.resolution, &version.arguments) {
        args.extend([
            "--width".into(),
            w.to_string(),
            "--height".into(),
            h.to_string(),
        ]);
    }
    if let (Some(server), false) = (&opts.quick_play, quick_play_feature) {
        args.extend([
            "--server".into(),
            server.host.clone(),
            "--port".into(),
            server.port_or_default().to_string(),
        ]);
    }

    Ok(LaunchCommand {
        program: opts.java.clone(),
        args,
        cwd: opts.game_dir.clone(),
    })
}

/// Kennt die Version die Feature-Rule `is_quick_play_multiplayer` (ab 1.20)?
fn supports_quick_play(version: &VersionJson) -> bool {
    version.arguments.as_ref().is_some_and(|a| {
        a.game.iter().any(|arg| match arg {
            Argument::Conditional { rules, .. } => rules.iter().any(|r| {
                r.features
                    .as_ref()
                    .is_some_and(|f| f.contains_key("is_quick_play_multiplayer"))
            }),
            Argument::Plain(_) => false,
        })
    })
}

fn classpath(paths: &Paths, version: &VersionJson, env: &Environment) -> Vec<String> {
    let libraries = paths.libraries();
    let mut seen = HashSet::new();
    let mut entries: Vec<String> = version
        .libraries
        .iter()
        .filter(|l| l.applies(env))
        .filter_map(|l| l.artifact())
        .map(|a| libraries.join(a.path).display().to_string())
        .filter(|p| seen.insert(p.clone()))
        .collect();
    entries.push(paths.client_jar(version.jar_id()).display().to_string());
    entries
}

fn placeholders(
    paths: &Paths,
    version: &VersionJson,
    opts: &LaunchOptions,
    classpath: String,
    separator: &str,
) -> HashMap<&'static str, String> {
    let path = |p: &Path| p.display().to_string();
    let assets_index = version
        .asset_index
        .as_ref()
        .map(|a| a.id.clone())
        .or_else(|| version.assets.clone())
        .unwrap_or_else(|| "legacy".into());
    let (width, height) = opts.resolution.unwrap_or((854, 480));
    let account = &opts.account;

    HashMap::from([
        ("auth_player_name", account.name.clone()),
        ("auth_uuid", account.uuid.simple().to_string()),
        ("auth_access_token", account.access_token.clone()),
        ("auth_session", account.access_token.clone()),
        ("auth_xuid", "0".into()),
        ("clientid", String::new()),
        ("user_type", account.user_type().into()),
        ("user_properties", "{}".into()),
        ("version_name", version.id.clone()),
        (
            "version_type",
            version.kind.clone().unwrap_or_else(|| "release".into()),
        ),
        ("game_directory", path(&opts.game_dir)),
        ("assets_root", path(&paths.assets())),
        (
            "game_assets",
            path(opts.game_assets.as_deref().unwrap_or(&paths.assets())),
        ),
        ("assets_index_name", assets_index),
        ("natives_directory", path(&paths.natives_dir(&version.id))),
        ("library_directory", path(&paths.libraries())),
        ("classpath", classpath),
        ("classpath_separator", separator.into()),
        ("launcher_name", LAUNCHER_NAME.into()),
        ("launcher_version", LAUNCHER_VERSION.into()),
        ("resolution_width", width.to_string()),
        ("resolution_height", height.to_string()),
        (
            "quickPlayMultiplayer",
            opts.quick_play
                .as_ref()
                .map(|s| s.display())
                .unwrap_or_default(),
        ),
    ])
}

/// Ersetzt alle `${name}` in `arg`. Unbekannte Platzhalter bleiben stehen.
pub fn substitute(arg: &str, vars: &HashMap<&'static str, String>) -> String {
    let mut out = String::with_capacity(arg.len());
    let mut rest = arg;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                match vars.get(key) {
                    Some(value) => out.push_str(value),
                    None => out.push_str(&rest[start..start + 2 + end + 1]),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

impl LaunchCommand {
    async fn command(&self) -> Result<tokio::process::Command> {
        tokio::fs::create_dir_all(&self.cwd)
            .await
            .map_err(Error::io(&self.cwd))?;
        let mut cmd = tokio::process::Command::new(&self.program);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: kein Konsolenfenster neben dem Spiel
        cmd.args(&self.args)
            .current_dir(&self.cwd)
            .stdin(Stdio::null());
        Ok(cmd)
    }

    /// Startet das Spiel mit stdout/stderr als Pipe. Endet der Launcher, endet auch das Spiel,
    /// sobald es wieder ins Log schreibt.
    pub async fn spawn(&self) -> Result<tokio::process::Child> {
        self.command()
            .await?
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(Error::io(&self.program))
    }

    /// Startet das Spiel mit stdout/stderr in `log` (wird überschrieben). Das Spiel läuft
    /// unabhängig vom Launcher weiter; das Log kann parallel mitgelesen werden.
    pub async fn spawn_logged(&self, log: &Path) -> Result<tokio::process::Child> {
        if let Some(parent) = log.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(Error::io(parent))?;
        }
        let out = std::fs::File::create(log).map_err(Error::io(log))?;
        let err = out.try_clone().map_err(Error::io(log))?;
        self.command()
            .await?
            .stdout(out)
            .stderr(err)
            .spawn()
            .map_err(Error::io(&self.program))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(root: &Path) -> LaunchOptions {
        LaunchOptions {
            java: "java".into(),
            account: Account::offline("Notch").unwrap(),
            game_dir: root.join("instances/test"),
            game_assets: None,
            quick_play: None,
            memory_mb: Some(4096),
            extra_jvm_args: vec![],
            resolution: None,
        }
    }

    fn linux() -> Environment {
        Environment {
            os_name: "linux".into(),
            arch: "x86_64".into(),
            os_version: String::new(),
            features: HashMap::new(),
        }
    }

    fn after<'a>(args: &'a [String], flag: &str) -> &'a str {
        let i = args
            .iter()
            .position(|a| a == flag)
            .unwrap_or_else(|| panic!("{flag} fehlt"));
        &args[i + 1]
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn spawn_logged_writes_stdout_and_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let cmd = LaunchCommand {
            program: "sh".into(),
            args: vec!["-c".into(), "echo aus-stdout; echo aus-stderr >&2".into()],
            cwd: dir.path().join("spiel"),
        };
        let log = dir.path().join("logs/latest.log");
        let status = cmd.spawn_logged(&log).await.unwrap().wait().await.unwrap();
        assert!(status.success());
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(
            text.contains("aus-stdout") && text.contains("aus-stderr"),
            "{text}"
        );
        assert!(dir.path().join("spiel").is_dir());
    }

    #[test]
    fn substitute_placeholders() {
        let vars = HashMap::from([("a", "1".to_string()), ("b", "zwei".to_string())]);
        assert_eq!(substitute("x=${a},y=${b}", &vars), "x=1,y=zwei");
        assert_eq!(substitute("${unbekannt}", &vars), "${unbekannt}");
        assert_eq!(substitute("kaputt ${a", &vars), "kaputt ${a");
        assert_eq!(substitute("ohne", &vars), "ohne");
    }

    #[test]
    fn modern_command() {
        let paths = Paths::new("/data");
        let v: VersionJson =
            serde_json::from_str(include_str!("../tests/fixtures/modern.json")).unwrap();
        let cmd = build_command(&paths, &v, &opts(Path::new("/data")), &linux()).unwrap();
        let args = &cmd.args;

        assert_eq!(args[0], "-Xmx4096M");
        assert!(args.contains(&"-Djava.library.path=/data/versions/1.21.1/natives".to_string()));
        assert!(args.contains(&"-Dminecraft.launcher.brand=twogg-pinecone".to_string()));

        let cp: Vec<&str> = after(args, "-cp").split(':').collect();
        assert_eq!(cp.last(), Some(&"/data/versions/1.21.1/1.21.1.jar"));
        assert!(
            cp.contains(&"/data/libraries/org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar")
        );
        assert!(!cp.iter().any(|p| p.contains("natives-windows")));

        let main = args
            .iter()
            .position(|a| a == "net.minecraft.client.main.Main")
            .unwrap();
        assert!(main > args.iter().position(|a| a == "-cp").unwrap());
        assert_eq!(after(args, "--username"), "Notch");
        assert_eq!(after(args, "--uuid"), "b50ad385829d3141a2167e7d7539ba7f");
        assert_eq!(after(args, "--assetIndex"), "17");
        assert_eq!(after(args, "--userType"), "legacy");
        assert!(!args.contains(&"--demo".to_string()));
        assert!(!args.contains(&"--width".to_string()));
        assert!(!args.iter().any(|a| a.contains("${")), "{args:?}");
    }

    #[test]
    fn legacy_command_with_resolution() {
        let paths = Paths::new("/data");
        let v: VersionJson =
            serde_json::from_str(include_str!("../tests/fixtures/legacy.json")).unwrap();
        let mut o = opts(Path::new("/data"));
        o.resolution = Some((1280, 720));
        let cmd = build_command(&paths, &v, &o, &linux()).unwrap();
        let args = &cmd.args;

        assert!(args.contains(&"-Djava.library.path=/data/versions/1.8.9/natives".to_string()));
        let cp = after(args, "-cp");
        assert!(cp.contains("lwjgl-2.9.4-nightly-20150209.jar"));
        assert!(
            !cp.contains("natives-linux"),
            "alte Natives gehören nicht in den Classpath"
        );
        assert_eq!(after(args, "--assetIndex"), "1.8");
        assert_eq!(after(args, "--userProperties"), "{}");
        assert_eq!(after(args, "--width"), "1280");
        assert!(!args.iter().any(|a| a.contains("${")), "{args:?}");
    }

    #[test]
    fn quick_play_modern_and_legacy() {
        let paths = Paths::new("/data");
        let server = ServerAddress {
            host: "mc.twogg.de".into(),
            port: Some(25570),
        };
        let mut o = opts(Path::new("/data"));
        o.quick_play = Some(server);

        let mut modern: VersionJson =
            serde_json::from_str(include_str!("../tests/fixtures/modern.json")).unwrap();
        modern.arguments.as_mut().unwrap().game.push(
            serde_json::from_str(
                r#"{"rules":[{"action":"allow","features":{"is_quick_play_multiplayer":true}}],
                    "value":["--quickPlayMultiplayer","${quickPlayMultiplayer}"]}"#,
            )
            .unwrap(),
        );
        let args = build_command(&paths, &modern, &o, &linux()).unwrap().args;
        assert_eq!(after(&args, "--quickPlayMultiplayer"), "mc.twogg.de:25570");
        assert!(!args.contains(&"--server".to_string()));

        // Ohne Server kein Quick-Play-Argument.
        let args = build_command(&paths, &modern, &opts(Path::new("/data")), &linux())
            .unwrap()
            .args;
        assert!(!args.contains(&"--quickPlayMultiplayer".to_string()));

        let legacy: VersionJson =
            serde_json::from_str(include_str!("../tests/fixtures/legacy.json")).unwrap();
        let args = build_command(&paths, &legacy, &o, &linux()).unwrap().args;
        assert_eq!(after(&args, "--server"), "mc.twogg.de");
        assert_eq!(after(&args, "--port"), "25570");
    }

    #[test]
    fn windows_classpath_separator() {
        let paths = Paths::new("C:/data");
        let v: VersionJson =
            serde_json::from_str(include_str!("../tests/fixtures/modern.json")).unwrap();
        let mut env = linux();
        env.os_name = "windows".into();
        let cmd = build_command(&paths, &v, &opts(Path::new("C:/data")), &env).unwrap();
        let cp = after(&cmd.args, "-cp");
        assert!(
            cp.contains(';') && cp.contains("natives-windows") && !cp.contains("natives-linux")
        );
        assert!(cmd.args.iter().any(|a| a.starts_with("-XX:HeapDumpPath=")));
    }
}
