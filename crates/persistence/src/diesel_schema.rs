// @generated automatically by Diesel CLI.
// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

// NOTE: The diesel.toml setting `sqlite_integer_primary_key_is_bigint = true`
// causes Diesel to map SQLite INTEGER PRIMARY KEY columns to BigInt (i64).
// This matches SQLite's actual 64-bit storage for these columns.

diesel::table! {
    areas (area_id) {
        area_id -> BigInt,
        bid_year_id -> BigInt,
        area_code -> Text,
        area_name -> Nullable<Text>,
        expected_user_count -> Nullable<Integer>,
        is_system_area -> Integer,
    }
}

diesel::table! {
    audit_events (event_id) {
        event_id -> BigInt,
        bid_year_id -> Nullable<BigInt>,
        area_id -> Nullable<BigInt>,
        year -> Integer,
        area_code -> Text,
        actor_operator_id -> BigInt,
        actor_login_name -> Text,
        actor_display_name -> Text,
        actor_json -> Text,
        cause_json -> Text,
        action_json -> Text,
        before_snapshot_json -> Text,
        after_snapshot_json -> Text,
        created_at -> Nullable<Text>,
    }
}

diesel::table! {
    bid_years (bid_year_id) {
        bid_year_id -> BigInt,
        year -> Integer,
        start_date -> Text,
        num_pay_periods -> Integer,
        is_active -> Integer,
        expected_area_count -> Nullable<Integer>,
        lifecycle_state -> Text,
    }
}

diesel::table! {
    operators (operator_id) {
        operator_id -> BigInt,
        login_name -> Text,
        display_name -> Text,
        password_hash -> Text,
        role -> Text,
        is_disabled -> Integer,
        created_at -> Text,
        disabled_at -> Nullable<Text>,
        last_login_at -> Nullable<Text>,
    }
}

diesel::table! {
    sessions (session_id) {
        session_id -> BigInt,
        session_token -> Text,
        operator_id -> BigInt,
        created_at -> Text,
        last_activity_at -> Text,
        expires_at -> Text,
    }
}

diesel::table! {
    state_snapshots (snapshot_id) {
        snapshot_id -> BigInt,
        bid_year_id -> BigInt,
        area_id -> BigInt,
        event_id -> BigInt,
        state_json -> Text,
        created_at -> Nullable<Text>,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> BigInt,
        bid_year_id -> BigInt,
        area_id -> BigInt,
        initials -> Text,
        name -> Text,
        user_type -> Text,
        crew -> Nullable<Integer>,
        cumulative_natca_bu_date -> Text,
        natca_bu_date -> Text,
        eod_faa_date -> Text,
        service_computation_date -> Text,
        lottery_value -> Nullable<Integer>,
    }
}

diesel::joinable!(areas -> bid_years (bid_year_id));
diesel::joinable!(audit_events -> areas (area_id));
diesel::joinable!(audit_events -> bid_years (bid_year_id));
diesel::joinable!(audit_events -> operators (actor_operator_id));
diesel::joinable!(sessions -> operators (operator_id));
diesel::joinable!(state_snapshots -> areas (area_id));
diesel::joinable!(state_snapshots -> audit_events (event_id));
diesel::joinable!(state_snapshots -> bid_years (bid_year_id));
diesel::joinable!(users -> areas (area_id));
diesel::joinable!(users -> bid_years (bid_year_id));

diesel::allow_tables_to_appear_in_same_query!(
    areas,
    audit_events,
    bid_years,
    operators,
    sessions,
    state_snapshots,
    users,
);

// Allow GROUP BY queries with columns from joined tables
diesel::allow_columns_to_appear_in_same_group_by_clause!(bid_years::year, areas::area_code,);
