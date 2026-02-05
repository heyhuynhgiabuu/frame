//! Auto-save service for Frame projects
//!
//! Periodically saves project state during recording to prevent data loss.
//! Also handles recovery from incomplete recordings after crashes.

use crate::{FrameResult, Project, Recording};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{debug, info, warn};

/// Configuration for auto-save behavior
#[derive(Debug, Clone)]
pub struct AutoSaveConfig {
    /// How often to auto-save during recording
    pub interval: Duration,
    /// Whether auto-save is enabled
    pub enabled: bool,
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(10),
            enabled: true,
        }
    }
}

/// Service that manages auto-saving projects
pub struct AutoSaveService {
    config: AutoSaveConfig,
    /// The current project being auto-saved (public for advanced usage)
    pub current_project: Option<Project>,
    last_save: Option<SystemTime>,
    is_saving: Arc<Mutex<bool>>,
}

impl AutoSaveService {
    /// Create a new auto-save service with default config
    pub fn new() -> Self {
        Self::with_config(AutoSaveConfig::default())
    }

    /// Create a new auto-save service with custom config
    pub fn with_config(config: AutoSaveConfig) -> Self {
        Self {
            config,
            current_project: None,
            last_save: None,
            is_saving: Arc::new(Mutex::new(false)),
        }
    }

    /// Check if auto-save is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Enable auto-save
    pub fn enable(&mut self) {
        self.config.enabled = true;
        info!("Auto-save enabled");
    }

    /// Disable auto-save
    pub fn disable(&mut self) {
        self.config.enabled = false;
        info!("Auto-save disabled");
    }

    /// Set auto-save interval
    pub fn set_interval(&mut self, interval: Duration) {
        self.config.interval = interval;
        debug!("Auto-save interval set to {:?}", interval);
    }

    /// Start a new project for auto-saving
    pub fn start_project(&mut self, name: impl Into<String>) -> FrameResult<Project> {
        let project = Project::new(name);

        // Create project directory and initial save
        std::fs::create_dir_all(project.project_dir())?;
        project.save()?;

        self.current_project = Some(project.clone());
        self.last_save = Some(SystemTime::now());

        info!("Started auto-save for project: {}", project.id);
        Ok(project)
    }

    /// Get the current project being auto-saved
    pub fn current_project(&self) -> Option<&Project> {
        self.current_project.as_ref()
    }

    /// Manually trigger a save
    pub async fn save_now(&mut self) -> FrameResult<()> {
        if let Some(project) = &self.current_project {
            let mut saving = self.is_saving.lock().await;
            *saving = true;

            project.save()?;
            self.last_save = Some(SystemTime::now());

            *saving = false;
            debug!("Project {} auto-saved", project.id);
        }
        Ok(())
    }

    /// Add a recording to the current project and save
    pub async fn add_recording(&mut self, recording: Recording) -> FrameResult<()> {
        // First, update the project state
        let project_id = if let Some(project) = &mut self.current_project {
            project.recordings.push(recording);
            project.updated_at = chrono::Utc::now();
            Some(project.id.clone())
        } else {
            None
        };

        // Then save separately to avoid borrow conflict
        if project_id.is_some() {
            self.save_now().await?;
            info!("Recording added to project {}", project_id.unwrap());
        }
        Ok(())
    }

    /// Mark the current recording as complete and finalize project
    pub async fn finalize_project(&mut self) -> FrameResult<Option<Project>> {
        if let Some(project) = &mut self.current_project {
            project.updated_at = chrono::Utc::now();
            project.save()?;

            info!("Project {} finalized", project.id);

            // Remove any incomplete recording markers
            let incomplete_marker = project.project_dir().join(".incomplete");
            if incomplete_marker.exists() {
                std::fs::remove_file(incomplete_marker)?;
            }

            let finalized_project = project.clone();
            self.current_project = None;
            self.last_save = None;

            Ok(Some(finalized_project))
        } else {
            Ok(None)
        }
    }

    /// Mark the current recording as incomplete (for crash recovery)
    pub fn mark_incomplete(&self) -> FrameResult<()> {
        if let Some(project) = &self.current_project {
            let marker = project.project_dir().join(".incomplete");
            std::fs::write(&marker, "")?;
            debug!("Marked project {} as incomplete", project.id);
        }
        Ok(())
    }

    /// Check if auto-save is currently in progress
    pub async fn is_saving(&self) -> bool {
        *self.is_saving.lock().await
    }

    /// Get time since last save
    pub fn time_since_last_save(&self) -> Option<Duration> {
        self.last_save
            .map(|t| SystemTime::now().duration_since(t).unwrap_or_default())
    }

    /// Run the auto-save loop
    /// This should be spawned as a background task
    pub async fn run_auto_save_loop(mut self) {
        if !self.config.enabled {
            return;
        }

        let mut ticker = interval(self.config.interval);

        loop {
            ticker.tick().await;

            if self.current_project.is_some() {
                if let Err(e) = self.save_now().await {
                    warn!("Auto-save failed: {}", e);
                }
            }
        }
    }
}

impl Default for AutoSaveService {
    fn default() -> Self {
        Self::new()
    }
}

/// Recovery service for handling incomplete projects after crashes
pub struct RecoveryService;

impl RecoveryService {
    /// Find all incomplete projects that need recovery
    pub fn find_incomplete_projects() -> FrameResult<Vec<PathBuf>> {
        let data_dir = directories::ProjectDirs::from("app", "frame", "Frame")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("frame"));

        let projects_dir = data_dir.join("projects");
        let mut incomplete = Vec::new();

        if !projects_dir.exists() {
            return Ok(incomplete);
        }

        for entry in std::fs::read_dir(projects_dir)? {
            let entry = entry?;
            let project_dir = entry.path();

            if project_dir.is_dir() {
                let incomplete_marker = project_dir.join(".incomplete");
                if incomplete_marker.exists() {
                    incomplete.push(project_dir);
                }
            }
        }

        Ok(incomplete)
    }

    /// Load an incomplete project for recovery
    pub fn load_incomplete_project(project_dir: &PathBuf) -> FrameResult<Option<Project>> {
        let project_file = project_dir.join("project.json");

        if !project_file.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(project_file)?;
        let project: Project = serde_json::from_str(&json)?;

        Ok(Some(project))
    }

    /// Mark a project as recovered
    pub fn mark_recovered(project_dir: &PathBuf) -> FrameResult<()> {
        let incomplete_marker = project_dir.join(".incomplete");
        if incomplete_marker.exists() {
            std::fs::remove_file(incomplete_marker)?;
            info!("Marked project as recovered: {:?}", project_dir);
        }
        Ok(())
    }

    /// Delete an incomplete project
    pub fn delete_incomplete_project(project_dir: &PathBuf) -> FrameResult<()> {
        std::fs::remove_dir_all(project_dir)?;
        info!("Deleted incomplete project: {:?}", project_dir);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_save_config_default() {
        let config = AutoSaveConfig::default();
        assert_eq!(config.interval, Duration::from_secs(10));
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_auto_save_service_new() {
        let service = AutoSaveService::new();
        assert!(service.is_enabled());
        assert!(service.current_project.is_none());
        assert!(!service.is_saving().await);
    }

    #[tokio::test]
    async fn test_auto_save_disabled() {
        let mut service = AutoSaveService::new();
        service.disable();
        assert!(!service.is_enabled());
        service.enable();
        assert!(service.is_enabled());
    }
}
