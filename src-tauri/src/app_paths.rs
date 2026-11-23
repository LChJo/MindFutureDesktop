use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const APP_DIR_NAME: &str = ".MoontonClaudeDesktop";

#[derive(Debug, Serialize, Deserialize)]
pub struct AppPaths {
    /// Root application data directory (~/.MoontonClaudeDesktop)
    pub app_data: String,
    /// Configuration directory
    pub config: String,
    /// Logs directory
    pub logs: String,
    /// Cache directory
    pub cache: String,
    /// User workspace directory (default terminal path)
    pub workspace: String,
}

impl AppPaths {
    pub fn new() -> Result<Self, String> {
        let home = dirs::home_dir()
            .ok_or_else(|| "Failed to get home directory".to_string())?;

        let app_data = home.join(APP_DIR_NAME);
        let config = app_data.join("config");
        let logs = app_data.join("logs");
        let cache = app_data.join("cache");
        let workspace = app_data.join("workspace");

        Ok(Self {
            app_data: app_data.to_string_lossy().to_string(),
            config: config.to_string_lossy().to_string(),
            logs: logs.to_string_lossy().to_string(),
            cache: cache.to_string_lossy().to_string(),
            workspace: workspace.to_string_lossy().to_string(),
        })
    }

    /// Ensure all directories exist
    pub fn ensure_directories(&self) -> Result<(), String> {
        let dirs = [
            &self.app_data,
            &self.config,
            &self.logs,
            &self.cache,
            &self.workspace,
        ];

        for dir in dirs {
            fs::create_dir_all(dir)
                .map_err(|e| format!("Failed to create directory {}: {}", dir, e))?;
        }

        Ok(())
    }
}

impl Default for AppPaths {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            app_data: String::new(),
            config: String::new(),
            logs: String::new(),
            cache: String::new(),
            workspace: String::new(),
        })
    }
}

pub fn get_app_paths() -> Result<AppPaths, String> {
    let paths = AppPaths::new()?;
    paths.ensure_directories()?;
    Ok(paths)
}

#[allow(dead_code)]
pub fn get_config_file_path(filename: &str) -> Result<PathBuf, String> {
    let paths = AppPaths::new()?;
    Ok(PathBuf::from(&paths.config).join(filename))
}

#[allow(dead_code)]
pub fn get_log_file_path(filename: &str) -> Result<PathBuf, String> {
    let paths = AppPaths::new()?;
    Ok(PathBuf::from(&paths.logs).join(filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_paths_new_succeeds() {
        // Should succeed on any machine with a home directory
        let paths = AppPaths::new();
        assert!(paths.is_ok());
    }

    #[test]
    fn app_paths_contain_app_dir_name() {
        let paths = AppPaths::new().unwrap();
        assert!(paths.app_data.contains(".MoontonClaudeDesktop"));
        assert!(paths.config.contains(".MoontonClaudeDesktop"));
        assert!(paths.logs.contains(".MoontonClaudeDesktop"));
        assert!(paths.cache.contains(".MoontonClaudeDesktop"));
        assert!(paths.workspace.contains(".MoontonClaudeDesktop"));
    }

    #[test]
    fn app_paths_subdirectories_correct() {
        let paths = AppPaths::new().unwrap();
        assert!(paths.config.ends_with("config"));
        assert!(paths.logs.ends_with("logs"));
        assert!(paths.cache.ends_with("cache"));
        assert!(paths.workspace.ends_with("workspace"));
    }

    #[test]
    fn app_paths_default_non_empty() {
        let paths = AppPaths::default();
        // On a normal system, default should produce non-empty paths
        assert!(!paths.app_data.is_empty());
    }

    #[test]
    fn get_config_file_path_appends_filename() {
        let path = get_config_file_path("settings.json").unwrap();
        assert!(path.to_string_lossy().contains("config"));
        assert!(path.to_string_lossy().ends_with("settings.json"));
    }

    #[test]
    fn get_log_file_path_appends_filename() {
        let path = get_log_file_path("app.log").unwrap();
        assert!(path.to_string_lossy().contains("logs"));
        assert!(path.to_string_lossy().ends_with("app.log"));
    }

    #[test]
    fn app_paths_serialization_roundtrip() {
        let paths = AppPaths::new().unwrap();
        let json = serde_json::to_string(&paths).unwrap();
        let parsed: AppPaths = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.app_data, paths.app_data);
        assert_eq!(parsed.config, paths.config);
        assert_eq!(parsed.logs, paths.logs);
        assert_eq!(parsed.cache, paths.cache);
        assert_eq!(parsed.workspace, paths.workspace);
    }
}
