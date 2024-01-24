use alloy_primitives::{Address, U256};
use brontes_macros::action_impl;
use brontes_pricing::Protocol;
use brontes_types::normalized_actions::{NormalizedFlashLoan, NormalizedLiquidation};

action_impl!(
    Protocol::AaveV3,
    crate::AaveV3::liquidationCallCall,
    Liquidation,
    [LiquidationEvent],
    call_data: true,
    |trace_index,
    _from_address: Address,
    target_address: Address,
    msg_sender: Address,
    call_data: liquidationCallCall,
    _db_tx: &DB | {
        return Some(NormalizedLiquidation {
            trace_index,
            pool: target_address,
            liquidator: msg_sender,
            debtor: call_data.user,
            collateral_asset: call_data.collateralAsset,
            debt_asset: call_data.debtAsset,
            covered_debt: call_data.debtToCover,
            // filled in later
            liquidated_collateral: U256::ZERO,
        })
    }
);

action_impl!(
    Protocol::AaveV3,
    crate::AaveV3::flashLoanCall,
    FlashLoan,
    [],
    call_data: true,
    |trace_index,
    from_address: Address,
    target_address: Address,
    _msg_sender: Address,
    call_data: flashLoanCall,
    _db_tx: &DB | {
        return Some(NormalizedFlashLoan {
            trace_index,
            from: from_address,
            pool: target_address,
            receiver_contract: call_data.receiverAddress,
            assets: call_data.assets,
            amounts: call_data.amounts,
            aave_mode: Some((call_data.interestRateModes, call_data.onBehalfOf)),
            // These fields are all empty at this stage, they will be filled upon finalized classification
            child_actions: vec![],
            repayments: vec![],
            fees_paid: vec![],


        })

    }
);

action_impl!(
    Protocol::AaveV3,
    crate::AaveV3::flashLoanSimpleCall,
    FlashLoan,
    [],
    call_data: true,
    |trace_index,
    from_address: Address,
    target_address: Address,
    _msg_sender: Address,
    call_data: flashLoanSimpleCall,
    _db_tx: &DB | {
        return Some(NormalizedFlashLoan {
            trace_index,
            from: from_address,
            pool: target_address,
            receiver_contract: call_data.receiverAddress,
            assets: vec![call_data.asset],
            amounts: vec![call_data.amount],
            aave_mode: None,
            // These fields are all empty at this stage, they will be filled upon finalized classification
            child_actions: vec![],
            repayments: vec![],
            fees_paid: vec![],


        })

    }
);
