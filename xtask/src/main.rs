// Copyright (C) 2024-2025 Fred Clausen and the ratatui project contributors
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! # xtask - Project Automation and Infrastructure Orchestration
//!
//! ## Phase 24D: Multi-Backend Validation Infrastructure
//!
//! As of Phase 24D, this xtask provides explicit, opt-in backend validation
//! for MySQL/MariaDB in addition to the default `SQLite` backend.
//!
//! ### Backend Testing Commands
//!
//! - `cargo test` — Runs all standard tests against `SQLite` (fast, no infrastructure)
//! - `cargo xtask test-mariadb` — Runs backend validation tests against `MariaDB`
//!
//! ### Implementation Details
//!
//! The `test-mariadb` command:
//! - Orchestrates Docker container lifecycle (start, wait, stop, cleanup)
//! - Provisions a `MariaDB` 11 container with test database
//! - Sets required environment variables for tests
//! - Executes explicitly ignored tests via `--ignored` flag
//! - Guarantees cleanup even on test failure
//!
//! ### Design Principles
//!
//! - No test infrastructure is embedded in test code
//! - No tests silently skip due to missing services
//! - External databases are opt-in only, never automatic
//! - Standard `cargo test` remains fast and infrastructure-free
//! - All backend-specific orchestration lives in xtask

#![deny(
    clippy::pedantic,
    //clippy::cargo,
    clippy::nursery,
    clippy::style,
    clippy::correctness,
    clippy::all
)]

use std::{fmt::Debug, io, process::Output, vec};

use cargo_metadata::MetadataCommand;
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use color_eyre::{eyre::Context, Result};
use diesel::sql_types::{Integer, Text};
use diesel::{MysqlConnection, QueryableByName, RunQueryDsl, SqliteConnection};
use duct::cmd;
use std::collections::{BTreeMap, BTreeSet};
use tracing::level_filters::LevelFilter;
use tracing_log::AsTrace;

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.log_level())
        .without_time()
        .init();

    match args.run() {
        Ok(()) => (),
        Err(err) => {
            tracing::error!("{err}");
            std::process::exit(1);
        }
    }
    Ok(())
}

#[derive(Debug, Parser)]
#[command(bin_name = "cargo xtask", styles = clap_cargo::style::CLAP_STYLING)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,
}

impl Args {
    fn run(self) -> Result<()> {
        self.command.run()
    }

    fn log_level(&self) -> LevelFilter {
        self.verbosity.log_level_filter().as_trace()
    }
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// Run CI checks (lint, build, test)
    CI,

    /// Build the project
    #[command(visible_alias = "b")]
    Build,

    /// Run cargo check
    #[command(visible_alias = "c")]
    Check,

    /// Check if README.md is up-to-date
    #[command(visible_alias = "cr")]
    CheckReadme,

    /// Generate code coverage report
    #[command(visible_alias = "cov")]
    Coverage,

    /// Check dependencies
    #[command(visible_alias = "cd")]
    Deny,

    // Check unused dependencies
    #[command(visible_alias = "m")]
    Machete,

    /// Lint formatting, typos, clippy, and docs
    #[command(visible_alias = "l")]
    Lint,

    /// Run clippy on the project
    #[command(visible_alias = "cl")]
    LintClippy,

    /// Check documentation for errors and warnings
    #[command(visible_alias = "d")]
    LintDocs,

    /// Check for formatting issues in the project
    #[command(visible_alias = "lf")]
    LintFormatting,

    /// Lint markdown files
    #[command(visible_alias = "md")]
    LintMarkdown,

    /// Check for typos in the project
    #[command(visible_alias = "lt")]
    LintTypos,

    /// Fix clippy warnings in the project
    #[command(visible_alias = "fc")]
    FixClippy,

    /// Fix formatting issues in the project
    #[command(visible_alias = "fmt")]
    FixFormatting,

    /// Fix typos in the project
    #[command(visible_alias = "typos")]
    FixTypos,

    /// Run tests
    #[command(visible_alias = "t")]
    Test,

    /// Run doc tests
    #[command(visible_alias = "td")]
    TestDocs,

    /// Run lib tests
    #[command(visible_alias = "tl")]
    TestLibs,

    /// Run `MariaDB` backend validation tests
    #[command(visible_alias = "tm")]
    TestMariadb,

    /// Verify schema parity between `SQLite` and `MySQL` migrations
    #[command(visible_alias = "vm")]
    VerifyMigrations,
}

// #[derive(Clone, Debug, ValueEnum, PartialEq, Eq)]
// enum Backend {
//     Crossterm,
//     Termion,
//     Termwiz,
// }

