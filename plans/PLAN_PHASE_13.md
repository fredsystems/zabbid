# Phase 13

## Phase 13: Operator Workflow & Live State UX

### Phase 13 Goal

Enable real, durable operator workflows through a browser-based UI with:

- Clear failure semantics
- Progressive, guided mutation flows
- Live visibility into system state
- Zero erosion of backend authority, audit guarantees, or domain invariants

Phase 13 exists to validate **how humans actually operate the system**, not to finalize UI polish or visual design.

---

### Phase 13 Scope

Phase 13 includes:

- A browser-based operator UI
- Wizard-driven workflows for structural mutations
- Explicit backend connectivity handling
- Live state observation via server-pushed events
- Clear separation between:
  - command execution
  - state observation
  - UI ergonomics

Phase 13 explicitly excludes:

- Leave bidding
- Bid round lifecycle logic
- Capacity or eligibility enforcement
- Optimistic writes
- Offline mutation
- Multi-bid-year workflows
- Authentication hardening
- Performance optimization
- Mobile or responsive layout polish

---

### Phase 13 Operator Model

- Exactly **one bid year is active** at a time
- Operators may observe all state
- Only one operator may execute a bid mutation at a time (future phase)
- All operators see live updates as state changes

The UI must reflect **what is happening**, not decide **what is allowed**.

---

### Phase 13 Connectivity Semantics

The UI must explicitly represent backend connectivity state.

#### Required States

- **Connecting**
  - Backend unreachable
  - No API responses available
  - UI displays:
    - connection status
    - retry indicator
- **Connected**
  - Backend reachable
  - Canonical state loaded
  - Full UI enabled
- **Disconnected**
  - Backend was reachable but connection lost
  - UI displays:
    - connection lost warning
    - last-known-good data (read-only)
    - automatic reconnect attempts

#### Rules

- Network failures must **not** be surfaced as API errors
- HTTP 500 must **never** represent “backend unavailable”
- Reconnection attempts must be automatic and bounded
- Successful reconnection must refresh canonical read models

---

### Phase 13 Command Execution Model

- All mutations are executed via HTTP APIs
- Commands include:
  - actor
  - cause
- Wizard-style flows are permitted and encouraged
- UI validation is **advisory only**
- Backend remains the final arbiter

#### Required Workflow (Phase 13)

- **Create Area**
  - Select bid year (single active)
  - Enter area identifier
  - Review audit context
  - Submit command
  - Display success or structured failure

No other write workflows are required in Phase 13.

---

### Phase 13 Read Model Usage

- UI must load canonical state via read-only APIs
- Read APIs must remain side-effect free
- UI must not replay audit logs
- UI must not infer state transitions

Read data is authoritative but ephemeral.

---

### Phase 13 Live State Observation

Phase 13 introduces **server-pushed state notifications**.

#### Transport

- WebSocket or equivalent server-push mechanism
- Separate from HTTP command APIs

#### Purpose

- Notify UIs of authoritative state changes
- Enable live dashboards and observers
- Reduce polling
- Support multi-operator awareness

#### Allowed Event Types

- Structural changes (e.g. area created)
- State transitions (future phases)
- Operator activity signals (future phases)

#### Prohibited Uses

- No command execution over WebSockets
- No domain validation over WebSockets
- No UI directives (“disable this”, “show that”)

WebSocket messages represent **facts**, not decisions.

---

### Phase 13 UI Behavior Rules

- UI must never assume success
- UI must always reflect backend responses
- UI must tolerate:
  - transient failures
  - reconnects
  - partial visibility
- UI must remain usable without reload after reconnect

---

### Phase 13 Tooling Alignment

- Any API change must:
  - update request/response DTOs
  - update server handlers
  - update UI consumers
  - update `api_cli.py`
- UI and CLI must remain consistent with the API surface
- Mismatches are considered failures

---

### Phase 13 Audit Semantics

- Audit behavior is unchanged
- All successful commands emit exactly one audit event
- Failed commands emit no audit events
- Live notifications do not replace audit inspection

---

### Phase 13 Testing Requirements

Tests must demonstrate:

- UI handles backend unavailable state correctly
- UI recovers automatically on backend restart
- Create Area workflow succeeds end-to-end
- Create Area workflow fails cleanly with structured errors
- Live updates reflect backend state changes
- No UI behavior bypasses backend validation

Testing may include:

- unit tests
- integration tests
- manual verification where appropriate

---

### Phase 13 Exit Criteria

Phase 13 is complete when all of the following are true:

- Operator UI loads and reflects canonical state
- Backend unavailability is handled explicitly and correctly
- Create Area workflow is usable and auditable
- Live state updates are streamed to connected UIs
- No domain logic exists in the frontend
- No commands are executed over WebSockets
- UI reflects backend authority at all times
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
