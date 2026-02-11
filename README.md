# 🚀 MoonCap

A terminal-based crypto market cap monitor powered by [DexScreener](https://dexscreener.com/). Built with Rust and [Ratatui](https://ratatui.rs/) for a beautiful live TUI dashboard.

![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange)
![License](https://img.shields.io/badge/License-MIT-blue)

## Features

- 📈 **Live market cap sparkline** — watch the chart grow in your terminal
- 🎯 **Target alerts** — set a target market cap and get notified when it hits
- 📊 **Full stats panel** — price, FDV, volume, liquidity, buys/sells, price changes
- 🔔 **Alarm system** — terminal bell (default) or MP3/WAV audio via `--alarm`
- ⚡ **Configurable intervals** — check as often or rarely as you want
- 🌐 **Multi-chain** — works with any chain DexScreener supports (Solana, Ethereum, BSC, etc.)

## Install

### From source

```bash
git clone https://github.com/yourusername/mooncap.git
cd mooncap
cargo install --path .
```

### With audio alarm support

Requires `libasound2-dev` on Linux:

```bash
# Ubuntu/Debian
sudo apt install libasound2-dev

# Then build with audio feature
cargo install --path . --features audio
```

## Usage

```bash
# Monitor a Solana pair with default settings (180s interval, $100K target)
mooncap --pair HXY8iBHRvKvA3MMTwHkNa6SJSLYPfZSc59vX8dGbLExW

# Full options
mooncap \
  --pair HXY8iBHRvKvA3MMTwHkNa6SJSLYPfZSc59vX8dGbLExW \
  --chain solana \
  --target 100000 \
  --interval 60 \
  --alarm alarm.mp3 \
  --alarm-duration 300

# Monitor an Ethereum pair
mooncap --pair 0x1234...abcd --chain ethereum --target 1000000
```

### CLI Options

| Flag | Description | Default |
|------|-------------|---------|
| `-p, --pair` | DEX pair address **(required)** | — |
| `-c, --chain` | Blockchain chain | `solana` |
| `-t, --target` | Target market cap ($) | `100000` |
| `-i, --interval` | Check interval (seconds) | `180` |
| `-a, --alarm` | Path to alarm audio file | Terminal bell |
| `--alarm-duration` | Alarm duration (seconds) | `300` |

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `q` / `Esc` | Quit |
| `r` | Force refresh now |
| `s` | Stop alarm |

## Dashboard Layout

```
┌─────────────────────────────────────────────────┐
│  🚀 MOONCAP — TokenName ($SYMBOL)               │
├───────────────────────┬─────────────────────────┤
│                       │  Price:    $0.00003347   │
│   Market Cap Chart    │  MCap:     $33.5K        │
│   (Sparkline)         │  FDV:      $33.5K        │
│                       │  Volume:   $4.0K         │
│                       │  Liq:      $14.4K        │
│   🎯 Target Progress  │  Target:   $100.0K  🎯   │
│   ████████░░░░ 33.5%  │  24h:      +22.66%      │
├───────────────────────┴─────────────────────────┤
│  [17:12] MCap: $33476 | Price: $0.00003347      │
│  Press 'q' quit · 'r' refresh · 's' stop alarm  │
└─────────────────────────────────────────────────┘
```

## License

MIT
