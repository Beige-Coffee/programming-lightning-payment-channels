#![allow(dead_code, unused_imports, unused_variables, unknown_lints, unused_must_use)]
use bitcoin::Network;
use serde_json;
use crate::internal::convert::{ListUnspentResponse, SignedTx};

#[derive(Clone)]
pub struct BitcoindClient {
    url: String,
    auth: String,
}

impl BitcoindClient {
    pub fn new(
        host: String,
        port: u16,
        rpc_user: String,
        rpc_password: String,
        _network: Network,
    ) -> std::io::Result<Self> {
        let auth = base64::encode(format!("{}:{}", rpc_user, rpc_password));

        Ok(Self {
            url: format!("http://{}:{}", host, port),
            auth,
        })
    }

    fn call_method<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: &Vec<serde_json::Value>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let request_body = serde_json::json!({
            "jsonrpc": "1.0",
            "id": "curios",
            "method": method,
            "params": params
        });

        let response: serde_json::Value = ureq::post(&self.url)
            .set("Authorization", &format!("Basic {}", self.auth))
            .send_json(&request_body)?
            .into_json()?;

        let result = response["result"].clone();
        Ok(serde_json::from_value(result)?)
    }

    pub fn list_unspent(&self) -> ListUnspentResponse {
        self.call_method::<ListUnspentResponse>("listunspent", &vec![])
            .unwrap()
    }

    pub fn sign_raw_transaction_with_wallet(&self, tx_hex: String) -> SignedTx {
        let tx_hex_json = serde_json::json!(tx_hex);
        let signed_tx: SignedTx = self
            .call_method("signrawtransactionwithwallet", &vec![tx_hex_json])
            .unwrap();
        signed_tx
    }
}

pub fn get_bitcoind_client() -> BitcoindClient {
    BitcoindClient::new(
        "0.0.0.0".to_string(),
        18443,
        "bitcoind".to_string(),
        "bitcoind".to_string(),
        Network::Regtest,
    )
    .unwrap()
}
