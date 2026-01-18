# Phase 26F — Override Visibility & Polish

## Objective

Make overridden canonical fields **visible, explainable, and auditable** in the UI, completing the Phase 26 vision of honest, lifecycle-aware administrative workflows.

Phase 25D introduced override semantics with audit trails. Phase 26B implemented override execution UI for area reassignment. Phase 26F closes the loop by making it obvious when canonical data has been overridden, why, and by whom.

This phase is **polish and transparency** — no new domain rules, just better visibility.

---

## In-Scope

### Backend Changes

1. **Override Detection API**
   - Add `is_overridden` flag to relevant entities (`UserInfo`, possibly `AreaInfo`)
   - Indicate if any canonical field has been overridden
   - Efficient query (avoid N+1 lookups)

2. **Override Details Endpoint (Optional)**
   - Fetch override metadata for a specific entity
   - Return override reason, actor, timestamp, audit event ID
   - Scoped to user or area

3. **Override Summary**
   - List which fields are overridden (e.g., "area assignment")
   - Include override reason
   - Link to audit event

### Frontend Changes

1. **Override Badge Display**
   - Show "Overridden" badge on entities with canonical overrides
   - User detail view
   - User list view (inline indicator)
   - Area view (if applicable)

2. **Override Details Tooltip/Panel**
   - Clicking badge or icon shows override details
   - Displays:
     - Which field was overridden (e.g., "Area Assignment")
     - Override reason
     - Actor who performed override
     - Timestamp
     - Audit event ID (if audit browsing exists)
   - Modal or expandable panel

3. **Override Indicator Styling**
   - Visually distinct (e.g., orange badge, icon)
   - Consistent across all views
   - Clear but not alarming (overrides are intentional)

4. **Field-Level Override Markers**
   - Individual fields show if overridden
   - Example: "Area: North (Overridden)"
   - Tooltip on field shows reason

5. **Mobile-First Design**
   - Override badges visible on mobile
   - Details panel scrollable
   - Touch-friendly interaction

---

## Out-of-Scope

- Reverting overrides (rollback is separate workflow)
- Editing override reasons retroactively
- Bulk override operations
- Override approval workflows
- Override permissions beyond admin role
- Audit log full browsing UI (separate feature)
- Historical override tracking (audit log covers this)
- Override analytics or reporting

---

## Backend Changes Details

### Files Affected

#### API Response Enhancement

**`crates/api/src/request_response.rs`**

Update `UserInfo`:

```rust
pub struct UserInfo {
    // ... existing fields ...
    pub capabilities: UserCapabilities,
    /// Whether any canonical field has been overridden.
    pub is_overridden: bool,
}
```

Optional: Add detailed override info type:

```rust
/// Override metadata for a single field.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OverrideInfo {
    /// The field that was overridden (e.g., "area_assignment").
    pub field: String,
    /// The override reason.
    pub reason: String,
    /// The actor who performed the override.
    pub actor: String,
    /// When the override occurred (ISO 8601).
    pub timestamp: String,
    /// The audit event ID.
    pub audit_event_id: i64,
}

/// Response for fetching override details.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GetOverrideDetailsResponse {
    /// The canonical bid year identifier.
    pub bid_year_id: i64,
    /// The bid year (display value).
    pub bid_year: u16,
    /// The user's canonical internal identifier.
    pub user_id: i64,
    /// The user's initials.
    pub initials: String,
    /// List of overridden fields with metadata.
    pub overrides: Vec<OverrideInfo>,
}
```

#### Handler Updates

**`crates/api/src/handlers.rs`**

Update `list_users` handler:

- Query override table to detect overridden users
- Set `is_overridden` flag in `UserInfo`
- Efficient query (JOIN or batched lookup)

Optional: Add handler:

```rust
pub fn get_override_details(
    persistence: &mut SqlitePersistence,
    user_id: i64,
    authenticated_actor: &AuthenticatedActor,
) -> Result<GetOverrideDetailsResponse, ApiError>
```

Fetches override records from override tables (area assignment, eligibility, etc.).

#### Persistence Layer

**`crates/persistence/src/queries/overrides.rs`** (if not exists, create)

Add query:

```rust
pub fn get_user_overrides(
    conn: &mut _,
    user_id: i64,
) -> Result<Vec<OverrideRecord>, PersistenceError>
```

Returns override records with reason, actor, timestamp, audit event ID.

Add query:

```rust
pub fn check_user_has_overrides(
    conn: &mut _,
    user_id: i64,
) -> Result<bool, PersistenceError>
```

Efficient check (returns true if any override exists).

#### Server Layer (Optional)

**`crates/server/src/main.rs`**

Add route (if detailed endpoint needed):

