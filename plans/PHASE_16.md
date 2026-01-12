# Phase 16

## Phase 16: Operator Management (Admin-Controlled)

### Phase 16 Goal

Provide a complete, auditable, and admin-only workflow for managing **operators**
(the humans who operate the system), without introducing new domain behavior
outside identity, authorization, and audit attribution.

Phase 16 ensures that operator identity, lifecycle, and permissions are
**explicitly managed**, **fully auditable**, and **structurally enforced**.

---

### Phase 16 Scope

Phase 16 includes:

- Admin-only creation of operators
- Admin-only viewing of existing operators
- Admin-only management actions on operators:
  - disable
  - re-enable
  - delete (only when allowed by domain rules)
- UI integration for operator management
- Authorization enforcement for all operator actions
- Audit attribution for all operator lifecycle changes

Phase 16 explicitly excludes:

- Bidding logic
- Leave calculations
- User (domain user) creation or modification
- Area or bid year changes
- UI polish beyond functional correctness
- Role granularity beyond Admin vs Bidder
- Two-factor authentication or advanced auth features

---

### Phase 16 Domain Concepts

#### Operator

An **Operator** represents a trusted system user who performs actions
on behalf of domain users.

Operators:

- Are distinct from domain users
- Have a unique, stable identifier
- Have a login name and display name
- Have exactly one role:
  - `Admin`
  - `Bidder`
- May be disabled
- May be deleted **only if never referenced by an audit event**

Operators are **canonical entities**, not derived from audit logs.

---

### Phase 16 Authorization Rules

- Only **Admins** may:
  - create operators
  - disable operators
  - re-enable operators
  - delete operators
  - view the full operator list

- **Bidders** may not:
  - create operators
  - manage operators
  - view operator management screens

Authorization failures must:

- Fail explicitly
- Return structured errors
- Emit **no audit events**

---

### Phase 16 Operator Lifecycle Rules

#### Creation

- Admin provides:
  - login name
  - display name
  - role
  - initial password
- Login names must be unique
- Passwords must be:
  - hashed
  - never stored or transmitted in plain text
- Successful creation emits exactly one audit event

---

#### Disabling

- Disabled operators:
  - cannot authenticate
  - remain visible in operator listings
  - remain referenced by audit history
- Disabling emits an audit event

---

#### Re-enabling

- Disabled operators may be re-enabled by Admins
- Re-enabling emits an audit event

---

#### Deletion

- Operators **may be deleted only if**:
  - they are not referenced by any audit event
- Deletion is forbidden once audit attribution exists
- Deletion emits an audit event
- Foreign key constraints must enforce this invariant

---

### Phase 16 Audit Semantics

- All operator lifecycle changes must emit audit events:
  - operator created
  - operator disabled
  - operator re-enabled
  - operator deleted
- Audit events must include:
  - acting operator (actor)
  - target operator identifier
  - action performed
- Authentication failures emit **no audit events**

---

### Phase 16 API Requirements

Required endpoints (Admin-only):

- Create operator
- List operators
- Disable operator
- Re-enable operator
- Delete operator (when allowed)

API rules:

- All write endpoints require a valid session
- Authorization enforced at API boundary
- No domain logic duplicated in API layer
- Structured error responses for all failures

---

### Phase 16 UI Requirements

#### Operator Management Screen

- Accessible only to Admins
- Lists all operators with:
  - login name
  - display name
  - role
  - status (active / disabled)
- Provides explicit actions:
  - disable
  - re-enable
  - delete (only when allowed)

UI rules:

- No action buttons shown if operator lacks permission
- No optimistic updates
- All state refreshed from API after mutation
- Clear error messages for failures

---

### Phase 16 Persistence Requirements

- Operators stored in canonical tables
- Sessions reference operators via foreign keys
- Audit events reference operators via foreign keys
- SQLite foreign key enforcement must be enabled
- Referential integrity must prevent invalid deletions

---

### Phase 16 Testing Requirements

Tests must demonstrate:

- Admins can create operators
- Bidders cannot create or manage operators
- Disabled operators cannot authenticate
- Operators referenced by audit events cannot be deleted
- Operators without audit references can be deleted
- All lifecycle changes emit exactly one audit event
- Authorization failures emit no audit events
- UI hides unauthorized actions

---

### Phase 16 Exit Criteria

Phase 16 is complete when all of the following are true:

- Operators can be fully managed by Admins
- Authorization rules are enforced consistently
- Operator lifecycle actions are auditable
- Referential integrity prevents invalid deletion
- UI correctly gates operator actions
- No domain behavior outside operator management was introduced
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
