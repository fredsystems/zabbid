# Phase 30I — API Surface Audit & Documentation

## Purpose

Perform a comprehensive audit of all active API endpoints, remove dead or
unreachable endpoints and associated code, and produce authoritative
documentation of the API surface.

This sub-phase delivers the **API cleanup and documentation** required to
complete Phase 30 and establish a baseline for future API governance.

---

## Scope

### A. API Endpoint Inventory

Enumerate all API endpoints by examining:

1. **Routing definitions**
   - `crates/server/src/main.rs` or equivalent server entry point
   - Any routing module files
   - Extract all registered routes

2. **Handler implementations**
   - `crates/api/src/handlers.rs`
   - Any other handler modules
   - Map each route to its handler function

3. **HTTP methods and paths**
   - Document method (GET, POST, PUT, DELETE, PATCH)
   - Document full path with parameters
   - Document whether route is public or authenticated

### B. Endpoint Reachability Analysis

For each endpoint discovered:

1. **Determine reachability**
   - Is the endpoint registered in routing?
   - Is the handler called from anywhere?
   - Is there a corresponding frontend API binding?

2. **Categorize endpoints**
   - **Active**: Used by frontend or documented public API
   - **Internal**: Used by backend only (e.g., health checks)
   - **Future**: Implemented but not yet wired (if any)
   - **Dead**: Not reachable, not used, candidate for removal

3. **Flag inconsistencies**
   - Handler exists but no route registered
   - Route exists but handler missing or broken
   - Frontend binding exists but endpoint doesn't

### C. Dead Code Removal

For all endpoints categorized as **Dead**:

1. **Confirm unreachability**
   - No routes point to handler
   - No frontend bindings exist
   - No tests reference the endpoint
   - No documentation mentions it

2. **Remove endpoint code**
   - Delete handler function from `handlers.rs`
   - Delete associated request/response types (if unused elsewhere)
   - Delete route registration (if exists)
   - Delete frontend bindings (if exist)
   - Delete tests specific to dead endpoint

3. **Verify removal safety**
   - Run full test suite after each removal
   - Ensure no regressions
   - Commit after each removal (or group related removals)

### D. API Documentation Generation

Create: `zabbid/docs/api.md`

This document must contain:

#### 1. Overview Section

- Purpose of the API
- Authentication model (session tokens, bootstrap tokens)
- Base URL structure
- Response format conventions
- Error response structure

#### 2. Endpoint Catalog

For each **Active** and **Internal** endpoint:

**Format:**

```markdown
### `METHOD /path/to/endpoint`

**Purpose:** Brief description of what this endpoint does

**Authorization:** Admin | Bidder | Public | Bootstrap

**Request:**

- Path parameters: (if any)
- Query parameters: (if any)
- Request body: (if applicable)
  - Type reference or inline structure

**Response:**

- Success: HTTP 200
  - Type reference or inline structure
- Errors: HTTP 4xx/5xx
  - Common error cases

**Lifecycle Constraints:**

- (if applicable, e.g., "Only available in Bootstrap_Complete state")

**Notes:**

- (any important implementation details or usage notes)
```

**Example:**

````markdown
### `POST /bid_years`

**Purpose:** Create a new bid year

**Authorization:** Admin

**Request:**

- Body:

  ```json
  {
    "cause_id": "string",
    "cause_description": "string",
    "year": 2026,
    "start_date": "2026-01-05",
    "num_pay_periods": 26
  }
  ```

**Response:**

- Success: HTTP 200

  ```json
  {
    "bid_year_id": 123,
    "year": 2026,
    "message": "Bid year created successfully"
  }
  ```

- Errors:
  - HTTP 400: Invalid input
  - HTTP 409: Bid year already exists
  - HTTP 401: Unauthorized

**Lifecycle Constraints:**

- Can be called in any state

**Notes:**

- Year must be unique across all bid years
- Start date must be an ISO 8601 date string
````

#### 3. Type Definitions Section

List all request/response types referenced in endpoint documentation:

- Use existing Rust type definitions as source of truth
- Convert to human-readable format (JSON schema or similar)
- Include all required and optional fields
- Document field semantics

#### 4. Authentication Section

Document authentication mechanisms:

- Session token authentication (for operators)
- Bootstrap token authentication (for initial setup)
- Public endpoints (no authentication required)
- Token format and lifecycle
- How to obtain tokens (login endpoints)

#### 5. Lifecycle States Reference

Document how lifecycle states affect API availability:

- Bootstrap
- Bootstrap_Complete
- Canonicalized
- Bidding_Active
- Bidding_Closed

For each state, list constraints on API operations.

#### 6. Error Response Structure

Document standard error response format:

