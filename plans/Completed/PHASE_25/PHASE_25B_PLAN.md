# Phase 25B Implementation Plan — "No Bid" Area Formalization

## Status

**DRAFT** — Awaiting approval before implementation

---

## Executive Summary

Phase 25B introduces a system-managed "No Bid" area that serves as:

1. **Staging area** for users without finalized area assignments
2. **Manual review queue** during bootstrap
3. **Deletion sink** (pre-canonicalization only)

Bootstrap cannot be completed while users remain in the No Bid area, forcing explicit administrative review.

---

## Dependencies

### Completed Phases

- **Phase 25A**: Lifecycle state machine (Draft → BootstrapComplete → Canonicalized → BiddingActive → BiddingClosed)

### Deferred to Future Phases

- **Phase 25C**: Canonical data tables
- **Phase 25D**: Override semantics
- **Import functionality**: Will use No Bid as staging area

---

## Schema Changes

### Migration: Add `is_system_area` to `areas`

#### SQLite (`migrations/2026-01-17-000002_add_system_area_flag/up.sql`)

```sql
-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Phase 25B: Add system area flag
ALTER TABLE areas ADD COLUMN is_system_area INTEGER NOT NULL DEFAULT 0 CHECK(is_system_area IN (0, 1));
```

#### MySQL (`migrations_mysql/2026-01-17-000002_add_system_area_flag/up.sql`)

```sql
-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

-- Phase 25B: Add system area flag
ALTER TABLE areas ADD COLUMN is_system_area TINYINT NOT NULL DEFAULT 0 CHECK(is_system_area IN (0, 1));
```

#### Down Migration (both backends)

```sql
-- Copyright (C) 2026 Fred Clausen
-- Use of this source code is governed by an MIT-style
-- license that can be found in the LICENSE file or at
-- https://opensource.org/licenses/MIT.

ALTER TABLE areas DROP COLUMN is_system_area;
```

### Constraints

- `is_system_area` is a boolean (0 = false, 1 = true)
- Default value is `0` (not a system area)
- At most one area per bid year may have `is_system_area = true`

---

## Domain Layer Changes

### File: `crates/domain/src/error.rs`

#### New Error Variants

```rust
/// System area already exists for this bid year.
SystemAreaAlreadyExists {
    /// The bid year.
    bid_year: u16,
},

/// Cannot complete bootstrap while users remain in No Bid area.
UsersInNoBidArea {
    /// The bid year.
    bid_year: u16,
    /// Count of users still in No Bid area.
    user_count: usize,
    /// Sample of user initials (first 5).
    sample_initials: Vec<String>,
},

/// Cannot delete a system area.
CannotDeleteSystemArea {
    /// The area code.
    area_code: String,
},

/// Cannot rename a system area.
CannotRenameSystemArea {
    /// The area code.
    area_code: String,
},

/// Cannot delete users after canonicalization.
CannotDeleteUserAfterCanonicalization {
    /// The bid year.
    bid_year: u16,
    /// The lifecycle state.
    lifecycle_state: String,
},

/// Cannot assign users to No Bid area after canonicalization.
CannotAssignToNoBidAfterCanonicalization {
    /// The bid year.
    bid_year: u16,
    /// The lifecycle state.
    lifecycle_state: String,
},
```

#### Display Implementation

```rust
Self::SystemAreaAlreadyExists { bid_year } => {
    write!(f, "System area already exists for bid year {bid_year}")
}
Self::UsersInNoBidArea { bid_year, user_count, sample_initials } => {
    write!(
        f,
        "Cannot complete bootstrap for bid year {}: {} user(s) remain in No Bid area (sample: {})",
        bid_year,
        user_count,
        sample_initials.join(", ")
    )
}
Self::CannotDeleteSystemArea { area_code } => {
    write!(f, "Cannot delete system area '{area_code}'")
}
Self::CannotRenameSystemArea { area_code } => {
    write!(f, "Cannot rename system area '{area_code}'")
}
Self::CannotDeleteUserAfterCanonicalization { bid_year, lifecycle_state } => {
    write!(
        f,
        "Cannot delete user after canonicalization (bid year {}, state: {})",
        bid_year, lifecycle_state
    )
}
Self::CannotAssignToNoBidAfterCanonicalization { bid_year, lifecycle_state } => {
    write!(
        f,
        "Cannot assign user to No Bid area after canonicalization (bid year {}, state: {})",
        bid_year, lifecycle_state
    )
}
```

### File: `crates/domain/src/types.rs`

#### Area Type Extension

Add field and methods to `Area` struct:

```rust
pub struct Area {
    area_id: Option<i64>,
    area_code: String,
    area_name: Option<String>,
    /// Phase 25B: Whether this is a system-managed area (e.g., "No Bid").
    is_system_area: bool,
}

impl Area {
    /// Creates a new regular (non-system) `Area`.
    pub fn new(area_code: &str) -> Self {
        Self {
            area_id: None,
            area_code: area_code.to_uppercase(),
            area_name: None,
            is_system_area: false,
        }
    }

    /// Creates a new system area (e.g., "No Bid").
    pub fn new_system_area(area_code: &str) -> Self {
        Self {
            area_id: None,
            area_code: area_code.to_uppercase(),
            area_name: None,
            is_system_area: true,
        }
    }

    /// Creates an `Area` with an existing persisted ID.
    pub fn with_id(
        area_id: i64,
        area_code: &str,
        area_name: Option<String>,
        is_system_area: bool,
    ) -> Self {
        Self {
            area_id: Some(area_id),
            area_code: area_code.to_uppercase(),
            area_name,
            is_system_area,
        }
    }

    /// Returns whether this is a system-managed area.
    pub const fn is_system_area(&self) -> bool {
        self.is_system_area
    }

    /// Returns the canonical name for the No Bid system area.
    pub const NO_BID_AREA_CODE: &'static str = "NO BID";
}
```

---

## Core Layer Changes

### File: `crates/core/src/command.rs`

No new commands needed. The "No Bid" area is created automatically during bid year bootstrap, not as an explicit command.

### Modifications to Existing Commands

None at the core layer. Command validation happens in `apply.rs`.

---

## Audit Events

### File: `crates/audit/src/event.rs`

#### New Event Variants

```rust
/// No Bid system area created during bid year bootstrap.
NoBidAreaCreated {
    /// The bid year ID.
    bid_year_id: i64,
    /// The area ID.
    area_id: i64,
    /// The area code (always "NO BID").
    area_code: String,
    /// Timestamp of the event.
    timestamp: OffsetDateTime,
},

/// User moved to No Bid area (pre-canonicalization deletion).
UserMovedToNoBid {
    /// The bid year ID.
    bid_year_id: i64,
    /// The user ID.
    user_id: i64,
    /// The user's initials.
    initials: String,
    /// The previous area ID.
    previous_area_id: i64,
    /// The previous area code.
    previous_area_code: String,
    /// The reason for the move.
    reason: String,
    /// Timestamp of the event.
    timestamp: OffsetDateTime,
},
```

---

## Persistence Layer Changes

### File: `crates/persistence/src/sqlite.rs`

#### New Queries

