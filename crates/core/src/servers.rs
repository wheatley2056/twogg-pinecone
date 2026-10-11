//! Server: Liste aus `servers.dat`, Adressen (inkl. SRV-Eintrag) und Status per Server List Ping.

use std::net::IpAddr;
use std::path::Path;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::{Error, Result, nbt};

pub const DEFAULT_PORT: u16 = 25565;
const TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerAddress {
    pub host: String,
    /// `None`: Standardport bzw. SRV-Eintrag.
    pub port: Option<u16>,
}

impl ServerAddress {
    /// `host`, `host:port`, `1.2.3.4:25565`, `[::1]:25565` oder `::1`.
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();
        let bad = || Error::Invalid(format!("Ungültige Serveradresse {input:?}"));
        if input.is_empty() || input.contains(char::is_whitespace) {
            return Err(bad());
        }
        if let Some(rest) = input.strip_prefix('[') {
            let (host, after) = rest.split_once(']').ok_or_else(bad)?;
            let port = match after.strip_prefix(':') {
                Some(p) => Some(p.parse().map_err(|_| bad())?),
                None if after.is_empty() => None,
                None => return Err(bad()),
            };
            return Ok(Self {
                host: host.into(),
                port,
            });
        }
        // Mehr als ein Doppelpunkt ohne Klammern: nackte IPv6-Adresse ohne Port.
        if input.matches(':').count() > 1 {
            input.parse::<IpAddr>().map_err(|_| bad())?;
            return Ok(Self {
                host: input.into(),
                port: None,
            });
        }
        match input.split_once(':') {
            Some((host, port)) if !host.is_empty() => Ok(Self {
                host: host.into(),
                port: Some(port.parse().map_err(|_| bad())?),
            }),
            Some(_) => Err(bad()),
            None => Ok(Self {
                host: input.into(),
                port: None,
            }),
        }
    }

    /// Für `--quickPlayMultiplayer` bzw. Anzeige: `host` oder `host:port`.
    pub fn display(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        match self.port {
            Some(p) => format!("{host}:{p}"),
            None => host,
        }
    }

    pub fn port_or_default(&self) -> u16 {
        self.port.unwrap_or(DEFAULT_PORT)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedServer {
    pub name: String,
    pub address: String,
    /// Server-Icon als PNG in Base64 (wie in `servers.dat` gespeichert).
    pub icon: Option<String>,
}

/// Serverliste eines Spielordners. Fehlt `servers.dat`, ist die Liste leer.
pub fn read_servers_dat(game_dir: &Path) -> Result<Vec<SavedServer>> {
    let path = game_dir.join("servers.dat");
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(&path)(e)),
    };
    let root = nbt::read(&data)?;
    let list = root
        .get("servers")
        .and_then(|s| s.as_list())
        .unwrap_or_default();
    Ok(list
        .iter()
        .filter(|s| s.get("hidden").and_then(|h| h.as_i64()) != Some(1))
        .filter_map(|s| {
            Some(SavedServer {
                name: s
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default()
                    .to_string(),
                address: s.get("ip")?.as_str()?.to_string(),
                icon: s.get("icon").and_then(|i| i.as_str()).map(str::to_string),
            })
        })
        .collect())
}

