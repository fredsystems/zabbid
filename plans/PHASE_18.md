# Phase 18: Bootstrap Workflow Completion

## Phase 18 Goal

Finalize the **bid year bootstrap workflow** so that a bid year can be fully prepared for bidding in a deliberate, explicit, and auditable way.

Phase 18 defines what it means for a bid year to be **structurally complete**, **operator-confirmed**, and **eligible to enter bidding**, without relying on implicit assumptions or partial setup.

This phase intentionally focuses on **workflow correctness and operator experience**, not bidding logic itself.

---

## Phase 18 Scope

Phase 18 includes:

- Completing the manual bootstrap workflow for a bid year
- Explicitly designating a single **active bid year**
- Defining and enforcing **bootstrap completeness gates**
- Manual creation and editing of domain users
- Guided UI workflows for bootstrap steps
- Visibility into incomplete or blocked bootstrap state
- Role-gated mutation (admin vs bidder vs viewer)
- Read-only access for unauthorized users

Phase 18 explicitly excludes:

- Bidding rounds or bid lifecycle logic
- Leave bidding or bid validation
- Seniority ordering or tie-breaking
- Performance optimizations
- Public-facing UI polish
- Mass-import automation beyond CSV ingestion scaffolding

---

## Phase 18 Core Concepts

### Bootstrap Completeness

A bid year is considered **bootstrapped** only when **all required structural criteria are met**.

Bootstrap completeness is:

- Explicit (never inferred)
- Deterministic
- Inspectable via API and UI
- Enforced by the backend

---

## Phase 18 Bootstrap Requirements

### Bid Year

For a bid year to be eligible:

- The bid year must exist
- The bid year must not be sealed
- Exactly one bid year may be marked as **active** at any time

Changing the active bid year:

- Is an admin-only action
- Emits an audit event
- Does not delete or modify historical data

---

### Areas

Area bootstrap requirements are **explicitly bounded**.

- The system must define an **expected number of areas** for the bid year
- Merely having one or more areas does not satisfy completeness
- Area creation is manual and admin-only

Bootstrap rule:

- The bid year is **blocked** until the number of created areas equals the expected count

The expected area count:

- Is provided explicitly by an admin
- Is auditable
- May be edited prior to sealing the bid year

---

### Users

Users are bootstrapped **per area**, not globally.

- Users must belong to exactly one area
- Users must be associated with the active bid year
- User creation and editing is admin-only

User bootstrap requirements:

- Each area must meet its **expected user count**
- The expected user count per area is explicitly defined
- A bid year cannot advance while any area is underfilled

Expected user counts:

- Are defined explicitly per area
- Are auditable
- May be adjusted prior to sealing the bid year

---

## Phase 18 User Creation Workflow

### Manual Entry

The system must support:

- Manual user creation via UI
- Manual editing of user metadata, subject to domain rules
- Validation of all user invariants on submission

Manual entry is the **primary, guaranteed workflow**.

---

### CSV Import (Selective)

Phase 18 introduces **selective CSV import** as an assisted workflow.

CSV import characteristics:

- Operators upload a CSV file
- The system previews parsed rows
- Operators select which rows to import
- Each row is validated individually
- Failed rows do not block valid rows
- No partial row mutation occurs

CSV import:

- Does not bypass domain rules
- Does not auto-create areas
- Does not auto-complete bootstrap steps
- Emits audit events per created user

CSV import exists to reduce operator fatigue, not to replace validation.

---

## Phase 18 — CSV User Import (Revised Specification)

Phase 18 includes a CSV-based user import workflow designed for large-scale bootstrap of users into areas. This workflow must be **robust, human-friendly, and deterministic**, with all validation owned by the backend.

---

### CSV Format Requirements

#### Header-Driven Parsing (Order-Independent)

- CSV files **must include a header row**
- Column **order is irrelevant**
- Columns are identified **by name**, not position
- Headers are matched using:
  - Case-insensitive comparison
  - Whitespace trimmed
  - Spaces converted to underscores

Example equivalence:

| CSV Header                 | Normalized                 |
| -------------------------- | -------------------------- |
| `Full Name`                | `full_name`                |
| `AREA_ID`                  | `area_id`                  |
| `Service Computation Date` | `service_computation_date` |

---

### Required Columns

The following columns **must be present** in the header:

```text
initials
full_name
area_id
service_computation_date
eod_date
```

Optional columns may be included but are not required.

```text
lottery
cumulative_natca_time
natca_time
crew
```

## Phase 18 UI Requirements

The UI must provide:

- A guided bootstrap flow for admins
- Clear visibility into which steps are complete or blocked
- Explicit reasons for any blocked step
- Read-only views for bidders and viewers
- No mutation controls for unauthorized roles

The UI must not:

- Infer completeness
- Hide blocking conditions
- Allow partial submission of invalid state

---

## Phase 18 API Requirements

The API must expose:

- Bootstrap status for the active bid year
- Area completeness status
- User completeness status per area
- Explicit reasons for blocking conditions
- Mutation endpoints gated by operator role

All completeness checks must live in the backend.

---

## Phase 18 Audit Semantics

- All bootstrap mutations emit audit events
- Audit events include:
  - actor operator identity
  - action performed
  - target entity
- Failed validations emit no audit events
- Viewing bootstrap status emits no audit events

---

## Phase 18 Exit Criteria

Phase 18 is complete when all of the following are true:

- A bid year can be fully bootstrapped via the UI
- Exactly one bid year can be designated active
- Area completeness is enforced explicitly
- User completeness per area is enforced explicitly
- CSV import supports selective, validated ingestion
- Unauthorized roles cannot mutate bootstrap state
- All blocking conditions are visible and explainable
- All actions are auditable
- No implicit assumptions exist in bootstrap logic
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
