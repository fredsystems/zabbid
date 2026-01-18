# Phase 26E — Bid Year Metadata Editing

## Objective

Enable admins to edit **non-structural bid year metadata** (labels, notes, descriptive fields) for operational context and documentation purposes, while preserving the immutability of structural fields post-canonicalization.

This phase is **optional** and **low-risk**. It provides operational convenience without affecting domain rules or lifecycle enforcement.

---

## In-Scope

### Backend Changes

1. **Bid Year Metadata Fields**
   - Add optional metadata columns to canonical bid years table:
     - `label` (short descriptive text, e.g., "FY2026 Primary")
     - `notes` (longer operational notes, e.g., "First year with new leave rules")
   - These fields are informational only
   - Do NOT affect domain logic or lifecycle

2. **Update Endpoint**
   - Add `UpdateBidYearMetadataRequest` / `update_bid_year_metadata`
   - Allow editing metadata fields in any lifecycle state
   - Structural fields (year, start_date, num_pay_periods) remain immutable post-creation
   - Audit event for metadata changes

3. **Lifecycle Independence**
   - Metadata edits allowed in all lifecycle states
   - No lifecycle gating for metadata (only for structural changes)
   - Metadata does not trigger lifecycle transitions

### Frontend Changes

1. **Bid Year Metadata Display**
   - Show label and notes in bid year overview
   - Inline edit for metadata fields
   - Clear visual distinction from structural fields
   - Lifecycle state displayed for context

2. **Lifecycle History Display (Read-Only)**
   - Show current lifecycle state
   - Show when state was last changed (if trackable)
   - Read-only view of lifecycle progression
   - Link to audit log (if available)

3. **Canonicalization Status Panel**
   - Display whether bid year is canonicalized
   - Show blocking conditions (if any)
   - Link to bootstrap completeness
   - Visual indicator of readiness

4. **Mobile-First Design**
   - Metadata editing works on mobile
   - Touch-friendly controls
   - Responsive layout

---

## Out-of-Scope

- Structural field editing (year, dates, pay periods) post-creation
- Lifecycle state transitions (already handled in Phase 25A)
- Active bid year selection (already implemented)
- Expected area count (already implemented)
- Area-level metadata (Phase 26C)
- User-level metadata (Phase 26B)
- Audit log browsing (future phase)
- Historical rollback UI
- Snapshot management UI

---

## Backend Changes Details

### Files Affected

#### Schema Changes

**`migrations/*/add_bid_year_metadata.sql`** (new migration)

Add columns to canonical bid years table:

```sql
ALTER TABLE canonical_bid_years
ADD COLUMN label TEXT NULL;

ALTER TABLE canonical_bid_years
ADD COLUMN notes TEXT NULL;
```

**Note**: Must create equivalent migrations for both SQLite and MySQL backends.

#### DTOs

**`crates/api/src/request_response.rs`**

Add request/response types:

```rust
/// Request to update bid year metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UpdateBidYearMetadataRequest {
    /// The canonical bid year identifier.
    pub bid_year_id: i64,
    /// Optional short label (e.g., "FY2026 Primary").
    pub label: Option<String>,
    /// Optional operational notes.
    pub notes: Option<String>,
}

/// Response for successful bid year metadata update.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UpdateBidYearMetadataResponse {
    /// The canonical bid year identifier.
    pub bid_year_id: i64,
    /// The bid year (display value).
    pub year: u16,
    /// The updated label.
    pub label: Option<String>,
    /// The updated notes.
    pub notes: Option<String>,
    /// Success message.
    pub message: String,
}
```

Update `BidYearInfo` to include metadata:

```rust
pub struct BidYearInfo {
    // ... existing fields ...
    pub lifecycle_state: String,
    /// Optional short label.
    pub label: Option<String>,
    /// Optional operational notes.
    pub notes: Option<String>,
}
```

#### Handler

**`crates/api/src/handlers.rs`**

Add handler:

```rust
pub fn update_bid_year_metadata(
    persistence: &mut SqlitePersistence,
    request: &UpdateBidYearMetadataRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<UpdateBidYearMetadataResponse, ApiError>
```

Validation logic:

- Enforce admin authorization
- Bid year must exist
- No lifecycle gating (metadata edits allowed in all states)
- Create audit event for metadata change
- Update canonical bid year record

#### Persistence

**`crates/persistence/src/mutations/canonical.rs`**