impl Command {
    fn run(self) -> Result<()> {
        match self {
            Self::CI => ci(),
            Self::Build => build(),
            Self::Check => check(),
            Self::Deny => deny(),
            Self::Machete => machete(),
            Self::CheckReadme => check_readme(),
            Self::Coverage => coverage(),
            Self::Lint => lint(),
            Self::LintClippy => lint_clippy(),
            Self::LintDocs => lint_docs(),
            Self::LintFormatting => lint_format(),
            Self::LintTypos => lint_typos(),
            Self::LintMarkdown => lint_markdown(),
            Self::FixClippy => fix_clippy(),
            Self::FixFormatting => fix_format(),
            Self::FixTypos => fix_typos(),
            Self::Test => test(),
            Self::TestDocs => test_docs(),
            Self::TestLibs => test_libs(),
            Self::TestMariadb => test_mariadb(),
            Self::VerifyMigrations => verify_migrations(),
        }
    }
}

/// Run CI checks (lint, build, test)
fn ci() -> Result<()> {
    lint()?;
    deny()?;
    machete()?;
    build()?;
    test()?;
    // FIXME: This should not be in CI, and instead needs to be moved out. Requires changes to GitHub Actions and/or pre-check
    test_mariadb()?;
    verify_migrations()?;
    Ok(())
}

fn deny() -> Result<()> {
    run_cargo(vec!["deny", "check"])
}

fn machete() -> Result<()> {
    cmd!("cargo-machete").run_with_trace()?;
    Ok(())
}

/// Build the project
fn build() -> Result<()> {
    run_cargo(vec!["build", "--all-targets", "--all-features"])
}

/// Run cargo check
fn check() -> Result<()> {
    run_cargo(vec!["check", "--all-targets", "--all-features"])
}

/// Run cargo-rdme to check if README.md is up-to-date with the library documentation
fn check_readme() -> Result<()> {
    run_cargo(vec!["rdme", "--workspace-project", "freminal", "--check"])
}

/// Generate code coverage report
fn coverage() -> Result<()> {
    run_cargo(vec![
        "llvm-cov",
        "--lcov",
        "--output-path",
        "target/lcov.info",
        "--all-features",
    ])
}

/// Lint formatting, typos, clippy, and docs (and a soft fail on markdown)
fn lint() -> Result<()> {
    lint_clippy()?;
    lint_docs()?;
    lint_format()?;
    lint_typos()?;
    if let Err(err) = lint_markdown() {
        tracing::warn!("known issue: markdownlint is currently noisy and can be ignored: {err}");
    }
    Ok(())
}

/// Run clippy on the project
fn lint_clippy() -> Result<()> {
    run_cargo(vec![
        "clippy",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ])
}

/// Fix clippy warnings in the project
fn fix_clippy() -> Result<()> {
    run_cargo(vec![
        "clippy",
        "--all-targets",
        "--all-features",
        "--fix",
        "--allow-dirty",
        "--allow-staged",
        "--",
        "-D",
        "warnings",
    ])
}

/// Check that docs build without errors using docs.rs-equivalent flags
fn lint_docs() -> Result<()> {
    let meta = MetadataCommand::new()
        .exec()
        .wrap_err("failed to get cargo metadata")?;

    for package in meta.workspace_default_packages() {
        cmd(
            "cargo",
            [
                "doc",
                "--no-deps",
                "--all-features",
                "--package",
                &package.name,
            ],
        )
        .env_remove("CARGO")
        .env("RUSTUP_TOOLCHAIN", "nightly")
        .env("RUSTDOCFLAGS", "--cfg docsrs -D warnings")
        .run_with_trace()?;
    }

    Ok(())
}

/// Lint formatting issues in the project
fn lint_format() -> Result<()> {
    run_cargo_nightly(vec!["fmt", "--all", "--check"])
}

/// Fix formatting issues in the project
fn fix_format() -> Result<()> {
    run_cargo_nightly(vec!["fmt", "--all"])
}

/// Lint markdown files using [markdownlint-cli2](https://github.com/DavidAnson/markdownlint-cli2)
fn lint_markdown() -> Result<()> {
    cmd!("markdownlint-cli2", "**/*.md", "!target", "!**/target").run_with_trace()?;

    Ok(())
}

/// Check for typos in the project using [typos-cli](https://github.com/crate-ci/typos/)
fn lint_typos() -> Result<()> {
    cmd!("typos").run_with_trace()?;
    Ok(())
}

/// Fix typos in the project
fn fix_typos() -> Result<()> {
    cmd!("typos", "-w").run_with_trace()?;
    Ok(())
}

/// Run tests for libs, backends, and docs
fn test() -> Result<()> {
    test_libs()?;
    test_docs()?; // run last because it's slow
    Ok(())
}

/// Run doc tests for the workspace's default packages
fn test_docs() -> Result<()> {
    run_cargo(vec!["test", "--doc", "--all-features"])
}

/// Run lib tests for the workspace's default packages
fn test_libs() -> Result<()> {
    run_cargo(vec!["test", "--all-targets", "--all-features"])
}

/// Run a cargo subcommand with the default toolchain
fn run_cargo(args: Vec<&str>) -> Result<()> {
    cmd("cargo", args).run_with_trace()?;
    Ok(())
}

/// Run a cargo subcommand with the nightly toolchain
fn run_cargo_nightly(args: Vec<&str>) -> Result<()> {
    cmd("cargo", args)
        // CARGO env var is set because we're running in a cargo subcommand
        .env_remove("CARGO")
        .env("RUSTUP_TOOLCHAIN", "nightly")
        .run_with_trace()?;
    Ok(())
}

