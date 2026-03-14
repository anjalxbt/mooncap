mod alarm;
mod api;
mod app;
mod daemon;
// remove this to avoid animation
mod splash;
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
#[command(name = "mooncap", version, about, long_about = None)]
struct Cli {
    /// The token/pair address to monitor (opens config modal if omitted)
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

    /// Run in background daemon mode (no TUI, survives terminal close).
    /// Sends a desktop notification when the target is hit.
    #[arg(short, long)]
    daemon: bool,

    /// Stop a running daemon for the given --pair address
    #[arg(long)]
    stop: bool,

    /// Internal flag: marks this process as the daemon worker (hidden)
    #[arg(long, hide = true)]
    daemon_worker: bool,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // --stop: kill a running daemon
    if cli.stop {
        let pair = cli.pair.as_deref().unwrap_or("");
        if pair.is_empty() {
            eprintln!("Error: --stop requires --pair <ADDRESS>");
            std::process::exit(1);
        }
        if let Err(e) = daemon::kill_daemon(pair) {
            eprintln!("{}", e);
            std::process::exit(1);
        }
        return Ok(());
    }

    // --daemon-worker: internal headless worker
    if cli.daemon_worker {
        let pair = cli.pair.unwrap_or_default();
        if pair.is_empty() {
            eprintln!("Error: --daemon-worker requires --pair");
            std::process::exit(1);
        }
        daemon::run_daemon_worker(
            pair,
            cli.chain,
            cli.target,
            cli.interval,
            cli.alarm,
            cli.alarm_duration,
        )
        .await;
        return Ok(());
    }

    // --daemon: spawn background process and exit
    if cli.daemon {
        let pair = cli.pair.as_deref().unwrap_or("");
        if pair.is_empty() {
            eprintln!("Error: --daemon requires --pair <ADDRESS>");
            std::process::exit(1);
        }
        match daemon::spawn_daemon(
            pair,
            &cli.chain,
            cli.target,
            cli.interval,
            cli.alarm.as_deref(),
            cli.alarm_duration,
        ) {
            Ok(pid) => {
                let log_path = daemon::log_file(pair);
                println!("🌙 MoonCap daemon started in background");
                println!("   PID:    {}", pid);
                println!("   Target: ${:.0}", cli.target);
                println!("   Log:    {}", log_path.display());
                println!();
                println!("   Stop with: mooncap --stop --pair {}", pair);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // Normal TUI mode
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
        // Check for a running daemon to resume from
        let daemons = daemon::find_running_daemons();
        if let Some(cfg) = daemons.into_iter().next() {
            // Kill the daemon and take over in TUI mode
            daemon::kill_daemon_quiet(&cfg.pair);
            App::new_with_config(
                cfg.pair,
                cfg.chain,
                cfg.target,
                cfg.interval,
                cfg.alarm.or(cli.alarm.clone()),
                cfg.alarm_duration,
            )
        } else {
            App::new_interactive(cli.alarm.clone(), cli.alarm_duration)
        }
    };

    let mut terminal = ratatui::init();

    // Play startup animation
    // remove this to avoid animation
    splash::run_splash(&mut terminal);

    let result = run_app(&mut terminal, &mut app).await;
    ratatui::restore();

    // If the user chose to go idle from the TUI, spawn a daemon
    if app.go_idle {
        match daemon::spawn_daemon(
            &app.pair_address,
            &app.chain,
            app.target_market_cap,
            app.check_interval,
            app.alarm_file.as_deref(),
            app.alarm_duration,
        ) {
            Ok(pid) => {
                let log_path = daemon::log_file(&app.pair_address);
                println!("🌙 MoonCap now running in background (idle mode)");
                println!("   PID:    {}", pid);
                println!("   Target: ${:.0}", app.target_market_cap);
                println!("   Log:    {}", log_path.display());
                println!();
                println!(
                    "   Stop with: mooncap --stop --pair {}",
                    app.pair_address
                );
            }
            Err(e) => {
                eprintln!("Error starting idle mode: {}", e);
            }
        }
    }

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
        KeyCode::Char('d') => {
            // Go idle — spawn daemon and exit TUI
            if app.configured && !app.pair_address.is_empty() {
                app.go_idle = true;
                app.running = false;
                if let Some(ref handle) = alarm_handle {
                    alarm::stop_alarm(handle);
                }
            }
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
