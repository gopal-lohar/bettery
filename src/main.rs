mod battery;
mod config;
mod threshold;

use battery::{energy_wh, list_batteries, read_battery, voltage_v};
use config::{load_config, Config};

fn main() {
    let cfg: Config = load_config();
    println!("bettery {} — Battery Monitor", env!("CARGO_PKG_VERSION"));
    println!(
        "Config: refresh={}ms  window={}%–{}%",
        cfg.refresh_interval_ms, cfg.charge_start_threshold, cfg.charge_end_threshold,
    );

    let bats = list_batteries();
    println!("\nFound {} batter(ies):", bats.len());

    for (i, bat_path) in bats.iter().enumerate() {
        match read_battery(bat_path) {
            Ok(info) => {
                let name = bat_path.rsplit('/').next().unwrap_or("?");
                println!("\n── Battery {i}: {name} ──");
                println!("  Status:     {}", info.status);
                println!("  Capacity:   {}%", info.capacity);
                println!("  Health:     {:.1}%", info.health_pct);
                println!(
                    "  Energy:     {:.2} / {:.2} Wh (design {:.2} Wh)",
                    energy_wh(info.energy_now),
                    energy_wh(info.energy_full),
                    energy_wh(info.energy_full_design),
                );
                println!("  Voltage:    {:.3} V", voltage_v(info.voltage_now));
                if let Some(pw) = info.power_now {
                    println!("  Power:      {:.2} W", pw as f64 / 1_000_000.0);
                }
                if let Some(cc) = info.cycle_count {
                    println!("  Cycles:     {cc}");
                }
                if let Some(t) = info.temp {
                    println!("  Temp:       {t:.1} °C");
                }
                if let Some(tr) = info.time_remaining {
                    println!("  Remaining:  {}h {}m", tr / 60, tr % 60);
                }
                if let Some(s) = info.charge_start {
                    println!("  Window:     {s}% – {}%", info.charge_end.unwrap_or(0));
                }
            }
            Err(e) => println!("  [{bat_path}] Error: {e}"),
        }
    }

    println!("\nThreshold module loaded. Use 's' and 'e' keys in TUI to set charge thresholds.");
}
