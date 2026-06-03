use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub refresh_interval_ms: u64,
    pub charge_start_threshold: u8,
    pub charge_end_threshold: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            refresh_interval_ms: 2000,
            charge_start_threshold: 20,
            charge_end_threshold: 80,
        }
    }
}

pub fn config_path() -> PathBuf {
    let mut path = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else {
        let mut home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
        home.push(".config");
        home
    };
    path.push("bettery/config.toml");
    path
}

pub fn load_config() -> Config {
    let path = config_path();
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    } else {
        let cfg = Config::default();
        let _ = save_config(&cfg);
        cfg
    }
}

pub fn save_config(cfg: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(cfg)?;
    fs::write(&path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let cfg = Config::default();
        assert_eq!(cfg.refresh_interval_ms, 2000);
        assert_eq!(cfg.charge_start_threshold, 20);
        assert_eq!(cfg.charge_end_threshold, 80);
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = Config {
            refresh_interval_ms: 5000,
            charge_start_threshold: 30,
            charge_end_threshold: 90,
        };
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.refresh_interval_ms, 5000);
        assert_eq!(deserialized.charge_start_threshold, 30);
        assert_eq!(deserialized.charge_end_threshold, 90);
    }

    #[test]
    fn partial_config_uses_defaults() {
        let toml_str = r#"refresh_interval_ms = 3000"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.refresh_interval_ms, 3000);
        assert_eq!(cfg.charge_start_threshold, 20);
        assert_eq!(cfg.charge_end_threshold, 80);
    }

    #[test]
    fn invalid_toml_falls_back() {
        let toml_str = r#"<<<garbage>>>"#;
        let cfg: Config = toml::from_str(toml_str).unwrap_or_default();
        assert_eq!(cfg.refresh_interval_ms, 2000);
    }
}
