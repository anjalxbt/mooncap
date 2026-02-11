use chrono::Local;

use crate::api::PairData;

/// Maximum number of history points to keep for the sparkline
const MAX_HISTORY: usize = 60;

/// Maximum number of log messages to keep
const MAX_LOG: usize = 100;

#[allow(dead_code)]
pub struct App {
    // Config
    pub pair_address: String,
    pub chain: String,
    pub target_market_cap: f64,
    pub check_interval: u64,
    pub alarm_file: Option<String>,

    // Live data
    pub token_name: String,
    pub token_symbol: String,
    pub current_price: f64,
    pub market_cap: f64,
    pub fdv: f64,
    pub volume_24h: f64,
    pub price_change_1h: f64,
    pub price_change_24h: f64,
    pub liquidity_usd: f64,
    pub buys_24h: u64,
    pub sells_24h: u64,

    // UI state
    pub market_cap_history: Vec<u64>,
    pub log_messages: Vec<String>,
    pub last_fetch: Option<String>,
    pub target_hit: bool,
    pub alarm_active: bool,
    pub running: bool,
    pub fetch_count: u64,
    pub error_count: u64,
}

impl App {
    pub fn new(
        pair_address: String,
        chain: String,
        target_market_cap: f64,
        check_interval: u64,
        alarm_file: Option<String>,
    ) -> Self {
        Self {
            pair_address,
            chain,
            target_market_cap,
            check_interval,
            alarm_file,

            token_name: String::from("Loading..."),
            token_symbol: String::from("???"),
            current_price: 0.0,
            market_cap: 0.0,
            fdv: 0.0,
            volume_24h: 0.0,
            price_change_1h: 0.0,
            price_change_24h: 0.0,
            liquidity_usd: 0.0,
            buys_24h: 0,
            sells_24h: 0,

            market_cap_history: Vec::new(),
            log_messages: Vec::new(),
            last_fetch: None,
            target_hit: false,
            alarm_active: false,
            running: true,
            fetch_count: 0,
            error_count: 0,
        }
    }

    pub fn update_from_pair_data(&mut self, data: &PairData) {
        if let Some(ref base) = data.base_token {
            if let Some(ref name) = base.name {
                self.token_name = name.clone();
            }
            if let Some(ref symbol) = base.symbol {
                self.token_symbol = symbol.clone();
            }
        }

        if let Some(ref price_str) = data.price_usd {
            self.current_price = price_str.parse().unwrap_or(0.0);
        }

        self.market_cap = data.market_cap.unwrap_or(data.fdv.unwrap_or(0.0));
        self.fdv = data.fdv.unwrap_or(0.0);

        if let Some(ref vol) = data.volume {
            self.volume_24h = vol.h24.unwrap_or(0.0);
        }

        if let Some(ref pc) = data.price_change {
            self.price_change_1h = pc.h1.unwrap_or(0.0);
            self.price_change_24h = pc.h24.unwrap_or(0.0);
        }

        if let Some(ref liq) = data.liquidity {
            self.liquidity_usd = liq.usd.unwrap_or(0.0);
        }

        if let Some(ref txns) = data.txns {
            if let Some(ref h24) = txns.h24 {
                self.buys_24h = h24.buys.unwrap_or(0);
                self.sells_24h = h24.sells.unwrap_or(0);
            }
        }

        // Track history for sparkline
        let mcap_u64 = self.market_cap as u64;
        self.market_cap_history.push(mcap_u64);
        if self.market_cap_history.len() > MAX_HISTORY {
            self.market_cap_history.remove(0);
        }

        self.fetch_count += 1;
        let now = Local::now().format("%H:%M:%S").to_string();
        self.last_fetch = Some(now.clone());

        let change_str = if self.price_change_1h >= 0.0 {
            format!("+{:.2}%", self.price_change_1h)
        } else {
            format!("{:.2}%", self.price_change_1h)
        };

        self.add_log(format!(
            "[{}] MCap: ${:.0} | Price: ${:.8} | 1h: {}",
            now, self.market_cap, self.current_price, change_str
        ));

        // Check target
        if self.market_cap >= self.target_market_cap && !self.target_hit {
            self.target_hit = true;
            self.alarm_active = true;
            self.add_log(format!(
                "[{}] 🔥 TARGET HIT! Market cap reached ${:.0} 🔥",
                now, self.market_cap
            ));
        }
    }

    pub fn add_log(&mut self, msg: String) {
        self.log_messages.push(msg);
        if self.log_messages.len() > MAX_LOG {
            self.log_messages.remove(0);
        }
    }

    pub fn add_error(&mut self, err: String) {
        self.error_count += 1;
        let now = Local::now().format("%H:%M:%S").to_string();
        self.add_log(format!("[{}] ❌ Error: {}", now, err));
    }

    pub fn progress(&self) -> f64 {
        if self.target_market_cap <= 0.0 {
            return 0.0;
        }
        (self.market_cap / self.target_market_cap * 100.0).min(100.0)
    }
}
