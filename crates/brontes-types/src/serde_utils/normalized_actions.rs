use alloy_primitives::TxHash;
use itertools::Itertools;
use sorella_db_databases::clickhouse::fixed_string::FixedString;

use crate::normalized_actions::{
    NormalizedBurn, NormalizedLiquidation, NormalizedMint, NormalizedSwap,
};

pub struct ClickhouseVecNormalizedSwap {
    pub trace_index: Vec<u64>,
    pub from:        Vec<FixedString>,
    pub recipient:   Vec<FixedString>,
    pub pool:        Vec<FixedString>,
    pub token_in:    Vec<FixedString>,
    pub token_out:   Vec<FixedString>,
    pub amount_in:   Vec<[u8; 32]>,
    pub amount_out:  Vec<[u8; 32]>,
}

impl From<Vec<NormalizedSwap>> for ClickhouseVecNormalizedSwap {
    fn from(value: Vec<NormalizedSwap>) -> Self {
        ClickhouseVecNormalizedSwap {
            trace_index: value.iter().map(|val| val.trace_index).collect(),
            from:        value
                .iter()
                .map(|val| format!("{:?}", val.from).into())
                .collect(),
            recipient:   value
                .iter()
                .map(|val| format!("{:?}", val.recipient).into())
                .collect(),
            pool:        value
                .iter()
                .map(|val| format!("{:?}", val.pool).into())
                .collect(),
            token_in:    value
                .iter()
                .map(|val| format!("{:?}", val.token_in).into())
                .collect(),
            token_out:   value
                .iter()
                .map(|val| format!("{:?}", val.token_out).into())
                .collect(),
            amount_in:   value
                .iter()
                .map(|val| val.amount_in.to_le_bytes())
                .collect(),
            amount_out:  value
                .iter()
                .map(|val| val.amount_out.to_le_bytes())
                .collect(),
        }
    }
}

#[derive(Default)]
pub struct ClickhouseDoubleVecNormalizedSwap {
    pub tx_hash:     Vec<FixedString>, /* clickhouse requires nested fields to have the same
                                        * number of rows */
    pub trace_index: Vec<u64>,
    pub from:        Vec<FixedString>,
    pub recipient:   Vec<FixedString>,
    pub pool:        Vec<FixedString>,
    pub token_in:    Vec<FixedString>,
    pub token_out:   Vec<FixedString>,
    pub amount_in:   Vec<[u8; 32]>,
    pub amount_out:  Vec<[u8; 32]>,
}

impl From<(Vec<TxHash>, Vec<Vec<NormalizedSwap>>)> for ClickhouseDoubleVecNormalizedSwap {
    fn from(value: (Vec<TxHash>, Vec<Vec<NormalizedSwap>>)) -> Self {
        let swaps: Vec<(FixedString, ClickhouseVecNormalizedSwap, usize)> = value
            .0
            .into_iter()
            .zip(value.1.into_iter())
            .map(|(tx, swaps)| {
                let num_swaps = swaps.len();
                (format!("{:?}", tx).into(), swaps.into(), num_swaps)
            })
            .collect::<Vec<_>>();

        let mut this = ClickhouseDoubleVecNormalizedSwap::default();

        swaps.into_iter().for_each(|(tx, inner_swaps, num_swaps)| {
            let tx_repeated = (0..num_swaps)
                .into_iter()
                .map(|_| tx.clone())
                .collect::<Vec<FixedString>>();

            if tx_repeated.len() != num_swaps {
                panic!(
                    "The repetitions of tx hash must be equal to the number of swaps when \
                     serializing for clickhouse"
                )
            }

            this.tx_hash.extend(tx_repeated);
            this.trace_index.extend(inner_swaps.trace_index);
            this.from.extend(inner_swaps.from);
            this.recipient.extend(inner_swaps.recipient);
            this.pool.extend(inner_swaps.pool);
            this.token_in.extend(inner_swaps.token_in);
            this.token_out.extend(inner_swaps.token_out);
            this.amount_in.extend(inner_swaps.amount_in);
            this.amount_out.extend(inner_swaps.amount_out);
        });

        this
    }
}

