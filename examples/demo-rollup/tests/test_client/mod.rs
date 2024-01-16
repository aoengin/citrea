use std::str::FromStr;
use std::time::Duration;

use ethereum_types::H160;
use ethers_core::abi::Address;
use ethers_core::k256::ecdsa::SigningKey;
use ethers_core::types::transaction::eip2718::TypedTransaction;
use ethers_core::types::{Block, BlockId, Bytes, Eip1559TransactionRequest, Transaction, TxHash};
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Middleware, PendingTransaction, Provider};
use ethers_signers::Wallet;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use reth_primitives::BlockNumberOrTag;
use sov_evm::LogResponse;

pub const MAX_FEE_PER_GAS: u64 = 1000000001;
const GAS: u64 = 900000u64;

pub struct TestClient {
    chain_id: u64,
    pub(crate) from_addr: Address,
    client: SignerMiddleware<Provider<Http>, Wallet<SigningKey>>,
    http_client: HttpClient,
}

impl TestClient {
    #[allow(dead_code)]
    pub(crate) async fn new(
        chain_id: u64,
        key: Wallet<SigningKey>,
        from_addr: Address,
        rpc_addr: std::net::SocketAddr,
    ) -> Self {
        let host = format!("http://localhost:{}", rpc_addr.port());

        let provider = Provider::try_from(&host).unwrap();
        let client = SignerMiddleware::new_with_provider_chain(provider, key)
            .await
            .unwrap();

        let http_client = HttpClientBuilder::default().build(host).unwrap();

        Self {
            chain_id,
            from_addr,
            client,
            http_client,
        }
    }

    pub(crate) async fn send_publish_batch_request(&self) {
        let _: () = self
            .http_client
            .request("eth_publishBatch", rpc_params![])
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await
    }

    pub(crate) async fn deploy_contract(
        &self,
        byte_code: Bytes,
        nonce: Option<u64>,
    ) -> Result<PendingTransaction<'_, Http>, Box<dyn std::error::Error>> {
        let nonce = match nonce {
            Some(nonce) => nonce,
            None => {
                self.eth_get_transaction_count(self.from_addr, BlockNumberOrTag::Latest)
                    .await
            }
        };
        let req = Eip1559TransactionRequest::new()
            .from(self.from_addr)
            .chain_id(self.chain_id)
            .nonce(nonce)
            .max_priority_fee_per_gas(10u64)
            .max_fee_per_gas(MAX_FEE_PER_GAS)
            .gas(GAS)
            .data(byte_code);

        let typed_transaction = TypedTransaction::Eip1559(req);

