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
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

use app::App;

/// 🚀 MoonCap — Monitor any crypto token's market cap from DexScreener
#[derive(Parser)]
#[command(name = "mooncap", version, about)]
struct Cli {
    /// The DEX pair address to monitor (opens config modal if omitted)
    #[arg(short, long)]
    pair: Option<String>,

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

    let mut app = if let Some(ref pair) = cli.pair {
        App::new_with_config(
            pair.clone(),
            cli.chain.clone(),
            cli.target,
            cli.interval,
            cli.alarm.clone(),
            cli.alarm_duration,
        )
    } else {
        App::new_interactive(cli.alarm.clone(), cli.alarm_duration)
    };

    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, &mut app).await;
    ratatui::restore();

    if let Err(e) = result {
        eprintln!("Application error: {}", e);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
) -> io::Result<()> {
    let client = reqwest::Client::new();
    let mut last_fetch = Instant::now();
    let mut needs_immediate_fetch = app.configured; // fetch immediately if pre-configured
    let mut alarm_handle: Option<Arc<AtomicBool>> = None;
    let tick_rate = Duration::from_millis(200);

    while app.running {
        // Draw
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Only fetch data when configured and not in modal
        if app.configured
            && !app.modal_open
            && (needs_immediate_fetch
                || last_fetch.elapsed() >= Duration::from_secs(app.check_interval))
        {
            needs_immediate_fetch = false;
            last_fetch = Instant::now();

            match api::fetch_pair_data(&client, &app.chain, &app.pair_address).await {
                Ok(data) => {
                    app.update_from_pair_data(&data);

                    // Trigger alarm if target hit and no alarm running
                    if app.alarm_active && alarm_handle.is_none() {
                        let handle = alarm::start_alarm(
                            app.alarm_file.as_deref(),
                            app.alarm_duration,
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
                    if app.modal_open {
                        handle_modal_input(app, key.code, key.modifiers, &mut needs_immediate_fetch);
                    } else {
                        handle_normal_input(
                            app,
                            key.code,
                            &mut needs_immediate_fetch,
                            &mut alarm_handle,
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_modal_input(
    app: &mut App,
    key: KeyCode,
    modifiers: KeyModifiers,
    needs_immediate_fetch: &mut bool,
) {
    match key {
        KeyCode::Enter => {
            // Only submit if pair address is not empty
            if !app.modal_fields[0].trim().is_empty() {
                app.apply_modal_config();
                *needs_immediate_fetch = true;
            }
        }
        KeyCode::Esc => {
            if app.configured {
                // Already monitoring — just close the modal
                app.modal_open = false;
            } else {
                // First launch, not configured yet — quit the app
                app.running = false;
            }
        }
        KeyCode::Tab => {
            if modifiers.contains(KeyModifiers::SHIFT) {
                app.modal_prev_field();
            } else {
                app.modal_next_field();
            }
        }
        KeyCode::Down => {
            app.modal_next_field();
        }
        KeyCode::Up => {
            app.modal_prev_field();
        }
        KeyCode::Backspace => {
            app.modal_backspace();
        }
        KeyCode::Char(c) => {
            app.modal_type_char(c);
        }
        _ => {}
    }
}

fn handle_normal_input(
    app: &mut App,
    key: KeyCode,
    needs_immediate_fetch: &mut bool,
    alarm_handle: &mut Option<Arc<AtomicBool>>,
) {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.running = false;
            if let Some(ref handle) = alarm_handle {
                alarm::stop_alarm(handle);
            }
        }
        KeyCode::Char('r') => {
            *needs_immediate_fetch = true;
            app.add_log(format!(
                "[{}] 🔄 Manual refresh triggered",
                Local::now().format("%H:%M:%S")
            ));
        }
        KeyCode::Char('c') => {
            app.open_modal();
        }
        KeyCode::Char('s') => {
            if let Some(ref handle) = alarm_handle {
                alarm::stop_alarm(handle);
                app.alarm_active = false;
                app.add_log(format!(
                    "[{}] 🔇 Alarm stopped manually",
                    Local::now().format("%H:%M:%S")
                ));
            }
            *alarm_handle = None;
        }
        _ => {}
    }
}
