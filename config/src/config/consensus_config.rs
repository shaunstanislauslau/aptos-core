// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::config::SafetyRulesConfig;
use aptos_types::{account_address::AccountAddress, block_info::Round};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ConsensusConfig {
    pub contiguous_rounds: u32,
    pub max_block_size: u64,
    pub max_pruned_blocks_in_mem: usize,
    // Timeout for consensus to get an ack from mempool for executed transactions (in milliseconds)
    pub mempool_executed_txn_timeout_ms: u64,
    // Timeout for consensus to pull transactions from mempool and get a response (in milliseconds)
    pub mempool_txn_pull_timeout_ms: u64,
    pub round_initial_timeout_ms: u64,
    pub proposer_type: ConsensusProposerType,
    pub safety_rules: SafetyRulesConfig,
    // Only sync committed transactions but not vote for any pending blocks. This is useful when
    // validators coordinate on the latest version to apply a manual transaction.
    pub sync_only: bool,
    // Decides how long the leader waits before proposing empty block if there's no txns in mempool
    // the period = (poll_count - 1) * 30ms
    pub mempool_poll_count: u64,
    pub channel_size: usize,
}

impl Default for ConsensusConfig {
    fn default() -> ConsensusConfig {
        ConsensusConfig {
            contiguous_rounds: 2,
            max_block_size: 3000,
            max_pruned_blocks_in_mem: 100,
            mempool_txn_pull_timeout_ms: 1000,
            mempool_executed_txn_timeout_ms: 1000,
            round_initial_timeout_ms: 1000,
            proposer_type: ConsensusProposerType::LeaderReputation(LeaderReputationConfig {
                active_weights: 99,
                inactive_weights: 1,
            }),
            safety_rules: SafetyRulesConfig::default(),
            sync_only: false,
            mempool_poll_count: 20,
            channel_size: 30, // hard-coded
        }
    }
}

impl ConsensusConfig {
    pub fn set_data_dir(&mut self, data_dir: PathBuf) {
        self.safety_rules.set_data_dir(data_dir);
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ConsensusProposerType {
    // Choose the smallest PeerId as the proposer
    FixedProposer,
    // Round robin rotation of proposers
    RotatingProposer,
    // Committed history based proposer election
    LeaderReputation(LeaderReputationConfig),
    // Pre-specified proposers for each round,
    // or default proposer if round proposer not
    // specified
    RoundProposer(HashMap<Round, AccountAddress>),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LeaderReputationConfig {
    pub active_weights: u64,
    pub inactive_weights: u64,
}
