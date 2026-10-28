Commercial library by [Step Function I/O](https://stepfunc.io/)

# DNP3

Rust implementation of DNP3 (IEEE 1815) with idiomatic bindings for C, C++, .NET, and Java.

# Features

- Subset Level 3 master and outstation components in a single library
- Written in safe Rust with idiomatic bindings for C, C++, .NET Core, and Java.
- Supports TCP, TLS, UDP, and serial communication channels
- TLS is implemented using [rustls](https://github.com/rustls/rustls) not OpenSSL.
- Automatic mapping between DNP3 and higher-level measurement types
- Built-in logging and protocol decoding
- Blazing fast (and secure) zero-copy / zero-allocation parsing of application data
- Fully asynchronous implementation scales to the OS limit
- Runs on all platforms and operating systems supported by the [Tokio](https://tokio.rs/) runtime:
  - Official support for: Windows x64 and Linux x64, AArch64, ARMv7 and ARMv6
  - Unofficial support: MacOS, PowerPC, MIPS, FreeBSD, and others

# Security

Written in safe Rust, with sources of panics restricted by lints, and the parsing of data received from the network
proven panic-free at link time. See the [security page](https://docs.stepfunc.io/dnp3/1.8.0-RC2/guide/docs/security) of the guide for what is enforced and proven.

# Cargo Features

Default features can be disabled at compile time:
* `tls` - Build the library with support for mutually authenticated TLS using [ring](https://crates.io/crates/ring) for cryptography.
* `serial` - Build the library with support for serial ports

Optional features that may be enabled at compile time:
* `tls-aws-lc` - Same as the `tls` feature but uses [aws-lc-rs](https://crates.io/crates/aws-lc-rs) for cryptography.
* `serialization` - Add [serde](https://docs.rs/crate/serde/latest) de(serialization) support for public configuration types.

The minimum supported Rust version is 1.88.

# License

This crate is made available under a non-commercial / non-production license.
Refer to [`LICENSE.txt`](https://raw.githubusercontent.com/stepfunc/dnp3/main/LICENSE.txt) for the terms
of this non-commercial license.

This software is publicly available, but is not "open source".
__You must purchase a commercial license to use this software for profit.__

Please inquire about commercial licensing on our website:

[https://stepfunc.io/contact/](https://stepfunc.io/contact/)

### AI-assisted development

Subject to the applicable license, AI assistants are welcome for evaluating, integrating, debugging,
and testing this library and for authorized maintenance. Section 2.6 prohibits using the library,
its source, documentation, tests, binaries, or derived materials to design, develop, specify, test,
validate, or improve any other implementation of a protocol it implements, including through
requirement extraction or use as a reference, oracle, or test peer. This applies regardless of
commercial, open-source, private, or competitive status, subject to the exceptions in the agreement.
The license does not permit redistributing the software, including sending it to an AI service for
any of those purposes. Text and data mining rights, including for AI training, are reserved subject
to the permissions and exceptions stated in Section 3.3. This summary does not amend the license.
The `LICENSE.txt` included with each release governs that release; the latest versions are
[`LICENSE.txt`](https://github.com/stepfunc/dnp3/blob/main/LICENSE.txt) and
[`AGENTS.md`](https://github.com/stepfunc/dnp3/blob/main/AGENTS.md).

## Bindings

Bindings in C, C++, Java, and .NET Core are available for this library. See the
[documentation](https://stepfunc.io/products/libraries/dnp3/) for more details.
