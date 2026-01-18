# Phase 26C — Area Metadata Editing

## Objective

Enable safe, lifecycle-gated editing of **area operational metadata** while preserving the immutability of system areas and canonical structure post-canonicalization.

Admins need to update area display names and expected user counts during bootstrap setup. This phase makes those operations explicit, auditable, and lifecycle-aware.

---

## In-Scope

### Backend Changes

1. **Area Display Name Editing**
   - Add `UpdateAreaRequest` / `update_area` endpoint (if missing)
   - Allow editing `area_name` field (display name, not area code)
   - Lifecycle gate: pre-canonicalization only
   - System areas: always immutable (No Bid cannot be renamed)
   - Audit event for area metadata changes

2. **Expected User Count Editing**
   - Endpoint already exists: `set_expected_user_count`
   - Verify lifecycle gating
   - Ensure audit trail

3. **Validation Rules**
   - Area code is immutable (always)
   - Display name is optional metadata
   - System areas cannot be edited (name, expected count, or deletion)
   - Lifecycle state must be < Canonicalized for edits

### Frontend Changes

1. **Area Editing UI**
   - Inline edit for area display name
   - Inline edit for expected user count (already exists in `BootstrapCompleteness.tsx`)
   - Lifecycle-aware gating (disable post-canonicalization)
   - System area badge with disabled edit controls
   - Tooltips explaining restrictions

2. **Area List View Enhancements**
   - Show area display name prominently
   - Distinguish area code (immutable) from display name (editable)
   - Lifecycle state context
   - System area visual distinction

3. **Mobile-First Design**
   - Inline editing works on mobile
   - Touch-friendly controls
   - Stacked layout on small screens

---

## Out-of-Scope

- Area deletion (already restricted to pre-canonicalization)
- Area code changes (immutable by design)
- Area creation (already implemented)
- System area creation (automated, not manual)
- User assignment UI (handled in Phase 26B)
- Bulk area operations
- Area-level capabilities (user capabilities are separate)

---

## Backend Changes Details

### Files Affected

#### New Endpoint (if needed)

**`crates/api/src/request_response.rs`**

Add DTOs:

```rust
/// Request to update area metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UpdateAreaRequest {
    /// The canonical area identifier.
    pub area_id: i64,
    /// The new display name (optional).
    pub area_name: Option<String>,
}

/// Response for successful area update.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UpdateAreaResponse {
    /// The canonical bid year identifier.
    pub bid_year_id: i64,
    /// The bid year (display value).
    pub bid_year: u16,
    /// The canonical area identifier.
    pub area_id: i64,
    /// The area code (immutable).
    pub area_code: String,
    /// The updated display name.
    pub area_name: Option<String>,
    /// Success message.
    pub message: String,
}
```

**`crates/api/src/handlers.rs`**

Add handler:

```rust
pub fn update_area(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &UpdateAreaRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<UpdateAreaResponse, ApiError>
```

Validation logic:

- Enforce admin authorization
- Fetch lifecycle state for bid year containing this area
- Reject if lifecycle state >= Canonicalized
- Reject if area is system area (`is_system_area == true`)
- Update canonical area table
- Create audit event

**`crates/persistence/src/mutations/canonical.rs`**

Add mutation:

```rust
pub fn update_area_name(
    conn: &mut _,
    area_id: i64,
    area_name: Option<&str>,
) -> Result<(), PersistenceError>
```

#### Server Layer

**`crates/server/src/main.rs`**

Add route:

```rust
.route("/api/areas/update", post(handle_update_area))
```

Handler:

```rust
async fn handle_update_area(
    State(app_state): State<AppState>,
    session: AuthSession,
    Json(req): Json<UpdateAreaRequest>,
) -> Result<Json<UpdateAreaResponse>, ServerError>
```

### Lifecycle Enforcement

| Lifecycle State   | Area Name Edit | Expected Count Edit | System Area Edit |
| ----------------- | -------------- | ------------------- | ---------------- |
| Draft             | ✅ Allowed     | ✅ Allowed          | ❌ Never         |
| BootstrapComplete | ✅ Allowed     | ✅ Allowed          | ❌ Never         |
| Canonicalized     | ❌ Denied      | ❌ Denied           | ❌ Never         |
| BiddingActive     | ❌ Denied      | ❌ Denied           | ❌ Never         |
| BiddingClosed     | ❌ Denied      | ❌ Denied           | ❌ Never         |

---

## Frontend Changes Details

### Components Affected

#### Modified Components

**`ui/src/components/AreaView.tsx`**

Current state: Read-only display of areas.

Enhancements:

- Add inline edit button for area display name
- Show lifecycle state badge
- Disable edit for system areas
- Disable edit post-canonicalization
- Tooltip explanations

**`ui/src/components/BootstrapCompleteness.tsx`**

Current state: Has `AreaItem` component with inline edit for expected user count.

Enhancements:

- Add area display name inline edit
- Lifecycle-aware gating (already has some)
- System area detection

#### New Components (Optional)

**`ui/src/components/AreaEditForm.tsx`**

Inline edit form for area metadata:

