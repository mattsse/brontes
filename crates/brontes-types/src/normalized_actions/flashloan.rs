use std::fmt::Debug;

use malachite::Rational;
use reth_primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use sorella_db_databases::{clickhouse, clickhouse::Row};

pub use super::{Actions, NormalizedSwap, NormalizedTransfer};
use crate::{db::token_info::TokenInfoWithAddress, Protocol};

#[derive(Debug, Serialize, Clone, Row, Deserialize, PartialEq, Eq)]
pub struct NormalizedFlashLoan {
    pub protocol:          Protocol,
    pub trace_index:       u64,
    pub from:              Address,
    pub pool:              Address,
    pub receiver_contract: Address,
    pub assets:            Vec<TokenInfoWithAddress>,
    pub amounts:           Vec<Rational>,
    // Special case for Aave flashloan modes, see:
    // https://docs.aave.com/developers/guides/flash-loans#completing-the-flash-loan
    pub aave_mode:         Option<(Vec<U256>, Address)>,

    // Child actions contained within this flashloan in order of execution
    // They can be:
    //  - Swaps
    //  - Liquidations
    //  - Mints
    //  - Burns
    //  - Transfers
    pub child_actions: Vec<Actions>,
    pub repayments:    Vec<NormalizedTransfer>,
    pub fees_paid:     Vec<Rational>,
}

impl NormalizedFlashLoan {
    pub fn finish_classification(&mut self, actions: Vec<(u64, Actions)>) -> Vec<u64> {
        let mut nodes_to_prune = Vec::new();
        let mut a_token_addresses = Vec::new();
        let mut repay_tranfers = Vec::new();

        for (index, action) in actions.into_iter() {
            match &action {
                // Use a reference to `action` here
                Actions::Swap(_)
                | Actions::FlashLoan(_)
                | Actions::Liquidation(_)
                | Actions::Batch(_)
                | Actions::Burn(_)
                | Actions::Mint(_) => {
                    self.child_actions.push(action);
                    nodes_to_prune.push(index);
                }
                Actions::Transfer(t) => {
                    // get the a_token reserve address that will be the receiver of the flashloan
                    // repayment for this token
                    if let Some(i) = self.assets.iter().position(|x| *x == t.token) {
                        if t.to == self.receiver_contract && t.amount == self.amounts[i] {
                            a_token_addresses.push(t.token.address);
                        }
                    }
                    // if the receiver contract is sending the token to the AToken address then this
                    // is the flashloan repayement
                    else if t.from == self.receiver_contract && a_token_addresses.contains(&t.to)
                    {
                        repay_tranfers.push(t.clone());
                        nodes_to_prune.push(index);
                    } else {
                        self.child_actions.push(action);
                        nodes_to_prune.push(index);
                    }
                }
                _ => continue,
            }
        }
        let fees = Vec::new();

        // //TODO: deal with diff aave modes, where part of the flashloan is taken on as
        // // debt by the OnBehalfOf address
        // for (i, amount) in self.amounts.iter().enumerate() {
        //     let repay_amount = repay_tranfers
        //         .iter()
        //         .find(|t| t.token == self.assets[i])
        //         .map_or(U256::ZERO, |t| t.amount);
        //     let fee = repay_amount - amount;
        //     fees.push(fee);
        // }

        self.fees_paid = fees;
        self.repayments = repay_tranfers;

        nodes_to_prune
    }
}
