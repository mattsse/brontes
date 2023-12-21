use alloy_primitives::Address;
use brontes_types::libmdbx::serde::address_string;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sorella_db_databases::{clickhouse, Row};

use super::LibmdbxData;
use crate::tables::TokenDecimals;

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Row)]
pub struct TokenDecimalsData {
    #[serde(with = "address_string")]
    pub address:  Address,
    pub decimals: u8,
}

impl LibmdbxData<TokenDecimals> for TokenDecimalsData {
    fn into_key_val(
        &self,
    ) -> (
        <TokenDecimals as reth_db::table::Table>::Key,
        <TokenDecimals as reth_db::table::Table>::Value,
    ) {
        (self.address, self.decimals)
    }
}