/// Run `MariaDB` backend validation tests
///
/// This command provides explicit, opt-in backend validation for MySQL/MariaDB.
/// It orchestrates all required infrastructure and runs ignored tests that
/// validate schema compatibility, constraint enforcement, and transaction behavior.
///
/// ## What This Command Does
///
/// 1. Validates Docker is available
/// 2. Starts a `MariaDB` 11 container with test database
/// 3. Waits for `MariaDB` to be ready (up to 30 seconds)
/// 4. Sets required environment variables:
///    - `DATABASE_URL`: `MySQL` connection string
///    - `ZABBID_TEST_BACKEND`: Set to "mariadb"
/// 5. Runs ignored backend validation tests from `zab-bid-persistence`
/// 6. Stops and removes the container (always, even on failure)
///
/// ## Requirements
///
/// - Docker must be installed and running
/// - Port 3307 must be available (used for `MariaDB`)
/// - `MySQL` client libraries must be available for compilation
///   (provided by Nix environment)
///
/// ## Usage
///
/// ```bash
/// cargo xtask test-mariadb
/// ```
///
/// ## What Gets Tested
///
/// - Migration application on MySQL/MariaDB
/// - Foreign key constraint enforcement
/// - Unique constraint behavior
/// - Transaction and rollback semantics
/// - Backend-specific SQL compatibility
///
/// ## Failures
///
/// The command fails if:
/// - Docker is not available
/// - `MariaDB` container fails to start
/// - `MariaDB` doesn't become ready within timeout
/// - Any backend validation test fails
///
/// Container cleanup happens regardless of test outcome.
fn test_mariadb() -> Result<()> {
    use std::thread::sleep;
    use std::time::Duration;

    tracing::info!("Starting MariaDB backend validation");

    // Validate Docker is available
    tracing::info!("Checking Docker availability");
    cmd!("docker", "--version")
        .run_with_trace()
        .wrap_err("Docker is not available. Please install Docker.")?;

    // Container configuration
    let container_name = "zabbid-test-mariadb";
    let db_name = "zabbid_test";
    let db_user = "zabbid";
    let db_password = "test_password";
    let db_port = "3307"; // Use non-standard port to avoid conflicts

    // Stop and remove any existing container
    tracing::info!("Cleaning up any existing test container");
    let _ = cmd!("docker", "stop", container_name).run();
    let _ = cmd!("docker", "rm", container_name).run();

    // Start MariaDB container
    tracing::info!("Starting MariaDB container: {}", container_name);
    cmd!(
        "docker",
        "run",
        "--name",
        container_name,
        "-e",
        format!("MARIADB_DATABASE={db_name}"),
        "-e",
        format!("MARIADB_USER={db_user}"),
        "-e",
        format!("MARIADB_PASSWORD={db_password}"),
        "-e",
        "MARIADB_ROOT_PASSWORD=root_password",
        "-p",
        format!("{db_port}:3306"),
        "-d",
        "mariadb:11"
    )
    .run_with_trace()
    .wrap_err("Failed to start MariaDB container")?;

    // Wait for MariaDB to be ready
    tracing::info!("Waiting for MariaDB to be ready...");
    let max_attempts = 30;
    let mut ready = false;

    for attempt in 1..=max_attempts {
        sleep(Duration::from_secs(1));
        tracing::debug!("Connection attempt {}/{}", attempt, max_attempts);

        let result = cmd!(
            "docker",
            "exec",
            container_name,
            "mariadb",
            "-u",
            db_user,
            format!("-p{db_password}"),
            "-e",
            "SELECT 1"
        )
        .run();

        if result.is_ok() {
            ready = true;
            tracing::info!("MariaDB is ready");
            break;
        }
    }

    if !ready {
        let _ = cmd!("docker", "stop", container_name).run();
        let _ = cmd!("docker", "rm", container_name).run();
        return Err(color_eyre::eyre::eyre!(
            "MariaDB did not become ready within timeout"
        ));
    }

    // Set environment variables for tests
    let database_url = format!("mysql://{db_user}:{db_password}@127.0.0.1:{db_port}/{db_name}");

    // Run ignored tests with explicit opt-in
    // Filter to only backend_validation_tests module to avoid running non-ignored tests
    tracing::info!("Running MariaDB backend validation tests");
    let test_result = cmd!(
        "cargo",
        "test",
        "--package",
        "zab-bid-persistence",
        "backend_validation_tests",
        "--",
        "--ignored",
        "--test-threads=1"
    )
    .env("DATABASE_URL", &database_url)
    .env("ZABBID_TEST_BACKEND", "mariadb")
    .run_with_trace();

    // Always cleanup container
    tracing::info!("Stopping MariaDB container");
    let _ = cmd!("docker", "stop", container_name).run();
    let _ = cmd!("docker", "rm", container_name).run();

    // Propagate test result
    test_result.wrap_err("MariaDB backend validation tests failed")?;

    tracing::info!("MariaDB backend validation completed successfully");
    Ok(())
}

