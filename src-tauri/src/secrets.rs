use anyhow::{anyhow, Result};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

pub trait SecretBackend: Send + Sync {
    fn set_secret(&self, key: &str, secret: &str) -> Result<()>;
    fn get_secret(&self, key: &str) -> Result<String>;
    fn delete_secret(&self, key: &str) -> Result<()>;
    fn has_secret(&self, key: &str) -> Result<bool>;
}

#[derive(Debug)]
struct KeyringBackend {
    service_name: String,
}

impl KeyringBackend {
    fn entry(&self, key: &str) -> Result<Entry> {
        Ok(Entry::new(&self.service_name, key)?)
    }
}

impl SecretBackend for KeyringBackend {
    fn set_secret(&self, key: &str, secret: &str) -> Result<()> {
        self.entry(key)?.set_password(secret)?;
        Ok(())
    }

    fn get_secret(&self, key: &str) -> Result<String> {
        Ok(self.entry(key)?.get_password()?)
    }

    fn delete_secret(&self, key: &str) -> Result<()> {
        match self.entry(key)?.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    fn has_secret(&self, key: &str) -> Result<bool> {
        match self.entry(key)?.get_password() {
            Ok(value) => Ok(!value.is_empty()),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }
}

#[derive(Debug, Clone)]
struct FileBackend {
    path: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedSecrets {
    secrets: BTreeMap<String, String>,
}

impl FileBackend {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn load(&self) -> Result<PersistedSecrets> {
        if !self.path.exists() {
            return Ok(PersistedSecrets::default());
        }
        let raw = fs::read_to_string(&self.path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn save_all(&self, config: &PersistedSecrets) -> Result<()> {
        ensure_parent(&self.path)?;
        let raw = serde_json::to_string_pretty(config)?;
        fs::write(&self.path, raw)?;
        set_private_permissions(&self.path)?;
        Ok(())
    }
}

impl SecretBackend for FileBackend {
    fn set_secret(&self, key: &str, secret: &str) -> Result<()> {
        let mut config = self.load()?;
        config.secrets.insert(key.to_string(), secret.to_string());
        self.save_all(&config)
    }

    fn get_secret(&self, key: &str) -> Result<String> {
        self.load()?
            .secrets
            .get(key)
            .cloned()
            .ok_or_else(|| anyhow!("No matching entry found in secure storage"))
    }

    fn delete_secret(&self, key: &str) -> Result<()> {
        let mut config = self.load()?;
        config.secrets.remove(key);
        self.save_all(&config)
    }

    fn has_secret(&self, key: &str) -> Result<bool> {
        Ok(self.load()?.secrets.get(key).is_some_and(|value| !value.is_empty()))
    }
}

#[derive(Clone)]
pub struct SecretService {
    backend: Arc<dyn SecretBackend>,
    legacy_fallback: Arc<dyn SecretBackend>,
}

impl SecretService {
    pub fn new(app: &AppHandle, service_name: impl Into<String>) -> Result<Self> {
        let config_dir = app
            .path()
            .app_config_dir()
            .map_err(|error| anyhow!("Could not resolve the app config directory for secrets. {}", error))?;
        Ok(Self {
            backend: Arc::new(KeyringBackend {
                service_name: service_name.into(),
            }),
            legacy_fallback: Arc::new(FileBackend::new(config_dir.join("secrets.json"))),
        })
    }

    pub fn save(&self, key: &str, secret: &str) -> Result<()> {
        self.backend.set_secret(key, secret)?;
        match self.backend.get_secret(key) {
            Ok(value) if value == secret => {
                let _ = self.legacy_fallback.delete_secret(key);
            }
            _ => {
                // Keep an app-private fallback copy when the platform keyring write cannot
                // be verified immediately. This avoids "saved successfully" followed by a
                // missing-password failure on the next action.
                self.legacy_fallback.set_secret(key, secret)?;
            }
        }
        Ok(())
    }

    pub fn read(&self, key: &str) -> Result<String> {
        if let Ok(value) = self.backend.get_secret(key) {
            if !value.is_empty() {
                let _ = self.legacy_fallback.delete_secret(key);
                return Ok(value);
            }
        }

        if let Ok(legacy_value) = self.legacy_fallback.get_secret(key) {
            if legacy_value.is_empty() {
                return Err(anyhow!("Secret entry exists but is empty."));
            }

            let _ = self.backend.set_secret(key, &legacy_value);
            return Ok(legacy_value);
        }

        Err(anyhow!(
            "No password is saved for this profile. Enter the password again and save the profile to store it securely."
        ))
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        let backend_result = self.backend.delete_secret(key);
        let fallback_result = self.legacy_fallback.delete_secret(key);

        match (backend_result, fallback_result) {
            (Ok(_), _) | (_, Ok(_)) => Ok(()),
            (Err(primary), Err(_)) => Err(primary),
        }
    }

    pub fn exists(&self, key: &str) -> Result<bool> {
        match self.backend.has_secret(key) {
            Ok(true) => return Ok(true),
            Ok(false) => {}
            Err(_) => {}
        }
        self.legacy_fallback.has_secret(key)
    }
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        set_private_dir_permissions(parent)?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(unix)]
fn set_private_dir_permissions(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn set_private_dir_permissions(_path: &Path) -> Result<()> {
    Ok(())
}