```json
{
  "error": "string",
  "details": "string (optional)",
  "error_type": "ValidationError | NotFound | Unauthorized | etc."
}
```

```text

### E. Frontend API Bindings Audit

Review `ui/src/api.ts`:

1. **Identify unused bindings**
   - Functions not called from any component
   - Dead imports
   - Candidate for removal

2. **Identify missing bindings**
   - Active backend endpoints with no frontend binding
   - Should frontend binding be added?
   - Document as gap if intentional

3. **Verify binding consistency**
   - Does binding match backend endpoint signature?
   - Are types aligned with backend?
   - Are error cases handled?

4. **Remove dead frontend bindings**
   - Delete unused API functions
   - Delete unused types
   - Verify no regressions

### F. Type Definitions Audit

Review `ui/src/types.ts`:

1. **Identify unused types**
   - Types not referenced in components or API bindings
   - Candidate for removal

2. **Verify type accuracy**
   - Do frontend types match backend response structures?
   - Are all required fields present?
   - Are optional fields marked correctly?

3. **Remove dead types**
   - Delete unused type definitions
   - Verify no regressions

---

## Deliverables

At the end of this sub-phase:

1. **`docs/api.md`** — Complete API documentation
2. **Dead code removed** — All unreachable endpoints and bindings deleted
3. **Clean test runs** — All tests pass after removals
4. **Audit report** (optional) — Summary of what was removed and why

---

## Validation

### A. Completeness Validation

1. **Cross-reference inventory**
   - Every route in routing → has handler
   - Every handler → has route (or marked internal/test)
   - Every frontend binding → has backend endpoint

2. **Documentation coverage**
   - All active endpoints documented in `docs/api.md`
   - All internal endpoints documented (separate section)
   - No active endpoint missing from docs

### B. Removal Safety Validation

1. **Test suite passes**
   - Run `cargo xtask ci` after all removals
   - No regressions

2. **Frontend builds**
   - Run `npm run build` in `ui/` directory
   - No TypeScript errors
   - No unused import warnings

3. **Manual smoke test**
   - Log in as admin
   - Perform one representative workflow
   - Verify no broken functionality

---

## Process Rules (Out of Scope)

**Important:** Phase 30 explicitly does **not** define ongoing process rules
for keeping `docs/api.md` up to date.

This sub-phase establishes the **baseline documentation** only.

Future governance of API documentation (e.g., requiring updates on API changes)
will be defined in a later phase or explicitly added to AGENTS.md.

For now:

- The document is accurate **as of Phase 30 completion**
- No automation or enforcement is required
- Manual updates are expected when APIs change

---

## Documentation Format Constraints

### Markdown Compliance

All documentation must comply with:

- Existing `.markdownlint-cli2.yaml` rules
- `pre-commit run --all-files` checks
- No inline HTML (use Markdown only)
- Proper heading hierarchy
- No trailing whitespace
- Consistent code block formatting

### Clarity Requirements

- Use plain language
- Avoid jargon where possible
- Provide examples for complex endpoints
- Link related endpoints
- Explain lifecycle constraints clearly

### Maintenance Considerations

- Keep structure simple and scannable
- Group related endpoints logically
- Use consistent formatting throughout
- Make it easy to find endpoints by path or purpose

---

## Explicit Non-Goals

- No API design changes
- No endpoint behavior modifications
- No new endpoints
- No refactoring for API consistency
- No automated documentation generation
- No OpenAPI/Swagger spec generation (may be future work)
- No process rules for keeping docs up to date

---

## Completion Conditions

This sub-phase is complete when:

- API endpoint inventory complete
- All dead endpoints removed
- All dead frontend bindings removed
- All dead types removed
- `docs/api.md` exists and is complete
- All active endpoints documented
- All internal endpoints documented (separate section if needed)
- Authentication and lifecycle sections written
- Type definitions documented
- Error response structure documented
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
- Frontend builds without errors
- Manual smoke test passes
- All removals committed
- Documentation committed
- User reviews and approves documentation

---

## Stop-and-Ask Conditions

Stop immediately if:

- Endpoint removal causes test failures that can't be easily resolved
- Unclear whether an endpoint is truly dead or just unused currently
- Frontend binding exists but corresponding backend endpoint missing
- Backend endpoint exists but purpose is unclear
- Lifecycle constraints are ambiguous or inconsistent
- Documentation structure becomes unwieldy (too many endpoints)

If unclear whether to remove an endpoint, **ask first** rather than guessing.

---

## Risk Notes

- Removing endpoints is irreversible (within this phase)
- Dead code detection may have false positives (endpoint used in unexpected way)
- Frontend bindings may be prepared for future use
- Documentation accuracy depends on thorough inventory
- Large API surface may make documentation maintenance difficult
- No automated enforcement of documentation accuracy after Phase 30
```
