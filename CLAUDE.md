# DNP3 Codebase Guidelines

## Git Commit Guidelines
- **Never take credit**: Do not add "Generated with Claude Code" or "Co-Authored-By: Claude" to commit messages
- Keep commit messages professional and focused on the changes

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
- **Minimum Rust Version**: 1.85+ (the workspace `rust-version`, checked by the `msrv` CI job)

## Reference Documents
- The private repository `stepfunc/dnp3-reference`, checked out next to this one at `../dnp3-reference`, holds the DNP3 source documents: IEEE 1815-2012 and related standards, the DNP Users Group technical bulletins, and the IED certification test procedures, with searchable Markdown extractions and recorded interpretation decisions
- Use it to answer questions about what the spec or test procedures require. Read its `CLAUDE.md` first: it explains how to find and cite clauses, and that technical bulletins can amend 1815-2012
- If it isn't checked out, ask the user for it rather than working from memory of the spec
- **Keep it private**: the documents are copyrighted. This repository is public, so never copy their text, tables, or figures into code, comments, docs, commit messages, issues, or PRs. Cite by document and clause number instead (e.g. "IEEE 1815-2012 4.5.10", "TB2016-001")