# Future Integration: ternary-consensus

## Current State
Consensus algorithms for distributed ternary agents with `TernaryValue` (Neg/Zero/Pos), `RaftNode` for leader election and log replication, `ByzantineConsensus` for Byzantine fault-tolerant voting, `VotingRound` for simple majority/supermajority, and `ConsensusLog` for replicated decision logs.

## Integration Opportunities

### With Game-Theory Nash Equilibrium
Ternary consensus maps to game-theoretic equilibrium. Each node is a player with strategy {-1, 0, +1}. `VotingRound` is a coordination game where nodes want to agree. Nash equilibrium occurs when no node can improve by unilaterally changing its vote. `RaftNode`'s leader election is a focal point mechanism. `ByzantineConsensus` handles adversarial players (Byzantine nodes) who deviate from equilibrium.

### With ternary-room (Door Consensus)
When multiple rooms share a door, its state (Locked/Open/OneWay) requires consensus. `RaftNode` elects a door leader. `ConsensusLog` records all door state changes. `ByzantineConsensus` handles rooms that disagree about door state (due to network partitions or bugs).

### With ternary-distributed (Consensus on Gossip)
Gossip propagation eventually needs consensus for irreversible decisions (room deletion, agent termination). `VotingRound` provides lightweight consensus for routine decisions. `RaftNode` provides strong consensus for critical decisions. The combination: gossip for information, consensus for commitment.

## Potential in Mature Systems
Consensus operates at every level of the fleet. Room-level: agents vote on room actions. Fleet-level: rooms vote on resource allocation. Global-level: instances vote on system configuration. `ByzantineConsensus` ensures the system survives compromised nodes. The ternary vote space {-1, 0, +1} naturally models reject/abstain/approve decisions.

## Cross-Pollination Ideas
- `ConsensusLog` entries could be explained by `ternary-explain` — why did the fleet vote this way?
- `VotingRound` supermajority requirements could be calibrated by `negative-space-core`'s conservation ratios
- `ByzantineConsensus` detection could trigger `avoidance-cascade` monitoring — Byzantine behavior is a cascade in trust
- Nash equilibrium analysis could use `ternary-fitness` landscape tools for finding stable voting strategies

## Dependencies for Next Steps
- Integration with ternary-distributed's gossip for proposal dissemination
- Persistent ConsensusLog storage (crash recovery)
- Byzantine threshold configuration for different fleet security levels
- Connection to ternary-protocol for consensus message types
