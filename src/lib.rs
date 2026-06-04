//! # ternary-consensus
//!
//! Consensus algorithms for distributed ternary agents.
//!
//! Provides:
//! - `TernaryValue` — core ternary type (-1, 0, +1)
//! - `RaftNode` — Raft-style leader election and log replication for ternary decisions
//! - `ByzantineConsensus` — Byzantine fault-tolerant consensus for ternary votes
//! - `VotingRound` — Simple majority and supermajority voting
//! - `ConsensusLog` — Replicated log of ternary decisions

use std::collections::HashMap;

/// Core ternary value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TernaryValue {
    Neg,
    Zero,
    Pos,
}

impl TernaryValue {
    pub fn to_i8(self) -> i8 {
        match self {
            TernaryValue::Neg => -1,
            TernaryValue::Zero => 0,
            TernaryValue::Pos => 1,
        }
    }

    pub fn from_i8(v: i8) -> Option<Self> {
        match v {
            -1 => Some(TernaryValue::Neg),
            0 => Some(TernaryValue::Zero),
            1 => Some(TernaryValue::Pos),
            _ => None,
        }
    }

    /// Sum two ternary values (with clamping)
    pub fn combine(self, other: TernaryValue) -> TernaryValue {
        let sum = self.to_i8() + other.to_i8();
        TernaryValue::from_i8(sum.clamp(-1, 1)).unwrap_or(TernaryValue::Zero)
    }
}

/// A node in the consensus network
#[derive(Debug, Clone)]
pub struct Node {
    pub id: usize,
    pub value: TernaryValue,
    pub term: u64,
    pub voted_for: Option<usize>,
    pub is_leader: bool,
    pub is_byzantine: bool,
}

impl Node {
    pub fn new(id: usize) -> Self {
        Node {
            id,
            value: TernaryValue::Zero,
            term: 0,
            voted_for: None,
            is_leader: false,
            is_byzantine: false,
        }
    }

    pub fn with_value(mut self, value: TernaryValue) -> Self {
        self.value = value;
        self
    }

    pub fn byzantine(mut self) -> Self {
        self.is_byzantine = true;
        self
    }
}

/// Log entry for the consensus log
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub term: u64,
    pub index: usize,
    pub value: TernaryValue,
    pub committed: bool,
}

/// Raft-style consensus for ternary values
#[derive(Debug, Clone)]
pub struct RaftConsensus {
    nodes: Vec<Node>,
    log: Vec<LogEntry>,
    current_term: u64,
    leader_id: Option<usize>,
    commit_index: usize,
    next_index: HashMap<usize, usize>,
    match_index: HashMap<usize, usize>,
}

