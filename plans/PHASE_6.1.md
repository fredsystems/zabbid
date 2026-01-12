# PHASE_6.1.md

## Phase 6.1: Bootstrap & Structural Domain Rules

### Goal

Establish a valid, enforceable system baseline by implementing required bootstrap commands and structural domain constraints.

Phase 6.1 ensures the system cannot enter an invalid or partially-initialized state.

---

### Phase 6.1 Scope

Phase 6.1 includes:

- Bid year creation and validation
- Area creation within a bid year
- Listing existing bid years
- User creation with explicit structural validation
- Enforcement of baked-in crew semantics
- Enforcement of user classification (CPC, CPC-IT, Dev-D, Dev-R)

Phase 6.1 explicitly excludes:

- Bidding logic
- Crew reassignment or bid modification
- Seniority ordering or comparison
- Eligibility rules
- Round modeling
- Limits, capacity, or availability rules

---

### Phase 6.1 Bootstrap Requirements

- A fresh database with no data is a valid initial state
- No commands may succeed unless a bid year exists
- Bid years must:
  - be unique
  - represent a valid calendar year
- Areas must:
  - be explicitly created per bid year
  - exist before users may be created

Bootstrap order is enforced and must not be inferred.

---

### Phase 6.1 User Creation Rules

- Users are scoped to exactly one bid year
- User initials must be unique within a bid year
- User names are informational and not unique
- Users must belong to exactly one area
- Users may have zero or one crew assignment
- If provided, crew values must be one of 1–7
- User type must be one of:
  - CPC
  - CPC-IT
  - Dev-D
  - Dev-R

User creation must fail explicitly if any rule is violated.

---

### Phase 6.1 Crew Semantics

- Crews are baked-in domain constants
- Exactly seven crews exist, identified by numbers 1 through 7
- Each crew has a predefined RDO pattern
- Crews are not created, modified, or deleted
- Crews are not persisted as mutable data
- Crew assignment is optional at user creation
- Crew assignment is modeled as state and must be auditable

---

### Phase 6.1 Failure Semantics

Commands must fail explicitly if:

- The referenced bid year does not exist
- The referenced area does not exist
- The user type is not one of the allowed values
- The crew value is not one of 1–7 (if provided)
- The area does not belong to the referenced bid year
- The user initials are not unique within the bid year

All failures must return explicit, structured errors.

---

### Phase 6.1 Testing Requirements

Tests must demonstrate:

- Successful creation of bid years, areas, and users with valid input
- Explicit failure for all invalid bootstrap and user creation scenarios
- Enforcement of uniqueness and structural rules
- Crew assignment is optional and validated if present
- All failures are explicit and structured

---

### Phase 6.1 Exit Criteria

Phase 6.1 is complete when all of the following are true:

- Bid year, area, and user creation are implemented and validated
- All bootstrap and structural rules are enforced
- All failures are explicit and structured
- Crew semantics are enforced and auditable
- No invalid or partially-initialized state is possible
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