- Area display name input
- Expected user count input
- Save / Cancel buttons
- Loading state
- Error display

Can be embedded in `AreaItem` or `AreaView`.

#### API Integration

**`ui/src/api.ts`**

Add function:

```typescript
export async function updateArea(
  areaId: number,
  areaName: string | null,
): Promise<UpdateAreaResponse>;
```

#### Type Definitions

**`ui/src/types.ts`**

Add response type:

```typescript
export interface UpdateAreaResponse {
  bid_year_id: number;
  bid_year: number;
  area_id: number;
  area_code: string;
  area_name: string | null;
  message: string;
}
```

---

## Domain & UX Invariants

### Rules That Must Not Be Violated

1. **Area code is immutable**
   - Area code identifies the area (e.g., "North", "South")
   - Cannot be changed after creation
   - UI must never offer area code editing

2. **Display name is optional metadata**
   - `area_name` is for human-readable context
   - Distinct from `area_code`
   - Can be null (no display name)

3. **System areas are immutable**
   - No Bid area cannot be edited, deleted, or renamed
   - UI must clearly indicate system area status
   - Backend must enforce this invariant

4. **Lifecycle as gatekeeper**
   - Pre-canonicalization: metadata is flexible
   - Post-canonicalization: structure is locked
   - Overrides do NOT apply to area metadata (no override for area edits)

5. **Expected user count semantics**
   - No Bid area must not have expected user count
   - Zero users in No Bid is success, not error
   - Expected count is a bootstrap completeness check

6. **Audit trail required**
   - All area metadata changes must create audit events
   - Changes are attributable to an actor

### UX Patterns

#### Area Display

```text
Area: NORTH
Display Name: North Tower Operations
Users: 15 / 20 expected
[Edit Metadata] (if allowed)
```

#### System Area Display

```text
Area: NO BID
🔒 System Area
Users: 0
(Cannot be edited or deleted)
```

#### Lifecycle Restriction

```text
Area: SOUTH
Display Name: [___________]  (disabled)
(?) Tooltip: "Area metadata cannot be changed after canonicalization."
```

---

## Risks & Ambiguities

### 1. Backend Endpoint Existence

**Status**: Need to verify if `update_area` endpoint exists.

**Investigation**: Search codebase for `update_area`, `UpdateAreaRequest`.

**Finding**: No `update_area` endpoint found in Phase 25 assessment.

**Action**: Must implement new endpoint in Phase 26C.

---

### 2. Area Name vs Area Code Confusion

**Risk**: Admins may confuse area code (immutable) with display name (editable).

**Mitigation**:

- UI must clearly label "Area Code" (read-only) vs "Display Name" (editable)
- Use distinct styling (code in monospace, name in regular font)
- Tooltips explain difference

**Example**:

```text
Area Code: NORTH (cannot be changed)
Display Name: [North Tower Operations] (editable pre-canonicalization)
```

---

### 3. Expected User Count for No Bid

**Clarification**: No Bid area should not have an expected user count.

**UI Behavior**:

- Do not show "Expected User Count" field for No Bid
- Or show as "N/A" (not applicable)

**Backend**: Expected count for No Bid should be null or ignored.

---

### 4. Lifecycle State Context

**Question**: Should area editing UI show bid year lifecycle state?

**Recommendation**: Yes. Display lifecycle badge to provide context for why edits are disabled.

**Placement**: At top of area view or inline with area metadata.

---

### 5. Inline Edit vs Modal

**Design Decision**: Inline editing vs modal dialog for area metadata?

**Recommendation**: Inline editing for simplicity and consistency with user editing patterns. Modal is overkill for two fields.

---

### 6. Area Deletion

**Clarification**: Area deletion is out of scope for Phase 26C (already restricted).

**UI**: Do not add delete button. Deletion is bootstrap-phase only and handled elsewhere.

---

## Exit Criteria

Phase 26C is complete when:

1. ✅ `update_area` endpoint implemented (if needed)
2. ✅ Area display name editable pre-canonicalization
3. ✅ Expected user count editable pre-canonicalization
4. ✅ System areas cannot be edited (backend enforces)
5. ✅ Lifecycle state gates area editing (backend and frontend)
6. ✅ Area code remains immutable (never editable)
7. ✅ UI clearly distinguishes area code from display name
8. ✅ Audit events created for area metadata changes
9. ✅ Tooltips explain restrictions
10. ✅ No Bid area does not show expected user count (or shows N/A)
11. ✅ Mobile-friendly inline editing
12. ✅ `cargo xtask ci` passes
13. ✅ `pre-commit run --all-files` passes
14. ✅ Manual testing confirms lifecycle gating works

---

## Implementation Notes

### Suggested Implementation Order

1. **Backend endpoint**
   - Add `UpdateAreaRequest` / `UpdateAreaResponse` DTOs
   - Implement `update_area` handler
   - Add lifecycle state check
   - Add system area check
   - Create audit event

2. **Persistence layer**
   - Add `update_area_name` mutation (if needed)
   - Update canonical areas table

3. **Server layer**
   - Add route for `update_area`
   - Add handler

4. **API wrapper** (`ui/src/api.ts`)
   - Add `updateArea` function