```rust
/// Finds the system area (No Bid) for a given bid year.
///
/// Returns `None` if no system area exists.
pub fn find_system_area(
    &mut self,
    bid_year_id: i64,
) -> Result<Option<(i64, String)>, PersistenceError> {
    use crate::schema::areas::dsl::*;

    let result: Option<(i64, String)> = areas
        .filter(bid_year_id.eq(bid_year_id))
        .filter(is_system_area.eq(1))
        .select((area_id, area_code))
        .first(&mut self.connection)
        .optional()?;

    Ok(result)
}

/// Counts users in the system area (No Bid) for a given bid year.
pub fn count_users_in_system_area(
    &mut self,
    bid_year_id: i64,
) -> Result<usize, PersistenceError> {
    use crate::schema::areas::dsl as area_dsl;
    use crate::schema::users::dsl as user_dsl;

    // First find the system area ID
    let system_area_id: Option<i64> = area_dsl::areas
        .filter(area_dsl::bid_year_id.eq(bid_year_id))
        .filter(area_dsl::is_system_area.eq(1))
        .select(area_dsl::area_id)
        .first(&mut self.connection)
        .optional()?;

    if let Some(sys_area_id) = system_area_id {
        let count: i64 = user_dsl::users
            .filter(user_dsl::bid_year_id.eq(bid_year_id))
            .filter(user_dsl::area_id.eq(sys_area_id))
            .count()
            .get_result(&mut self.connection)?;

        Ok(count as usize)
    } else {
        Ok(0)
    }
}

/// Lists users in the system area (No Bid) for a given bid year.
///
/// Returns up to `limit` user initials.
pub fn list_users_in_system_area(
    &mut self,
    bid_year_id: i64,
    limit: i64,
) -> Result<Vec<String>, PersistenceError> {
    use crate::schema::areas::dsl as area_dsl;
    use crate::schema::users::dsl as user_dsl;

    // First find the system area ID
    let system_area_id: Option<i64> = area_dsl::areas
        .filter(area_dsl::bid_year_id.eq(bid_year_id))
        .filter(area_dsl::is_system_area.eq(1))
        .select(area_dsl::area_id)
        .first(&mut self.connection)
        .optional()?;

    if let Some(sys_area_id) = system_area_id {
        let initials: Vec<String> = user_dsl::users
            .filter(user_dsl::bid_year_id.eq(bid_year_id))
            .filter(user_dsl::area_id.eq(sys_area_id))
            .select(user_dsl::initials)
            .limit(limit)
            .load(&mut self.connection)?;

        Ok(initials)
    } else {
        Ok(Vec::new())
    }
}

/// Checks if an area is a system area.
pub fn is_system_area(
    &mut self,
    area_id: i64,
) -> Result<bool, PersistenceError> {
    use crate::schema::areas::dsl::*;

    let system_flag: i32 = areas
        .filter(area_id.eq(area_id))
        .select(is_system_area)
        .first(&mut self.connection)?;

    Ok(system_flag != 0)
}

/// Creates a system area (No Bid) for a bid year.
pub fn create_system_area(
    &mut self,
    bid_year_id: i64,
    area_code: &str,
) -> Result<i64, PersistenceError> {
    use crate::schema::areas::dsl::*;

    #[derive(Insertable)]
    #[diesel(table_name = crate::schema::areas)]
    struct NewSystemArea<'a> {
        bid_year_id: i64,
        area_code: &'a str,
        is_system_area: i32,
    }

    diesel::insert_into(areas)
        .values(&NewSystemArea {
            bid_year_id,
            area_code,
            is_system_area: 1,
        })
        .execute(&mut self.connection)?;

    // SQLite-specific: get the last inserted row ID
    let new_area_id: i64 = diesel::select(diesel::dsl::sql::<diesel::sql_types::BigInt>(
        "last_insert_rowid()",
    ))
    .get_result(&mut self.connection)?;

    Ok(new_area_id)
}

/// Moves a user to the system area (No Bid).
pub fn move_user_to_system_area(
    &mut self,
    user_id: i64,
    system_area_id: i64,
) -> Result<(), PersistenceError> {
    use crate::schema::users::dsl::*;

    diesel::update(users.filter(user_id.eq(user_id)))
        .set(area_id.eq(system_area_id))
        .execute(&mut self.connection)?;

    Ok(())
}
```

#### MySQL Backend Equivalents

Create `crates/persistence/src/mysql.rs` with monomorphic implementations of the same queries using MySQL-specific syntax where needed.

---

## API Layer Changes

### File: `crates/api/src/handlers.rs`

#### Modified Handler: `transition_to_bootstrap_complete`

Add check for users in No Bid area before allowing transition:

```rust
pub fn transition_to_bootstrap_complete(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &TransitionToBootstrapCompleteRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<TransitionToBootstrapCompleteResponse, ApiError> {
    // ... existing authorization and state validation ...

    // Phase 25B: Check for users in No Bid area
    let users_in_no_bid: usize = persistence
        .count_users_in_system_area(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to check No Bid area: {e}"),
        })?;

    if users_in_no_bid > 0 {
        let sample_initials: Vec<String> = persistence
            .list_users_in_system_area(request.bid_year_id, 5)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to list users in No Bid area: {e}"),
            })?;

        return Err(translate_domain_error(DomainError::UsersInNoBidArea {
            bid_year: year,
            user_count: users_in_no_bid,
            sample_initials,
        }));
    }

    // ... proceed with existing logic ...
}
```

#### Modified Handler: `create_bid_year`

Auto-create No Bid area during bid year bootstrap:

```rust
pub fn create_bid_year(
    persistence: &mut SqlitePersistence,
    metadata: &mut BootstrapMetadata,
    request: &CreateBidYearRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<CreateBidYearResponse, ApiError> {
    // ... existing bid year creation logic ...

    // Phase 25B: Auto-create No Bid system area
    let no_bid_area_id: i64 = persistence
        .create_system_area(new_bid_year_id, Area::NO_BID_AREA_CODE)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to create No Bid area: {e}"),
        })?;

    // Emit audit event for No Bid area creation
    let no_bid_event = AuditEvent::NoBidAreaCreated {
        bid_year_id: new_bid_year_id,
        area_id: no_bid_area_id,
        area_code: Area::NO_BID_AREA_CODE.to_string(),
        timestamp: OffsetDateTime::now_utc(),
    };

    persistence
        .persist_audit_event(&no_bid_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist No Bid area audit event: {e}"),
        })?;

    // ... return response ...
}
```

#### Modified Handler: `list_areas`

Filter system areas from public endpoints:

