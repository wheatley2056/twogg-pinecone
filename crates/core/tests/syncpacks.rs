//! Sync-Packs und Vorlagen gegen einen nachgebauten Modrinth-Server.

mod common;

use std::collections::BTreeMap;

use common::MockServer;
use pinecone_core::content::ContentKind;
use pinecone_core::download::Downloader;
use pinecone_core::launcher::Launcher;
use pinecone_core::loader::{LoaderKind, LoaderSpec};
use pinecone_core::modrinth::Modrinth;
use pinecone_core::paths::Paths;
use pinecone_core::profiles::Profile;
use pinecone_core::syncpacks::{AutoRule, SyncEntry, SyncMode};
use serde_json::json;

fn sha1(data: &str) -> String {
    use sha1::{Digest, Sha1};
    format!("{:x}", Sha1::digest(data.as_bytes()))
}

fn version(
    server: &MockServer,
    project: &str,
    file: &str,
    deps: serde_json::Value,
) -> serde_json::Value {
    let body = format!("inhalt-{file}");
    json!({
        "id": format!("{project}-v1"), "project_id": project, "name": project, "version_number": "1.0",
        "version_type": "release", "loaders": ["fabric"], "game_versions": ["1.21.1"],
        "date_published": "2026-10-01T00:00:00Z",
        "files": [{ "hashes": { "sha1": sha1(&body) }, "url": format!("{}/files/{file}", server.base),
                    "filename": file, "primary": true, "size": body.len() }],
        "dependencies": deps
    })
}

fn modrinth(project: &str, kind: ContentKind) -> SyncEntry {
    SyncEntry::Modrinth {
        project_id: project.into(),
        kind,
        name: project.into(),
    }
}

