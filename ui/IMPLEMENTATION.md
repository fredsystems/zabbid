# Phase 12 Implementation Notes

## Overview

This document records the implementation decisions for Phase 12: Operator-Focused UI & API Co-Design.

## Objectives Achieved

1. ✅ Built a real, durable operator UI (not throwaway)
2. ✅ Validated existing API ergonomics
3. ✅ Maintained strict backend authority
4. ✅ No domain logic duplicated in frontend
5. ✅ No weakening of audit or validation semantics
6. ✅ Read APIs remain ergonomic and aggregated

## UI Technology Choices

### Framework: React with TypeScript

**Rationale:**

- **Explicit data fetching**: React's `useEffect` provides clear, predictable data loading
- **Predictable state handling**: `useState` is sufficient; no complex state management needed
- **Clear error surfaces**: React error boundaries and component-level error states
- **Strong typing**: TypeScript enforces type safety and prevents runtime errors

### Build Tool: Vite

**Rationale:**

- Fast development server
- Built-in proxy support for backend API
- Modern ESM-based bundling
- Minimal configuration required

### Routing: React Router

**Rationale:**

- Standard, well-understood routing solution
- Type-safe with TypeScript
- Supports nested routes and URL parameters

## UI Components Implemented

### 1. Bootstrap Overview (`/`)

**Purpose:** System-wide view of all bid years

**Features:**

- Lists all bid years with canonical metadata
- Displays area count and user count per bid year
- Identifies the "active" bid year (see Active Bid Year Logic below)
- Provides navigation to area views

**API Dependencies:**

- `GET /bid_years` - single call provides all necessary data

### 2. Area View (`/bid-year/:year/areas`)

**Purpose:** View all areas within a bid year

**Features:**

- Lists all areas for selected bid year
- Shows area_id and user count
- Provides navigation to user list

**API Dependencies:**

- `GET /areas?bid_year={year}` - single call, aggregated data

### 3. User List View (`/bid-year/:year/area/:areaId/users`)

**Purpose:** View all users in an area with leave availability

**Features:**

- Displays user metadata (initials, name, type, crew)
- Shows earned leave (days + hours)
- Shows remaining leave (days + hours)
- Highlights exhaustion and overdraw status
- Visual indicators for leave status

**API Dependencies:**

- `GET /users?bid_year={year}&area={area}` - **single call** returns all users with leave data
- **No N+1 queries**: leave availability is computed server-side

**API Ergonomics Validation:**

This endpoint demonstrates excellent API ergonomics:

- Aggregates user data with leave calculations
- Eliminates need for client-side joins or computation
- Returns rich, operator-focused data in one call

### 4. User Detail View (`/bid-year/:year/area/:areaId/user/:initials`)

**Purpose:** Detailed view of a single user's leave data

**Features:**

- Full user metadata display
- Leave accrual breakdown (from Phase 9 rich model)
- Derived totals (earned, used, remaining)
- Human-readable calculation explanation
- Status indicators (available, exhausted, overdrawn)

**API Dependencies:**

- `GET /leave/availability?bid_year={year}&area={area}&initials={initials}`
- Returns rich calculation details including explanation text

## Active Bid Year Logic

### Problem

Phase 11 and 12 requirements state:

> "The system operates on exactly one active bid year at a time"
> "Identify the single active bid year"

However, no backend implementation of "active bid year" exists in:

- Domain layer
- Core layer
- Persistence layer

### Solution for Phase 12

**UI-level convention** (temporary, pending backend implementation):

```typescript
const activeBidYear = bidYears.length === 1 ? bidYears[0] : null;
```

**Rules:**

- If exactly one bid year exists → it is "active"
- If zero or multiple bid years exist → operator must manually select

**Rationale:**

- Phase 12 non-goals: "You must NOT: Add new domain rules"
- This is a read-only UI convenience, not domain logic
- Explicitly documented as temporary
- Does not weaken backend authority
- Does not introduce persistence or state

**Future Work:**

A proper "active bid year" concept should be added to the domain/core layers in a future phase, with:

- Enforcement at domain level
- Persistence of active status
- Validation that only one bid year is active
- API endpoints to get/set active bid year

## API Ergonomics Assessment

### Existing APIs Are Sufficient

No new APIs were required. The existing endpoints provide:

1. **Aggregated data**: User lists include leave availability
2. **Rich models**: Leave availability returns detailed breakdown
3. **Single-call efficiency**: No N+1 query patterns
4. **Clear error responses**: Backend errors surface cleanly

### No API Changes Required

The Phase 11 APIs already support all Phase 12 UI requirements:

- ✅ Bootstrap overview data
- ✅ Area listing with counts
- ✅ User listing with leave data
- ✅ Detailed leave availability

This validates that Phase 11 API ergonomics work was successful.

## Frontend Validation Strategy

### No Authoritative Validation

The UI performs **zero** authoritative validation.

### Early Feedback Only

The UI may provide early feedback for user experience:

- Required field indicators
- Basic format checks (e.g., numeric inputs)

But backend validation is always final.

### Error Surfacing

Backend validation errors are surfaced clearly:

- HTTP error responses are caught and displayed
- Error messages from backend are shown verbatim
- No masking or recovery attempts

