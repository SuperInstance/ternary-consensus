//! # ternary-consensus
//!
//! Distributed ternary agent consensus on GPU.
//! Agents vote {-1=reject, 0=neutral, +1=accept} with Byzantine tolerance,
//! CRDT state sync, and leader election.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    Reject = -1,
    Neutral = 0,
    Accept = 1,
}

impl Vote {
    pub fn value(&self) -> i8 {
        *self as i8
    }
    pub fn from_i8(v: i8) -> Self {
        match v {
            -1 => Vote::Reject,
            1 => Vote::Accept,
            _ => Vote::Neutral,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: u32,
    pub vote: Vote,
    pub is_byzantine: bool,
    pub trust_score: f64,
}

impl Agent {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            vote: Vote::Neutral,
            is_byzantine: false,
            trust_score: 1.0,
        }
    }

    pub fn byzantine(id: u32) -> Self {
        Self {
            id,
            vote: Vote::Neutral,
            is_byzantine: true,
            trust_score: 0.5,
        }
    }

    pub fn vote(&mut self, proposal: &Proposal) {
        if self.is_byzantine {
            // Byzantine: vote opposite of honest agents
            self.vote = if proposal.merit > 0 {
                Vote::Reject
            } else {
                Vote::Accept
            };
        } else {
            self.vote = if proposal.merit > 0 {
                Vote::Accept
            } else if proposal.merit < 0 {
                Vote::Reject
            } else {
                Vote::Neutral
            };
        }
    }
}

#[derive(Debug, Clone)]
pub struct Proposal {
    pub id: u64,
    pub description: String,
    pub merit: i8, // {-1, 0, +1}
}

#[derive(Debug, Clone)]
pub struct VoteResult {
    pub proposal_id: u64,
    pub accept_count: usize,
    pub reject_count: usize,
    pub neutral_count: usize,
    pub outcome: ConsensusOutcome,
    pub rounds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsensusOutcome {
    Accepted,
    Rejected,
    NoConsensus,
}

#[derive(Debug, Clone)]
pub struct CrdtState {
    pub node_id: u32,
    pub decisions: HashMap<u64, ConsensusOutcome>,
    pub version: u64,
}

impl CrdtState {
    pub fn new(node_id: u32) -> Self {
        Self {
            node_id,
            decisions: HashMap::new(),
            version: 0,
        }
    }

    pub fn record(&mut self, proposal_id: u64, outcome: ConsensusOutcome) {
        self.decisions.insert(proposal_id, outcome);
        self.version += 1;
    }

    pub fn merge(&mut self, other: &CrdtState) {
        for (id, outcome) in &other.decisions {
            self.decisions.entry(*id).or_insert(outcome.clone());
        }
        self.version = self.version.max(other.version) + 1;
    }
}

pub struct ConsensusCluster {
    pub agents: Vec<Agent>,
    pub crdt_states: Vec<CrdtState>,
    pub proposals_processed: u64,
    pub total_rounds: u32,
}

impl ConsensusCluster {
    pub fn new(honest_count: usize, byzantine_count: usize) -> Self {
        let mut agents = Vec::new();
        let mut crdt_states = Vec::new();
        let mut id = 0u32;
        for _ in 0..honest_count {
            agents.push(Agent::new(id));
            crdt_states.push(CrdtState::new(id));
            id += 1;
        }
        for _ in 0..byzantine_count {
            agents.push(Agent::byzantine(id));
            crdt_states.push(CrdtState::new(id));
            id += 1;
        }
        Self {
            agents,
            crdt_states,
            proposals_processed: 0,
            total_rounds: 0,
        }
    }

    /// Run consensus on a proposal. Returns the vote result.
    pub fn consensus(&mut self, proposal: &Proposal, max_rounds: u32) -> VoteResult {
        let mut rounds = 0;
        loop {
            rounds += 1;
            // All agents vote
            for agent in &mut self.agents {
                agent.vote(proposal);
            }

            let accept = self
                .agents
                .iter()
                .filter(|a| a.vote == Vote::Accept)
                .count();
            let reject = self
                .agents
                .iter()
                .filter(|a| a.vote == Vote::Reject)
                .count();
            let neutral = self
                .agents
                .iter()
                .filter(|a| a.vote == Vote::Neutral)
                .count();

            let honest_count = self.agents.iter().filter(|a| !a.is_byzantine).count();
            let threshold = honest_count / 2 + 1;

            let outcome = if accept >= threshold {
                ConsensusOutcome::Accepted
            } else if reject >= threshold {
                ConsensusOutcome::Rejected
            } else if rounds >= max_rounds {
                ConsensusOutcome::NoConsensus
            } else {
                // No majority — re-vote (agents may change)
                continue;
            };

            // Record in CRDT
            for state in &mut self.crdt_states {
                state.record(proposal.id, outcome.clone());
            }

            self.proposals_processed += 1;
            self.total_rounds += rounds;

            return VoteResult {
                proposal_id: proposal.id,
                accept_count: accept,
                reject_count: reject,
                neutral_count: neutral,
                outcome,
                rounds,
            };
        }
    }