```rust
.route("/api/users/:user_id/overrides", get(handle_get_override_details))
```

---

## Frontend Changes Details

### Components Affected

#### Modified Components

**`ui/src/components/UserDetailView.tsx`**

Enhancements:

- Display override badge if `is_overridden === true`
- Click badge to show override details modal/panel
- Show override indicator on individual fields
- Tooltip on badge: "One or more canonical fields have been overridden"

**`ui/src/components/UserListView.tsx`**

Enhancements:

- Inline override badge/icon in user list rows
- Hover or click shows brief override info
- Visual distinction for overridden users

**`ui/src/components/UserEditView.tsx`** (from Phase 26B)

Enhancements:

- Show override indicator on overridden fields
- Tooltip shows override reason
- Link to full override details

#### New Components

**`ui/src/components/OverrideDetailsPanel.tsx`**

Displays detailed override information:

- List of overridden fields
- Override reason for each
- Actor, timestamp
- Audit event ID (link if audit browsing exists)
- Close button

Can be modal or expandable panel.

Structure:

```typescript
interface OverrideDetailsPanelProps {
  userId: number;
  onClose: () => void;
}

export function OverrideDetailsPanel({
  userId,
  onClose,
}: OverrideDetailsPanelProps) {
  // Fetch override details
  // Display in structured format
}
```

#### API Integration

**`ui/src/api.ts`**

Add function (if detailed endpoint exists):

```typescript
export async function getOverrideDetails(
  userId: number,
): Promise<GetOverrideDetailsResponse>;
```

#### Type Definitions

**`ui/src/types.ts`**

Update `UserInfo`:

```typescript
export interface UserInfo {
  // ... existing fields ...
  capabilities: UserCapabilities;
  is_overridden: boolean;
}
```

Add types (if detailed endpoint exists):

```typescript
export interface OverrideInfo {
  field: string;
  reason: string;
  actor: string;
  timestamp: string;
  audit_event_id: number;
}

export interface GetOverrideDetailsResponse {
  bid_year_id: number;
  bid_year: number;
  user_id: number;
  initials: string;
  overrides: OverrideInfo[];
}
```

---

## Domain & UX Invariants

### Rules That Must Not Be Violated

1. **Override badge is informational**
   - Does not prevent editing or actions
   - Indicates historical override, not current restriction
   - Overrides are permanent until rolled back (separate workflow)

2. **Override reason is immutable**
   - Cannot edit override reason retroactively
   - Reason is part of audit trail
   - Changes require new override or rollback

3. **Override visibility does not grant permissions**
   - Viewing override details does not allow reverting
   - Admin role still required for override actions
   - Read-only display for all users (if visible)

4. **Audit trail integrity**
   - Override details link to audit events
   - No synthetic or computed override data
   - Source of truth is override tables + audit log

5. **Performance consideration**
   - Override detection must be efficient
   - No N+1 queries in user lists
   - Batch or JOIN queries preferred

### UX Patterns

#### Override Badge

```text
User: ABC
Name: Alice Brown
Area: North ⚠️ Overridden
```

Tooltip on badge:

```text
This user's area assignment was overridden.
Click for details.
```

#### Override Details Panel

```text
+------------------------------------------+
| Override Details — User ABC              |
+------------------------------------------+
| Area Assignment Override                 |
| Reason: User requested transfer due to   |
|         medical accommodation.           |
| Overridden by: admin                     |
| Date: 2026-01-20 15:45 UTC               |
| Audit Event: #1234                       |
+------------------------------------------+
| [Close]                                  |
+------------------------------------------+
```

#### Field-Level Indicator

```text
Area: North ⚠️
(?) Tooltip: "Overridden on 2026-01-20. Reason: Medical accommodation."
```

#### User List with Override

```text
+------------------------------------------+
| ABC — Alice Brown            ⚠️          |
| Area: North, Type: CPC, Crew: 1          |
| [Edit] [View Details]                    |
+------------------------------------------+
```

---

## Risks & Ambiguities

### 1. Override Table Schema

**Ambiguity**: Do override tables exist? What is their structure?

**Investigation Needed**: Review Phase 25D implementation.

**Expected**:

- `canonical_area_overrides` table
- `canonical_eligibility_overrides` table
- Columns: user_id, field, reason, actor, timestamp, audit_event_id

**If Missing**: Phase 26F may need to add override detection queries.

---

### 2. Performance of Override Detection

**Risk**: Checking `is_overridden` for every user in `list_users` could be slow.

**Mitigation**:

- Use LEFT JOIN in query
- Batch override checks
- Cache override status in canonical table (denormalized flag)

**Recommendation**: Start with JOIN. Optimize if performance issue observed.

---

### 3. Multiple Overrides on Same User

