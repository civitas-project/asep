----------------------- MODULE INVARIANT_SKELETON -----------------------
\* @design:FR-[MODULE]-[ID]
\* @criticality:high
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS EntityId, InitState, ActiveState, ClosedState
VARIABLES entities, observers

TypeOK ==
    /\ entities \in [EntityId -> {InitState, ActiveState, ClosedState}]
    /\ observers \in SUBSET EntityId

Init ==
    /\ entities = [e \in EntityId |-> InitState]
    /\ observers = {}

Activate(e) ==
    /\ entities[e] = InitState
    /\ entities' = [entities EXCEPT ![e] = ActiveState]
    /\ UNCHANGED observers

Observe(e) ==
    /\ entities[e] /= ClosedState
    /\ observers' = observers \cup {e}
    /\ UNCHANGED entities

Close(e) ==
    /\ entities[e] /= ClosedState
    /\ entities' = [entities EXCEPT ![e] = ClosedState]
    /\ observers' = observers \ {e}

Next ==
    \E e \in EntityId :
        \/ Activate(e)
        \/ Observe(e)
        \/ Close(e)

Spec == Init /\ [][Next]_<<entities, observers>>

\* Safety Invariant: Closed entities are never currently observed
ClosedEntitiesNeverObserved ==
    \A e \in observers : entities[e] /= ClosedState

=============================================================================
