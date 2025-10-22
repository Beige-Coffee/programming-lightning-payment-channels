#![allow(dead_code, unused_imports, unused_variables, mismatched_lifetime_syntaxes, unknown_lints, unused_must_use)]
use bitcoin::hash_types::{BlockHash};
use bitcoin::{Network };
use lightning_block_sync::http::HttpEndpoint;
use lightning_block_sync::rpc::RpcClient;
use bitcoin::secp256k1::PublicKey as Secp256k1PublicKey;
use bitcoin::address::Address;
use lightning_block_sync::{AsyncBlockSourceResult, BlockData, BlockHeaderData, BlockSource};
use serde_json;
use std::str::FromStr;
use bitcoin::blockdata::transaction::Transaction;
use std::sync::Arc;
use bitcoin::consensus::{encode};
use crate::internal::convert::{
    ListUnspentResponse, SignedTx};
use lightning::chain::chaininterface::{BroadcasterInterface};

#[derive(Clone)]
pub struct BitcoindClient {
    pub bitcoind_rpc_client: Arc<RpcClient>,
    pub handle: tokio::runtime::Handle,
}

impl BitcoindClient {
    pub async fn new(
        host: String, port: u16, rpc_user: String, rpc_password: String, network: Network,
    ) -> std::io::Result<Self> {
        let http_endpoint = HttpEndpoint::for_host(host.clone()).with_port(port);
        let rpc_credentials =
            base64::encode(format!("{}:{}", rpc_user.clone(), rpc_password.clone()));
        let bitcoind_rpc_client = RpcClient::new(&rpc_credentials, http_endpoint)?;

        let client =Self {
            bitcoind_rpc_client: Arc::new(bitcoind_rpc_client),
            handle: tokio::runtime::Handle::current(),
        };

        Ok(client)
    }

    pub async fn list_unspent(&self) -> ListUnspentResponse {
        self.bitcoind_rpc_client
            .call_method::<ListUnspentResponse>("listunspent", &vec![])
            .await
            .unwrap()
    }

    pub async fn sign_raw_transaction_with_wallet(&self, tx_hex: String) -> SignedTx {
        let tx_hex_json = serde_json::json!(tx_hex);
        let signed_tx: SignedTx = self.bitcoind_rpc_client
            .call_method("signrawtransactionwithwallet", &vec![tx_hex_json])
            .await
            .unwrap();
        //println!("Signed Tx: {}", &signed_tx.hex);
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