**Question**: Can a user have multiple overrides (e.g., area + eligibility)?

**Answer**: Yes, per Phase 25D design.

**UI Implication**: Override details panel must list all overrides, not just one.

---

### 4. Override History vs Current Override

**Clarification**: If area is overridden multiple times, which override to show?

**Recommendation**: Show **most recent** override per field. Historical overrides are in audit log.

---

### 5. Override Badge Color/Style

**Design Question**: What color/icon for override badge?

**Recommendation**:

- Color: Orange or amber (informational, not error)
- Icon: ⚠️ or 🔄 (override/change symbol)
- Not red (not an error or violation)

---

### 6. Audit Log Browsing Dependency

**Ambiguity**: If audit log browsing UI does not exist, audit event ID link goes nowhere.

**Recommendation**: If no audit browsing, show audit event ID as plain text (for reference). Link if browsing exists (future).

---

## Exit Criteria

Phase 26F is complete when:

1. ✅ `is_overridden` flag added to `UserInfo` (backend)
2. ✅ Override detection query efficient (no N+1)
3. ✅ Override badge displayed on users with overrides
4. ✅ Override details panel functional (shows reason, actor, timestamp)
5. ✅ Field-level override indicators visible
6. ✅ Tooltips explain override status
7. ✅ Mobile-friendly override details view
8. ✅ Override badge styling consistent across views
9. ✅ Optional: `get_override_details` endpoint implemented
10. ✅ `cargo xtask ci` passes
11. ✅ `pre-commit run --all-files` passes
12. ✅ Manual testing confirms override visibility works

---

## Implementation Notes

### Suggested Implementation Order

1. **Backend override detection**
   - Add `is_overridden` flag to `UserInfo`
   - Update `list_users` query to detect overrides
   - Test performance (JOIN vs batched query)

2. **Optional: Override details endpoint**
   - Add `get_override_details` handler
   - Query override tables for user
   - Return override metadata

3. **Frontend badge display**
   - Add override badge to `UserDetailView`
   - Add override indicator to `UserListView`
   - Style consistently

4. **Override details panel**
   - Create `OverrideDetailsPanel` component
   - Fetch override details (if endpoint exists)
   - Display structured info

5. **Field-level indicators**
   - Add override marker to individual fields
   - Tooltips with override reason

6. **Mobile testing**
   - Test badge visibility on small screens
   - Test details panel scrolling
   - Verify touch interaction

---

## UI Design Patterns

### User Detail View with Override

**Desktop:**

```text
+------------------------------------------------+
| User: ABC — Alice Brown                        |
| Bid Year 2026 — Canonicalized 🔒               |
| ⚠️ Canonical Override Applied                   |
|    [View Override Details]                     |
+------------------------------------------------+
| Initials: ABC                                  |
| Name: Alice Brown                              |
| Area: North ⚠️ (Overridden)                     |
| Type: CPC                                      |
| Crew: 1                                        |
+------------------------------------------------+
| Seniority Information                          |
| Cumulative NATCA BU: 2020-03-15                |
| NATCA BU: 2021-06-01                           |
| EOD/FAA: 2019-01-10                            |
| SCD: 2019-01-10                                |
| Lottery: 42                                    |
+------------------------------------------------+
| [Edit User] [Back to List]                     |
+------------------------------------------------+
```

### Override Details Modal

```text
+------------------------------------------------+
| Override Details — User ABC                    |
+------------------------------------------------+
| This user has 1 canonical override:            |
|                                                |
| Area Assignment Override                       |
| ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ |
| Original Area: South                           |
| Overridden To: North                           |
|                                                |
| Reason:                                        |
| User requested transfer due to medical         |
| accommodation. Approved by facility manager.   |
|                                                |
| Performed by: admin (Jane Doe)                 |
| Date: 2026-01-20 15:45:32 UTC                  |
| Audit Event ID: #1234                          |
+------------------------------------------------+
| [Close]                                        |
+------------------------------------------------+
```

### User List with Override Indicator

```text
+------------------------------------------------+
| Users in Area North                            |
+------------------------------------------------+
| ABC — Alice Brown                         ⚠️   |
| Type: CPC, Crew: 1, Hours: 120 / 150           |
| [Edit] [View Details]                          |
+------------------------------------------------+
| DEF — David Edwards                            |
| Type: CPC-IT, Crew: 2, Hours: 80 / 150         |
| [Edit] [View Details]                          |
+------------------------------------------------+
```

---

## Testing Strategy

### Backend Tests

**Unit Tests** (`crates/api/src/handlers.rs`):

- `test_list_users_includes_override_flag`
- `test_override_flag_true_when_override_exists`
- `test_override_flag_false_when_no_override`
- `test_get_override_details_returns_metadata`
- `test_get_override_details_requires_auth`

