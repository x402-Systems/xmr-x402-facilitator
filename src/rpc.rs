//use crate::models::X402Requirement;
use serde_json::json;

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

    /// Fetches real XMR price from CoinGecko and converts USD to Piconero
    pub async fn get_xmr_price_piconero(&self, usd_amount: f64) -> Result<u64, String> {
        let url = "https://api.coingecko.com/api/v3/simple/price?ids=monero&vs_currencies=usd";
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("Client build error: {}", e))?;

        let res = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Price API returned status {}", res.status()));
        }

        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let xmr_price_usd = json["monero"]["usd"]
            .as_f64()
            .ok_or("Price data missing in response")?;

        let xmr_required = usd_amount / xmr_price_usd;
        Ok((xmr_required * 1_000_000_000_000.0) as u64)
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

        // check_tx_key returns "received" (amount) and "confirmations"
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
