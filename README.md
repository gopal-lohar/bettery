# Bettery

A minimal TUI battery manager for Linux.

## Features

- Live battery stats: capacity, health, voltage, energy, power draw, cycle count, temperature
- Set charge start/end thresholds (requires root)
- Color-coded bars: green/yellow/red based on battery level and health
- Config file at `~/.config/bettery/config.toml`

## Build

```bash
cargo build --release
```

## Run

```bash
./target/release/bettery
```

Setting thresholds requires root:

```bash
sudo ./target/release/bettery
```

## Keybindings

| Key | Action |
|-----|--------|
| `s` | Set start threshold |
| `e` | Set end threshold |
| `n` | Switch to next battery |
| `q` | Quit |
| `Ctrl+C` | Quit |
| `Esc` | Cancel input |
