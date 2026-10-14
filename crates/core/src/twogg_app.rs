//! Anbindung an twogg.app (Community-Plattform): Events öffentlich, mit Konto auch Anmeldungen,
//! Server-Adressen und Freunde.
//!
//! Die API (`api.twogg.app`) kennt die Clients `web`, `desktop` und `mobile` und verlangt eine
//! Mindestversion. Pinecone meldet sich als `desktop` mit der Version, die `/health` gerade
//! verlangt; Login über Discord läuft wie bei der TwoGG-Desktop-App über einen Handoff-Token
//! (`twogg://auth/handoff?token=…`), den Pinecone abfängt und gegen Zugangsdaten tauscht.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::OnceCell;

use crate::accounts::SecretStore;
use crate::{Error, Result};

pub const DEFAULT_API: &str = "https://api.twogg.app";
pub const WEB: &str = "https://twogg.app";
const FALLBACK_VERSION: &str = "1.0.27";
const REFRESH_KEY: &str = "twogg-app-refresh";

/// API-Adresse: `TWOGG_APP_API` (z. B. für Tests), sonst api.twogg.app.
pub fn api_url() -> String {
    std::env::var("TWOGG_APP_API")
        .ok()
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_API.to_string())
        .trim_end_matches('/')
        .to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEvent {
    pub id: String,
    /// Ohne Emojis (Pinecone zeigt keine).
    pub title: String,
    /// Erster Absatz der Beschreibung als Klartext.
    pub summary: String,
    /// `STANDARD`, `TEAM` oder `ROLEPLAY`.
    pub event_type: String,
    /// ISO-Zeitpunkt; fehlt, solange der Termin offen ist.
    pub event_date: Option<String>,
    pub registration_deadline: Option<String>,
    pub registration_closed: bool,
    pub requires_application: bool,
    pub minecraft_version: Option<String>,
    pub max_players: Option<u32>,
    pub team_size: Option<u32>,
    pub logo_url: Option<String>,
    pub banner_url: Option<String>,
    pub ended: bool,
    /// Seite des Events auf twogg.app (Anmeldung, Teams, Gilde).
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub mc_username: Option<String>,
    #[serde(default)]
    pub mc_uuid: Option<String>,
}

/// Stand des angemeldeten Nutzers bei einem Event.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Participation {
    pub event_id: String,
    /// `registered`, `team`, `pre` (Vormerkung) oder `invite`.
    pub kind: String,
    /// `approved`, `pending` … wie von twogg.app geliefert.
    pub status: String,
    /// Team bzw. Gilde.
    pub team_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinInfo {
    #[serde(default)]
    pub approved: bool,
    #[serde(default)]
    pub server_address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppFriend {
    pub id: String,
    pub username: String,
    pub mc_username: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone)]
struct Tokens {
    access: String,
    refresh: String,
}

/// Client mit gespeicherter Anmeldung (Refresh-Token im Schlüsselbund).
pub struct TwoggApp {
    http: reqwest::Client,
    base: String,
    version: OnceCell<String>,
    store: Arc<dyn SecretStore>,
    tokens: Mutex<Option<Tokens>>,
}

impl TwoggApp {
    pub fn new(store: Arc<dyn SecretStore>) -> Result<Self> {
        Self::with_base(api_url(), store)
    }

