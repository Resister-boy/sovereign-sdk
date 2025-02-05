use std::net::SocketAddr;

use borsh::BorshSerialize;
use demo_stf::app::DefaultPrivateKey;
use demo_stf::genesis_config::GenesisPaths;
use demo_stf::runtime::RuntimeCall;
use jsonrpsee::core::client::{Subscription, SubscriptionClientT};
use jsonrpsee::rpc_params;
use methods::MOCK_DA_ELF;
use sov_demo_rollup::DemoProverConfig;
use sov_modules_api::default_context::DefaultContext;
use sov_modules_api::transaction::Transaction;
use sov_modules_api::{PrivateKey, Spec};
use sov_risc0_adapter::host::Risc0Host;
use sov_rollup_interface::mocks::MockDaSpec;
use sov_sequencer::utils::SimpleClient;

use super::test_helpers::start_rollup;
const TOKEN_SALT: u64 = 0;
const TOKEN_NAME: &str = "test_token";

const TEST_GENESIS_PATHS: GenesisPaths<&str> = GenesisPaths {
    bank_genesis_path: "../test-data/genesis/integration-tests/bank.json",
    sequencer_genesis_path: "../test-data/genesis/integration-tests/sequencer_registry.json",
    value_setter_genesis_path: "../test-data/genesis/integration-tests/value_setter.json",
    accounts_genesis_path: "../test-data/genesis/integration-tests/accounts.json",
    chain_state_genesis_path: "../test-data/genesis/integration-tests/chain_state.json",
    #[cfg(feature = "experimental")]
    evm_genesis_path: "../test-data/genesis/integration-tests/evm.json",
};

async fn send_test_create_token_tx(rpc_address: SocketAddr) -> Result<(), anyhow::Error> {
    let key = DefaultPrivateKey::generate();
    let user_address: <DefaultContext as Spec>::Address = key.to_address();

    let token_address = sov_bank::get_token_address::<DefaultContext>(
        TOKEN_NAME,
        user_address.as_ref(),
        TOKEN_SALT,
    );

    let msg = RuntimeCall::<DefaultContext, MockDaSpec>::bank(sov_bank::CallMessage::<
        DefaultContext,
    >::CreateToken {
        salt: TOKEN_SALT,
        token_name: TOKEN_NAME.to_string(),
        initial_balance: 1000,
        minter_address: user_address,
        authorized_minters: vec![],
    });
    let tx = Transaction::<DefaultContext>::new_signed_tx(&key, msg.try_to_vec().unwrap(), 0);

    let port = rpc_address.port();
    let client = SimpleClient::new("localhost", port).await?;

    let mut slot_processed_subscription: Subscription<u64> = client
        .ws()
        .subscribe(
            "ledger_subscribeSlots",
            rpc_params![],
            "ledger_unsubscribeSlots",
        )
        .await?;

    client.send_transaction(tx).await?;

    // Wait until the rollup has processed the next slot
    let _ = slot_processed_subscription.next().await;

    let balance_response = sov_bank::BankRpcClient::<DefaultContext>::balance_of(
        client.http(),
        user_address,
        token_address,
    )
    .await?;
    assert_eq!(balance_response.amount.unwrap_or_default(), 1000);
    Ok(())
}

#[tokio::test]
async fn bank_tx_tests() -> Result<(), anyhow::Error> {
    let (port_tx, port_rx) = tokio::sync::oneshot::channel();

    let prover = Risc0Host::new(MOCK_DA_ELF);
    let config = DemoProverConfig::Execute;

    let rollup_task = tokio::spawn(async {
        start_rollup(port_tx, Some((prover, config)), &TEST_GENESIS_PATHS).await;
    });

    // Wait for rollup task to start:
    let port = port_rx.await.unwrap();

    // If the rollup throws an error, return it and stop trying to send the transaction
    tokio::select! {
        err = rollup_task => err?,
        res = send_test_create_token_tx(port) => res?,
    };
    Ok(())
}
