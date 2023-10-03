use std::{collections::HashSet, path::PathBuf, pin::Pin, sync::Arc};

use alloy_etherscan::Client;
use brontes_types::structured_trace::TxTrace;
use ethers::prelude::{Middleware, Provider};
use ethers_core::types::Chain;
use ethers_reth::type_conversions::{ToEthers, ToReth};
use futures::Future;
use reth_interfaces::{RethError, RethResult};
use reth_primitives::{BlockId, BlockNumber, BlockNumberOrTag, Header, H256};
use reth_provider::{BlockIdReader, BlockNumReader, HeaderProvider, ReceiptProvider};
use reth_rpc_api::EthApiServer;
use reth_rpc_types::trace::parity::TraceType;
use reth_tracing::TracingClient;
use tokio::{sync::mpsc::UnboundedSender, task::JoinError};

use self::parser::TraceParser;
use crate::{
    executor::{Executor, TaskKind},
    init_trace,
};

mod parser;
mod utils;
use brontes_metrics::{trace::types::TraceMetricEvent, PoirotMetricEvents};
#[allow(dead_code)]
pub(crate) const UNKNOWN: &str = "unknown";
#[allow(dead_code)]
pub(crate) const RECEIVE: &str = "receive";
#[allow(dead_code)]
pub(crate) const FALLBACK: &str = "fallback";

const CACHE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10_000);
const CACHE_DIRECTORY: &str = "./abi_cache";

use reth_rpc::eth::error::EthApiError;
use reth_rpc_types::{trace::parity::TraceResultsWithTransactionHash, TransactionReceipt};

#[async_trait::async_trait]
pub trait TracingProvider: Send + Sync + 'static {
    async fn block_hash_for_id(&self, block_num: u64) -> reth_interfaces::RethResult<Option<H256>>;

    async fn best_block_number(&self) -> reth_interfaces::RethResult<u64>;

    async fn replay_block_transactions(
        &self,
        block_id: BlockId,
        trace_type: HashSet<TraceType>,
    ) -> Result<Option<Vec<TraceResultsWithTransactionHash>>, EthApiError>;

    async fn block_receipts(
        &self,
        number: BlockNumberOrTag,
    ) -> reth_interfaces::RethResult<Option<Vec<TransactionReceipt>>>;

    async fn header_by_number(
        &self,
        number: BlockNumber,
    ) -> reth_interfaces::RethResult<Option<Header>>;
}

#[async_trait::async_trait]
trait TracingP: Send + Sync + 'static {
    async fn block_hash_for_id(&self, block_num: u64) -> reth_interfaces::RethResult<Option<H256>>;

    async fn best_block_number(&self) -> reth_interfaces::RethResult<u64>;

    async fn replay_block_transactions(
        &self,
        block_id: BlockId,
        trace_type: HashSet<TraceType>,
    ) -> Result<Option<Vec<TraceResultsWithTransactionHash>>, EthApiError>;

    async fn block_receipts(
        &self,
        number: BlockNumberOrTag,
    ) -> reth_interfaces::RethResult<Option<Vec<TransactionReceipt>>>;

    async fn header_by_number(
        &self,
        number: BlockNumber,
    ) -> reth_interfaces::RethResult<Option<Header>>;
}

#[async_trait::async_trait]
impl TracingProvider for dyn TracingP {
    async fn block_hash_for_id(&self, block_num: u64) -> reth_interfaces::RethResult<Option<H256>> {
        self.block_hash_for_id(block_num).await
    }

    async fn best_block_number(&self) -> reth_interfaces::RethResult<u64> {
        self.best_block_number().await
    }

    async fn replay_block_transactions(
        &self,
        block_id: BlockId,
        trace_type: HashSet<TraceType>,
    ) -> Result<Option<Vec<TraceResultsWithTransactionHash>>, EthApiError> {
        self.replay_block_transactions(block_id, trace_type).await
    }

    async fn block_receipts(
        &self,
        number: BlockNumberOrTag,
    ) -> reth_interfaces::RethResult<Option<Vec<TransactionReceipt>>> {
        self.block_receipts(number).await
    }

    async fn header_by_number(
        &self,
        number: BlockNumber,
    ) -> reth_interfaces::RethResult<Option<Header>> {
        self.header_by_number(number).await
    }
}