/// Verify schema parity between `SQLite` and `MySQL` migrations
///
/// This command enforces that backend-specific migrations in `migrations/` (`SQLite`)
/// and `migrations_mysql/` (`MySQL`) produce semantically identical schemas.
///
/// ## What This Command Does
///
/// 1. Provisions ephemeral databases:
///    - `SQLite` (in-memory)
///    - `MariaDB` (Docker container)
/// 2. Applies backend-specific migrations to each
/// 3. Introspects resulting schemas (tables, columns, types, constraints)
/// 4. Normalizes backend-specific type representations
/// 5. Compares schemas structurally
/// 6. Fails hard on any mismatch
/// 7. Cleans up all resources (always, even on failure)
///
/// ## Requirements
///
/// - Docker must be installed and running
/// - Port 3308 must be available (used for `MariaDB` verification)
///
/// ## Usage
///
/// ```bash
/// cargo xtask verify-migrations
/// ```
///
/// ## Failures
///
/// The command fails if:
/// - Docker is not available
/// - `MariaDB` container fails to start
/// - Migrations fail to apply on either backend
/// - Schemas do not match structurally
///
/// Container cleanup happens regardless of outcome.
#[allow(clippy::too_many_lines)]
fn verify_migrations() -> Result<()> {
    use std::thread::sleep;
    use std::time::Duration;

    use diesel::Connection;
    use diesel_migrations::{embed_migrations, MigrationHarness};

    tracing::info!("Starting schema parity verification");

    // Validate Docker is available
    tracing::info!("Checking Docker availability");
    cmd!("docker", "--version")
        .run_with_trace()
        .wrap_err("Docker is not available. Please install Docker.")?;

    // Container configuration
    let container_name = "zabbid-verify-migrations";
    let db_name = "zabbid_verify";
    let db_user = "zabbid";
    let db_password = "verify_password";
    let db_port = "3308"; // Different port from test-mariadb to avoid conflicts

    // Stop and remove any existing container
    tracing::info!("Cleaning up any existing verification container");
    let _ = cmd!("docker", "stop", container_name).run();
    let _ = cmd!("docker", "rm", container_name).run();

    // Start MariaDB container
    tracing::info!("Starting MariaDB container: {}", container_name);
    cmd!(
        "docker",
        "run",
        "--name",
        container_name,
        "-e",
        format!("MARIADB_DATABASE={db_name}"),
        "-e",
        format!("MARIADB_USER={db_user}"),
        "-e",
        format!("MARIADB_PASSWORD={db_password}"),
        "-e",
        "MARIADB_ROOT_PASSWORD=root_password",
        "-p",
        format!("{db_port}:3306"),
        "-d",
        "mariadb:11"
    )
    .run_with_trace()
    .wrap_err("Failed to start MariaDB container")?;

    // Define cleanup function
    let cleanup = || {
        tracing::info!("Cleaning up MariaDB container");
        let _ = cmd!("docker", "stop", container_name).run();
        let _ = cmd!("docker", "rm", container_name).run();
    };

    // Wait for MariaDB to be ready
    tracing::info!("Waiting for MariaDB to be ready...");
    let max_attempts = 30;
    let mut ready = false;

    for attempt in 1..=max_attempts {
        sleep(Duration::from_secs(1));
        tracing::debug!("Connection attempt {}/{}", attempt, max_attempts);

        let result = cmd!(
            "docker",
            "exec",
            container_name,
            "mariadb",
            "-u",
            db_user,
            format!("-p{db_password}"),
            "-e",
            "SELECT 1"
        )
        .run();

        if result.is_ok() {
            ready = true;
            tracing::info!("MariaDB is ready");
            break;
        }
    }

    if !ready {
        cleanup();
        return Err(color_eyre::eyre::eyre!(
            "MariaDB did not become ready within timeout"
        ));
    }

    // Apply migrations and introspect schemas
    let verification_result = (|| -> Result<()> {
        // SQLite migrations
        tracing::info!("Applying SQLite migrations");
        #[allow(clippy::items_after_statements)]
        const SQLITE_MIGRATIONS: diesel_migrations::EmbeddedMigrations =
            embed_migrations!("../crates/persistence/migrations");

        let mut sqlite_conn = SqliteConnection::establish(":memory:")
            .wrap_err("Failed to create SQLite in-memory database")?;

        diesel::sql_query("PRAGMA foreign_keys = ON")
            .execute(&mut sqlite_conn)
            .wrap_err("Failed to enable foreign keys on SQLite")?;

        sqlite_conn
            .run_pending_migrations(SQLITE_MIGRATIONS)
            .map_err(|e| color_eyre::eyre::eyre!("Failed to apply SQLite migrations: {}", e))?;

        tracing::info!("SQLite migrations applied successfully");

        // MySQL migrations
        tracing::info!("Applying MySQL migrations");
        #[allow(clippy::items_after_statements)]
        const MYSQL_MIGRATIONS: diesel_migrations::EmbeddedMigrations =
            embed_migrations!("../crates/persistence/migrations_mysql");

        let database_url = format!("mysql://{db_user}:{db_password}@127.0.0.1:{db_port}/{db_name}");
        let mut mysql_conn =
            MysqlConnection::establish(&database_url).wrap_err("Failed to connect to MariaDB")?;

        mysql_conn
            .run_pending_migrations(MYSQL_MIGRATIONS)
            .map_err(|e| color_eyre::eyre::eyre!("Failed to apply MySQL migrations: {}", e))?;

        tracing::info!("MySQL migrations applied successfully");

        // Introspect SQLite schema
        tracing::info!("Introspecting SQLite schema");
        let sqlite_schema = introspect_sqlite_schema(&mut sqlite_conn)?;

        // Introspect MySQL schema
        tracing::info!("Introspecting MySQL schema");
        let mysql_schema = introspect_mysql_schema(&mut mysql_conn)?;

        // Compare schemas
        tracing::info!("Comparing schemas");
        compare_schemas(&sqlite_schema, &mysql_schema)?;

        tracing::info!("✓ Schema parity verification passed");
        Ok(())
    })();

    // Always cleanup
    cleanup();

    // Propagate result
    verification_result
}

