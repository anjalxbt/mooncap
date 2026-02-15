use chrono::Local;

use crate::api::PairData;

/// Maximum number of history points to keep for the sparkline
const MAX_HISTORY: usize = 60;

/// Maximum number of log messages to keep
const MAX_LOG: usize = 100;

/// Field labels for the config modal
pub const MODAL_FIELD_LABELS: [&str; 4] = ["Token / Pair Address", "Chain", "Target MCap ($)", "Interval (s)"];

#[allow(dead_code)]
pub struct App {
    // Config
    pub pair_address: String,
    pub chain: String,
    pub target_market_cap: f64,
    pub check_interval: u64,
    pub alarm_file: Option<String>,
    pub alarm_duration: u64,

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

    // Modal state
    pub modal_open: bool,
    pub modal_fields: [String; 4], // [pair, chain, target, interval]
    pub modal_active_field: usize,
    pub configured: bool,
}

impl App {
    /// Create app with modal open and mock values (no CLI args provided)
    pub fn new_interactive(alarm_file: Option<String>, alarm_duration: u64) -> Self {
        let mut app = Self {
            pair_address: String::new(),
            chain: String::from("solana"),
            target_market_cap: 100000.0,
            check_interval: 180,
            alarm_file,
            alarm_duration,

            token_name: String::from("MoonCap Demo"),
            token_symbol: String::from("MOON"),
            current_price: 0.00004200,
            market_cap: 42000.0,
            fdv: 42000.0,
            volume_24h: 6900.0,
            price_change_1h: 4.20,
            price_change_24h: 13.37,
            liquidity_usd: 8500.0,
            buys_24h: 420,
            sells_24h: 69,

            market_cap_history: vec![35000, 36500, 38000, 37200, 39000, 40500, 41000, 42000],
            log_messages: Vec::new(),
            last_fetch: None,
            target_hit: false,
            alarm_active: false,
            running: true,
            fetch_count: 0,
            error_count: 0,

            modal_open: true,
            modal_fields: [
                String::new(),                // pair
                String::from("solana"),        // chain
                String::from("100000"),        // target
                String::from("180"),           // interval
            ],
            modal_active_field: 0,
            configured: false,
        };

        let now = Local::now().format("%H:%M:%S").to_string();
        app.add_log(format!(
            "[{}] 🚀 MoonCap started — press Enter to configure",
            now
        ));

        app
    }

    /// Create app pre-configured from CLI arguments (existing behavior)
    pub fn new_with_config(
        pair_address: String,
        chain: String,
        target_market_cap: f64,
        check_interval: u64,
        alarm_file: Option<String>,
        alarm_duration: u64,
    ) -> Self {
        let mut app = Self {
            pair_address: pair_address.clone(),
            chain: chain.clone(),
            target_market_cap,
            check_interval,
            alarm_file,
            alarm_duration,

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

            modal_open: false,
            modal_fields: [
                pair_address,
                chain.clone(),
                format!("{}", target_market_cap as u64),
                format!("{}", check_interval),
            ],
            modal_active_field: 0,
            configured: true,
        };

        let now = Local::now().format("%H:%M:%S").to_string();
        app.add_log(format!(
            "[{}] 🚀 MoonCap started | Chain: {} | Target: ${:.0}",
            now, chain, target_market_cap
        ));
        app.add_log(format!(
            "[{}] 📡 Monitoring pair: {}",
            now, app.pair_address
        ));
        app.add_log(format!(
            "[{}] ⏱  Check interval: {}s",
            now, check_interval
        ));

        app
    }

    /// Apply the modal field values to the app config
    pub fn apply_modal_config(&mut self) {
        self.pair_address = self.modal_fields[0].trim().to_string();
        self.chain = if self.modal_fields[1].trim().is_empty() {
            String::from("solana")
        } else {
            self.modal_fields[1].trim().to_string()
        };
        self.target_market_cap = self.modal_fields[2]
            .trim()
            .parse::<f64>()
            .unwrap_or(100000.0);
        self.check_interval = self.modal_fields[3]
            .trim()
            .parse::<u64>()
            .unwrap_or(180);

        self.configured = true;
        self.modal_open = false;

        // Reset live data for the new pair
        self.token_name = String::from("Loading...");
        self.token_symbol = String::from("???");
        self.current_price = 0.0;
        self.market_cap = 0.0;
        self.fdv = 0.0;
        self.volume_24h = 0.0;
        self.price_change_1h = 0.0;
        self.price_change_24h = 0.0;
        self.liquidity_usd = 0.0;
        self.buys_24h = 0;
        self.sells_24h = 0;
        self.market_cap_history.clear();
        self.target_hit = false;
        self.alarm_active = false;
        self.fetch_count = 0;
        self.error_count = 0;

        let now = Local::now().format("%H:%M:%S").to_string();
        self.log_messages.clear();
        self.add_log(format!(
            "[{}] 🚀 Configured | Chain: {} | Target: ${:.0}",
            now, self.chain, self.target_market_cap
        ));
        self.add_log(format!(
            "[{}] 📡 Monitoring pair: {}",
            now, self.pair_address
        ));
        self.add_log(format!(
            "[{}] ⏱  Check interval: {}s",
            now, self.check_interval
        ));
    }

    /// Open the modal with current config values pre-filled
    pub fn open_modal(&mut self) {
        self.modal_fields = [
            self.pair_address.clone(),
            self.chain.clone(),
            format!("{}", self.target_market_cap as u64),
            format!("{}", self.check_interval),
        ];
        self.modal_active_field = 0;
        self.modal_open = true;
    }

    /// Navigate to next modal field
    pub fn modal_next_field(&mut self) {
        self.modal_active_field = (self.modal_active_field + 1) % 4;
    }

    /// Navigate to previous modal field
    pub fn modal_prev_field(&mut self) {
        self.modal_active_field = if self.modal_active_field == 0 {
            3
        } else {
            self.modal_active_field - 1
        };
    }

    /// Type a character into the active modal field
    pub fn modal_type_char(&mut self, c: char) {
        self.modal_fields[self.modal_active_field].push(c);
    }

    /// Delete last character from the active modal field
    pub fn modal_backspace(&mut self) {
        self.modal_fields[self.modal_active_field].pop();
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
