//! Microsoft-Login und Account-Speicher gegen einen Mock von Microsoft, Xbox und Minecraft-API.

mod common;

use common::MockServer;
use pinecone_core::Error;
use pinecone_core::accounts::{AccountStore, KeyringStore, MemoryStore, SecretStore, refresh_key};
use pinecone_core::auth::AccountKind;
use pinecone_core::msa::{Endpoints, MsaClient};

const PROFILE_ID: &str = "b50ad385829d3141a2167e7d7539ba7f";

fn client(server: &MockServer) -> MsaClient {
    MsaClient::with_endpoints(
        "test-client",
        Endpoints {
            microsoft: format!("{}/ms", server.base),
            xbox_user: format!("{}/xbl", server.base),
            xsts: format!("{}/xsts", server.base),
            minecraft: format!("{}/mc", server.base),
        },
    )
    .unwrap()
}

/// Mock mit erfolgreichem Xbox-, XSTS-, Minecraft- und Profil-Schritt.
async fn happy_server() -> MockServer {
    let server = MockServer::start().await;
    server
        .on("POST", "/xbl", 200, r#"{"Token":"xbl-token","DisplayClaims":{"xui":[{"uhs":"hash123"}]}}"#)
        .on("POST", "/xsts", 200, r#"{"Token":"xsts-token","DisplayClaims":{"xui":[{"uhs":"hash123"}]}}"#)
        .on("POST", "/mc/authentication/login_with_xbox", 200, r#"{"access_token":"mc-token","expires_in":86400}"#)
        .on(
            "GET",
            "/mc/minecraft/profile",
            200,
            format!(
                r#"{{"id":"{PROFILE_ID}","name":"Tobi","skins":[{{"id":"s1","state":"ACTIVE","url":"https://textures.minecraft.net/texture/abc","variant":"SLIM"}}],"capes":[]}}"#
            ),
        );
    server
}

#[tokio::test]
async fn device_code_flow_to_profile() {
    let server = happy_server().await;
    server
        .on(
            "POST",
            "/ms/devicecode",
            200,
            r#"{"device_code":"dev","user_code":"ABCD-EFGH","verification_uri":"https://microsoft.com/link","expires_in":60,"interval":0,"message":"Gehe zu …"}"#,
        )
        .on("POST", "/ms/token", 400, r#"{"error":"authorization_pending"}"#)
        .on("POST", "/ms/token", 400, r#"{"error":"authorization_pending"}"#)
        .on("POST", "/ms/token", 200, r#"{"access_token":"ms-access","refresh_token":"ms-refresh","expires_in":3600}"#);

    let msa = client(&server);
    let code = msa.request_device_code().await.unwrap();
    assert_eq!(code.user_code, "ABCD-EFGH");
    let devicecode = &server.requests("/ms/devicecode")[0];
    assert_eq!(devicecode.method, "POST");
    assert!(devicecode.body.contains("client_id=test-client"));
    assert!(
        devicecode
            .body
            .contains("scope=XboxLive.signin+offline_access")
    );

    let tokens = msa.poll_device_code(&code).await.unwrap();
    assert_eq!(
        server.requests("/ms/token").len(),
        3,
        "zweimal pending, dann Erfolg"
    );
    assert!(
        server.requests("/ms/token")[0]
            .body
            .contains("device_code=dev")
    );

    let login = msa.login_minecraft(tokens).await.unwrap();
    assert_eq!(login.profile.name, "Tobi");
    assert_eq!(login.access_token, "mc-token");
    assert_eq!(login.refresh_token, "ms-refresh");
    assert_eq!(
        login.profile.active_skin().unwrap().variant.as_deref(),
        Some("SLIM")
    );

    assert!(
        server.requests("/xbl")[0]
            .body
            .contains(r#""RpsTicket":"d=ms-access""#)
    );
    assert!(
        server.requests("/xsts")[0]
            .body
            .contains(r#""UserTokens":["xbl-token"]"#)
    );
    let mc = &server.requests("/mc/authentication/login_with_xbox")[0];
    assert!(
        mc.body.contains("XBL3.0 x=hash123;xsts-token"),
        "{}",
        mc.body
    );
    let profile = &server.requests("/mc/minecraft/profile")[0];
    assert_eq!(
        profile.headers.get("authorization").map(String::as_str),
        Some("Bearer mc-token")
    );
}

#[tokio::test]
async fn declined_and_expired_codes() {
    let server = MockServer::start().await;
    server.on(
        "POST",
        "/ms/token",
        400,
        r#"{"error":"authorization_declined"}"#,
    );
    let msa = client(&server);
    let code = pinecone_core::msa::DeviceCode {
        device_code: "dev".into(),
        user_code: "X".into(),
        verification_uri: "u".into(),
        expires_in: 60,
        interval: 0,
        message: String::new(),
    };
    let err = msa.poll_device_code(&code).await.unwrap_err();
    assert!(
        matches!(&err, Error::Auth(m) if m.contains("abgelehnt")),
        "{err}"
    );

    let server = MockServer::start().await;
    server.on("POST", "/ms/token", 400, r#"{"error":"expired_token"}"#);
    let err = client(&server).poll_device_code(&code).await.unwrap_err();
    assert!(
        matches!(&err, Error::Auth(m) if m.contains("abgelaufen")),
        "{err}"
    );
}

#[tokio::test]
async fn xbox_errors_are_explained() {
    let server = MockServer::start().await;
    server
        .on(
            "POST",
            "/xbl",
            200,
            r#"{"Token":"xbl-token","DisplayClaims":{"xui":[{"uhs":"h"}]}}"#,
        )
        .on(
            "POST",
            "/xsts",
            401,
            r#"{"Identity":"0","XErr":2148916233,"Message":"","Redirect":""}"#,
        );
    let tokens = pinecone_core::msa::MicrosoftTokens {
        access_token: "a".into(),
        refresh_token: "r".into(),
    };
    let err = client(&server).login_minecraft(tokens).await.unwrap_err();
    assert!(
        matches!(&err, Error::Auth(m) if m.contains("kein Xbox-Profil")),
        "{err}"
    );
}

#[tokio::test]
async fn missing_game_and_unapproved_app() {
    let tokens = || pinecone_core::msa::MicrosoftTokens {
        access_token: "a".into(),
        refresh_token: "r".into(),
    };

    let server = happy_server().await;
    server.replace("GET", "/mc/minecraft/profile", 404, "{}");
    let err = client(&server).login_minecraft(tokens()).await.unwrap_err();
    assert!(
        matches!(&err, Error::Auth(m) if m.contains("besitzt Minecraft")),
        "{err}"
    );

    let server = MockServer::start().await;
    server
        .on(
            "POST",
            "/xbl",
            200,
            r#"{"Token":"x","DisplayClaims":{"xui":[{"uhs":"h"}]}}"#,
        )
        .on(
            "POST",
            "/xsts",
            200,
            r#"{"Token":"y","DisplayClaims":{"xui":[{"uhs":"h"}]}}"#,
        )
        .on(
            "POST",
            "/mc/authentication/login_with_xbox",
            403,
            r#"{"errorMessage":"Invalid app registration"}"#,
        );
    let err = client(&server).login_minecraft(tokens()).await.unwrap_err();
    assert!(
        matches!(&err, Error::Auth(m) if m.contains("freigeschaltet")),
        "{err}"
    );
}

#[tokio::test]
async fn account_store_keeps_tokens_out_of_json_and_refreshes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("accounts.json");
    let store = AccountStore::new(&path, MemoryStore::default());

    let server = happy_server().await;
    server.on(
        "POST",
        "/ms/token",
        200,
        r#"{"access_token":"ms-access-2","refresh_token":"ms-refresh-2"}"#,
    );
    let msa = client(&server);

    let login = msa
        .login_minecraft(pinecone_core::msa::MicrosoftTokens {
            access_token: "a".into(),
            refresh_token: "ms-refresh-1".into(),
        })
        .await
        .unwrap();
    let account = store.add_microsoft(&login).unwrap();
    assert_eq!(account.kind, AccountKind::Microsoft);
    assert_eq!(account.skin_variant.as_deref(), Some("SLIM"));
    store.add_offline("Zweitname").unwrap();

    let json = std::fs::read_to_string(&path).unwrap();
    assert!(
        !json.contains("ms-refresh") && !json.contains("mc-token"),
        "Tokens gehören nicht ins JSON: {json}"
    );
    assert_eq!(
        store.load().unwrap().active_account().unwrap().name,
        "Zweitname"
    );

    // Offline-Sitzung braucht kein Netz.
    let offline = store.session(None, None).await.unwrap();
    assert_eq!(offline.kind, AccountKind::Offline);
    assert_eq!(offline.access_token, "0");

    // Microsoft-Sitzung: Refresh, neuer Token wird gespeichert.
    let session = store.session(Some("tobi"), Some(&msa)).await.unwrap();
    assert_eq!(session.access_token, "mc-token");
    assert_eq!(session.uuid.simple().to_string(), PROFILE_ID);
    let token_req = &server.requests("/ms/token")[0];
    assert!(
        token_req.body.contains("grant_type=refresh_token")
            && token_req.body.contains("refresh_token=ms-refresh-1")
    );
    let key = refresh_key(&session.uuid);
    assert_eq!(
        store.secrets().get(&key).unwrap().as_deref(),
        Some("ms-refresh-2"),
        "Refresh-Token rotiert"
    );

    // Ohne Client-ID kein Microsoft-Start, mit klarer Meldung.
    let err = store.session(Some("Tobi"), None).await.unwrap_err();
    assert!(
        matches!(&err, Error::Auth(m) if m.contains("PINECONE_MSA_CLIENT_ID")),
        "{err}"
    );

    // Entfernen löscht auch den Schlüsselbund-Eintrag.
    store.remove("Tobi").unwrap();
    assert_eq!(
        store.secrets().get(&key).unwrap(),
        None,
        "Token aus dem Schlüsselbund gelöscht"
    );
    let accounts = store.load().unwrap();
    assert_eq!(accounts.accounts.len(), 1);
    assert_eq!(accounts.active_account().unwrap().name, "Zweitname");
}

#[test]
fn memory_store_roundtrip() {
    let s = MemoryStore::default();
    s.set("k", "v").unwrap();
    assert_eq!(s.get("k").unwrap().as_deref(), Some("v"));
    s.delete("k").unwrap();
    assert_eq!(s.get("k").unwrap(), None);
}

/// Schreibt, liest und löscht einen Testeintrag im echten OS-Schlüsselbund.
/// Nicht im normalen Testlauf: `cargo test -- --ignored keyring`.
#[test]
#[ignore]
fn keyring_roundtrip() {
    let key = "pinecone-test-eintrag";
    KeyringStore.set(key, "geheim").unwrap();
    assert_eq!(KeyringStore.get(key).unwrap().as_deref(), Some("geheim"));
    KeyringStore.delete(key).unwrap();
    assert_eq!(KeyringStore.get(key).unwrap(), None);
}
