import requests
import time
import json

# Configuration
FACILITATOR_URL = "http://localhost:3113/content"
# Your customer wallet port (Spender)
CUSTOMER_WALLET_RPC = "http://localhost:18084/json_rpc" 

def pay_xmr(address, amount_piconero):
    """
    Talks to the customer's wallet to fulfill the x402 request.
    Requests a 'tx_key' which acts as a private proof of payment.
    """
    print(f"💸 Auto-paying {amount_piconero} piconero to {address}...")
    
    payload = {
        "jsonrpc": "2.0",
        "id": "0",
        "method": "transfer",
        "params": {
            "destinations": [{"amount": amount_piconero, "address": address}],
            "account_index": 0,
            "priority": 1,
            "get_tx_key": True
        }
    }
    
    try:
        response = requests.post(CUSTOMER_WALLET_RPC, json=payload)
        result = response.json()
        
        if "result" in result:
            tx_hash = result["result"]["tx_hash"]
            tx_key = result["result"]["tx_key"]
            print(f"✅ Transaction sent!")
            print(f"🔗 Hash: {tx_hash}")
            print(f"🔑 Proof Key: {tx_key}")
            return tx_hash, tx_key
        else:
            error_msg = result.get('error', {}).get('message', 'Unknown RPC error')
            print(f"❌ Payment failed: {error_msg}")
            return None, None
            
    except Exception as e:
        print(f"❌ Connection error to wallet: {e}")
        return None, None

def fetch_resource():
    print(f"🚀 Attempting to fetch protected resource: {FACILITATOR_URL}")
    
    # Initial Attempt (Will trigger 402)
    resp = requests.get(FACILITATOR_URL)
    
    if resp.status_code == 402:
        print("⚠️ Received HTTP 402: Payment Required.")
        try:
            data = resp.json()
            address = data["address"]
            amount = data["amount_piconero"]
        except Exception:
            print("❌ Failed to parse x402 requirement from server.")
            return
        
        # Pay the Invoice and get the cryptographic proof (tx_key)
        txid, txkey = pay_xmr(address, amount)
        
        if txid and txkey:
            # Short sleep to ensure the node mempool has seen the TX
            print("⏳ Waiting for transaction propagation (15s)...")
            time.sleep(15) 
            
            # 3. Retry with ALL components: Address + TX ID + TX Key
            print("🔄 Retrying request with cryptographic proof headers...")
            headers = {
                "x-monero-address": address,
                "x-monero-tx-id": txid,
                "x-monero-tx-key": txkey
            }
            
            final_resp = requests.get(FACILITATOR_URL, headers=headers)
            
            if final_resp.status_code == 200:
                print(f"\n🎉 SUCCESS! Resource Unlocked:")
                print("-" * 30)
                print(final_resp.text)
                print("-" * 30)
            else:
                print(f"❌ Access Denied: {final_resp.status_code}")
                print(f"Response: {final_resp.text}")
    
    elif resp.status_code == 200:
        print(f"✅ Resource already unlocked: {resp.text}")
    else:
        print(f"❓ Unexpected status: {resp.status_code}")

if __name__ == "__main__":
    fetch_resource()
