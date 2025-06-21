use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub audio: AudioConfig,
    pub download: DownloadConfig,
    pub ui: UiConfig,
    pub keybindings: KeyBindings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub default_volume: u8,
    pub audio_format: String,
    pub audio_quality: String,
    pub mpv_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    pub music_dir: String,
    pub playlist_dir: String,
    pub max_concurrent_downloads: usize,
    pub auto_create_playlist: bool,
    pub filename_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub default_playlist: String,
    pub show_progress: bool,
    pub show_waveform: bool,
    pub theme: String,
    pub update_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    pub play_pause: Vec<String>,
    pub next_track: Vec<String>,
    pub previous_track: Vec<String>,
    pub volume_up: Vec<String>,
    pub volume_down: Vec<String>,
    pub search: Vec<String>,
    pub quit: Vec<String>,
    pub help: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            audio: AudioConfig {
                default_volume: 70,
                audio_format: "mp3".to_string(),
                audio_quality: "0".to_string(),
                mpv_args: vec![
                    "--no-video".to_string(),
                    "--quiet".to_string(),
                    "--no-terminal".to_string(),
                    "--input-ipc-server=/tmp/mpvsocket".to_string(),
                ],
            },
            download: DownloadConfig {
                music_dir: "music".to_string(),
                playlist_dir: "playlists".to_string(),
                max_concurrent_downloads: 3,
                auto_create_playlist: true,
                filename_template: "%(title).100s.%(ext)s".to_string(),
            },
            ui: UiConfig {
                default_playlist: "default".to_string(),
                show_progress: true,
                show_waveform: false,
                theme: "default".to_string(),
                update_interval_ms: 100,
            },
            keybindings: KeyBindings {
                play_pause: vec![" ".to_string(), "p".to_string()],
                next_track: vec!["n".to_string(), "Right".to_string()],
                previous_track: vec!["b".to_string(), "Left".to_string()],
                volume_up: vec!["+".to_string(), "Up".to_string()],
                volume_down: vec!["-".to_string(), "Down".to_string()],
                search: vec!["s".to_string()],
                quit: vec!["q".to_string(), "Esc".to_string()],
                help: vec!["h".to_string(), "F1".to_string()],
            },
        }
    }
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        Self::load_from_file(&path).unwrap_or_else(|_| {
            let config = Self::default();
            // Try to save default config
            let _ = config.save_to_file(&path);
            config
        })
    }

    pub fn ensure_directories(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.download.music_dir)?;
        fs::create_dir_all(&self.download.playlist_dir)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.audio.default_volume > 100 {
            return Err("Default volume must be between 0 and 100".to_string());
        }

        if self.download.max_concurrent_downloads == 0 {
            return Err("Max concurrent downloads must be at least 1".to_string());
        }

        if self.ui.update_interval_ms < 10 {
            return Err("Update interval must be at least 10ms".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.audio.default_volume, 70);
        assert_eq!(config.download.music_dir, "music");
        assert!(config.validate().is_ok());
    }

        #[test]
    fn test_config_save_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = Config::default();

        // Save config to temp file
        config.save_to_file(temp_file.path()).unwrap();

        // Load config back from temp file
        let loaded = Config::load_from_file(temp_file.path()).unwrap();

        assert_eq!(loaded.audio.default_volume, config.audio.default_volume);
        assert_eq!(loaded.download.music_dir, config.download.music_dir);
        assert_eq!(loaded.keybindings.play_pause, config.keybindings.play_pause);
        assert!(loaded.validate().is_ok());
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        config.audio.default_volume = 150;
        assert!(config.validate().is_err());

        config.audio.default_volume = 50;
        config.download.max_concurrent_downloads = 0;
        assert!(config.validate().is_err());

        config.download.max_concurrent_downloads = 1;
        config.ui.update_interval_ms = 5;
        assert!(config.validate().is_err());

        config.ui.update_interval_ms = 100;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_load_or_default_creates_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        // Remove the file to test auto-creation
        std::fs::remove_file(&path).unwrap();

        let config = Config::load_or_default(&path);
        assert_eq!(config.audio.default_volume, 70);
        assert!(path.exists());
    }

    #[test]
    fn test_ensure_directories() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.download.music_dir = tmp_dir.path().join("music_test").to_string_lossy().to_string();
        config.download.playlist_dir = tmp_dir.path().join("playlists_test").to_string_lossy().to_string();

        assert!(config.ensure_directories().is_ok());
        assert!(Path::new(&config.download.music_dir).exists());
        assert!(Path::new(&config.download.playlist_dir).exists());
    }
}
