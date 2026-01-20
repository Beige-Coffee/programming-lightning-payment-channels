#![allow(dead_code, unused_imports, unused_variables, unused_must_use)]
use bitcoin::{Address, BlockHash, Txid};
use std::str::FromStr;
use bitcoin::secp256k1::PublicKey;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct SignedTx {
  pub complete: bool,
  pub hex: String,
}

#[derive(Debug, Deserialize)]
pub struct ListUnspentResponse(pub Vec<ListUnspentUtxo>);

#[derive(Debug, Clone, Deserialize)]
pub struct ListUnspentUtxo {
  #[serde(deserialize_with = "deserialize_txid")]
  pub txid: Txid,
  pub vout: u32,
  #[serde(deserialize_with = "deserialize_amount")]
  pub amount: u64,
  #[serde(deserialize_with = "deserialize_address")]
  pub address: Address,
}

fn deserialize_txid<'de, D>(deserializer: D) -> Result<Txid, D::Error>
where
  D: Deserializer<'de>,
{
  let s = String::deserialize(deserializer)?;
  Txid::from_str(&s).map_err(serde::de::Error::custom)
}

fn deserialize_amount<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
  D: Deserializer<'de>,
{
  let btc_amount = f64::deserialize(deserializer)?;
  bitcoin::Amount::from_btc(btc_amount)
    .map(|amt| amt.to_sat())
    .map_err(serde::de::Error::custom)
}

fn deserialize_address<'de, D>(deserializer: D) -> Result<Address, D::Error>
where
  D: Deserializer<'de>,
{
  let s = String::deserialize(deserializer)?;
  Address::from_str(&s)
    .map(|addr| addr.assume_checked())
    .map_err(serde::de::Error::custom)
}