```rust
pub fn list_areas(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &ListAreasRequest,
    authenticated_actor: Option<&AuthenticatedActor>,
) -> Result<ListAreasResponse, ApiError> {
    // ... existing logic ...

    // Phase 25B: Filter system areas for non-admin users
    let areas: Vec<AreaInfo> = if authenticated_actor
        .map_or(false, |actor| actor.role == Role::Admin)
    {
        // Admin: include all areas
        all_areas
    } else {
        // Public: exclude system areas
        all_areas
            .into_iter()
            .filter(|area| !area.is_system_area)
            .collect()
    };

    Ok(ListAreasResponse { areas })
}
```

### File: `crates/api/src/request_response.rs`

#### Modified Response Types

Add `is_system_area` field to `AreaInfo`:

```rust
pub struct AreaInfo {
    pub area_id: i64,
    pub area_code: String,
    pub area_name: Option<String>,
    pub user_count: usize,
    /// Phase 25B: Whether this is a system-managed area.
    pub is_system_area: bool,
}
```

### File: `crates/api/src/error.rs`

Update `translate_domain_error` for new error variants:

```rust
DomainError::SystemAreaAlreadyExists { bid_year } => {
    ApiError::DomainRuleViolation {
        rule: String::from("system_area_uniqueness"),
        message: format!("System area already exists for bid year {bid_year}"),
    }
}
DomainError::UsersInNoBidArea { bid_year, user_count, sample_initials } => {
    ApiError::DomainRuleViolation {
        rule: String::from("no_bid_area_empty"),
        message: format!(
            "Cannot complete bootstrap for bid year {}: {} user(s) remain in No Bid area ({})",
            bid_year,
            user_count,
            sample_initials.join(", ")
        ),
    }
}
DomainError::CannotDeleteSystemArea { area_code } => {
    ApiError::DomainRuleViolation {
        rule: String::from("system_area_immutable"),
        message: format!("Cannot delete system area '{area_code}'"),
    }
}
DomainError::CannotRenameSystemArea { area_code } => {
    ApiError::DomainRuleViolation {
        rule: String::from("system_area_immutable"),
        message: format!("Cannot rename system area '{area_code}'"),
    }
}
DomainError::CannotDeleteUserAfterCanonicalization { bid_year, lifecycle_state } => {
    ApiError::DomainRuleViolation {
        rule: String::from("no_deletion_after_canonicalization"),
        message: format!(
            "Cannot delete user after canonicalization (bid year {}, state: {})",
            bid_year, lifecycle_state
        ),
    }
}
DomainError::CannotAssignToNoBidAfterCanonicalization { bid_year, lifecycle_state } => {
    ApiError::DomainRuleViolation {
        rule: String::from("no_assignment_to_no_bid_after_canonicalization"),
        message: format!(
            "Cannot assign user to No Bid area after canonicalization (bid year {}, state: {})",
            bid_year, lifecycle_state
        ),
    }
}
```

---

## Testing Strategy

### File: `crates/core/src/tests/phase_25b_no_bid.rs`

```rust
//! Phase 25B: No Bid area tests

use super::*;

#[test]
fn test_no_bid_area_created_during_bootstrap() {
    // Test that No Bid area is auto-created when bid year is created
}

#[test]
fn test_cannot_complete_bootstrap_with_users_in_no_bid() {
    // Test that transition to BootstrapComplete fails if users in No Bid
}

#[test]
fn test_can_complete_bootstrap_with_empty_no_bid() {
    // Test that transition succeeds when No Bid is empty
}

#[test]
fn test_system_area_cannot_be_deleted() {
    // Test that attempting to delete No Bid area fails
}

#[test]
fn test_system_area_cannot_be_renamed() {
    // Test that attempting to rename No Bid area fails
}

#[test]
fn test_user_deletion_moves_to_no_bid_pre_canonicalization() {
    // Test that "deleting" a user pre-canonicalization moves them to No Bid
}

#[test]
fn test_user_deletion_fails_post_canonicalization() {
    // Test that user deletion is not allowed after canonicalization
}

#[test]
fn test_cannot_assign_to_no_bid_post_canonicalization() {
    // Test that moving users INTO No Bid fails after canonicalization
}

#[test]
fn test_can_move_out_of_no_bid_post_canonicalization() {
    // Test that moving users OUT of No Bid is allowed (cleanup scenario)
}

#[test]
fn test_no_bid_area_hidden_from_public_api() {
    // Test that public endpoints exclude system areas
}

#[test]
fn test_no_bid_area_visible_to_admin() {
    // Test that admin endpoints include system areas
}

#[test]
fn test_audit_event_for_no_bid_creation() {
    // Test that NoBidAreaCreated event is emitted
}

#[test]
fn test_audit_event_for_user_moved_to_no_bid() {
    // Test that UserMovedToNoBid event is emitted
}
```

