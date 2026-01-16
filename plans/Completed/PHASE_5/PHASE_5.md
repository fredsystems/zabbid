# PHASE_5.md

## Phase 5: Write APIs & Authorization

**Goal:**
Expose controlled, authenticated, and authorized write access to the system while preserving all domain, audit, persistence, and rollback guarantees.

Phase 5 introduces _who is allowed to mutate state_, not _how state behaves_.

---

### Phase 5 Scope

Phase 5 includes:

- Authentication of system actors
- Authorization of state-changing actions based on actor roles
- Write-capable APIs or interfaces that execute domain commands
- Enforcement of admin vs bidder authority boundaries
- Attribution of all state changes to an authenticated actor
- Persistence and audit of all authorized state transitions
- Tests validating authorization behavior and failure modes

Phase 5 explicitly excludes:

- New domain rules or validations
- Changes to persistence, rollback, or snapshot semantics
- UI or frontend concerns
- Performance optimization
- Multi-role or fine-grained permission systems

---

### Phase 5: Actor & Authorization Model

- All write actions are performed by authenticated actors
- Actors are distinct from domain users
- Roles apply only to actors, never to domain users

The system recognizes exactly two roles:

#### Phase 5: Admin

Admins are authorized to perform structural and corrective actions, including but not limited to:

- creating or modifying bid years
- creating or modifying areas
- creating or modifying users
- performing rollbacks
- creating checkpoints
- finalizing rounds or equivalent milestone events
- any other system-level or corrective actions

#### Phase 5: Bidder

Bidders are authorized to perform bidding-related actions, including:

- entering new bids
- modifying existing bids
- withdrawing or correcting bids

Bidders perform bidding actions on behalf of domain users.
They are not the same entities as the users whose bids are represented.

---

### Phase 5: Authorization Enforcement

- Authorization must be enforced before command execution
- Unauthorized actions must fail explicitly and deterministically
- Authorization failures must not mutate state or emit audit events
- Domain logic must remain unaware of actor roles
- Core state transitions must be role-agnostic

---

### Phase 5: Write Semantics

- All state changes must occur via explicit domain commands
- All successful write operations must:
  - execute domain validation
  - persist changes atomically
  - emit audit events attributing the acting actor
- Failed write operations must:
  - fail without mutating state
  - fail without emitting audit events
  - return structured, testable errors

---

### Phase 5: Error Handling

- Authentication and authorization errors must be explicit and structured
- Errors must not leak domain or persistence implementation details
- Authorization errors must be distinguishable from domain validation failures

---

### Phase 5 Testing Requirements

Tests must demonstrate:

- Successful execution of authorized write actions
- Explicit failure of unauthorized write actions
- Correct attribution of actors in audit events
- No state mutation on authorization failure
- No audit emission on authorization failure
- Preservation of all domain, persistence, and rollback invariants

Tests must not bypass authentication or authorization logic.

---

### Phase 5 Exit Criteria

Phase 5 is complete when all of the following are true:

- Write APIs are available for all intended commands
- All write operations require authenticated actors
- Authorization rules correctly enforce admin vs bidder roles
- Unauthorized actions fail explicitly and safely
- Audit events correctly attribute acting actors
- Domain, persistence, rollback, and snapshot semantics remain unchanged
- No role or authorization logic leaks into domain or core layers
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
