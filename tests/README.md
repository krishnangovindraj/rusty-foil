# Integration Tests

This directory contains integration tests for rusty-foil that require a running TypeDB instance.

## Prerequisites

1. **Install TypeDB**: Download and install TypeDB from [https://typedb.com/download](https://typedb.com/download)

2. **Start TypeDB Server**:
   ```bash
   typedb server
   ```

   By default, TypeDB runs on `localhost:1729`

3. **Set up credentials**: The tests use default credentials:
   - Username: `admin`
   - Password: `password`

## Running the Tests

Since these tests require a running TypeDB instance, they are marked with `#[ignore]` by default.

### Run all integration tests:
```bash
cargo test --test integration_test -- --ignored
```

### Run a specific test:
```bash
cargo test --test integration_test test_fetch_schema_from_typedb -- --ignored
```

### Run tests with output:
```bash
cargo test --test integration_test -- --ignored --nocapture
```

## Test Structure

- `test_fetch_schema_from_typedb`: Basic test to verify schema fetching works
- `test_owns_relationships`: Verifies ownership relationships are correctly fetched
- `test_relates_relationships`: Verifies relation roles are correctly fetched
- `test_plays_relationships`: Verifies plays relationships are correctly fetched

## Test Database

Tests use a temporary database named `rusty_foil_test` which is:
- Created before each test
- Populated with a test schema
- Deleted after each test completes

## Troubleshooting

If tests fail:
1. Ensure TypeDB server is running: `typedb server status`
2. Check the server address matches `localhost:1729`
3. Verify credentials are correct
4. Check logs for connection errors