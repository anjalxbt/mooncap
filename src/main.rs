mod alarm;
mod api;
mod app;
mod ui;

use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Local;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;

use app::App;

/// 🚀 MoonCap — Monitor any crypto token's market cap from DexScreener
#[derive(Parser)]
#[command(name = "mooncap", version, about)]
struct Cli {
    /// The DEX pair address to monitor
    #[arg(short, long)]
    pair: String,

    /// Blockchain chain (e.g. solana, ethereum, bsc)
    #[arg(short, long, default_value = "solana")]
    chain: String,

    /// Target market cap to trigger alarm
    #[arg(short, long, default_value = "100000")]
    target: f64,

    /// Interval between API checks in seconds
    #[arg(short, long, default_value = "180")]
    interval: u64,

    /// Path to an alarm audio file (mp3/wav). Falls back to terminal bell if not set.
    #[arg(short, long)]
    alarm: Option<String>,

    /// Alarm duration in seconds once target is hit
    #[arg(long, default_value = "300")]
    alarm_duration: u64,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let mut app = App::new(
        cli.pair.clone(),
        cli.chain.clone(),
        cli.target,
        cli.interval,
        cli.alarm.clone(),
    );

    let now = Local::now().format("%H:%M:%S").to_string();
    app.add_log(format!(
        "[{}] 🚀 MoonCap started | Chain: {} | Target: ${:.0}",
        now, cli.chain, cli.target
    ));
    app.add_log(format!(
        "[{}] 📡 Monitoring pair: {}",
        now, cli.pair
    ));
    app.add_log(format!(
        "[{}] ⏱  Check interval: {}s",
        now, cli.interval
    ));

    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, &mut app, &cli).await;
    ratatui::restore();

    if let Err(e) = result {
        eprintln!("Application error: {}", e);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    cli: &Cli,
) -> io::Result<()> {
    let client = reqwest::Client::new();
    let mut last_fetch = Instant::now() - Duration::from_secs(cli.interval + 1);
    let mut alarm_handle: Option<Arc<AtomicBool>> = None;
    let tick_rate = Duration::from_millis(200);

    while app.running {
        // Draw
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Check if it's time to fetch
        if last_fetch.elapsed() >= Duration::from_secs(cli.interval) {
            last_fetch = Instant::now();

            match api::fetch_pair_data(&client, &cli.chain, &cli.pair).await {
                Ok(data) => {
                    app.update_from_pair_data(&data);

                    // Trigger alarm if target hit and no alarm running
                    if app.alarm_active && alarm_handle.is_none() {
                        let handle = alarm::start_alarm(
                            cli.alarm.as_deref(),
                            cli.alarm_duration,
                        );
                        alarm_handle = Some(handle);
                    }
                }
                Err(e) => {
                    app.add_error(e);
                }
            }
        }

        // Handle input (non-blocking with timeout)
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.running = false;
                            if let Some(ref handle) = alarm_handle {
                                alarm::stop_alarm(handle);
                            }
                        }
                        KeyCode::Char('r') => {
                            // Force immediate refresh
                            last_fetch =
                                Instant::now() - Duration::from_secs(cli.interval + 1);
                            app.add_log(format!(
                                "[{}] 🔄 Manual refresh triggered",
                                Local::now().format("%H:%M:%S")
                            ));
                        }
                        KeyCode::Char('s') => {
                            // Stop alarm
                            if let Some(ref handle) = alarm_handle {
                                alarm::stop_alarm(handle);
                                alarm_handle = None;
                                app.alarm_active = false;
                                app.add_log(format!(
                                    "[{}] 🔇 Alarm stopped manually",
                                    Local::now().format("%H:%M:%S")
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}
