# DNP3 Codebase Guidelines

## Licensing and AI-Assisted Development

@AGENTS.md

Read `LICENSE.txt` before substantial analysis, copying, extraction,
translation, or reuse. Follow the imported agent policy. Applicable
written license terms and authorizations govern.

## Git Commit Guidelines
- **Never take credit**: Do not add "Generated with Claude Code" or "Co-Authored-By: Claude" to commit messages or PR descriptions. `main` merges through a squash merge queue, so a PR's description becomes its commit body
- Keep commit messages professional and focused on the changes

## Build, Test, Lint Commands
- Build: `cargo build` (default features `tls`, `serial`; add e.g. `--features serialization,unstable`)
- Run all tests: `cargo test --features unstable` (what CI runs; without `unstable` the tests of unstable APIs are skipped)
- Run specific test: `cargo test --package dnp3 -- outstation::tests::time::responds_to_delay_measure`
- Run specific module: `cargo test --package dnp3 -- master::tests::startup`
- Run an example: examples are workspace packages, e.g. `cargo run -p example-master -- tcp-client` or `cargo run -p example-outstation -- tcp-server` (`-- --help` lists transports and options)
- Run with logging: the examples ignore `RUST_LOG`; pass `--log-level`, e.g. `cargo run -p example-outstation -- --log-level debug tcp-server`
- Format code: `cargo fmt --all` (CI runs `cargo fmt --all -- --check`)
- Check lints: `cargo clippy --all-targets --features serialization,unstable -- -D warnings`. CI also clippies `dnp3` with `--no-default-features` plus each feature alone, and with `tls-aws-lc` in place of `tls` (`linting` and `features` jobs in `.github/workflows/ci.yml`)
- MSRV check: `cargo +1.88.0 check --locked --workspace --all-targets` (the `msrv` CI job)
- Docs: `cargo doc -p dnp3 --no-deps`; the guide is Docusaurus under `guide/` (`npm ci && npm run build`)
- Bindings: `cargo build --release -p dnp3-ffi -p dnp3-ffi-java`, then `cargo run --bin dnp3-bindings -- --c` (or `--dotnet`, `--java`). This generates into `ffi/bindings/` and runs the binding tests; `--no-tests` skips them
- No-panic check: `RUSTFLAGS="--cfg dnp3_no_panic" CARGO_PROFILE_RELEASE_LTO=thin CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 cargo test -p dnp3 --lib --release --no-run` (use a separate `CARGO_TARGET_DIR`). A link error names a function marked `no_panic` that the optimizer couldn't prove panic-free; see `dnp3/src/no_panic_check.rs`. The `no-panic` CI job pins its toolchain, since a newer one can fail the link without a code change

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
- **Lints**: the `dnp3` crate inherits `[workspace.lints.rust]` from `Cargo.toml`: `unsafe_code` is forbidden, and `missing_docs`, `unreachable_pub`, `unnameable_types`, `trivial_casts`, `missing_copy_implementations`, `non_ascii_idents` and `unused` are denied. `dnp3/src/lib.rs` also denies about 30 clippy lints (see it for the full list): panic constructs (`unwrap_used`, `expect_used`, `panic`, `unreachable`, `todo`, ...), `indexing_slicing`, `arithmetic_side_effects`, `as_conversions`, printing, and `allow_attributes`, to guard against accidental panics and unchecked conversions. CI fails on any warning
- **Other crates**: the FFI, bindings and example crates don't inherit these lints. `dnp3-ffi` uses `unsafe` for the C ABI and denies only `unwrap_used` and `expect_used`
- **Suppressing a lint**: `#[allow]` is denied in `dnp3`; use the narrowest `#[expect(lint, reason = "...")]`
- **Panic exceptions**: `unwrap`, `expect`, indexing and unchecked arithmetic are allowed in library code only where input can't reach them: the invariant that protects them is local and trivially true (e.g. a `u8` index into a 256-entry table), or they can only fire after another panic already happened (e.g. a poisoned mutex). Each gets a narrow `#[expect(lint, reason = "...")]`
- **Lints in tests**: `dnp3/clippy.toml` allows `unwrap`, `expect`, `panic` and indexing in test code. Arithmetic and `as` conversions stay denied; a test module that needs arithmetic says so with a module-level `#![expect(clippy::arithmetic_side_effects, reason = "...")]`
- **Error Handling**: Use proper error types and propagation
- **Naming**: Follow Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- **Imports**: Organize imports by standard lib, external crates, then internal modules
- **Features**: `tls` (ring) and `tls-aws-lc` (AWS-LC), `serial`, `serialization` (serde on public config types), `unstable` (APIs exempt from semver), and `ffi` (only for the FFI build); defaults are `tls` and `serial`. Code must build with `--no-default-features` and with each feature but `ffi` alone (the `features` CI job)
- **Minimum Rust Version**: 1.88+ (the workspace `rust-version`, checked by the `msrv` CI job)

## Coding and Testing Conventions
- Compare structured values when testing behavior. Existing formatting tests and credential-redaction tests inspect formatted output; redaction tests assert only that secrets are absent
- Don't install tracing subscribers or assert on emitted log counts, levels or text
- No `macro_rules!` just to compress code; use traits, generics and plain Rust
- When replacing `as` casts, use `x.into()` where the type is inferred, `T::from(x)` only when it isn't, and `cast_signed`/`cast_unsigned` for same-width sign changes
- Pre-allocated fixed buffers are `Box<[u8]>` with a private length, never `Vec`; don't add allocation to paths that don't allocate
- In traits, async methods are `fn ... -> impl Future<Output = ...> + Send`, not `async fn`; prefer generics over `dyn` or cfg swaps
- New tests that drive a session or wait on mock I/O run inside `crate::util::test_support::watchdog`: a 30 s wall-clock timeout fails the test when polled; if it cannot finish, the process aborts after another 5 s
- Avoid comments that merely repeat the lint policy; document necessary invariants and give lint exceptions a narrow `#[expect(..., reason = "...")]`
- Don't add test infrastructure where none exists (e.g. `dnp3-ffi` has no tests) without asking

## Reference Documents
- The private repository `stepfunc/protocol-reference`, checked out next to this one at `../protocol-reference`, holds the protocol source documents. The DNP3 ones are under `protocols/dnp3/`: IEEE 1815-2012 and related standards, the DNP Users Group technical bulletins, and the IED certification test procedures. Some documents (1815-2012 and the 2.7 and 3.1r1 test procedures) have searchable Markdown extractions; the bulletins are PDFs with an index of what each one changes. It also records our interpretation decisions
- Use it to answer questions about what the spec or test procedures require. Read its `CLAUDE.md` and `protocols/dnp3/CLAUDE.md` first and follow their guidance on reading, citing, and interpreting the documents. Before relying on a 1815-2012 requirement, check its bulletin index for amendments
- Its rules for maintaining that repository (such as committing straight to `main`) apply only there, not here
- If it isn't checked out, ask the user for it rather than working from memory of the spec
- **Keep it private**: the documents are copyrighted. This repository is public, so never copy or upload their PDFs, extractions, screenshots, text, tables, or figures into code, comments, docs, commit messages, issues, PRs, or attachments. Cite by document and clause or test number instead (e.g. "IEEE 1815-2012 4.5.10", "TB2016-001")
