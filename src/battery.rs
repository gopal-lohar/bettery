use anyhow::{Context, Result};
use std::{fs, path::Path};

fn read_sysfs(bat_path: &str, file: &str) -> Result<String> {
    let path = format!("{bat_path}/{file}");
    Ok(fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {path}"))?
        .trim()
        .to_string())
}

fn read_u64(bat_path: &str, file: &str) -> Result<u64> {
    read_sysfs(bat_path, file)?
        .parse::<u64>()
        .with_context(|| format!("Parse error: {file}"))
}

fn read_u8(bat_path: &str, file: &str) -> Result<u8> {
    read_sysfs(bat_path, file)?
        .parse::<u8>()
        .with_context(|| format!("Parse error: {file}"))
}

fn try_read_u64(bat_path: &str, file: &str) -> Option<u64> {
    read_u64(bat_path, file).ok()
}

pub fn list_batteries() -> Vec<String> {
    let dir = Path::new("/sys/class/power_supply");
    let mut bats: Vec<String> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with("BAT") {
                    Some(format!("/sys/class/power_supply/{name}"))
                } else {
                    None
                }
            })
            .collect(),
        Err(_) => vec!["/sys/class/power_supply/BAT0".into()],
    };
    bats.sort();
    if bats.is_empty() {
        bats.push("/sys/class/power_supply/BAT0".into());
    }
    bats
}

fn charge_to_energy_uv(charge_uah: u64, voltage_uv: u64) -> u64 {
    (charge_uah as u128 * voltage_uv as u128 / 1_000_000) as u64
}

#[derive(Debug, Clone)]
pub struct BatteryInfo {
    pub capacity: u8,
    pub status: String,
    pub energy_now: u64,
    pub energy_full: u64,
    pub energy_full_design: u64,
    pub voltage_now: u64,
    pub cycle_count: Option<u32>,
    pub charge_start: Option<u8>,
    pub charge_end: Option<u8>,
    pub health_pct: f64,
    pub power_now: Option<u64>,
    pub temp: Option<f64>,
    pub time_remaining: Option<u64>,
}

pub fn read_battery(bat_path: &str) -> Result<BatteryInfo> {
    let voltage_now = read_u64(bat_path, "voltage_now")?;

    let (energy_full, energy_full_design) = if let (Ok(ef), Ok(efd)) = (
        read_u64(bat_path, "energy_full"),
        read_u64(bat_path, "energy_full_design"),
    ) {
        (ef, efd)
    } else {
        let charge_full = read_u64(bat_path, "charge_full")?;
        let charge_full_design = read_u64(bat_path, "charge_full_design")?;
        (
            charge_to_energy_uv(charge_full, voltage_now),
            charge_to_energy_uv(charge_full_design, voltage_now),
        )
    };

    let health_pct = compute_health_pct(energy_full, energy_full_design);

    let energy_now = read_u64(bat_path, "energy_now").unwrap_or_else(|_| {
        let charge_now =
            read_u64(bat_path, "charge_now").expect("neither energy_now nor charge_now available");
        charge_to_energy_uv(charge_now, voltage_now)
    });

    let power_now = try_read_u64(bat_path, "power_now").or_else(|| {
        let current_now = try_read_u64(bat_path, "current_now")?;
        Some((current_now as u128 * voltage_now as u128 / 1_000_000) as u64)
    });

    let temp = try_read_u64(bat_path, "temp").map(|t| t as f64 / 10.0);

    let status = read_sysfs(bat_path, "status")?;
    let time_remaining = compute_time_remaining(&status, energy_now, energy_full, power_now);

    Ok(BatteryInfo {
        capacity: read_u8(bat_path, "capacity")?,
        status,
        energy_now,
        energy_full,
        energy_full_design,
        voltage_now,
        cycle_count: read_u64(bat_path, "cycle_count").ok().map(|v| v as u32),
        charge_start: read_u8(bat_path, "charge_control_start_threshold").ok(),
        charge_end: read_u8(bat_path, "charge_control_end_threshold").ok(),
        health_pct,
        power_now,
        temp,
        time_remaining,
    })
}

