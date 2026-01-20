// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! CSV preview and validation for bulk user import.
//!
//! This module provides CSV parsing and validation for user data without
//! persisting or mutating canonical state.

use csv::StringRecord;
use std::collections::{HashMap, HashSet};
use zab_bid::BootstrapMetadata;
use zab_bid_domain::{
    Area, BidYear, Crew, Initials, SeniorityData, User, UserType, validate_user_fields,
};
use zab_bid_persistence::SqlitePersistence;

use crate::error::ApiError;

/// A single row result from CSV preview validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvRowResult {
    /// The row number (1-based, excluding header).
    pub row_number: usize,
    /// The parsed initials (if valid).
    pub initials: Option<String>,
    /// The parsed name (if valid).
    pub name: Option<String>,
    /// The parsed area ID (if valid).
    pub area_id: Option<String>,
    /// The parsed user type (if valid).
    pub user_type: Option<String>,
    /// The parsed crew (if valid).
    pub crew: Option<u8>,
    /// The row status.
    pub status: CsvRowStatus,
    /// Zero or more validation errors.
    pub errors: Vec<String>,
}

/// Status of a CSV row validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvRowStatus {
    /// Row is valid and can be imported.
    Valid,
    /// Row has validation errors and cannot be imported.
    Invalid,
}

/// Result of CSV preview validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvPreviewResult {
    /// Per-row validation results.
    pub rows: Vec<CsvRowResult>,
    /// Total number of rows.
    pub total_rows: usize,
    /// Number of valid rows.
    pub valid_count: usize,
    /// Number of invalid rows.
    pub invalid_count: usize,
}

/// Required CSV column headers (case-insensitive, normalized).
const REQUIRED_HEADERS: &[&str] = &[
    "initials",
    "name",
    "area_id",
    "crew",
    "user_type",
    "service_computation_date",
    "eod_faa_date",
];

/// Normalizes a CSV header string for case-insensitive, whitespace-tolerant matching.
fn normalize_header(header: &str) -> String {
    header.trim().to_lowercase().replace(' ', "_")
}

/// Validates that all required headers are present in the CSV.
fn validate_headers(headers: &StringRecord) -> Result<HashMap<String, usize>, ApiError> {
    let mut header_map: HashMap<String, usize> = HashMap::new();

    // Build normalized header map
    for (idx, header) in headers.iter().enumerate() {
        let normalized: String = normalize_header(header);
        header_map.insert(normalized, idx);
    }

    // Check all required headers are present
    let mut missing: Vec<String> = Vec::new();
    for required in REQUIRED_HEADERS {
        if !header_map.contains_key(*required) {
            missing.push(String::from(*required));
        }
    }

    if !missing.is_empty() {
        return Err(ApiError::InvalidCsvFormat {
            reason: format!("Missing required headers: {}", missing.join(", ")),
        });
    }

    Ok(header_map)
}

/// Extracts and validates a required field from a CSV row.
fn parse_required_field(
    get_field: &impl Fn(&str) -> Option<String>,
    field_name: &str,
    errors: &mut Vec<String>,
) -> String {
    get_field(field_name).unwrap_or_else(|| {
        errors.push(format!("{field_name}: required field is missing or empty"));
        String::new()
    })
}

