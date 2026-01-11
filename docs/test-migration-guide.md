# Test Migration Guide: Session-Based Authentication

## Overview

Phase 14b introduced session-based authentication for all state-changing operations. Tests must be updated to:

1. Create operators before performing actions
2. Login to obtain session tokens
3. Include session tokens in Authorization headers
4. Remove `actor_id` and `actor_role` from request payloads

## Quick Reference

### Before (Phase 14a and earlier)

```rust
#[tokio::test]
async fn test_register_user_as_admin() {
    let app_state = create_test_app_state();
    let app = build_router(app_state);

    let req = RegisterUserApiRequest {
        actor_id: String::from("admin1"),
        actor_role: String::from("admin"),
        cause_id: String::from("test"),
        cause_description: String::from("Test"),
        bid_year: 2026,
        initials: String::from("AB"),
        name: String::from("Test User"),
        area: String::from("TestArea"),
        user_type: String::from("CPC"),
        crew: Some(1),
        cumulative_natca_bu_date: String::from("2020-01-01"),
        natca_bu_date: String::from("2020-01-01"),
        eod_faa_date: String::from("2020-01-01"),
        service_computation_date: String::from("2020-01-01"),
        lottery_value: None,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/users")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), HttpStatusCode::OK);
}
```

### After (Phase 14b)

```rust
#[tokio::test]
async fn test_register_user_as_admin() {
    let app_state = create_test_app_state();
    let app = build_router(app_state.clone());

    // 1. Create operator and login
    let admin_token = create_operator_and_login(
        &app_state,
        "admin1",
        "Admin User",
        "Admin"
    ).await;

    // 2. Request payload no longer contains actor_id/actor_role
    let req = RegisterUserApiRequest {
        cause_id: String::from("test"),
        cause_description: String::from("Test"),
        bid_year: 2026,
        initials: String::from("AB"),
        name: String::from("Test User"),
        area: String::from("TestArea"),
        user_type: String::from("CPC"),
        crew: Some(1),
        cumulative_natca_bu_date: String::from("2020-01-01"),
        natca_bu_date: String::from("2020-01-01"),
        eod_faa_date: String::from("2020-01-01"),
        service_computation_date: String::from("2020-01-01"),
        lottery_value: None,
    };

    // 3. Include Authorization header with session token
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/users")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", admin_token))
                .body(Body::from(serde_json::to_string(&req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), HttpStatusCode::OK);
}
```

## Helper Functions

Add these helper functions to your test module:

```rust
/// Helper to create an operator and get a session token.
async fn create_operator_and_login(
    app_state: &AppState,
    login_name: &str,
    display_name: &str,
    role: &str,
) -> String {
    // Create operator
    let mut persistence = app_state.persistence.lock().await;
    persistence
        .create_operator(login_name, display_name, role)
        .expect("Failed to create operator");
    drop(persistence);

    // Login to get session token
    let mut persistence = app_state.persistence.lock().await;
    let login_req = zab_bid_api::LoginRequest {
        login_name: login_name.to_string(),
    };
    let response = zab_bid_api::login(&mut persistence, login_req)
        .expect("Failed to login");
    response.session_token
}
```

## Request Structure Changes

All request structures no longer include `actor_id` and `actor_role`:

### RegisterUserApiRequest

**Removed fields:**

- `actor_id: String`
- `actor_role: String`

### AdminActionRequest

**Removed fields:**

- `actor_id: String`
- `actor_role: String`

### CreateBidYearApiRequest

**Removed fields:**

- `actor_id: String`
- `actor_role: String`

### CreateAreaApiRequest

**Removed fields:**

- `actor_id: String`
- `actor_role: String`

## Testing Authorization

### Test Admin-only actions reject Bidders

```rust
#[tokio::test]
async fn test_create_bid_year_as_bidder_fails() {
    let app_state = create_test_app_state();
    let app = build_router(app_state.clone());

    // Create bidder and login
    let bidder_token = create_operator_and_login(
        &app_state,
        "bidder1",
        "Bidder User",
        "Bidder"
    ).await;

    let req = CreateBidYearApiRequest {
        cause_id: String::from("test"),
        cause_description: String::from("Test"),
        year: 2026,
        start_date: String::from("2026-01-04"),
        num_pay_periods: 26,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/bid_years")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", bidder_token))
                .body(Body::from(serde_json::to_string(&req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Bidders cannot create bid years
    assert_eq!(response.status(), HttpStatusCode::FORBIDDEN);
}
```

### Test unauthenticated requests are rejected

