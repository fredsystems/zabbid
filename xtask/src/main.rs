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
use duct::cmd;
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
    test_mariadb()?;
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
