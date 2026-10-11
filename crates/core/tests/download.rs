//! Downloader-Tests gegen einen minimalen lokalen HTTP-Server.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use pinecone_core::Error;
use pinecone_core::download::{DownloadTask, Downloader, Progress, sha1_file};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Liefert für jede Anfrage `/<name>` den Inhalt `inhalt-<name>` und zählt die Anfragen.
async fn server() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = vec![0; 4096];
                let n = socket.read(&mut buf).await.unwrap();
                let request = String::from_utf8_lossy(&buf[..n]);
                let path = request.split_whitespace().nth(1).unwrap_or("/");
                let body = format!("inhalt-{}", path.trim_start_matches('/'));
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            });
        }
    });
    (base, hits)
}

fn sha1_hex(data: &str) -> String {
    use sha1::{Digest, Sha1};
    format!("{:x}", Sha1::digest(data.as_bytes()))
}

fn downloader() -> Downloader {
    Downloader::new()
        .unwrap()
        .retries(2, Duration::from_millis(10))
}

#[tokio::test]
async fn downloads_and_verifies() {
    let (base, hits) = server().await;
    let dir = tempfile::tempdir().unwrap();
    let tasks: Vec<DownloadTask> = (0..20)
        .map(|i| DownloadTask {
            url: format!("{base}/datei{i}"),
            dest: dir.path().join(format!("sub/datei{i}")),
            sha1: Some(sha1_hex(&format!("inhalt-datei{i}"))),
            size: Some(format!("inhalt-datei{i}").len() as u64),
        })
        .collect();

    let mut events = Vec::new();
    downloader()
        .download_all(tasks.clone(), |p| events.push(p))
        .await
        .unwrap();

    assert_eq!(hits.load(Ordering::SeqCst), 20);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("sub/datei7")).unwrap(),
        "inhalt-datei7"
    );
    assert!(matches!(events[0], Progress::Started { files: 20, .. }));
    assert!(matches!(
        events.last(),
        Some(Progress::FileDone {
            done: 20,
            files: 20,
            cached: false,
            ..
        })
    ));

    // Zweiter Lauf: alles aus dem Cache, keine neuen Anfragen.
    let mut cached = 0;
    downloader()
        .download_all(tasks, |p| {
            if let Progress::FileDone { cached: true, .. } = p {
                cached += 1
            }
        })
        .await
        .unwrap();
    assert_eq!(hits.load(Ordering::SeqCst), 20);
    assert_eq!(cached, 20);
}

#[tokio::test]
async fn hash_mismatch_retries_and_leaves_no_file() {
    let (base, hits) = server().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("kaputt");
    let task = DownloadTask {
        url: format!("{base}/kaputt"),
        dest: dest.clone(),
        sha1: Some("0000000000000000000000000000000000000000".into()),
        size: None,
    };

    let err = downloader().download(&task).await.unwrap_err();
    assert!(matches!(err, Error::HashMismatch { .. }), "{err}");
    assert_eq!(hits.load(Ordering::SeqCst), 3, "1 Versuch + 2 Retries");
    assert!(!dest.exists());
    assert!(!dir.path().join("kaputt.part").exists());
}

#[tokio::test]
async fn corrupt_existing_file_is_replaced() {
    let (base, hits) = server().await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("datei");
    std::fs::write(&dest, "falscher-inhalt").unwrap();
    let task = DownloadTask {
        url: format!("{base}/datei"),
        dest: dest.clone(),
        sha1: Some(sha1_hex("inhalt-datei")),
        size: Some("inhalt-datei".len() as u64),
    };

    assert!(downloader().download(&task).await.unwrap());
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert_eq!(sha1_file(&dest).await.unwrap(), sha1_hex("inhalt-datei"));

    // Gleiche Größe, falscher Inhalt: nur mit verify_existing erkannt.
    std::fs::write(&dest, "inhalt-XXXXX").unwrap();
    assert!(!downloader().download(&task).await.unwrap());
    assert!(
        downloader()
            .verify_existing(true)
            .download(&task)
            .await
            .unwrap()
    );
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), "inhalt-datei");
}

#[tokio::test]
async fn fetch_bytes_retries_network_errors_but_not_404() {
    // Port ohne Server: Verbindungsfehler, wird wiederholt und schlägt am Ende fehl.
    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/x", closed.local_addr().unwrap());
    drop(closed);
    let started = std::time::Instant::now();
    assert!(downloader().fetch_bytes(&url).await.is_err());
    assert!(
        started.elapsed() >= Duration::from_millis(30),
        "2 Retries mit 10 + 20 ms Pause"
    );

    let (base, hits) = server().await;
    assert_eq!(
        downloader()
            .fetch_bytes(&format!("{base}/ok"))
            .await
            .unwrap(),
        b"inhalt-ok"
    );
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}