```rust
#[tokio::test]
async fn test_unauthenticated_request_fails() {
    let app_state = create_test_app_state();
    let app = build_router(app_state);

    let req = CreateBidYearApiRequest {
        cause_id: String::from("test"),
        cause_description: String::from("Test"),
        year: 2026,
        start_date: String::from("2026-01-04"),
        num_pay_periods: 26,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/bid_years")
                .header("content-type", "application/json")
                // No Authorization header!
                .body(Body::from(serde_json::to_string(&req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), HttpStatusCode::UNAUTHORIZED);
}
```

### Test expired sessions are rejected

```rust
#[tokio::test]
async fn test_expired_session_rejected() {
    let app_state = create_test_app_state();
    let app = build_router(app_state);

    // Use an invalid/expired token
    let fake_token = "expired_or_invalid_token";

    let req = CreateBidYearApiRequest {
        cause_id: String::from("test"),
        cause_description: String::from("Test"),
        year: 2026,
        start_date: String::from("2026-01-04"),
        num_pay_periods: 26,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/bid_years")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", fake_token))
                .body(Body::from(serde_json::to_string(&req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), HttpStatusCode::UNAUTHORIZED);
}
```

### Test disabled operators cannot authenticate

```rust
#[tokio::test]
async fn test_disabled_operator_cannot_act() {
    let app_state = create_test_app_state();
    let app = build_router(app_state.clone());

    // Create operator
    let mut persistence = app_state.persistence.lock().await;
    let operator_id = persistence
        .create_operator("admin1", "Admin User", "Admin")
        .expect("Failed to create operator");

    // Disable the operator
    persistence
        .disable_operator(operator_id)
        .expect("Failed to disable operator");
    drop(persistence);

    // Try to login
    let mut persistence = app_state.persistence.lock().await;
    let login_req = zab_bid_api::LoginRequest {
        login_name: String::from("admin1"),
    };
    let result = zab_bid_api::login(&mut persistence, login_req);

    // Login should fail for disabled operator
    assert!(result.is_err());
}
```

## Migration Checklist

For each test:

- [ ] Add `app_state.clone()` when creating the app if you need to access app_state later
- [ ] Call `create_operator_and_login()` before making authenticated requests
- [ ] Remove `actor_id` and `actor_role` from all request structures
- [ ] Add `.header("authorization", format!("Bearer {}", token))` to requests
- [ ] Verify expected HTTP status codes (401 vs 403 vs 200/201)
- [ ] Ensure no audit events are created on authentication/authorization failures

## Common Pitfalls

1. **Forgetting to clone app_state**: If you need app_state after building the router, clone it first
2. **Wrong status codes**: Authentication failures return 401, authorization failures return 403
3. **Missing Authorization header**: All state-changing endpoints now require authentication
4. **Reusing tokens across tests**: Each test should create its own operator and session
5. **Testing read endpoints**: Most read endpoints don't require authentication yet

## Authorization Matrix

| Action          | Admin | Bidder   | Unauthenticated |
| --------------- | ----- | -------- | --------------- |
| Create Bid Year | ✅    | ❌ (403) | ❌ (401)        |
| Create Area     | ✅    | ❌ (403) | ❌ (401)        |
| Register User   | ✅    | ❌ (403) | ❌ (401)        |
| Checkpoint      | ✅    | ❌ (403) | ❌ (401)        |
| Finalize        | ✅    | ❌ (403) | ❌ (401)        |
| Rollback        | ✅    | ❌ (403) | ❌ (401)        |
| List Bid Years  | ✅    | ✅       | ✅              |
| List Areas      | ✅    | ✅       | ✅              |
| List Users      | ✅    | ✅       | ✅              |

## Session Management

### Login Endpoint

```http
POST /api/auth/login
Content-Type: application/json

{
  "login_name": "admin1"
}
```

Response:

```json
{
  "session_token": "session_1234567890_9876543210",
  "login_name": "admin1",
  "display_name": "Admin User",
  "role": "Admin",
  "expires_at": "2026-02-03T12:34:56Z"
}
```

### Logout Endpoint

```http
POST /api/auth/logout
Authorization: Bearer session_1234567890_9876543210
Content-Type: application/json

{
  "session_token": "session_1234567890_9876543210"
}
```

### Who Am I Endpoint

```http
GET /api/auth/me
Authorization: Bearer session_1234567890_9876543210
```

Response:

```json
{
  "login_name": "admin1",
  "display_name": "Admin User",
  "role": "Admin",
  "is_disabled": false
}
```

## Next Steps

1. Update all existing tests following this guide
2. Add new tests for authentication edge cases
3. Ensure `cargo xtask ci` passes
4. Ensure `pre-commit run --all-files` passes