Add mutation:

```rust
pub fn update_bid_year_metadata(
    conn: &mut _,
    bid_year_id: i64,
    label: Option<&str>,
    notes: Option<&str>,
) -> Result<(), PersistenceError>
```

Update `list_bid_years` query to include metadata fields.

#### Server Layer

**`crates/server/src/main.rs`**

Add route:

```rust
.route("/api/bid-years/update-metadata", post(handle_update_bid_year_metadata))
```

Handler:

```rust
async fn handle_update_bid_year_metadata(
    State(app_state): State<AppState>,
    session: AuthSession,
    Json(req): Json<UpdateBidYearMetadataRequest>,
) -> Result<Json<UpdateBidYearMetadataResponse>, ServerError>
```

---

## Frontend Changes Details

### Components Affected

#### Modified Components

**`ui/src/components/BootstrapCompleteness.tsx`**

Enhance `BidYearItem` component:

- Display label (if set) prominently
- Display notes (if set) in collapsible section
- Add inline edit for label and notes
- Show "Last Updated" timestamp (if trackable)
- Lifecycle state badge (already exists)

**`ui/src/components/BootstrapOverview.tsx`**

Similar enhancements to bid year display.

#### New Components (Optional)

**`ui/src/components/BidYearMetadataEdit.tsx`**

Inline edit form for bid year metadata:

- Label input (short text, max ~100 chars)
- Notes textarea (longer text, max ~1000 chars)
- Save / Cancel buttons
- Loading state
- Error display

Can be embedded in existing bid year components.

#### API Integration

**`ui/src/api.ts`**

Add function:

```typescript
export async function updateBidYearMetadata(
  bidYearId: number,
  label: string | null,
  notes: string | null,
): Promise<UpdateBidYearMetadataResponse>;
```

#### Type Definitions

**`ui/src/types.ts`**

Update `BidYearInfo`:

```typescript
export interface BidYearInfo {
  // ... existing fields ...
  lifecycle_state: string;
  label?: string | null;
  notes?: string | null;
}
```

Add response type:

```typescript
export interface UpdateBidYearMetadataResponse {
  bid_year_id: number;
  bid_year: number;
  label: string | null;
  notes: string | null;
  message: string;
}
```

---

## Domain & UX Invariants

### Rules That Must Not Be Violated

1. **Metadata is informational only**
   - Does not affect domain logic
   - Does not trigger lifecycle transitions
   - Does not validate against bootstrap rules

2. **Structural fields remain immutable**
   - Year, start_date, num_pay_periods cannot be edited
   - Only metadata fields (label, notes) are editable
   - Structural integrity preserved

3. **Lifecycle independence**
   - Metadata edits allowed in all lifecycle states
   - Draft, BootstrapComplete, Canonicalized, BiddingActive, BiddingClosed
   - No gating required

4. **Audit trail required**
   - All metadata changes must create audit events
   - Changes are attributable to an actor
   - Reason is implicit (operational documentation)

5. **Optional fields**
   - Label and notes are optional (can be null)
   - Empty values are valid
   - No minimum length requirements

### UX Patterns

#### Bid Year with Metadata

```text
+---------------------------------------+
| Bid Year 2026 — Draft                 |
| Label: FY2026 Primary Bidding         |
| [Edit Metadata]                       |
+---------------------------------------+
| Year: 2026 (Jan 4, 2026 – Jan 2, 2027)        |
| Pay Periods: 26                       |
| Lifecycle: Draft                      |
| Areas: 5, Users: 120                  |
+---------------------------------------+
| Notes:                                |
| First year implementing new leave     |
| accrual rules from Phase 9.           |
| [Edit]                                |
+---------------------------------------+
```

#### Inline Metadata Edit

```text
+---------------------------------------+
| Bid Year 2026 — Draft                 |
+---------------------------------------+
| Label:                                |
| [__________________________]          |
|                                       |
| Notes:                                |
| [_________________________________]   |
| [_________________________________]   |
| [_________________________________]   |
|                                       |
| [Save] [Cancel]                       |
+---------------------------------------+
```

#### Lifecycle History (Read-Only)

