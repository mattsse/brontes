use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    hash::Hash,
    io::{BufWriter, Write},
    path::Path,
    str::FromStr
};

use clickhouse::{Client, Row};
use ethers_core::types::{Address, Chain, H160};
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use strum::Display;

const TOKEN_MAPPING_FILE: &str = "token_mapping.rs";
const TOKEN_QUERIES: &str = "SELECT toString(address),decimals FROM tokens";

fn main() {
    dotenv::dotenv().ok();
    println!("cargo:rerun-if-env-changed=RUN_BUILD_SCRIPT");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async move {
        let path = Path::new(&env::var("OUT_DIR").unwrap()).join(TOKEN_MAPPING_FILE);
        let mut file = BufWriter::new(File::create(&path).unwrap());
        build_token_details_map(&mut file).await;
        build_asset_map(&mut file);
    });
}

#[derive(Debug, Serialize, Deserialize, Clone, Row)]
pub struct TokenDetails {
    address:  String,
    decimals: u8
}

async fn build_token_details_map(file: &mut BufWriter<File>) {
    let mut phf_map: phf_codegen::Map<[u8; 20]> = phf_codegen::Map::new();
    #[cfg(feature = "server")]
    {
        let client = build_db();
        let rows = query_db::<TokenDetails>(&client, TOKEN_QUERIES).await;

        for row in rows {
            phf_map
                .entry(H160::from_str(&row.address).unwrap().0, row.decimals.to_string().as_str());
        }
    }

    writeln!(
        file,
        "pub static TOKEN_TO_DECIMALS: phf::Map<[u8; 20], u8> = \n{};\n",
        phf_map.build()
    )
    .unwrap();
}

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize, Display,
)]
pub enum Blockchain {
    /// to represent an all query
    Optimism,
    Ethereum,
    Bsc,
    Gnosis,
    Polygon,
    Fantom,
    Klaytn,
    Arbitrum,
    Avalanche,
    Aurora
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenList {
    pub tokens: Vec<Token>
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Token {
    pub chain_addresses: HashMap<Blockchain, Vec<Address>>,
    /// e.g USDC, USDT, ETH, BTC
    pub global_id:       String
}

impl Hash for Token {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.global_id.hash(state)
    }
}

fn build_asset_map(file: &mut BufWriter<File>) {
    let tokens: TokenList = serde_json::from_str(
        &fs::read_to_string("../../ticker_address_mapping/assets.json").unwrap()
    )
    .unwrap();

    let mut phf_map = phf_codegen::Map::new();

    for mut token in tokens.tokens {
        let Some(eth_addrs) = token.chain_addresses.remove(&Blockchain::Ethereum) else { continue };
        for addr in eth_addrs {
            phf_map.entry(addr.0, &format!("\"{}\"", token.global_id));
        }
    }

    writeln!(
        file,
        "pub static TOKEN_ADDRESS_TO_TICKER: phf::Map<[u8; 20], &'static str> = \n{};\n",
        phf_map.build()
    )
    .unwrap();
}

/// builds the clickhouse database client
fn build_db() -> Client {
    // clickhouse path
    let clickhouse_path = format!(
        "{}:{}",
        &env::var("CLICKHOUSE_URL").expect("CLICKHOUSE_URL not found in .env"),
        &env::var("CLICKHOUSE_PORT").expect("CLICKHOUSE_PORT not found in .env")
    );

    // builds the https connector
    let https = HttpsConnector::new();
    let https_client = hyper::Client::builder().build::<_, hyper::Body>(https);

    // builds the clickhouse client

    Client::with_http_client(https_client)
        .with_url(clickhouse_path)
        .with_user(env::var("CLICKHOUSE_USER").expect("CLICKHOUSE_USER not found in .env"))
        .with_password(env::var("CLICKHOUSE_PASS").expect("CLICKHOUSE_PASS not found in .env"))
        .with_database(
            env::var("CLICKHOUSE_DATABASE").expect("CLICKHOUSE_DATABASE not found in .env")
        )
}

async fn query_db<T: Row + for<'a> Deserialize<'a>>(db: &Client, query: &str) -> Vec<T> {
    db.query(query).fetch_all::<T>().await.unwrap()
}