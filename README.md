# Ternary Consensus — Byzantine-Tolerant Distributed Voting with CRDT State Sync

**Ternary Consensus** implements distributed agreement among ternary agents using three-valued votes: {-1=reject, 0=neutral, +1=accept}. It provides Byzantine fault tolerance, CRDT-based state synchronization across nodes, and leader election — all built on the ternary alphabet where every decision reduces to one of three states.

## Why It Matters

Byzantine agreement is the cornerstone of reliable distributed systems. Classical protocols like PBFT use binary accept/reject, forcing neutral agents to commit to one side. Ternary voting adds the crucial **neutral** option: agents can abstain without blocking consensus, more accurately modeling real-world preferences. This is particularly valuable in fleet GPU management, where nodes may be uncertain about workload suitability but shouldn't veto proposals outright. The CRDT state layer ensures that consensus decisions propagate reliably even under network partitions, since CRDT merges are commutative, associative, and idempotent by construction.

## How It Works

### Voting Protocol

Each agent votes on a `Proposal` (which carries a ternary `merit` value in {-1, 0, +1}). Honest agents vote according to the proposal's merit; Byzantine agents vote opposite to disrupt. The `VoteResult` tallies accept/reject/neutral counts and determines a `ConsensusOutcome` (Accepted, Rejected, NoConsensus) over potentially multiple rounds.

### Byzantine Tolerance

Byzantine agents are modeled with a reduced `trust_score` (0.5 vs 1.0 for honest) and vote opposite to the proposal's merit. In the current implementation, a proposal is accepted when accept votes reach a simple majority of honest agents (`⌊honest / 2⌋ + 1`), and rejected when reject votes reach the same threshold; otherwise the round re-votes up to a caller-provided limit and falls back to `NoConsensus`. The classical Lamport–Shostak–Pease bound (n ≥ 3f + 1 tolerating f Byzantine faults) is the target resilience model; strict BFT quorums (e.g. a >⅔ acceptance threshold) are not yet enforced (see [Status](#status)).

### CRDT State Replication

`CrdtState` stores a per-node `HashMap` of proposal outcomes plus a monotonically increasing version counter. `merge` takes the union of decisions and, on a key collision, keeps the existing entry (first-writer-wins) — i.e. an add-only/state-union CRDT. Full Last-Writer-Wins (LWW) semantics with per-decision versioning is planned but not yet implemented. Merge complexity is O(k) for k decisions.

### Leader Election

`elect_leader` performs plurality voting among honest agents (each honest agent votes for itself); the candidate with the most votes wins, with ties broken by iteration order. `trust_score` is tracked per agent but is not yet consulted by the election — trust-weighted leader election is planned (see [Status](#status)).

## Status

**Implemented**
- Ternary voting with honest vs byzantine agent behavior (`-1`/`0`/`+1`)
- Multi-round tally producing `Accepted` / `Rejected` / `NoConsensus`
- Per-node `CrdtState` with union-merge and a monotonically increasing version counter
- Plurality leader election among honest agents

**Planned / not yet enforced**
- Strict BFT quorum (n ≥ 3f + 1) and a >⅔ acceptance threshold — current threshold is a simple majority of honest agents
- Trust-weighted leader election — `trust_score` is recorded but not yet consulted
- Last-Writer-Wins per-decision CRDT merge — current `merge` is add-only (first-writer-wins on collision)

## Quick Start

```rust
use ternary_consensus::{Agent, ConsensusOutcome, CrdtState, Proposal, Vote};

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

// Each agent votes on the proposal.
for agent in &mut agents {
    agent.vote(&proposal);
}

// Tally votes.
let accepts = agents.iter().filter(|a| a.vote == Vote::Accept).count();
let rejects = agents.iter().filter(|a| a.vote == Vote::Reject).count();
assert_eq!(accepts, 3); // 3 honest agents accept
assert_eq!(rejects, 1); // byzantine agent rejects

// Record the decision in a node's CRDT state.
let mut state = CrdtState::new(0);
state.record(proposal.id, ConsensusOutcome::Accepted);
assert!(state.decisions.contains_key(&1));
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
