//! Gespeicherte Accounts: Liste in `accounts.json`, Microsoft-Refresh-Tokens im OS-Schlüsselbund.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{Account, AccountKind, offline_uuid};
use crate::msa::{MinecraftLogin, MsaClient};
use crate::{Error, LAUNCHER_NAME, Result};

/// Ablage für Geheimnisse (Refresh-Tokens). Im Betrieb der OS-Schlüsselbund, in Tests im Speicher.
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>>;
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
}

/// Windows Credential Manager, macOS Keychain bzw. Secret Service (KWallet, GNOME Keyring).
pub struct KeyringStore;

impl KeyringStore {
    fn entry(key: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(LAUNCHER_NAME, key).map_err(keyring_error)
    }
}

fn keyring_error(e: keyring::Error) -> Error {
    Error::Keyring(e.to_string())
}

impl SecretStore for KeyringStore {
    fn get(&self, key: &str) -> Result<Option<String>> {
        match Self::entry(key)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(keyring_error(e)),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        Self::entry(key)?.set_password(value).map_err(keyring_error)
    }

    fn delete(&self, key: &str) -> Result<()> {
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(keyring_error(e)),
        }
    }
}

#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl SecretStore for MemoryStore {
    fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(self.0.lock().unwrap().get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        self.0.lock().unwrap().insert(key.into(), value.into());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.0.lock().unwrap().remove(key);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredAccount {
    pub uuid: Uuid,
    pub name: String,
    pub kind: AccountKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin_url: Option<String>,
    /// `CLASSIC` oder `SLIM`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin_variant: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Accounts {
    #[serde(default)]
    pub active: Option<Uuid>,
    #[serde(default)]
    pub accounts: Vec<StoredAccount>,
}

impl Accounts {
    pub fn active_account(&self) -> Option<&StoredAccount> {
        let id = self.active?;
        self.accounts.iter().find(|a| a.uuid == id)
    }

    /// Sucht per Name (ohne Groß-/Kleinschreibung) oder UUID.
    pub fn find(&self, selector: &str) -> Option<&StoredAccount> {
        let uuid = Uuid::parse_str(selector).ok();
        self.accounts
            .iter()
            .find(|a| Some(a.uuid) == uuid || a.name.eq_ignore_ascii_case(selector))
    }

    fn upsert(&mut self, account: StoredAccount) {
        self.active = Some(account.uuid);
        match self.accounts.iter_mut().find(|a| a.uuid == account.uuid) {
            Some(existing) => *existing = account,
            None => self.accounts.push(account),
        }
    }
}

/// Schlüssel des Refresh-Tokens im Schlüsselbund.
pub fn refresh_key(uuid: &Uuid) -> String {
    format!("msa-refresh:{}", uuid.simple())
}

pub struct AccountStore<S: SecretStore = KeyringStore> {
    path: PathBuf,
    secrets: S,
}

impl<S: SecretStore> AccountStore<S> {
    pub fn new(path: impl Into<PathBuf>, secrets: S) -> Self {
        Self {
            path: path.into(),
            secrets,
        }
    }

    pub fn secrets(&self) -> &S {
        &self.secrets
    }

    pub fn load(&self) -> Result<Accounts> {
        match std::fs::read(&self.path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(Error::json(self.path.display().to_string()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Accounts::default()),
            Err(e) => Err(Error::io(&self.path)(e)),
        }
    }

    fn save(&self, accounts: &Accounts) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::io(parent))?;
        }
        let json = serde_json::to_vec_pretty(accounts).map_err(Error::json("accounts.json"))?;
        // Erst vollständig schreiben, dann ersetzen: kein halbes accounts.json nach Absturz.
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(Error::io(&tmp))?;
        std::fs::rename(&tmp, &self.path).map_err(Error::io(&self.path))
    }

    fn get(&self, selector: &str) -> Result<(Accounts, StoredAccount)> {
        let accounts = self.load()?;
        let account = accounts
            .find(selector)
            .cloned()
            .ok_or_else(|| Error::Invalid(format!("Kein Account {selector:?} gespeichert")))?;
        Ok((accounts, account))
    }

    /// Speichert ein frisch angemeldetes Microsoft-Konto und macht es aktiv.
    pub fn add_microsoft(&self, login: &MinecraftLogin) -> Result<StoredAccount> {
        let uuid = Uuid::parse_str(&login.profile.id)
            .map_err(|_| Error::Auth(format!("Ungültige Profil-UUID {}", login.profile.id)))?;
        self.secrets
            .set(&refresh_key(&uuid), &login.refresh_token)?;
        let skin = login.profile.active_skin();
        let account = StoredAccount {
            uuid,
            name: login.profile.name.clone(),
            kind: AccountKind::Microsoft,
            skin_url: skin.map(|s| s.url.clone()),
            skin_variant: skin.and_then(|s| s.variant.clone()),
        };
        let mut accounts = self.load()?;
        accounts.upsert(account.clone());
        self.save(&accounts)?;
        Ok(account)
    }

    pub fn add_offline(&self, name: &str) -> Result<StoredAccount> {
        let session = Account::offline(name)?;
        let account = StoredAccount {
            uuid: offline_uuid(name),
            name: session.name,
            kind: AccountKind::Offline,
            skin_url: None,
            skin_variant: None,
        };
        let mut accounts = self.load()?;
        accounts.upsert(account.clone());
        self.save(&accounts)?;
        Ok(account)
    }

    pub fn set_active(&self, selector: &str) -> Result<StoredAccount> {
        let (mut accounts, account) = self.get(selector)?;
        accounts.active = Some(account.uuid);
        self.save(&accounts)?;
        Ok(account)
    }

    /// Entfernt einen Account samt Refresh-Token aus dem Schlüsselbund.
    pub fn remove(&self, selector: &str) -> Result<StoredAccount> {
        let (mut accounts, account) = self.get(selector)?;
        if account.kind == AccountKind::Microsoft {
            self.secrets.delete(&refresh_key(&account.uuid))?;
        }
        accounts.accounts.retain(|a| a.uuid != account.uuid);
        if accounts.active == Some(account.uuid) {
            accounts.active = accounts.accounts.first().map(|a| a.uuid);
        }
        self.save(&accounts)?;
        Ok(account)
    }

    /// Spielbereite Sitzung. Microsoft-Konten werden dafür per Refresh-Token neu angemeldet;
    /// der neue Refresh-Token ersetzt den alten, Name und Skin werden aktualisiert.
    pub async fn session(
        &self,
        selector: Option<&str>,
        msa: Option<&MsaClient>,
    ) -> Result<Account> {
        let accounts = self.load()?;
        let account = match selector {
            Some(sel) => accounts.find(sel),
            None => accounts.active_account(),
        }
        .cloned()
        .ok_or_else(|| match selector {
            Some(sel) => Error::Invalid(format!("Kein Account {sel:?} gespeichert")),
            None => Error::Invalid(
                "Kein aktiver Account. Erst `account login` oder `account offline <name>`".into(),
            ),
        })?;

        match account.kind {
            AccountKind::Offline => Account::offline(&account.name),
            AccountKind::Microsoft => {
                let msa = msa.ok_or_else(|| {
                    Error::Auth(
                        "Microsoft-Login nicht konfiguriert (PINECONE_MSA_CLIENT_ID fehlt)".into(),
                    )
                })?;
                let key = refresh_key(&account.uuid);
                let refresh = self.secrets.get(&key)?.ok_or_else(|| {
                    Error::Auth(format!(
                        "Kein Login für {} gespeichert, bitte neu anmelden",
                        account.name
                    ))
                })?;
                let tokens = msa.refresh(&refresh).await?;
                let login = msa.login_minecraft(tokens).await?;
                if login.profile.id != account.uuid.simple().to_string() {
                    return Err(Error::Auth(
                        "Anmeldung gehört zu einem anderen Minecraft-Profil".into(),
                    ));
                }
                self.add_microsoft(&login)?;
                Ok(Account {
                    name: login.profile.name,
                    uuid: account.uuid,
                    access_token: login.access_token,
                    kind: AccountKind::Microsoft,
                })
            }
        }
    }
}
