//! Discord-Status (F-41): „Spielt REVIVAL: Avalon“ in der lokalen Discord-App. Standardmäßig aus,
//! eingeschaltet unter Einstellungen → Allgemein. Übertragen werden nur Instanzname, Version und
//! Startzeit – nie die Serveradresse. Läuft Discord nicht, passiert nichts.
//!
//! Die Verbindung hält ein eigener Thread; Anfragen kommen über einen Kanal und blockieren nie
//! den Launcher.

use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};

use discord_rich_presence::activity::{Activity, Assets, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

/// Application-ID aus dem Discord-Entwicklerportal; zur Laufzeit oder beim Bauen gesetzt.
pub fn client_id() -> Option<String> {
    std::env::var("PINECONE_DISCORD_CLIENT_ID")
        .ok()
        .or_else(|| option_env!("PINECONE_DISCORD_CLIENT_ID").map(str::to_string))
        .filter(|id| !id.trim().is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
}

enum Msg {
    Playing {
        instance: String,
        version: String,
        started: i64,
    },
    Clear,
}

/// Was gerade angezeigt wird: Instanz-ID des zuletzt gestarteten Spiels.
pub struct Presence {
    tx: Option<Sender<Msg>>,
    shown: Mutex<Option<String>>,
}

impl Presence {
    pub fn new() -> Self {
        let tx = client_id().map(|id| {
            let (tx, rx) = channel::<Msg>();
            std::thread::spawn(move || worker(&id, rx));
            tx
        });
        Self {
            tx,
            shown: Mutex::default(),
        }
    }

    pub fn configured(&self) -> bool {
        self.tx.is_some()
    }

    /// Zuletzt gestartetes Spiel anzeigen (bei mehreren laufenden gilt das neueste).
    pub fn playing(&self, profile_id: &str, instance: &str, version: &str, started: u64) {
        let Some(tx) = &self.tx else { return };
        *self.shown.lock().unwrap() = Some(profile_id.to_string());
        let _ = tx.send(Msg::Playing {
            instance: instance.to_string(),
            version: version.to_string(),
            started: started as i64,
        });
    }

    /// Spiel beendet: nur leeren, wenn genau dieses angezeigt wird.
    pub fn stopped(&self, profile_id: &str) {
        let mut shown = self.shown.lock().unwrap();
        if shown.as_deref() == Some(profile_id) {
            *shown = None;
            self.send_clear();
        }
    }

    /// Status entfernen (z. B. wenn die Einstellung ausgeschaltet wird).
    pub fn clear(&self) {
        *self.shown.lock().unwrap() = None;
        self.send_clear();
    }

    fn send_clear(&self) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Msg::Clear);
        }
    }
}

fn worker(client_id: &str, rx: std::sync::mpsc::Receiver<Msg>) {
    let mut client = DiscordIpcClient::new(client_id);
    let mut connected = false;
    for msg in rx {
        // Discord kann jederzeit starten oder beenden: bei Bedarf neu verbinden, Fehler leise.
        if !connected {
            connected = client.connect().is_ok();
            if !connected {
                continue;
            }
        }
        let result = match &msg {
            Msg::Playing {
                instance,
                version,
                started,
            } => client.set_activity(
                Activity::new()
                    .details(instance.as_str())
                    .state(version.as_str())
                    .timestamps(Timestamps::new().start(*started))
                    .assets(
                        Assets::new()
                            .large_image("pinecone")
                            .large_text("TwoGG Pinecone"),
                    ),
            ),
            Msg::Clear => client.clear_activity(),
        };
        if result.is_err() {
            let _ = client.close();
            connected = false;
        }
    }
    let _ = client.close();
}