5. **Frontend UI**
   - Add inline edit for area display name
   - Lifecycle-aware gating
   - System area detection

6. **Testing**
   - Backend: lifecycle state enforcement
   - Backend: system area rejection
   - Frontend: inline edit workflow
   - Mobile: touch interaction

---

## UI Design Patterns

### Area Item (Editable State)

**Desktop:**

```text
+----------------------------------------+
| Area: NORTH                            |
| Display Name: North Tower Operations   |
|               [Edit]                   |
| Expected Users: 15 / 20                |
|                 [Edit]                 |
| Lifecycle: Draft                       |
+----------------------------------------+
```

**Editing:**

```text
+----------------------------------------+
| Area: NORTH                            |
| Display Name: [_____________________]  |
|               [Save] [Cancel]          |
| Expected Users: [___] / [___]          |
|                 [Save] [Cancel]        |
+----------------------------------------+
```

### Area Item (System Area)

```text
+----------------------------------------+
| Area: NO BID  🔒 System Area           |
| Users: 0                               |
| (System areas cannot be edited)        |
+----------------------------------------+
```

### Area Item (Canonicalized)

```text
+----------------------------------------+
| Area: SOUTH                            |
| Display Name: South Operations         |
|               [Edit] (disabled)        |
| Expected Users: 10 / 10                |
|                 [Edit] (disabled)      |
| Lifecycle: Canonicalized 🔒            |
| (?) Metadata locked after canonicalization |
+----------------------------------------+
```

---

## Testing Strategy

### Backend Tests

**Unit Tests** (`crates/api/src/handlers.rs`):

- `test_update_area_allowed_in_draft`
- `test_update_area_denied_after_canonicalization`
- `test_update_area_denied_for_system_area`
- `test_update_area_requires_admin`
- `test_update_area_creates_audit_event`

**Integration Tests** (optional):

- Full API flow: create area → update → verify change
- Lifecycle transition: update in draft → canonicalize → attempt update (should fail)

### Frontend Tests

**Manual Testing Checklist:**

**Draft State:**

- [ ] Can edit area display name
- [ ] Can edit expected user count
- [ ] Changes persist after save
- [ ] Cancel restores original value

**System Area:**

- [ ] No Bid area shows system badge
- [ ] Edit buttons disabled or hidden
- [ ] Tooltip explains why

**Canonicalized State:**

- [ ] Edit buttons disabled
- [ ] Tooltip explains lifecycle restriction
- [ ] Lifecycle badge visible

**Mobile:**

- [ ] Inline edit works on 375px width
- [ ] Buttons are tappable
- [ ] No layout overflow

---

## Dependencies

### Required Existing Code

- `Area` domain type with `area_name` field (exists)
- `is_system_area` flag (exists, Phase 25B)
- Lifecycle state enforcement infrastructure (exists, Phase 25A)
- Canonical areas table (exists, Phase 25C)
- `set_expected_user_count` endpoint (exists)

### New Backend Code

- `update_area` endpoint and handler
- `update_area_name` persistence mutation

### Frontend Dependencies

- Inline edit pattern (can reuse from existing components)
- Lifecycle state display (exists)
- No new libraries required

---

## Rollout Considerations

### Backward Compatibility

**API Addition**: New `update_area` endpoint is additive.

**Impact**: No breaking changes. Existing area display remains functional.

**Frontend**: Area views enhanced with editing; read-only views unchanged.

### Rollback Plan

If Phase 26C needs to be reverted:

1. Remove `update_area` route
2. Remove inline edit UI
3. Restore read-only area display

No data migration needed (area metadata already exists).

---

## Non-Goals

- Area deletion enhancements
- Area code editing (immutable)
- Area-level permissions
- Bulk area operations
- CSV import for areas
- Area templates
- Area cloning

---

## Mobile-First Compliance

All UI must follow mobile-first guidelines from `AGENTS.md`:

### Required Patterns

- **Inline editing**: Forms stack vertically on mobile
- **Touch targets**: Edit buttons >= 44px height
- **Labels**: Above inputs on mobile (not side-by-side)
- **Buttons**: Full-width or stacked on mobile

### Component-Specific

**Area Item Inline Edit:**

- Display name input: full width on mobile
- Expected count input: numeric keyboard on mobile
- Save/Cancel buttons: stacked vertically on mobile

**Area List:**

- Area cards stack vertically
- No horizontal scroll
- System area badge visible

---

## Validation Checklist

Before marking Phase 26C complete, verify:

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (backend tests)
- [ ] `npm run build` succeeds
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes
- [ ] Manual test: Edit area name in Draft state (works)
- [ ] Manual test: Edit area name in Canonicalized state (disabled)
- [ ] Manual test: Attempt to edit No Bid area (rejected)
- [ ] Manual test: Expected user count editing works
- [ ] Mobile test: Inline edit usable on 375px width
- [ ] Code review confirms lifecycle enforcement
- [ ] Audit events logged for area changes

---

## Next Phase

**Phase 26D** will implement the No Bid review workflow, making it operationally clear how to resolve users remaining in the No Bid area.
