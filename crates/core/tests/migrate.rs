//! Versionswechsel gegen einen nachgebauten Modrinth-Server.
//! Ziel ist Vanilla, damit keine Loader-Meta-API gebraucht wird.

mod common;

use std::collections::HashMap;

use common::MockServer;
use pinecone_core::download::Downloader;
use pinecone_core::launcher::Launcher;
use pinecone_core::loader::{LoaderKind, LoaderSpec};
use pinecone_core::migrate::{ItemStatus, MigrateOptions, MigrationTarget, MissingChoice};
use pinecone_core::modrinth::Modrinth;
use pinecone_core::paths::Paths;
use pinecone_core::profiles::Profile;
use serde_json::json;

fn sha1(data: &str) -> String {
    use sha1::{Digest, Sha1};
    format!("{:x}", Sha1::digest(data.as_bytes()))
}

fn version(
    server: &MockServer,
    id: &str,
    project: &str,
    number: &str,
    file: &str,
) -> serde_json::Value {
    let body = format!("inhalt-{file}");
    json!({
        "id": id, "project_id": project, "name": format!("{project} {number}"), "version_number": number,
        "version_type": "release", "loaders": ["minecraft"], "game_versions": ["1.21.1"],
        "date_published": "2026-10-01T00:00:00Z",
        "files": [{ "hashes": { "sha1": sha1(&body) }, "url": format!("{}/files/{file}", server.base),
                    "filename": file, "primary": true, "size": body.len() }],
        "dependencies": []
    })
}

#[tokio::test]
async fn migrates_to_vanilla_downgrade_into_new_profile() {
    let server = MockServer::start().await;
    let sodium = version(&server, "S1", "sodium", "1.0", "sodium-1.jar");
    let rp1 = version(&server, "R1", "faithful", "1.0", "rp-1.zip");
    let rp2 = version(&server, "R2", "faithful", "2.0", "rp-2.zip");
    server.on(
        "POST",
        "/v2/version_files",
        200,
        json!({ sha1("inhalt-sodium-1.jar"): sodium, sha1("inhalt-rp-1.zip"): rp1 }).to_string(),
    );
    server.on(
        "POST",
        "/v2/version_files/update",
        200,
        json!({ sha1("inhalt-rp-1.zip"): rp2 }).to_string(),
    );
    server.on("GET", "/v2/version/R2", 200, rp2.to_string());
    server.on("GET", "/v2/projects", 200, "[]");
    server.on("GET", "/files/rp-2.zip", 200, "inhalt-rp-2.zip");

    let dir = tempfile::tempdir().unwrap();
    let launcher = Launcher::with_services(
        Paths::new(dir.path().join("data")),
        Downloader::new().unwrap(),
        Modrinth::with_base(format!("{}/v2", server.base)).unwrap(),
        None,
    )
    .unwrap();
    let mut profile = Profile::new("Alt", "26.3");
    profile.loader = Some(LoaderSpec {
        kind: LoaderKind::Fabric,
        version: "0.19.5".into(),
    });
    let profile = launcher.profiles().create(profile).unwrap();
    let game = launcher.game_dir(&profile);
    for (rel, body) in [
        ("mods/sodium-1.jar", "inhalt-sodium-1.jar"),
        ("mods/eigene.jar", "eigene"),
        ("resourcepacks/rp-1.zip", "inhalt-rp-1.zip"),
        ("resourcepacks/lokal.zip", "lokal"),
        ("saves/Welt/level.dat", "welt"),
        ("logs/latest.log", "log"),
        ("options.txt", "fov:90"),
    ] {
        let p = game.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    let target = MigrationTarget {
        minecraft: "1.21.1".into(),
        loader: None,
    };
    let plan = launcher.plan_migration(&profile, &target).await.unwrap();
    assert!(plan.downgrade);
    assert_eq!(plan.worlds, 1);
    assert_eq!(plan.loader, None);
    let status: HashMap<&str, ItemStatus> = plan
        .items
        .iter()
        .map(|i| (i.key.as_str(), i.status))
        .collect();
    assert_eq!(
        status,
        HashMap::from([
            ("mods/sodium-1.jar", ItemStatus::Missing),
            ("mods/eigene.jar", ItemStatus::Missing),
            ("resourcepacks/rp-1.zip", ItemStatus::Update),
            ("resourcepacks/lokal.zip", ItemStatus::Kept),
        ])
    );
    let rp = plan
        .items
        .iter()
        .find(|i| i.key == "resourcepacks/rp-1.zip")
        .unwrap();
    assert_eq!(rp.to_version.as_deref(), Some("2.0"));

    let result = launcher
        .migrate(
            &profile,
            &MigrateOptions {
                target,
                name: None,
                include_worlds: false,
                missing: HashMap::from([("mods/sodium-1.jar".into(), MissingChoice::Drop)]),
            },
        )
        .await
        .unwrap();
    assert!(
        result.report.warnings.is_empty(),
        "{:?}",
        result.report.warnings
    );
    let new = result.profile;
    assert_eq!(new.name, "Alt (1.21.1)");
    assert_eq!(
        (new.minecraft.as_str(), new.loader.as_ref()),
        ("1.21.1", None)
    );
    let new_game = launcher.game_dir(&new);
    let exists = |rel: &str| new_game.join(rel).exists();
    assert!(!exists("mods/sodium-1.jar"), "weggelassen");
    assert!(
        exists("mods/eigene.jar.disabled"),
        "deaktiviert mitgenommen"
    );
    assert!(exists("resourcepacks/rp-2.zip") && !exists("resourcepacks/rp-1.zip"));
    assert!(exists("resourcepacks/lokal.zip"));
    assert!(exists("options.txt"));
    assert!(!exists("saves") && !exists("logs"));
    assert!(launcher.packs().pack_info(&new.id).unwrap().is_none());

    // Das Original bleibt unverändert.
    assert!(game.join("mods/sodium-1.jar").is_file());
    assert!(game.join("resourcepacks/rp-1.zip").is_file());
    assert!(game.join("saves/Welt/level.dat").is_file());
    let original = launcher.profiles().get(&profile.id).unwrap();
    assert_eq!(original.minecraft, "26.3");

    // Ein zweiter Umzug bekommt einen freien Namen.
    let again = launcher
        .migrate(
            &original,
            &MigrateOptions {
                target: MigrationTarget {
                    minecraft: "1.21.1".into(),
                    loader: None,
                },
                name: None,
                include_worlds: true,
                missing: HashMap::new(),
            },
        )
        .await
        .unwrap();
    assert_eq!(again.profile.name, "Alt (1.21.1) (2)");
    assert!(
        launcher
            .game_dir(&again.profile)
            .join("saves/Welt/level.dat")
            .is_file()
    );
}

#[tokio::test]
async fn rejects_same_target() {
    let dir = tempfile::tempdir().unwrap();
    let launcher = Launcher::with_services(
        Paths::new(dir.path().join("data")),
        Downloader::new().unwrap(),
        Modrinth::with_base("http://127.0.0.1:9/v2").unwrap(),
        None,
    )
    .unwrap();
    let profile = launcher
        .profiles()
        .create(Profile::new("Vanilla", "1.21.1"))
        .unwrap();
    let err = launcher
        .plan_migration(
            &profile,
            &MigrationTarget {
                minecraft: "1.21.1".into(),
                loader: None,
            },
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("schon"), "{err}");
}
