use wasm_bindgen::prelude::*;

use solana_api_types::Client;

struct SolanaApiClient {}

#[wasm_bindgen]
pub async fn run() -> Result<JsValue, JsValue> {
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getEpochInfo"
    });
    let req = serde_json::to_vec(&req).unwrap();

    let r = reqwest::Client::new()
        .post("https://api.devnet.solana.com")
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .body(req)
        .send()
        .await
        .unwrap();

    let text = r.text().await.unwrap();
    Ok(JsValue::from_str(&text))
}