/// Parses a CSV row into a `User` domain object if possible.
///
/// Returns `Ok(User)` if all fields are valid, or `Err(Vec<String>)` with error messages.
fn parse_csv_row(
    record: &StringRecord,
    header_map: &HashMap<String, usize>,
    bid_year: &BidYear,
) -> Result<User, Vec<String>> {
    let mut errors: Vec<String> = Vec::new();

    // Extract fields using header map
    let get_field = |name: &str| -> Option<String> {
        header_map
            .get(name)
            .and_then(|&idx| record.get(idx))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    // Parse required fields
    let initials_str: String =
        parse_required_field(&get_field, "initials", &mut errors).to_uppercase();
    let name: String = parse_required_field(&get_field, "name", &mut errors);
    let area_id_str: String = parse_required_field(&get_field, "area_id", &mut errors);
    let user_type_str: String = parse_required_field(&get_field, "user_type", &mut errors);
    let service_computation_date: String =
        parse_required_field(&get_field, "service_computation_date", &mut errors);
    let eod_faa_date: String = parse_required_field(&get_field, "eod_faa_date", &mut errors);

    // Parse crew (required in CSV, but optional in domain)
    let crew_str: Option<String> = get_field("crew");
    #[allow(clippy::option_if_let_else)]
    let crew_opt: Option<u8> = if let Some(val) = crew_str {
        if let Ok(num) = val.parse::<u8>() {
            Some(num)
        } else {
            errors.push(format!("crew: invalid number '{val}'"));
            None
        }
    } else {
        errors.push(String::from("crew: required field is missing or empty"));
        None
    };

    // Optional seniority fields (use empty string if missing)
    let cumulative_natca_bu_date: String =
        get_field("cumulative_natca_bu_date").unwrap_or_default();
    let natca_bu_date: String = get_field("natca_bu_date").unwrap_or_default();

    // Parse lottery_value (optional)
    let lottery_value: Option<u32> = get_field("lottery_value").and_then(|val| {
        val.parse::<u32>().map_or_else(
            |_| {
                errors.push(format!("lottery_value: invalid number '{val}'"));
                None
            },
            Some,
        )
    });

    // If any required field is missing, return early
    if !errors.is_empty() {
        return Err(errors);
    }

    // Build domain objects - collect all errors before returning
    let initials: Initials = Initials::new(&initials_str);
    let area: Area = Area::new(&area_id_str);

    // Parse user_type - collect error but continue validation
    let user_type_opt: Option<UserType> = match user_type_str.as_str() {
        "CPC" => Some(UserType::CPC),
        "CPC-IT" => Some(UserType::CpcIt),
        "Dev-R" => Some(UserType::DevR),
        "Dev-D" => Some(UserType::DevD),
        _ => {
            errors.push(format!(
                "user_type: invalid value '{user_type_str}' (must be CPC, CPC-IT, Dev-R, or Dev-D)"
            ));
            None
        }
    };

    // Parse crew - collect error but continue validation
    let crew: Option<Crew> = crew_opt.and_then(|num| match Crew::new(num) {
        Ok(c) => Some(c),
        Err(e) => {
            errors.push(format!("crew: {e}"));
            None
        }
    });

    // If we accumulated any errors during parsing, return them all
    if !errors.is_empty() {
        return Err(errors);
    }

    // All validations passed - build the user
    let user_type: UserType = user_type_opt.ok_or_else(|| {
        vec![String::from(
            "user_type missing after validation (internal error)",
        )]
    })?;

    let seniority_data: SeniorityData = SeniorityData::new(
        cumulative_natca_bu_date,
        natca_bu_date,
        eod_faa_date,
        service_computation_date,
        lottery_value,
    );

    let user: User = User::new(
        bid_year.clone(),
        initials,
        name,
        area,
        user_type,
        crew,
        seniority_data,
        false, // excluded_from_bidding: default to false
        false, // excluded_from_leave_calculation: default to false
    );

    Ok(user)
}

/// Validates a parsed user against domain rules and persistence state.
fn validate_user_against_metadata(
    user: &User,
    metadata: &BootstrapMetadata,
    persistence: &mut SqlitePersistence,
    seen_initials: &HashSet<String>,
) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();

    // Validate user fields (domain-level checks)
    if let Err(e) = validate_user_fields(user) {
        errors.push(format!("validation: {e}"));
    }

    // Check if area exists in metadata
    let area_exists: bool = metadata
        .areas
        .iter()
        .any(|(by, a)| by == &user.bid_year && a.id() == user.area.id());

    if !area_exists {
        errors.push(format!(
            "area_id: area '{}' does not exist in bid year {}",
            user.area.id(),
            user.bid_year.year()
        ));
    }

    // Check initials uniqueness against existing state across all areas
    // We need to check all areas in the bid year
    let mut initials_exists_in_db = false;
    for (bid_year, area) in &metadata.areas {
        if bid_year != &user.bid_year {
            continue;
        }

        if let Ok(state) = persistence.get_current_state(bid_year, area)
            && state
                .users
                .iter()
                .any(|u| u.initials.value() == user.initials.value())
        {
            initials_exists_in_db = true;
            break;
        }
    }

    if initials_exists_in_db {
        errors.push(format!(
            "initials: user with initials '{}' already exists in bid year {}",
            user.initials.value(),
            user.bid_year.year()
        ));
    }

    // Check initials uniqueness within the CSV itself
    if seen_initials.contains(user.initials.value()) {
        errors.push(format!(
            "initials: duplicate within CSV - '{}' appears multiple times",
            user.initials.value()
        ));
    }

    errors
}

