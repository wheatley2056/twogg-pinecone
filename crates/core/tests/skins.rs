//! Skin-Import über Spielernamen gegen eine nachgebaute Mojang-API.

mod common;

use base64::Engine;
use common::MockServer;
use pinecone_core::paths::Paths;
use pinecone_core::skins::{MojangEndpoints, SkinLibrary, SkinModel};
use serde_json::json;

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut b = vec![
        0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13,
    ];
    b.extend_from_slice(b"IHDR");
    b.extend_from_slice(&w.to_be_bytes());
    b.extend_from_slice(&h.to_be_bytes());
    b
}

fn library(server: &MockServer, dir: &std::path::Path) -> SkinLibrary {
    SkinLibrary::with_endpoints(
        &Paths::new(dir),
        MojangEndpoints {
            api: format!("{}/api", server.base),
            session: format!("{}/session", server.base),
        },
    )
    .unwrap()
    .allow_texture_host("127.0.0.1")
}

fn textures(server: &MockServer, slim: bool) -> String {
    let mut skin = json!({ "url": format!("{}/texture/abc", server.base) });
    if slim {
        skin["metadata"] = json!({ "model": "slim" });
    }
    let payload = json!({ "textures": { "SKIN": skin } }).to_string();
    let value = base64::engine::general_purpose::STANDARD.encode(payload);
    json!({ "id": "abc", "name": "Tobi", "properties": [{ "name": "textures", "value": value }] })
        .to_string()
}

#[tokio::test]
async fn imports_skin_by_player_name() {
    let server = MockServer::start().await;
    server
        .on(
            "GET",
            "/api/users/profiles/minecraft/tobi",
            200,
            r#"{"id":"abc","name":"Tobi"}"#,
        )
        .on(
            "GET",
            "/session/session/minecraft/profile/abc",
            200,
            textures(&server, true),
        )
        .on("GET", "/texture/abc", 200, png(64, 64))
        .on("GET", "/api/users/profiles/minecraft/Niemand", 404, "{}");
    let dir = tempfile::tempdir().unwrap();
    let lib = library(&server, dir.path());

    let skin = lib.import_player("tobi").await.unwrap();
    assert_eq!((skin.name.as_str(), skin.model), ("Tobi", SkinModel::Slim));
    assert_eq!(lib.list().unwrap().len(), 1);

    let err = lib.import_player("Niemand").await.unwrap_err();
    assert!(err.to_string().contains("gibt es nicht"), "{err}");
    assert!(lib.import_player("kein name!").await.is_err());
}

#[tokio::test]
async fn rejects_foreign_texture_hosts_and_hd_skins() {
    let server = MockServer::start().await;
    server.on("GET", "/texture/hd", 200, png(128, 128));
    let dir = tempfile::tempdir().unwrap();
    let lib = library(&server, dir.path());
    let err = lib
        .import_texture("https://example.com/skin.png", "x", SkinModel::Classic)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("nicht erlaubt"), "{err}");
    let err = lib
        .import_texture(
            &format!("{}/texture/hd", server.base),
            "x",
            SkinModel::Classic,
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("64×64"), "{err}");
}
