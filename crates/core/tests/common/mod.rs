//! Minimaler HTTP-Mock-Server für Tests: feste Antworten je Route, Anfragen werden protokolliert.
// Nicht jeder Test nutzt jede Hilfsfunktion.
#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

type Routes = HashMap<(String, String), VecDeque<(u16, String)>>;

#[derive(Clone)]
pub struct MockServer {
    pub base: String,
    routes: Arc<Mutex<Routes>>,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl MockServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = MockServer {
            base: format!("http://{}", listener.local_addr().unwrap()),
            routes: Arc::default(),
            requests: Arc::default(),
        };
        let s = server.clone();
        tokio::spawn(async move {
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                let s = s.clone();
                tokio::spawn(async move { s.handle(socket).await });
            }
        });
        server
    }

    /// Antwort für `METHOD /pfad` einreihen. Mehrere Antworten werden der Reihe nach geliefert,
    /// die letzte bleibt danach bestehen.
    pub fn on(&self, method: &str, path: &str, status: u16, body: impl Into<String>) -> &Self {
        self.routes
            .lock()
            .unwrap()
            .entry((method.into(), path.into()))
            .or_default()
            .push_back((status, body.into()));
        self
    }

    /// Bisherige Antworten einer Route verwerfen und durch diese ersetzen.
    pub fn replace(&self, method: &str, path: &str, status: u16, body: impl Into<String>) -> &Self {
        self.routes
            .lock()
            .unwrap()
            .remove(&(method.to_string(), path.to_string()));
        self.on(method, path, status, body)
    }

    /// Anfragen an `path` (ohne Query-String verglichen).
    pub fn requests(&self, path: &str) -> Vec<Request> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.path.split('?').next() == Some(path))
            .cloned()
            .collect()
    }

    async fn handle(&self, mut socket: tokio::net::TcpStream) {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let header_end = loop {
            let n = socket.read(&mut chunk).await.unwrap();
            if n == 0 {
                return;
            }
            buf.extend_from_slice(&chunk[..n]);
            if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break i + 4;
            }
        };
        let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
        let mut lines = head.lines();
        let mut first = lines.next().unwrap_or_default().split_whitespace();
        let method = first.next().unwrap_or_default().to_string();
        let path = first.next().unwrap_or_default().to_string();
        let headers: HashMap<String, String> = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
            .collect();
        let len: usize = headers
            .get("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        while buf.len() < header_end + len {
            let n = socket.read(&mut chunk).await.unwrap();
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        let body = String::from_utf8_lossy(&buf[header_end..]).into_owned();
        self.requests.lock().unwrap().push(Request {
            method: method.clone(),
            path: path.clone(),
            headers,
            body,
        });

        let (status, body) = {
            let mut routes = self.routes.lock().unwrap();
            // Routen ohne Query-String vergleichen; protokolliert wird der volle Pfad.
            let route = path.split('?').next().unwrap_or_default().to_string();
            match routes.get_mut(&(method, route)) {
                Some(queue) if queue.len() > 1 => queue.pop_front().unwrap(),
                Some(queue) => queue.front().cloned().unwrap(),
                None => (404, "{}".into()),
            }
        };
        let response = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(response.as_bytes()).await;
    }
}
