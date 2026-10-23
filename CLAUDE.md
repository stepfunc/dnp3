# DNP3 Codebase Guidelines

## Licensing and AI-Assisted Development
This repository is proprietary, source-available software, not open source. Before substantial code extraction, translation, or reuse, read `LICENSE.txt` and `AGENTS.md`.

AI-assisted evaluation, integration, testing, and authorized maintenance are welcome. Using this repository (source, docs, tests, or binaries) to create, specify, or test any other DNP3 implementation, commercial or open source, is not permitted (LICENSE.txt Section 2.6). Follow `AGENTS.md`; applicable written license terms take precedence.

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
- No-panic check: `RUSTFLAGS="--cfg dnp3_no_panic" CARGO_PROFILE_RELEASE_LTO=thin CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 cargo test -p dnp3 --lib --release --no-run` (use a separate `CARGO_TARGET_DIR`). A link error names a function marked `no_panic` that the optimizer couldn't prove panic-free; see `dnp3/src/no_panic_check.rs`

## Conformance Tests
- `.github/workflows/conformance.yml` runs the DNP3 IED certification procedures against the outstation with dnp3c (about 12 minutes). It runs on release tags (gating publishing), weekly on `main`, on demand (`gh workflow run conformance.yml --ref <branch>`), and on PRs labeled `conformance`
- Label a PR `conformance` (`gh pr create --label conformance` or `gh pr edit <N> --add-label conformance`) when it changes what the outstation sends or accepts: the outstation, the application, transport, or link layers, or the Java FFI the tests drive. Don't label master-only, docs, CI, or other-binding changes
- On PRs the result is advisory, not a required check, so look at it before merging

## Dependency Security Advisories
- The `audit` (cargo-audit) and `osv-scanner` CI jobs fail when an advisory is published against a locked dependency, including on unrelated PRs. Check whether `main` is affected, then fix it in its own PR against `main` (usually `cargo update -p <crate>`) and rebase open PRs once it merges
- The PR description must say whether dnp3, as configured, is exposed and why, citing the code that decides it. This analysis is what determines whether released versions need a patch or an announcement
- To assess exposure, check what the released lockfiles (`git show <tag>:Cargo.lock`) pinned and how dnp3 uses the affected code. For TLS: `sfio-rustls-config` builds the trust store from `RootCertStore::empty()` plus only the configured CA file (no system or `webpki-roots` certificates), `CertificateMode::AuthorityBased` uses the webpki verifiers against that store, and `CertificateMode::SelfSigned` rejects any intermediates and compares the peer certificate byte for byte without building a chain
- Add a `:shield:` changelog entry naming the advisory and the exposure. Rust users pick up a fixed dependency on their next `cargo update`, so a lockfile update only changes the prebuilt bindings (C/C++, .NET, Java); say so
- If released versions are exposed in a realistic configuration, raise a patch release with the maintainer; otherwise the fix ships in the next release

## Code Style Guidelines
- **No unsafe code**: Unsafe code is forbidden (`unsafe_code = "forbid"`)
- **Documentation**: All public items must be documented (`missing_docs = "deny"`)
- **API Design**: Avoid unreachable public items (`unreachable_pub = "deny"`)
- **Error Handling**: Use proper error types and propagation (no panics in library code)
- **Panic exceptions**: `unwrap`, `expect`, indexing and unchecked arithmetic are allowed in library code only where input can't reach them: the invariant that protects them is local and trivially true (e.g. a `u8` index into a 256-entry table), or they can only fire after another panic already happened (e.g. a poisoned mutex). Each gets a narrow `#[expect(lint, reason = "...")]`
- **Naming**: Follow Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- **Imports**: Organize imports by standard lib, external crates, then internal modules
- **Features**: Support compile-time features (tls, serial, serialization)
- **Minimum Rust Version**: 1.88+ (the workspace `rust-version`, checked by the `msrv` CI job)

## Reference Documents
- The private repository `stepfunc/protocol-reference`, checked out next to this one at `../protocol-reference`, holds the protocol source documents. The DNP3 ones are under `protocols/dnp3/`: IEEE 1815-2012 and related standards, the DNP Users Group technical bulletins, and the IED certification test procedures. Some documents (1815-2012 and the 2.7 and 3.1r1 test procedures) have searchable Markdown extractions; the bulletins are PDFs with an index of what each one changes. It also records our interpretation decisions
- Use it to answer questions about what the spec or test procedures require. Read its `CLAUDE.md` and `protocols/dnp3/CLAUDE.md` first and follow their guidance on reading, citing, and interpreting the documents. Before relying on a 1815-2012 requirement, check its bulletin index for amendments
- Its rules for maintaining that repository (such as committing straight to `main`) apply only there, not here
- If it isn't checked out, ask the user for it rather than working from memory of the spec
- **Keep it private**: the documents are copyrighted. This repository is public, so never copy or upload their PDFs, extractions, screenshots, text, tables, or figures into code, comments, docs, commit messages, issues, PRs, or attachments. Cite by document and clause or test number instead (e.g. "IEEE 1815-2012 4.5.10", "TB2016-001")