/// Previews and validates CSV user data without persisting.
///
/// # Arguments
///
/// * `csv_content` - The raw CSV content as a string
/// * `bid_year` - The bid year to validate against
/// * `metadata` - The current bootstrap metadata
/// * `persistence` - The persistence layer for querying existing users
///
/// # Returns
///
/// * `Ok(CsvPreviewResult)` with per-row validation results
/// * `Err(ApiError)` if CSV format is invalid or cannot be parsed
#[allow(clippy::too_many_lines)]
pub fn preview_csv_users(
    csv_content: &str,
    bid_year: &BidYear,
    metadata: &BootstrapMetadata,
    persistence: &mut SqlitePersistence,
) -> Result<CsvPreviewResult, ApiError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(csv_content.as_bytes());

    // Get and validate headers
    let headers: StringRecord = reader
        .headers()
        .map_err(|e| ApiError::InvalidCsvFormat {
            reason: format!("Failed to read CSV headers: {e}"),
        })?
        .clone();

    let header_map: HashMap<String, usize> = validate_headers(&headers)?;

    let mut results: Vec<CsvRowResult> = Vec::new();
    let mut seen_initials: HashSet<String> = HashSet::new();

    // Process each row
    for (idx, result) in reader.records().enumerate() {
        let row_number: usize = idx + 1;

        let record: StringRecord = match result {
            Ok(rec) => rec,
            Err(e) => {
                results.push(CsvRowResult {
                    row_number,
                    initials: None,
                    name: None,
                    area_id: None,
                    user_type: None,
                    crew: None,
                    status: CsvRowStatus::Invalid,
                    errors: vec![format!("CSV parse error: {e}")],
                });
                continue;
            }
        };

        // Try to parse the row
        match parse_csv_row(&record, &header_map, bid_year) {
            Ok(user) => {
                // Validate against domain rules and metadata
                let validation_errors: Vec<String> =
                    validate_user_against_metadata(&user, metadata, persistence, &seen_initials);

                let status: CsvRowStatus = if validation_errors.is_empty() {
                    CsvRowStatus::Valid
                } else {
                    CsvRowStatus::Invalid
                };

                // Track initials for intra-CSV uniqueness check
                seen_initials.insert(user.initials.value().to_string());

                results.push(CsvRowResult {
                    row_number,
                    initials: Some(user.initials.value().to_string()),
                    name: Some(user.name.clone()),
                    area_id: Some(user.area.id().to_string()),
                    user_type: Some(format!("{:?}", user.user_type)),
                    crew: user.crew.as_ref().map(Crew::number),
                    status,
                    errors: validation_errors,
                });
            }
            Err(mut parse_errors) => {
                // Parsing failed - extract what we can for display
                let initials_opt: Option<String> = header_map
                    .get("initials")
                    .and_then(|&idx| record.get(idx))
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty());

                let name_opt: Option<String> = header_map
                    .get("name")
                    .and_then(|&idx| record.get(idx))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                let area_id_opt: Option<String> = header_map
                    .get("area_id")
                    .and_then(|&idx| record.get(idx))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                let user_type_opt: Option<String> = header_map
                    .get("user_type")
                    .and_then(|&idx| record.get(idx))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                let crew_opt: Option<u8> = header_map
                    .get("crew")
                    .and_then(|&idx| record.get(idx).and_then(|s| s.trim().parse::<u8>().ok()));

                // Even though parsing failed, we can still run some validation checks
                // to provide more complete error feedback

                // Check initials length (domain rule)
                if let Some(ref initials) = initials_opt
                    && initials.len() != 2
                {
                    parse_errors.push(
                        "validation: Invalid initials: Initials must be exactly 2 characters"
                            .to_string(),
                    );
                }

                // Check if area exists in metadata
                if let Some(ref area_code) = area_id_opt {
                    let area_exists: bool = metadata
                        .areas
                        .iter()
                        .any(|(by, a)| by == bid_year && a.id() == area_code);

                    if !area_exists {
                        parse_errors.push(format!(
                            "area_id: area '{area_code}' does not exist in bid year {}",
                            bid_year.year()
                        ));
                    }
                }

                results.push(CsvRowResult {
                    row_number,
                    initials: initials_opt,
                    name: name_opt,
                    area_id: area_id_opt,
                    user_type: user_type_opt,
                    crew: crew_opt,
                    status: CsvRowStatus::Invalid,
                    errors: parse_errors,
                });
            }
        }
    }

    let total_rows: usize = results.len();
    let valid_count: usize = results
        .iter()
        .filter(|r| r.status == CsvRowStatus::Valid)
        .count();
    let invalid_count: usize = total_rows - valid_count;

    Ok(CsvPreviewResult {
        rows: results,
        total_rows,
        valid_count,
        invalid_count,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use zab_bid::{BootstrapResult, Command, apply_bootstrap};
    use zab_bid_audit::{Actor, Cause};

    fn create_test_bid_year() -> BidYear {
        BidYear::new(2026)
    }

    fn create_test_persistence() -> SqlitePersistence {
        SqlitePersistence::new_in_memory().expect("Failed to create in-memory persistence")
    }

    fn create_test_actor() -> Actor {
        Actor::with_operator(
            String::from("test-actor"),
            String::from("admin"),
            1,
            String::from("test_admin"),
            String::from("Test Admin"),
        )
    }

    fn create_test_cause() -> Cause {
        Cause::new(String::from("test"), String::from("Test bootstrap"))
    }

    fn bootstrap_test_persistence(persistence: &mut SqlitePersistence) {
        // Create test operator first to satisfy foreign key constraints
        persistence
            .create_operator("test_admin", "Test Admin", "password", "Admin")
            .expect("Failed to create operator");

        let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

        // Create bid year using bootstrap command
        // January 4, 2026 is a Sunday
        let create_bid_year_cmd: Command = Command::CreateBidYear {
            year: 2026,
            start_date: time::Date::from_calendar_date(2026, time::Month::January, 4).unwrap(),
            num_pay_periods: 26,
        };
        let placeholder_bid_year = BidYear::new(2026);
        let bid_year_result: BootstrapResult = apply_bootstrap(
            &metadata,
            &placeholder_bid_year,
            create_bid_year_cmd,
            create_test_actor(),
            create_test_cause(),
        )
        .expect("Failed to apply bootstrap bid year");
        persistence
            .persist_bootstrap(&bid_year_result)
            .expect("Failed to persist bid year");
        metadata.bid_years.push(BidYear::new(2026));

        // Create area using bootstrap command
        let create_area_cmd: Command = Command::CreateArea {
            area_id: String::from("ZAB"),
        };
        let active_bid_year = BidYear::new(2026);
        let area_result: BootstrapResult = apply_bootstrap(
            &metadata,
            &active_bid_year,
            create_area_cmd,
            create_test_actor(),
            create_test_cause(),
        )
        .expect("Failed to apply bootstrap area");
        persistence
            .persist_bootstrap(&area_result)
            .expect("Failed to persist area");
    }

    #[test]
    fn test_normalize_header() {
        assert_eq!(normalize_header("Initials"), "initials");
        assert_eq!(normalize_header("Area ID"), "area_id");
        assert_eq!(normalize_header("  User Type  "), "user_type");
        assert_eq!(
            normalize_header("SERVICE COMPUTATION DATE"),
            "service_computation_date"
        );
    }

    #[test]
    fn test_missing_required_headers() {
        let csv: &str = "initials,name\nAB,Alice Brown\n";
        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: Result<CsvPreviewResult, ApiError> =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence);
        assert!(result.is_err());
        match result {
            Err(ApiError::InvalidCsvFormat { reason }) => {
                assert!(reason.contains("Missing required headers"));
            }
            _ => panic!("Expected InvalidCsvFormat error"),
        }
    }

    #[test]
    fn test_valid_csv_all_fields() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date,lottery_value,cumulative_natca_bu_date,natca_bu_date\n\
                         AB,Alice Brown,ZAB,1,CPC,2020-01-01,2020-01-01,42,2019-01-01,2019-06-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.total_rows, 1);
        assert_eq!(result.valid_count, 1);
        assert_eq!(result.invalid_count, 0);

        let row: &CsvRowResult = &result.rows[0];
        assert_eq!(row.status, CsvRowStatus::Valid);
        assert_eq!(row.initials, Some(String::from("AB")));
        assert_eq!(row.name, Some(String::from("Alice Brown")));
        assert_eq!(row.area_id, Some(String::from("ZAB")));
        assert_eq!(row.crew, Some(1));
        assert!(row.errors.is_empty());
    }

    #[test]
    fn test_column_order_independence() {
        let csv: &str = "name,eod_faa_date,initials,user_type,service_computation_date,area_id,crew\n\
                         Alice Brown,2020-01-01,AB,CPC,2020-01-01,ZAB,1\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.valid_count, 1);
    }

    #[test]
    fn test_extra_columns_ignored() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date,extra_column,another_extra\n\
                         AB,Alice Brown,ZAB,1,CPC,2020-01-01,2020-01-01,ignored,also_ignored\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.valid_count, 1);
    }

    #[test]
    fn test_invalid_initials() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         A,Alice Brown,ZAB,1,CPC,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.invalid_count, 1);
        let row: &CsvRowResult = &result.rows[0];
        assert_eq!(row.status, CsvRowStatus::Invalid);
        assert!(!row.errors.is_empty());
    }

    #[test]
    fn test_invalid_crew() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         AB,Alice Brown,ZAB,8,CPC,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.invalid_count, 1);
        let row: &CsvRowResult = &result.rows[0];
        assert_eq!(row.status, CsvRowStatus::Invalid);
        assert!(row.errors.iter().any(|e| e.contains("crew")));
    }

    #[test]
    fn test_invalid_user_type() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         AB,Alice Brown,ZAB,1,INVALID,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.invalid_count, 1);
        let row: &CsvRowResult = &result.rows[0];
        assert!(row.errors.iter().any(|e| e.contains("user_type")));
    }

    #[test]
    fn test_area_does_not_exist() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         AB,Alice Brown,NONEXISTENT,1,CPC,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.invalid_count, 1);
        let row: &CsvRowResult = &result.rows[0];
        assert!(row.errors.iter().any(|e| e.contains("does not exist")));
    }

    #[test]
    fn test_duplicate_initials_in_csv() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         AB,Alice Brown,ZAB,1,CPC,2020-01-01,2020-01-01\n\
                         AB,Another Person,ZAB,2,CPC,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.total_rows, 2);
        // First occurrence is valid, second is invalid
        assert_eq!(result.valid_count, 1);
        assert_eq!(result.invalid_count, 1);

        let row2: &CsvRowResult = &result.rows[1];
        assert!(
            row2.errors
                .iter()
                .any(|e| e.contains("duplicate within CSV"))
        );
    }

    #[test]
    fn test_mixed_valid_invalid_rows() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         AB,Alice Brown,ZAB,1,CPC,2020-01-01,2020-01-01\n\
                         CD,Charlie Delta,ZAB,8,CPC,2020-01-01,2020-01-01\n\
                         EF,Eve Foster,ZAB,2,Dev-R,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.total_rows, 3);
        assert_eq!(result.valid_count, 2);
        assert_eq!(result.invalid_count, 1);
    }

    #[test]
    fn test_missing_required_field() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         AB,,ZAB,1,CPC,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.invalid_count, 1);
        let row: &CsvRowResult = &result.rows[0];
        assert!(row.errors.iter().any(|e| e.contains("name")));
    }

    // Gap 5: Multiple errors on a single CSV row
    #[test]
    fn test_multiple_errors_on_single_row() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         A,Alice Brown,NONEXISTENT,8,INVALID,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.total_rows, 1);
        assert_eq!(result.invalid_count, 1);

        let row: &CsvRowResult = &result.rows[0];
        assert_eq!(row.status, CsvRowStatus::Invalid);
        assert_eq!(row.row_number, 1);

        // Verify all three errors are reported (not short-circuited)
        assert!(
            row.errors.len() >= 3,
            "Expected at least 3 errors, got {}: {:?}",
            row.errors.len(),
            row.errors
        );

        // Check for invalid initials error
        assert!(
            row.errors
                .iter()
                .any(|e| e.contains("initials") || e.contains("Initials")),
            "Missing initials error. Errors: {:?}",
            row.errors
        );

        // Check for nonexistent area error
        assert!(
            row.errors
                .iter()
                .any(|e| e.contains("area") && e.contains("does not exist")),
            "Missing area does not exist error. Errors: {:?}",
            row.errors
        );

        // Check for invalid crew error
        assert!(
            row.errors.iter().any(|e| e.contains("crew")),
            "Missing crew error. Errors: {:?}",
            row.errors
        );
    }

    // Gap 5: Multiple rows with independent failures
    #[test]
    fn test_multiple_rows_with_independent_failures() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         A,Alice,ZAB,1,CPC,2020-01-01,2020-01-01\n\
                         BC,Bob,NONEXISTENT,2,CPC,2020-01-01,2020-01-01\n\
                         CD,Charlie,ZAB,9,CPC,2020-01-01,2020-01-01\n\
                         EF,Eve,ZAB,3,BADTYPE,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.total_rows, 4);
        assert_eq!(result.invalid_count, 4);

        // Row 1: invalid initials (too short)
        let row1: &CsvRowResult = &result.rows[0];
        assert_eq!(row1.row_number, 1);
        assert_eq!(row1.status, CsvRowStatus::Invalid);
        assert!(
            row1.errors
                .iter()
                .any(|e| e.contains("initials") || e.contains("Initials"))
        );

        // Row 2: nonexistent area
        let row2: &CsvRowResult = &result.rows[1];
        assert_eq!(row2.row_number, 2);
        assert_eq!(row2.status, CsvRowStatus::Invalid);
        assert!(
            row2.errors
                .iter()
                .any(|e| e.contains("area") && e.contains("does not exist"))
        );

        // Row 3: invalid crew (9 is out of range)
        let row3: &CsvRowResult = &result.rows[2];
        assert_eq!(row3.row_number, 3);
        assert_eq!(row3.status, CsvRowStatus::Invalid);
        assert!(row3.errors.iter().any(|e| e.contains("crew")));

        // Row 4: invalid user type
        let row4: &CsvRowResult = &result.rows[3];
        assert_eq!(row4.row_number, 4);
        assert_eq!(row4.status, CsvRowStatus::Invalid);
        assert!(row4.errors.iter().any(|e| e.contains("user_type")));

        // Verify each row's errors are independent
        // Row 1 should only have initials error, not area or crew errors
        assert!(
            !row1.errors.iter().any(|e| e.contains("does not exist")),
            "Row 1 should not have area error"
        );
        assert!(
            !row2
                .errors
                .iter()
                .any(|e| e.contains("initials") || e.contains("Initials")),
            "Row 2 should not have initials error"
        );
    }

    // Gap 5: Error messages reference correct row numbers
    #[test]
    fn test_error_messages_have_correct_row_numbers() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         AB,Alice,ZAB,1,CPC,2020-01-01,2020-01-01\n\
                         CD,Charlie,ZAB,2,CPC,2020-01-01,2020-01-01\n\
                         E,Invalid,ZAB,3,CPC,2020-01-01,2020-01-01\n\
                         FG,Frank,ZAB,4,CPC,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.total_rows, 4);
        assert_eq!(result.valid_count, 3);
        assert_eq!(result.invalid_count, 1);

        // Row 3 (row_number = 3) should be invalid
        let row3: &CsvRowResult = &result.rows[2];
        assert_eq!(row3.row_number, 3, "Row number should be 3 (1-based)");
        assert_eq!(row3.status, CsvRowStatus::Invalid);

        // Valid rows should have correct row numbers
        assert_eq!(result.rows[0].row_number, 1);
        assert_eq!(result.rows[1].row_number, 2);
        assert_eq!(result.rows[3].row_number, 4);
    }

    // Gap 5: Mixed valid and invalid rows - valid rows not rejected
    #[test]
    fn test_mixed_valid_invalid_rows_preserves_valid() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         AB,Alice,ZAB,1,CPC,2020-01-01,2020-01-01\n\
                         X,Invalid,ZAB,2,CPC,2020-01-01,2020-01-01\n\
                         CD,Charlie,ZAB,3,CPC,2020-01-01,2020-01-01\n\
                         EF,Eve,BADAREA,4,CPC,2020-01-01,2020-01-01\n\
                         GH,George,ZAB,5,CPC,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence).expect("valid CSV");

        assert_eq!(result.total_rows, 5);
        assert_eq!(result.valid_count, 3);
        assert_eq!(result.invalid_count, 2);

        // Verify valid rows are actually valid
        assert_eq!(result.rows[0].status, CsvRowStatus::Valid);
        assert!(result.rows[0].errors.is_empty());
        assert_eq!(result.rows[0].initials, Some(String::from("AB")));

        assert_eq!(result.rows[2].status, CsvRowStatus::Valid);
        assert!(result.rows[2].errors.is_empty());
        assert_eq!(result.rows[2].initials, Some(String::from("CD")));

        assert_eq!(result.rows[4].status, CsvRowStatus::Valid);
        assert!(result.rows[4].errors.is_empty());
        assert_eq!(result.rows[4].initials, Some(String::from("GH")));

        // Verify invalid rows have errors
        assert_eq!(result.rows[1].status, CsvRowStatus::Invalid);
        assert!(!result.rows[1].errors.is_empty());

        assert_eq!(result.rows[3].status, CsvRowStatus::Invalid);
        assert!(!result.rows[3].errors.is_empty());
    }

    // Gap 5: Empty file (no headers, no data)
    #[test]
    fn test_empty_csv_file() {
        let csv: &str = "";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: Result<CsvPreviewResult, ApiError> =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence);

        assert!(result.is_err(), "Empty CSV should fail");
        match result {
            Err(ApiError::InvalidCsvFormat { reason }) => {
                assert!(
                    reason.contains("Failed to read CSV headers")
                        || reason.contains("Missing required headers"),
                    "Expected header-related error, got: {reason}"
                );
            }
            _ => panic!("Expected InvalidCsvFormat error for empty CSV"),
        }
    }

    // Gap 5: Header-only file (no data rows)
    #[test]
    fn test_header_only_csv_file() {
        let csv: &str =
            "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence)
                .expect("header-only CSV should succeed");

        assert_eq!(result.total_rows, 0, "Should have no data rows");
        assert_eq!(result.valid_count, 0);
        assert_eq!(result.invalid_count, 0);
        assert!(result.rows.is_empty());
    }

    // Gap 5: Duplicate headers
    #[test]
    fn test_duplicate_headers() {
        let csv: &str = "initials,name,initials,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         AB,Alice,AB,ZAB,1,CPC,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        // CSV with duplicate headers should parse (last occurrence wins in HashMap)
        // This is technically allowed by csv crate, so we just verify it doesn't crash
        let result: Result<CsvPreviewResult, ApiError> =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence);

        // Either it succeeds (using last occurrence) or fails with format error
        // Both behaviors are acceptable - the key is not crashing
        match result {
            Ok(preview) => {
                // If it succeeds, verify basic structure
                assert_eq!(preview.total_rows, 1);
            }
            Err(ApiError::InvalidCsvFormat { .. }) => {
                // Also acceptable - some CSV parsers reject duplicates
            }
            Err(e) => panic!("Unexpected error type: {e:?}"),
        }
    }

    // Gap 5: Error message determinism - verify stable ordering
    #[test]
    fn test_error_message_determinism() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         A,Bob,NONEXISTENT,9,INVALID,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        // Run the same CSV multiple times and verify error messages are identical
        let mut error_sets: Vec<Vec<String>> = Vec::new();

        for _ in 0..5 {
            let result: CsvPreviewResult =
                preview_csv_users(csv, &bid_year, &metadata, &mut persistence)
                    .expect("CSV should parse");

            assert_eq!(result.invalid_count, 1);
            let row: &CsvRowResult = &result.rows[0];
            error_sets.push(row.errors.clone());
        }

        // All error sets should be identical
        let first_errors: &Vec<String> = &error_sets[0];
        for errors in &error_sets[1..] {
            assert_eq!(
                errors, first_errors,
                "Error messages should be deterministic across runs"
            );
        }

        // Verify we have multiple errors
        assert!(first_errors.len() >= 3, "Should have at least 3 errors");
    }

    // Gap 5: Error messages reference correct column names
    #[test]
    fn test_error_messages_reference_column_names() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         A,Alice,BADAREA,99,BADTYPE,2020-01-01,2020-01-01\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence)
                .expect("CSV should parse");

        assert_eq!(result.invalid_count, 1);
        let row: &CsvRowResult = &result.rows[0];

        // Verify error messages contain the relevant field names
        let errors_text: String = row.errors.join(" ");

        // Should mention initials or validation (for the 1-char initials)
        assert!(
            errors_text.contains("initials")
                || errors_text.contains("Initials")
                || errors_text.contains("validation"),
            "Should mention initials field. Errors: {:?}",
            row.errors
        );

        // Should mention area
        assert!(
            errors_text.contains("area"),
            "Should mention area field. Errors: {:?}",
            row.errors
        );

        // Should mention crew
        assert!(
            errors_text.contains("crew"),
            "Should mention crew field. Errors: {:?}",
            row.errors
        );

        // Should mention user_type
        assert!(
            errors_text.contains("user_type"),
            "Should mention user_type field. Errors: {:?}",
            row.errors
        );
    }

    // Gap 5: Multiple missing required fields
    #[test]
    fn test_multiple_missing_required_fields() {
        let csv: &str = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date\n\
                         AB,,,1,CPC,2020-01-01,\n";

        let bid_year: BidYear = create_test_bid_year();
        let mut persistence: SqlitePersistence = create_test_persistence();
        bootstrap_test_persistence(&mut persistence);
        let metadata: BootstrapMetadata = persistence
            .get_bootstrap_metadata()
            .expect("Failed to get metadata");

        let result: CsvPreviewResult =
            preview_csv_users(csv, &bid_year, &metadata, &mut persistence)
                .expect("CSV should parse");

        assert_eq!(result.invalid_count, 1);
        let row: &CsvRowResult = &result.rows[0];
        assert_eq!(row.status, CsvRowStatus::Invalid);

        // Should have errors for both missing fields
        assert!(
            row.errors.len() >= 2,
            "Should have at least 2 errors for missing fields"
        );

        let errors_text: String = row.errors.join(" ");
        assert!(
            errors_text.contains("name"),
            "Should mention missing name field. Errors: {:?}",
            row.errors
        );
        assert!(
            errors_text.contains("area_id"),
            "Should mention missing area_id field. Errors: {:?}",
            row.errors
        );
    }
}
