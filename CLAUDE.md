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

## Conformance Tests
- `.github/workflows/conformance.yml` runs the DNP3 IED certification procedures against the outstation with dnp3c (about 12 minutes). It runs on release tags (gating publishing), weekly on `main`, on demand (`gh workflow run conformance.yml --ref <branch>`), and on PRs labeled `conformance`
- Label a PR `conformance` (`gh pr create --label conformance` or `gh pr edit <N> --add-label conformance`) when it changes what the outstation sends or accepts: the outstation, the application, transport, or link layers, or the Java FFI the tests drive. Don't label master-only, docs, CI, or other-binding changes
- On PRs the result is advisory, not a required check, so look at it before merging

## Code Style Guidelines
- **No unsafe code**: Unsafe code is forbidden (`unsafe_code = "forbid"`)
- **Documentation**: All public items must be documented (`missing_docs = "deny"`)
- **API Design**: Avoid unreachable public items (`unreachable_pub = "deny"`)
- **Error Handling**: Use proper error types and propagation (no panics in library code)
- **Naming**: Follow Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- **Imports**: Organize imports by standard lib, external crates, then internal modules
- **Features**: Support compile-time features (tls, serial, serialization)
- **Minimum Rust Version**: 1.87+ (the workspace `rust-version`, checked by the `msrv` CI job)

## Reference Documents
- The private repository `stepfunc/dnp3-reference`, checked out next to this one at `../dnp3-reference`, holds the DNP3 source documents: IEEE 1815-2012 and related standards, the DNP Users Group technical bulletins, and the IED certification test procedures. Some documents (1815-2012 and the 2.7 and 3.1r1 test procedures) have searchable Markdown extractions; the bulletins are PDFs with an index of what each one changes. It also records our interpretation decisions
- Use it to answer questions about what the spec or test procedures require. Read its `CLAUDE.md` first and follow its guidance on reading, citing, and interpreting the documents. Before relying on a 1815-2012 requirement, check its bulletin index for amendments
- Its rules for maintaining that repository (such as committing straight to `main`) apply only there, not here
- If it isn't checked out, ask the user for it rather than working from memory of the spec
- **Keep it private**: the documents are copyrighted. This repository is public, so never copy or upload their PDFs, extractions, screenshots, text, tables, or figures into code, comments, docs, commit messages, issues, PRs, or attachments. Cite by document and clause or test number instead (e.g. "IEEE 1815-2012 4.5.10", "TB2016-001")