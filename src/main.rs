mod config;

use config::{load_config, Config};

fn main() {
    let cfg: Config = load_config();
    println!("bettery {} — Battery Monitor", env!("CARGO_PKG_VERSION"));
    println!(
        "Config: refresh={}ms  window={}%–{}%",
        cfg.refresh_interval_ms, cfg.charge_start_threshold, cfg.charge_end_threshold,
    );
}
