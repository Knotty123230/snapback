use std::{
    env,
    fs::{create_dir_all, File},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub backup_default_path: Option<String>,
    pub backup_info_default_path: Option<String>,
    pub max_backup_count: Option<u32>,
    pub compress_backups: Option<bool>,
    pub exclude_patterns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backup_default_path: None,
            backup_info_default_path: None,
            max_backup_count: Some(100),
            compress_backups: Some(false),
            exclude_patterns: vec![
                "target/".to_string(),
                "node_modules/".to_string(),
                ".git/".to_string(),
                "*.tmp".to_string(),
                "*.log".to_string(),
            ],
        }
    }
}

impl Config {
    /// Load configuration from multiple sources with priority:
    /// 1. Environment variables
    /// 2. Config file in user's config directory
    /// 3. Config file in current directory  
    /// 4. Default values
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Self::default();

        // Try to load from config file
        if let Ok(file_config) = Self::load_from_file() {
            config = Self::merge_configs(config, file_config);
        }

        // Override with environment variables
        config = Self::load_from_env(config);

        Ok(config)
    }

    /// Legacy method for backwards compatibility
    pub fn read_config() -> Self {
        Self::load().unwrap_or_default()
    }

    fn load_from_file() -> anyhow::Result<Self> {
        let config_paths = [
            Self::get_user_config_path(),
            Self::get_local_config_path(),
        ];

        for config_path in &config_paths {
            if config_path.exists() {
                let content = std::fs::read_to_string(config_path)?;
                let config: Config = serde_json::from_str(&content)?;
                return Ok(config);
            }
        }

        Self::create_default_config()?;
        Ok(Self::default())
    }

    fn load_from_env(mut config: Config) -> Self {
        // Override with environment variables if present
        if let Ok(backup_path) = env::var("SNAPBACK_BACKUP_PATH") {
            config.backup_default_path = Some(backup_path);
        }

        if let Ok(info_path) = env::var("SNAPBACK_INFO_PATH") {
            config.backup_info_default_path = Some(info_path);
        }

        if let Ok(max_count) = env::var("SNAPBACK_MAX_BACKUPS") {
            if let Ok(count) = max_count.parse::<u32>() {
                config.max_backup_count = Some(count);
            }
        }

        if let Ok(compress) = env::var("SNAPBACK_COMPRESS") {
            config.compress_backups = Some(compress.to_lowercase() == "true");
        }

        config
    }

    fn merge_configs(mut base: Config, override_config: Config) -> Config {
        if override_config.backup_default_path.is_some() {
            base.backup_default_path = override_config.backup_default_path;
        }
        if override_config.backup_info_default_path.is_some() {
            base.backup_info_default_path = override_config.backup_info_default_path;
        }
        if override_config.max_backup_count.is_some() {
            base.max_backup_count = override_config.max_backup_count;
        }
        if override_config.compress_backups.is_some() {
            base.compress_backups = override_config.compress_backups;
        }
        if !override_config.exclude_patterns.is_empty() {
            base.exclude_patterns = override_config.exclude_patterns;
        }
        base
    }

    fn create_default_config() -> anyhow::Result<()> {
        let config_path = Self::get_user_config_path();
        
        if let Some(parent) = config_path.parent() {
            create_dir_all(parent)?;
        }

        let default_config = Self::default();
        let json = serde_json::to_string_pretty(&default_config)?;
        
        let mut file = File::create(&config_path)?;
        file.write_all(json.as_bytes())?;
        
        println!("Created default config at: {}", config_path.display());
        Ok(())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::get_user_config_path();
        
        if let Some(parent) = config_path.parent() {
            create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, json)?;
        
        println!("Config saved to: {}", config_path.display());
        Ok(())
    }

    fn get_user_config_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("snapback").join("config.json")
        } else {
            // Fallback for systems without standard config dir
            Self::get_home_config_path()
        }
    }

    fn get_home_config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".snapback")
            .join("config.json")
    }

    fn get_local_config_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("snapback.json")
    }

    // Getters with smart defaults
    pub fn get_default_backup_path(&self) -> String {
        if let Some(ref path) = self.backup_default_path {
            if !path.is_empty() {
                return path.clone();
            }
        }

        // Smart default based on OS
        self.get_default_data_dir()
            .join("snapback")
            .join("backups")
            .to_string_lossy()
            .to_string()
    }

    pub fn get_default_backup_info_path(&self) -> String {
        if let Some(ref path) = self.backup_info_default_path {
            if !path.is_empty() {
                return path.clone();
            }
        }

        // Smart default based on OS  
        self.get_default_data_dir()
            .join("snapback")
            .join("backup_info")
            .to_string_lossy()
            .to_string()
    }

    fn get_default_data_dir(&self) -> PathBuf {
        // Try platform-specific data directory
        if let Some(data_dir) = dirs::data_dir() {
            data_dir
        } else if let Some(home_dir) = dirs::home_dir() {
            home_dir.join(".local").join("share")
        } else {
            // Fallback to current directory
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    }

    pub fn get_max_backup_count(&self) -> u32 {
        self.max_backup_count.unwrap_or(100)
    }

    pub fn is_compress_enabled(&self) -> bool {
        self.compress_backups.unwrap_or(false)
    }

    pub fn get_exclude_patterns(&self) -> &[String] {
        &self.exclude_patterns
    }

    pub fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        for pattern in &self.exclude_patterns {
            if pattern.ends_with('/') {
                // Directory pattern
                let dir_pattern = pattern.trim_end_matches('/');
                if path_str.contains(dir_pattern) {
                    return true;
                }
            } else if pattern.starts_with("*.") {
                // Extension pattern
                let ext = pattern.trim_start_matches("*.");
                if let Some(file_ext) = path.extension() {
                    if file_ext == ext {
                        return true;
                    }
                }
            } else if path_str.contains(pattern) {
                // General pattern
                return true;
            }
        }
        
        false
    }

    pub fn print_config(&self) {
        println!("SnapBack Configuration:");
        println!("  Backup Path: {}", self.get_default_backup_path());
        println!("  Info Path: {}", self.get_default_backup_info_path());
        println!("  Max Backups: {}", self.get_max_backup_count());
        println!("  Compression: {}", self.is_compress_enabled());
        println!("  Exclude Patterns: {:?}", self.exclude_patterns);
        println!("  Config File: {}", Self::get_user_config_path().display());
    }
}
