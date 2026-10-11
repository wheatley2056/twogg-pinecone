//! Inhaltsverwaltung gegen einen nachgebauten Modrinth-Server.

mod common;

use common::MockServer;
use pinecone_core::content::{ContentKind, ContentManager, Source};
use pinecone_core::download::Downloader;
use pinecone_core::loader::{LoaderKind, LoaderSpec};
use pinecone_core::modrinth::Modrinth;
use pinecone_core::paths::Paths;
use pinecone_core::profiles::Profile;
use serde_json::json;

fn sha1(data: &str) -> String {
    use sha1::{Digest, Sha1};
    format!("{:x}", Sha1::digest(data.as_bytes()))
}

/// Version-JSON wie von Modrinth; Dateiinhalt = `inhalt-<datei>` (so liefert ihn der Mock aus).
fn version(
    server: &MockServer,
    id: &str,
    project: &str,
    number: &str,
    file: &str,
    deps: serde_json::Value,
) -> serde_json::Value {
    let body = format!("inhalt-{file}");
    json!({
        "id": id, "project_id": project, "name": format!("{project} {number}"), "version_number": number,
        "version_type": "release", "loaders": ["fabric"], "game_versions": ["26.3"],
        "date_published": "2026-10-01T00:00:00Z",
        "files": [{ "hashes": { "sha1": sha1(&body) }, "url": format!("{}/files/{file}", server.base),
                    "filename": file, "primary": true, "size": body.len() }],
        "dependencies": deps
    })
}

async fn setup() -> (
    MockServer,
    tempfile::TempDir,
    ContentManager,
    Profile,
    std::path::PathBuf,
) {
    let server = MockServer::start().await;
    for file in ["sodium-1.jar", "sodium-2.jar", "fabric-api.jar"] {
        server.on(
            "GET",
            &format!("/files/{file}"),
            200,
            format!("inhalt-{file}"),
        );
    }
    let sodium1 = version(
        &server,
        "S1",
        "sodium",
        "1.0",
        "sodium-1.jar",
        json!([{ "project_id": "fabric-api", "dependency_type": "required" },
                                                                                 { "project_id": "optifine", "dependency_type": "incompatible" }]),
    );
    let sodium2 = version(
        &server,
        "S2",
        "sodium",
        "2.0",
        "sodium-2.jar",
        json!([{ "project_id": "fabric-api", "dependency_type": "required" }]),
    );
    let api = version(
        &server,
        "F1",
        "fabric-api",
        "0.162.0",
        "fabric-api.jar",
        json!([]),
    );
    server
        .on("GET", "/v2/project/sodium/version", 200, json!([sodium1]).to_string())
        .on("GET", "/v2/project/fabric-api/version", 200, json!([api]).to_string())
        .on("GET", "/v2/version/S2", 200, sodium2.to_string())
        .on("GET", "/v2/projects", 200, json!([
            { "id": "sodium", "slug": "sodium", "project_type": "mod", "title": "Sodium", "description": "", "icon_url": "https://cdn/sodium.png" },
            { "id": "fabric-api", "slug": "fabric-api", "project_type": "mod", "title": "Fabric API", "description": "" }
        ]).to_string())
        .on("POST", "/v2/version_files/update", 200, json!({ sha1("inhalt-sodium-1.jar"): sodium2 }).to_string());

    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path());
    let manager = ContentManager::new(
        paths,
        Downloader::new().unwrap(),
        Modrinth::with_base(format!("{}/v2", server.base)).unwrap(),
    );
    let mut profile = Profile::new("Test", "26.3");
    profile.id = "test".into();
    profile.loader = Some(LoaderSpec {
        kind: LoaderKind::Fabric,
        version: "0.19.5".into(),
    });
    let game_dir = dir.path().join("instances/test/.minecraft");
    (server, dir, manager, profile, game_dir)
}

