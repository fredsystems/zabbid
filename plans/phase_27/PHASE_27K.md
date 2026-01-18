# Phase 27K — Tooling Cleanup

## Purpose

Remove references to unsupported tooling and dead paths to prevent confusion and maintain accurate documentation.

## Scope

### Analysis Tasks

- Search for `api_cli.py`: `find . -name "api_cli.py"`
- Grep for references: `grep -r "api_cli" --include="*.md" --include="*.py" --include="*.sh" --include="*.toml"`
- Check for Python dependencies in requirements files
- Review documentation for tooling references
- Identify any scripts that depend on removed tooling
- Check CI configuration for obsolete tooling references

### Removal Tasks

- Remove `api_cli.py` file if present
- Remove Python requirements related to CLI
- Update documentation removing CLI references
- Remove or update any scripts depending on CLI
- Update README or setup docs to reflect supported workflows
- Clean up any obsolete shell scripts or utilities

### Documentation Updates

- Ensure supported workflows (xtask, cargo commands) are clearly documented
- Update any getting-started or onboarding documentation
- Remove broken links or references to removed tooling

## Explicit Non-Goals

- Do NOT remove xtask functionality
- Do NOT remove legitimate Python tooling (if any exists for valid purposes)
- Do NOT remove supported CLI interfaces
- Do NOT modify core build or test infrastructure
- Do NOT remove tooling that is actually being used

## Files Likely to Be Affected

### Potential Files to Remove

- `api_cli.py` (if present)
- `requirements.txt` or `pyproject.toml` (if they exist and contain CLI deps)
- Shell scripts wrapping the CLI
- Documentation files describing CLI usage

### Documentation to Update

- `README.md`
- `docs/` directory (if exists)
- Developer setup guides
- Any onboarding documentation

### Configuration Files to Check

- `.github/workflows/` (CI configuration)
- `justfile` or `Makefile` (if they exist)
- Shell scripts in root or `scripts/` directory

## Search Patterns

Execute the following searches to identify cleanup targets:

```bash
# Find api_cli.py file
find . -name "api_cli.py" -o -name "*api_cli*"

# Find all references to api_cli
grep -rn "api_cli" . --include="*.md" --include="*.py" --include="*.sh" --include="*.toml" --include="*.yaml" --include="*.yml"

# Find Python requirement files
find . -name "requirements.txt" -o -name "pyproject.toml" -o -name "setup.py"

# Find shell scripts that might wrap CLI
find . -name "*.sh" -exec grep -l "api_cli\|python.*cli" {} \;

# Check documentation for CLI references
grep -rn "CLI\|command.line\|api_cli" docs/ --include="*.md" 2>/dev/null || true
grep -rn "CLI\|command.line\|api_cli" README.md 2>/dev/null || true
```

## Verification Steps

After removal, verify:

1. No broken documentation links:
   - Check all markdown files for broken internal links
   - Verify external links still valid

2. No CI breakage:
   - Review CI configuration for removed tooling references
   - Ensure CI still passes after removal

3. Supported workflows documented:
   - `cargo xtask` commands are documented
   - `cargo` commands are documented
   - No ambiguity about how to perform common tasks

4. Clean grep results:
   - `grep -r "api_cli"` returns zero results (or only in this plan doc)
   - No orphaned Python dependencies

## Example Removals

### File Removal

If `api_cli.py` exists and is no longer supported:

- Delete the file
- Remove from `.gitignore` if listed
- Remove from any `.dockerignore` if present

### Documentation Update

Before:

```markdown
## API Testing

Use the API CLI to test endpoints:

    python api_cli.py --endpoint /users --method GET
```

After:

```markdown
## API Testing

Use curl or the test suite to test endpoints:

    cargo test
    # or
    curl -X GET <http://localhost:8080/api/users>
```

### Requirements File Update

Before `requirements.txt`:

```text
requests==2.28.0
click==8.1.0
api-client-lib==1.0.0
```

After:

```text
# No Python dependencies required for this project
# Development uses Rust toolchain via Nix environment
```

Or remove the file entirely if empty.

## Completion Conditions

- No references to `api_cli.py` remain in codebase
- No broken documentation links after removal
- No orphaned Python dependencies
- Supported workflows clearly documented
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
- Git commit contains only tooling cleanup changes
- Commit message clearly explains what was removed and why

## Dependencies

None (can run anytime after Phase 27A)

## Blocks

None

## Execution Notes

### Before Removal

Before removing any file or reference:

1. Confirm the file/tool is actually obsolete
2. Grep to find all references
3. Plan replacement workflow (if any)
4. Document supported alternative in commit message

### Preservation Check

If uncertain whether tooling is obsolete:

- Check git history for recent usage
- Check CI logs for execution
- Search for imports or dependencies
- Ask user for confirmation

Do NOT remove tooling that is actively used.

### Documentation Quality

When updating documentation to remove CLI references:

- Provide clear alternatives
- Update examples to use supported tools
- Maintain same level of detail
- Ensure workflow is still clear

### Incremental Removal

If multiple obsolete tools are found:

- Remove each in separate commits
- Document each removal clearly
- Verify builds pass between removals

This makes it easier to revert if needed.

### Post-Removal Verification

After removal:

```bash
# Verify no references remain
grep -r "api_cli" . --exclude-dir=.git --exclude="PHASE_27K.md"

# Verify CI passes
cargo xtask ci

# Verify pre-commit passes
pre-commit run --all-files

# Verify documentation builds (if applicable)
# (e.g., mdbook build, if docs use mdbook)
```

### Handling Uncertainty

If analysis reveals:

- Tooling might still be used
- References are unclear
- Alternative workflow is not obvious

Stop and ask user for guidance rather than guessing.

## Success Criteria

This phase succeeds when:

- All obsolete tooling removed
- All references to removed tooling cleaned up
- Documentation accurately reflects supported workflows
- No broken links or references
- All automated checks pass
- Codebase is cleaner and less confusing for new contributors
