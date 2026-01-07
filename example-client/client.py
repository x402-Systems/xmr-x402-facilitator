import requests
import time
import json

# Configuration
FACILITATOR_URL = "http://localhost:3113/content"
CUSTOMER_WALLET_RPC = "http://localhost:18084/json_rpc" # Your customer wallet port

def pay_xmr(address, amount_piconero):
    """Talks to the customer's wallet to fulfill the x402 request."""
    print(f"💸 Auto-paying {amount_piconero} piconero to {address}...")
    
    payload = {
        "jsonrpc": "2.0",
        "id": "0",
        "method": "transfer",
        "params": {
            "destinations": [{"amount": amount_piconero, "address": address}],
            "account_index": 0,
            "priority": 1,
        }
    }
    
    response = requests.post(CUSTOMER_WALLET_RPC, json=payload)
    result = response.json()
    
    if "result" in result:
        tx_hash = result["result"]["tx_hash"]
        print(f"✅ Transaction sent! Hash: {tx_hash}")
        return True
    else:
        print(f"❌ Payment failed: {result.get('error')}")
        return False

def fetch_resource():
    print(f"🚀 Attempting to fetch: {FACILITATOR_URL}")
    
    # 1. Initial Attempt
    resp = requests.get(FACILITATOR_URL)
    
    if resp.status_code == 402:
        print("⚠️ Received HTTP 402: Payment Required.")
        data = resp.json()
        
        # 2. Extract x402 Data
        address = data["address"]
        amount = data["amount_piconero"]
        
        # 3. Pay the Invoice
        if pay_xmr(address, amount):
            print("⏳ Waiting for transaction to propagate (10s)...")
            time.sleep(10) # Wait for mempool visibility
            
            # 4. Retry with the subaddress in the header
            print("🔄 Retrying request with payment proof...")
            headers = {"x-monero-address": address}
            final_resp = requests.get(FACILITATOR_URL, headers=headers)
            
            if final_resp.status_code == 200:
                print(f"\n🎉 SUCCESS: {final_resp.text}")
            else:
                print(f"❌ Still failed: {final_resp.status_code} - {final_resp.text}")
    
    elif resp.status_code == 200:
        print(f"✅ Resource already unlocked: {resp.text}")
    else:
        print(f"❓ Unexpected status: {resp.status_code}")

if __name__ == "__main__":
    fetch_resource()