#[tokio::test]
async fn install_with_dependency_update_and_toggle() {
    let (server, _dir, manager, profile, game) = setup().await;

    let report = manager
        .install(&profile, &game, ContentKind::Mod, "sodium", None)
        .await
        .unwrap();
    let names: Vec<&str> = report.installed.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["Sodium", "Fabric API"]);
    assert!(game.join("mods/sodium-1.jar").is_file() && game.join("mods/fabric-api.jar").is_file());
    let versions_req = &server.requests("/v2/project/sodium/version")[0];
    assert!(
        versions_req.path.contains("loaders=%5B%22fabric%22%5D"),
        "Filter fehlt: {}",
        versions_req.path
    );

    let list = manager.list("test", &game).unwrap();
    assert_eq!(list.len(), 2);
    let sodium = list.iter().find(|c| c.key == "mods/sodium-1.jar").unwrap();
    assert_eq!(sodium.name, "Sodium");
    assert_eq!(sodium.icon_url.as_deref(), Some("https://cdn/sodium.png"));
    assert!(!sodium.unidentified && sodium.enabled);

    // Erneut installieren: nichts Neues, Abhängigkeit ist schon da.
    let again = manager
        .install(&profile, &game, ContentKind::Mod, "sodium", None)
        .await
        .unwrap();
    assert!(again.installed.is_empty());

    // Deaktivieren, dann Update: neue Datei bleibt deaktiviert, alte verschwindet.
    manager
        .set_enabled(&game, "mods/sodium-1.jar", false)
        .unwrap();
    assert!(game.join("mods/sodium-1.jar.disabled").is_file());
    let updates = manager.check_updates(&profile, &game).await.unwrap();
    assert_eq!(updates.len(), 1);
    assert_eq!(
        (updates[0].current.as_deref(), updates[0].latest.as_str()),
        (Some("1.0"), "2.0")
    );
    manager
        .apply_update(&profile, &game, &updates[0])
        .await
        .unwrap();
    assert!(game.join("mods/sodium-2.jar.disabled").is_file());
    assert!(!game.join("mods/sodium-1.jar.disabled").exists());
    let list = manager.list("test", &game).unwrap();
    let sodium = list.iter().find(|c| c.name == "Sodium").unwrap();
    assert_eq!(
        (
            sodium.key.as_str(),
            sodium.enabled,
            sodium.version.as_deref()
        ),
        ("mods/sodium-2.jar", false, Some("2.0"))
    );

    manager.remove("test", &game, "mods/sodium-2.jar").unwrap();
    assert!(!game.join("mods/sodium-2.jar.disabled").exists());
    assert_eq!(manager.list("test", &game).unwrap().len(), 1);
}

