// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    epoch_manager::EpochManager,
    network::NetworkTask,
    network_interface::{ConsensusNetworkEvents, ConsensusNetworkSender},
    persistent_liveness_storage::StorageWriteProxy,
    state_computer::ExecutionProxy,
    txn_manager::MempoolProxy,
    util::time_service::ClockTimeService,
};
use aptos_config::config::NodeConfig;
use aptos_logger::prelude::*;
use aptos_mempool::ConsensusRequest;
use consensus_notifications::ConsensusNotificationSender;
use event_notifications::ReconfigNotificationListener;
use execution_correctness::ExecutionCorrectnessManager;
use futures::channel::mpsc;
use network::application::storage::PeerMetadataStorage;
use std::sync::Arc;
use storage_interface::DbReaderWriter;
use tokio::runtime::{self, Runtime};

/// Helper function to start consensus based on configuration and return the runtime
pub fn start_consensus(
    node_config: &NodeConfig,
    mut network_sender: ConsensusNetworkSender,
    network_events: ConsensusNetworkEvents,
    state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
    consensus_to_mempool_sender: mpsc::Sender<ConsensusRequest>,
    aptos_db: DbReaderWriter,
    reconfig_events: ReconfigNotificationListener,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
) -> Runtime {
    let runtime = runtime::Builder::new_multi_thread()
        .thread_name("consensus")
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime!");
    let storage = Arc::new(StorageWriteProxy::new(node_config, aptos_db.reader.clone()));
    let txn_manager = Arc::new(MempoolProxy::new(
        consensus_to_mempool_sender,
        node_config.consensus.mempool_poll_count,
        node_config.consensus.mempool_txn_pull_timeout_ms,
        node_config.consensus.mempool_executed_txn_timeout_ms,
    ));
    let execution_correctness_manager = ExecutionCorrectnessManager::new(node_config, aptos_db);

    let state_computer = Arc::new(ExecutionProxy::new(
        execution_correctness_manager.client(),
        txn_manager.clone(),
        state_sync_notifier,
        runtime.handle(),
    ));

    let time_service = Arc::new(ClockTimeService::new(runtime.handle().clone()));

    let (timeout_sender, timeout_receiver) = channel::new(1_024, &counters::PENDING_ROUND_TIMEOUTS);
    let (self_sender, self_receiver) = channel::new(1_024, &counters::PENDING_SELF_MESSAGES);
    network_sender.initialize(peer_metadata_storage);

    let epoch_mgr = EpochManager::new(
        node_config,
        time_service,
        self_sender,
        network_sender,
        timeout_sender,
        txn_manager,
        state_computer,
        storage,
        reconfig_events,
    );

    let (network_task, network_receiver) = NetworkTask::new(network_events, self_receiver);

    runtime.spawn(network_task.start());
    runtime.spawn(epoch_mgr.start(timeout_receiver, network_receiver));

    debug!("Consensus started.");
    runtime
}
