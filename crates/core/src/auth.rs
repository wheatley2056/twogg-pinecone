//! Accounts für den Spielstart. Login und Speicherung: siehe `msa` und `accounts`.

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountKind {
    Offline,
    Microsoft,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub name: String,
    pub uuid: Uuid,
    pub access_token: String,
    pub kind: AccountKind,
}

impl Account {
    /// Offline-Account wie im Vanilla-Server: UUID v3 aus `OfflinePlayer:<name>`.
    pub fn offline(name: &str) -> Result<Self> {
        if !valid_player_name(name) {
            return Err(Error::InvalidPlayerName(name.to_string()));
        }
        Ok(Self {
            name: name.to_string(),
            uuid: offline_uuid(name),
            access_token: "0".into(),
            kind: AccountKind::Offline,
        })
    }

    /// Wert für `${user_type}`.
    pub fn user_type(&self) -> &'static str {
        match self.kind {
            AccountKind::Offline => "legacy",
            AccountKind::Microsoft => "msa",
        }
    }
}

pub fn valid_player_name(name: &str) -> bool {
    (1..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Entspricht Javas `UUID.nameUUIDFromBytes("OfflinePlayer:" + name)`.
pub fn offline_uuid(name: &str) -> Uuid {
    let mut bytes: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_matches_java() {
        assert_eq!(
            offline_uuid("Notch").to_string(),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert_eq!(
            offline_uuid("wheatley").to_string(),
            "1c68cbf6-4084-3cfa-9a10-b5a342601f7f"
        );
        assert_eq!(offline_uuid("Notch").get_version_num(), 3);
    }

    #[test]
    fn player_names() {
        assert!(Account::offline("Tobi_04").is_ok());
        assert!(Account::offline("").is_err());
        assert!(Account::offline("Leer zeichen").is_err());
        assert!(Account::offline("VielZuLangerName17").is_err());
    }
}
