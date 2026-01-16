# Phase 1

## Phase 1: Domain Rules & Enforcement

**Goal:**
Establish and prove the **pattern** for defining, enforcing, failing, and auditing real domain rules.

Phase 1 validates _how_ rules are expressed and applied, not how many rules exist.

---

### Phase 1 Objectives

- Formalize bid-year–scoped domain vocabulary
- Implement a representative domain rule end-to-end
- Enforce domain rules through explicit core state transitions
- Produce auditable state changes for successful transitions
- Establish a repeatable pattern for adding future rules

---

### Phase 1 Scope

Phase 1 includes:

- Bid-year–scoped user domain modeling
- Domain rule definition as pure, testable functions
- Core command handling and rule enforcement
- Structured domain and core error handling
- Audit event generation for successful transitions
- Unit tests demonstrating success and failure paths

Phase 1 explicitly excludes:

- Seniority ordering or tie-breaking logic
- Bidding round modeling or round-specific rules
- Persistence or database integration
- API exposure (HTTP, gRPC, etc.)
- Authentication or authorization
- Configuration-driven behavior
- Performance optimization

---

### Phase 1 Representative Rule

Phase 1 will implement **one representative domain rule** to prove the enforcement pattern.

The initial rule is:

- **Within a bid year, user initials must be unique**

This rule must:

- Be scoped strictly to a single bid year
- Treat initials as the sole user identifier
- Fail explicitly with structured domain errors
- Be enforced before any state mutation
- Produce an audit event only on success

No additional rules may be added unless explicitly approved.

---

### Phase 1 Audit Guarantees

- Every successful state transition emits exactly one audit event
- Audit events must capture:
  - actor
  - cause
  - action performed
  - before and after state
- Failed transitions must not emit audit events
- Audit records are immutable once created

---

### Phase 1 Testing Requirements

Tests must demonstrate:

- Domain rules accept valid input
- Domain rules reject invalid input with structured errors
- Core transitions enforce domain rules consistently
- Failed transitions do not mutate state
- Successful transitions emit exactly one audit event
- Audit events accurately reflect before and after state

Tests should serve as executable examples of the rule enforcement pattern.

---

### Phase 1 Exit Criteria

Phase 1 is complete when all of the following are true:

- Bid-year–scoped user domain model is established
- The representative domain rule is implemented and tested
- Rule failures are explicit and structured
- Core state transitions enforce rules atomically
- Audit events are generated only for successful transitions
- Adding an additional rule would follow an obvious, mechanical pattern
- No round-specific or seniority-resolution logic exists
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
