use serde_json::json;
use std::time::Duration;

pub struct MoneroClient {
    pub rpc_url: String,
}

impl MoneroClient {
    pub async fn create_subaddress(&self) -> Result<String, String> {
        let client = reqwest::Client::new();
        let res = client
            .post(&self.rpc_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": "0",
                "method": "create_address",
                "params": { "account_index": 0 }
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        Ok(json["result"]["address"]
            .as_str()
            .ok_or("No address in RPC response")?
            .to_string())
    }

    /// Primary price fetcher with failover logic
    pub async fn get_xmr_price_piconero(&self, usd_amount: f64) -> Result<u64, String> {
        match self.fetch_kraken_price().await {
            Ok(price) => {
                println!("💹 Price Feed: Kraken @ ${:.2}", price);
                return Ok(self.convert_to_piconero(usd_amount, price));
            }
            Err(e) => eprintln!("⚠️ Kraken Price Fail: {}", e),
        }

        match self.fetch_cryptocompare_price().await {
            Ok(price) => {
                println!("💹 Price Feed: CryptoCompare @ ${:.2}", price);
                return Ok(self.convert_to_piconero(usd_amount, price));
            }
            Err(e) => eprintln!("❌ CryptoCompare Price Fail: {}", e),
        }

        Err("Failed to fetch XMR price from all providers".into())
    }

    async fn fetch_kraken_price(&self) -> Result<f64, String> {
        let url = "https://api.kraken.com/0/public/Ticker?pair=XMRUSD";
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?;

        let res = client.get(url).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

        let price_str = json["result"]["XXMRZUSD"]["c"][0]
            .as_str()
            .ok_or("Invalid Kraken JSON structure")?;

        price_str.parse::<f64>().map_err(|e| e.to_string())
    }

    async fn fetch_cryptocompare_price(&self) -> Result<f64, String> {
        let url = "https://min-api.cryptocompare.com/data/price?fsym=XMR&tsyms=USD";
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?;

        let res = client.get(url).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

        json["USD"]
            .as_f64()
            .ok_or("USD field missing in CryptoCompare response".into())
    }

    fn convert_to_piconero(&self, usd_amount: f64, xmr_price: f64) -> u64 {
        let xmr_required = usd_amount / xmr_price;
        (xmr_required * 1_000_000_000_000.0) as u64
    }

    pub async fn check_payment(&self, address: String) -> Result<u64, String> {
        let client = reqwest::Client::new();
        let res = client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": "0",
                "method": "get_transfers",
                "params": {
                    "in": true,
                    "account_index": 0,
                    "pool": true
                }
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        let mut total_received = 0;

        if let Some(transfers) = json["result"]["in"].as_array() {
            for t in transfers {
                if t["address"] == address {
                    total_received += t["amount"].as_u64().unwrap_or(0);
                }
            }
        }

        if let Some(pool) = json["result"]["pool"].as_array() {
            for t in pool {
                if t["address"] == address {
                    total_received += t["amount"].as_u64().unwrap_or(0);
                }
            }
        }

        Ok(total_received)
    }

    pub async fn verify_payment_proof(
        &self,
        txid: String,
        tx_key: String,
        address: String,
    ) -> Result<(u64, u64), String> {
        let client = reqwest::Client::new();
        let res = client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": "0",
                "method": "check_tx_key",
                "params": {
                    "txid": txid,
                    "tx_key": tx_key,
                    "address": address
                }
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

        if let Some(result) = json.get("result") {
            let received = result["received"].as_u64().unwrap_or(0);
            let confirmations = result["confirmations"].as_u64().unwrap_or(0);
            println!(
                "🔍 Proof Verified: Tx {} sent {} piconero to {}",
                txid, received, address
            );
            Ok((received, confirmations))
        } else {
            Err("Invalid payment proof or transaction not found".to_string())
        }
    }
}
