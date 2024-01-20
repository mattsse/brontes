use brontes_types::{exchanges::StaticBindingsDb, serde_utils::primitives::address_string};
use reth_primitives::Address;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sorella_db_databases::clickhouse::{self, Row};

use super::{utils::static_bindings, LibmdbxData};
use crate::libmdbx::AddressToProtocol;

/// rlp encoding for libmdbx here is fine since it is just an enum
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Row)]
pub struct AddressToProtocolData {
    #[serde(with = "address_string")]
    pub address: Address,

    #[serde(with = "static_bindings")]
    pub classifier_name: StaticBindingsDb,
}

impl AddressToProtocolData {
    pub fn new(address: Address, classifier_name: StaticBindingsDb) -> Self {
        Self { classifier_name, address }
    }
}

impl LibmdbxData<AddressToProtocol> for AddressToProtocolData {
    fn into_key_val(
        &self,
    ) -> (
        <AddressToProtocol as reth_db::table::Table>::Key,
        <AddressToProtocol as reth_db::table::Table>::Value,
    ) {
        (self.address, self.classifier_name)
    }
}