use std::{env, path};

use alloy_primitives::Address;
use brontes_database::libmdbx::{LibmdbxReadWriter, LibmdbxWriter};
use brontes_types::Protocol;
use serde::Deserialize;
use toml::Table;

const CONFIG_FILE_NAME: &str = "classifier_config.toml";

fn main() {
    insert_manually_defined_classifiers()
}

#[derive(Debug, Deserialize, Default)]
pub struct TokenInfoWithAddressToml {
    pub symbol:   String,
    pub decimals: u8,
    pub address:  Address,
}
fn insert_manually_defined_classifiers() {
    // don't run on local
    let _ = dotenv::dotenv();

    let Ok(prod_brontes_db_endpoint) = env::var("BRONTES_DB_PATH") else { return };
    let Ok(test_brontes_db_endpoint) = env::var("BRONTES_TEST_DB_PATH") else { return };
    let Ok(prod_libmdbx) = LibmdbxReadWriter::init_db(prod_brontes_db_endpoint, None) else {
        return
    };
    let Ok(test_libmdbx) = LibmdbxReadWriter::init_db(test_brontes_db_endpoint, None) else {
        return
    };

    let mut workspace_dir = workspace_dir();
    workspace_dir.push(CONFIG_FILE_NAME);

    let config: Table =
        toml::from_str(&std::fs::read_to_string(workspace_dir).expect("no config file"))
            .expect("failed to parse toml");

    for (protocol, inner) in config {
        let protocol: Protocol = protocol.parse().unwrap();
        for (address, table) in inner.as_table().unwrap() {
            let token_addr: Address = address.parse().unwrap();
            let init_block = table.get("init_block").unwrap().as_integer().unwrap() as u64;

            let table: Vec<TokenInfoWithAddressToml> = table
                .get("token_info")
                .map(|i| i.clone().try_into())
                .unwrap_or(Ok(vec![]))
                .unwrap_or(vec![]);

            for t_info in &table {
                prod_libmdbx
                    .write_token_info(t_info.address, t_info.decimals, t_info.symbol.clone())
                    .unwrap();
                test_libmdbx
                    .write_token_info(t_info.address, t_info.decimals, t_info.symbol.clone())
                    .unwrap();
            }

            let token_addrs = if table.len() < 2 {
                [Address::default(), Address::default()]
            } else {
                [table[0].address, table[1].address]
            };

            prod_libmdbx
                .insert_pool(init_block, token_addr, token_addrs, protocol)
                .unwrap();
            test_libmdbx
                .insert_pool(init_block, token_addr, token_addrs, protocol)
                .unwrap();
        }
    }
}

fn workspace_dir() -> path::PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = path::Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}