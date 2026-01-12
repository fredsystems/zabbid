# PHASE_3.md

## Phase 3: Persistence, Audit History, and Rollback

### Goal

Introduce durable persistence for audit history and derived state while preserving domain authority, audit guarantees, and rollback semantics.

Phase 3 proves that system state can be safely reconstructed, corrected, and inspected at any point in time without rewriting history.

---

### Phase 3 Objectives

- Persist audit events in a durable, append-only store
- Persist derived state snapshots to support efficient recovery
- Support explicit rollback as a first-class, auditable action
- Enable reconstruction of effective state at any point in time
- Ensure persistence failures do not result in partial or unaudited state changes

---

### Phase 3 Scope

Phase 3 includes:

- A persistence adapter implemented below the core layer
- Durable storage of audit events, ordered in time
- Durable storage of state snapshots per bid year and area
- SQLite-backed persistence suitable for read-heavy workloads
- Atomic persistence of audit events and snapshots
- Persistence-backed tests demonstrating correctness and recovery

Phase 3 explicitly excludes:

- Authentication or authorization
- Background jobs or asynchronous processing
- Performance tuning or query optimization
- Distributed systems or multi-node coordination
- Schema migration or versioning strategies
- Alternate persistence backends
- UI or frontend concerns

---

### Persistence Semantics

- The audit log is the authoritative source of truth
- Audit events are append-only and are never rewritten or deleted
- All audit events are scoped to a single bid year and a single area
- State is derived by replaying audit events in order
- Rollback is modeled as an explicit audit event
- Rollback establishes a prior effective state as authoritative going forward
- Rollback does not erase or modify historical audit events

---

### State Snapshots

- State is conceptually a complete, materialized snapshot per bid year and area
- Snapshots exist to accelerate recovery and replay
- Snapshots must not alter the meaning of the audit log
- Full state snapshots must be persisted at:
  - rollback events
  - round finalized events
  - explicit checkpoint events
- All other audit events persist deltas only

---

### Failure Guarantees

- Persistence operations must be atomic
- If persistence fails, the transition must fail
- Partial persistence of state or audit data is forbidden
- In-memory state must not advance unless persistence succeeds

---

### Phase 3 Testing Requirements

Tests must demonstrate:

- Successful persistence of audit events
- Persistence of full state snapshots at required boundaries
- Rollback recorded as an auditable event
- Reconstruction of effective state at a given point in time
- No partial persistence on simulated failure

Tests must use deterministic infrastructure provided by the development environment.

---

### Phase 3 Exit Criteria

Phase 3 is complete when all of the following are true:

- Audit events are durably persisted and ordered
- State snapshots are persisted at defined boundaries
- Rollback is implemented as an auditable event
- Effective state can be reconstructed at any point in time
- Persistence failures do not result in partial or unaudited state changes
- Domain and core behavior are unchanged by persistence
- No persistence details leak into domain or core layers
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently.
