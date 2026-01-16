# PHASE_6.2.md

## Phase 6.2: Bootstrap API Completeness

### Phase 6.2 Goal

Ensure the entire bootstrap process is fully accessible, enforceable, and observable through explicit API endpoints.

Phase 6.2 exists to guarantee that a system can be initialized from an empty database using only supported HTTP APIs, without implicit behavior or out-of-band setup.

---

### Phase 6.2 Scope

Phase 6.2 includes:

- API endpoints for bid year creation and listing
- API endpoints for area creation and listing
- API endpoints for user listing (structural visibility only)
- End-to-end bootstrap via HTTP from an empty database
- Authorization and audit coverage for all bootstrap actions
- API-level tests validating bootstrap behavior

Phase 6.2 explicitly excludes:

- Bidding logic or crew reassignment
- Seniority logic
- Eligibility or capacity rules
- Round modeling or round lifecycle
- New domain rules or state transitions
- Persistence schema changes
- UI or frontend concerns

---

### Phase 6.2 Bootstrap API Requirements

The following bootstrap steps must be achievable exclusively via API calls:

1. Create a bid year
2. List existing bid years
3. Create one or more areas within a bid year
4. List areas for a given bid year
5. Create users within existing bid years and areas
6. List users per bid year and area

No implicit creation or side effects are allowed.

---

### Phase 6.2 Required API Endpoints

Phase 6.2 must expose API endpoints for:

#### Bid Years

- Create bid year
- List bid years

Bid year listing must never fail.

---

#### Areas

- Create area within a bid year
- List areas for a bid year

Area creation must fail explicitly if the bid year does not exist.

---

#### Users (Structural)

- Create user (already exists)
- List users for a given bid year and area

User listing is read-only and must not mutate state.

---

### Phase 6.2 Failure Semantics

Bootstrap API calls must fail explicitly if:

- A bid year does not exist
- An area does not exist
- A bid year is duplicated
- Structural preconditions are violated

Failure guarantees:

- No state mutation
- No audit event emission
- Structured, deterministic errors

---

### Phase 6.2 Audit Requirements

- All successful bootstrap actions must emit exactly one audit event
- Audit events must attribute:
  - acting actor
  - cause
  - action performed
- Failed bootstrap actions must not emit audit events

---

### Phase 6.2 Exit Criteria

Phase 6.2 is complete when all of the following are true:

- A fresh database can be fully bootstrapped via API alone
- Bid years can be created and listed via API
- Areas can be created and listed via API
- Users can be created and listed via API
- All bootstrap actions are auditable
- No domain rules are duplicated in the API layer
- No new domain behavior is introduced
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
