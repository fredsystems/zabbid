# PHASE_4.md

## Phase 4: Read Models & Queries

### Goal

Expose system state and audit history in a safe, deterministic, read-only manner without weakening domain authority, audit guarantees, or rollback semantics.

Phase 4 exists to make the system observable and inspectable while preserving all correctness invariants established in earlier phases.

---

### Phase 4 Scope

Phase 4 includes:

- Read-only access to effective state per (bid year, area)
- Read-only access to historical state at a specific point in time
- Read-only access to ordered audit event timelines
- Deterministic state reconstruction using persisted audit events and snapshots
- Query helpers or interfaces that surface domain data without mutation
- Tests validating correctness of reads and reconstruction

Phase 4 explicitly excludes:

- Any form of state mutation
- Write-capable APIs or command execution
- Authentication or authorization
- New domain rules or validations
- Persistence schema changes
- Background jobs, caching layers, or async processing
- Performance optimization beyond correctness

---

### Read Semantics

- All read operations must be side-effect free
- Reads must not emit audit events
- Reads must not modify in-memory or persisted state
- Reads must not depend on mutable global state
- Read results must be fully derivable from persisted data

---

### Required Read Capabilities

Phase 4 must support, at minimum:

#### Effective State Queries

- Retrieve the current effective state for a given bid year and area
- The effective state must reflect all audit events, including rollbacks

#### Historical State Queries

- Retrieve the effective state for a given bid year and area at a specific point in time
- Time-based queries must be deterministic
- If a timestamp does not correspond exactly to an event, the most recent prior event defines the state

#### Audit Timeline Queries

- Retrieve the ordered list of audit events for a given bid year and area
- Audit events must be returned in strict chronological order
- Rollback events must appear in the timeline as first-class events

---

### State Reconstruction Rules

- State reconstruction must:
  - start from the most recent snapshot at or before the target point
  - replay audit deltas forward deterministically
- Rollback events must alter the effective state for subsequent reconstruction
- Reconstruction logic must not depend on in-memory history
- Reconstruction must yield identical results across repeated executions

---

### Error Handling

- Invalid read requests (e.g. unknown bid year, area, or timestamp) must fail explicitly
- Errors must be structured and testable
- Read errors must not leak persistence or infrastructure details

---

### Phase 4 Testing Requirements

Tests must demonstrate:

- Retrieval of current effective state
- Retrieval of historical state at a given time
- Correct handling of rollback events during reconstruction
- Deterministic reconstruction from persisted data
- No mutation of state during read operations

Tests must not rely on mocks that bypass persistence behavior.

---

### Phase 4 Exit Criteria

Phase 4 is complete when all of the following are true:

- Current and historical state can be queried safely
- Audit timelines are accessible and ordered correctly
- Rollback effects are visible in read results
- All read paths are side-effect free
- State reconstruction is deterministic and tested
- No write paths or mutation logic are introduced
- Domain, persistence, and audit semantics remain unchanged
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
