use std::{collections::HashMap, str::FromStr};

use brontes_classifier::{
    test_utils::{
        build_raw_test_tree, get_traces_with_meta, helper_decode_transfer, helper_prove_dyn_action,
    },
    Classifier,
};
use brontes_core::test_utils::init_trace_parser;
use brontes_database::database::Database;
use reth_primitives::{H160, H256};
use tokio::sync::mpsc::unbounded_channel;

use crate::UNIT_TESTS_BLOCK_NUMBER;

/// Uniswap V2 - Bone Shibaswap <> Weth
fn token_mapping() -> HashMap<H160, (H160, H160)> {
    let mut map = HashMap::new();
    map.insert(
        H160::from_str("0xF7d31825946e7fD99eF07212d34B9Dad84C396b7").unwrap(),
        (
            H160::from_str("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2").unwrap(),
            H160::from_str("0x9813037ee2218799597d83d4a5b6f3b6778218d9").unwrap(),
        ),
    );
    map
}

async fn test_classified_tree() {
    let (tx, _rx) = unbounded_channel();
    let tracer = init_trace_parser(tokio::runtime::Handle::current().clone(), tx);

    let db = Database::default();
    let classifier = Classifier::new();

    let (traces, header, metadata) =
        get_traces_with_meta(&tracer, db, UNIT_TESTS_BLOCK_NUMBER).await;

    let tree = classifier.build_tree(traces, header, &metadata);
}

#[tokio::test]
async fn test_try_classify_unknown_exchanges() {
    let (tx, _rx) = unbounded_channel();
    let tracer = init_trace_parser(tokio::runtime::Handle::current().clone(), tx);

    let db = Database::default();
    let classifier = Classifier::new();

    let token_mapping = token_mapping();

    let mut tree = build_raw_test_tree(&tracer, db, UNIT_TESTS_BLOCK_NUMBER).await;
    let node = &mut tree.roots.drain(7..8).collect::<Vec<_>>()[0].head;

    let (token_0, token_1) = token_mapping
        .get(&H160::from_str("0xF7d31825946e7fD99eF07212d34B9Dad84C396b7").unwrap())
        .unwrap();

    let addr = node.address;
    let subactions = node.get_all_sub_actions();
    let logs = subactions
        .iter()
        .flat_map(|i| i.get_logs())
        .collect::<Vec<_>>();

    println!("{:?}\n", &logs);

    let mut transfer_data = Vec::new();

    // index all transfers. due to tree this should only be two transactions
    for log in logs {
        if let Some((token, from, to, value)) = helper_decode_transfer(&log) {
            // if tokens don't overlap and to & from don't overlap
            if (token_0 != &token && token_1 != &token) || (from != addr && to != addr) {
                continue
            }

            transfer_data.push((token, from, to, value));
        }
    }

    println!("{:?}", &transfer_data);

    //let res = helper_prove_dyn_action(classifier, node, *token_0, *token_1);
    //println!("{:?}", res);
}
