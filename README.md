# Ternary Consensus — Byzantine-Tolerant Distributed Voting with CRDT State Sync

**Ternary Consensus** implements distributed agreement among ternary agents using three-valued votes: {-1=reject, 0=neutral, +1=accept}. It provides Byzantine fault tolerance, CRDT-based state synchronization across nodes, and leader election — all built on the ternary alphabet where every decision reduces to one of three states.

## Why It Matters

Byzantine agreement is the cornerstone of reliable distributed systems. Classical protocols like PBFT use binary accept/reject, forcing neutral agents to commit to one side. Ternary voting adds the crucial **neutral** option: agents can abstain without blocking consensus, more accurately modeling real-world preferences. This is particularly valuable in fleet GPU management, where nodes may be uncertain about workload suitability but shouldn't veto proposals outright. The CRDT state layer ensures that consensus decisions propagate reliably even under network partitions, since CRDT merges are commutative, associative, and idempotent by construction.

## How It Works

### Voting Protocol

Each agent votes on a `Proposal` (which carries a ternary `merit` value in {-1, 0, +1}). Honest agents vote according to the proposal's merit; Byzantine agents vote opposite to disrupt. The `VoteResult` tallies accept/reject/neutral counts and determines a `ConsensusOutcome` (Accepted, Rejected, NoConsensus) over potentially multiple rounds.

### Byzantine Tolerance

Following the Lamport-Lamport-Shostak bound, the system tolerates f Byzantine agents with n ≥ 3f + 1 total agents. Byzantine agents are modeled with reduced `trust_score` (0.5 vs 1.0 for honest). The consensus threshold typically requires >⅔ non-reject votes for acceptance.

### CRDT State Replication

`CrdtState` maintains a per-node HashMap of proposal outcomes with a monotonically increasing version counter. When nodes sync, they merge by taking the latest version for each proposal — this is a Last-Writer-Wins (LWW) map, a well-known state-based CRDT. Merge complexity is O(k) for k decisions.

### Leader Election

Leaders are selected based on accumulated trust scores. An agent's trust score increases with successful consensus participation and decreases when detected as Byzantine. This provides Sybil resistance proportional to the trust bootstrap.

## Quick Start

```rust
use ternary_consensus::{Agent, Proposal, Vote, CrdtState};

let mut agents = vec![
    Agent::new(1),
    Agent::new(2),
    Agent::new(3),
    Agent::byzantine(4), // Byzantine agent
];

let proposal = Proposal {
    id: 1,
    description: "Deploy model v2".into(),
    merit: 1, // +1 = positive merit
};

// Each agent votes
for agent in &mut agents {
    agent.vote(&proposal);
}

// Tally votes
let accepts = agents.iter().filter(|a| a.vote == Vote::Accept).count();
let rejects = agents.iter().filter(|a| a.vote == Vote::Reject).count();
```

```bash
cargo add ternary-consensus
```

## API

| Type / Function | Description |
|---|---|
| `Vote` | `Reject(-1)`, `Neutral(0)`, `Accept(1)` |
| `Agent` | `{ id, vote, is_byzantine, trust_score }` with `vote(&Proposal)` |
| `Proposal` | `{ id, description, merit: i8 }` |
| `VoteResult` | Tallies + `ConsensusOutcome` |
| `CrdtState` | LWW-map CRDT for decision propagation |

## Architecture Notes

This is the governance layer of **SuperInstance**. Fleet-wide decisions — model deployment, weight updates, resource allocation — flow through ternary consensus. The three-valued vote maps to the γ (growth = accept), η (entropy = reject), and neutral states of γ + η = C. The CRDT layer ensures decisions survive network partitions. See [Architecture](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md).

## References

- Lamport, Leslie; Shostak, Robert; Pease, Marshall. "The Byzantine Generals Problem," *ACM TOPLAS*, 4(3), 1982.
- Shapiro, Marc et al. "A Comprehensive Study of Convergent and Commutative Replicated Data Types," *INRIA RR-7506*, 2011 — CRDT foundations.
- Castro, Miguel & Liskov, Barbara. "Practical Byzantine Fault Tolerance," *OSDI*, 1999 — PBFT protocol.

## License

MIT
