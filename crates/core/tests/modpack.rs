//! Modpacks: Import, Update mit Drei-Wege-Vergleich und Snapshot, Rollback, Export.

mod common;

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use common::MockServer;
use pinecone_core::content::ContentManager;
use pinecone_core::download::Downloader;
use pinecone_core::loader::LoaderKind;
use pinecone_core::modpack::{
    ChangeAction, ConflictReason, ExportOptions, PackManager, PackSource, Resolution, read_mrpack,
};
use pinecone_core::modrinth::Modrinth;
use pinecone_core::paths::Paths;
use pinecone_core::profiles::ProfileStore;
use serde_json::json;

fn sha1(data: &str) -> String {
    use sha1::{Digest, Sha1};
    format!("{:x}", Sha1::digest(data.as_bytes()))
}

/// Baut ein `.mrpack`: `downloads` werden vom Mock geladen, `overrides` liegen im Archiv.
fn mrpack(
    path: &Path,
    server: &MockServer,
    version: &str,
    downloads: &[&str],
    overrides: &[(&str, &str)],
    deps: serde_json::Value,
) {
    let files: Vec<serde_json::Value> = downloads
        .iter()
        .map(|name| {
            let body = format!("inhalt-{name}");
            json!({ "path": format!("mods/{name}"), "hashes": { "sha1": sha1(&body), "sha512": "00" },
                    "env": { "client": "required", "server": "required" },
                    "downloads": [format!("{}/files/{name}", server.base)], "fileSize": body.len() })
        })
        .collect();
    let index = json!({ "formatVersion": 1, "game": "minecraft", "versionId": version, "name": "Test-Pack",
                        "files": files, "dependencies": deps });
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    zip.start_file("modrinth.index.json", opts).unwrap();
    zip.write_all(index.to_string().as_bytes()).unwrap();
    for (rel, content) in overrides {
        zip.start_file(format!("overrides/{rel}"), opts).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

async fn setup() -> (MockServer, tempfile::TempDir, PackManager) {
    let server = MockServer::start().await;
    for name in ["a1.jar", "a2.jar", "b.jar", "alt.jar"] {
        server.on(
            "GET",
            &format!("/files/{name}"),
            200,
            format!("inhalt-{name}"),
        );
    }
    // Hash-Abgleich beim Erkennen: nichts bekannt (alles „lokal“).
    server.on("POST", "/v2/version_files", 200, "{}");
    server.on("GET", "/v2/projects", 200, "[]");
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::new(dir.path().join("data"));
    let modrinth = Modrinth::with_base(format!("{}/v2", server.base)).unwrap();
    let content = ContentManager::new(paths.clone(), Downloader::new().unwrap(), modrinth.clone());
    let packs = PackManager::new(
        paths.clone(),
        Downloader::new().unwrap(),
        content,
        modrinth,
        ProfileStore::new(paths),
    )
    .allow_host("127.0.0.1");
    (server, dir, packs)
}

fn read(dir: &Path, rel: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(rel)).ok()
}

#[tokio::test]
async fn import_update_with_conflicts_and_rollback() {
    let (server, dir, packs) = setup().await;
    let v1 = dir.path().join("v1.mrpack");
    let deps = json!({ "minecraft": "1.21.1", "fabric-loader": "0.19.3" });
    mrpack(
        &v1,
        &server,
        "1.0",
        &["a1.jar", "alt.jar"],
        &[("config/mod.json", "pack-v1"), ("options.txt", "o1")],
        deps.clone(),
    );

    let profile = packs
        .import(&v1, PackSource::File, None)
        .await
        .unwrap()
        .profile;
    assert_eq!(profile.name, "Test-Pack");
    assert_eq!(profile.loader.as_ref().unwrap().version, "0.19.3");
    let game = dir
        .path()
        .join("data/instances")
        .join(&profile.id)
        .join(".minecraft");
    assert_eq!(read(&game, "mods/a1.jar").as_deref(), Some("inhalt-a1.jar"));
    assert_eq!(read(&game, "config/mod.json").as_deref(), Some("pack-v1"));
    let info = packs.pack_info(&profile.id).unwrap().unwrap();
    assert_eq!((info.version.as_str(), info.files.len()), ("1.0", 4));

    // Zweiter Import gleichen Namens bekommt eine Nummer.
    assert_eq!(
        packs
            .import(&v1, PackSource::File, None)
            .await
            .unwrap()
            .profile
            .name,
        "Test-Pack (2)"
    );

    // Der Nutzer passt eine Config an, deaktiviert eine Pack-Mod und legt eine eigene Mod ab.
    std::fs::write(game.join("config/mod.json"), "meins").unwrap();
    std::fs::rename(game.join("mods/a1.jar"), game.join("mods/a1.jar.disabled")).unwrap();
    std::fs::write(game.join("mods/eigene.jar"), "eigene").unwrap();

    // v2: a1 → a2 (anderer Name: a1 entfällt, a2 neu), alt.jar entfernt, b.jar neu,
    // Config im Pack geändert (Konflikt), options.txt unverändert.
    let v2 = dir.path().join("v2.mrpack");
    mrpack(
        &v2,
        &server,
        "2.0",
        &["a2.jar", "b.jar"],
        &[("config/mod.json", "pack-v2"), ("options.txt", "o1")],
        deps,
    );
    let plan = packs
        .plan_update(&profile, &read_mrpack(&v2).unwrap())
        .await
        .unwrap();
    let action = |p: &str| plan.changes.iter().find(|c| c.path == p).map(|c| c.action);
    assert_eq!(action("mods/a2.jar"), Some(ChangeAction::Add));
    assert_eq!(action("mods/b.jar"), Some(ChangeAction::Add));
    assert_eq!(action("mods/alt.jar"), Some(ChangeAction::Remove));
    assert_eq!(
        action("mods/a1.jar"),
        Some(ChangeAction::Remove),
        "deaktivierte, unveränderte Pack-Mod fällt weg"
    );
    assert_eq!(action("options.txt"), None);
    assert_eq!(plan.conflicts.len(), 1);
    assert_eq!(plan.conflicts[0].path, "config/mod.json");
    assert_eq!(plan.conflicts[0].reason, ConflictReason::BothChanged);
    assert_eq!(plan.conflicts[0].default, Resolution::KeepMine);

    let snapshot = packs
        .apply_update(&profile, &v2, PackSource::File, &HashMap::new())
        .await
        .unwrap();
    assert_eq!(read(&game, "mods/a2.jar").as_deref(), Some("inhalt-a2.jar"));
    assert_eq!(read(&game, "mods/b.jar").as_deref(), Some("inhalt-b.jar"));
    assert!(read(&game, "mods/alt.jar").is_none() && read(&game, "mods/a1.jar.disabled").is_none());
    assert_eq!(
        read(&game, "config/mod.json").as_deref(),
        Some("meins"),
        "Nutzer-Config bleibt"
    );
    assert_eq!(
        read(&game, "mods/eigene.jar").as_deref(),
        Some("eigene"),
        "eigene Mod bleibt"
    );
    assert_eq!(
        packs.pack_info(&profile.id).unwrap().unwrap().version,
        "2.0"
    );
    assert_eq!(packs.snapshots(&profile.id).len(), 1);

    // Ein weiteres Update ohne Pack-Änderung an der Config bringt keinen Konflikt mehr.
    let replan = packs
        .plan_update(&profile, &read_mrpack(&v2).unwrap())
        .await
        .unwrap();
    assert!(
        replan.conflicts.is_empty() && replan.changes.is_empty(),
        "{replan:?}"
    );
    assert_eq!(replan.kept, ["config/mod.json"]);

    // Zurückrollen stellt v1 inkl. deaktivierter Mod wieder her.
    packs.rollback(&profile, &snapshot.id).unwrap();
    assert_eq!(
        read(&game, "mods/a1.jar.disabled").as_deref(),
        Some("inhalt-a1.jar")
    );
    assert_eq!(
        read(&game, "mods/alt.jar").as_deref(),
        Some("inhalt-alt.jar")
    );
    assert!(read(&game, "mods/a2.jar").is_none() && read(&game, "mods/b.jar").is_none());
    assert_eq!(read(&game, "config/mod.json").as_deref(), Some("meins"));
    assert_eq!(
        packs.pack_info(&profile.id).unwrap().unwrap().version,
        "1.0"
    );
    assert!(packs.snapshots(&profile.id).is_empty());
}

#[tokio::test]
async fn take_pack_resolution_overwrites_config() {
    let (server, dir, packs) = setup().await;
    let deps = json!({ "minecraft": "1.21.1", "fabric-loader": "0.19.3" });
    let v1 = dir.path().join("v1.mrpack");
    mrpack(
        &v1,
        &server,
        "1",
        &[],
        &[("config/mod.json", "pack-v1")],
        deps.clone(),
    );
    let profile = packs
        .import(&v1, PackSource::File, None)
        .await
        .unwrap()
        .profile;
    let game = dir
        .path()
        .join("data/instances")
        .join(&profile.id)
        .join(".minecraft");
    std::fs::write(game.join("config/mod.json"), "meins").unwrap();
    let v2 = dir.path().join("v2.mrpack");
    mrpack(
        &v2,
        &server,
        "2",
        &[],
        &[("config/mod.json", "pack-v2")],
        deps,
    );
    let choice = HashMap::from([("config/mod.json".to_string(), Resolution::TakePack)]);
    packs
        .apply_update(&profile, &v2, PackSource::File, &choice)
        .await
        .unwrap();
    assert_eq!(read(&game, "config/mod.json").as_deref(), Some("pack-v2"));
}

#[tokio::test]
async fn rejects_unsafe_packs() {
    let (server, dir, _packs) = setup().await;
    let ok_deps = json!({ "minecraft": "1.21.1", "fabric-loader": "0.19.3" });

    let traversal = dir.path().join("t.mrpack");
    mrpack(
        &traversal,
        &server,
        "1",
        &[],
        &[("../../ausbruch.txt", "x")],
        ok_deps.clone(),
    );
    assert!(read_mrpack(&traversal).is_err());

    let forge = dir.path().join("f.mrpack");
    mrpack(
        &forge,
        &server,
        "1",
        &[],
        &[],
        json!({ "minecraft": "1.20.1", "forge": "47.2.0" }),
    );
    let forge_pack = read_mrpack(&forge).unwrap();
    assert_eq!(forge_pack.loader.unwrap().kind, LoaderKind::Forge);

    // Download-Host nicht erlaubt: Import scheitert, es bleibt kein halbes Profil zurück.
    let strict = PackManager::new(
        Paths::new(dir.path().join("data")),
        Downloader::new().unwrap(),
        ContentManager::new(
            Paths::new(dir.path().join("data")),
            Downloader::new().unwrap(),
            Modrinth::with_base(format!("{}/v2", server.base)).unwrap(),
        ),
        Modrinth::with_base(format!("{}/v2", server.base)).unwrap(),
        ProfileStore::new(Paths::new(dir.path().join("data"))),
    );
    let pack = dir.path().join("h.mrpack");
    mrpack(&pack, &server, "1", &["a1.jar"], &[], ok_deps);
    let err = strict
        .import(&pack, PackSource::File, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("nicht erlaubt"), "{err}");
    assert!(
        ProfileStore::new(Paths::new(dir.path().join("data")))
            .list()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn exports_links_and_own_files() {
    let (server, dir, packs) = setup().await;
    let deps = json!({ "minecraft": "1.21.1", "fabric-loader": "0.19.3" });
    let v1 = dir.path().join("v1.mrpack");
    mrpack(
        &v1,
        &server,
        "1",
        &["a1.jar"],
        &[("config/mod.json", "cfg")],
        deps,
    );
    let profile = packs
        .import(&v1, PackSource::File, None)
        .await
        .unwrap()
        .profile;
    let game = dir
        .path()
        .join("data/instances")
        .join(&profile.id)
        .join(".minecraft");
    std::fs::write(game.join("mods/eigene.jar"), "eigene").unwrap();
    std::fs::rename(game.join("mods/a1.jar"), game.join("mods/a1.jar.disabled")).unwrap();

    // a1.jar ist deaktiviert und wird daher nicht exportiert; Unbekanntes kommt ins Archiv.
    let out = dir.path().join("export.mrpack");
    let report = packs
        .export(&profile, &out, &ExportOptions::new("1.0.0"))
        .await
        .unwrap();
    assert_eq!(
        (report.linked, report.included),
        (0, 2),
        "eigene.jar + config/mod.json"
    );
    let pack = read_mrpack(&out).unwrap();
    assert_eq!(pack.version, "1.0.0");
    assert_eq!(pack.loader.unwrap().version, "0.19.3");
    let keys: Vec<&String> = pack.files.keys().collect();
    assert_eq!(keys, ["config/mod.json", "mods/eigene.jar"]);
}

#[tokio::test]
async fn export_options_choose_folders_and_never_share_servers() {
    let (server, dir, packs) = setup().await;
    let deps = json!({ "minecraft": "1.21.1", "fabric-loader": "0.19.3" });
    let v1 = dir.path().join("v1.mrpack");
    mrpack(&v1, &server, "1", &[], &[("config/mod.json", "cfg")], deps);
    let profile = packs
        .import(&v1, PackSource::File, None)
        .await
        .unwrap()
        .profile;
    let game = dir
        .path()
        .join("data/instances")
        .join(&profile.id)
        .join(".minecraft");
    for (rel, body) in [
        ("mods/eigene.jar", "jar"),
        ("resourcepacks/rp.zip", "rp"),
        ("shaderpacks/sh.zip", "sh"),
        ("options.txt", "fov:90"),
        ("servers.dat", "geheim"),
        ("saves/Welt/level.dat", "welt"),
    ] {
        let path = game.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
    let export = |opts: ExportOptions| {
        let out = dir.path().join(format!("{}.mrpack", opts.version));
        let packs = packs.clone();
        let profile = profile.clone();
        async move {
            packs.export(&profile, &out, &opts).await.unwrap();
            let pack = read_mrpack(&out).unwrap();
            (pack.summary, pack.files.into_keys().collect::<Vec<_>>())
        }
    };

    let (summary, keys) = export(ExportOptions {
        summary: Some("  Für Freunde  ".into()),
        ..ExportOptions::new("standard")
    })
    .await;
    assert_eq!(summary.as_deref(), Some("Für Freunde"));
    assert_eq!(
        keys,
        [
            "config/mod.json",
            "mods/eigene.jar",
            "resourcepacks/rp.zip",
            "shaderpacks/sh.zip"
        ]
    );

    let (_, keys) = export(ExportOptions {
        resourcepacks: false,
        shaders: false,
        config: false,
        options: true,
        ..ExportOptions::new("minimal")
    })
    .await;
    assert_eq!(keys, ["mods/eigene.jar", "options.txt"]);

    let err = packs
        .export(
            &profile,
            &dir.path().join("x.mrpack"),
            &ExportOptions::new(" "),
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Versionsnummer"), "{err}");
}

#[tokio::test]
async fn preview_counts_content_and_flags_foreign_hosts() {
    let (server, dir, packs) = setup().await;
    let deps = json!({ "minecraft": "1.21.1", "quilt-loader": "0.26.4" });
    let v1 = dir.path().join("v1.mrpack");
    mrpack(
        &v1,
        &server,
        "2.1",
        &["a1.jar", "b.jar"],
        &[
            ("config/mod.json", "cfg"),
            ("resourcepacks/rp.zip", "rp"),
            ("options.txt", "o"),
        ],
        deps.clone(),
    );
    let preview = packs.preview(&v1).await.unwrap();
    assert_eq!(
        (preview.name.as_str(), preview.version.as_str()),
        ("Test-Pack", "2.1")
    );
    assert_eq!(preview.minecraft, "1.21.1");
    assert_eq!(preview.loader.unwrap().version, "0.26.4");
    assert_eq!(
        (
            preview.mods,
            preview.resourcepacks,
            preview.shaders,
            preview.other_files
        ),
        (2, 1, 0, 2)
    );
    assert_eq!(
        preview.download_size,
        ("inhalt-a1.jar".len() + "inhalt-b.jar".len()) as u64
    );
    assert!(preview.archive_size > 0);
    assert!(preview.blocked_hosts.is_empty());
    // Vorschau legt kein Profil an.
    assert!(!dir.path().join("data/instances").exists());

    // Ein Pack, das von einem fremden Host laden will, wird schon in der Vorschau gemeldet.
    let strict = PackManager::new(
        Paths::new(dir.path().join("strict")),
        Downloader::new().unwrap(),
        ContentManager::new(
            Paths::new(dir.path().join("strict")),
            Downloader::new().unwrap(),
            Modrinth::with_base(format!("{}/v2", server.base)).unwrap(),
        ),
        Modrinth::with_base(format!("{}/v2", server.base)).unwrap(),
        ProfileStore::new(Paths::new(dir.path().join("strict"))),
    );
    let preview = strict.preview(&v1).await.unwrap();
    assert_eq!(preview.blocked_hosts, ["127.0.0.1"]);
}

/// CurseForge-Pack (ZIP mit manifest.json) gegen eine nachgebaute CurseForge-API.
#[tokio::test]
async fn imports_curseforge_pack_with_blocked_file() {
    use pinecone_core::curseforge::CurseForge;
    let (server, dir, packs) = setup().await;
    let file = |id: u64, mod_id: u64, name: &str, blocked: bool| {
        let body = format!("inhalt-{name}");
        json!({ "id": id, "modId": mod_id, "displayName": name, "fileName": name, "releaseType": 1, "fileDate": "2026-01-01T00:00:00Z",
                "fileLength": body.len(), "downloadUrl": if blocked { serde_json::Value::Null } else { json!(format!("{}/files/{name}", server.base)) },
                "hashes": [{ "value": sha1(&body), "algo": 1 }], "gameVersions": ["1.21.1"], "dependencies": [] })
    };
    server
        .on("POST", "/api/v1/curseforge/v1/mods/files", 200, json!({ "data": [file(11, 1, "a1.jar", false), file(22, 2, "b.jar", true), file(33, 3, "alt.jar", false)] }).to_string())
        .on("POST", "/api/v1/curseforge/v1/mods", 200, json!({ "data": [
            { "id": 1, "name": "Mod A", "slug": "mod-a", "classId": 6 },
            { "id": 2, "name": "Mod B", "slug": "mod-b", "classId": 6, "links": { "websiteUrl": "https://www.curseforge.com/minecraft/mc-mods/mod-b" } },
            { "id": 3, "name": "Paket", "slug": "paket", "classId": 12 } ] }).to_string());

    let zip_path = dir.path().join("cf.zip");
    let manifest = json!({ "minecraft": { "version": "1.21.1", "modLoaders": [{ "id": "fabric-0.16.5", "primary": true }] },
        "manifestType": "minecraftModpack", "manifestVersion": 1, "name": "CF-Pack", "version": "3.0",
        "files": [{ "projectID": 1, "fileID": 11, "required": true }, { "projectID": 2, "fileID": 22, "required": true },
                  { "projectID": 3, "fileID": 33, "required": true }, { "projectID": 9, "fileID": 99, "required": false }],
        "overrides": "overrides" });
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    zip.start_file("manifest.json", opts).unwrap();
    zip.write_all(manifest.to_string().as_bytes()).unwrap();
    zip.start_file("overrides/config/cf.json", opts).unwrap();
    zip.write_all(b"cf").unwrap();
    zip.finish().unwrap();

    // Ohne CurseForge: klare Meldung.
    let err = packs
        .import(&zip_path, PackSource::File, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("nicht verfügbar"), "{err}");

    // Wie in Release-Builds: über den Proxy des Pinecone-Servers, ohne eigenen Schlüssel.
    let packs = packs.with_curseforge(Some(CurseForge::via_server(&server.base).unwrap()));
    let result = packs
        .import(&zip_path, PackSource::File, None)
        .await
        .unwrap();
    assert_eq!(result.profile.loader.as_ref().unwrap().version, "0.16.5");
    let game = dir
        .path()
        .join("data/instances")
        .join(&result.profile.id)
        .join(".minecraft");
    assert_eq!(read(&game, "mods/a1.jar").as_deref(), Some("inhalt-a1.jar"));
    assert_eq!(
        read(&game, "resourcepacks/alt.jar").as_deref(),
        Some("inhalt-alt.jar"),
        "Klasse 12 → resourcepacks/"
    );
    assert_eq!(read(&game, "config/cf.json").as_deref(), Some("cf"));
    assert_eq!(result.missing.len(), 1);
    assert_eq!(result.missing[0].path, "mods/b.jar");
    assert_eq!(
        result.missing[0].url,
        "https://www.curseforge.com/minecraft/mc-mods/mod-b/files/22"
    );
    let req = &server.requests("/api/v1/curseforge/v1/mods/files")[0];
    // Kein Schlüssel im Launcher; der Server erkennt ihn an der Launcher-Kennung.
    assert_eq!(req.headers.get("x-api-key"), None);
    assert!(
        req.headers
            .get("x-pinecone-client")
            .is_some_and(|c| c.starts_with("pinecone/")),
        "{:?}",
        req.headers
    );
    assert!(
        !req.body.contains("99"),
        "optionale Dateien werden nicht geladen: {}",
        req.body
    );
}

#[tokio::test]
async fn import_takes_pack_logo_as_instance_icon() {
    let (server, dir, packs) = setup().await;
    let packs = packs.with_twogg(Some(
        pinecone_core::twogg::TwoggClient::with_base(&server.base).unwrap(),
    ));
    let mut logo = Vec::new();
    image::RgbaImage::from_pixel(256, 256, image::Rgba([233, 162, 59, 255]))
        .write_to(
            &mut std::io::Cursor::new(&mut logo),
            image::ImageFormat::Png,
        )
        .unwrap();
    server.on("GET", "/media/logo.png", 200, logo);
    server.on(
        "GET",
        "/api/v1/packs/test-pack",
        200,
        json!({ "id": "test-pack", "title": "Test", "summary": "", "iconUrl": format!("{}/media/logo.png", server.base),
                "featured": false, "updatedAt": 0, "downloads": 0, "latest": null })
        .to_string(),
    );
    let v1 = dir.path().join("v1.mrpack");
    mrpack(
        &v1,
        &server,
        "1.0",
        &["a1.jar"],
        &[],
        json!({ "minecraft": "1.21.1" }),
    );
    let source = PackSource::Twogg {
        pack_id: "test-pack".into(),
        version_id: "v1".into(),
    };
    let result = packs.import(&v1, source, None).await.unwrap();
    let icon = result.profile.icon.expect("Logo als Symbol");
    assert!(icon.starts_with("data:image/png;base64,"));

    // Ohne Logo an der Quelle bleibt das Symbol leer, der Import klappt trotzdem.
    let plain = packs.import(&v1, PackSource::File, None).await.unwrap();
    assert_eq!(plain.profile.icon, None);
    assert_eq!(packs.fill_missing_icons().await.unwrap(), 0);
}

#[tokio::test]
async fn withdrawn_twogg_version_is_reported_with_alternative() {
    let (server, dir, packs) = setup().await;
    let packs = packs.with_twogg(Some(
        pinecone_core::twogg::TwoggClient::with_base(&server.base).unwrap(),
    ));
    let version = |id: &str, v: &str| {
        json!({ "id": id, "version": v, "minecraft": "1.21.1", "loader": null, "loaderVersion": null,
                "channel": "release", "changelogHtml": "", "size": 1, "sha1": "x", "publishedAt": 0 })
    };
    server.on(
        "GET",
        "/api/v1/packs/avalon",
        200,
        json!({ "id": "avalon", "title": "Avalon", "summary": "", "iconUrl": null, "featured": false,
                "updatedAt": 0, "downloads": 0, "latest": version("v1", "1.0"),
                "versions": [version("v1", "1.0")],
                "withdrawn": [{ "id": "v2", "version": "1.1", "reason": "Absturz im Nether" }] })
        .to_string(),
    );
    let file = dir.path().join("v2.mrpack");
    mrpack(
        &file,
        &server,
        "1.1",
        &["a1.jar"],
        &[],
        json!({ "minecraft": "1.21.1" }),
    );
    let source = PackSource::Twogg {
        pack_id: "avalon".into(),
        version_id: "v2".into(),
    };
    let profile = packs.import(&file, source, None).await.unwrap().profile;
    let update = packs
        .check_update(&profile)
        .await
        .unwrap()
        .expect("Hinweis");
    assert_eq!(update.withdrawn.as_deref(), Some("Absturz im Nether"));
    // Angeboten wird die ältere, noch verfügbare Version.
    assert_eq!(
        (update.version_id.as_str(), update.latest.as_str()),
        ("v1", "1.0")
    );
}

// ---------- Signierte Event-Packs (F-10, F-11) ----------

fn sha512(data: &[u8]) -> String {
    pinecone_core::eventpack::sha512_hex(data)
}

#[tokio::test]
async fn signed_event_pack_install_lock_and_repair() {
    use pinecone_core::eventpack::{self, LockedFile, Manifest, MrpackRef};

    let (server, dir, packs) = setup().await;
    let packs = packs.with_twogg(Some(
        pinecone_core::twogg::TwoggClient::with_base(&server.base).unwrap(),
    ));
    // Testschlüssel: in Debug-Builds gilt die Variable auch zur Laufzeit.
    let (private, public) = eventpack::generate_key();
    // SAFETY: nur dieser Test liest die Variable; andere Tests prüfen keine Event-Packs.
    unsafe { std::env::set_var("PINECONE_EVENTPACK_KEYS", format!("evt-test={public}")) };
    let key = eventpack::signing_key(&private).unwrap();

    let file = dir.path().join("event.mrpack");
    mrpack(
        &file,
        &server,
        "1.4.0",
        &["a1.jar"],
        &[("mods/core.jar", "core"), ("config/c.toml", "x=1")],
        json!({ "minecraft": "1.21.1" }),
    );
    let bytes = std::fs::read(&file).unwrap();
    let manifest = |min_launcher: &str| Manifest {
        schema: eventpack::SCHEMA.into(),
        pack_id: "avalon".into(),
        version: "1.4.0".into(),
        channel: "release".into(),
        event_id: "evt-42".into(),
        title: "REVIVAL: Avalon".into(),
        minecraft: "1.21.1".into(),
        loader: None,
        min_launcher_version: min_launcher.into(),
        required: true,
        allow_extra_mods: false,
        recommended_memory_mb: None,
        mrpack: MrpackRef {
            url: "x".into(),
            size: bytes.len() as u64,
            sha512: sha512(&bytes),
        },
        locked: vec![
            LockedFile {
                path: "mods/a1.jar".into(),
                sha512: sha512(b"inhalt-a1.jar"),
            },
            LockedFile {
                path: "mods/core.jar".into(),
                sha512: sha512(b"core"),
            },
        ],
        published_at: "2026-10-14T00:00:00Z".into(),
        key_id: "evt-test".into(),
    };
    let signed = |m: &Manifest| serde_json::to_string(&eventpack::sign(m, &key).unwrap()).unwrap();
    let version = json!({ "id": "v1", "version": "1.4.0", "minecraft": "1.21.1", "loader": null,
        "loaderVersion": null, "channel": "release", "changelogHtml": "", "size": bytes.len(),
        "sha1": format!("{:x}", <sha1::Sha1 as sha1::Digest>::digest(&bytes)), "publishedAt": 0, "signed": true });
    server.on(
        "GET",
        "/api/v1/packs/avalon",
        200,
        json!({ "id": "avalon", "title": "Avalon", "summary": "", "iconUrl": null, "featured": false,
                "updatedAt": 0, "downloads": 0, "latest": version, "versions": [version], "eventId": "evt-42" })
        .to_string(),
    );
    server.on(
        "POST",
        "/api/v1/packs/avalon/versions/v1/download",
        200,
        json!({ "url": format!("{}/dl/v1", server.base), "expires": 0, "size": bytes.len(),
                "sha1": version["sha1"], "sha512": sha512(&bytes), "fileName": "avalon.mrpack" })
        .to_string(),
    );
    server.on("GET", "/dl/v1", 200, bytes.clone());

    // Veränderte Signatur: abgelehnt, nichts installiert (AK-10.1).
    let mut tampered = eventpack::sign(&manifest("0.1.0"), &key).unwrap();
    tampered.manifest["allowExtraMods"] = json!(true);
    server.on(
        "GET",
        "/api/v1/packs/avalon/versions/v1/manifest",
        200,
        serde_json::to_string(&tampered).unwrap(),
    );
    let err = packs.download_twogg("avalon", None).await.unwrap_err();
    assert!(err.to_string().contains(eventpack::NOT_SIGNED), "{err}");

    // Zu alter Launcher (AK-10.3).
    server.replace(
        "GET",
        "/api/v1/packs/avalon/versions/v1/manifest",
        200,
        signed(&manifest("99.0.0")),
    );
    let err = packs.download_twogg("avalon", None).await.unwrap_err();
    assert!(err.to_string().contains("aktualisiere Pinecone"), "{err}");

    // Echt: installieren, Event-Daten im Lock (AK-10.4).
    server.replace(
        "GET",
        "/api/v1/packs/avalon/versions/v1/manifest",
        200,
        signed(&manifest("0.1.0")),
    );
    let (archive, source) = packs.download_twogg("avalon", None).await.unwrap();
    let profile = packs.import(&archive, source, None).await.unwrap().profile;
    let paths = Paths::new(dir.path().join("data"));
    let game = paths.instance_dir(&profile.id).join(".minecraft");
    let modrinth = Modrinth::with_base(format!("{}/v2", server.base)).unwrap();
    let content = ContentManager::new(paths.clone(), Downloader::new().unwrap(), modrinth);
    let lock = content.load_lock(&profile.id).unwrap();
    let event = lock.pack.unwrap().event.expect("Event-Lock");
    assert_eq!(
        (event.event_id.as_str(), event.version.as_str()),
        ("evt-42", "1.4.0")
    );
    assert_eq!(event.locked.len(), 2);

    // Pflicht-Mods lassen sich nicht abschalten oder entfernen (AK-11.1), Configs schon frei.
    assert!(
        content
            .set_enabled(&profile.id, &game, "mods/a1.jar", false)
            .is_err()
    );
    assert!(content.remove(&profile.id, &game, "mods/core.jar").is_err());
    assert!(game.join("mods/a1.jar").is_file());

    // Nichts zu tun: leerer Bericht.
    let r = packs
        .check_event_integrity(&profile, false)
        .await
        .unwrap()
        .unwrap();
    assert!(r.repaired.is_empty() && r.disabled.is_empty(), "{r:?}");

    // Verändert, gelöscht, Zusatz-Mod: repariert bzw. abgeschaltet (AK-11.2, AK-11.3).
    std::fs::write(game.join("mods/a1.jar"), "manipuliert").unwrap();
    std::fs::remove_file(game.join("mods/core.jar")).unwrap();
    std::fs::write(game.join("mods/extra.jar"), "eigene Mod").unwrap();
    let r = packs
        .check_event_integrity(&profile, false)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.repaired, vec!["mods/a1.jar", "mods/core.jar"]);
    assert_eq!(r.disabled, vec!["mods/extra.jar"]);
    assert_eq!(read(&game, "mods/a1.jar").as_deref(), Some("inhalt-a1.jar"));
    assert_eq!(read(&game, "mods/core.jar").as_deref(), Some("core"));
    assert!(game.join("mods/extra.jar.disabled").is_file());

    // Offline: Datei aus dem Archiv geht, Download-Datei nicht.
    std::fs::remove_file(game.join("mods/core.jar")).unwrap();
    let r = packs
        .check_event_integrity(&profile, true)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.repaired, vec!["mods/core.jar"]);
    std::fs::remove_file(game.join("mods/a1.jar")).unwrap();
    let err = packs
        .check_event_integrity(&profile, true)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("offline nicht geladen"), "{err}");
    packs.check_event_integrity(&profile, false).await.unwrap();

    // Pflicht-Update (F-12): Neueste Version ist die installierte → nichts zu tun.
    server.on(
        "GET",
        "/api/v1/packs/avalon/manifest/latest",
        200,
        signed(&manifest("0.1.0")),
    );
    assert_eq!(packs.required_event_update(&profile).await.unwrap(), None);

    // 1.5.0 als Pflicht: a1 → a2, core.jar neu; Spieler hat core.jar und Config verändert.
    let file2 = dir.path().join("event2.mrpack");
    mrpack(
        &file2,
        &server,
        "1.5.0",
        &["a2.jar"],
        &[("mods/core.jar", "core2"), ("config/c.toml", "x=2")],
        json!({ "minecraft": "1.21.1" }),
    );
    let bytes2 = std::fs::read(&file2).unwrap();
    let mut m2 = manifest("0.1.0");
    m2.version = "1.5.0".into();
    m2.mrpack.size = bytes2.len() as u64;
    m2.mrpack.sha512 = sha512(&bytes2);
    m2.locked = vec![
        LockedFile {
            path: "mods/a2.jar".into(),
            sha512: sha512(b"inhalt-a2.jar"),
        },
        LockedFile {
            path: "mods/core.jar".into(),
            sha512: sha512(b"core2"),
        },
    ];
    let mut v2 = version.clone();
    v2["id"] = json!("v2");
    v2["version"] = json!("1.5.0");
    v2["size"] = json!(bytes2.len());
    v2["sha1"] = json!(format!(
        "{:x}",
        <sha1::Sha1 as sha1::Digest>::digest(&bytes2)
    ));
    server.replace(
        "GET",
        "/api/v1/packs/avalon",
        200,
        json!({ "id": "avalon", "title": "Avalon", "summary": "", "iconUrl": null, "featured": false,
                "updatedAt": 0, "downloads": 0, "latest": v2, "versions": [v2, version], "eventId": "evt-42" })
        .to_string(),
    );
    server.on(
        "GET",
        "/api/v1/packs/avalon/versions/v2/manifest",
        200,
        signed(&m2),
    );
    server.on(
        "POST",
        "/api/v1/packs/avalon/versions/v2/download",
        200,
        json!({ "url": format!("{}/dl/v2", server.base), "expires": 0, "size": bytes2.len(),
                "sha1": v2["sha1"], "sha512": sha512(&bytes2), "fileName": "avalon.mrpack" })
        .to_string(),
    );
    server.on("GET", "/dl/v2", 200, bytes2.clone());
    std::fs::write(game.join("mods/core.jar"), "eigener core").unwrap();
    std::fs::write(game.join("config/c.toml"), "x=5").unwrap();

    // Nicht als Pflicht markiert: bleibt beim Hinweis (AK-12.2).
    let mut optional = m2.clone();
    optional.required = false;
    server.replace(
        "GET",
        "/api/v1/packs/avalon/manifest/latest",
        200,
        signed(&optional),
    );
    assert_eq!(packs.required_event_update(&profile).await.unwrap(), None);

    server.replace(
        "GET",
        "/api/v1/packs/avalon/manifest/latest",
        200,
        signed(&m2),
    );
    assert_eq!(
        packs.required_event_update(&profile).await.unwrap(),
        Some(("1.4.0".into(), "1.5.0".into()))
    );
    assert!(!game.join("mods/a1.jar").exists());
    assert_eq!(read(&game, "mods/a2.jar").as_deref(), Some("inhalt-a2.jar"));
    assert_eq!(read(&game, "mods/core.jar").as_deref(), Some("core2"));
    assert_eq!(read(&game, "config/c.toml").as_deref(), Some("x=5"));
    let event = content
        .load_lock(&profile.id)
        .unwrap()
        .pack
        .unwrap()
        .event
        .unwrap();
    assert_eq!(event.version, "1.5.0");
    assert_eq!(packs.snapshots(&profile.id).len(), 1);
    assert_eq!(packs.required_event_update(&profile).await.unwrap(), None);

    // Server nicht erreichbar: Start mit dem bisherigen Stand.
    server.replace("GET", "/api/v1/packs/avalon/manifest/latest", 503, "");
    assert_eq!(packs.required_event_update(&profile).await.unwrap(), None);
}
