mod common;

use common::MockServer;
use pinecone_core::twogg::TwoggClient;
use serde_json::json;

#[tokio::test]
async fn friend_code_invites_and_sync_shares() {
    let server = MockServer::start().await;
    server
        .on("GET", "/api/v1/friends/code", 200, json!({ "code": "ABCD-EFGH-JK" }).to_string())
        .on("POST", "/api/v1/friends/requests", 200, json!({ "status": "requested", "name": "Bob" }).to_string())
        .on("POST", "/api/v1/invites", 201, json!({ "id": "x1", "expiresAt": 1 }).to_string())
        .on("POST", "/api/v1/invites/x1/answer", 200, json!({ "ok": true }).to_string())
        .on("POST", "/api/v1/sync-shares", 201, json!({ "code": "QWERTY2345", "expiresAt": 1 }).to_string())
        .on("GET", "/api/v1/sync-shares/QWERTY2345", 200, json!({ "data": { "name": "Tasten" }, "owner": "Alice" }).to_string())
        .on("GET", "/api/v1/friends", 200, json!({
            "friends": [], "incoming": [], "outgoing": [],
            "invites": [{ "id": "x1", "uuid": "11111111111111111111111111111111", "name": "Alice",
                "server": "mc.twogg.de", "pack": "pinecone://twogg/avalon", "instance": "Avalon",
                "status": "open", "expiresAt": 99 }],
            "sentInvites": []
        }).to_string());
    let c = TwoggClient::with_base(&server.base).unwrap();

    assert_eq!(c.friend_code("tok", false).await.unwrap(), "ABCD-EFGH-JK");
    let (status, name) = c.friend_request_code("tok", "abcd efgh jk").await.unwrap();
    assert_eq!((status.as_str(), name.as_str()), ("requested", "Bob"));
    assert!(
        server.requests("/api/v1/friends/requests")[0]
            .body
            .contains("\"code\"")
    );

    c.invite(
        "tok",
        "22222222-2222-2222-2222-222222222222",
        "mc.twogg.de",
        Some("pinecone://twogg/avalon"),
        None,
    )
    .await
    .unwrap();
    let body = &server.requests("/api/v1/invites")[0].body;
    assert!(
        body.contains("\"to\":\"22222222222222222222222222222222\""),
        "{body}"
    );
    c.invite_answer("tok", "x1", true).await.unwrap();
    assert!(c.invite_answer("tok", "../x", true).await.is_err());

    let list = c.friends("tok").await.unwrap();
    assert_eq!(
        list.invites[0].pack.as_deref(),
        Some("pinecone://twogg/avalon")
    );
    // Gegenüber der Oberfläche in camelCase.
    assert!(
        serde_json::to_string(&list)
            .unwrap()
            .contains("\"sentInvites\"")
    );

    assert_eq!(
        c.sync_share("tok", &json!({ "name": "Tasten" }))
            .await
            .unwrap(),
        "QWERTY2345"
    );
    let shared = c.sync_share_get("tok", "QWERTY2345").await.unwrap();
    assert_eq!(shared.owner, "Alice");
    assert_eq!(shared.data["name"], "Tasten");
    assert_eq!(
        server.requests("/api/v1/friends/code")[0]
            .headers
            .get("authorization")
            .map(String::as_str),
        Some("Bearer tok")
    );
}