async fn setup() -> (MockServer, tempfile::TempDir, Launcher) {
    let server = MockServer::start().await;
    let sodium = version(
        &server,
        "sodium",
        "sodium.jar",
        json!([{ "project_id": "fabric-api", "dependency_type": "required" }]),
    );
    let api = version(&server, "fabric-api", "fabric-api.jar", json!([]));
    let faithful = version(&server, "faithful", "faithful.zip", json!([]));
    server.on(
        "GET",
        "/v2/project/sodium/version",
        200,
        json!([sodium]).to_string(),
    );
    server.on(
        "GET",
        "/v2/project/fabric-api/version",
        200,
        json!([api]).to_string(),
    );
    server.on(
        "GET",
        "/v2/project/faithful/version",
        200,
        json!([faithful]).to_string(),
    );
    server.on("GET", "/v2/project/gone/version", 200, "[]");
    server.on("POST", "/v2/version_files", 200, "{}");
    server.on("GET", "/v2/projects", 200, "[]");
    for file in ["sodium.jar", "fabric-api.jar", "faithful.zip"] {
        server.on(
            "GET",
            &format!("/files/{file}"),
            200,
            format!("inhalt-{file}"),
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let launcher = Launcher::with_services(
        Paths::new(dir.path().join("data")),
        Downloader::new().unwrap(),
        Modrinth::with_base(format!("{}/v2", server.base)).unwrap(),
        None,
    )
    .unwrap();
    (server, dir, launcher)
}

fn profile(launcher: &Launcher, name: &str, loader: Option<LoaderKind>) -> Profile {
    let mut p = Profile::new(name, "1.21.1");
    p.loader = loader.map(|kind| LoaderSpec {
        kind,
        version: "0.19.5".into(),
    });
    launcher.profiles().create(p).unwrap()
}

#[tokio::test]
async fn sync_pack_installs_tracks_and_removes_only_its_content() {
    let (_server, dir, launcher) = setup().await;
    let sync = launcher.syncpacks();
    let mut pack = sync.create("Meine Basics", SyncMode::Sync).unwrap();
    assert_eq!(pack.id, "meine-basics");
    pack.entries = vec![
        modrinth("sodium", ContentKind::Mod),
        modrinth("faithful", ContentKind::Resourcepack),
        modrinth("gone", ContentKind::Mod),
    ];
    pack.options = BTreeMap::from([("fov".into(), "0.25".into())]);
    sync.save(&pack).unwrap();
    let cfg = dir.path().join("sodium-options.json");
    std::fs::write(&cfg, "{\"quality\":1}").unwrap();
    sync.add_file(&pack.id, &cfg, "config/sodium-options.json")
        .await
        .unwrap();

    // Der Nutzer hat Fabric API selbst installiert: Sodium nutzt sie, sie bleibt aber seine.
    let p = profile(&launcher, "A", Some(LoaderKind::Fabric));
    let game = launcher.game_dir(&p);
    launcher
        .content()
        .install(&p, &game, ContentKind::Mod, "fabric-api", None)
        .await
        .unwrap();

    let report = sync.set_enabled(&p, &pack.id, true).await.unwrap();
    assert_eq!(
        report.installed,
        ["sodium", "faithful", "config/sodium-options.json"]
    );
    assert_eq!(report.unavailable, ["gone"]);
    assert!(game.join("mods/sodium.jar").is_file());
    assert!(game.join("resourcepacks/faithful.zip").is_file());
    assert_eq!(
        std::fs::read_to_string(game.join("config/sodium-options.json")).unwrap(),
        "{\"quality\":1}"
    );
    assert_eq!(
        std::fs::read_to_string(game.join("options.txt")).unwrap(),
        "version:2147483647\nfov:0.25\n"
    );
    let lock = launcher.content().load_lock(&p.id).unwrap();
    assert_eq!(
        lock.files["mods/sodium.jar"].sync.as_deref(),
        Some("meine-basics")
    );
    assert_eq!(lock.files["mods/fabric-api.jar"].sync, None);
    let listed = launcher.content().list(&p.id, &game).unwrap();
    assert!(
        listed
            .iter()
            .any(|c| c.key == "mods/sodium.jar" && c.sync.as_deref() == Some("meine-basics"))
    );
    assert!(sync.for_profile(&p.id).unwrap()[0].enabled);

    // Eigene Änderung an der Config bleibt beim nächsten Abgleich.
    std::fs::write(game.join("config/sodium-options.json"), "{\"quality\":2}").unwrap();
    let again = sync.sync(&p).await.unwrap();
    assert!(again.installed.is_empty(), "{:?}", again.installed);
    assert_eq!(
        std::fs::read_to_string(game.join("config/sodium-options.json")).unwrap(),
        "{\"quality\":2}"
    );

    // Aus dem Sync-Pack entfernt → beim Abgleich aus dem Profil entfernt.
    let mut current = sync.get(&pack.id).unwrap();
    current.entries.retain(|e| e.label() != "faithful");
    sync.save(&current).unwrap();
    let r = sync.sync(&p).await.unwrap();
    assert_eq!(r.removed, ["faithful"]);
    assert!(!game.join("resourcepacks/faithful.zip").exists());

    // Ausschalten: Sodium weg, Fabric API (vom Nutzer) und geänderte Config bleiben.
    let off = sync.set_enabled(&p, &pack.id, false).await.unwrap();
    assert_eq!(off.removed, ["sodium"]);
    assert!(!game.join("mods/sodium.jar").exists());
    assert!(game.join("mods/fabric-api.jar").is_file());
    assert!(game.join("config/sodium-options.json").is_file());
    assert!(!sync.for_profile(&p.id).unwrap()[0].enabled);

    // Löschen entfernt die Markierung in Profilen.
    sync.set_enabled(&p, &pack.id, true).await.unwrap();
    sync.delete(&pack.id).unwrap();
    let lock = launcher.content().load_lock(&p.id).unwrap();
    assert!(lock.files.values().all(|e| e.sync.is_none()));
    assert!(sync.list().unwrap().is_empty());
}

#[tokio::test]
async fn templates_apply_once_and_auto_rules_pick_profiles() {
    let (_server, _dir, launcher) = setup().await;
    let sync = launcher.syncpacks();
    let mut template = sync.create("Start-Mods", SyncMode::Template).unwrap();
    template.entries = vec![modrinth("sodium", ContentKind::Mod)];
    template.auto = AutoRule {
        enabled: true,
        loaders: vec![Some(LoaderKind::Fabric)],
    };
    sync.save(&template).unwrap();

    let fabric = profile(&launcher, "Fabric", Some(LoaderKind::Fabric));
    let vanilla = profile(&launcher, "Vanilla", None);
    let r = sync.apply_auto(&fabric).await.unwrap();
    assert_eq!(r.installed, ["sodium"]);
    assert!(
        sync.apply_auto(&vanilla)
            .await
            .unwrap()
            .installed
            .is_empty()
    );

    // Vorlage: normaler Inhalt ohne Markierung, nicht als „eingeschaltet“ geführt.
    let lock = launcher.content().load_lock(&fabric.id).unwrap();
    assert_eq!(lock.files["mods/sodium.jar"].sync, None);
    assert!(!sync.for_profile(&fabric.id).unwrap()[0].enabled);
    assert!(
        launcher
            .game_dir(&vanilla)
            .join("mods/sodium.jar")
            .metadata()
            .is_err()
    );
}

#[tokio::test]
async fn rejects_unsafe_files_and_names() {
    let (_server, dir, launcher) = setup().await;
    let sync = launcher.syncpacks();
    assert!(sync.create("   ", SyncMode::Sync).is_err());
    let pack = sync.create("X", SyncMode::Sync).unwrap();
    let src = dir.path().join("f");
    std::fs::write(&src, "x").unwrap();
    assert!(sync.add_file(&pack.id, &src, "../ausbruch").await.is_err());
    assert!(sync.add_file(&pack.id, &src, "servers.dat").await.is_err());
    assert!(sync.get("../x").is_err());
    let mut bad = sync.get(&pack.id).unwrap();
    bad.options.insert("a:b".into(), "1".into());
    assert!(sync.save(&bad).is_err());
}

#[tokio::test]
async fn first_enable_replaces_generated_config_and_options() {
    let (_server, dir, launcher) = setup().await;
    let sync = launcher.syncpacks();
    let mut pack = sync.create("Steuerung", SyncMode::Sync).unwrap();
    pack.options = BTreeMap::from([
        ("fov".into(), "0.25".into()),
        ("key_key.zoom".into(), "key.keyboard.c".into()),
    ]);
    sync.save(&pack).unwrap();
    let cfg = dir.path().join("zoom.json");
    std::fs::write(&cfg, "{\"zoom\":4}").unwrap();
    sync.add_file(&pack.id, &cfg, "config/zoom.json")
        .await
        .unwrap();

    // Die Instanz wurde schon gespielt: Minecraft und die Mod haben ihre Standardwerte geschrieben.
    let p = profile(&launcher, "Gespielt", Some(LoaderKind::Fabric));
    let game = launcher.game_dir(&p);
    std::fs::create_dir_all(game.join("config")).unwrap();
    std::fs::write(game.join("config/zoom.json"), "{\"zoom\":2}").unwrap();
    std::fs::write(
        game.join("options.txt"),
        "version:3955\nfov:0.0\nkey_key.zoom:key.keyboard.z\n",
    )
    .unwrap();

    sync.set_enabled(&p, &pack.id, true).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(game.join("config/zoom.json")).unwrap(),
        "{\"zoom\":4}"
    );
    assert_eq!(
        std::fs::read_to_string(game.join("options.txt")).unwrap(),
        "version:3955\nfov:0.25\nkey_key.zoom:key.keyboard.c\n"
    );
    assert_eq!(
        sync.enabled_in(&pack.id).unwrap(),
        std::slice::from_ref(&p.id)
    );
    assert_eq!(
        sync.profile_options(&p.id)["key_key.zoom"],
        "key.keyboard.c"
    );

    // Danach geänderte Werte bleiben.
    std::fs::write(game.join("options.txt"), "version:3955\nfov:0.5\n").unwrap();
    sync.sync(&p).await.unwrap();
    let text = std::fs::read_to_string(game.join("options.txt")).unwrap();
    assert!(
        text.contains("fov:0.5") && text.contains("key_key.zoom:key.keyboard.c"),
        "{text}"
    );
}
