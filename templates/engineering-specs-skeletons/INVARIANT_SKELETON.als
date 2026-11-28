// @design:FR-[MODULE]-[ID]
// @criticality:high
module [module_name]

// Optional: Linear sequence ordering module
// open util/ordering[SeqNum] as SO

// 1. Entities and Value Types (Signatures)
sig EntityId {}

abstract sig StateKind {}
one sig StateInit, StateActive, StateClosed extends StateKind {}

sig RecordEntity {
  id: one EntityId,
  state: one StateKind
}

sig ObserverSession {
  target: one EntityId
}

// 2. State & Safety Invariants (Predicates)
pred ValidObservation[r: RecordEntity, o: ObserverSession] {
  r.id = o.target and r.state != StateClosed
}

// 3. Security & Integrity Theorems (Assertions)
assert ClosedEntitiesNeverObserved {
  all o: ObserverSession, r: RecordEntity |
    ValidObservation[r, o] implies r.state != StateClosed
}

// 4. Bounded Verification Command
// Run via: alloy exec -t text docs/invariants/[model].als
// Must confirm UNSAT (zero counterexamples)
check ClosedEntitiesNeverObserved for 6