    pub fn with_base(base: impl Into<String>, store: Arc<dyn SecretStore>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(crate::user_agent())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|source| Error::Http {
                url: String::new(),
                source,
            })?;
        Ok(Self {
            http,
            base: base.into().trim_end_matches('/').to_string(),
            version: OnceCell::new(),
            store,
            tokens: Mutex::new(None),
        })
    }

    /// Discord-Login im Browserfenster; endet mit einer Weiterleitung auf `twogg://auth/handoff`.
    pub fn discord_login_url(&self) -> String {
        format!("{}/auth/discord?client=desktop", self.base)
    }

    /// Ist ein Konto verknüpft (Refresh-Token vorhanden)?
    pub fn linked(&self) -> bool {
        self.tokens.lock().unwrap().is_some()
            || self.store.get(REFRESH_KEY).ok().flatten().is_some()
    }

    async fn client_version(&self) -> &str {
        self.version
            .get_or_init(|| async {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Health {
                    required_desktop_version: Option<String>,
                }
                let url = format!("{}/health", self.base);
                match self.http.get(&url).send().await {
                    Ok(r) => r
                        .json::<Health>()
                        .await
                        .ok()
                        .and_then(|h| h.required_desktop_version),
                    Err(_) => None,
                }
                .unwrap_or_else(|| FALLBACK_VERSION.to_string())
            })
            .await
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        auth: Option<&str>,
    ) -> Result<reqwest::RequestBuilder> {
        let version = self.client_version().await.to_string();
        let mut req = self
            .http
            .request(method, format!("{}{path}", self.base))
            .header("X-Twogg-Client", "desktop")
            .header("X-Twogg-Client-Version", version);
        if let Some(token) = auth {
            req = req.bearer_auth(token);
        }
        Ok(req)
    }

    async fn send(&self, req: reqwest::RequestBuilder, path: &str) -> Result<reqwest::Response> {
        let url = format!("{}{path}", self.base);
        let resp = req.send().await.map_err(|source| Error::Http {
            url: url.clone(),
            source,
        })?;
        if resp.status().is_success() {
            return Ok(resp);
        }
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let message = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_string))
            .unwrap_or_else(|| format!("HTTP {status}"));
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(Error::Auth(format!("twogg.app: {message}")));
        }
        Err(Error::Invalid(format!("twogg.app: {message}")))
    }

    async fn json<T: serde::de::DeserializeOwned>(
        &self,
        resp: reqwest::Response,
        path: &str,
    ) -> Result<T> {
        let url = format!("{}{path}", self.base);
        resp.json()
            .await
            .map_err(|source| Error::Http { url, source })
    }

    /// Öffentliche Events, nächste zuerst, beendete hinten.
    pub async fn events(&self) -> Result<Vec<AppEvent>> {
        let path = "/events?limit=50";
        let req = self.request(reqwest::Method::GET, path, None).await?;
        let resp = self.send(req, path).await?;
        let body: Value = self.json(resp, path).await?;
        let mut events: Vec<AppEvent> = body["items"]
            .as_array()
            .map(|items| items.iter().filter_map(parse_event).collect())
            .unwrap_or_default();
        events.sort_by(|a, b| {
            a.ended.cmp(&b.ended).then_with(|| match a.ended {
                // Beendete: neueste zuerst; offene: nächster Termin zuerst, „Termin folgt“ danach.
                true => b.event_date.cmp(&a.event_date),
                false => match (&a.event_date, &b.event_date) {
                    (Some(x), Some(y)) => x.cmp(y),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.title.cmp(&b.title),
                },
            })
        });
        Ok(events)
    }

    // --- Anmeldung -------------------------------------------------------------------------

    /// Tauscht den Handoff-Token aus der Discord-Weiterleitung gegen eine Anmeldung.
    pub async fn redeem_handoff(&self, token: &str) -> Result<AppUser> {
        let path = "/auth/desktop-handoff";
        let req = self
            .request(reqwest::Method::POST, path, None)
            .await?
            .json(&serde_json::json!({ "token": token }));
        let resp = self.send(req, path).await?;
        let body: Value = self.json(resp, path).await?;
        self.take_tokens(&body)?;
        self.me().await
    }

    /// Anmeldung mit E-Mail und Passwort (twogg.app-Konten ohne Discord).
    pub async fn login_password(&self, email: &str, password: &str) -> Result<AppUser> {
        let path = "/auth/login";
        let req = self
            .request(reqwest::Method::POST, path, None)
            .await?
            .json(&serde_json::json!({ "email": email, "password": password }));
        let resp = self.send(req, path).await?;
        let body: Value = self.json(resp, path).await?;
        self.take_tokens(&body)?;
        self.me().await
    }

    pub async fn logout(&self) -> Result<()> {
        let tokens = self.tokens.lock().unwrap().take();
        if let Some(t) = tokens {
            // Sitzung auch serverseitig beenden; scheitert das, reicht das lokale Abmelden.
            let path = "/auth/logout";
            if let Ok(req) = self
                .request(reqwest::Method::POST, path, Some(&t.access))
                .await
            {
                let _ = req.send().await;
            }
        }
        self.store.delete(REFRESH_KEY)
    }

    fn take_tokens(&self, body: &Value) -> Result<()> {
        let access = body["accessToken"].as_str();
        let refresh = body["refreshToken"].as_str();
        let (Some(access), Some(refresh)) = (access, refresh) else {
            return Err(Error::Auth(
                "twogg.app hat keine Zugangsdaten geliefert".into(),
            ));
        };
        self.store.set(REFRESH_KEY, refresh)?;
        *self.tokens.lock().unwrap() = Some(Tokens {
            access: access.into(),
            refresh: refresh.into(),
        });
        Ok(())
    }

    async fn refresh(&self) -> Result<String> {
        let refresh = self
            .tokens
            .lock()
            .unwrap()
            .as_ref()
            .map(|t| t.refresh.clone());
        let refresh = match refresh {
            Some(r) => r,
            None => self
                .store
                .get(REFRESH_KEY)?
                .ok_or_else(|| Error::Auth("Kein twogg.app-Konto verknüpft".into()))?,
        };
        let path = "/auth/refresh";
        let req = self
            .request(reqwest::Method::POST, path, None)
            .await?
            .json(&serde_json::json!({ "refreshToken": refresh }));
        let resp = match self.send(req, path).await {
            Ok(r) => r,
            Err(Error::Auth(m)) => {
                // Abgelaufen oder widerrufen: Verknüpfung aufheben.
                *self.tokens.lock().unwrap() = None;
                self.store.delete(REFRESH_KEY)?;
                return Err(Error::Auth(m));
            }
            Err(e) => return Err(e),
        };
        let body: Value = self.json(resp, path).await?;
        let access = body["accessToken"]
            .as_str()
            .ok_or_else(|| Error::Auth("twogg.app hat keinen Zugang geliefert".into()))?
            .to_string();
        let new_refresh = body["refreshToken"]
            .as_str()
            .unwrap_or(&refresh)
            .to_string();
        if new_refresh != refresh {
            self.store.set(REFRESH_KEY, &new_refresh)?;
        }
        *self.tokens.lock().unwrap() = Some(Tokens {
            access: access.clone(),
            refresh: new_refresh,
        });
        Ok(access)
    }

    /// Angemeldete Anfrage; bei abgelaufenem Zugang einmal erneuern und wiederholen.
    async fn authed(&self, method: reqwest::Method, path: &str) -> Result<reqwest::Response> {
        let current = self
            .tokens
            .lock()
            .unwrap()
            .as_ref()
            .map(|t| t.access.clone());
        let access = match current {
            Some(a) => a,
            None => self.refresh().await?,
        };
        let req = self.request(method.clone(), path, Some(&access)).await?;
        match self.send(req, path).await {
            Err(Error::Auth(_)) => {
                let access = self.refresh().await?;
                let req = self.request(method, path, Some(&access)).await?;
                self.send(req, path).await
            }
            other => other,
        }
    }

    pub async fn me(&self) -> Result<AppUser> {
        let path = "/auth/me";
        let resp = self.authed(reqwest::Method::GET, path).await?;
        let body: Value = self.json(resp, path).await?;
        // Je nach Version direkt oder unter `user`.
        let user = if body.get("user").is_some_and(Value::is_object) {
            body["user"].clone()
        } else {
            body
        };
        let mut user: AppUser =
            serde_json::from_value(user).map_err(Error::json("twogg.app /auth/me"))?;
        user.avatar_url = user.avatar_url.and_then(image_url);
        Ok(user)
    }

    pub async fn participations(&self) -> Result<Vec<Participation>> {
        let path = "/events/my/registrations";
        let resp = self.authed(reqwest::Method::GET, path).await?;
        let body: Value = self.json(resp, path).await?;
        Ok(parse_participations(&body))
    }

    /// Server-Adresse eines Events, sobald sie für den Nutzer freigegeben ist.
    pub async fn join_info(&self, event_id: &str) -> Result<JoinInfo> {
        if !valid_id(event_id) {
            return Err(Error::Invalid("Ungültige Event-ID".into()));
        }
        let path = format!("/events/{event_id}/join");
        let resp = self.authed(reqwest::Method::GET, &path).await?;
        self.json(resp, &path).await
    }

    pub async fn friends(&self) -> Result<Vec<AppFriend>> {
        let path = "/friends";
        let resp = self.authed(reqwest::Method::GET, path).await?;
        let body: Value = self.json(resp, path).await?;
        Ok(parse_friends(&body))
    }
}

