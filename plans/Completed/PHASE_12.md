# PHASE_12.md

## Phase 12: Operator-Focused UI & API Co-Design

### Phase 12 Goal

Establish a **real, durable operator UI** and use it to validate and refine API ergonomics **without weakening domain authority**.

Phase 12 exists to ensure the system is _usable by humans_ while preserving all existing invariants:

- the backend remains the sole arbiter of correctness
- the audit model remains intact
- the API remains command-driven for writes and authoritative for reads

The UI produced in this phase is **not throwaway**.
It is intentionally minimal but structurally final.

---

### Phase 12 Scope

Phase 12 includes:

- A browser-based operator UI implemented in **TypeScript**
- UI flows for system bootstrap and inspection
- Read-model–driven API ergonomics refinements motivated by real UI usage
- Explicit separation between:
  - **UI validation** (early feedback)
  - **backend validation** (authoritative)
- Incremental API evolution to support operator workflows

Phase 12 explicitly excludes:

- Styling or visual polish
- Mobile responsiveness
- Authentication UX
- Public API stability guarantees
- Any domain rule changes
- Any weakening of audit or validation semantics

---

### Phase 12 Core Principles

#### Backend Authority

- The UI may validate inputs for operator feedback
- The backend remains the **final arbiter** of all correctness
- UI validation failures must never be treated as authoritative

#### No CRUD Drift

- Write operations remain **command-oriented**
- Reads may be ergonomic, aggregated, or denormalized
- The UI must not imply mutable resource semantics

#### One Active Bid Year

- The system operates on **exactly one active bid year at a time**
- UI workflows must not allow multi-bid-year mutation contexts
- Any bid year switching must be explicit and intentional

---

### Phase 12 UI Responsibilities

The UI must support, at minimum:

#### Bootstrap Overview

- View all bid years
- Identify the active bid year
- See bootstrap completeness at a glance:
  - area count
  - user count
  - missing structures

#### Area View

- List all areas in the active bid year
- Display:
  - area identifier
  - user count
- Navigate into an area

#### User List View

- List users within an area
- Display:
  - initials
  - name
  - user type
  - leave availability (days / hours)
  - exhaustion / overdraw indicators

#### User Detail View

- Show full user metadata
- Show leave accrual breakdown (rich model)
- Show derived totals
- Read-only in Phase 12

---

### Phase 12 API Evolution Rules

API changes are allowed **only** if they:

- Reduce client complexity
- Eliminate client-side inference
- Preserve existing domain and audit semantics
- Remain read-only unless explicitly approved

Any API change must:

- Update request/response DTOs
- Update server handlers
- Update persistence queries if required
- Update `api_cli.py`
- Include tests covering the new surface

---

### Phase 12 Frontend Technology Constraints

- Implemented in **TypeScript**
- Runs entirely in a web browser
- No domain logic duplicated
- No persistence assumptions
- Treats the API as authoritative

Framework choice is flexible but must support:

- explicit data fetching
- clear state management
- predictable error handling

---

### Phase 12 Validation Strategy

- UI performs **early, non-authoritative checks**:
  - required fields
  - obvious structural errors
- Backend performs **final validation**:
  - domain rules
  - invariants
  - audit emission
- Backend errors must surface cleanly in the UI

---

### Phase 12 Testing Requirements

Phase 12 must demonstrate:

- API surfaces are sufficient to drive the UI without hacks
- No UI flow requires undocumented API behavior
- API ergonomics reduce request chaining and guesswork
- Backend remains unchanged in authority and invariants

UI testing may be minimal but must prove:

- successful rendering of real data
- error handling paths are visible and understandable

---

### Phase 12 Exit Criteria

Phase 12 is complete when all of the following are true:

- A real operator UI exists and runs in the browser
- The UI can perform full bootstrap inspection
- The UI can navigate bid year → area → user
- Leave availability is visible without additional API calls
- Any API changes are reflected in `api_cli.py`
- No domain rules were added or modified
- No audit semantics were altered
- The backend remains the sole authority for correctness
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

---

### Phase 12 Non-Goals

Phase 12 is **not**:

- A UX polish phase
- A public API stabilization phase
- A frontend rewrite experiment
- A replacement for backend validation
- A shortcut around auditability

Phase 12 is about **making the right thing usable**.
