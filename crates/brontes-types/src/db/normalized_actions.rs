use clickhouse::DbRow;
use itertools::MultiUnzip;
use reth_primitives::B256;
use serde::{ser::SerializeStruct, Deserialize, Serialize};

use crate::{normalized_actions::Actions, GasDetails, Node, Root};

#[derive(Debug, Clone)]
pub struct TransactionRoot {
    pub tx_hash:     B256,
    pub tx_idx:      usize,
    pub gas_details: GasDetails,
    pub trace_nodes: Vec<TraceNode>,
}

impl From<&Root<Actions>> for TransactionRoot {
    fn from(value: &Root<Actions>) -> Self {
        let tx_data = &value.data_store.0;
        let mut trace_nodes = Vec::new();
        make_trace_nodes(&value.head, tx_data, &mut trace_nodes);

        Self {
            tx_hash: value.tx_hash,
            tx_idx: value.position,
            gas_details: value.gas_details,
            trace_nodes,
        }
    }
}

impl Serialize for TransactionRoot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ser_struct = serializer.serialize_struct("TransactionRoot", 7)?;

        ser_struct.serialize_field("tx_hash", &format!("{:?}", self.tx_hash))?;
        ser_struct.serialize_field("tx_idx", &self.tx_idx)?;
        ser_struct.serialize_field(
            "gas_details",
            &(
                self.gas_details.coinbase_transfer,
                self.gas_details.priority_fee,
                self.gas_details.gas_used,
                self.gas_details.effective_gas_price,
            ),
        )?;

        let (trace_idx, trace_address, action_kind, action): (Vec<_>, Vec<_>, Vec<_>, Vec<_>) =
            self.trace_nodes
                .iter()
                .map(|node| {
                    (
                        node.trace_idx,
                        node.trace_address.clone(),
                        node.action_kind,
                        node.action
                            .as_ref()
                            .map(|a| serde_json::to_string(a).unwrap()),
                    )
                })
                .multiunzip();

        ser_struct.serialize_field("trace_nodes.trace_idx", &trace_idx)?;
        ser_struct.serialize_field("trace_nodes.trace_address", &trace_address)?;
        ser_struct.serialize_field("trace_nodes.action_kind", &action_kind)?;
        ser_struct.serialize_field("trace_nodes.action", &action)?;

        ser_struct.end()
    }
}

impl DbRow for TransactionRoot {
    const COLUMN_NAMES: &'static [&'static str] = &[
        "tx_hash",
        "tx_idx",
        "gas_details",
        "trace_nodes.trace_idx",
        "trace_nodes.trace_address",
        "trace_nodes.action_kind",
        "trace_nodes.action",
    ];
}

fn make_trace_nodes(node: &Node, actions: &[Option<Actions>], trace_nodes: &mut Vec<TraceNode>) {
    trace_nodes.push((node, actions).into());

    for n in &node.inner {
        make_trace_nodes(n, actions, trace_nodes)
    }
}

#[derive(Debug, Clone)]
pub struct TraceNode {
    pub trace_idx:     u64,
    pub trace_address: Vec<u64>,
    pub action_kind:   Option<ActionKind>,
    pub action:        Option<Actions>,
}

impl From<(&Node, &[Option<Actions>])> for TraceNode {
    fn from(value: (&Node, &[Option<Actions>])) -> Self {
        let (node, actions) = value;
        let action = actions
            .iter()
            .enumerate()
            .find(|(i, _)| *i == node.data)
            .map(|(_, a)| a)
            .cloned()
            .flatten();
        Self {
            trace_idx: node.index,
            trace_address: node
                .trace_address
                .iter()
                .map(|i| *i as u64)
                .collect::<Vec<_>>()
                .clone(),
            action_kind: action.as_ref().map(Into::into),
            action,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum ActionKind {
    Swap,
    SwapWithFee,
    FlashLoan,
    Batch,
    Transfer,
    Mint,
    Burn,
    Collect,
    Liquidation,
    Unclassified,
    SelfDestruct,
    EthTransfer,
    NewPool,
    PoolConfigUpdate,
    Aggregator,
    Revert,
}

impl From<&Actions> for ActionKind {
    fn from(value: &Actions) -> Self {
        match value {
            Actions::Swap(_) => ActionKind::Swap,
            Actions::SwapWithFee(_) => ActionKind::SwapWithFee,
            Actions::FlashLoan(_) => ActionKind::FlashLoan,
            Actions::Batch(_) => ActionKind::Batch,
            Actions::Mint(_) => ActionKind::Mint,
            Actions::Burn(_) => ActionKind::Burn,
            Actions::Transfer(_) => ActionKind::Transfer,
            Actions::Liquidation(_) => ActionKind::Liquidation,
            Actions::Collect(_) => ActionKind::Collect,
            Actions::SelfDestruct(_) => ActionKind::SelfDestruct,
            Actions::EthTransfer(_) => ActionKind::EthTransfer,
            Actions::Unclassified(_) => ActionKind::Unclassified,
            Actions::NewPool(_) => ActionKind::NewPool,
            Actions::PoolConfigUpdate(_) => ActionKind::PoolConfigUpdate,
            Actions::Aggregator(_) => ActionKind::Aggregator,
            Actions::Revert => ActionKind::Revert,
        }
    }
}

impl Serialize for ActionKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        format!("{:?}", self).serialize(serializer)
    }
}

#[cfg(test)]
pub mod test {
    use std::sync::Arc;

    use alloy_primitives::hex;
    use brontes_classifier::test_utils::ClassifierTestUtils;
    use brontes_types::{
        db::normalized_actions::{ActionKind, TransactionRoot},
        normalized_actions::Actions,
        BlockTree,
    };

    async fn load_tree() -> Arc<BlockTree<Actions>> {
        let classifier_utils = ClassifierTestUtils::new().await;
        let tx = hex!("31dedbae6a8e44ec25f660b3cd0e04524c6476a0431ab610bb4096f82271831b").into();
        classifier_utils.build_tree_tx(tx).await.unwrap().into()
    }

    #[brontes_macros::test]
    async fn test_into_tx_root() {
        let tree = load_tree().await;
        let root = &tree.clone().tx_roots[0];
        let tx_root = TransactionRoot::from(root);

        let burns = tx_root
            .trace_nodes
            .iter()
            .filter_map(|node| node.action_kind)
            .filter(|action| matches!(action, ActionKind::Burn))
            .count();
        assert_eq!(burns, 1);

        let swaps = tx_root
            .trace_nodes
            .iter()
            .filter_map(|node| node.action_kind)
            .filter(|action| matches!(action, ActionKind::Swap))
            .count();
        assert_eq!(swaps, 3);
    }
}