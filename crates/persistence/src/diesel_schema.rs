// @generated automatically by Diesel CLI.
// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

diesel::table! {
    areas (area_id) {
        area_id -> BigInt,
        bid_year_id -> BigInt,
        area_code -> Text,
        area_name -> Nullable<Text>,
        expected_user_count -> Nullable<Integer>,
        is_system_area -> Integer,
        round_group_id -> Nullable<BigInt>,
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
        label -> Nullable<Text>,
        notes -> Nullable<Text>,
        bid_timezone -> Nullable<Text>,
        bid_start_date -> Nullable<Text>,
        bid_window_start_time -> Nullable<Text>,
        bid_window_end_time -> Nullable<Text>,
        bidders_per_area_per_day -> Nullable<Integer>,
    }
}

diesel::table! {
    bid_status (bid_status_id) {
        bid_status_id -> BigInt,
        bid_year_id -> BigInt,
        area_id -> BigInt,
        user_id -> BigInt,
        round_id -> BigInt,
        status -> Text,
        updated_at -> Text,
        updated_by -> BigInt,
        notes -> Nullable<Text>,
    }
}

diesel::table! {
    bid_status_history (history_id) {
        history_id -> BigInt,
        bid_status_id -> BigInt,
        audit_event_id -> BigInt,
        previous_status -> Nullable<Text>,
        new_status -> Text,
        transitioned_at -> Text,
        transitioned_by -> BigInt,
        notes -> Nullable<Text>,
    }
}

diesel::table! {
    canonical_area_membership (id) {
        id -> BigInt,
        bid_year_id -> BigInt,
        audit_event_id -> BigInt,
        user_id -> BigInt,
        area_id -> BigInt,
        is_overridden -> Integer,
        override_reason -> Nullable<Text>,
    }
}

diesel::table! {
    canonical_bid_order (id) {
        id -> BigInt,
        bid_year_id -> BigInt,
        audit_event_id -> BigInt,
        user_id -> BigInt,
        bid_order -> Nullable<Integer>,
        is_overridden -> Integer,
        override_reason -> Nullable<Text>,
    }
}

diesel::table! {
    bid_windows (bid_window_id) {
        bid_window_id -> BigInt,
        bid_year_id -> BigInt,
        area_id -> BigInt,
        user_id -> BigInt,
        window_start_datetime -> Text,
        window_end_datetime -> Text,
    }
}

diesel::table! {
    canonical_bid_windows (id) {
        id -> BigInt,
        bid_year_id -> BigInt,
        audit_event_id -> BigInt,
        user_id -> BigInt,
        window_start_date -> Nullable<Text>,
        window_end_date -> Nullable<Text>,
        is_overridden -> Integer,
        override_reason -> Nullable<Text>,
    }
}

diesel::table! {
    canonical_eligibility (id) {
        id -> BigInt,
        bid_year_id -> BigInt,
        audit_event_id -> BigInt,
        user_id -> BigInt,
        can_bid -> Integer,
        is_overridden -> Integer,
        override_reason -> Nullable<Text>,
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
    round_groups (round_group_id) {
        round_group_id -> BigInt,
        bid_year_id -> BigInt,
        name -> Text,
        editing_enabled -> Integer,
    }
}

diesel::table! {
    rounds (round_id) {
        round_id -> BigInt,
        round_group_id -> BigInt,
        round_number -> Integer,
        name -> Text,
        slots_per_day -> Integer,
        max_groups -> Integer,
        max_total_hours -> Integer,
        include_holidays -> Integer,
        allow_overbid -> Integer,
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
        excluded_from_bidding -> Integer,
        excluded_from_leave_calculation -> Integer,
        no_bid_reviewed -> Integer,
    }
}

diesel::joinable!(areas -> bid_years (bid_year_id));
diesel::joinable!(areas -> round_groups (round_group_id));
diesel::joinable!(audit_events -> areas (area_id));
diesel::joinable!(audit_events -> bid_years (bid_year_id));
diesel::joinable!(audit_events -> operators (actor_operator_id));
diesel::joinable!(bid_status -> areas (area_id));
diesel::joinable!(bid_status -> bid_years (bid_year_id));
diesel::joinable!(bid_status -> rounds (round_id));
diesel::joinable!(bid_status -> users (user_id));
diesel::joinable!(bid_status_history -> audit_events (audit_event_id));
diesel::joinable!(bid_status_history -> bid_status (bid_status_id));
diesel::joinable!(canonical_area_membership -> areas (area_id));
diesel::joinable!(canonical_area_membership -> audit_events (audit_event_id));
diesel::joinable!(canonical_area_membership -> bid_years (bid_year_id));
diesel::joinable!(canonical_area_membership -> users (user_id));
diesel::joinable!(bid_windows -> areas (area_id));
diesel::joinable!(bid_windows -> bid_years (bid_year_id));
diesel::joinable!(bid_windows -> users (user_id));
diesel::joinable!(canonical_bid_order -> audit_events (audit_event_id));
diesel::joinable!(canonical_bid_order -> bid_years (bid_year_id));
diesel::joinable!(canonical_bid_order -> users (user_id));
diesel::joinable!(canonical_bid_windows -> audit_events (audit_event_id));
diesel::joinable!(canonical_bid_windows -> bid_years (bid_year_id));
diesel::joinable!(canonical_bid_windows -> users (user_id));
diesel::joinable!(canonical_eligibility -> audit_events (audit_event_id));
diesel::joinable!(canonical_eligibility -> bid_years (bid_year_id));
diesel::joinable!(canonical_eligibility -> users (user_id));
diesel::joinable!(round_groups -> bid_years (bid_year_id));
diesel::joinable!(rounds -> round_groups (round_group_id));
diesel::joinable!(sessions -> operators (operator_id));
diesel::joinable!(state_snapshots -> areas (area_id));
diesel::joinable!(state_snapshots -> audit_events (event_id));
diesel::joinable!(state_snapshots -> bid_years (bid_year_id));
diesel::joinable!(users -> areas (area_id));
diesel::joinable!(users -> bid_years (bid_year_id));

diesel::allow_tables_to_appear_in_same_query!(
    areas,
    audit_events,
    bid_status,
    bid_status_history,
    bid_years,
    bid_windows,
    canonical_area_membership,
    canonical_bid_order,
    canonical_bid_windows,
    canonical_eligibility,
    operators,
    round_groups,
    rounds,
    sessions,
    state_snapshots,
    users,
);

// Allow GROUP BY queries with columns from joined tables
diesel::allow_columns_to_appear_in_same_group_by_clause!(bid_years::year, areas::area_code,);
