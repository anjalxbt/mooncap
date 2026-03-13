use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, Instant};

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::api;

/// Daemon config saved alongside the PID file so the TUI can resume
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaemonConfig {
    pub pair: String,
    pub chain: String,
    pub target: f64,
    pub interval: u64,
    pub alarm: Option<String>,
    pub alarm_duration: u64,
}

/// Returns the pidfile path for a given pair address
pub fn pid_file(pair: &str) -> PathBuf {
    let safe = pair.chars().take(12).collect::<String>();
    PathBuf::from(format!("/tmp/mooncap-{}.pid", safe))
}

/// Returns the logfile path for a given pair address
pub fn log_file(pair: &str) -> PathBuf {
    let safe = pair.chars().take(12).collect::<String>();
    PathBuf::from(format!("/tmp/mooncap-{}.log", safe))
}

/// Returns the config file path for a given pair address
fn config_file(pair: &str) -> PathBuf {
    let safe = pair.chars().take(12).collect::<String>();
    PathBuf::from(format!("/tmp/mooncap-{}.json", safe))
}

/// Save daemon config to disk
fn save_config(pair: &str, config: &DaemonConfig) {
    let path = config_file(pair);
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = fs::write(path, json);
    }
}

/// Scan /tmp for any running mooncap daemons and return their configs
pub fn find_running_daemons() -> Vec<DaemonConfig> {
    let mut results = Vec::new();

    let entries = match fs::read_dir("/tmp") {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("mooncap-") && name.ends_with(".pid") {
            // Read PID and check if alive
            if let Ok(contents) = fs::read_to_string(entry.path()) {
                if let Ok(pid) = contents.trim().parse::<u32>() {
                    if process_is_alive(pid) {
                        // Find the matching config file
                        let stem = name.trim_end_matches(".pid");
                        let cfg_path = PathBuf::from(format!("/tmp/{}.json", stem));
                        if let Ok(json) = fs::read_to_string(&cfg_path) {
                            if let Ok(config) = serde_json::from_str::<DaemonConfig>(&json) {
                                results.push(config);
                            }
                        }
                    }
                }
            }
        }
    }

    results
}

/// Kill a running daemon by pair address and clean up its files
pub fn kill_daemon(pair: &str) -> Result<(), String> {
    let pid_path = pid_file(pair);
    let cfg_path = config_file(pair);

    if !pid_path.exists() {
        return Err(format!(
            "No daemon PID file found at {:?}. Is a daemon running?",
            pid_path
        ));
    }

    let contents = fs::read_to_string(&pid_path)
        .map_err(|e| format!("Failed to read PID file: {}", e))?;
    let pid: u32 = contents
        .trim()
        .parse()
        .map_err(|_| "Invalid PID in file".to_string())?;

    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }

    let _ = fs::remove_file(&pid_path);
    let _ = fs::remove_file(&cfg_path);
    Ok(())
}

/// Kill daemon silently (no prints, used when TUI resumes)
pub fn kill_daemon_quiet(pair: &str) {
    let _ = kill_daemon(pair);
}

/// Spawn a background daemon worker. Relaunches the binary with --daemon-worker.
/// Returns the PID of the spawned process.
pub fn spawn_daemon(
    pair: &str,
    chain: &str,
    target: f64,
    interval: u64,
    alarm: Option<&str>,
    alarm_duration: u64,
) -> Result<u32, String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?;

    let pid_path = pid_file(pair);
    let log_path = log_file(pair);

    // Check if daemon is already running
    if pid_path.exists() {
        if let Ok(contents) = fs::read_to_string(&pid_path) {
            if let Ok(pid) = contents.trim().parse::<u32>() {
                if process_is_alive(pid) {
                    return Err(format!("Daemon already running with PID {}", pid));
                }
            }
        }
        let _ = fs::remove_file(&pid_path);
    }

    // Save config so the TUI can resume later
    let config = DaemonConfig {
        pair: pair.to_string(),
        chain: chain.to_string(),
        target,
        interval,
        alarm: alarm.map(|s| s.to_string()),
        alarm_duration,
    };
    save_config(pair, &config);

    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Failed to open log file: {}", e))?;

    let log_err = log.try_clone().map_err(|e| e.to_string())?;

    let mut cmd = process::Command::new(&exe);
    cmd.arg("--daemon-worker")
        .arg("--pair").arg(pair)
        .arg("--chain").arg(chain)
        .arg("--target").arg(target.to_string())
        .arg("--interval").arg(interval.to_string())
        .arg("--alarm-duration").arg(alarm_duration.to_string());

    if let Some(a) = alarm {
        cmd.arg("--alarm").arg(a);
    }

    cmd.stdout(log)
        .stderr(log_err)
        .stdin(process::Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    let child = cmd.spawn().map_err(|e| format!("Failed to spawn daemon: {}", e))?;
    let pid = child.id();

    fs::write(&pid_path, pid.to_string())
        .map_err(|e| format!("Failed to write PID file: {}", e))?;

    Ok(pid)
}

