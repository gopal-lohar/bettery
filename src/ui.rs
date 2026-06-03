use crate::battery::{energy_wh, power_w, voltage_v, BatteryInfo};
use crate::config::Config;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

const BAR: &str = "━";

fn capacity_color(pct: u8) -> Color {
    match pct {
        0..=15 => Color::Red,
        16..=30 => Color::LightRed,
        31..=60 => Color::Yellow,
        _ => Color::Green,
    }
}

fn health_color(pct: f64) -> Color {
    if pct >= 80.0 {
        Color::Green
    } else if pct >= 60.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn bar_line(pct: u16, width: usize, color: Color) -> Line<'static> {
    let fill = (pct as usize * width / 100).min(width);
    let empty = width - fill;
    let mut v = Vec::with_capacity(2);
    if fill > 0 {
        v.push(Span::styled(BAR.repeat(fill), Style::new().fg(color)));
    }
    if empty > 0 {
        v.push(Span::styled(
            BAR.repeat(empty),
            Style::new().fg(Color::DarkGray),
        ));
    }
    Line::from(v)
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    f: &mut Frame,
    info: &BatteryInfo,
    cfg: &Config,
    input_mode: bool,
    input_buf: &str,
    input_label: &str,
    status_msg: &str,
    battery_name: &str,
) {
    let size = f.size();
    let bw = 48;
    let col_w = 60;

    let power_str = info
        .power_now
        .map(|pw| {
            let label = if info.status == "Charging" {
                "Charge"
            } else {
                "Draw"
            };
            format!("{label:<12}{:.2} W", power_w(pw))
        })
        .unwrap_or_default();
    let cycle_str = info
        .cycle_count
        .map(|c| format!("{:<12}{}", "Cycles", c))
        .unwrap_or_default();
    let temp_str = info
        .temp
        .map(|t| format!("{:<12}{:.1} °C", "Temp", t))
        .unwrap_or_default();
    let time_label = match info.status.as_str() {
        "Charging" => "Until full",
        "Discharging" => "Remaining",
        _ => "Time",
    };
    let time_str = info
        .time_remaining
        .map(|m| format!("{time_label:<12}{}h {}m", m / 60, m % 60))
        .unwrap_or_default();

    let info_n = 2
        + if power_str.is_empty() { 0 } else { 1 }
        + if cycle_str.is_empty() { 0 } else { 1 }
        + if temp_str.is_empty() { 0 } else { 1 }
        + if time_str.is_empty() { 0 } else { 1 };

    let content_lines = 1 + 1 + 6 + 1 + info_n + 1 + 1;
    let top = size.height.saturating_sub(content_lines) / 2;

    let center = Layout::vertical([
        Constraint::Length(top),
        Constraint::Length(content_lines),
        Constraint::Fill(1),
    ])
    .split(size)[1];

    let col = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(col_w),
        Constraint::Fill(1),
    ])
    .split(center)[1];

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(info_n),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(col);

    let s = |c: Color| Style::default().fg(c);

    let sc = match info.status.as_str() {
        "Charging" => Color::Green,
        "Discharging" => Color::Yellow,
        "Full" => Color::Cyan,
        _ => Color::Gray,
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(battery_name, Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled(
                &info.status,
                Style::default()
                    .fg(sc)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{}%", info.capacity),
                Style::default()
                    .fg(capacity_color(info.capacity))
                    .add_modifier(Modifier::BOLD),
            ),
        ])),
        rows[0],
    );

    let cap_c = capacity_color(info.capacity);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{}%", info.capacity),
                s(cap_c).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Capacity", s(Color::DarkGray)),
        ])),
        rows[2],
    );
    f.render_widget(
        Paragraph::new(bar_line(info.capacity as u16, bw, cap_c)),
        rows[3],
    );

    let hl_c = health_color(info.health_pct);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{:.1}%", info.health_pct),
                s(hl_c).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Health", s(Color::DarkGray)),
        ])),
        rows[4],
    );
    let hpct = info.health_pct.min(100.0) as u16;
    f.render_widget(
        Paragraph::new(bar_line(hpct, bw, hl_c)),
        rows[5],
    );

    let start = info
        .charge_start
        .unwrap_or(cfg.charge_start_threshold);
    let end = info.charge_end.unwrap_or(cfg.charge_end_threshold);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{start}% – {end}%"),
                s(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Window", s(Color::DarkGray)),
        ])),
        rows[6],
    );
    f.render_widget(
        Paragraph::new(bar_line(end as u16, bw, Color::Cyan)),
        rows[7],
    );

    let mut infos: Vec<Line> = Vec::new();
    infos.push(Line::from(format!(
        "{:<12}{:.2} / {:.2} Wh  (Design {:.2} Wh)",
        "Energy",
        energy_wh(info.energy_now),
        energy_wh(info.energy_full),
        energy_wh(info.energy_full_design)
    )));
    infos.push(Line::from(format!(
        "{:<12}{:.3} V",
        "Voltage",
        voltage_v(info.voltage_now)
    )));
    if !power_str.is_empty() {
        infos.push(Line::from(power_str));
    }
    if !cycle_str.is_empty() {
        infos.push(Line::from(cycle_str));
    }
    if !temp_str.is_empty() {
        infos.push(Line::from(temp_str));
    }
    if !time_str.is_empty() {
        infos.push(Line::from(time_str));
    }
    f.render_widget(
        Paragraph::new(infos).style(s(Color::DarkGray)),
        rows[9],
    );

    let (status_text, status_style) = if input_mode {
        (format!("{input_label}: {input_buf}_"), s(Color::Yellow))
    } else if !status_msg.is_empty() {
        let col = if status_msg.starts_with("Error") {
            Color::Red
        } else {
            Color::DarkGray
        };
        (status_msg.to_string(), s(col))
    } else {
        (
            "s  start    e  end    n  next    q  quit".to_string(),
            s(Color::DarkGray),
        )
    };
    f.render_widget(
        Paragraph::new(status_text).style(status_style),
        rows[11],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_color_red() {
        assert_eq!(capacity_color(0), Color::Red);
        assert_eq!(capacity_color(15), Color::Red);
    }

    #[test]
    fn capacity_color_light_red() {
        assert_eq!(capacity_color(16), Color::LightRed);
        assert_eq!(capacity_color(30), Color::LightRed);
    }

    #[test]
    fn capacity_color_yellow() {
        assert_eq!(capacity_color(31), Color::Yellow);
        assert_eq!(capacity_color(60), Color::Yellow);
    }

    #[test]
    fn capacity_color_green() {
        assert_eq!(capacity_color(61), Color::Green);
        assert_eq!(capacity_color(100), Color::Green);
    }

    #[test]
    fn health_color_green() {
        assert_eq!(health_color(80.0), Color::Green);
        assert_eq!(health_color(100.0), Color::Green);
    }

    #[test]
    fn health_color_yellow() {
        assert_eq!(health_color(60.0), Color::Yellow);
        assert_eq!(health_color(79.99), Color::Yellow);
    }

    #[test]
    fn health_color_red() {
        assert_eq!(health_color(0.0), Color::Red);
        assert_eq!(health_color(59.99), Color::Red);
    }

    #[test]
    fn bar_full() {
        let line = bar_line(100, 10, Color::Green);
        let spans: Vec<&str> = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(spans, ["━".repeat(10)]);
    }

    #[test]
    fn bar_empty() {
        let line = bar_line(0, 10, Color::Green);
        let spans: Vec<&str> = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(spans, ["━".repeat(10)]);
    }

    #[test]
    fn bar_half() {
        let line = bar_line(50, 10, Color::Green);
        let spans: Vec<&str> = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0], "━".repeat(5));
        assert_eq!(spans[1], "━".repeat(5));
    }

    #[test]
    fn bar_clamped() {
        let line = bar_line(200, 10, Color::Green);
        let spans: Vec<&str> = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(spans, ["━".repeat(10)]);
    }
}
