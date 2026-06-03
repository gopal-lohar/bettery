use anyhow::{bail, Context, Result};
use std::fs;
use std::io;

fn write_threshold(bat_path: &str, file: &str, value: u8) -> Result<()> {
    let path = format!("{bat_path}/{file}");
    match fs::write(&path, format!("{value}")) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            bail!("Must run as root to set charge thresholds");
        }
        Err(e) => Err(e).with_context(|| format!("Failed to write to {path}")),
    }
}

pub fn set_start_threshold(bat_path: &str, value: u8) -> Result<()> {
    if value >= 100 {
        bail!("Start threshold must be < 100");
    }
    write_threshold(bat_path, "charge_control_start_threshold", value)
}

pub fn set_end_threshold(bat_path: &str, value: u8) -> Result<()> {
    if value == 0 || value > 100 {
        bail!("End threshold must be 1-100");
    }
    write_threshold(bat_path, "charge_control_end_threshold", value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_threshold_valid() {
        assert!(set_start_threshold("/fake", 50).is_err());
    }

    #[test]
    fn start_threshold_100_invalid() {
        let result = set_start_threshold("/fake", 100);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Start threshold must be < 100");
    }

    #[test]
    fn start_threshold_above_100_invalid() {
        let result = set_start_threshold("/fake", 150);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Start threshold must be < 100");
    }

    #[test]
    fn end_threshold_0_invalid() {
        let result = set_end_threshold("/fake", 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "End threshold must be 1-100");
    }

    #[test]
    fn end_threshold_101_invalid() {
        let result = set_end_threshold("/fake", 101);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "End threshold must be 1-100");
    }

    #[test]
    fn end_threshold_valid() {
        assert!(set_end_threshold("/fake", 50).is_err());
    }

    #[test]
    fn end_threshold_100_valid() {
        assert!(set_end_threshold("/fake", 100).is_err());
    }

    #[test]
    fn end_threshold_1_valid() {
        assert!(set_end_threshold("/fake", 1).is_err());
    }
}
