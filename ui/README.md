# Zabbid Operator UI

A minimal, functional operator interface for the Zabbid leave bidding system.

## Purpose

This UI is intentionally minimal but structurally final. It exists to:

- Provide operators with a read-only view of system state
- Validate and improve API ergonomics
- Demonstrate that the backend is the sole source of truth
- Support operator workflows without duplicating domain logic

This is **not** a throwaway prototype. It is a durable operator interface.

## Architecture Constraints

- **No domain logic in the frontend** - All validation and business rules live in the backend
- **Backend authority** - The UI treats all API responses as authoritative
- **No persistence assumptions** - The UI does not cache or persist state
- **Read-only in Phase 12** - Write capabilities are deferred to later phases
- **Single active bid year** - The system operates on exactly one active bid year at a time

## Features

### Bootstrap Overview

- Displays all bid years in the system
- Identifies the active bid year (if exactly one exists)
- Shows bootstrap completeness: area count and total user count per bid year
- No mutations from this view

### Area View

- Lists all areas for a selected bid year
- Displays area_id and user count for each area
- Allows navigation into an area to view users

### User List View

- Lists users for a selected area
- Displays initials, name, user_type, crew
- Shows earned leave (days + hours)
- Shows remaining leave (days + hours)
- Displays exhaustion and overdraw indicators
- All data comes from one API call (no N+1 queries)

### User Detail View

- Shows full user metadata
- Shows leave accrual breakdown (rich model from Phase 9)
- Shows derived totals and availability
- Displays human-readable calculation explanation
- Read-only in Phase 12

## Technology Stack

- **TypeScript** - Strict type checking, explicit types
- **React** - Predictable state handling, clear error surfaces
- **React Router** - Client-side routing
- **Vite** - Fast development server with backend proxy
- **No state management library** - React state is sufficient for this phase

## Setup

### Prerequisites

- Node.js 18 or later
- npm or yarn
- The Zabbid backend server running on port 8080

### Installation

```bash
cd ui
npm install
```

### Development

Start the development server:

```bash
npm run dev
```

The UI will be available at `http://localhost:3000`.
API requests to `/api/*` are proxied to `http://127.0.0.1:8080`.

### Build

Build for production:

```bash
npm run build
```

The built files will be in the `dist` directory.

### Type Checking

Run TypeScript type checking:

```bash
npm run typecheck
```

## API Dependencies

The UI depends on the following backend endpoints:

- `GET /bid_years` - List all bid years with metadata
- `GET /areas?bid_year={year}` - List areas for a bid year
- `GET /users?bid_year={year}&area={area}` - List users with leave availability
- `GET /leave/availability?bid_year={year}&area={area}&initials={initials}` - Get detailed leave data
- `GET /bootstrap/status` - Get system-wide bootstrap status

All API types are defined in `src/types.ts` and must remain in sync with the backend DTOs.

## Design Decisions

### Active Bid Year Logic

The UI determines the "active" bid year using a simple rule:

- If exactly one bid year exists, it is considered active
- If zero or multiple bid years exist, the operator must manually select one

This is a temporary Phase 12 ergonomic helper. The backend does not yet enforce a single active bid year at the domain level, though AGENTS.md and PLAN.md indicate this should be added in a future phase.

### No Frontend Validation

The UI performs no authoritative validation. All validation is performed by the backend. The UI may perform early, non-authoritative checks for user experience (e.g., required fields), but backend validation is always final.

### Error Handling

API errors are surfaced clearly to the operator. The UI does not attempt to recover from or mask backend errors.

## Testing

UI tests are minimal in Phase 12 but demonstrate:

- Real data rendering from API calls
- Visible error handling paths
- No reliance on undocumented API behavior

## Non-Goals

Phase 12 explicitly does NOT include:

- Write capabilities (beyond existing commands)
- Performance optimization
- Aesthetic polish
- Public API design
- CRUD semantics
- Multi-bid-year workflows

## Future Work

Future phases may add:

- Write operations (bid entry, user management)
- Authentication and authorization UI
- Real-time updates
- Enhanced error recovery
- Accessibility improvements
- Additional operator workflows

## License

Copyright (C) 2026 Fred Clausen

Use of this source code is governed by an MIT-style license that can be found in the LICENSE file or at <https://opensource.org/licenses/MIT>.