#[tokio::test]
async fn no_alpha_updates_for_releases() {
    let (server, _dir, manager, profile, game) = setup().await;
    manager
        .install(&profile, &game, ContentKind::Mod, "sodium", None)
        .await
        .unwrap();
    // Neueste Version ist nur eine Alpha: kein Update für die installierte Release-Version.
    let mut alpha = version(
        &server,
        "S3",
        "sodium",
        "3.0-alpha",
        "sodium-2.jar",
        json!([]),
    );
    alpha["version_type"] = json!("alpha");
    server.replace(
        "POST",
        "/v2/version_files/update",
        200,
        json!({ sha1("inhalt-sodium-1.jar"): alpha }).to_string(),
    );
    assert!(
        manager
            .check_updates(&profile, &game)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn identifies_manual_files_by_hash() {
    let (server, dir, manager, profile, game) = setup().await;
    let api = version(
        &server,
        "F1",
        "fabric-api",
        "0.162.0",
        "fabric-api.jar",
        json!([]),
    );
    server.on(
        "POST",
        "/v2/version_files",
        200,
        json!({ sha1("inhalt-fabric-api.jar"): api }).to_string(),
    );

    // Von Hand abgelegt: eine Modrinth-Datei unter anderem Namen und eine eigene.
    std::fs::create_dir_all(game.join("mods")).unwrap();
    std::fs::write(game.join("mods/fapi-kopie.jar"), "inhalt-fabric-api.jar").unwrap();
    std::fs::write(game.join("mods/eigene-mod.jar"), "selbst gebaut").unwrap();
    assert!(
        manager
            .list("test", &game)
            .unwrap()
            .iter()
            .all(|c| c.unidentified)
    );

    let list = manager.identify("test", &game).await.unwrap();
    let fapi = list
        .iter()
        .find(|c| c.key == "mods/fapi-kopie.jar")
        .unwrap();
    assert_eq!(fapi.name, "Fabric API");
    assert!(
        matches!(&fapi.source, Source::Modrinth { project_id, .. } if project_id == "fabric-api")
    );
    let own = list
        .iter()
        .find(|c| c.key == "mods/eigene-mod.jar")
        .unwrap();
    assert_eq!(
        (own.name.as_str(), &own.source, own.unidentified),
        ("eigene-mod", &Source::Local, false)
    );

    // Lokale Datei hinzufügen, falsche Endung wird abgelehnt.
    let src = dir.path().join("pack.zip");
    std::fs::write(&src, "zip").unwrap();
    assert_eq!(
        manager
            .add_local("test", &game, ContentKind::Resourcepack, &src)
            .await
            .unwrap(),
        "resourcepacks/pack.zip"
    );
    assert!(
        manager
            .add_local("test", &game, ContentKind::Mod, &src)
            .await
            .is_err()
    );

    // Vanilla-Profile bekommen keine Mods.
    let mut vanilla = profile.clone();
    vanilla.loader = None;
    assert!(
        manager
            .install(&vanilla, &game, ContentKind::Mod, "sodium", None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn curseforge_install_with_dependencies_and_update() {
    use pinecone_core::curseforge::CurseForge;
    let (server, _dir, manager, profile, game) = setup().await;
    let file = |id: u64,
                mod_id: u64,
                name: &str,
                date: &str,
                deps: serde_json::Value,
                blocked: bool| {
        let body = format!("inhalt-{name}");
        json!({ "id": id, "modId": mod_id, "displayName": name, "fileName": name, "releaseType": 1, "fileDate": date,
                "fileLength": body.len(),
                "downloadUrl": if blocked { serde_json::Value::Null } else { json!(format!("{}/files/{name}", server.base)) },
                "hashes": [{ "value": sha1(&body), "algo": 1 }], "gameVersions": ["26.3"], "dependencies": deps })
    };
    for name in ["jei-1.jar", "jei-2.jar", "lib.jar"] {
        server.on(
            "GET",
            &format!("/files/{name}"),
            200,
            format!("inhalt-{name}"),
        );
    }
    server
        .on("GET", "/v1/mods/100/files", 200, json!({ "data": [file(1001, 100, "jei-1.jar", "2026-01-01", json!([
            { "modId": 200, "relationType": 3 }, { "modId": 300, "relationType": 3 }]), false)] }).to_string())
        .on("GET", "/v1/mods/200/files", 200, json!({ "data": [file(2001, 200, "lib.jar", "2026-01-01", json!([]), false)] }).to_string())
        .on("GET", "/v1/mods/300/files", 200, json!({ "data": [file(3001, 300, "gesperrt.jar", "2026-01-01", json!([]), true)] }).to_string())
        .on("POST", "/v1/mods", 200, json!({ "data": [
            { "id": 100, "name": "JEI", "slug": "jei", "logo": { "thumbnailUrl": "https://media.forgecdn.net/jei.png" } },
            { "id": 200, "name": "Bibliothek", "slug": "lib" },
            { "id": 300, "name": "Gesperrt", "slug": "gesperrt" } ] }).to_string());

    // Ohne API-Key: klare Meldung.
    let err = manager
        .install_curseforge(&profile, &game, ContentKind::Mod, 100, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("CurseForge"), "{err}");

    let manager = manager.with_curseforge(Some(CurseForge::with_base(&server.base, "k").unwrap()));
    let report = manager
        .install_curseforge(&profile, &game, ContentKind::Mod, 100, None)
        .await
        .unwrap();
    let names: Vec<&str> = report.installed.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["JEI", "Bibliothek"]);
    assert_eq!(report.warnings.len(), 1);
    assert!(
        report.warnings[0].contains("manuell") && report.warnings[0].contains("/files/3001"),
        "{:?}",
        report.warnings
    );
    assert!(
        server.requests("/v1/mods/100/files")[0]
            .path
            .contains("modLoaderType=4"),
        "Fabric-Filter"
    );
    let jei = manager
        .list("test", &game)
        .unwrap()
        .into_iter()
        .find(|c| c.name == "JEI")
        .unwrap();
    assert_eq!(
        jei.icon_url.as_deref(),
        Some("https://media.forgecdn.net/jei.png")
    );
    assert!(matches!(
        jei.source,
        Source::Curseforge {
            project_id: 100,
            file_id: 1001
        }
    ));

    // Neue Datei erscheint: Update wird angeboten und ersetzt die alte Datei.
    server.replace("GET", "/v1/mods/100/files", 200, json!({ "data": [
        file(1002, 100, "jei-2.jar", "2026-06-01", json!([{ "modId": 200, "relationType": 3 }]), false),
        file(1001, 100, "jei-1.jar", "2026-01-01", json!([]), false)] }).to_string());
    let updates = manager.check_updates(&profile, &game).await.unwrap();
    let up = updates
        .iter()
        .find(|u| u.platform == "curseforge")
        .expect("CurseForge-Update");
    assert_eq!(
        (up.latest.as_str(), up.version_id.as_str()),
        ("jei-2.jar", "1002")
    );
    server.on(
        "POST",
        "/v1/mods/files",
        200,
        json!({ "data": [file(1002, 100, "jei-2.jar", "2026-06-01", json!([]), false)] })
            .to_string(),
    );
    manager.apply_update(&profile, &game, up).await.unwrap();
    assert!(game.join("mods/jei-2.jar").is_file() && !game.join("mods/jei-1.jar").exists());
}
