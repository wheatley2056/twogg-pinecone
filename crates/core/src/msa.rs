//! Microsoft-Login: Device-Code-Flow → Xbox Live → XSTS → Minecraft-Services → Profil.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Error, LAUNCHER_NAME, LAUNCHER_VERSION, Result};

const SCOPE: &str = "XboxLive.signin offline_access";

/// Client-ID der Azure-App. Laufzeit-Variable vor eingebautem Wert beim Kompilieren.
pub fn client_id_from_env() -> Option<String> {
    std::env::var("PINECONE_MSA_CLIENT_ID")
        .ok()
        .or_else(|| option_env!("PINECONE_MSA_CLIENT_ID").map(str::to_string))
        .filter(|id| !id.trim().is_empty())
}

#[derive(Debug, Clone)]
pub struct Endpoints {
    /// OAuth2-Basis, z. B. `https://login.microsoftonline.com/consumers/oauth2/v2.0`.
    pub microsoft: String,
    pub xbox_user: String,
    pub xsts: String,
    pub minecraft: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            microsoft: "https://login.microsoftonline.com/consumers/oauth2/v2.0".into(),
            xbox_user: "https://user.auth.xboxlive.com/user/authenticate".into(),
            xsts: "https://xsts.auth.xboxlive.com/xsts/authorize".into(),
            minecraft: "https://api.minecraftservices.com".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    #[serde(default = "default_interval")]
    pub interval: u64,
    /// Fertiger Hinweistext von Microsoft (Sprache je nach Konto).
    #[serde(default)]
    pub message: String,
}

fn default_interval() -> u64 {
    5
}