/// Liest den Token aus `twogg://auth/handoff?token=…`; andere Adressen ergeben `None`.
pub fn handoff_token(url: &str) -> Option<String> {
    let url = reqwest::Url::parse(url).ok()?;
    if url.scheme() != "twogg" || url.host_str() != Some("auth") || url.path() != "/handoff" {
        return None;
    }
    url.query_pairs()
        .find(|(k, _)| k == "token")
        .map(|(_, v)| v.into_owned())
        .filter(|t| !t.is_empty() && t.len() < 4096)
}

/// Fehlermeldung aus `twogg://auth/handoff?error=…`.
pub fn handoff_error(url: &str) -> Option<String> {
    let url = reqwest::Url::parse(url).ok()?;
    if url.scheme() != "twogg" {
        return None;
    }
    url.query_pairs()
        .find(|(k, _)| k == "error")
        .map(|(_, v)| v.into_owned())
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Bild-Adresse von twogg.app: absolute https-Adressen bleiben, Pfade wie `/storage/…` zeigen
/// auf twogg.app, alles andere (z. B. `javascript:`) fällt weg.
fn image_url(u: String) -> Option<String> {
    if u.starts_with("https://") {
        Some(u)
    } else if u.starts_with('/') && !u.starts_with("//") {
        Some(format!("https://twogg.app{u}"))
    } else {
        None
    }
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Nur Text, keine Emojis oder Symbole aus Titeln (Design: keine Emojis in der Oberfläche).
pub fn strip_emoji(s: &str) -> String {
    let kept: String = s
        .chars()
        .filter(|c| {
            let n = *c as u32;
            !(matches!(n, 0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0xFE0F | 0x200D | 0x2B00..=0x2BFF))
        })
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Erster sinnvoller Absatz einer Markdown-/HTML-Beschreibung als Klartext.
pub fn summary(markdown: &str) -> String {
    let no_tags = regex::Regex::new(r"<[^>]*>")
        .unwrap()
        .replace_all(markdown, "");
    let paragraph = no_tags
        .split("\n\n")
        .map(str::trim)
        .find(|p| {
            !p.is_empty()
                && !p.starts_with('#')
                && !p.starts_with("---")
                && !p.starts_with('|')
                // Einzeilige Untertitel in Kursiv überspringen, wenn danach mehr kommt.
                && !(p.starts_with('*') && p.ends_with('*') && !p.contains('\n') && p.len() < 80)
        })
        .unwrap_or("");
    let plain = regex::Regex::new(r"[*_`>]|\[([^\]]*)\]\([^)]*\)")
        .unwrap()
        .replace_all(paragraph, "$1");
    strip_emoji(&plain)
}

fn parse_event(v: &Value) -> Option<AppEvent> {
    let id = str_field(v, "id")?;
    if !valid_id(&id) {
        return None;
    }
    Some(AppEvent {
        url: format!("{WEB}/events/{id}"),
        title: strip_emoji(&str_field(v, "title")?),
        summary: summary(v["description"].as_str().unwrap_or("")),
        event_type: str_field(v, "eventType").unwrap_or_else(|| "STANDARD".into()),
        event_date: str_field(v, "eventDate"),
        registration_deadline: str_field(v, "registrationDeadline"),
        registration_closed: v["registrationClosed"].as_bool().unwrap_or(false),
        requires_application: v["requiresApplication"].as_bool().unwrap_or(false),
        minecraft_version: str_field(v, "minecraftVersion"),
        max_players: v["maxPlayers"].as_u64().map(|n| n as u32),
        team_size: v["pluginConfig"]["team_size"].as_u64().map(|n| n as u32),
        logo_url: str_field(v, "logoUrl").and_then(image_url),
        banner_url: str_field(v, "bannerUrl").and_then(image_url),
        ended: !v["matchEndedAt"].is_null(),
        id,
    })
}

fn parse_participations(body: &Value) -> Vec<Participation> {
    let mut out = Vec::new();
    for r in body["registrations"].as_array().into_iter().flatten() {
        if let Some(event_id) = str_field(&r["event"], "id") {
            out.push(Participation {
                event_id,
                kind: "registered".into(),
                status: str_field(r, "status").unwrap_or_default(),
                team_name: None,
            });
        }
    }
    for t in body["teams"].as_array().into_iter().flatten() {
        let team = &t["team"];
        if let Some(event_id) =
            str_field(team, "eventId").or_else(|| str_field(&team["event"], "id"))
        {
            out.push(Participation {
                event_id,
                kind: "team".into(),
                status: str_field(team, "status").unwrap_or_default(),
                team_name: str_field(team, "teamName"),
            });
        }
    }
    for p in body["preRegistrations"].as_array().into_iter().flatten() {
        if let Some(event_id) = str_field(p, "eventId") {
            out.push(Participation {
                event_id,
                kind: "pre".into(),
                status: str_field(p, "status").unwrap_or_default(),
                team_name: None,
            });
        }
    }
    for i in body["teamInvites"].as_array().into_iter().flatten() {
        if let Some(event_id) = str_field(&i["event"], "id") {
            out.push(Participation {
                event_id,
                kind: "invite".into(),
                status: "pending".into(),
                team_name: str_field(i, "teamName"),
            });
        }
    }
    out
}

fn parse_friends(body: &Value) -> Vec<AppFriend> {
    body["friendships"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["status"].as_str() == Some("accepted"))
        .filter_map(|f| {
            let u = &f["user"];
            Some(AppFriend {
                id: str_field(u, "id")?,
                username: str_field(u, "username")?,
                mc_username: str_field(u, "mcUsername"),
                avatar_url: str_field(u, "avatarUrl").and_then(image_url),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_urls() {
        assert_eq!(
            handoff_token("twogg://auth/handoff?token=abc123").as_deref(),
            Some("abc123")
        );
        assert_eq!(handoff_token("twogg://auth/handoff"), None);
        assert_eq!(
            handoff_token("https://evil.example/auth/handoff?token=x"),
            None
        );
        assert_eq!(
            handoff_error("twogg://auth/handoff?error=oauth_failed").as_deref(),
            Some("oauth_failed")
        );
    }

    #[test]
    fn events_are_cleaned_up() {
        let raw = serde_json::json!({
            "id": "94181fb6-d005-45f2-80d2-0216826ffd65",
            "title": "LATEST 2026 🔱 RISE OF ATLANTIS",
            "description": "*Das jährliche PvP-Survival-Event von TwoGG*\n\n**LATEST** ist das jährliche Aushängeschild-Event von TwoGG: ein **Minecraft-PvP-Survival**.\n\n---\n\n### Thema",
            "eventType": "TEAM",
            "eventDate": "2026-10-03T17:00:00.000Z",
            "pluginConfig": { "team_size": 2 },
            "logoUrl": "https://twogg.app/storage/events/admin/x.png",
            "bannerUrl": "javascript:alert(1)",
            "matchEndedAt": "2026-10-03T18:36:49.600Z"
        });
        let e = parse_event(&raw).unwrap();
        assert_eq!(e.title, "LATEST 2026 RISE OF ATLANTIS");
        assert_eq!(
            e.summary,
            "LATEST ist das jährliche Aushängeschild-Event von TwoGG: ein Minecraft-PvP-Survival."
        );
        assert!(e.ended);
        assert_eq!(e.team_size, Some(2));
        assert_eq!(e.banner_url, None);
        assert_eq!(
            image_url("/storage/avatars/a.png".into()).as_deref(),
            Some("https://twogg.app/storage/avatars/a.png")
        );
        assert_eq!(image_url("//evil.example/x.png".into()), None);
        assert_eq!(
            e.url,
            "https://twogg.app/events/94181fb6-d005-45f2-80d2-0216826ffd65"
        );

        let iframe = serde_json::json!({ "id": "a", "title": "WINTERLAND: Icebreaker",
            "description": "<iframe src=\"/events/x.html\"></iframe>", "matchEndedAt": null });
        let e = parse_event(&iframe).unwrap();
        assert_eq!(e.summary, "");
        assert!(!e.ended);
        assert!(parse_event(&serde_json::json!({ "id": "../x", "title": "t" })).is_none());
    }

    #[test]
    fn participations_and_friends() {
        let body = serde_json::json!({
            "registrations": [{ "id": "r", "status": "approved", "event": { "id": "e1", "title": "A" } }],
            "teams": [{ "team": { "eventId": "e2", "teamName": "Die Treue Hand", "status": "approved" } }],
            "preRegistrations": [{ "eventId": "e3", "status": "pending" }],
            "teamInvites": [{ "teamName": "Eisenherz", "event": { "id": "e4" } }]
        });
        let p = parse_participations(&body);
        assert_eq!(p.len(), 4);
        assert_eq!(p[1].team_name.as_deref(), Some("Die Treue Hand"));
        assert_eq!(p[2].kind, "pre");

        let friends = parse_friends(&serde_json::json!({ "friendships": [
            { "status": "accepted", "user": { "id": "u1", "username": "Rex", "mcUsername": "RexMC", "avatarUrl": null } },
            { "status": "pending", "user": { "id": "u2", "username": "Lina" } }
        ]}));
        assert_eq!(friends.len(), 1);
        assert_eq!(friends[0].mc_username.as_deref(), Some("RexMC"));
    }
}
