//! The paired bridge, stored in `~/.config/kandela/bridge.json`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub bridge_id: String,
    pub bridge_name: String,
    pub address: String,
    pub app_key: String,
    #[serde(default)]
    pub client_key: Option<String>,
    /// Hex SHA-256 of the bridge's TLS certificate; see [`crate::pinning`].
    /// Missing in configs saved before pinning existed.
    #[serde(default)]
    pub cert_sha256: Option<String>,
}

fn path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("kandela").join("bridge.json"))
}

/// Moves settings from the app's working name (`~/.config/hue-desktop`) to
/// `~/.config/kandela`, so an existing pairing keeps working.
pub fn migrate_legacy() {
    let Some(base) = dirs::config_dir() else {
        return;
    };
    let (old, new) = (base.join("hue-desktop"), base.join("kandela"));
    if !old.is_dir() || new.exists() {
        return;
    }
    match std::fs::rename(&old, &new) {
        Ok(()) => {
            let _ = std::fs::rename(new.join("hue-desktop.conf"), new.join("kandela.conf"));
            log::info!("moved settings from {} to {}", old.display(), new.display());
        }
        Err(e) => log::warn!("could not move {}: {e}", old.display()),
    }
}

pub fn load() -> Option<Config> {
    let text = std::fs::read_to_string(path()?).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(config: &Config) -> std::io::Result<()> {
    let path = path().ok_or_else(|| std::io::Error::other("no config directory"))?;
    std::fs::create_dir_all(path.parent().expect("path has a parent"))?;
    let text = serde_json::to_string_pretty(config).map_err(std::io::Error::other)?;
    // The app key grants full control of the lights; keep it private.
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(text.as_bytes())
    }
    #[cfg(not(unix))]
    std::fs::write(path, text)
}

pub fn remove() {
    if let Some(p) = path() {
        let _ = std::fs::remove_file(p);
    }
}
