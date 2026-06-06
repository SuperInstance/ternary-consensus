# ternary-consensus

**Distributed ternary agent consensus for GPU clusters with Byzantine fault tolerance, CRDT state synchronization, and leader election.**

## Background

Classical distributed consensus — from Paxos to Raft — assumes binary outcomes: a proposal is either accepted or rejected. But GPU cluster decisions often involve a richer decision space: *accept*, *reject*, or *abstain/neutral*. The `ternary-consensus` crate introduces **three-valued voting** where agents cast votes from the set {-1, 0, +1}, enabling a third "neutral" state that captures uncertainty, lack of information, or deliberate abstention.

This maps naturally to real-world GPU scheduling: a node may vote *accept* if it has capacity, *reject* if it's overloaded, or *neutral* if it lacks visibility into the proposed workload. Unlike binary consensus (à la Raft), ternary consensus doesn't force premature commitment when information is incomplete.

The design draws on Byzantine fault tolerance literature (Lamport's BFT), CRDT-based state machines (Shapiro et al.), and leader election protocols from ZooKeeper/Zab.

## How It Works

The ternary consensus protocol operates as follows:

1. **Proposal Submission**: A proposal carries an `id`, `description`, and a `merit` value in {-1, 0, +1}. The merit encodes the proposer's initial assessment.

2. **Agent Voting**: Each agent evaluates the proposal and casts a `Vote`:
   - **`+1` (Accept)**: Agent endorses the proposal.
   - **`0` (Neutral)**: Agent abstains — neither supports nor opposes.
   - **`-1` (Reject)**: Agent opposes the proposal.

3. **Byzantine Handling**: Byzantine agents deliberately vote *opposite* of honest agents. The protocol tolerates up to `f` Byzantine agents among `3f + 1` total agents (matching classical BFT bounds). Byzantine agents have reduced trust scores.

4. **Consensus Threshold**: A proposal is accepted if `accept_count ≥ threshold` (majority of honest agents). Similarly for rejection. If no threshold is reached within `max_rounds`, the outcome is `NoConsensus`.

5. **CRDT State Sync**: After each decision, all nodes record the outcome in a CRDT (Conflict-free Replicated Data Type) state. States are merged across nodes using last-writer-wins semantics with monotonically increasing versions, ensuring eventual consistency even during partitions.

6. **Leader Election**: Leaders are elected via ternary voting — honest agents nominate candidates, and the candidate with the highest vote sum wins.

## Experimental Results

The test suite demonstrates:

- **Basic consensus**: A cluster of honest agents reaches consensus on a positive-merit proposal in a single round.
- **Byzantine tolerance**: A cluster with 3 honest and 1 Byzantine agent still reaches consensus despite adversarial voting.
- **Multi-round convergence**: When initial votes are split, the protocol retries until a threshold is met or the round limit is exhausted.
- **CRDT merge correctness**: After sync, all nodes have identical decision histories regardless of initial partition state.
- **Leader election**: The protocol consistently elects honest agents as leaders even with Byzantine participants.
- **Average round tracking**: The cluster tracks `avg_rounds` to measure convergence efficiency over time.

## Impact for GPU Cluster Computing

Ternary consensus is fundamentally different from binary consensus in GPU environments:

- **Reduced false positives**: Binary forced-choice leads to spurious rejections when nodes simply lack information. The neutral vote prevents this.
- **Faster convergence**: In simulations, ternary consensus reaches stable decisions in fewer rounds because neutral votes don't count against either side — they simply don't contribute.
- **Better GPU utilization**: Neutral votes allow the cluster to proceed with partial agreement, avoiding the all-or-nothing bottleneck of binary protocols.

## Use Cases

1. **GPU Job Scheduling**: A cluster of GPU nodes vote on whether to accept a new inference workload. Nodes with spare capacity vote +1, overloaded nodes vote -1, and nodes with unknown state vote 0.
2. **Model Deployment Consensus**: Before deploying a new ML model version, cluster nodes vote on readiness. A neutral vote indicates "I haven't validated yet" — deployment proceeds when enough nodes are +1 without needing unanimous agreement.
3. **Resource Allocation in Federated Learning**: Participating data centers vote on global hyperparameter changes. Neutral votes allow centers with incomplete training data to abstain without blocking progress.
4. **Cluster Health Decisions**: Nodes vote on whether to evict a suspected-failing node. Neutral votes let healthy nodes defer judgment until more evidence is available.

## Open Questions

1. **Optimal Byzantine threshold**: Can ternary consensus tolerate `> f` Byzantine agents among `3f + 1` by leveraging the neutral vote as a signal of uncertainty?
2. **Network partition behavior**: How does ternary consensus behave under asymmetric partitions where some nodes can only see neutral votes from a subset of peers?
3. **GPU kernel integration**: Can voting happen inside a CUDA/OpenCL kernel where each thread-block casts a ternary vote, achieving sub-microsecond consensus?

## Connection to Oxide Stack

`ternary-consensus` is the **decision layer** of the five-layer GPU runtime:

| Layer | Crate | Role |
|-------|-------|------|
| 1 — Identity | `ternary-version` | Version vectors for state identity |
| 2 — Communication | `ternary-epidemic` | Gossip-based state propagation |
| 3 — Agreement | **`ternary-consensus`** | Distributed decision-making |
| 4 — Enforcement | `ternary-lease`, `ternary-semaphore` | Resource control |
| 5 — Observation | `ternary-sketch`, `ternary-bloom-filter` | Monitoring and analytics |

Without agreement, the GPU cluster cannot coordinate — this crate is what makes a fleet act as one.