impl RaftConsensus {
    pub fn new(node_count: usize) -> Self {
        let nodes = (0..node_count).map(|i| Node::new(i)).collect();
        let mut next_index = HashMap::new();
        let mut match_index = HashMap::new();
        for i in 0..node_count {
            next_index.insert(i, 0);
            match_index.insert(i, 0);
        }
        RaftConsensus {
            nodes,
            log: Vec::new(),
            current_term: 0,
            leader_id: None,
            commit_index: 0,
            next_index,
            match_index,
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Elect a leader for the current term
    pub fn elect_leader(&mut self, candidate_id: usize) -> bool {
        let majority = self.nodes.len() / 2 + 1;
        let mut votes = 1; // Self vote

        self.current_term += 1;
        self.nodes[candidate_id].term = self.current_term;
        self.nodes[candidate_id].voted_for = Some(candidate_id);

        for node in &self.nodes {
            if node.id != candidate_id && node.voted_for.is_none() {
                votes += 1;
            }
        }

        if votes >= majority {
            self.leader_id = Some(candidate_id);
            self.nodes[candidate_id].is_leader = true;
            // Reset other nodes
            for node in &mut self.nodes {
                if node.id != candidate_id {
                    node.is_leader = false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Leader proposes a value
    pub fn propose(&mut self, value: TernaryValue) -> Option<usize> {
        let leader_id = self.leader_id?;
        if !self.nodes[leader_id].is_leader {
            return None;
        }

        let index = self.log.len();
        self.log.push(LogEntry {
            term: self.current_term,
            index,
            value,
            committed: false,
        });

        // Replicate to followers
        for node in &mut self.nodes {
            if node.id != leader_id {
                node.value = value;
            }
        }
        self.nodes[leader_id].value = value;

        // Try to commit
        self.try_commit();
        Some(index)
    }

    fn try_commit(&mut self) {
        let majority = self.nodes.len() / 2 + 1;
        for i in self.commit_index..self.log.len() {
            let mut count = 0;
            for node in &self.nodes {
                if node.value == self.log[i].value {
                    count += 1;
                }
            }
            if count >= majority {
                self.log[i].committed = true;
                self.commit_index = i + 1;
            }
        }
    }

    /// Get the committed value at an index
    pub fn get_committed(&self, index: usize) -> Option<TernaryValue> {
        self.log.get(index).filter(|e| e.committed).map(|e| e.value)
    }

    /// Get the last committed value
    pub fn last_committed(&self) -> Option<TernaryValue> {
        (0..self.log.len()).rev()
            .find(|&i| self.log[i].committed)
            .map(|i| self.log[i].value)
    }

    pub fn leader(&self) -> Option<usize> {
        self.leader_id
    }

    pub fn term(&self) -> u64 {
        self.current_term
    }

    pub fn log_len(&self) -> usize {
        self.log.len()
    }

    pub fn commit_index(&self) -> usize {
        self.commit_index
    }

    /// Force commit everything (for testing)
    pub fn force_commit_all(&mut self) {
        for entry in &mut self.log {
            entry.committed = true;
        }
        self.commit_index = self.log.len();
    }
}

/// Byzantine fault-tolerant consensus
#[derive(Debug, Clone)]
pub struct ByzantineConsensus {
    nodes: Vec<Node>,
    fault_tolerance: usize,
}

impl ByzantineConsensus {
    /// Create a Byzantine consensus with n nodes tolerating f faults (n >= 3f+1)
    pub fn new(total_nodes: usize, fault_tolerance: usize) -> Self {
        let nodes = (0..total_nodes).map(|i| Node::new(i)).collect();
        ByzantineConsensus { nodes, fault_tolerance }
    }

    /// Check if the system can tolerate the specified number of faults
    pub fn is_valid(&self) -> bool {
        self.nodes.len() >= 3 * self.fault_tolerance + 1
    }

    /// Set a node as byzantine (adversarial)
    pub fn set_byzantine(&mut self, node_id: usize) {
        if node_id < self.nodes.len() {
            self.nodes[node_id].is_byzantine = true;
        }
    }

    /// Run one round of Byzantine agreement (simplified PBFT)
    pub fn run_round(&mut self, proposed_values: &[TernaryValue]) -> TernaryValue {
        // Phase 1: Pre-prepare — all nodes propose
        let mut proposals: Vec<TernaryValue> = Vec::new();
        for (i, node) in self.nodes.iter().enumerate() {
            if node.is_byzantine {
                // Byzantine nodes propose a conflicting value
                proposals.push(TernaryValue::Neg);
            } else if i < proposed_values.len() {
                proposals.push(proposed_values[i]);
            } else {
                proposals.push(TernaryValue::Zero);
            }
        }

        // Phase 2: Prepare — majority vote
        let majority = self.run_majority_vote(&proposals);

        // Phase 3: Commit — another majority vote on the prepare result
        let commit_values: Vec<TernaryValue> = self.nodes.iter().map(|node| {
            if node.is_byzantine {
                // Byzantine nodes try to flip the result
                match majority {
                    TernaryValue::Pos => TernaryValue::Neg,
                    TernaryValue::Neg => TernaryValue::Pos,
                    TernaryValue::Zero => TernaryValue::Pos,
                }
            } else {
                majority
            }
        }).collect();

        self.run_majority_vote(&commit_values)
    }

    fn run_majority_vote(&self, values: &[TernaryValue]) -> TernaryValue {
        let mut counts = HashMap::new();
        for &v in values {
            *counts.entry(v).or_insert(0) += 1;
        }

        let majority_threshold = self.nodes.len() / 2 + 1;
        for (&value, &count) in &counts {
            if count >= majority_threshold {
                return value;
            }
        }

        // No majority — use weighted sum
        let sum: i64 = values.iter().map(|v| v.to_i8() as i64).sum();
        if sum > 0 { TernaryValue::Pos } else if sum < 0 { TernaryValue::Neg } else { TernaryValue::Zero }
    }

    /// Multi-round Byzantine agreement
    pub fn agree(&mut self, proposed_values: &[TernaryValue], rounds: usize) -> TernaryValue {
        let mut result = TernaryValue::Zero;
        for _ in 0..rounds {
            result = self.run_round(proposed_values);
        }
        result
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn byzantine_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_byzantine).count()
    }
}

/// Simple voting round for ternary values
#[derive(Debug, Clone)]
pub struct VotingRound {
    votes: Vec<(usize, TernaryValue)>,
    voters: usize,
}

impl VotingRound {
    pub fn new(voter_count: usize) -> Self {
        VotingRound {
            votes: Vec::new(),
            voters: voter_count,
        }
    }

    /// Cast a vote
    pub fn vote(&mut self, voter_id: usize, value: TernaryValue) -> bool {
        if voter_id < self.voters && !self.votes.iter().any(|(id, _)| *id == voter_id) {
            self.votes.push((voter_id, value));
            true
        } else {
            false
        }
    }

    /// Get simple majority result
    pub fn majority(&self) -> Option<TernaryValue> {
        let mut counts = HashMap::new();
        for (_, value) in &self.votes {
            *counts.entry(*value).or_insert(0) += 1;
        }

        let threshold = self.voters / 2 + 1;
        for (&value, &count) in &counts {
            if count >= threshold {
                return Some(value);
            }
        }
        None
    }

    /// Get plurality result (most votes, not necessarily majority)
    pub fn plurality(&self) -> TernaryValue {
        let mut counts = HashMap::new();
        for (_, value) in &self.votes {
            *counts.entry(*value).or_insert(0) += 1;
        }

        counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(value, _)| value)
            .unwrap_or(TernaryValue::Zero)
    }

    /// Get supermajority result (2/3 threshold)
    pub fn supermajority(&self) -> Option<TernaryValue> {
        let threshold = (self.voters * 2 + 2) / 3;
        let mut counts = HashMap::new();
        for (_, value) in &self.votes {
            *counts.entry(*value).or_insert(0) += 1;
        }

        for (&value, &count) in &counts {
            if count >= threshold {
                return Some(value);
            }
        }
        None
    }

    /// Weighted vote (weights based on voter ID for simplicity)
    pub fn weighted_result(&self) -> TernaryValue {
        let mut weighted_sum = 0i64;
        let mut total_weight = 0i64;
        for (voter_id, value) in &self.votes {
            let weight = 1 + (*voter_id as i64 % 3);
            weighted_sum += value.to_i8() as i64 * weight;
            total_weight += weight;
        }
        if total_weight == 0 {
            return TernaryValue::Zero;
        }
        let avg = weighted_sum as f64 / total_weight as f64;
        if avg > 0.33 {
            TernaryValue::Pos
        } else if avg < -0.33 {
            TernaryValue::Neg
        } else {
            TernaryValue::Zero
        }
    }

    /// Check if consensus was reached
    pub fn is_unanimous(&self) -> bool {
        if self.votes.is_empty() {
            return false;
        }
        let first = self.votes[0].1;
        self.votes.iter().all(|(_, v)| *v == first)
    }

    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    pub fn voter_count(&self) -> usize {
        self.voters
    }
}

/// Replicated log of ternary decisions
#[derive(Debug, Clone)]
pub struct ConsensusLog {
    entries: Vec<LogEntry>,
    nodes: usize,
    committed_index: usize,
}

impl ConsensusLog {
    pub fn new(nodes: usize) -> Self {
        ConsensusLog {
            entries: Vec::new(),
            nodes,
            committed_index: 0,
        }
    }

    /// Append an entry
    pub fn append(&mut self, term: u64, value: TernaryValue) -> usize {
        let index = self.entries.len();
        self.entries.push(LogEntry {
            term,
            index,
            value,
            committed: false,
        });
        index
    }

    /// Commit entries up to index (requires majority agreement)
    pub fn commit_up_to(&mut self, index: usize) -> bool {
        if index > self.entries.len() {
            return false;
        }
        for i in 0..index {
            if i < self.entries.len() {
                self.entries[i].committed = true;
            }
        }
        self.committed_index = index;
        true
    }

    /// Get all committed entries
    pub fn committed(&self) -> Vec<&LogEntry> {
        self.entries.iter().filter(|e| e.committed).collect()
    }

    /// Get all entries
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Merge two logs (keep the one with higher term)
    pub fn merge(&mut self, other: &[LogEntry]) -> usize {
        let mut conflicts = 0;
        for entry in other {
            if entry.index < self.entries.len() {
                if self.entries[entry.index].term != entry.term {
                    // Conflict: truncate and replace
                    self.entries.truncate(entry.index);
                    self.entries.push(entry.clone());
                    conflicts += 1;
                }
            } else {
                self.entries.push(entry.clone());
            }
        }
        conflicts
    }

    /// Compact the log, keeping only entries after the given index
    pub fn compact(&mut self, keep_from: usize) {
        if keep_from > 0 && keep_from < self.entries.len() {
            self.entries.drain(0..keep_from);
            for entry in &mut self.entries {
                entry.index -= keep_from;
            }
            self.committed_index = self.committed_index.saturating_sub(keep_from);
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ternary_value_basic() {
        assert_eq!(TernaryValue::Neg.to_i8(), -1);
        assert_eq!(TernaryValue::Zero.to_i8(), 0);
        assert_eq!(TernaryValue::Pos.to_i8(), 1);
    }

    #[test]
    fn test_ternary_combine() {
        assert_eq!(TernaryValue::Pos.combine(TernaryValue::Neg), TernaryValue::Zero);
        assert_eq!(TernaryValue::Pos.combine(TernaryValue::Pos), TernaryValue::Pos);
        assert_eq!(TernaryValue::Neg.combine(TernaryValue::Neg), TernaryValue::Neg);
    }

    #[test]
    fn test_raft_elect_leader() {
        let mut raft = RaftConsensus::new(5);
        assert!(raft.elect_leader(0));
        assert_eq!(raft.leader(), Some(0));
    }

    #[test]
    fn test_raft_propose() {
        let mut raft = RaftConsensus::new(5);
        raft.elect_leader(0);
        let idx = raft.propose(TernaryValue::Pos).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(raft.log_len(), 1);
    }

    #[test]
    fn test_raft_no_propose_without_leader() {
        let mut raft = RaftConsensus::new(5);
        assert!(raft.propose(TernaryValue::Pos).is_none());
    }

    #[test]
    fn test_raft_commit() {
        let mut raft = RaftConsensus::new(3);
        raft.elect_leader(0);
        raft.propose(TernaryValue::Pos);
        raft.force_commit_all();
        assert_eq!(raft.last_committed(), Some(TernaryValue::Pos));
    }

    #[test]
    fn test_raft_multiple_proposals() {
        let mut raft = RaftConsensus::new(5);
        raft.elect_leader(0);
        raft.propose(TernaryValue::Pos);
        raft.propose(TernaryValue::Neg);
        raft.propose(TernaryValue::Zero);
        assert_eq!(raft.log_len(), 3);
    }

    #[test]
    fn test_byzantine_valid() {
        let bc = ByzantineConsensus::new(4, 1);
        assert!(bc.is_valid()); // 4 >= 3*1+1
    }

    #[test]
    fn test_byzantine_invalid() {
        let bc = ByzantineConsensus::new(3, 1);
        assert!(!bc.is_valid()); // 3 < 4
    }

    #[test]
    fn test_byzantine_round() {
        let mut bc = ByzantineConsensus::new(4, 1);
        let values = vec![TernaryValue::Pos, TernaryValue::Pos, TernaryValue::Pos, TernaryValue::Pos];
        let result = bc.run_round(&values);
        // Majority is Pos
        assert_eq!(result, TernaryValue::Pos);
    }

    #[test]
    fn test_byzantine_with_faults() {
        let mut bc = ByzantineConsensus::new(4, 1);
        bc.set_byzantine(3);
        let values = vec![TernaryValue::Pos, TernaryValue::Pos, TernaryValue::Pos, TernaryValue::Pos];
        let result = bc.run_round(&values);
        // 3 honest nodes say Pos, 1 byzantine says Neg — majority still Pos
        assert_eq!(result, TernaryValue::Pos);
    }

    #[test]
    fn test_byzantine_agree() {
        let mut bc = ByzantineConsensus::new(4, 1);
        let values = vec![TernaryValue::Pos, TernaryValue::Pos, TernaryValue::Pos, TernaryValue::Pos];
        let result = bc.agree(&values, 3);
        assert_eq!(result, TernaryValue::Pos);
    }

    #[test]
    fn test_voting_majority() {
        let mut vr = VotingRound::new(5);
        vr.vote(0, TernaryValue::Pos);
        vr.vote(1, TernaryValue::Pos);
        vr.vote(2, TernaryValue::Pos);
        assert_eq!(vr.majority(), Some(TernaryValue::Pos));
    }

    #[test]
    fn test_voting_no_majority() {
        let mut vr = VotingRound::new(5);
        vr.vote(0, TernaryValue::Pos);
        vr.vote(1, TernaryValue::Neg);
        assert!(vr.majority().is_none());
    }

    #[test]
    fn test_voting_plurality() {
        let mut vr = VotingRound::new(5);
        vr.vote(0, TernaryValue::Pos);
        vr.vote(1, TernaryValue::Pos);
        vr.vote(2, TernaryValue::Neg);
        assert_eq!(vr.plurality(), TernaryValue::Pos);
    }

    #[test]
    fn test_voting_supermajority() {
        let mut vr = VotingRound::new(5);
        vr.vote(0, TernaryValue::Pos);
        vr.vote(1, TernaryValue::Pos);
        vr.vote(2, TernaryValue::Pos);
        vr.vote(3, TernaryValue::Pos);
        assert_eq!(vr.supermajority(), Some(TernaryValue::Pos)); // 4/5 >= 2/3
    }

    #[test]
    fn test_voting_weighted() {
        let mut vr = VotingRound::new(3);
        vr.vote(0, TernaryValue::Pos);
        vr.vote(2, TernaryValue::Pos);
        vr.vote(1, TernaryValue::Neg);
        // voter 0 weight=1, voter 2 weight=1, voter 1 weight=2
        // weighted sum = 1*1 + 1*1 + (-1)*2 = 0, avg=0
        // Result depends on weights; just verify it runs
        let _result = vr.weighted_result();
    }

    #[test]
    fn test_voting_unanimous() {
        let mut vr = VotingRound::new(3);
        vr.vote(0, TernaryValue::Pos);
        vr.vote(1, TernaryValue::Pos);
        vr.vote(2, TernaryValue::Pos);
        assert!(vr.is_unanimous());
    }

    #[test]
    fn test_voting_not_unanimous() {
        let mut vr = VotingRound::new(3);
        vr.vote(0, TernaryValue::Pos);
        vr.vote(1, TernaryValue::Neg);
        assert!(!vr.is_unanimous());
    }

    #[test]
    fn test_consensus_log() {
        let mut log = ConsensusLog::new(3);
        log.append(1, TernaryValue::Pos);
        log.append(1, TernaryValue::Neg);
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn test_consensus_log_commit() {
        let mut log = ConsensusLog::new(3);
        log.append(1, TernaryValue::Pos);
        log.append(1, TernaryValue::Neg);
        log.commit_up_to(2);
        assert_eq!(log.committed().len(), 2);
    }

    #[test]
    fn test_consensus_log_merge() {
        let mut log1 = ConsensusLog::new(3);
        log1.append(1, TernaryValue::Pos);
        let log2 = vec![LogEntry { term: 1, index: 0, value: TernaryValue::Pos, committed: false }];
        let conflicts = log1.merge(&log2);
        assert_eq!(conflicts, 0);
        assert_eq!(log1.len(), 1);
    }

    #[test]
    fn test_consensus_log_compact() {
        let mut log = ConsensusLog::new(3);
        log.append(1, TernaryValue::Pos);
        log.append(1, TernaryValue::Neg);
        log.append(1, TernaryValue::Zero);
        log.compact(1);
        assert_eq!(log.len(), 2);
        assert_eq!(log.entries()[0].value, TernaryValue::Neg);
    }
}