pub fn energy_wh(uwh: u64) -> f64 {
    uwh as f64 / 1_000_000.0
}

pub fn voltage_v(uv: u64) -> f64 {
    uv as f64 / 1_000_000.0
}

#[allow(dead_code)]
pub fn power_w(uw: u64) -> f64 {
    uw as f64 / 1_000_000.0
}

fn compute_health_pct(energy_full: u64, energy_full_design: u64) -> f64 {
    if energy_full_design == 0 {
        return 100.0;
    }
    (energy_full as f64 / energy_full_design as f64) * 100.0
}

fn compute_time_remaining(
    status: &str,
    energy_now: u64,
    energy_full: u64,
    power_now: Option<u64>,
) -> Option<u64> {
    let pw = power_now.filter(|&p| p > 0)?;
    let hrs = match status {
        "Charging" if energy_now < energy_full => {
            Some((energy_full - energy_now) as f64 / pw as f64)
        }
        "Discharging" => Some(energy_now as f64 / pw as f64),
        _ => None,
    }?;
    let mins = (hrs * 60.0).round() as u64;
    if mins < 1 {
        None
    } else {
        Some(mins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_to_energy_exact() {
        // 1 Ah at 1 V = 1 Wh = 1_000_000 µWh
        assert_eq!(charge_to_energy_uv(1_000_000, 1_000_000), 1_000_000);
    }

    #[test]
    fn charge_to_energy_zero_voltage() {
        assert_eq!(charge_to_energy_uv(5_000_000, 0), 0);
    }

    #[test]
    fn energy_wh_conversion() {
        assert!((energy_wh(1_000_000) - 1.0).abs() < f64::EPSILON);
        assert!((energy_wh(500_000) - 0.5).abs() < f64::EPSILON);
        assert!((energy_wh(0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn voltage_v_conversion() {
        assert!((voltage_v(11_520_000) - 11.52).abs() < f64::EPSILON);
        assert!((voltage_v(0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn power_w_conversion() {
        assert!((power_w(8_500_000) - 8.5).abs() < f64::EPSILON);
    }

    #[test]
    fn health_pct_perfect() {
        let hp = compute_health_pct(50_000_000, 50_000_000);
        assert!((hp - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn health_pct_degraded() {
        let hp = compute_health_pct(40_000_000, 50_000_000);
        assert!((hp - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn health_pct_zero_design() {
        let hp = compute_health_pct(50_000_000, 0);
        assert!((hp - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn health_pct_over_100() {
        let hp = compute_health_pct(60_000_000, 50_000_000);
        assert!((hp - 120.0).abs() < f64::EPSILON);
    }

    #[test]
    fn time_charging() {
        let t = compute_time_remaining("Charging", 40_000_000, 50_000_000, Some(10_000_000));
        // 10 Wh left at 10 W = 1 hour = 60 min
        assert_eq!(t, Some(60));
    }

    #[test]
    fn time_discharging() {
        let t = compute_time_remaining("Discharging", 30_000_000, 50_000_000, Some(15_000_000));
        // 30 Wh at 15 W = 2 hours = 120 min
        assert_eq!(t, Some(120));
    }

    #[test]
    fn time_full_battery() {
        let t = compute_time_remaining("Full", 50_000_000, 50_000_000, Some(10_000_000));
        assert_eq!(t, None);
    }

    #[test]
    fn time_no_power() {
        let t = compute_time_remaining("Discharging", 30_000_000, 50_000_000, None);
        assert_eq!(t, None);
    }

    #[test]
    fn time_zero_power() {
        let t = compute_time_remaining("Discharging", 30_000_000, 50_000_000, Some(0));
        assert_eq!(t, None);
    }

    #[test]
    fn time_less_than_minute() {
        let t = compute_time_remaining("Discharging", 1_000, 50_000_000, Some(1_000_000));
        assert_eq!(t, None);
    }
}
