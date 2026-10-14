//! Manifest aus einem `.mrpack` erzeugen, signieren, prüfen (F-10).

use std::io::Write;

use pinecone_core::eventpack::{self, Keyring, ManifestOptions};

fn sha1_hex(data: &[u8]) -> String {
    use sha1::Digest;
    sha1::Sha1::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn write_pack(path: &std::path::Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    let index = serde_json::json!({
        "formatVersion": 1, "game": "minecraft", "versionId": "1.4.0", "name": "Avalon",
        "files": [
            { "path": "mods/a.jar", "hashes": { "sha1": sha1_hex(b"a"), "sha512": eventpack::sha512_hex(b"a") },
              "downloads": ["https://cdn.modrinth.com/data/x/a.jar"], "fileSize": 1 },
            { "path": "mods/server.jar", "hashes": { "sha1": sha1_hex(b"s"), "sha512": eventpack::sha512_hex(b"s") },
              "env": { "client": "unsupported", "server": "required" },
              "downloads": ["https://cdn.modrinth.com/data/x/s.jar"], "fileSize": 1 }
        ],
        "dependencies": { "minecraft": "1.21.1", "fabric-loader": "0.16.10" }
    });
    zip.start_file("modrinth.index.json", opts).unwrap();
    zip.write_all(index.to_string().as_bytes()).unwrap();
    for (name, body) in [
        ("overrides/mods/core.jar", "core"),
        ("client-overrides/mods/core.jar", "client-core"),
        ("overrides/config/a.toml", "x=1"),
        ("overrides/options.txt", "fov:90"),
    ] {
        zip.start_file(name, opts).unwrap();
        zip.write_all(body.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

fn options() -> ManifestOptions {
    ManifestOptions {
        pack_id: "avalon".into(),
        event_id: "evt-42".into(),
        title: "REVIVAL: Avalon".into(),
        channel: "release".into(),
        min_launcher_version: "0.10.0".into(),
        required: true,
        allow_extra_mods: false,
        recommended_memory_mb: Some(6144),
        server_url: "https://pinecone.test/".into(),
        key_id: "test".into(),
        published_at: "2026-10-14T00:00:00Z".into(),
    }
}

#[test]
fn manifest_from_mrpack_locks_mods_but_not_configs() {
    let dir = tempfile::tempdir().unwrap();
    let pack = dir.path().join("avalon.mrpack");
    write_pack(&pack);

    let m = eventpack::build_manifest(&pack, &options()).unwrap();
    assert_eq!(m.version, "1.4.0");
    assert_eq!(m.minecraft, "1.21.1");
    assert_eq!(
        m.mrpack.url,
        "https://pinecone.test/api/v1/packs/avalon/versions/1.4.0/download"
    );
    let locked: Vec<(&str, &str)> = m
        .locked
        .iter()
        .map(|l| (l.path.as_str(), l.sha512.as_str()))
        .collect();
    let core = eventpack::sha512_hex(b"client-core");
    let a = eventpack::sha512_hex(b"a");
    assert_eq!(
        locked,
        vec![("mods/a.jar", a.as_str()), ("mods/core.jar", core.as_str())],
        "Server-Mods, Configs und options.txt sind nicht gesperrt; client-overrides gewinnen"
    );

    let (private, public) = eventpack::generate_key();
    let key = eventpack::signing_key(&private).unwrap();
    let signed = eventpack::sign(&m, &key).unwrap();
    let keyring = Keyring::parse(&format!("test={public}")).unwrap();
    let verified = keyring.verify(&signed).unwrap();
    eventpack::check_mrpack(&verified, &pack).unwrap();

    // Anderes Archiv: SHA-512 passt nicht (AK-10.2).
    let other = dir.path().join("other.mrpack");
    write_pack(&other);
    std::fs::OpenOptions::new()
        .append(true)
        .open(&other)
        .unwrap()
        .write_all(b"x")
        .unwrap();
    assert!(eventpack::check_mrpack(&verified, &other).is_err());
}