```text
+---------------------------------------+
| Lifecycle History                     |
+---------------------------------------+
| Current: Canonicalized 🔒             |
| Last Changed: 2026-01-15 14:32 UTC    |
|                                       |
| History:                              |
| • Draft → BootstrapComplete           |
|   2026-01-10 09:15 (by admin)         |
| • BootstrapComplete → Canonicalized   |
|   2026-01-15 14:32 (by admin)         |
+---------------------------------------+
```

**Note**: Lifecycle history requires audit log querying, which may be out of scope. If not available, show only current state.

---

## Risks & Ambiguities

### 1. Migration for Existing Bid Years

**Risk**: Adding metadata columns requires database migration.

**Impact**: Existing bid years will have `NULL` for label and notes.

**Mitigation**: Migration is backward-compatible. NULL is valid.

**Decision**: Standard migration. No data backfill needed.

---

### 2. Lifecycle History Tracking

**Ambiguity**: How to display lifecycle transition history?

**Options**:

1. Query audit log for lifecycle transitions (complex)
2. Add `lifecycle_changed_at` timestamp to canonical table (simpler)
3. Read-only display of current state only (simplest)

**Recommendation**: Option 3 for Phase 26E. Full history is audit log browsing (future phase).

---

### 3. Label Length Limits

**Question**: Should label have a maximum length?

**Recommendation**: Yes. Enforce reasonable limit:

- Label: max 100 characters (short descriptor)
- Notes: max 2000 characters (longer operational notes)

**Validation**: Backend validates length, frontend shows character count.

---

### 4. Rich Text for Notes

**Question**: Should notes support rich text (Markdown, HTML)?

**Decision**: Not in Phase 26E. Plain text only. Rich text is future enhancement.

---

### 5. Metadata in API Responses

**Question**: Should all bid year API responses include metadata?

**Recommendation**: Yes. Add to `BidYearInfo`. Low cost, high value.

**Impact**: `list_bid_years`, `get_bootstrap_completeness`, etc. include metadata.

---

### 6. Audit Event Payload

**Question**: What should audit event include for metadata changes?

**Recommendation**: Include before/after values:

- `before_label`, `after_label`
- `before_notes`, `after_notes`

**Reason**: Metadata changes should be auditable like other state changes.

---

## Exit Criteria

Phase 26E is complete when:

1. ✅ Database migration adds `label` and `notes` columns
2. ✅ `update_bid_year_metadata` endpoint implemented
3. ✅ Metadata editable in all lifecycle states (no gating)
4. ✅ `BidYearInfo` includes label and notes in API responses
5. ✅ UI displays metadata prominently
6. ✅ Inline edit for metadata functional
7. ✅ Audit events created for metadata changes
8. ✅ Lifecycle state displayed for context (read-only)
9. ✅ Mobile-friendly metadata editing
10. ✅ Length validation enforced (label <= 100, notes <= 2000)
11. ✅ `cargo xtask ci` passes
12. ✅ `cargo xtask verify-migrations` passes
13. ✅ `pre-commit run --all-files` passes
14. ✅ Manual testing confirms metadata edits work

---

## Implementation Notes

### Suggested Implementation Order

1. **Database migration**
   - Create SQLite migration
   - Create MySQL migration
   - Run `cargo xtask verify-migrations`

2. **Persistence layer**
   - Add `update_bid_year_metadata` mutation
   - Update `list_bid_years` query to include metadata

3. **API layer**
   - Add DTOs (request/response)
   - Implement handler
   - Add length validation

4. **Server layer**
   - Add route
   - Add handler

5. **Frontend API wrapper**
   - Add `updateBidYearMetadata` function

6. **Frontend UI**
   - Update `BidYearInfo` type
   - Add inline edit component
   - Integrate with existing bid year displays

7. **Testing**
   - Backend: metadata update tests
   - Frontend: inline edit workflow
   - Mobile: touch interaction

---

## UI Design Patterns

### Bid Year Card with Metadata

**Desktop:**

```text
+-----------------------------------------------+
| Bid Year 2026 — Canonicalized 🔒              |
| "FY2026 Primary Bidding"                      |
+-----------------------------------------------+
| Year: 2026 (Jan 4, 2026 – Jan 2, 2027)        |
| Pay Periods: 26                               |
| Areas: 5, Users: 120                          |
| Expected Areas: 5 ✓, All areas complete ✓    |
+-----------------------------------------------+
| Notes:                                        |
| First year implementing new leave accrual    |
| rules from Phase 9. Watch for operator       |
| questions during bidding.                     |
|                                               |
| [Edit Metadata]                               |
+-----------------------------------------------+
```

