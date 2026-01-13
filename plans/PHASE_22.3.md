# Phase 22.3 — Capability-Driven UI Gating

## Goal

Eliminate UI-side guessing about what actions an operator is allowed to perform.

The backend must explicitly communicate **capabilities** to the UI, and the UI must gate **actions** (buttons, controls) based solely on those capabilities.

This phase does **not** hide pages, change navigation, or relax backend enforcement.

---

## Non-Goals

Phase 22.3 explicitly does **NOT**:

- Change domain rules
- Change authorization enforcement
- Hide routes or pages based on permissions
- Introduce role-based UI branching
- Add explanations for _why_ actions are disallowed
- Replace backend authorization with frontend checks

Backend authorization remains mandatory and authoritative.

---

## Scope Overview

### Backend

- Expose operator capabilities as explicit boolean flags
- Capabilities reflect **current system state + domain invariants**
- No capability explains _why_ it is false

### Frontend

- Render pages consistently
- Enable/disable or hide **individual actions** based on capabilities
- Never infer permissions from roles

---

## Backend Work

### 1. Define Capability Model

Introduce a **read-only capability structure** associated with the authenticated operator.

Example (illustrative, not prescriptive):

```json
{
  "capabilities": {
    "can_create_operator": true,
    "can_disable_operator": false,
    "can_delete_operator": false,
    "can_create_user": true,
    "can_update_user": true,
    "can_delete_user": false,
    "can_bootstrap": false
  }
}
```

Rules:

- Capabilities are booleans only
- No counts, IDs, or reasons
- Computed dynamically
- Deterministic and testable
- Capabilities MAY be scoped:
  - globally (operator-level)
  - per-entity (target-level)

---

## Capability Model

Capabilities exist at **two explicit levels**:

### 1. Global Capabilities (Operator-Level)

These answer:

> “Is this operator ever allowed to perform this class of action?”

Examples:

- `can_create_operator`
- `can_create_user`
- `can_bootstrap`
- `can_modify_users`
- `can_view_admin_pages`

Global capabilities depend on:

- Operator role
- Operator disabled state
- System-wide state (e.g. bootstrap complete)

If a global capability is `false`, **no instance of that action is permitted**.

---

### 2. Target-Specific Capabilities (Entity-Level)

These answer:

> “Is this operator allowed to perform this action on _this specific entity_?”

Examples:

- `can_disable_operator`
- `can_delete_operator`
- `can_delete_user`
- `can_disable_user`
- `can_move_user_area`

These MUST account for **domain invariants that depend on the target**.

Examples:

- `can_disable_operator == false` for:
  - the last active admin
  - a disabled operator
- `can_delete_operator == false` for:
  - the last active admin
- `can_delete_user == false` for:
  - users with bid data
- `can_move_user_area == false` for:
  - users locked by bidding start (future phase)

Target-scoped capabilities must be computed **per entity instance**.

---

## Capability Computation Rules

Capabilities must be derived from:

- Operator role
- Operator disabled state
- Global domain invariants
- Target entity state
- System state (e.g. active bid year, bidding started)

Capabilities must **never**:

- Encode UI behavior
- Replace authorization checks
- Leak internal counts or reasons
- Require the UI to infer logic

---

## Capability Exposure

Capabilities must be exposed explicitly by the backend.

### Global Capabilities

Exposed once per session, e.g.:

- Included in `/me` or `/session`

Example shape:

```json
{
  "capabilities": {
    "can_bootstrap": true,
    "can_create_user": true,
    "can_create_operator": false
  }
}
```

---

### Target-Specific Capabilities

Exposed **alongside each entity read model**.

Example (`OperatorInfo`):

```json
{
  "operator_id": 3,
  "username": "fred",
  "role": "Admin",
  "is_disabled": false,
  "capabilities": {
    "can_disable": false,
    "can_delete": false
  }
}
```

Example (`UserInfo`):

```json
{
  "user_id": 42,
  "initials": "FC",
  "name": "Fred Clausen",
  "capabilities": {
    "can_delete": false,
    "can_move_area": true,
    "can_edit_seniority": true
  }
}
```

Requirements:

- Read-only
- No side effects
- Fully derived
- Covered by tests
- Deterministic for a given request

---

## Backend Enforcement Reminder (No Change)

All mutating endpoints must still:

- Perform authorization checks
- Enforce domain invariants
- Reject invalid operations even if UI allows them

Capabilities are **advisory**, not authoritative.

---

## Frontend Work

### Action Gating Only

The UI must:

- Render pages normally
- Gate buttons and destructive controls using capabilities
- Never recompute permission logic

Examples:

- Disable “Delete Operator” if `operator.capabilities.can_delete == false`
- Disable “Disable Operator” if `operator.capabilities.can_disable == false`
- Disable “Delete User” if `user.capabilities.can_delete == false`

---

### No Role-Based UI Logic

The UI must NOT:

- Branch on role names
- Assume Admin == allowed
- Hardcode domain rules

All gating flows through capability flags only.

---

### UX Expectations

- Disabled buttons must be visually disabled
- Optional tooltip text: “Action not permitted”
- No explanation of _why_
- No references to roles, counts, or invariants

---

## Testing Requirements

### Testing Requirements Backend

- Global capability computation tests
- Target-specific capability tests:
  - last active admin
  - disabled operators
  - users with protected state
- Capability correctness for mixed operator roles
- Mutation endpoints must ignore capability flags

### Testing RequirementsFrontend

- Button state reflects capability flags
- No role checks in UI code
- Pages render regardless of permissions
- Only actions are gated

---

## Exit Criteria

Phase 22.3 is complete when:

- UI no longer guesses permissions
- Backend provides explicit global and target-scoped capabilities
- All destructive actions are capability-gated
- Domain invariants remain backend-enforced
- No role-based logic exists in the UI
- All changes are fully tested

---

## Design Principle

> **Roles describe identity.
> Capabilities describe permission.
> Targets constrain applicability.
> The backend decides all three.
> The UI only reflects.**