## Type Safety

### TypeScript Types Match Backend DTOs

All types in `src/types.ts` are derived from backend API DTOs:

- `BidYearInfo` ← `api::request_response::BidYearInfo`
- `AreaInfo` ← `api::request_response::AreaInfo`
- `UserInfo` ← `api::request_response::UserInfo`
- `LeaveAvailabilityResponse` ← `api::request_response::GetLeaveAvailabilityResponse`

### Type Sync Responsibility

The frontend types must remain manually synchronized with backend DTOs.

**Process:**

1. Backend API DTO changes
2. Update frontend `src/types.ts`
3. TypeScript compiler catches usage issues
4. Update components as needed

**Future Improvement:**

Consider generating TypeScript types from Rust DTOs using a tool like `typeshare` or `ts-rs`.

## No Domain Logic in Frontend

### What's NOT in the Frontend

- ✅ No leave accrual calculations
- ✅ No validation rules
- ✅ No business logic
- ✅ No domain concepts (crews, seniority, etc.)
- ✅ No state derivation
- ✅ No audit logic

### What IS in the Frontend

- ✅ UI state (loading, error states)
- ✅ Navigation and routing
- ✅ Data display and formatting
- ✅ Visual indicators derived from API data
- ✅ User interaction handling

## Testing Strategy

### Phase 12 Testing Focus

Testing demonstrates:

1. UI can drive bootstrap inspection end-to-end
2. No reliance on undocumented API behavior
3. API ergonomics eliminate request chaining
4. No domain logic duplication

### Current Testing

- Manual testing via development server
- Type checking via `tsc --noEmit`

### Future Testing

Future phases may add:

- Unit tests for components
- Integration tests for API client
- End-to-end tests for workflows
- Visual regression tests

## Development Workflow

### Running Locally

1. Start backend server on port 8080:

   ```bash
   cd crates/server
   cargo run -- --database ../../test.db
   ```

2. Start UI development server:

   ```bash
   cd ui
   npm install
   npm run dev
   ```

3. Navigate to `http://localhost:3000`

### API Proxy

Vite proxies `/api/*` to `http://127.0.0.1:8080`, eliminating CORS issues during development.

## Constraints Honored

### From AGENTS.md

- ✅ No domain logic in frontend
- ✅ Backend remains sole arbiter of correctness
- ✅ No weakening of audit/validation semantics
- ✅ Frontend validation is non-authoritative

### From PLAN.md Phase 12

- ✅ No new domain rules
- ✅ Read APIs remain ergonomic
- ✅ Exactly one active bid year concept (UI-level for now)
- ✅ No CRUD drift
- ✅ Backend authority maintained

### Phase 12 Non-Goals Respected

- ✅ No write capabilities added
- ✅ No performance optimization
- ✅ No aesthetic polish
- ✅ No public API design
- ✅ No CRUD semantics
- ✅ No multi-bid-year workflows

## File Structure

```text
ui/
├── src/
│   ├── components/
│   │   ├── BootstrapOverview.tsx    # Bootstrap overview view
│   │   ├── AreaView.tsx              # Area listing view
│   │   ├── UserListView.tsx          # User list with leave data
│   │   └── UserDetailView.tsx        # User detail with full breakdown
│   ├── api.ts                        # API client functions
│   ├── types.ts                      # TypeScript types from backend DTOs
│   ├── App.tsx                       # Main app with routing
│   ├── App.css                       # Functional styling
│   └── main.tsx                      # Entry point
├── index.html                        # HTML template
├── package.json                      # Dependencies
├── tsconfig.json                     # TypeScript config (strict)
├── vite.config.ts                    # Vite config with proxy
└── README.md                         # UI documentation
```

## Future Work

### Potential Phase 13+ Enhancements

1. **Write Operations**
   - User registration form
   - Bid entry interface
   - Administrative actions

2. **Authentication/Authorization**
   - Login flow
   - Role-based UI elements
   - Session management

3. **Real-Time Updates**
   - WebSocket connection
   - Live state updates
   - Optimistic UI updates

4. **Enhanced Operator Workflows**
   - Bulk operations
   - Search and filtering
   - Data export

5. **Accessibility**
   - ARIA labels
   - Keyboard navigation
   - Screen reader support

6. **Type Generation**
   - Automatic TS type generation from Rust
   - Eliminate manual type sync

## Exit Criteria Met

- ✅ UI can drive bootstrap inspection end-to-end
- ✅ UI does not rely on undocumented API behavior
- ✅ API ergonomics eliminate unnecessary request chaining
- ✅ No domain logic was duplicated in the UI
- ✅ No audit or persistence semantics were changed
- ✅ Real data rendering works
- ✅ Visible error handling paths exist
- ✅ Active bid year context is handled (UI-level convention)
- ✅ API and CLI consistency maintained (no API changes)

## Conclusion

Phase 12 successfully delivers a minimal, durable operator UI that:

- Validates excellent API ergonomics from Phase 11
- Maintains strict backend authority
- Introduces zero domain logic in the frontend
- Provides clear, functional operator workflows
- Requires no API changes

The UI is production-ready for read-only operator workflows and serves as a foundation for future write capabilities.