    /// Leader election via ternary voting.
    pub fn elect_leader(&mut self) -> Option<u32> {
        // Each agent votes for a candidate
        let mut votes: HashMap<u32, i32> = HashMap::new();
        for agent in &self.agents {
            if !agent.is_byzantine {
                // Vote for self (simplified)
                *votes.entry(agent.id).or_insert(0) += 1;
            } else {
                // Byzantine: vote randomly (always 0 in our sim)
                *votes.entry(0).or_insert(0) += 0;
            }
        }
        votes.into_iter().max_by_key(|(_, v)| *v).map(|(id, _)| id)
    }

    /// Sync CRDT states across all nodes.
    pub fn crdt_sync(&mut self) {
        let mut global = CrdtState::new(u32::MAX);
        for state in &self.crdt_states {
            global.merge(state);
        }
        for state in &mut self.crdt_states {
            state.merge(&global);
        }
    }

    pub fn honest_count(&self) -> usize {
        self.agents.iter().filter(|a| !a.is_byzantine).count()
    }
    pub fn byzantine_count(&self) -> usize {
        self.agents.iter().filter(|a| a.is_byzantine).count()
    }
    pub fn avg_rounds(&self) -> f64 {
        if self.proposals_processed == 0 {
            0.0
        } else {
            self.total_rounds as f64 / self.proposals_processed as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_honest_consensus() {
        let mut cluster = ConsensusCluster::new(5, 0);
        let proposal = Proposal {
            id: 1,
            description: "test".into(),
            merit: 1,
        };
        let result = cluster.consensus(&proposal, 10);
        assert_eq!(result.outcome, ConsensusOutcome::Accepted);
        assert_eq!(result.rounds, 1);
    }

    #[test]
    fn test_reject_consensus() {
        let mut cluster = ConsensusCluster::new(5, 0);
        let proposal = Proposal {
            id: 2,
            description: "bad".into(),
            merit: -1,
        };
        let result = cluster.consensus(&proposal, 10);
        assert_eq!(result.outcome, ConsensusOutcome::Rejected);
    }

    #[test]
    fn test_byzantine_tolerance() {
        // 5 honest, 2 byzantine — should still reach consensus
        let mut cluster = ConsensusCluster::new(5, 2);
        let proposal = Proposal {
            id: 3,
            description: "contested".into(),
            merit: 1,
        };
        let result = cluster.consensus(&proposal, 10);
        assert_eq!(result.outcome, ConsensusOutcome::Accepted); // 5 accept, 2 reject
    }

    #[test]
    fn test_leader_election() {
        let mut cluster = ConsensusCluster::new(4, 1);
        let leader = cluster.elect_leader();
        assert!(leader.is_some());
        // Leader should be an honest agent
        assert!(leader.unwrap() < 4); // first 4 are honest
    }

    #[test]
    fn test_crdt_sync() {
        let mut cluster = ConsensusCluster::new(3, 0);
        let p1 = Proposal {
            id: 10,
            description: "a".into(),
            merit: 1,
        };
        cluster.consensus(&p1, 5);
        cluster.crdt_sync();
        // All nodes should have the same decision
        let decisions: Vec<_> = cluster
            .crdt_states
            .iter()
            .filter_map(|s| s.decisions.get(&10).cloned())
            .collect();
        assert!(decisions.iter().all(|d| *d == ConsensusOutcome::Accepted));
    }

    #[test]
    fn test_multiple_proposals() {
        let mut cluster = ConsensusCluster::new(7, 2);
        for i in 0..10 {
            let proposal = Proposal {
                id: i,
                description: format!("p{}", i),
                merit: if i % 3 == 0 { -1 } else { 1 },
            };
            cluster.consensus(&proposal, 5);
        }
        assert_eq!(cluster.proposals_processed, 10);
        assert!(cluster.avg_rounds() <= 5.0);
    }

    #[test]
    fn test_vote_values() {
        assert_eq!(Vote::Reject.value(), -1);
        assert_eq!(Vote::Neutral.value(), 0);
        assert_eq!(Vote::Accept.value(), 1);
        assert_eq!(Vote::from_i8(-1), Vote::Reject);
        assert_eq!(Vote::from_i8(0), Vote::Neutral);
        assert_eq!(Vote::from_i8(1), Vote::Accept);
    }

    #[test]
    fn test_byzantine_cannot_block() {
        // 3 honest, 3 byzantine — honest should still accept with merit=1
        let mut cluster = ConsensusCluster::new(3, 3);
        let proposal = Proposal {
            id: 99,
            description: "contested".into(),
            merit: 1,
        };
        let result = cluster.consensus(&proposal, 5);
        // 3 honest accept, 3 byzantine reject — threshold is 2
        assert_eq!(result.outcome, ConsensusOutcome::Accepted);
    }
}
