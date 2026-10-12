//! Kern von TwoGG Pinecone. Kennt keine GUI und wird von CLI und Tauri-Backend genutzt.

pub mod accounts;
pub mod archive;
pub mod auth;
pub mod content;
pub mod curseforge;
pub mod diagnostics;
pub mod download;
pub mod error;
pub mod forge;
pub mod hooks;
pub mod install;
pub mod java;
pub mod jvm;
pub mod launch;
pub mod launcher;
pub mod loader;
pub mod manifest;
pub mod migrate;
pub mod modpack;
pub mod modrinth;
pub mod msa;
pub mod nbt;
pub mod paths;
pub mod profiles;
pub mod rules;
pub mod screenshots;
pub mod servers;
pub mod settings;
pub mod share;
pub mod skins;
pub mod stats;
pub mod syncpacks;
pub mod twogg;
pub mod twogg_app;
pub mod worlds;

pub use error::{Error, Result};

/// Name und Version, die dem Spiel als `${launcher_name}` / `${launcher_version}` übergeben werden.
pub const LAUNCHER_NAME: &str = "twogg-pinecone";
pub const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
