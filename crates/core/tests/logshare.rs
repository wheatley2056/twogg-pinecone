mod common;

use common::MockServer;
use pinecone_core::logshare;

#[tokio::test]
async fn uploads_cleaned_log_and_returns_link() {
    let server = MockServer::start().await;
    server.on(
        "POST",
        "/1/log",
        200,
        r#"{"success":true,"id":"abc123","url":"https://mclo.gs/abc123","raw":"https://api.mclo.gs/1/raw/abc123"}"#,
    );
    let link = logshare::upload(&server.base, "Hallo von /home/tobias/x mit 10.0.0.7")
        .await
        .unwrap();
    assert_eq!(link, "https://mclo.gs/abc123");
    let req = &server.requests("/1/log")[0];
    assert!(req.body.starts_with("content="), "{}", req.body);
    assert!(!req.body.contains("tobias"), "{}", req.body);
    assert!(!req.body.contains("10.0.0.7"), "{}", req.body);
}

#[tokio::test]
async fn reports_rejection_and_refuses_empty_logs() {
    let server = MockServer::start().await;
    server.on(
        "POST",
        "/1/log",
        400,
        r#"{"success":false,"error":"Content too large"}"#,
    );
    let err = logshare::upload(&server.base, "x").await.unwrap_err();
    assert!(err.to_string().contains("Content too large"), "{err}");
    assert!(logshare::upload(&server.base, "  \n ").await.is_err());
}
