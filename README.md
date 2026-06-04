# ternary-consensus

Distributed consensus algorithms for ternary agents — Raft-style leader election, Byzantine fault-tolerant agreement, majority/plurality/supermajority voting, and replicated decision logs.

## Why This Exists

In distributed systems where agents vote on ternary decisions (approve/abstain/reject, buy/hold/sell, +1/0/−1), standard binary consensus protocols don't directly apply. This crate adapts classic consensus algorithms — Raft and Byzantine agreement — for the three-valued domain, along with flexible voting mechanisms and a replicated log for recording ternary decisions.

## Core Concepts

- **TernaryValue** — Core type: `Neg` (−1), `Zero` (0), `Pos` (+1) with clamped combining
- **RaftConsensus** — Leader election, proposal replication, and commit for ternary values
- **ByzantineConsensus** — PBFT-style agreement tolerating up to f faulty nodes (requires n ≥ 3f+1)
- **VotingRound** — Simple, supermajority, plurality, weighted, and unanimous voting
- **ConsensusLog** — Replicated, mergeable, compactable log of ternary decisions

## Quick Start

```toml
# Cargo.toml
[dependencies]
ternary-consensus = "0.1"
```

```rust
use ternary_consensus::*;

// --- Raft-style consensus ---
let mut raft = RaftConsensus::new(5);

// Elect a leader
assert!(raft.elect_leader(0));
println!("Leader: {:?}", raft.leader());

// Propose and commit a value
raft.propose(TernaryValue::Pos);
raft.force_commit_all();
assert_eq!(raft.last_committed(), Some(TernaryValue::Pos));

// Multiple proposals
raft.propose(TernaryValue::Neg);
raft.propose(TernaryValue::Zero);
println!("Log length: {}", raft.log_len());

// --- Byzantine fault tolerance ---
let mut byz = ByzantineConsensus::new(4, 1);  // 4 nodes, tolerate 1 fault
assert!(byz.is_valid());  // 4 >= 3*1 + 1

// One node goes Byzantine
byz.set_byzantine(3);
let values = vec![
    TernaryValue::Pos,
    TernaryValue::Pos,
    TernaryValue::Pos,
    TernaryValue::Pos,  // Byzantine node will lie
];
let result = byz.run_round(&values);
println!("Consensus despite Byzantine: {:?}", result);

// Multi-round agreement
let final_value = byz.agree(&values, 3);

// --- Voting rounds ---
let mut vote = VotingRound::new(5);
vote.vote(0, TernaryValue::Pos);
vote.vote(1, TernaryValue::Pos);
vote.vote(2, TernaryValue::Pos);
vote.vote(3, TernaryValue::Neg);

println!("Majority: {:?}", vote.majority());        // Some(Pos)
println!("Plurality: {:?}", vote.plurality());       // Pos
println!("Supermajority: {:?}", vote.supermajority()); // Some(Pos) - 3/5 >= 2/3
println!("Unanimous: {}", vote.is_unanimous());      // false
println!("Weighted: {:?}", vote.weighted_result());

// --- Replicated log ---
let mut log = ConsensusLog::new(3);
let idx = log.append(1, TernaryValue::Pos);
log.append(1, TernaryValue::Neg);
log.append(1, TernaryValue::Zero);
log.commit_up_to(3);

println!("Committed entries: {}", log.committed().len());

// Merge logs from different nodes
let mut log2 = ConsensusLog::new(3);
log2.append(1, TernaryValue::Pos);
let conflicts = log.merge(log2.entries());

// Compact old entries
log.compact(1);
println!("After compaction: {} entries", log.len());
```

## API Overview

| Type | Description |
|---|---|
| `TernaryValue` | `Neg`/`Zero`/`Pos` with `to_i8`, `from_i8`, `combine` |
| `RaftConsensus` | `elect_leader`, `propose`, `get_committed`, `last_committed` |
| `ByzantineConsensus` | `run_round`, `agree`, `set_byzantine`, `is_valid` |
| `VotingRound` | `vote`, `majority`, `plurality`, `supermajority`, `weighted_result`, `is_unanimous` |
| `ConsensusLog` | `append`, `commit_up_to`, `committed`, `merge`, `compact` |
| `Node` | Network node with id, value, term, leader flag, Byzantine flag |
| `LogEntry` | Term, index, value, committed flag |

## How It Works

**Raft**: A candidate requests votes from peers. With a majority, it becomes leader and accepts proposals. The leader replicates each proposal to all nodes. Once a majority of nodes have the value, it's committed. The commit index advances as entries gain majority acknowledgment.

**Byzantine Agreement**: A simplified PBFT protocol runs in three phases: (1) Pre-prepare — each node proposes its value, (2) Prepare — majority vote on proposals, (3) Commit — another majority vote on the prepare result. Byzantine nodes propose conflicting values or flip the prepare result, but honest majority prevails when `n ≥ 3f + 1`.

**Voting**: Five resolution strategies for ternary votes:
- **Majority**: >50% threshold (strict)
- **Supermajority**: ≥2/3 threshold (for critical decisions)
- **Plurality**: Most votes wins (no threshold)
- **Weighted**: Voter-ID-based weights, threshold at ±0.33 average
- **Unanimous**: All votes must agree

**Consensus Log**: Append-only log with commit tracking. Logs from different nodes can be merged (higher term wins conflicts). Compaction trims old entries for long-running systems.

## Use Cases

1. **Distributed decision-making** — Multi-agent systems that need to agree on ternary actions (advance/hold/retreat)
2. **Financial consensus** — Trading systems where nodes vote on market signals (buy/hold/sell) with Byzantine fault tolerance
3. **Governance protocols** — On-chain voting with ternary ballots (for/against/abstain) requiring supermajority
4. **Sensor fusion** — Multiple sensors reporting ternary readings (high/normal/low) needing agreement despite faulty nodes

## Ecosystem

Part of the **SuperInstance** ternary computing crate family:

- `ternary-compression-v2` — Multi-algorithm ternary compression
- `ternary-hash` — Hashing and fingerprinting for ternary data
- `ternary-pca` — Principal component analysis on ternary values
- `ternary-ga` — Genetic algorithms with ternary genomes
- `ternary-matrix` — Compact ternary matrix operations
- `ternary-reservoir` — Echo state networks with ternary nodes
- `ternary-evolution-advanced` — Advanced evolutionary optimization
- `ternary-geometry` — Geometric algorithms in ternary space
- `ternary-causality` — Causal inference for ternary systems

## License

MIT
