#![allow(dead_code, unused_imports, unused_variables, unknown_lints, unused_must_use)]
use bitcoin::{Network};
use serde_json;
use crate::internal::convert::{ListUnspentResponse, SignedTx};

#[derive(Clone)]
pub struct BitcoindClient {
    client: reqwest::Client,
    url: String,
    auth: String,
}

impl BitcoindClient {
    pub async fn new(
        host: String,
        port: u16,
        rpc_user: String,
        rpc_password: String,
        _network: Network,
    ) -> std::io::Result<Self> {
        let auth = base64::encode(format!("{}:{}", rpc_user, rpc_password));

        Ok(Self {
            client: reqwest::Client::new(),
            url: format!("http://{}:{}", host, port),
            auth,
        })
    }

    async fn call_method<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: &Vec<serde_json::Value>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(&self.url)
            .header("Authorization", format!("Basic {}", self.auth))
            .json(&serde_json::json!({
                "jsonrpc": "1.0",
                "id": "curios",
                "method": method,
                "params": params
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let result = response["result"].clone();
        Ok(serde_json::from_value(result)?)
    }

    pub async fn list_unspent(&self) -> ListUnspentResponse {
        self.call_method::<ListUnspentResponse>("listunspent", &vec![])
            .await
            .unwrap()
    }

    pub async fn sign_raw_transaction_with_wallet(&self, tx_hex: String) -> SignedTx {
        let tx_hex_json = serde_json::json!(tx_hex);
        let signed_tx: SignedTx = self
            .call_method("signrawtransactionwithwallet", &vec![tx_hex_json])
            .await
            .unwrap();
        signed_tx
    }
}

pub async fn get_bitcoind_client() -> BitcoindClient {
    let bitcoind = BitcoindClient::new(
        "0.0.0.0".to_string(),
        18443,
        "bitcoind".to_string(),
        "bitcoind".to_string(),
        Network::Regtest,
    )
    .await
    .unwrap();

    bitcoind
}