        let receipt_req = self
            .client
            .send_transaction(typed_transaction, None)
            .await?;
        Ok(receipt_req)
    }

    pub(crate) async fn deploy_contract_call(
        &self,
        byte_code: Bytes,
        nonce: Option<u64>,
    ) -> Result<Bytes, Box<dyn std::error::Error>> {
        let nonce = match nonce {
            Some(nonce) => nonce,
            None => {
                self.eth_get_transaction_count(self.from_addr, BlockNumberOrTag::Latest)
                    .await
            }
        };
        let req = Eip1559TransactionRequest::new()
            .from(self.from_addr)
            .chain_id(self.chain_id)
            .nonce(nonce)
            .max_priority_fee_per_gas(10u64)
            .max_fee_per_gas(MAX_FEE_PER_GAS)
            .gas(GAS)
            .data(byte_code);

        let typed_transaction = TypedTransaction::Eip1559(req);

        let receipt_req = self.eth_call(typed_transaction, None).await?;

        Ok(receipt_req)
    }

    pub(crate) async fn contract_transaction(
        &self,
        contract_address: H160,
        data: Bytes,
        nonce: Option<u64>,
    ) -> PendingTransaction<'_, Http> {
        let nonce = match nonce {
            Some(nonce) => nonce,
            None => {
                self.eth_get_transaction_count(self.from_addr, BlockNumberOrTag::Latest)
                    .await
            }
        };
        self.eth_get_transaction_count(self.from_addr, BlockNumberOrTag::Latest)
            .await;
        let req = Eip1559TransactionRequest::new()
            .from(self.from_addr)
            .to(contract_address)
            .chain_id(self.chain_id)
            .nonce(nonce)
            .max_priority_fee_per_gas(10u64)
            .max_fee_per_gas(MAX_FEE_PER_GAS)
            .gas(GAS)
            .data(data);

        let typed_transaction = TypedTransaction::Eip1559(req);

        self.client
            .send_transaction(typed_transaction, None)
            .await
            .unwrap()
    }

    pub(crate) async fn contract_transaction_with_custom_fee(
        &self,
        contract_address: H160,
        data: Bytes,
        max_priority_fee_per_gas: u64,
        max_fee_per_gas: u64,
        value: Option<u64>,
        nonce: Option<u64>,
    ) -> PendingTransaction<'_, Http> {
        let nonce = match nonce {
            Some(nonce) => nonce,
            None => {
                self.eth_get_transaction_count(self.from_addr, BlockNumberOrTag::Latest)
                    .await
            }
        };
        let req = Eip1559TransactionRequest::new()
            .from(self.from_addr)
            .to(contract_address)
            .chain_id(self.chain_id)
            .nonce(nonce)
            .max_priority_fee_per_gas(max_priority_fee_per_gas)
            .max_fee_per_gas(max_fee_per_gas)
            .gas(GAS)
            .data(data)
            .value(value.unwrap_or(0u64));

        let typed_transaction = TypedTransaction::Eip1559(req);

        self.client
            .send_transaction(typed_transaction, None)
            .await
            .unwrap()
    }

    pub(crate) async fn contract_call<T: FromStr>(
        &self,
        contract_address: H160,
        data: Bytes,
        nonce: Option<u64>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let nonce = match nonce {
            Some(nonce) => nonce,
            None => {
                self.eth_get_transaction_count(self.from_addr, BlockNumberOrTag::Latest)
                    .await
            }
        };
        let req = Eip1559TransactionRequest::new()
            .from(self.from_addr)
            .to(contract_address)
            .chain_id(self.chain_id)
            .nonce(nonce)
            .max_priority_fee_per_gas(10u64)
            .max_fee_per_gas(MAX_FEE_PER_GAS)
            .gas(GAS)
            .data(data);

        let typed_transaction = TypedTransaction::Eip1559(req);

        let receipt_req = self.client.call(&typed_transaction, None).await?;

        T::from_str(&receipt_req.to_string()).map_err(|_| "Failed to parse bytes".into())
    }

    pub(crate) async fn send_eth(
        &self,
        to_addr: Address,
        max_priority_fee_per_gas: Option<u64>,
        max_fee_per_gas: Option<u64>,
        nonce: Option<u64>,
        value: u128,
    ) -> Result<PendingTransaction<'_, Http>, anyhow::Error> {
        let nonce = match nonce {
            Some(nonce) => nonce,
            None => {
                self.eth_get_transaction_count(self.from_addr, BlockNumberOrTag::Latest)
                    .await
            }
        };

        let req = Eip1559TransactionRequest::new()
            .from(self.from_addr)
            .to(to_addr)
            .chain_id(self.chain_id)
            .nonce(nonce)
            .max_priority_fee_per_gas(max_priority_fee_per_gas.unwrap_or(10u64))
            .max_fee_per_gas(max_fee_per_gas.unwrap_or(MAX_FEE_PER_GAS))
            .gas(GAS)
            .value(value);

        let typed_transaction = TypedTransaction::Eip1559(req);

        self.client
            .send_transaction(typed_transaction, None)
            .await
            .map_err(|e| e.into())
    }

    pub(crate) async fn web3_client_version(&self) -> String {
        self.http_client
            .request("web3_clientVersion", rpc_params![])
            .await
            .unwrap()
    }

    pub(crate) async fn web3_sha3(&self, bytes: String) -> String {
        self.http_client
            .request("web3_sha3", rpc_params![bytes])
            .await
            .unwrap()
    }

    pub(crate) async fn eth_accounts(&self) -> Vec<Address> {
        self.http_client
            .request("eth_accounts", rpc_params![])
            .await
            .unwrap()
    }

    #[allow(dead_code)]
    pub(crate) async fn eth_send_transaction(
        &self,
        tx: TypedTransaction,
    ) -> PendingTransaction<'_, Http> {
        self.client
            .provider()
            .send_transaction(tx, None)
            .await
            .unwrap()
    }

    pub(crate) async fn eth_chain_id(&self) -> u64 {
        let chain_id: ethereum_types::U64 = self
            .http_client
            .request("eth_chainId", rpc_params![])
            .await
            .unwrap();

        chain_id.as_u64()
    }

    pub(crate) async fn eth_get_balance(
        &self,
        address: Address,
        block_number: BlockNumberOrTag,
    ) -> ethereum_types::U256 {
        self.http_client
            .request("eth_getBalance", rpc_params![address, block_number])
            .await
            .unwrap()
    }

    pub(crate) async fn eth_get_storage_at(
        &self,
        address: Address,
        index: ethereum_types::U256,
        block_number: BlockNumberOrTag,
    ) -> ethereum_types::U256 {
        self.http_client
            .request(
                "eth_getStorageAt",
                rpc_params![address, index, block_number],
            )
            .await
            .unwrap()
    }

    pub(crate) async fn eth_get_code(
        &self,
        address: Address,
        block_number: BlockNumberOrTag,
    ) -> Bytes {
        self.http_client
            .request("eth_getCode", rpc_params![address, block_number])
            .await
            .unwrap()
    }

    pub(crate) async fn eth_get_transaction_count(
        &self,
        address: Address,
        block_number: BlockNumberOrTag,
    ) -> u64 {
        let count: ethereum_types::U64 = self
            .http_client
            .request(
                "eth_getTransactionCount",
                rpc_params![address, block_number],
            )
            .await
            .unwrap();

        count.as_u64()
    }

    pub(crate) async fn eth_gas_price(&self) -> ethereum_types::U256 {
        self.http_client
            .request("eth_gasPrice", rpc_params![])
            .await
            .unwrap()
    }

    pub(crate) async fn eth_fee_history(
        &self,
        block_count: String,
        newest_block: BlockNumberOrTag,
        reward_percentiles: Option<Vec<f64>>,
    ) -> FeeHistory {
        let rpc_params = rpc_params![block_count, newest_block, reward_percentiles];
        self.http_client
            .request("eth_feeHistory", rpc_params)
            .await
            .unwrap()
    }

    pub(crate) async fn eth_get_block_by_number(
        &self,
        block_number: Option<BlockNumberOrTag>,
    ) -> Block<TxHash> {
        self.http_client
            .request("eth_getBlockByNumber", rpc_params![block_number, false])
            .await
            .unwrap()
    }

    pub(crate) async fn eth_get_block_by_number_with_detail(
        &self,
        block_number: Option<BlockNumberOrTag>,
    ) -> Block<Transaction> {
        self.http_client
            .request("eth_getBlockByNumber", rpc_params![block_number, true])
            .await
            .unwrap()
    }

    #[allow(dead_code)]
    pub(crate) async fn eth_get_transaction_by_hash(&self, tx_hash: TxHash) -> Option<Transaction> {
        self.http_client
            .request("eth_getTransactionByHash", rpc_params![tx_hash])
            .await
            .unwrap()
    }

    pub(crate) async fn eth_get_block_receipts(
        &self,
        block_number_or_hash: BlockId,
    ) -> Vec<ethers_core::types::TransactionReceipt> {
        self.http_client
            .request("eth_getBlockReceipts", rpc_params![block_number_or_hash])
            .await
            .unwrap()
    }

    pub(crate) async fn eth_get_transaction_receipt(
        &self,
        tx_hash: TxHash,
    ) -> Option<ethers_core::types::TransactionReceipt> {
        self.http_client
            .request("eth_getTransactionReceipt", rpc_params![tx_hash])
            .await
            .unwrap()
    }

    pub(crate) async fn eth_get_tx_by_block_hash_and_index(
        &self,
        block_hash: ethereum_types::H256,
        index: ethereum_types::U256,
    ) -> Transaction {
        self.http_client
            .request(
                "eth_getTransactionByBlockHashAndIndex",
                rpc_params![block_hash, index],
            )
            .await
            .unwrap()
    }

    pub(crate) async fn eth_get_tx_by_block_number_and_index(
        &self,
        block_number: BlockNumberOrTag,
        index: ethereum_types::U256,
    ) -> Transaction {
        self.http_client
            .request(
                "eth_getTransactionByBlockNumberAndIndex",
                rpc_params![block_number, index],
            )
            .await
            .unwrap()
    }

    pub(crate) async fn eth_call(
        &self,
        tx: TypedTransaction,
        block_number: Option<BlockNumberOrTag>,
    ) -> Result<Bytes, Box<dyn std::error::Error>> {
        self.http_client
            .request("eth_call", rpc_params![tx, block_number])
            .await
            .map_err(|e| e.into())
    }

    #[allow(dead_code)]
    pub(crate) async fn eth_estimate_gas(
        &self,
        tx: TypedTransaction,
        block_number: Option<BlockNumberOrTag>,
    ) -> u64 {
        let gas: ethereum_types::U64 = self
            .http_client
            .request("eth_estimateGas", rpc_params![tx, block_number])
            .await
            .unwrap();

        gas.as_u64()
    }

    /// params is a tuple of (fromBlock, toBlock, address, topics, blockHash)
    /// any of these params are optional
    pub(crate) async fn eth_get_logs<P>(&self, params: P) -> Vec<LogResponse>
    where
        P: serde::Serialize,
    {
        let rpc_params = rpc_params!(params);
        let eth_logs: Vec<LogResponse> = self
            .http_client
            .request("eth_getLogs", rpc_params)
            .await
            .unwrap();
        eth_logs
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
// ethers version of FeeHistory doesn't accept None reward
pub struct FeeHistory {
    pub base_fee_per_gas: Vec<ethers::types::U256>,
    pub gas_used_ratio: Vec<f64>,
    pub oldest_block: ethers::types::U256,
    pub reward: Option<Vec<Vec<ethers::types::U256>>>,
}