**Editing:**

```text
+-----------------------------------------------+
| Bid Year 2026 — Canonicalized 🔒              |
+-----------------------------------------------+
| Label (optional, max 100 chars):              |
| [FY2026 Primary Bidding_________________]     |
| 23 / 100 characters                           |
|                                               |
| Notes (optional, max 2000 chars):             |
| [_________________________________________]   |
| [_________________________________________]   |
| [_________________________________________]   |
| 156 / 2000 characters                         |
|                                               |
| [Save Changes] [Cancel]                       |
+-----------------------------------------------+
```

---

## Testing Strategy

### Backend Tests

**Unit Tests** (`crates/api/src/handlers.rs`):

- `test_update_bid_year_metadata_allowed_in_all_states`
- `test_update_bid_year_metadata_requires_admin`
- `test_update_bid_year_metadata_validates_length`
- `test_update_bid_year_metadata_creates_audit_event`
- `test_update_bid_year_metadata_handles_null_values`

**Migration Tests**:

- `cargo xtask verify-migrations` (both backends)

### Frontend Tests

**Manual Testing Checklist:**

**Draft State:**

- [ ] Can edit label and notes
- [ ] Changes persist after save
- [ ] Cancel restores original values

**Canonicalized State:**

- [ ] Can still edit metadata (no lifecycle gating)
- [ ] Structural fields remain read-only

**Validation:**

- [ ] Label > 100 chars shows error
- [ ] Notes > 2000 chars shows error
- [ ] Character count updates as user types

**Mobile:**

- [ ] Metadata edit works on 375px width
- [ ] Textarea scrollable
- [ ] Save/Cancel buttons tappable

---

## Dependencies

### Required Existing Code

- Canonical bid years table (exists, Phase 25C)
- Lifecycle state infrastructure (exists, Phase 25A)
- Audit event system (exists)
- `BidYearInfo` type (exists)

### New Database Schema

- `label` column (TEXT, nullable)
- `notes` column (TEXT, nullable)

### No New External Dependencies

This phase uses only existing infrastructure.

---

## Rollout Considerations

### Backward Compatibility

**Schema Change**: New columns added to canonical bid years table.

**Impact**: Existing bid years will have NULL metadata. Valid and expected.

**API Change**: `BidYearInfo` includes new fields (additive, not breaking).

**Frontend**: Components must handle null metadata gracefully.

### Rollback Plan

If Phase 26E needs to be reverted:

1. Remove metadata editing UI
2. Revert migration (if necessary)
3. Remove `update_bid_year_metadata` endpoint

**Note**: If migration has run in production, metadata columns will remain but be unused.

---

## Non-Goals

- Structural field editing (year, dates, pay periods)
- Rich text formatting (Markdown, HTML)
- Metadata versioning (history of changes)
- Metadata templates
- Bulk metadata operations
- Metadata export/import
- Custom metadata fields (label and notes only)

---

## Mobile-First Compliance

All UI must follow mobile-first guidelines from `AGENTS.md`:

### Required Patterns

- **Inline editing**: Forms stack vertically on mobile
- **Touch targets**: Edit buttons >= 44px height
- **Textareas**: Scrollable, resize with content
- **Character counters**: Visible but not intrusive
- **Buttons**: Full-width or stacked on mobile

### Component-Specific

**Metadata Edit Form:**

- Label input: full width on mobile
- Notes textarea: full width, auto-height or scrollable
- Save/Cancel buttons: stacked vertically on mobile
- Character count: below input, small font

---

## Validation Checklist

Before marking Phase 26E complete, verify:

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (all backend tests)
- [ ] `npm run build` succeeds
- [ ] `cargo xtask ci` passes
- [ ] `cargo xtask verify-migrations` passes
- [ ] `pre-commit run --all-files` passes
- [ ] Manual test: Edit metadata in Draft state (works)
- [ ] Manual test: Edit metadata in Canonicalized state (works)
- [ ] Manual test: Length validation enforced
- [ ] Manual test: Null values handled correctly
- [ ] Mobile test: Metadata edit usable on 375px width
- [ ] Code review confirms no lifecycle gating for metadata
- [ ] Audit events logged for metadata changes

---

## Next Phase

**Phase 26F** will implement override visibility and polish, making overridden canonical fields transparent and auditable in the UI.
