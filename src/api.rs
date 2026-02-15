use serde::Deserialize;

#[allow(dead_code)]
const BASE_URL: &str = "https://api.dexscreener.com/latest/dex";

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DexResponse {
    pub pairs: Option<Vec<PairData>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PairData {
    pub chain_id: Option<String>,
    pub dex_id: Option<String>,
    pub pair_address: Option<String>,
    pub base_token: Option<Token>,
    pub quote_token: Option<Token>,
    pub price_native: Option<String>,
    pub price_usd: Option<String>,
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,
    pub txns: Option<Txns>,
    pub volume: Option<Volume>,
    pub price_change: Option<PriceChange>,
    pub liquidity: Option<Liquidity>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct Token {
    pub address: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct Txns {
    pub m5: Option<TxnCount>,
    pub h1: Option<TxnCount>,
    pub h6: Option<TxnCount>,
    pub h24: Option<TxnCount>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TxnCount {
    pub buys: Option<u64>,
    pub sells: Option<u64>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct Volume {
    pub h24: Option<f64>,
    pub h6: Option<f64>,
    pub h1: Option<f64>,
    pub m5: Option<f64>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct PriceChange {
    pub h1: Option<f64>,
    pub h6: Option<f64>,
    pub h24: Option<f64>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct Liquidity {
    pub usd: Option<f64>,
    pub base: Option<f64>,
    pub quote: Option<f64>,
}

pub async fn fetch_pair_data(
    client: &reqwest::Client,
    chain: &str,
    address: &str,
) -> Result<PairData, String> {
    // Try the /tokens/ endpoint first (works with contract addresses)
    let token_url = format!("{}/tokens/{}", BASE_URL, address);
    if let Ok(result) = try_fetch(client, &token_url).await {
        return Ok(result);
    }

    // Fall back to /pairs/{chain}/{address} (works with pair addresses)
    let pair_url = format!("{}/pairs/{}/{}", BASE_URL, chain, address);
    try_fetch(client, &pair_url).await
}

async fn try_fetch(client: &reqwest::Client, url: &str) -> Result<PairData, String> {
    let response = client
        .get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()));
    }

    let data: DexResponse = response
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    data.pairs
        .and_then(|pairs| pairs.into_iter().next())
        .ok_or_else(|| "No pair data found in response".to_string())
}