pub struct ClickhouseVecNormalizedMintOrBurn {
    pub trace_index: Vec<u64>,
    pub from:        Vec<FixedString>,
    pub to:          Vec<FixedString>,
    pub recipient:   Vec<FixedString>,
    pub tokens:      Vec<Vec<FixedString>>,
    pub amounts:     Vec<Vec<[u8; 32]>>,
}

impl From<Vec<NormalizedMint>> for ClickhouseVecNormalizedMintOrBurn {
    fn from(value: Vec<NormalizedMint>) -> Self {
        ClickhouseVecNormalizedMintOrBurn {
            trace_index: value.iter().map(|val| val.trace_index).collect(),
            from:        value
                .iter()
                .map(|val| format!("{:?}", val.from).into())
                .collect(),
            to:          value
                .iter()
                .map(|val| format!("{:?}", val.to).into())
                .collect(),
            recipient:   value
                .iter()
                .map(|val| format!("{:?}", val.recipient).into())
                .collect(),

            tokens:  value
                .iter()
                .map(|val| {
                    val.token
                        .iter()
                        .map(|t| format!("{:?}", t).into())
                        .collect_vec()
                })
                .collect(),
            amounts: value
                .iter()
                .map(|val| val.amount.iter().map(|amt| amt.to_le_bytes()).collect_vec())
                .collect(),
        }
    }
}

impl From<Vec<NormalizedBurn>> for ClickhouseVecNormalizedMintOrBurn {
    fn from(value: Vec<NormalizedBurn>) -> Self {
        ClickhouseVecNormalizedMintOrBurn {
            trace_index: value.iter().map(|val| val.trace_index).collect(),
            from:        value
                .iter()
                .map(|val| format!("{:?}", val.from).into())
                .collect(),
            to:          value
                .iter()
                .map(|val| format!("{:?}", val.to).into())
                .collect(),
            recipient:   value
                .iter()
                .map(|val| format!("{:?}", val.recipient).into())
                .collect(),

            tokens:  value
                .iter()
                .map(|val| {
                    val.token
                        .iter()
                        .map(|t| format!("{:?}", t).into())
                        .collect_vec()
                })
                .collect(),
            amounts: value
                .iter()
                .map(|val| val.amount.iter().map(|amt| amt.to_le_bytes()).collect_vec())
                .collect(),
        }
    }
}

pub struct ClickhouseVecNormalizedLiquidation {
    pub trace_index:           Vec<u64>,
    pub pool:                  Vec<FixedString>,
    pub liquidator:            Vec<FixedString>,
    pub debtor:                Vec<FixedString>,
    pub collateral_asset:      Vec<FixedString>,
    pub debt_asset:            Vec<FixedString>,
    pub covered_debt:          Vec<[u8; 32]>,
    pub liquidated_collateral: Vec<[u8; 32]>,
}

impl From<Vec<NormalizedLiquidation>> for ClickhouseVecNormalizedLiquidation {
    fn from(value: Vec<NormalizedLiquidation>) -> Self {
        ClickhouseVecNormalizedLiquidation {
            trace_index: value.iter().map(|val| val.trace_index).collect(),
            pool:        value
                .iter()
                .map(|val| format!("{:?}", val.pool).into())
                .collect(),
            liquidator:  value
                .iter()
                .map(|val| format!("{:?}", val.liquidator).into())
                .collect(),
            debtor:      value
                .iter()
                .map(|val| format!("{:?}", val.debtor).into())
                .collect(),

            collateral_asset:      value
                .iter()
                .map(|val| format!("{:?}", val.collateral_asset).into())
                .collect(),
            debt_asset:            value
                .iter()
                .map(|val| format!("{:?}", val.debt_asset).into())
                .collect(),
            covered_debt:          value
                .iter()
                .map(|val| val.covered_debt.to_le_bytes())
                .collect(),
            liquidated_collateral: value
                .iter()
                .map(|val| val.liquidated_collateral.to_le_bytes())
                .collect(),
        }
    }
}