#[async_trait::async_trait]
impl<T: Middleware + 'static> TracingP for T
where
    RethError: From<<T as Middleware>::Error>,
    EthApiError: From<<T as Middleware>::Error>,
{
    async fn block_hash_for_id(&self, block_num: u64) -> reth_interfaces::RethResult<Option<H256>> {
        Ok(self
            .get_block(block_num)
            .await?
            .map(|h| h.hash.map(|e| e.into()))
            .flatten())
    }

    async fn best_block_number(&self) -> reth_interfaces::RethResult<u64> {
        self.get_block_number()
            .await
            .map(|r| r.as_u64())
            .map_err(|e| e.into())
    }

    async fn replay_block_transactions(
        &self,
        block_id: BlockId,
        trace_type: HashSet<TraceType>,
    ) -> Result<Option<Vec<TraceResultsWithTransactionHash>>, EthApiError> {
        let block_id = match block_id {
            BlockId::Number(t) => t.as_number().unwrap(),
            _ => return Err(EthApiError::PrevrandaoNotSet),
        };
        Ok(Some(
            self.trace_replay_block_transactions(
                block_id.into(),
                trace_type
                    .into_iter()
                    .map(|i| i.into_ethers())
                    .collect::<Vec<_>>(),
            )
            .await?
            .into_iter()
            .map(|m| m.into_reth())
            .collect::<Vec<_>>(),
        ))
    }

    async fn block_receipts(
        &self,
        number: BlockNumberOrTag,
    ) -> reth_interfaces::RethResult<Option<Vec<TransactionReceipt>>> {
        let number = number.as_number().unwrap();

        Ok(Some(
            self.get_block_receipts(number)
                .await?
                .into_iter()
                .map(|t| t.into_reth())
                .collect::<Vec<TransactionReceipt>>(),
        ))
    }

    async fn header_by_number(
        &self,
        number: BlockNumber,
    ) -> reth_interfaces::RethResult<Option<Header>> {
        let a = self.get_block(number).await?.unwrap();
        let mut header = Header::default();
        header.base_fee_per_gas = a.base_fee_per_gas.map(|f| f.as_u64());

        Ok(Some(header))
    }
}

#[async_trait::async_trait]
impl TracingProvider for TracingClient {
    async fn block_hash_for_id(&self, block_num: u64) -> reth_interfaces::RethResult<Option<H256>> {
        self.trace
            .provider()
            .block_hash_for_id(BlockId::Number(BlockNumberOrTag::Number(block_num)))
    }

    async fn best_block_number(&self) -> reth_interfaces::RethResult<u64> {
        self.trace.provider().best_block_number()
    }

    async fn replay_block_transactions(
        &self,
        block_id: BlockId,
        trace_type: HashSet<TraceType>,
    ) -> Result<Option<Vec<TraceResultsWithTransactionHash>>, EthApiError> {
        self.trace
            .replay_block_transactions(block_id, trace_type)
            .await
    }

    async fn block_receipts(
        &self,
        number: BlockNumberOrTag,
    ) -> reth_interfaces::RethResult<Option<Vec<TransactionReceipt>>> {
        Ok(Some(self.api.block_receipts(number).await.unwrap().unwrap()))
    }

    async fn header_by_number(
        &self,
        number: BlockNumber,
    ) -> reth_interfaces::RethResult<Option<Header>> {
        self.trace.provider().header_by_number(number)
    }
}

pub type ParserFuture = Pin<
    Box<dyn Future<Output = Result<Option<(Vec<TxTrace>, Header)>, JoinError>> + Send + 'static>,
>;

pub struct Parser<T: TracingProvider> {
    executor: Executor,
    parser:   Arc<TraceParser<T>>,
}

impl<T: TracingProvider> Parser<T> {
    pub fn new(
        metrics_tx: UnboundedSender<PoirotMetricEvents>,
        etherscan_key: &str,
        tracing: T,
    ) -> Self {
        let executor = Executor::new();
        // let tracer =
        //     Arc::new(TracingClient::new(Path::new(db_path),
        // executor.runtime.handle().clone()));

        let etherscan_client = Client::new_cached(
            Chain::Mainnet,
            etherscan_key,
            Some(PathBuf::from(CACHE_DIRECTORY)),
            CACHE_TIMEOUT,
        )
        .unwrap();
        let parser = TraceParser::new(etherscan_client, Arc::new(tracing), Arc::new(metrics_tx));

        Self { executor, parser: Arc::new(parser) }
    }

    pub async fn get_block_hash_for_number(
        &self,
        block_num: u64,
    ) -> reth_interfaces::RethResult<Option<H256>> {
        self.parser.tracer.block_hash_for_id(block_num.into()).await
    }

    pub async fn get_latest_block_number(&self) -> RethResult<BlockNumber> {
        self.parser.tracer.best_block_number().await
    }

    /// executes the tracing of a given block
    pub fn execute(&self, block_num: u64) -> ParserFuture {
        let parser = self.parser.clone();
        Box::pin(self.executor.spawn_result_task_as(
            async move { parser.execute_block(block_num).await },
            TaskKind::Default,
        )) as ParserFuture
    }
}
