use crate::process::ProcessManager;
use crate::profiles::ProfilesService;
use crate::secrets::SecretService;
use anyhow::Result;
use tauri::AppHandle;

pub struct AppState {
    pub app: AppHandle,
    pub profiles: ProfilesService,
    pub secrets: SecretService,
    pub process: ProcessManager,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Result<Self> {
        Ok(Self {
            app: app.clone(),
            profiles: ProfilesService::new(app)?,
            secrets: SecretService::new(app, "Git FTP Desktop")?,
            process: ProcessManager::new(),
        })
    }
}
