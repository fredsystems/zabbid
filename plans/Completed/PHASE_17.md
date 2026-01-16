# Phase 17

## Phase 17: Operator Credential Management & Password Enforcement

### Phase 17 Goal

Finalize the operator authentication model by implementing secure, auditable, and user-friendly password management for all operators.

Phase 17 ensures that:

- No operator credentials are insecure by default
- All operators can manage their own credentials safely
- Admins can recover or reset access without compromising auditability
- Authentication is robust enough for long-term use without overengineering

This phase **closes the authentication and operator lifecycle loop**.

---

### Phase 17 Scope

Phase 17 includes:

- Secure password creation for operators
- Enforced password policy validation
- Self-service password changes for operators
- Admin-initiated password resets
- Session invalidation on password changes
- Full audit coverage for all credential-related actions
- UI support for all password workflows

Phase 17 explicitly excludes:

- Multi-factor authentication (2FA)
- External identity providers (OAuth, SSO, LDAP)
- Password recovery via email or out-of-band channels
- Account lockout policies beyond disable/enable
- Public-facing user authentication (non-operators)

---

### Phase 17 Operator Credential Rules

#### Password Storage

- Passwords must be:
  - Hashed using a secure, modern algorithm
  - Salted
  - Never stored or logged in plaintext
- Passwords must never appear in:
  - Audit events
  - Logs
  - Error messages
  - API responses

---

#### Password Policy (Backend-Enforced)

Passwords must satisfy all of the following:

- Minimum length: **12 characters**
- Must satisfy at least **3 of 4**:
  - Uppercase letter
  - Lowercase letter
  - Digit
  - Symbol
- Must not equal:
  - login_name
  - display_name
- Validation must be deterministic and constant-time

Invalid passwords must fail explicitly with structured errors.

---

### Phase 17 Operator Flows

#### Operator Creation

- Password is **required** at creation
- Password and confirmation must match
- Password policy is enforced
- Admin-only action
- Emits audit event:
  - actor = admin operator
  - target_operator = created operator

No default or placeholder passwords are permitted.

---

#### Self-Service Password Change

Any authenticated operator may change their own password.

Requirements:

- Current password must be provided
- New password must satisfy policy
- Password confirmation required
- All active sessions for that operator are invalidated

Audit requirements:

- Exactly one audit event emitted on success
- actor = operator
- target_operator = same operator
- Failed attempts emit no audit events

---

#### Admin Password Reset

Admins may reset the password of another operator.

Requirements:

- Admin authentication required
- New password must satisfy policy
- Old password is not required
- All active sessions for the target operator are invalidated

Audit requirements:

- Exactly one audit event emitted on success
- actor = admin
- target_operator = affected operator

---

### Phase 17 Session Semantics

- Password changes (self or admin-initiated) must:
  - Invalidate all existing sessions for the operator
- Sessions remain ephemeral and non-audited
- Authentication failures emit no audit events

---

### Phase 17 UI Requirements

#### Operator UI

- “Change Password” option must always be visible
- Clear validation feedback before submission
- Forced logout after successful password change

#### Admin UI

- Operator creation includes password fields
- Admin password reset action available per operator
- Disabled operators cannot authenticate
- Deleted operators only allowed if not referenced by audit events

UI validation must assist, but backend validation is authoritative.

---

### Phase 17 API Requirements

New or updated endpoints must support:

- Operator creation with password
- Operator self password change
- Admin password reset for operators

All credential-related endpoints must:

- Require authentication
- Enforce authorization
- Return structured errors
- Never expose sensitive data

`api_cli.py` must be updated to reflect all API changes.

---

### Phase 17 Audit Semantics

- All credential mutations emit audit events
- Audit events must include:
  - actor_operator_id
  - actor_login_name
  - target_operator_id (when applicable)
- Password values are never recorded
- Authentication and authorization failures emit no audit events

---

### Phase 17 Testing Requirements

Tests must demonstrate:

- Password policy enforcement
- Successful operator creation with valid passwords
- Explicit failure on invalid passwords
- Self-service password change behavior
- Admin password reset behavior
- Session invalidation after password changes
- Authorization enforcement (admin vs bidder)
- No audit events on failed credential operations

All tests must pass under:

- `cargo test --all-targets --all-features`
- `cargo xtask ci`
- `pre-commit run --all-files`

---

### Phase 17 Exit Criteria

Phase 17 is complete when all of the following are true:

- No operator has a default or placeholder password
- Password policy is enforced consistently
- Operators can change their own passwords
- Admins can reset other operators’ passwords
- Sessions are invalidated correctly
- All credential changes are auditable
- No sensitive data leaks occur
- UI supports all credential workflows
- `api_cli.py` matches the API surface
- CI and pre-commit checks pass consistently