/// Normalized schema representation
#[derive(Debug, Clone, PartialEq, Eq)]
struct Schema {
    tables: BTreeMap<String, Table>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Table {
    columns: BTreeMap<String, Column>,
    primary_keys: BTreeSet<String>,
    foreign_keys: BTreeSet<ForeignKey>,
    unique_constraints: BTreeSet<UniqueConstraint>,
    indexes: BTreeSet<Index>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Column {
    name: String,
    normalized_type: String,
    nullable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ForeignKey {
    from_column: String,
    to_table: String,
    to_column: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct UniqueConstraint {
    columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Index {
    name: String,
    columns: Vec<String>,
}

/// Introspect `SQLite` schema
#[allow(clippy::too_many_lines)]
fn introspect_sqlite_schema(conn: &mut SqliteConnection) -> Result<Schema> {
    use diesel::RunQueryDsl;

    #[derive(QueryableByName)]
    struct TableName {
        #[diesel(sql_type = Text)]
        name: String,
    }

    #[derive(QueryableByName)]
    struct ColumnInfo {
        #[diesel(sql_type = Integer)]
        #[allow(dead_code)]
        cid: i32,
        #[diesel(sql_type = Text)]
        name: String,
        #[diesel(sql_type = Text)]
        r#type: String,
        #[diesel(sql_type = Integer)]
        notnull: i32,
        #[diesel(sql_type = Integer)]
        pk: i32,
    }

    #[derive(QueryableByName)]
    struct ForeignKeyInfo {
        #[diesel(sql_type = Text)]
        table: String,
        #[diesel(sql_type = Text)]
        from: String,
        #[diesel(sql_type = Text)]
        to: String,
    }

    #[derive(QueryableByName)]
    struct IndexInfo {
        #[diesel(sql_type = Text)]
        name: String,
        #[diesel(sql_type = Integer)]
        #[allow(dead_code)]
        unique: i32,
        #[diesel(sql_type = Text)]
        origin: String,
    }

    #[derive(QueryableByName)]
    struct IndexColumnInfo {
        #[diesel(sql_type = Text)]
        name: String,
    }

    let mut schema = Schema {
        tables: BTreeMap::new(),
    };

    // Get all tables
    let tables: Vec<TableName> = diesel::sql_query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '__diesel_schema_migrations' ORDER BY name"
    )
    .load(conn)
    .wrap_err("Failed to query SQLite tables")?;

    for table in tables {
        let mut table_info = Table {
            columns: BTreeMap::new(),
            primary_keys: BTreeSet::new(),
            foreign_keys: BTreeSet::new(),
            unique_constraints: BTreeSet::new(),
            indexes: BTreeSet::new(),
        };

        // Get columns
        let columns: Vec<ColumnInfo> =
            diesel::sql_query(format!("PRAGMA table_info({})", table.name))
                .load(conn)
                .wrap_err(format!("Failed to get columns for table {}", table.name))?;

        for col in columns {
            let normalized_type = normalize_sqlite_type(&col.r#type);
            table_info.columns.insert(
                col.name.clone(),
                Column {
                    name: col.name.clone(),
                    normalized_type,
                    nullable: col.notnull == 0,
                },
            );

            if col.pk > 0 {
                table_info.primary_keys.insert(col.name);
            }
        }

        // Get foreign keys
        let fks: Vec<ForeignKeyInfo> =
            diesel::sql_query(format!("PRAGMA foreign_key_list({})", table.name))
                .load(conn)
                .wrap_err(format!(
                    "Failed to get foreign keys for table {}",
                    table.name
                ))?;

        for fk in fks {
            table_info.foreign_keys.insert(ForeignKey {
                from_column: fk.from,
                to_table: fk.table,
                to_column: fk.to,
            });
        }

        // Get indexes and unique constraints
        let indexes: Vec<IndexInfo> =
            diesel::sql_query(format!("PRAGMA index_list({})", table.name))
                .load(conn)
                .wrap_err(format!("Failed to get indexes for table {}", table.name))?;

        for idx in indexes {
            let index_columns: Vec<IndexColumnInfo> =
                diesel::sql_query(format!("PRAGMA index_info({})", idx.name))
                    .load(conn)
                    .wrap_err(format!("Failed to get index columns for {}", idx.name))?;

            let column_names: Vec<String> = index_columns.into_iter().map(|c| c.name).collect();

            // If origin is 'u', it's a unique constraint (including sqlite_autoindex_*)
            if idx.origin == "u" {
                table_info.unique_constraints.insert(UniqueConstraint {
                    columns: column_names,
                });
            } else if !idx.name.starts_with("sqlite_autoindex_") {
                // Regular index (skip auto-generated indexes that aren't unique constraints)
                table_info.indexes.insert(Index {
                    name: idx.name,
                    columns: column_names,
                });
            }
        }

        schema.tables.insert(table.name, table_info);
    }

    Ok(schema)
}

/// Introspect `MySQL` schema
#[allow(clippy::too_many_lines)]
fn introspect_mysql_schema(conn: &mut MysqlConnection) -> Result<Schema> {
    use diesel::RunQueryDsl;

    #[derive(QueryableByName)]
    struct TableName {
        #[diesel(sql_type = Text)]
        table_name: String,
    }

    #[derive(QueryableByName)]
    struct ColumnInfo {
        #[diesel(sql_type = Text)]
        column_name: String,
        #[diesel(sql_type = Text)]
        data_type: String,
        #[diesel(sql_type = Text)]
        is_nullable: String,
        #[diesel(sql_type = Text)]
        column_key: String,
    }

    #[derive(QueryableByName)]
    #[allow(clippy::struct_field_names)]
    struct ForeignKeyInfo {
        #[diesel(sql_type = Text)]
        column_name: String,
        #[diesel(sql_type = Text)]
        referenced_table_name: String,
        #[diesel(sql_type = Text)]
        referenced_column_name: String,
    }

    #[derive(QueryableByName)]
    #[allow(clippy::struct_field_names)]
    struct UniqueConstraintInfo {
        #[diesel(sql_type = Text)]
        constraint_name: String,
        #[diesel(sql_type = Text)]
        column_name: String,
    }

    #[derive(QueryableByName)]
    struct IndexInfo {
        #[diesel(sql_type = Text)]
        index_name: String,
        #[diesel(sql_type = Text)]
        column_name: String,
        #[diesel(sql_type = Integer)]
        non_unique: i32,
    }

    let mut schema = Schema {
        tables: BTreeMap::new(),
    };

    // Get database name from connection
    let db_name = "zabbid_verify";

    // Get all tables
    let tables: Vec<TableName> = diesel::sql_query(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = ? AND table_name != '__diesel_schema_migrations' ORDER BY table_name"
    )
    .bind::<Text, _>(db_name)
    .load(conn)
    .wrap_err("Failed to query MySQL tables")?;

    for table in tables {
        let mut table_info = Table {
            columns: BTreeMap::new(),
            primary_keys: BTreeSet::new(),
            foreign_keys: BTreeSet::new(),
            unique_constraints: BTreeSet::new(),
            indexes: BTreeSet::new(),
        };

        // Get columns
        let columns: Vec<ColumnInfo> = diesel::sql_query(
            "SELECT column_name, data_type, is_nullable, column_key FROM information_schema.columns WHERE table_schema = ? AND table_name = ? ORDER BY ordinal_position"
        )
        .bind::<Text, _>(db_name)
        .bind::<Text, _>(&table.table_name)
        .load(conn)
        .wrap_err(format!("Failed to get columns for table {}", table.table_name))?;

        for col in columns {
            let normalized_type = normalize_mysql_type(&col.data_type);
            table_info.columns.insert(
                col.column_name.clone(),
                Column {
                    name: col.column_name.clone(),
                    normalized_type,
                    nullable: col.is_nullable == "YES",
                },
            );

            if col.column_key == "PRI" {
                table_info.primary_keys.insert(col.column_name);
            }
        }

        // Get foreign keys
        let fks: Vec<ForeignKeyInfo> = diesel::sql_query(
            "SELECT column_name, referenced_table_name, referenced_column_name \
             FROM information_schema.key_column_usage \
             WHERE table_schema = ? AND table_name = ? AND referenced_table_name IS NOT NULL \
             ORDER BY column_name",
        )
        .bind::<Text, _>(db_name)
        .bind::<Text, _>(&table.table_name)
        .load(conn)
        .wrap_err(format!(
            "Failed to get foreign keys for table {}",
            table.table_name
        ))?;

        for fk in fks {
            table_info.foreign_keys.insert(ForeignKey {
                from_column: fk.column_name,
                to_table: fk.referenced_table_name,
                to_column: fk.referenced_column_name,
            });
        }

        // Get unique constraints
        let unique_constraints: Vec<UniqueConstraintInfo> = diesel::sql_query(
            "SELECT tc.constraint_name, kcu.column_name \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
               ON tc.constraint_name = kcu.constraint_name \
               AND tc.table_schema = kcu.table_schema \
               AND tc.table_name = kcu.table_name \
             WHERE tc.constraint_type = 'UNIQUE' \
               AND tc.table_schema = ? \
               AND tc.table_name = ? \
             ORDER BY tc.constraint_name, kcu.ordinal_position",
        )
        .bind::<Text, _>(db_name)
        .bind::<Text, _>(&table.table_name)
        .load(conn)
        .wrap_err(format!(
            "Failed to get unique constraints for table {}",
            table.table_name
        ))?;

        // Group by constraint name to handle multi-column constraints
        let mut constraint_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for uc in unique_constraints {
            constraint_map
                .entry(uc.constraint_name)
                .or_default()
                .push(uc.column_name);
        }

        for (_name, columns) in constraint_map {
            table_info
                .unique_constraints
                .insert(UniqueConstraint { columns });
        }

        // Get indexes (excluding primary key and unique constraints)
        let indexes: Vec<IndexInfo> = diesel::sql_query(
            "SELECT index_name, column_name, non_unique FROM information_schema.statistics \
             WHERE table_schema = ? AND table_name = ? AND index_name != 'PRIMARY' \
             ORDER BY index_name, seq_in_index",
        )
        .bind::<Text, _>(db_name)
        .bind::<Text, _>(&table.table_name)
        .load(conn)
        .wrap_err(format!(
            "Failed to get indexes for table {}",
            table.table_name
        ))?;

        let mut index_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for idx in indexes {
            // Skip unique indexes (non_unique = 0) as they're tracked as constraints
            if idx.non_unique == 0 {
                continue;
            }

            index_map
                .entry(idx.index_name)
                .or_default()
                .push(idx.column_name);
        }

        for (name, columns) in index_map {
            table_info.indexes.insert(Index { name, columns });
        }

        schema.tables.insert(table.table_name, table_info);
    }

    Ok(schema)
}

/// Normalize `SQLite` type to common representation
fn normalize_sqlite_type(sqlite_type: &str) -> String {
    let normalized = sqlite_type.to_uppercase();
    if normalized.contains("INT") {
        "integer".to_string()
    } else if normalized.contains("TEXT")
        || normalized.contains("CHAR")
        || normalized.contains("CLOB")
    {
        "text".to_string()
    } else if normalized.contains("REAL")
        || normalized.contains("FLOA")
        || normalized.contains("DOUB")
    {
        "real".to_string()
    } else if normalized.contains("BLOB") {
        "blob".to_string()
    } else {
        "text".to_string() // Default for SQLite
    }
}

/// Normalize `MySQL` type to common representation
#[allow(clippy::match_same_arms)]
fn normalize_mysql_type(mysql_type: &str) -> String {
    let normalized = mysql_type.to_uppercase();
    match normalized.as_str() {
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "BIGINT" => "integer".to_string(),
        "DECIMAL" | "NUMERIC" | "FLOAT" | "DOUBLE" | "REAL" => "real".to_string(),
        "CHAR" | "VARCHAR" | "TINYTEXT" | "TEXT" | "MEDIUMTEXT" | "LONGTEXT" => "text".to_string(),
        "BINARY" | "VARBINARY" | "TINYBLOB" | "BLOB" | "MEDIUMBLOB" | "LONGBLOB" => {
            "blob".to_string()
        }
        _ => "text".to_string(),
    }
}

/// Compare schemas and fail on mismatch
#[allow(clippy::too_many_lines)]
fn compare_schemas(sqlite_schema: &Schema, mysql_schema: &Schema) -> Result<()> {
    let sqlite_tables: BTreeSet<_> = sqlite_schema.tables.keys().collect();
    let mysql_tables: BTreeSet<_> = mysql_schema.tables.keys().collect();

    // Check table parity
    if sqlite_tables != mysql_tables {
        let mut errors = Vec::new();

        for table in sqlite_tables.difference(&mysql_tables) {
            errors.push(format!(
                "  - Table '{table}' exists in SQLite but not in MySQL"
            ));
        }

        for table in mysql_tables.difference(&sqlite_tables) {
            errors.push(format!(
                "  - Table '{table}' exists in MySQL but not in SQLite"
            ));
        }

        return Err(color_eyre::eyre::eyre!(
            "❌ Schema parity check FAILED: Table mismatch\n{}",
            errors.join("\n")
        ));
    }

    // Check each table
    for table_name in sqlite_tables {
        let sqlite_table = &sqlite_schema.tables[table_name];
        let mysql_table = &mysql_schema.tables[table_name];

        // Check columns
        let sqlite_columns: BTreeSet<_> = sqlite_table.columns.keys().collect();
        let mysql_columns: BTreeSet<_> = mysql_table.columns.keys().collect();

        if sqlite_columns != mysql_columns {
            let mut errors = Vec::new();

            for col in sqlite_columns.difference(&mysql_columns) {
                errors.push(format!(
                    "    - Column '{col}' exists in SQLite but not in MySQL"
                ));
            }

            for col in mysql_columns.difference(&sqlite_columns) {
                errors.push(format!(
                    "    - Column '{col}' exists in MySQL but not in SQLite"
                ));
            }

            return Err(color_eyre::eyre::eyre!(
                "❌ Schema parity check FAILED: Column mismatch in table '{}'\n{}",
                table_name,
                errors.join("\n")
            ));
        }

        // Check column types and nullability
        for col_name in sqlite_columns {
            let sqlite_col = &sqlite_table.columns[col_name];
            let mysql_col = &mysql_table.columns[col_name];

            if sqlite_col.normalized_type != mysql_col.normalized_type {
                return Err(color_eyre::eyre::eyre!(
                    "❌ Schema parity check FAILED: Type mismatch in table '{}', column '{}'\n  SQLite: {}\n  MySQL: {}",
                    table_name,
                    col_name,
                    sqlite_col.normalized_type,
                    mysql_col.normalized_type
                ));
            }

            if sqlite_col.nullable != mysql_col.nullable {
                return Err(color_eyre::eyre::eyre!(
                    "❌ Schema parity check FAILED: Nullability mismatch in table '{}', column '{}'\n  SQLite nullable: {}\n  MySQL nullable: {}",
                    table_name,
                    col_name,
                    sqlite_col.nullable,
                    mysql_col.nullable
                ));
            }
        }

        // Check primary keys
        if sqlite_table.primary_keys != mysql_table.primary_keys {
            return Err(color_eyre::eyre::eyre!(
                "❌ Schema parity check FAILED: Primary key mismatch in table '{}'\n  SQLite: {:?}\n  MySQL: {:?}",
                table_name,
                sqlite_table.primary_keys,
                mysql_table.primary_keys
            ));
        }

        // Check foreign keys
        if sqlite_table.foreign_keys != mysql_table.foreign_keys {
            return Err(color_eyre::eyre::eyre!(
                "❌ Schema parity check FAILED: Foreign key mismatch in table '{}'\n  SQLite: {:?}\n  MySQL: {:?}",
                table_name,
                sqlite_table.foreign_keys,
                mysql_table.foreign_keys
            ));
        }

        // Check unique constraints
        if sqlite_table.unique_constraints != mysql_table.unique_constraints {
            return Err(color_eyre::eyre::eyre!(
                "❌ Schema parity check FAILED: Unique constraint mismatch in table '{}'\n  SQLite: {:?}\n  MySQL: {:?}",
                table_name,
                sqlite_table.unique_constraints,
                mysql_table.unique_constraints
            ));
        }

        // Check indexes (by columns, not by name since names may differ)
        // MySQL/InnoDB auto-creates indexes for FK columns, so MySQL may have
        // additional single-column indexes on FK columns that SQLite doesn't have.
        // We verify that all SQLite indexes exist in MySQL, and allow MySQL to
        // have additional FK-related indexes.
        let sqlite_index_columns: BTreeSet<_> =
            sqlite_table.indexes.iter().map(|i| &i.columns).collect();
        let mysql_index_columns: BTreeSet<_> =
            mysql_table.indexes.iter().map(|i| &i.columns).collect();

        // Get FK columns for this table
        let fk_columns: BTreeSet<String> = mysql_table
            .foreign_keys
            .iter()
            .map(|fk| fk.from_column.clone())
            .collect();

        // Check that all SQLite indexes exist in MySQL
        for sqlite_idx_cols in &sqlite_index_columns {
            if !mysql_index_columns.contains(sqlite_idx_cols) {
                return Err(color_eyre::eyre::eyre!(
                    "❌ Schema parity check FAILED: Index missing in MySQL for table '{}'\n  Missing index columns: {:?}",
                    table_name,
                    sqlite_idx_cols
                ));
            }
        }

        // Check that any additional MySQL indexes are single-column FK indexes
        for mysql_idx_cols in &mysql_index_columns {
            if !sqlite_index_columns.contains(mysql_idx_cols) {
                // Allow single-column FK indexes in MySQL
                let is_single_fk_index =
                    mysql_idx_cols.len() == 1 && fk_columns.contains(&mysql_idx_cols[0]);

                if !is_single_fk_index {
                    return Err(color_eyre::eyre::eyre!(
                        "❌ Schema parity check FAILED: Unexpected index in MySQL for table '{}'\n  Extra index columns: {:?}\n  (Only single-column FK indexes are allowed as MySQL-specific)",
                        table_name,
                        mysql_idx_cols
                    ));
                }
            }
        }
    }

    Ok(())
}

/// An extension trait for `duct::Expression` that logs the command being run
/// before running it.
trait ExpressionExt {
    /// Run the command and log the command being run
    fn run_with_trace(&self) -> io::Result<Output>;
}

impl ExpressionExt for duct::Expression {
    fn run_with_trace(&self) -> io::Result<Output> {
        tracing::info!("running command: {:?}", self);
        self.run().inspect_err(|_| {
            // The command that was run may have scrolled off the screen, so repeat it here
            tracing::error!("failed to run command: {:?}", self);
        })
    }
}