#[derive(Debug, Clone, Deserialize)]
pub struct MicrosoftTokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    /// UUID ohne Bindestriche.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<Skin>,
    #[serde(default)]
    pub capes: Vec<Cape>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Skin {
    pub id: String,
    pub state: String,
    pub url: String,
    #[serde(default)]
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cape {
    pub id: String,
    pub state: String,
    pub url: String,
    #[serde(default)]
    pub alias: Option<String>,
}

impl Profile {
    pub fn active_skin(&self) -> Option<&Skin> {
        self.skins.iter().find(|s| s.state == "ACTIVE")
    }
}

/// Ergebnis eines erfolgreichen Logins.
#[derive(Debug, Clone)]
pub struct MinecraftLogin {
    pub profile: Profile,
    /// Bearer-Token für das Spiel (`${auth_access_token}`), ca. 24 h gültig.
    pub access_token: String,
    /// Neuer Microsoft-Refresh-Token; ersetzt den alten.
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct OAuthError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxResponse {
    token: String,
    display_claims: DisplayClaims,
}

#[derive(Debug, Deserialize)]
struct DisplayClaims {
    xui: Vec<XboxUser>,
}

#[derive(Debug, Deserialize)]
struct XboxUser {
    uhs: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxError {
    #[serde(default)]
    x_err: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct MinecraftToken {
    access_token: String,
}

pub struct MsaClient {
    http: reqwest::Client,
    client_id: String,
    endpoints: Endpoints,
}

fn auth(msg: impl Into<String>) -> Error {
    Error::Auth(msg.into())
}

impl MsaClient {
    pub fn new(client_id: impl Into<String>) -> Result<Self> {
        Self::with_endpoints(client_id, Endpoints::default())
    }

    pub fn with_endpoints(client_id: impl Into<String>, endpoints: Endpoints) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(format!("{LAUNCHER_NAME}/{LAUNCHER_VERSION}"))
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|source| Error::Http {
                url: String::new(),
                source,
            })?;
        Ok(Self {
            http,
            client_id: client_id.into(),
            endpoints,
        })
    }

    /// Schritt 1: Code anfordern, den der Nutzer unter `verification_uri` eingibt.
    pub async fn request_device_code(&self) -> Result<DeviceCode> {
        let url = format!("{}/devicecode", self.endpoints.microsoft);
        let resp = self
            .http
            .post(&url)
            .form(&[("client_id", self.client_id.as_str()), ("scope", SCOPE)])
            .send()
            .await
            .map_err(|source| Error::Http {
                url: url.clone(),
                source,
            })?;
        let status = resp.status();
        let body = resp.text().await.map_err(|source| Error::Http {
            url: url.clone(),
            source,
        })?;
        if !status.is_success() {
            let detail = serde_json::from_str::<OAuthError>(&body)
                .map(|e| e.error_description.unwrap_or(e.error))
                .unwrap_or(body);
            return Err(auth(format!(
                "Microsoft lehnt die Anfrage ab ({status}): {detail}"
            )));
        }
        serde_json::from_str(&body).map_err(Error::json("Device-Code-Antwort"))
    }

    /// Schritt 2: Warten, bis der Nutzer den Code bestätigt hat.
    pub async fn poll_device_code(&self, code: &DeviceCode) -> Result<MicrosoftTokens> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(code.expires_in);
        let mut interval = Duration::from_secs(code.interval);
        loop {
            match self
                .token_request(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("client_id", &self.client_id),
                    ("device_code", &code.device_code),
                ])
                .await?
            {
                Ok(tokens) => return Ok(tokens),
                Err(e) => match e.error.as_str() {
                    "authorization_pending" => {}
                    "slow_down" => interval += Duration::from_secs(5),
                    "authorization_declined" => return Err(auth("Anmeldung wurde abgelehnt")),
                    "expired_token" => {
                        return Err(auth("Der Code ist abgelaufen, bitte erneut anmelden"));
                    }
                    other => {
                        return Err(auth(format!(
                            "Anmeldung fehlgeschlagen: {}",
                            e.error_description.unwrap_or(other.into())
                        )));
                    }
                },
            }
            if tokio::time::Instant::now() + interval > deadline {
                return Err(auth("Der Code ist abgelaufen, bitte erneut anmelden"));
            }
            tokio::time::sleep(interval).await;
        }
    }

    /// Neuen Microsoft-Token mit dem gespeicherten Refresh-Token holen.
    pub async fn refresh(&self, refresh_token: &str) -> Result<MicrosoftTokens> {
        self.token_request(&[
            ("grant_type", "refresh_token"),
            ("client_id", &self.client_id),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ])
        .await?
        .map_err(|e| {
            auth(format!(
                "Sitzung abgelaufen, bitte neu anmelden ({})",
                e.error_description.unwrap_or(e.error)
            ))
        })
    }

    async fn token_request(
        &self,
        form: &[(&str, &str)],
    ) -> Result<Result<MicrosoftTokens, OAuthError>> {
        let url = format!("{}/token", self.endpoints.microsoft);
        let resp = self
            .http
            .post(&url)
            .form(form)
            .send()
            .await
            .map_err(|source| Error::Http {
                url: url.clone(),
                source,
            })?;
        let status = resp.status();
        let body = resp.text().await.map_err(|source| Error::Http {
            url: url.clone(),
            source,
        })?;
        if status.is_success() {
            Ok(Ok(
                serde_json::from_str(&body).map_err(Error::json("Token-Antwort"))?
            ))
        } else {
            Ok(Err(serde_json::from_str(&body).unwrap_or(OAuthError {
                error: format!("HTTP {status}"),
                error_description: Some(body),
            })))
        }
    }

    /// Schritte 3–6: Xbox Live, XSTS, Minecraft-Token und Profil.
    pub async fn login_minecraft(&self, ms: MicrosoftTokens) -> Result<MinecraftLogin> {
        let xbl = self
            .xbox_request(
                &self.endpoints.xbox_user,
                json!({
                    "Properties": {
                        "AuthMethod": "RPS",
                        "SiteName": "user.auth.xboxlive.com",
                        "RpsTicket": format!("d={}", ms.access_token),
                    },
                    "RelyingParty": "http://auth.xboxlive.com",
                    "TokenType": "JWT",
                }),
            )
            .await?;
        let xsts = self
            .xbox_request(
                &self.endpoints.xsts,
                json!({
                    "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl.token] },
                    "RelyingParty": "rp://api.minecraftservices.com/",
                    "TokenType": "JWT",
                }),
            )
            .await?;
        let uhs = xsts
            .display_claims
            .xui
            .first()
            .map(|u| u.uhs.clone())
            .ok_or_else(|| auth("XSTS-Antwort ohne Benutzer-Hash"))?;

        let url = format!(
            "{}/authentication/login_with_xbox",
            self.endpoints.minecraft
        );
        let resp = self
            .http
            .post(&url)
            .json(&json!({ "identityToken": format!("XBL3.0 x={uhs};{}", xsts.token) }))
            .send()
            .await
            .map_err(|source| Error::Http {
                url: url.clone(),
                source,
            })?;
        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(auth(
                "Minecraft lehnt die App ab (403). Neue Azure-Apps müssen erst von Mojang freigeschaltet werden",
            ));
        }
        let mc: MinecraftToken = resp
            .error_for_status()
            .map_err(|source| Error::Http {
                url: url.clone(),
                source,
            })?
            .json()
            .await
            .map_err(|source| Error::Http {
                url: url.clone(),
                source,
            })?;

        let profile = self.profile(&mc.access_token).await?;
        Ok(MinecraftLogin {
            profile,
            access_token: mc.access_token,
            refresh_token: ms.refresh_token,
        })
    }

    /// Minecraft-Profil (Name, UUID, Skins, Capes) zu einem Minecraft-Token.
    pub async fn profile(&self, minecraft_token: &str) -> Result<Profile> {
        let url = format!("{}/minecraft/profile", self.endpoints.minecraft);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(minecraft_token)
            .send()
            .await
            .map_err(|source| Error::Http {
                url: url.clone(),
                source,
            })?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(auth(
                "Dieses Microsoft-Konto besitzt Minecraft: Java Edition nicht (oder hat noch kein Profil)",
            ));
        }
        resp.error_for_status()
            .map_err(|source| Error::Http {
                url: url.clone(),
                source,
            })?
            .json()
            .await
            .map_err(|source| Error::Http { url, source })
    }

    async fn xbox_request(&self, url: &str, body: serde_json::Value) -> Result<XboxResponse> {
        let resp = self
            .http
            .post(url)
            .header("x-xbl-contract-version", "1")
            .json(&body)
            .send()
            .await
            .map_err(|source| Error::Http {
                url: url.into(),
                source,
            })?;
        let status = resp.status();
        let text = resp.text().await.map_err(|source| Error::Http {
            url: url.into(),
            source,
        })?;
        if !status.is_success() {
            let code = serde_json::from_str::<XboxError>(&text)
                .ok()
                .and_then(|e| e.x_err);
            return Err(auth(xbox_error_message(code, status.as_u16())));
        }
        serde_json::from_str(&text).map_err(Error::json(url))
    }
}

/// Verständliche Meldungen für bekannte Xbox-Fehlercodes.
pub fn xbox_error_message(code: Option<u64>, status: u16) -> String {
    match code {
        Some(2148916227) => "Dieses Xbox-Konto ist gesperrt".into(),
        Some(2148916233) => "Zu diesem Microsoft-Konto gibt es kein Xbox-Profil. Bitte einmal auf xbox.com anmelden und ein Profil anlegen".into(),
        Some(2148916235) => "Xbox Live ist in deinem Land nicht verfügbar".into(),
        Some(2148916236) | Some(2148916237) => "Das Konto muss erst eine Altersverifizierung abschließen (Südkorea)".into(),
        Some(2148916238) => "Kinderkonto: Ein Erwachsener muss das Konto erst einer Microsoft-Familie hinzufügen".into(),
        Some(other) => format!("Xbox-Anmeldung fehlgeschlagen (Fehlercode {other})"),
        None => format!("Xbox-Anmeldung fehlgeschlagen (HTTP {status})"),
    }
}
