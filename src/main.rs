mod battery;
mod config;
mod threshold;
mod ui;

use anyhow::Result;
use battery::{list_batteries, read_battery, BatteryInfo};
use config::{load_config, save_config};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    time::{Duration, Instant},
};

#[derive(Debug, PartialEq)]
enum InputTarget {
    Start,
    End,
}

struct App {
    input_mode: bool,
    input_buf: String,
    input_target: Option<InputTarget>,
    status_msg: String,
    status_msg_until: Option<Instant>,
    batteries: Vec<String>,
    battery_idx: usize,
}

impl App {
    fn new() -> Self {
        let batteries = list_batteries();
        Self {
            input_mode: false,
            input_buf: String::new(),
            input_target: None,
            status_msg: String::new(),
            status_msg_until: None,
            battery_idx: 0,
            batteries,
        }
    }

    fn set_status(&mut self, msg: impl Into<String>, duration_secs: u64) {
        self.status_msg = msg.into();
        self.status_msg_until = Some(Instant::now() + Duration::from_secs(duration_secs));
    }

    fn clear_status_if_expired(&mut self) {
        if let Some(until) = self.status_msg_until {
            if Instant::now() >= until {
                self.status_msg.clear();
                self.status_msg_until = None;
            }
        }
    }

    fn input_label(&self, info: &BatteryInfo, cfg: &config::Config) -> String {
        match self.input_target {
            Some(InputTarget::Start) => {
                let cur = info
                    .charge_start
                    .unwrap_or(cfg.charge_start_threshold);
                format!("Set start threshold (current {cur}%) [0-99]")
            }
            Some(InputTarget::End) => {
                let cur = info.charge_end.unwrap_or(cfg.charge_end_threshold);
                format!("Set end threshold (current {cur}%) [1-100]")
            }
            None => String::new(),
        }
    }
}

fn main() -> Result<()> {
    let cfg = load_config();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, cfg);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut cfg: config::Config,
) -> Result<()> {
    let mut app = App::new();
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(cfg.refresh_interval_ms);

    let bat_path = &app.batteries[app.battery_idx];
    let mut info = read_battery(bat_path)?;

    loop {
        app.clear_status_if_expired();

        let bat_name = app.batteries[app.battery_idx]
            .rsplit('/')
            .next()
            .unwrap_or("?");

        terminal.draw(|f| {
            ui::draw(
                f,
                &info,
                &cfg,
                app.input_mode,
                &app.input_buf,
                &app.input_label(&info, &cfg),
                &app.status_msg,
                bat_name,
            )
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if app.input_mode {
                    match key.code {
                        KeyCode::Esc => {
                            app.input_mode = false;
                            app.input_buf.clear();
                            app.input_target = None;
                        }
                        KeyCode::Backspace => {
                            app.input_buf.pop();
                        }
                        KeyCode::Enter => {
                            let val: Result<u8, _> = app.input_buf.trim().parse();
                            match val {
                                Ok(v) => {
                                    let bat_path = app.batteries[app.battery_idx].clone();
                                    let result = match app.input_target {
                                        Some(InputTarget::Start) => {
                                            let end = info
                                                .charge_end
                                                .unwrap_or(cfg.charge_end_threshold);
                                            if v >= end {
                                                Err(anyhow::anyhow!(
                                                    "Start must be less than end ({end}%)"
                                                ))
                                            } else {
                                                threshold::set_start_threshold(&bat_path, v).map(
                                                    |_| {
                                                        cfg.charge_start_threshold = v;
                                                    },
                                                )
                                            }
                                        }
                                        Some(InputTarget::End) => {
                                            let start = info
                                                .charge_start
                                                .unwrap_or(cfg.charge_start_threshold);
                                            if v <= start {
                                                Err(anyhow::anyhow!(
                                                    "End must be greater than start ({start}%)"
                                                ))
                                            } else {
                                                threshold::set_end_threshold(&bat_path, v).map(
                                                    |_| {
                                                        cfg.charge_end_threshold = v;
                                                    },
                                                )
                                            }
                                        }
                                        None => Ok(()),
                                    };
                                    match result {
                                        Ok(_) => {
                                            let _ = save_config(&cfg);
                                            app.set_status(format!("Threshold set to {v}%"), 3);
                                            if let Ok(fresh) = read_battery(&bat_path) {
                                                info = fresh;
                                            }
                                        }
                                        Err(e) => {
                                            app.set_status(format!("Error: {e}"), 4);
                                        }
                                    }
                                }
                                Err(_) => app.set_status("Error: invalid number", 3),
                            }
                            app.input_mode = false;
                            app.input_buf.clear();
                            app.input_target = None;
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() => {
                            if app.input_buf.len() < 3 {
                                app.input_buf.push(c);
                            }
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break;
                        }
                        KeyCode::Char('s') => {
                            app.input_mode = true;
                            app.input_target = Some(InputTarget::Start);
                            app.input_buf.clear();
                        }
                        KeyCode::Char('e') => {
                            app.input_mode = true;
                            app.input_target = Some(InputTarget::End);
                            app.input_buf.clear();
                        }
                        KeyCode::Char('n') if app.batteries.len() > 1 => {
                            app.battery_idx =
                                (app.battery_idx + 1) % app.batteries.len();
                            let bat_path = &app.batteries[app.battery_idx];
                            if let Ok(fresh) = read_battery(bat_path) {
                                info = fresh;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            let bat_path = &app.batteries[app.battery_idx];
            if let Ok(fresh) = read_battery(bat_path) {
                info = fresh;
            }
            last_tick = Instant::now();
        }
    }

    Ok(())
}