/// Letzter Server aus einem Spiel-Log (`Connecting to host, port`).
pub fn last_server_in_log(log: &str) -> Option<ServerAddress> {
    let line = log
        .lines()
        .rev()
        .find_map(|l| l.split_once("Connecting to ").map(|(_, rest)| rest))?;
    let (host, port) = line.trim().rsplit_once(", ")?;
    let port: u16 = port.trim().parse().ok()?;
    Some(ServerAddress {
        host: host.to_string(),
        port: (port != DEFAULT_PORT).then_some(port),
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    /// Nachricht des Tages ohne Formatierungscodes.
    pub motd: String,
    pub version: String,
    pub protocol: i32,
    pub online: u32,
    pub max: u32,
    pub ping_ms: u32,
    /// `data:image/png;base64,…`
    pub favicon: Option<String>,
}

#[derive(Deserialize)]
struct RawStatus {
    #[serde(default)]
    version: Option<RawVersion>,
    #[serde(default)]
    players: Option<RawPlayers>,
    #[serde(default)]
    description: serde_json::Value,
    #[serde(default)]
    favicon: Option<String>,
}

#[derive(Deserialize)]
struct RawVersion {
    name: String,
    protocol: i32,
}

#[derive(Deserialize)]
struct RawPlayers {
    max: u32,
    online: u32,
}

/// Text einer Chat-Komponente (String, `{"text","extra"}` oder Liste) ohne `§`-Codes.
pub fn chat_to_plain(value: &serde_json::Value) -> String {
    fn walk(v: &serde_json::Value, out: &mut String) {
        match v {
            serde_json::Value::String(s) => out.push_str(s),
            serde_json::Value::Array(items) => items.iter().for_each(|i| walk(i, out)),
            serde_json::Value::Object(map) => {
                if let Some(t) = map.get("text") {
                    walk(t, out);
                }
                if let Some(extra) = map.get("extra") {
                    walk(extra, out);
                }
            }
            _ => {}
        }
    }
    let mut raw = String::new();
    walk(value, &mut raw);
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out.trim().to_string()
}

fn write_varint(buf: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        if v & !0x7f == 0 {
            buf.push(v as u8);
            return;
        }
        buf.push((v as u8 & 0x7f) | 0x80);
        v >>= 7;
    }
}

async fn read_varint(stream: &mut TcpStream) -> Result<i32> {
    let mut result = 0u32;
    for i in 0..5 {
        let b = stream.read_u8().await.map_err(net_err)?;
        result |= ((b & 0x7f) as u32) << (7 * i);
        if b & 0x80 == 0 {
            return Ok(result as i32);
        }
    }
    Err(Error::Invalid(
        "Ungültige Serverantwort (VarInt zu lang)".into(),
    ))
}

fn packet(id: i32, data: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    write_varint(&mut body, id);
    body.extend_from_slice(data);
    let mut out = Vec::new();
    write_varint(&mut out, body.len() as i32);
    out.extend(body);
    out
}

fn net_err(e: std::io::Error) -> Error {
    Error::Invalid(format!("Server nicht erreichbar: {e}"))
}

/// Löst die Adresse auf: ohne Port wird zuerst `_minecraft._tcp.<host>` (SRV) gefragt.
async fn resolve(addr: &ServerAddress) -> Result<(String, u16)> {
    if addr.port.is_some() || addr.host.parse::<IpAddr>().is_ok() || addr.host == "localhost" {
        return Ok((addr.host.clone(), addr.port_or_default()));
    }
    let lookup = async {
        let resolver = hickory_resolver::TokioResolver::builder_tokio()
            .ok()?
            .build()
            .ok()?;
        let lookup = resolver
            .srv_lookup(format!("_minecraft._tcp.{}.", addr.host))
            .await
            .ok()?;
        // Niedrigste Priorität zuerst (RFC 2782); Gewichtung ist für einen Ping unerheblich.
        let srv = lookup
            .answers()
            .iter()
            .filter_map(|r| match &r.data {
                hickory_resolver::proto::rr::RData::SRV(srv) => Some(srv),
                _ => None,
            })
            .min_by_key(|srv| srv.priority)?;
        let target = srv.target.to_utf8();
        Some((target.trim_end_matches('.').to_string(), srv.port))
    };
    match tokio::time::timeout(Duration::from_secs(3), lookup).await {
        Ok(Some(found)) => Ok(found),
        _ => Ok((addr.host.clone(), DEFAULT_PORT)),
    }
}

/// Server List Ping (ab Minecraft 1.7): Status-JSON und Antwortzeit.
pub async fn ping(addr: &ServerAddress) -> Result<ServerStatus> {
    tokio::time::timeout(TIMEOUT, ping_inner(addr))
        .await
        .map_err(|_| Error::Invalid(format!("{} antwortet nicht", addr.display())))?
}

async fn ping_inner(addr: &ServerAddress) -> Result<ServerStatus> {
    let (host, port) = resolve(addr).await?;
    let mut stream = TcpStream::connect((host.as_str(), port))
        .await
        .map_err(net_err)?;
    stream.set_nodelay(true).ok();

    // Handshake mit der ursprünglich eingegebenen Adresse (wichtig für Proxys mit virtuellen Hosts).
    let mut hs = Vec::new();
    write_varint(&mut hs, 767);
    write_varint(&mut hs, addr.host.len() as i32);
    hs.extend(addr.host.as_bytes());
    hs.extend(port.to_be_bytes());
    write_varint(&mut hs, 1);
    let mut out = packet(0x00, &hs);
    out.extend(packet(0x00, &[]));
    let sent = Instant::now();
    stream.write_all(&out).await.map_err(net_err)?;

    let _len = read_varint(&mut stream).await?;
    let first_byte_ms = sent.elapsed().as_millis() as u32;
    if read_varint(&mut stream).await? != 0x00 {
        return Err(Error::Invalid("Unerwartete Serverantwort".into()));
    }
    let json_len = read_varint(&mut stream).await?;
    if !(0..=1 << 20).contains(&json_len) {
        return Err(Error::Invalid("Serverantwort zu groß".into()));
    }
    let mut json = vec![0u8; json_len as usize];
    stream.read_exact(&mut json).await.map_err(net_err)?;
    let raw: RawStatus = serde_json::from_slice(&json).map_err(Error::json("Serverstatus"))?;

    // Eigentlicher Ping; manche Server schließen vorher die Verbindung, dann zählt die Statusantwort.
    let ping_ms = async {
        let start = Instant::now();
        stream
            .write_all(&packet(0x01, &0x5049_4e45_434f_4e45i64.to_be_bytes()))
            .await
            .ok()?;
        read_varint(&mut stream).await.ok()?;
        Some(start.elapsed().as_millis() as u32)
    }
    .await
    .unwrap_or(first_byte_ms);

    Ok(ServerStatus {
        motd: chat_to_plain(&raw.description),
        version: raw
            .version
            .as_ref()
            .map(|v| chat_to_plain(&serde_json::Value::String(v.name.clone())))
            .unwrap_or_default(),
        protocol: raw.version.map(|v| v.protocol).unwrap_or(-1),
        online: raw.players.as_ref().map(|p| p.online).unwrap_or(0),
        max: raw.players.map(|p| p.max).unwrap_or(0),
        ping_ms,
        favicon: raw
            .favicon
            .filter(|f| f.starts_with("data:image/png;base64,")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn parses_addresses() {
        let a = |s: &str| ServerAddress::parse(s).unwrap();
        assert_eq!(
            a("mc.twogg.de"),
            ServerAddress {
                host: "mc.twogg.de".into(),
                port: None
            }
        );
        assert_eq!(a("mc.twogg.de:25570").port, Some(25570));
        assert_eq!(a("127.0.0.1:25565").host, "127.0.0.1");
        assert_eq!(
            a("[::1]:25566"),
            ServerAddress {
                host: "::1".into(),
                port: Some(25566)
            }
        );
        assert_eq!(a("::1").port, None);
        assert_eq!(a("[::1]:25566").display(), "[::1]:25566");
        assert!(ServerAddress::parse("").is_err());
        assert!(ServerAddress::parse("host:abc").is_err());
        assert!(ServerAddress::parse(":25565").is_err());
        assert!(ServerAddress::parse("mit leerzeichen").is_err());
    }

    #[test]
    fn chat_components() {
        let v = serde_json::json!({"text": "§aTwoGG ", "extra": [{"text": "Server", "bold": true}, " §7| 1.21"]});
        assert_eq!(chat_to_plain(&v), "TwoGG Server | 1.21");
        assert_eq!(chat_to_plain(&serde_json::json!("§6Hallo")), "Hallo");
    }

    #[test]
    fn last_server_from_log() {
        let log = "[12:00:00] [Render thread/INFO]: Connecting to alt.example, 25565\n\
                   [12:30:00] [Render thread/INFO]: Connecting to mc.twogg.de, 25570\n\
                   [12:31:00] [Render thread/INFO]: Stopping!";
        assert_eq!(
            last_server_in_log(log),
            Some(ServerAddress {
                host: "mc.twogg.de".into(),
                port: Some(25570)
            })
        );
        assert_eq!(last_server_in_log("nichts"), None);
    }

    #[test]
    fn reads_saved_servers_without_hidden() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_servers_dat(dir.path()).unwrap().is_empty());
        let data = crate::nbt::tests::servers_dat(&[
            ("TwoGG", "mc.twogg.de", false),
            ("Versteckt", "x", true),
        ]);
        std::fs::write(dir.path().join("servers.dat"), data).unwrap();
        let servers = read_servers_dat(dir.path()).unwrap();
        assert_eq!(
            servers,
            [SavedServer {
                name: "TwoGG".into(),
                address: "mc.twogg.de".into(),
                icon: None
            }]
        );
    }

    /// Minimaler Minecraft-Server, der nur den Status beantwortet.
    async fn fake_server(json: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            // Handshake und Status-Request lesen.
            for _ in 0..2 {
                let len = read_varint(&mut s).await.unwrap();
                let mut buf = vec![0; len as usize];
                s.read_exact(&mut buf).await.unwrap();
            }
            let mut data = Vec::new();
            write_varint(&mut data, json.len() as i32);
            data.extend(json.as_bytes());
            s.write_all(&packet(0x00, &data)).await.unwrap();
            // Ping beantworten.
            let len = read_varint(&mut s).await.unwrap();
            let mut buf = vec![0; len as usize];
            s.read_exact(&mut buf).await.unwrap();
            s.write_all(&packet(0x01, &buf[1..])).await.unwrap();
        });
        port
    }

    #[tokio::test]
    async fn pings_status() {
        let port = fake_server(
            r#"{"version":{"name":"Paper 1.21.1","protocol":767},"players":{"max":50,"online":7},
               "description":{"text":"§2TwoGG §fCommunity"},"favicon":"data:image/png;base64,AAAA"}"#,
        )
        .await;
        let status = ping(&ServerAddress {
            host: "127.0.0.1".into(),
            port: Some(port),
        })
        .await
        .unwrap();
        assert_eq!(status.motd, "TwoGG Community");
        assert_eq!(status.version, "Paper 1.21.1");
        assert_eq!((status.online, status.max), (7, 50));
        assert_eq!(
            status.favicon.as_deref(),
            Some("data:image/png;base64,AAAA")
        );
    }

    #[tokio::test]
    async fn unreachable_server_errors() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        assert!(
            ping(&ServerAddress {
                host: "127.0.0.1".into(),
                port: Some(port)
            })
            .await
            .is_err()
        );
    }
}