### File: `crates/persistence/src/tests/phase_25b_no_bid.rs`

```rust
//! Phase 25B: Persistence layer tests for No Bid area

#[test]
fn test_find_system_area() {
    // Test finding the No Bid area
}

#[test]
fn test_count_users_in_system_area() {
    // Test counting users in No Bid
}

#[test]
fn test_list_users_in_system_area() {
    // Test listing users in No Bid with limit
}

#[test]
fn test_is_system_area() {
    // Test checking if an area is a system area
}

#[test]
fn test_create_system_area() {
    // Test creating a system area
}

#[test]
fn test_move_user_to_system_area() {
    // Test moving a user to No Bid
}
```

### Integration Tests

- End-to-end workflow: create bid year → users in No Bid → cannot complete bootstrap
- End-to-end workflow: create bid year → assign users to areas → complete bootstrap
- API visibility tests (public vs admin)

---

## Implementation Checklist

### Phase 1: Schema & Migrations

- [ ] Create SQLite migration for `is_system_area` column
- [ ] Create MySQL migration for `is_system_area` column
- [ ] Run `cargo xtask verify-migrations` to confirm parity
- [ ] Update Diesel schema files

### Phase 2: Domain Layer

- [ ] Add new error variants to `DomainError`
- [ ] Add `is_system_area` field to `Area` type
- [ ] Add `NO_BID_AREA_CODE` constant
- [ ] Add `new_system_area()` and `is_system_area()` methods
- [ ] Update `Area::with_id()` signature
- [ ] Update domain tests

### Phase 3: Audit Events

- [ ] Add `NoBidAreaCreated` event variant
- [ ] Add `UserMovedToNoBid` event variant
- [ ] Update audit serialization/deserialization
- [ ] Update audit tests

### Phase 4: Persistence Layer

- [ ] Implement SQLite queries for system area operations
- [ ] Implement MySQL queries for system area operations
- [ ] Update persistence tests
- [ ] Test with both backends

### Phase 5: API Layer

- [ ] Update `create_bid_year` to auto-create No Bid area
- [ ] Update `transition_to_bootstrap_complete` to check No Bid
- [ ] Update `list_areas` to filter system areas
- [ ] Update `AreaInfo` response type
- [ ] Update error translation
- [ ] Update API tests

### Phase 6: Integration & Validation

- [ ] Run `cargo xtask ci`
- [ ] Run `pre-commit run --all-files`
- [ ] Manual testing via `api_cli.py`
- [ ] Git add all modified files

### Phase 7: Documentation

- [ ] Update `api_cli.py` for new response fields
- [ ] Update dictionary if needed
- [ ] Mark phase as complete in AGENTS.md

---

## Exit Criteria

Phase 25B is complete when:

1. ✅ `is_system_area` column exists in `areas` table (both SQLite and MySQL)
2. ✅ No Bid area auto-created during bid year bootstrap
3. ✅ Bootstrap completeness check fails if users in No Bid area
4. ✅ System areas cannot be deleted
5. ✅ System areas cannot be renamed
6. ✅ System areas hidden from public API endpoints
7. ✅ System areas visible to admin endpoints
8. ✅ All tests pass (`cargo xtask ci`)
9. ✅ Pre-commit hooks pass (`pre-commit run --all-files`)
10. ✅ Migrations verified (`cargo xtask verify-migrations`)
11. ✅ No breaking changes to existing APIs
12. ✅ `api_cli.py` updated for new response fields

---

## Open Questions / Decisions Needed

1. **User deletion command**: Should we add an explicit `DeleteUser` command, or handle this as a special case of `UpdateUser` that moves to No Bid?

2. **Area deletion command**: Should we add an explicit `DeleteArea` command, or is this out of scope until later?

3. **Rename semantics**: Do we need a `RenameArea` command, or is this handled via `UpdateArea`?

4. **Bootstrap completeness message**: Should the error include ALL user initials in No Bid, or just a sample?

5. **MySQL backend**: Should we implement MySQL persistence in parallel, or defer to later?

---

## Notes

- This phase does NOT introduce user deletion or area deletion commands
- This phase only establishes the infrastructure for No Bid as a staging area
- Future import functionality will rely on this infrastructure
- The No Bid area concept aligns with the audit-first, explicit state philosophy
- System areas are a general concept; No Bid is the first instance

---

## References

- Phase 25A: Lifecycle state machine
- AGENTS.md: Domain invariants, audit rules, persistence rules
- Existing `areas` table schema
- Existing `Area` domain type
