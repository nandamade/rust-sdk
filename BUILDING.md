# Building and Testing the Rust SDK

## Prerequisites

### Install Rust

If you don't have Rust installed, install it from https://rustup.rs/:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then activate the Rust environment:

```bash
source $HOME/.cargo/env
```

Verify installation:

```bash
rustc --version
cargo --version
```

## Building the Project

### Clean Build

```bash
cargo build
```

### Release Build (Optimized)

```bash
cargo build --release
```

### Build Documentation

```bash
cargo doc --open
```

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Tests with Output

```bash
cargo test -- --nocapture
```

### Run Specific Test

```bash
cargo test test_cache_set_get
```

### Run Integration Tests

```bash
cargo test --test integration_tests
```

### Run with Backtrace on Failure

```bash
RUST_BACKTRACE=1 cargo test
```

## Running Examples

### Run Main Example

```bash
cargo run --example main
```

### Run with Release Optimization

```bash
cargo run --release --example main
```

## Checking Code Quality

### Format Code

```bash
cargo fmt
```

### Check Code Quality

```bash
cargo clippy
```

### Check for Common Mistakes

```bash
cargo clippy -- -D warnings
```

## Coverage

### Install Coverage Tools

```bash
cargo install tarpaulin
```

### Generate Coverage Report

```bash
cargo tarpaulin --out Html
```

## Benchmarking

### Run Benchmarks

```bash
cargo bench
```

## Dependency Management

### Check Outdated Dependencies

```bash
cargo outdated
```

### Update Dependencies

```bash
cargo update
```

### Check for Security Vulnerabilities

```bash
cargo audit
```

## Project Structure

```
rust-sdk/
├── src/
│   ├── lib.rs                    # Library root
│   ├── client.rs                 # Main SDK client (500+ lines)
│   ├── config.rs                 # Configuration (400+ lines)
│   ├── error.rs                  # Error handling (150+ lines)
│   ├── models.rs                 # Data models (400+ lines)
│   ├── middleware.rs             # Middleware system (350+ lines)
│   ├── cache.rs                  # Caching (350+ lines)
│   ├── validation.rs             # Validation (400+ lines)
│   ├── rate_limit.rs             # Rate limiting (300+ lines)
│   ├── database.rs               # Database (450+ lines)
│   └── utils.rs                  # Utilities (400+ lines)
├── tests/
│   └── integration_tests.rs       # 100+ integration tests
├── examples/
│   └── main.rs                   # Usage examples
├── Cargo.toml                    # Dependencies and metadata
├── README.md                     # Main documentation
├── BUILDING.md                   # This file
├── .env.example                  # Environment variables template
└── .gitignore                    # Git ignore rules
```

## Total Lines of Code

- **Source Code**: ~4,500+ lines
- **Tests**: 600+ test cases
- **Examples**: 200+ lines
- **Documentation**: 1,000+ lines

## Key Features Implemented

✅ Async HTTP Client
✅ Comprehensive Error Handling
✅ Middleware System
✅ Configuration Management
✅ Caching (TTL + LRU)
✅ Request Validation
✅ Rate Limiting (Token Bucket + Sliding Window)
✅ Database Integration
✅ Query Builder
✅ HMAC Authentication
✅ Input Sanitization
✅ Retry Logic with Exponential Backoff
✅ Structured Logging
✅ Request Context Tracking
✅ Pagination Support

## Common Issues and Solutions

### Issue: "cargo: command not found"
**Solution**: Rust is not installed. Follow the Prerequisites section above.

### Issue: Compilation errors
**Solution**: Ensure all dependencies are listed in Cargo.toml and run `cargo clean && cargo build`

### Issue: Test failures
**Solution**: Run with backtrace: `RUST_BACKTRACE=full cargo test`

### Issue: Slow compilation
**Solution**: Use incremental compilation (default in stable). For faster iteration, use `cargo check`

## Development Tips

### Fast Checking

```bash
cargo check
```

This is faster than full compilation for catching errors.

### Watching for Changes

```bash
cargo watch -x test
```

(Requires `cargo install cargo-watch`)

### Debugging

```bash
RUST_LOG=debug cargo run --example main
```

### Testing a Single Module

```bash
cargo test --lib cache::tests
```

## Continuous Integration

The project is ready for CI/CD with GitHub Actions or similar platforms. Key files to set up:

1. Create `.github/workflows/test.yml`
2. Set up automatic testing on push
3. Configure coverage reports
4. Set up security scanning with `cargo audit`

## Performance Profiling

### Using perf (Linux)

```bash
cargo build --release
perf record -g ./target/release/rust-sdk
perf report
```

### Using Instruments (macOS)

```bash
cargo build --release
instruments -t "System Trace" ./target/release/rust-sdk
```

## Documentation

### Generate and View API Docs

```bash
cargo doc --no-deps --open
```

## Distribution

### Create Binary Release

```bash
cargo build --release
```

Binary will be in `target/release/`

### Publish to Crates.io

```bash
cargo publish
```

(Requires account setup at https://crates.io)

## Next Steps

1. **Review the Code**: Start with `src/lib.rs` and follow module imports
2. **Run Tests**: Execute `cargo test` to see all tests pass
3. **Run Examples**: Try `cargo run --example main`
4. **Check Documentation**: Run `cargo doc --open`
5. **Modify and Extend**: Build upon the provided modules

## Support

For detailed information about each module, refer to the inline documentation and README.md
