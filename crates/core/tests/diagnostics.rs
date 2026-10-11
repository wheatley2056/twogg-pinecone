//! Diagnose-Export: Inhalt des ZIPs und Schwärzen von Tokens.

use std::io::Read;

use pinecone_core::diagnostics;
use pinecone_core::download::Downloader;
use pinecone_core::launcher::Launcher;
use pinecone_core::modrinth::Modrinth;
use pinecone_core::paths::Paths;
use pinecone_core::profiles::Profile;

#[tokio::test]
async fn exports_logs_crashes_and_config_without_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path().join("data"));
    let launcher = Launcher::with_services(
        paths.clone(),
        Downloader::new().unwrap(),
        Modrinth::with_base("http://127.0.0.1:9/v2").unwrap(),
        None,
    )
    .unwrap();
    let a = launcher
        .profiles()
        .create(Profile::new("A", "1.21.1"))
        .unwrap();
    let b = launcher
        .profiles()
        .create(Profile::new("B", "1.8.9"))
        .unwrap();
    let game = launcher.game_dir(&a);
    let token = "eyJhbGciOiJIUzI1NiJ9.eyJ4dWlkIjoiMTIzNDU2Nzg5MCJ9.c2lnbmF0dXJlc2lnbmF0dXJl";
    for (rel, body) in [
        (
            "logs/latest.log",
            format!("[main/INFO]: Setting user: Tobi\nargs --accessToken {token}\n"),
        ),
        ("logs/pinecone-console.log", "Konsole\n".to_string()),
        (
            "crash-reports/crash-1.txt",
            "---- Minecraft Crash Report ----".to_string(),
        ),
        ("mods/eigene.jar", "jar".to_string()),
    ] {
        let p = game.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    std::fs::write(paths.accounts_file(), r#"{"accounts":[{"name":"geheim"}]}"#).unwrap();
    let logs = dir.path().join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("pinecone.log.2026-10-11"), "INFO gestartet\n").unwrap();

    let out = dir.path().join("diag.zip");
    let report = diagnostics::export(
        &launcher,
        &out,
        Some(std::slice::from_ref(&a.id)),
        Some(&logs),
    )
    .await
    .unwrap();
    assert!(report.bytes > 0);

    let mut zip = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let mut names: Vec<String> = zip.file_names().map(str::to_string).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "info.txt",
            "launcher/pinecone.log.2026-10-11",
            "profiles/a/crash-reports/crash-1.txt",
            "profiles/a/inhalte.txt",
            "profiles/a/latest.log",
            "profiles/a/pinecone-console.log",
            "profiles/a/profile.json",
        ]
    );
    assert!(
        !names.iter().any(|n| n.contains(&b.id)),
        "nur gewählte Profile"
    );
    let mut read = |name: &str| {
        let mut s = String::new();
        zip.by_name(name).unwrap().read_to_string(&mut s).unwrap();
        s
    };
    let log = read("profiles/a/latest.log");
    assert!(
        log.contains("Setting user: Tobi") && !log.contains(token),
        "{log}"
    );
    assert!(read("profiles/a/inhalte.txt").contains("mods/eigene.jar"));
    assert!(read("info.txt").contains("A (a): Minecraft 1.21.1"));
}