**Performance Tests** (optional):

- Measure `list_users` query time with overrides
- Verify no N+1 queries

### Frontend Tests

**Manual Testing Checklist:**

**Override Badge:**

- [ ] Badge displayed on users with overrides
- [ ] Badge not displayed on users without overrides
- [ ] Badge visible in detail view
- [ ] Badge visible in list view
- [ ] Badge tooltip shows helpful text

**Override Details:**

- [ ] Clicking badge opens details panel
- [ ] Details show override reason
- [ ] Details show actor and timestamp
- [ ] Details show audit event ID
- [ ] Panel closes correctly

**Field-Level Indicators:**

- [ ] Overridden field shows indicator
- [ ] Non-overridden fields do not show indicator
- [ ] Tooltip on field shows reason

**Mobile:**

- [ ] Badge visible on 375px width
- [ ] Details panel scrollable
- [ ] Touch interaction works

---

## Dependencies

### Required Existing Code

- Override tables (from Phase 25D)
  - `canonical_area_overrides`
  - `canonical_eligibility_overrides` (or similar)
- `UserInfo` type
- `list_users` endpoint
- Override execution endpoints (from Phase 26B)

### Optional Backend

- `get_override_details` endpoint (nice-to-have, not required)
- Can derive from existing override tables

### Frontend Dependencies

- Modal/panel component (or create new)
- Tooltip component (likely exists)
- No new external libraries

---

## Rollout Considerations

### Backward Compatibility

**API Change**: `UserInfo` gains `is_overridden` field (additive).

**Impact**: Frontend must handle new field. Older clients see field but may ignore.

**Migration**: None required (computed flag, not stored).

### Rollback Plan

If Phase 26F needs to be reverted:

1. Remove `is_overridden` from `UserInfo`
2. Remove override badge UI
3. Remove override details panel

Backend override tables remain (unchanged from Phase 25D).

---

## Non-Goals

- Reverting or undoing overrides (rollback is separate)
- Editing override reasons retroactively
- Override approval workflows
- Override analytics dashboard
- Full audit log browsing UI (separate feature)
- Override expiration or time-based rules
- Multi-level override permissions

---

## Mobile-First Compliance

All UI must follow mobile-first guidelines from `AGENTS.md`:

### Required Patterns

- **Badges**: Visible and tappable on mobile
- **Details panel**: Scrollable modal on mobile
- **Touch targets**: Close button >= 44px
- **Text wrapping**: Override reason text wraps on small screens
- **No horizontal scroll**: Panel fits within viewport width

### Component-Specific

**Override Badge:**

- Minimum size 24px (visible, tappable)
- Positioned to not overlap other content
- Tooltip replaced with tap-to-expand on mobile

**Override Details Panel:**

- Full-screen modal on mobile (or large panel)
- Scrollable if content exceeds viewport
- Close button in top-right corner
- Text wraps appropriately

---

## Validation Checklist

Before marking Phase 26F complete, verify:

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (backend tests)
- [ ] `npm run build` succeeds
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes
- [ ] Manual test: User with override shows badge
- [ ] Manual test: User without override has no badge
- [ ] Manual test: Override details panel displays correctly
- [ ] Manual test: Field-level indicators visible
- [ ] Mobile test: Badge visible and tappable on 375px width
- [ ] Performance test: `list_users` query remains fast
- [ ] Code review confirms override detection is efficient

---

## Phase 26 Completion

Phase 26F is the final sub-phase of Phase 26.

Upon completion of Phase 26F, the following should be true:

1. ✅ Backend capabilities are lifecycle-aware (26A)
2. ✅ User editing is honest and lifecycle-gated (26B)
3. ✅ Area metadata is editable with lifecycle awareness (26C)
4. ✅ No Bid review workflow is operational (26D)
5. ✅ Bid year metadata is editable (26E)
6. ✅ Overrides are visible and transparent (26F)

**Phase 26 Exit Criteria:**

- Admins can clearly see what is editable and why
- All allowed edits are surfaced and functional
- Forbidden edits are disabled with explanation
- No Bid workflow is operable and intuitive
- Canonical and override data are transparent
- No domain invariants from Phase 25 are weakened

**System State After Phase 26:**

The system is now **operationally complete** for bootstrap and lifecycle management. Admins have honest, clear tools to:

- Set up bid years
- Configure areas
- Manage users
- Understand and resolve blockers
- Apply overrides when needed
- See when overrides have been applied

**Next Steps Beyond Phase 26:**

- Bidding logic implementation
- Round management
- Proxy bidding
- Audit log browsing
- Performance optimization
- Bulk operations
- Historical reporting

Phase 26 establishes the **foundation for operational integrity**. All future phases build on this solid ground.
