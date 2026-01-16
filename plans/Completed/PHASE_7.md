# PHASE_7.md

## Phase 7: Canonical State Refactor

**Goal:**
Introduce explicit, relational canonical storage for current operational state while preserving all existing audit, snapshot, and rollback guarantees.

This phase aligns the implementation with clarified architectural intent.
Earlier phases are not reinterpreted or modified.

---

### Phase 7 Scope

Phase 7 includes:

- Introduction of canonical relational tables for current state
  - users
  - areas
  - bid years
  - (future) bids
- Refactoring write paths to:
  - update canonical tables
  - emit audit events
  - optionally persist snapshots
- Refactoring read APIs to:
  - read from canonical tables for current state
  - use audit/snapshots only for historical queries
- Transactional consistency between:
  - canonical state mutation
  - audit event persistence
- Migration of existing persistence logic to respect this separation

Phase 7 explicitly excludes:

- New domain rules
- New business logic
- Changes to audit semantics
- Changes to rollback semantics
- Performance optimization
- Schema versioning strategies
- Data migrations for existing deployments

---

### Phase 7 Canonical vs Derived Responsibilities

- Canonical tables represent **current authoritative state**
- Audit events represent **what happened and when**
- Snapshots represent **accelerators for historical reconstruction**
- Current-state APIs must not replay audit logs
- Historical APIs must not read canonical tables directly

---

### Phase 7 Invariants

Phase 7 must preserve all existing invariants:

- All state mutations are auditable
- Audit logs remain append-only
- Rollbacks are explicit events
- Historical state reconstruction remains deterministic
- No state change occurs without an audit event

---

### Phase 7 Testing Requirements

Tests must demonstrate:

- Canonical tables reflect current state correctly
- Audit events are emitted for all mutations
- Snapshots reflect canonical state at the correct event
- Read APIs return correct data without audit replay
- Historical queries remain correct and deterministic

---

### Phase 7 Exit Criteria

Phase 7 is complete when:

- Canonical tables exist and are populated correctly
- Current-state APIs read exclusively from canonical tables
- Historical APIs remain functional and unchanged
- Audit and snapshot behavior is preserved
- No existing domain rules are altered
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
