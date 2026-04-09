use crate::models::{DeploymentProfile, RunRecord, SavedRepository};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use tokio::fs;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RepositoryConfig {
    path: String,
    profiles: Vec<DeploymentProfile>,
    last_selected_profile_id: Option<String>,
    last_opened_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PersistedConfig {
    repositories: Vec<RepositoryConfig>,
    run_history: Vec<RunRecord>,
}

pub struct ProfilesService {
    config_path: PathBuf,
    write_lock: Mutex<()>,
}

impl ProfilesService {
    pub fn new(app: &AppHandle) -> Result<Self> {
        let config_dir = app
            .path()
            .app_config_dir()
            .context("Could not resolve the app config directory.")?;
        Ok(Self {
            config_path: config_dir.join("config.json"),
            write_lock: Mutex::new(()),
        })
    }

    pub async fn list_profiles(&self, repo_path: &str) -> Result<Vec<DeploymentProfile>> {
        let _guard = self.write_lock.lock().await;
        let config = self.load_config().await?;
        Ok(config
            .repositories
            .into_iter()
            .find(|repo| repo.path == repo_path)
            .map(|repo| repo.profiles)
            .unwrap_or_default())
    }

    pub async fn get_profile(&self, repo_path: &str, profile_id: &str) -> Result<DeploymentProfile> {
        let profiles = self.list_profiles(repo_path).await?;
        profiles
            .into_iter()
            .find(|profile| profile.id == profile_id)
            .ok_or_else(|| anyhow!("Profile not found."))
    }

    pub async fn save_profile(&self, repo_path: &str, profile: DeploymentProfile) -> Result<DeploymentProfile> {
        let _guard = self.write_lock.lock().await;
        let mut config = self.load_config().await?;
        let repo = Self::repo_mut(&mut config, repo_path);
        if let Some(index) = repo.profiles.iter().position(|item| item.id == profile.id) {
            repo.profiles[index] = profile.clone();
        } else {
            repo.profiles.push(profile.clone());
        }
        repo.last_selected_profile_id = Some(profile.id.clone());
        self.save_config(&config).await?;
        Ok(profile)
    }

    pub async fn delete_profile(&self, repo_path: &str, profile_id: &str) -> Result<Option<DeploymentProfile>> {
        let _guard = self.write_lock.lock().await;
        let mut config = self.load_config().await?;
        let repo = Self::repo_mut(&mut config, repo_path);
        let removed = if let Some(index) = repo.profiles.iter().position(|item| item.id == profile_id) {
            Some(repo.profiles.remove(index))
        } else {
            None
        };
        if repo.last_selected_profile_id.as_deref() == Some(profile_id) {
            repo.last_selected_profile_id = repo.profiles.first().map(|profile| profile.id.clone());
        }
        self.save_config(&config).await?;
        Ok(removed)
    }

    pub async fn remove_repository(&self, repo_path: &str) -> Result<Option<Vec<DeploymentProfile>>> {
        let _guard = self.write_lock.lock().await;
        let mut config = self.load_config().await?;
        let removed = if let Some(index) = config.repositories.iter().position(|repo| repo.path == repo_path) {
            let repo = config.repositories.remove(index);
            config.run_history.retain(|run| run.repo_path != repo_path);
            Some(repo.profiles)
        } else {
            None
        };
        self.save_config(&config).await?;
        Ok(removed)
    }

    pub async fn mark_repo_opened(&self, repo_path: &str) -> Result<()> {
        let _guard = self.write_lock.lock().await;
        let mut config = self.load_config().await?;
        let repo = Self::repo_mut(&mut config, repo_path);
        repo.last_opened_at = Some(crate::git_ftp::timestamp_now());
        self.save_config(&config).await
    }

    pub async fn set_last_selected_profile(&self, repo_path: &str, profile_id: Option<&str>) -> Result<()> {
        let _guard = self.write_lock.lock().await;
        let mut config = self.load_config().await?;
        let repo = Self::repo_mut(&mut config, repo_path);
        repo.last_selected_profile_id = profile_id.map(ToOwned::to_owned);
        self.save_config(&config).await
    }

    pub async fn append_run_history(&self, record: RunRecord) -> Result<()> {
        let _guard = self.write_lock.lock().await;
        let mut config = self.load_config().await?;
        config.run_history.insert(0, record);
        config.run_history.truncate(60);
        self.save_config(&config).await
    }

    pub async fn update_profile_last_deployed(
        &self,
        repo_path: &str,
        profile_id: &str,
        timestamp: &str,
    ) -> Result<()> {
        let _guard = self.write_lock.lock().await;
        let mut config = self.load_config().await?;
        let repo = Self::repo_mut(&mut config, repo_path);
        if let Some(profile) = repo.profiles.iter_mut().find(|item| item.id == profile_id) {
            profile.last_deployed_at = Some(timestamp.to_string());
            profile.updated_at = timestamp.to_string();
        }
        self.save_config(&config).await
    }

    pub async fn get_run_history(&self, repo_path: Option<&str>) -> Result<Vec<RunRecord>> {
        let _guard = self.write_lock.lock().await;
        let config = self.load_config().await?;
        let history = match repo_path {
            Some(path) => config
                .run_history
                .into_iter()
                .filter(|run| run.repo_path == path)
                .collect(),
            None => config.run_history,
        };
        Ok(history)
    }

    pub async fn list_known_repositories(&self) -> Result<Vec<SavedRepository>> {
        let _guard = self.write_lock.lock().await;
        let mut repositories = self
            .load_config()
            .await?
            .repositories
            .into_iter()
            .map(|repo| SavedRepository {
                path: repo.path,
                last_opened_at: repo.last_opened_at,
                profile_count: repo.profiles.len(),
                last_selected_profile_id: repo.last_selected_profile_id,
            })
            .collect::<Vec<_>>();

        repositories.sort_by(|left, right| {
            right
                .last_opened_at
                .as_deref()
                .cmp(&left.last_opened_at.as_deref())
                .then_with(|| left.path.cmp(&right.path))
        });

        Ok(repositories)
    }

    pub async fn has_repository(&self, repo_path: &str) -> Result<bool> {
        let _guard = self.write_lock.lock().await;
        let config = self.load_config().await?;
        Ok(config.repositories.iter().any(|repo| repo.path == repo_path))
    }

    async fn load_config(&self) -> Result<PersistedConfig> {
        if !self.config_path.exists() {
            return Ok(PersistedConfig::default());
        }
        let raw = fs::read_to_string(&self.config_path)
            .await
            .with_context(|| format!("Could not read config file: {}", self.config_path.display()))?;
        let parsed = serde_json::from_str::<PersistedConfig>(&raw)
            .with_context(|| format!("Could not parse config file: {}", self.config_path.display()))?;
        Ok(parsed)
    }

    async fn save_config(&self, config: &PersistedConfig) -> Result<()> {
        ensure_parent(&self.config_path).await?;
        let raw = serde_json::to_string_pretty(config)?;
        fs::write(&self.config_path, raw)
            .await
            .with_context(|| format!("Could not write config file: {}", self.config_path.display()))?;
        Ok(())
    }

    fn repo_mut<'a>(config: &'a mut PersistedConfig, repo_path: &str) -> &'a mut RepositoryConfig {
        if let Some(index) = config.repositories.iter().position(|repo| repo.path == repo_path) {
            &mut config.repositories[index]
        } else {
            config.repositories.push(RepositoryConfig {
                path: repo_path.to_string(),
                ..RepositoryConfig::default()
            });
            config.repositories.last_mut().expect("repository inserted")
        }
    }
}

async fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    Ok(())
}
