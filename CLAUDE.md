# DNP3 Codebase Guidelines

## Build, Test, Lint Commands
- Build: `cargo build [--features tls,serial,serialization]`
- Run all tests: `cargo test`
- Run specific test: `cargo test --package dnp3 -- outstation::tests::time::responds_to_delay_measure`
- Run specific module: `cargo test --package dnp3 -- master::tests::startup`
- Run example: `cargo run --example master [args]`
- Format code: `cargo fmt`
- Check lints: `cargo clippy`
- Run with logging: `RUST_LOG=debug cargo run --example outstation`

## Code Style Guidelines
- **No unsafe code**: Unsafe code is forbidden (`unsafe_code = "forbid"`)
- **Documentation**: All public items must be documented (`missing_docs = "deny"`)
- **API Design**: Avoid unreachable public items (`unreachable_pub = "deny"`)
- **Error Handling**: Use proper error types and propagation (no panics in library code)
- **Naming**: Follow Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- **Imports**: Organize imports by standard lib, external crates, then internal modules
- **Features**: Support compile-time features (tls, serial, serialization)
- **Minimum Rust Version**: 1.75+