/// Check if a process is alive
fn process_is_alive(pid: u32) -> bool {
    PathBuf::from(format!("/proc/{}", pid)).exists()
}

/// The headless background worker loop
pub async fn run_daemon_worker(
    pair: String,
    chain: String,
    target: f64,
    interval: u64,
    alarm_file: Option<String>,
    alarm_duration: u64,
) {
    let pid = process::id();
    let pid_path = pid_file(&pair);
    let log_path = log_file(&pair);

    let _ = fs::write(&pid_path, pid.to_string());

    // Also write config in case it wasn't written by the parent
    let config = DaemonConfig {
        pair: pair.clone(),
        chain: chain.clone(),
        target,
        interval,
        alarm: alarm_file.clone(),
        alarm_duration,
    };
    save_config(&pair, &config);

    let log = |msg: &str| {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let line = format!("[{}] {}\n", now, msg);
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map(|mut f| {
                use std::io::Write;
                let _ = f.write_all(line.as_bytes());
            });
        print!("{}", line);
    };

    log(&format!(
        "🚀 MoonCap daemon started | PID: {} | Chain: {} | Target: ${:.0} | Interval: {}s",
        pid, chain, target, interval
    ));
    log(&format!("📡 Monitoring: {}", pair));

    let client = reqwest::Client::new();
    let mut last_fetch = Instant::now() - Duration::from_secs(interval + 1);

    loop {
        if last_fetch.elapsed() >= Duration::from_secs(interval) {
            last_fetch = Instant::now();

            match api::fetch_pair_data(&client, &chain, &pair).await {
                Ok(data) => {
                    let market_cap = data.market_cap.unwrap_or(data.fdv.unwrap_or(0.0));
                    let price = data
                        .price_usd
                        .as_deref()
                        .unwrap_or("0")
                        .parse::<f64>()
                        .unwrap_or(0.0);
                    let name = data
                        .base_token
                        .as_ref()
                        .and_then(|t| t.name.as_deref())
                        .unwrap_or("Token");
                    let symbol = data
                        .base_token
                        .as_ref()
                        .and_then(|t| t.symbol.as_deref())
                        .unwrap_or("???");

                    log(&format!(
                        "✓ {} ({}) | MCap: ${:.0} | Price: ${:.8} | Target: ${:.0}",
                        name, symbol, market_cap, price, target
                    ));

                    if market_cap >= target {
                        log(&format!(
                            "🔥 TARGET HIT! {} reached ${:.0}",
                            name, market_cap
                        ));

                        fire_alarm(name, symbol, market_cap, target, alarm_file.as_deref(), alarm_duration);

                        let _ = fs::remove_file(&pid_path);
                        let _ = fs::remove_file(config_file(&pair));
                        log("Daemon exiting after alarm.");
                        return;
                    }
                }
                Err(e) => {
                    log(&format!("❌ Fetch error: {}", e));
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Trigger desktop notification + audio alarm
fn fire_alarm(
    name: &str,
    symbol: &str,
    market_cap: f64,
    target: f64,
    alarm_file: Option<&str>,
    alarm_duration: u64,
) {
    let summary = format!("🚀 MoonCap — {} hit target!", symbol);
    let body = format!(
        "{} ({}) market cap reached ${:.0}\nTarget was ${:.0}",
        name, symbol, market_cap, target
    );

    let _ = process::Command::new("notify-send")
        .args(["--urgency=critical", "--expire-time=0", &summary, &body])
        .spawn();

    let end = Instant::now() + Duration::from_secs(alarm_duration);

    if let Some(file) = alarm_file {
        while Instant::now() < end {
            let status = if file.ends_with(".mp3") {
                process::Command::new("mpg123")
                    .args(["-q", file])
                    .status()
            } else {
                process::Command::new("paplay")
                    .arg(file)
                    .status()
            };

            if status.is_err() || Instant::now() >= end {
                break;
            }
        }
    } else {
        while Instant::now() < end {
            print!("\x07");
            std::thread::sleep(Duration::from_secs(2));
        }
    }
}
