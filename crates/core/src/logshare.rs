//! Log teilen (F-53): Spielausgabe bereinigen und auf Wunsch zu mclo.gs hochladen.
//!
//! Bereinigt werden Tokens (wie im Diagnose-Export), der Benutzername in Pfaden des eigenen
//! Rechners und IP-Adressen. Hochgeladen wird nur der bereinigte Text, den der Nutzer vorher
//! gesehen hat; mclo.gs nimmt höchstens 25 000 Zeilen bzw. 10 MiB, darüber bleiben die letzten.

use std::sync::LazyLock;
use std::time::Duration;

use regex::Regex;
use serde::Deserialize;

use crate::{Error, Result};

pub const API: &str = "https://api.mclo.gs";
pub const MAX_LINES: usize = 25_000;
pub const MAX_BYTES: usize = 10 * 1024 * 1024;

static USER_PATHS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        // /home/<name>/, /Users/<name>/
        r"(/(?:home|Users)/)[^/\s]+",
        // C:\Users\<name>\ (auch mit / und doppelten Backslashes)
        r"(?i)([A-Z]:[\\/]{1,2}Users[\\/]{1,2})[^\\/\s]+",
    ]
    .iter()
    .map(|p| Regex::new(p).expect("gültiges Muster"))
    .collect()
});

static IPV4: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b")
        .expect("gültiges Muster")
});

/// Bereinigt ein Log und kürzt es auf die Grenzen von mclo.gs (die letzten Zeilen bleiben).
pub fn prepare(text: &str) -> String {
    let mut out = crate::diagnostics::redact(text);
    for re in USER_PATHS.iter() {
        out = re.replace_all(&out, "${1}<user>").into_owned();
    }
    // Vierteilige Zahlengruppen sind fast immer IPs. Ausnahme: Gruppen ab `0.` bzw. `1.` sind
    // eher Versionen (z. B. `1.20.1.2`) als Adressen, die einen Spieler verraten.
    out = IPV4
        .replace_all(&out, |c: &regex::Captures| {
            let ip = &c[0];
            if ip.starts_with("0.") || ip.starts_with("1.") {
                ip.to_string()
            } else {
                "<ip>".into()
            }
        })
        .into_owned();
    let lines: Vec<&str> = out.lines().collect();
    let mut start = lines.len().saturating_sub(MAX_LINES);
    let mut size: usize = lines[start..].iter().map(|l| l.len() + 1).sum();
    while size > MAX_BYTES && start < lines.len() {
        size -= lines[start].len() + 1;
        start += 1;
    }
    lines[start..].join("\n")
}

#[derive(Deserialize)]
struct Response {
    success: bool,
    url: Option<String>,
    error: Option<String>,
}

/// Lädt einen schon bereinigten Text hoch und gibt den Link zurück. Zur Sicherheit wird er
/// noch einmal bereinigt.
pub async fn upload(base: &str, text: &str) -> Result<String> {
    let content = prepare(text);
    if content.trim().is_empty() {
        return Err(Error::Invalid("Das Log ist leer".into()));
    }
    let url = format!("{}/1/log", base.trim_end_matches('/'));
    let http_err = |source| Error::Http {
        url: url.clone(),
        source,
    };
    let http = reqwest::Client::builder()
        .user_agent(crate::user_agent())
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(http_err)?;
    let response = http
        .post(&url)
        .form(&[("content", content.as_str())])
        .send()
        .await
        .map_err(http_err)?;
    let status = response.status();
    let body: Response = response.json().await.map_err(http_err)?;
    match (body.success, body.url) {
        (true, Some(link)) => Ok(link),
        _ => Err(Error::Invalid(format!(
            "mclo.gs hat das Log abgelehnt ({status}): {}",
            body.error.unwrap_or_else(|| "unbekannter Fehler".into())
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_tokens_user_names_and_ips() {
        let log = "Setting user: Tobi\n\
            --accessToken eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijklmnop\n\
            Loading /home/tobias/.local/share/twogg-pinecone/instances/a\n\
            Loading C:\\Users\\Tobias Kern\\AppData\\Roaming\n\
            Connecting to 192.168.178.20, 25565\n\
            Fabric Loader 0.16.5, Minecraft 1.21.11";
        let out = prepare(log);
        assert!(!out.contains("eyJ"), "{out}");
        assert!(out.contains("/home/<user>/.local"), "{out}");
        assert!(!out.contains("tobias"), "{out}");
        assert!(out.contains(r"C:\Users\<user>"), "{out}");
        assert!(out.contains("Connecting to <ip>, 25565"), "{out}");
        assert!(
            out.contains("Minecraft 1.21.11"),
            "Versionen bleiben: {out}"
        );
    }

    #[test]
    fn keeps_the_last_lines() {
        let log: String = (0..MAX_LINES + 10)
            .map(|i| format!("Zeile {i}\n"))
            .collect();
        let out = prepare(&log);
        assert_eq!(out.lines().count(), MAX_LINES);
        assert!(out.starts_with("Zeile 10\n"));
        assert!(out.ends_with(&format!("Zeile {}", MAX_LINES + 9)));
    }
}
